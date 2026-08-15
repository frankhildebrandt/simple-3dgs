//! Finished-training library: ingest, list, `.3dgs` zip import/export.

use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::PipelineError;
use crate::geo::GeoFix;
use crate::project::{first_frame, OUTPUT_PLY, OUTPUT_SPZ, VIEW_JSON};
use crate::settings::PipelineSettings;

pub const LIBRARY_FILE: &str = "library.json";
pub const META_FILE: &str = "meta.json";
pub const POSTER_JPG: &str = "poster.jpg";
pub const POSTER_PNG: &str = "poster.png";
pub const SCRATCH_DIR: &str = ".scratch";

/// On-disk record for one archived splat.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveMeta {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub source_kind: String,
    pub source_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<PipelineSettings>,
    #[serde(default)]
    pub frame_count: u32,
    #[serde(default)]
    pub ply_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geo: Option<GeoFix>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
}

/// UI-facing entry with absolute paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveEntry {
    #[serde(flatten)]
    pub meta: ArchiveMeta,
    pub ply_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poster_path: Option<String>,
    pub dir: String,
    pub has_ply: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct LibraryIndex {
    ids: Vec<String>,
}

/// Root folder chosen by the user. Scratch lives beside archived entries.
#[derive(Debug, Clone)]
pub struct ArchiveLibrary {
    root: PathBuf,
}

/// Inputs for copying a finished training into the library.
pub struct IngestRequest<'a> {
    pub ply: &'a Path,
    pub frames_dir: Option<&'a Path>,
    pub source: &'a Path,
    pub source_kind: &'a str,
    pub settings: Option<PipelineSettings>,
    pub frame_count: u32,
    pub geo: Option<GeoFix>,
    pub reuse_id: Option<String>,
}

impl ArchiveLibrary {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, PipelineError> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|err| PipelineError::from_io_path(err, &root))?;
        fs::create_dir_all(root.join(SCRATCH_DIR))
            .map_err(|err| PipelineError::from_io_path(err, &root))?;
        let lib = Self { root };
        if !lib.index_path().is_file() {
            lib.write_index(&LibraryIndex::default())?;
        }
        Ok(lib)
    }

    #[allow(dead_code)]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Scratch folder for this source, on the same volume as the archive.
    pub fn scratch_dir(&self, source: &Path) -> PathBuf {
        self.root.join(SCRATCH_DIR).join(scratch_name(source))
    }

    pub fn entry_dir(&self, id: &str) -> PathBuf {
        self.root.join(id)
    }

    pub fn list(&self) -> Result<Vec<ArchiveEntry>, PipelineError> {
        let index = self.read_index()?;
        let mut entries = Vec::new();
        for id in index.ids {
            if let Some(entry) = self.load_entry(&id)? {
                entries.push(entry);
            }
        }
        Ok(entries)
    }

    pub fn get(&self, id: &str) -> Result<ArchiveEntry, PipelineError> {
        self.load_entry(id)?
            .ok_or_else(|| PipelineError::message(format!("Archive entry `{id}` was not found.")))
    }

    /// Copies PLY + poster + meta into a new (or reused) archive folder.
    pub fn ingest(&self, request: IngestRequest<'_>) -> Result<ArchiveEntry, PipelineError> {
        if !request.ply.is_file() {
            return Err(PipelineError::message(
                "Cannot archive: scene.ply is missing.",
            ));
        }
        let id = match request.reuse_id {
            Some(id) if is_safe_id(&id) => id,
            _ => unique_id(request.source, &self.read_index()?.ids),
        };
        let dest = self.entry_dir(&id);
        fs::create_dir_all(&dest)?;
        let ply_dest = dest.join(OUTPUT_PLY);
        if request.ply.canonicalize().ok() != ply_dest.canonicalize().ok() {
            fs::copy(request.ply, &ply_dest)?;
        }
        let stale_spz = dest.join(OUTPUT_SPZ);
        if stale_spz.is_file() {
            fs::remove_file(&stale_spz)?;
        }
        if let Some(parent) = request.ply.parent() {
            let view = parent.join(VIEW_JSON);
            if view.is_file() {
                fs::copy(&view, dest.join(VIEW_JSON))?;
            }
        }
        let poster = copy_poster(request.frames_dir, &dest)?;
        let ply_bytes = fs::metadata(&ply_dest)?.len();
        let meta = ArchiveMeta {
            id: id.clone(),
            title: title_from_source(request.source),
            created_at: chrono::Utc::now().to_rfc3339(),
            source_kind: request.source_kind.to_string(),
            source_name: file_name(request.source),
            settings: request.settings,
            frame_count: request.frame_count,
            ply_bytes,
            geo: request.geo,
            poster: poster.clone(),
        };
        self.write_meta(&meta)?;
        self.append_id(&id)?;
        Ok(self.get(&id)?)
    }

    /// Updates the display title. The folder id stays the same.
    pub fn rename(&self, id: &str, title: &str) -> Result<ArchiveEntry, PipelineError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(PipelineError::message("Title cannot be empty."));
        }
        let mut entry = self.get(id)?;
        entry.meta.title = title.to_string();
        self.write_meta(&entry.meta)?;
        self.get(id)
    }

    /// Deletes the entry folder and drops it from the library index.
    pub fn remove(&self, id: &str) -> Result<(), PipelineError> {
        let entry = self.get(id)?;
        fs::remove_dir_all(&entry.dir)?;
        let mut index = self.read_index()?;
        index.ids.retain(|existing| existing != id);
        self.write_index(&index)
    }

    /// Replaces the poster with a JPEG screenshot from the viewer.
    pub fn set_poster(&self, id: &str, jpeg: &[u8]) -> Result<ArchiveEntry, PipelineError> {
        if jpeg.is_empty() {
            return Err(PipelineError::message("Preview image is empty."));
        }
        let entry = self.get(id)?;
        let dir = PathBuf::from(&entry.dir);
        let png = dir.join(POSTER_PNG);
        if png.is_file() {
            fs::remove_file(&png)?;
        }
        fs::write(dir.join(POSTER_JPG), jpeg)?;
        let mut meta = entry.meta;
        meta.poster = Some(POSTER_JPG.to_string());
        self.write_meta(&meta)?;
        self.get(id)
    }

    /// True when `{dir}/scene.spz` exists and is at least as new as `scene.ply`.
    /// SPZ-only entries are already compressed, so they count as fresh.
    pub fn spz_is_fresh(&self, id: &str) -> Result<bool, PipelineError> {
        let entry = self.get(id)?;
        let spz = Path::new(&entry.dir).join(OUTPUT_SPZ);
        if !spz.is_file() {
            return Ok(false);
        }
        if !entry.has_ply {
            return Ok(true);
        }
        let ply = Path::new(&entry.dir).join(OUTPUT_PLY);
        let ply_mtime = fs::metadata(ply)?.modified()?;
        let spz_mtime = fs::metadata(&spz)?.modified()?;
        Ok(spz_mtime >= ply_mtime)
    }

    /// Atomically replaces the derived SPZ cache for an archive entry.
    pub fn write_spz(&self, id: &str, bytes: &[u8]) -> Result<(), PipelineError> {
        if bytes.is_empty() {
            return Err(PipelineError::message(
                "Cannot cache SPZ: encoder returned no data.",
            ));
        }
        let entry = self.get(id)?;
        let dest = Path::new(&entry.dir).join(OUTPUT_SPZ);
        let tmp = dest.with_extension("spz.tmp");
        fs::write(&tmp, bytes)?;
        fs::rename(&tmp, dest)?;
        Ok(())
    }

    /// Copies the cached `scene.spz` to a user-chosen path.
    pub fn export_spz(&self, id: &str, dest: &Path) -> Result<(), PipelineError> {
        let entry = self.get(id)?;
        let src = Path::new(&entry.dir).join(OUTPUT_SPZ);
        if !src.is_file() {
            return Err(PipelineError::message(
                "Cannot export SPZ: scene.spz is missing.",
            ));
        }
        if let Some(parent) = dest.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::copy(&src, dest)?;
        Ok(())
    }

    /// Drops `scene.ply` after a valid `scene.spz` exists, shrinking the archive.
    pub fn drop_uncompressed_ply(&self, id: &str) -> Result<ArchiveEntry, PipelineError> {
        let entry = self.get(id)?;
        let dir = PathBuf::from(&entry.dir);
        let ply = dir.join(OUTPUT_PLY);
        let spz = dir.join(OUTPUT_SPZ);
        if !ply.is_file() {
            return Ok(entry);
        }
        let spz_bytes = match fs::metadata(&spz) {
            Ok(meta) if meta.len() > 0 => meta.len(),
            _ => {
                return Err(PipelineError::message(
                    "Cannot convert to SPZ: scene.spz is missing.",
                ));
            }
        };
        fs::remove_file(&ply)?;
        let mut meta = entry.meta;
        meta.ply_bytes = spz_bytes;
        self.write_meta(&meta)?;
        self.get(id)
    }

    pub fn export_3dgs(&self, id: &str, dest: &Path) -> Result<(), PipelineError> {
        let entry = self.get(id)?;
        let src = PathBuf::from(&entry.dir);
        let file = fs::File::create(dest)?;
        let mut zip = ZipWriter::new(file);
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        add_zip_file(&mut zip, &src.join(META_FILE), META_FILE, opts)?;
        let splat = if src.join(OUTPUT_PLY).is_file() {
            OUTPUT_PLY
        } else {
            OUTPUT_SPZ
        };
        add_zip_file(&mut zip, &src.join(splat), splat, opts)?;
        if src.join(VIEW_JSON).is_file() {
            add_zip_file(&mut zip, &src.join(VIEW_JSON), VIEW_JSON, opts)?;
        }
        if let Some(poster) = &entry.meta.poster {
            add_zip_file(&mut zip, &src.join(poster), poster, opts)?;
        }
        zip.finish().map_err(zip_err)?;
        Ok(())
    }

    /// Unpacks a `.3dgs` zip into the library. Collision on id gets a fresh suffix.
    pub fn import_3dgs(&self, zip_path: &Path) -> Result<ArchiveEntry, PipelineError> {
        let file = fs::File::open(zip_path).map_err(|_| {
            PipelineError::message(format!("Could not open {}", zip_path.display()))
        })?;
        let mut archive = ZipArchive::new(file)
            .map_err(|err| PipelineError::message(format!("Not a valid .3dgs archive: {err}")))?;
        let tmp = self
            .root
            .join(SCRATCH_DIR)
            .join(format!("import-{}", Uuid::new_v4()));
        fs::create_dir_all(&tmp)?;
        for i in 0..archive.len() {
            let mut zipped = archive.by_index(i).map_err(zip_err)?;
            let name = zipped.name().to_string();
            let dest = safe_unzip_path(&tmp, &name)?;
            if zipped.is_dir() {
                fs::create_dir_all(&dest)?;
                continue;
            }
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out = fs::File::create(&dest)?;
            std::io::copy(&mut zipped, &mut out)?;
        }
        let meta_path = find_meta(&tmp)?;
        let mut meta: ArchiveMeta =
            serde_json::from_slice(&fs::read(&meta_path)?).map_err(json_err)?;
        let splat_src = meta_path.parent().unwrap_or(&tmp);
        let ply_src = splat_src.join(OUTPUT_PLY);
        let spz_src = splat_src.join(OUTPUT_SPZ);
        if !ply_src.is_file() && !spz_src.is_file() {
            let _ = fs::remove_dir_all(&tmp);
            return Err(PipelineError::message(
                "The .3dgs file is missing scene.ply or scene.spz.",
            ));
        }
        let mut index = self.read_index()?;
        if !is_safe_id(&meta.id) || index.ids.contains(&meta.id) {
            meta.id = unique_id(Path::new(&meta.source_name), &index.ids);
        }
        meta.source_kind = "import".into();
        let dest = self.entry_dir(&meta.id);
        if dest.exists() {
            fs::remove_dir_all(&dest)?;
        }
        fs::create_dir_all(&dest)?;
        if ply_src.is_file() {
            fs::copy(&ply_src, dest.join(OUTPUT_PLY))?;
        }
        if spz_src.is_file() {
            fs::copy(&spz_src, dest.join(OUTPUT_SPZ))?;
        }
        let view_src = splat_src.join(VIEW_JSON);
        if view_src.is_file() {
            fs::copy(&view_src, dest.join(VIEW_JSON))?;
        }
        if let Some(poster) = &meta.poster {
            let src_poster = meta_path.parent().unwrap_or(&tmp).join(poster);
            if src_poster.is_file() {
                fs::copy(&src_poster, dest.join(poster))?;
            }
        }
        let splat = if dest.join(OUTPUT_PLY).is_file() {
            dest.join(OUTPUT_PLY)
        } else {
            dest.join(OUTPUT_SPZ)
        };
        let ply_bytes = fs::metadata(splat)?.len();
        meta.ply_bytes = ply_bytes;
        fs::write(
            dest.join(META_FILE),
            serde_json::to_vec_pretty(&meta).map_err(json_err)?,
        )?;
        index.ids.push(meta.id.clone());
        self.write_index(&index)?;
        let _ = fs::remove_dir_all(&tmp);
        self.get(&meta.id)
    }

    fn load_entry(&self, id: &str) -> Result<Option<ArchiveEntry>, PipelineError> {
        if !is_safe_id(id) {
            return Ok(None);
        }
        let dir = self.entry_dir(id);
        let meta_path = dir.join(META_FILE);
        if !meta_path.is_file() {
            return Ok(None);
        }
        let meta: ArchiveMeta = serde_json::from_slice(&fs::read(meta_path)?).map_err(json_err)?;
        let ply = dir.join(OUTPUT_PLY);
        let spz = dir.join(OUTPUT_SPZ);
        let has_ply = ply.is_file();
        let splat = if has_ply {
            ply
        } else if spz.is_file() {
            spz
        } else {
            return Ok(None);
        };
        let poster_path = meta
            .poster
            .as_ref()
            .map(|name| dir.join(name))
            .filter(|path| path.is_file())
            .map(|path| path.to_string_lossy().into_owned());
        Ok(Some(ArchiveEntry {
            meta,
            ply_path: splat.to_string_lossy().into_owned(),
            poster_path,
            dir: dir.to_string_lossy().into_owned(),
            has_ply,
        }))
    }

    fn index_path(&self) -> PathBuf {
        self.root.join(LIBRARY_FILE)
    }

    fn read_index(&self) -> Result<LibraryIndex, PipelineError> {
        let path = self.index_path();
        if !path.is_file() {
            return Ok(LibraryIndex::default());
        }
        serde_json::from_slice(&fs::read(path)?).map_err(json_err)
    }

    fn write_index(&self, index: &LibraryIndex) -> Result<(), PipelineError> {
        fs::write(
            self.index_path(),
            serde_json::to_vec_pretty(index).map_err(json_err)?,
        )?;
        Ok(())
    }

    fn append_id(&self, id: &str) -> Result<(), PipelineError> {
        let mut index = self.read_index()?;
        if !index.ids.iter().any(|existing| existing == id) {
            index.ids.push(id.to_string());
            self.write_index(&index)?;
        }
        Ok(())
    }

    fn write_meta(&self, meta: &ArchiveMeta) -> Result<(), PipelineError> {
        fs::write(
            self.entry_dir(&meta.id).join(META_FILE),
            serde_json::to_vec_pretty(meta).map_err(json_err)?,
        )?;
        Ok(())
    }
}

fn copy_poster(frames_dir: Option<&Path>, dest: &Path) -> Result<Option<String>, PipelineError> {
    let Some(frames) = frames_dir else {
        return Ok(None);
    };
    let Some(src) = first_frame(frames) else {
        return Ok(None);
    };
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg")
        .to_ascii_lowercase();
    let name = if ext == "png" { POSTER_PNG } else { POSTER_JPG };
    fs::copy(&src, dest.join(name))?;
    Ok(Some(name.to_string()))
}

fn add_zip_file(
    zip: &mut ZipWriter<fs::File>,
    path: &Path,
    name: &str,
    opts: SimpleFileOptions,
) -> Result<(), PipelineError> {
    if !path.is_file() {
        return Err(PipelineError::message(format!(
            "Missing {name} for export."
        )));
    }
    zip.start_file(name, opts).map_err(zip_err)?;
    let mut file = fs::File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    zip.write_all(&buf)?;
    Ok(())
}

fn find_meta(root: &Path) -> Result<PathBuf, PipelineError> {
    let direct = root.join(META_FILE);
    if direct.is_file() {
        return Ok(direct);
    }
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            let nested = path.join(META_FILE);
            if nested.is_file() {
                return Ok(nested);
            }
        }
    }
    Err(PipelineError::message(
        "The .3dgs file is missing meta.json.",
    ))
}

fn safe_unzip_path(base: &Path, name: &str) -> Result<PathBuf, PipelineError> {
    let path = Path::new(name);
    if path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(PipelineError::message(
            "The .3dgs file contains an unsafe path.",
        ));
    }
    Ok(base.join(path))
}

fn unique_id(source: &Path, taken: &[String]) -> String {
    let date = chrono::Utc::now().format("%Y-%m-%d");
    let slug = slug(source);
    let short = &Uuid::new_v4().simple().to_string()[..4];
    let mut id = format!("{date}_{slug}_{short}");
    let mut n = 1;
    while taken.contains(&id) {
        n += 1;
        id = format!("{date}_{slug}_{short}{n}");
    }
    id
}

fn scratch_name(source: &Path) -> String {
    let slug = slug(source);
    let hash = short_hash(&source.to_string_lossy());
    format!("{slug}_{hash}")
}

fn slug(source: &Path) -> String {
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("scene");
    let mut out: String = stem
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    let out = out.trim_matches('-');
    let mut out = if out.is_empty() {
        "scene".to_string()
    } else {
        out.to_string()
    };
    if out.len() > 32 {
        out.truncate(32);
    }
    out
}

fn short_hash(text: &str) -> String {
    let mut hash: u32 = 2166136261;
    for byte in text.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16777619);
    }
    format!("{hash:04x}")
}

fn title_from_source(source: &Path) -> String {
    source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string()
}

fn file_name(source: &Path) -> String {
    source
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn is_safe_id(id: &str) -> bool {
    !id.is_empty()
        && !id.contains("..")
        && !id.contains('/')
        && !id.contains('\\')
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn json_err(err: serde_json::Error) -> PipelineError {
    PipelineError::message(format!("Invalid archive metadata: {err}"))
}

fn zip_err(err: impl std::fmt::Display) -> PipelineError {
    PipelineError::message(format!("Zip error: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::{GeoFix, GeoSource};
    use crate::preset::Preset;
    use tempfile::tempdir;

    fn sample_geo() -> GeoFix {
        GeoFix {
            lat: 52.52,
            lon: 13.405,
            alt: Some(12.0),
            source: GeoSource::Quicktime,
        }
    }

    fn ingest_ply(lib: &ArchiveLibrary, dir: &Path, name: &str) -> ArchiveEntry {
        let ply = dir.join(name);
        fs::write(&ply, b"ply\nfake\n").unwrap();
        let frames = dir.join("frames");
        fs::create_dir_all(&frames).unwrap();
        fs::write(frames.join("frame_00001.jpg"), b"poster").unwrap();
        lib.ingest(IngestRequest {
            ply: &ply,
            frames_dir: Some(&frames),
            source: Path::new("/clips/Brandenburg.MOV"),
            source_kind: "video",
            settings: Some(PipelineSettings::from_preset(Preset::Fast)),
            frame_count: 8,
            geo: Some(sample_geo()),
            reuse_id: None,
        })
        .unwrap()
    }

    #[test]
    fn ingest_writes_meta_ply_poster_and_index() {
        let dir = tempdir().unwrap();
        let lib = ArchiveLibrary::open(dir.path().join("archive")).unwrap();
        let entry = ingest_ply(&lib, dir.path(), "scene.ply");
        assert!(Path::new(&entry.ply_path).is_file());
        assert_eq!(entry.meta.source_kind, "video");
        assert_eq!(entry.meta.frame_count, 8);
        assert_eq!(entry.meta.geo.as_ref().unwrap().lat, 52.52);
        assert_eq!(entry.meta.poster.as_deref(), Some("poster.jpg"));
        assert!(entry.has_ply);
        assert_eq!(lib.list().unwrap().len(), 1);
        assert!(!entry.meta.id.contains('/'));
    }

    #[test]
    fn reuse_id_overwrites_same_entry() {
        let dir = tempdir().unwrap();
        let lib = ArchiveLibrary::open(dir.path().join("archive")).unwrap();
        let first = ingest_ply(&lib, dir.path(), "a.ply");
        let ply = dir.path().join("b.ply");
        fs::write(&ply, b"ply\nnew\n").unwrap();
        let second = lib
            .ingest(IngestRequest {
                ply: &ply,
                frames_dir: None,
                source: Path::new("/clips/Brandenburg.MOV"),
                source_kind: "video",
                settings: None,
                frame_count: 9,
                geo: None,
                reuse_id: Some(first.meta.id.clone()),
            })
            .unwrap();
        assert_eq!(second.meta.id, first.meta.id);
        assert_eq!(lib.list().unwrap().len(), 1);
        assert_eq!(
            fs::read(Path::new(&second.ply_path)).unwrap(),
            b"ply\nnew\n"
        );
    }

    #[test]
    fn zip_roundtrip_preserves_geo() {
        let dir = tempdir().unwrap();
        let lib = ArchiveLibrary::open(dir.path().join("archive")).unwrap();
        let entry = ingest_ply(&lib, dir.path(), "scene.ply");
        let zip_path = dir.path().join("scene.3dgs");
        lib.export_3dgs(&entry.meta.id, &zip_path).unwrap();
        let other = ArchiveLibrary::open(dir.path().join("other")).unwrap();
        let imported = other.import_3dgs(&zip_path).unwrap();
        assert_eq!(imported.meta.geo.as_ref().unwrap().lon, 13.405);
        assert_eq!(imported.meta.source_kind, "import");
        assert!(Path::new(&imported.ply_path).is_file());
        assert_eq!(other.list().unwrap().len(), 1);
    }

    #[test]
    fn import_rejects_zip_slip() {
        let dir = tempdir().unwrap();
        let zip_path = dir.path().join("evil.3dgs");
        let file = fs::File::create(&zip_path).unwrap();
        let mut zip = ZipWriter::new(file);
        zip.start_file("../outside.txt", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"nope").unwrap();
        zip.finish().unwrap();
        let lib = ArchiveLibrary::open(dir.path().join("archive")).unwrap();
        let err = lib.import_3dgs(&zip_path).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("unsafe"));
    }

    #[test]
    fn scratch_dir_is_under_archive() {
        let dir = tempdir().unwrap();
        let lib = ArchiveLibrary::open(dir.path().join("archive")).unwrap();
        let scratch = lib.scratch_dir(Path::new("/Volumes/SSD/clip.mp4"));
        assert!(scratch.starts_with(lib.root().join(SCRATCH_DIR)));
        assert!(scratch
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("clip_"));
    }

    #[test]
    fn scratch_not_listed() {
        let dir = tempdir().unwrap();
        let lib = ArchiveLibrary::open(dir.path().join("archive")).unwrap();
        fs::create_dir_all(lib.scratch_dir(Path::new("a.mp4"))).unwrap();
        assert!(lib.list().unwrap().is_empty());
    }

    #[test]
    fn ingest_and_zip_preserve_view_json() {
        let dir = tempdir().unwrap();
        let lib = ArchiveLibrary::open(dir.path().join("archive")).unwrap();
        let ply = dir.path().join("scene.ply");
        fs::write(&ply, b"ply\n").unwrap();
        fs::write(
            dir.path().join(VIEW_JSON),
            r#"{"position":[1.0,2.0,3.0],"quaternion":[0.0,0.0,0.0,1.0]}"#,
        )
        .unwrap();
        let entry = lib
            .ingest(IngestRequest {
                ply: &ply,
                frames_dir: None,
                source: Path::new("clip.mp4"),
                source_kind: "video",
                settings: None,
                frame_count: 8,
                geo: None,
                reuse_id: None,
            })
            .unwrap();
        assert!(Path::new(&entry.dir).join(VIEW_JSON).is_file());
        let zip_path = dir.path().join("scene.3dgs");
        lib.export_3dgs(&entry.meta.id, &zip_path).unwrap();
        let other = ArchiveLibrary::open(dir.path().join("other")).unwrap();
        let imported = other.import_3dgs(&zip_path).unwrap();
        assert!(Path::new(&imported.dir).join(VIEW_JSON).is_file());
    }

    #[test]
    fn rename_updates_title_not_id() {
        let dir = tempdir().unwrap();
        let lib = ArchiveLibrary::open(dir.path().join("archive")).unwrap();
        let entry = ingest_ply(&lib, dir.path(), "scene.ply");
        let id = entry.meta.id.clone();
        let renamed = lib.rename(&id, "  Gate  ").unwrap();
        assert_eq!(renamed.meta.id, id);
        assert_eq!(renamed.meta.title, "Gate");
        assert_eq!(lib.get(&id).unwrap().meta.title, "Gate");
    }

    #[test]
    fn rename_rejects_blank_title() {
        let dir = tempdir().unwrap();
        let lib = ArchiveLibrary::open(dir.path().join("archive")).unwrap();
        let entry = ingest_ply(&lib, dir.path(), "scene.ply");
        let err = lib.rename(&entry.meta.id, "   ").unwrap_err();
        assert!(err.to_string().to_lowercase().contains("empty"));
        assert_eq!(lib.get(&entry.meta.id).unwrap().meta.title, "Brandenburg");
    }

    #[test]
    fn remove_drops_folder_and_index() {
        let dir = tempdir().unwrap();
        let lib = ArchiveLibrary::open(dir.path().join("archive")).unwrap();
        let entry = ingest_ply(&lib, dir.path(), "scene.ply");
        let folder = PathBuf::from(&entry.dir);
        lib.remove(&entry.meta.id).unwrap();
        assert!(lib.list().unwrap().is_empty());
        assert!(!folder.exists());
        assert!(lib.get(&entry.meta.id).is_err());
    }

    #[test]
    fn remove_missing_id_errors() {
        let dir = tempdir().unwrap();
        let lib = ArchiveLibrary::open(dir.path().join("archive")).unwrap();
        let err = lib.remove("nope").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn set_poster_writes_jpeg_and_drops_png() {
        let dir = tempdir().unwrap();
        let lib = ArchiveLibrary::open(dir.path().join("archive")).unwrap();
        let entry = ingest_ply(&lib, dir.path(), "scene.ply");
        let folder = PathBuf::from(&entry.dir);
        fs::write(folder.join(POSTER_PNG), b"old-png").unwrap();
        fs::remove_file(folder.join(POSTER_JPG)).unwrap();
        let mut meta = entry.meta.clone();
        meta.poster = Some(POSTER_PNG.to_string());
        lib.write_meta(&meta).unwrap();

        let updated = lib.set_poster(&meta.id, b"jpeg-bytes").unwrap();
        assert_eq!(updated.meta.poster.as_deref(), Some("poster.jpg"));
        assert_eq!(fs::read(folder.join(POSTER_JPG)).unwrap(), b"jpeg-bytes");
        assert!(!folder.join(POSTER_PNG).exists());
        assert!(updated.poster_path.unwrap().ends_with("poster.jpg"));
    }

    #[test]
    fn set_poster_rejects_empty_bytes() {
        let dir = tempdir().unwrap();
        let lib = ArchiveLibrary::open(dir.path().join("archive")).unwrap();
        let entry = ingest_ply(&lib, dir.path(), "scene.ply");
        let err = lib.set_poster(&entry.meta.id, b"").unwrap_err();
        assert!(err.to_string().to_lowercase().contains("empty"));
    }

    #[test]
    fn spz_cache_is_stale_until_written() {
        let dir = tempdir().unwrap();
        let lib = ArchiveLibrary::open(dir.path().join("archive")).unwrap();
        let entry = ingest_ply(&lib, dir.path(), "scene.ply");
        assert!(!lib.spz_is_fresh(&entry.meta.id).unwrap());
        lib.write_spz(&entry.meta.id, b"spz-bytes").unwrap();
        assert!(lib.spz_is_fresh(&entry.meta.id).unwrap());
        assert_eq!(
            fs::read(Path::new(&entry.dir).join(OUTPUT_SPZ)).unwrap(),
            b"spz-bytes"
        );
    }

    #[test]
    fn replacing_ply_invalidates_spz_cache() {
        let dir = tempdir().unwrap();
        let lib = ArchiveLibrary::open(dir.path().join("archive")).unwrap();
        let entry = ingest_ply(&lib, dir.path(), "scene.ply");
        lib.write_spz(&entry.meta.id, b"spz-bytes").unwrap();
        fs::write(Path::new(&entry.ply_path), b"ply\nnewer\n").unwrap();
        assert!(!lib.spz_is_fresh(&entry.meta.id).unwrap());
    }

    #[test]
    fn ingest_reuse_drops_stale_spz() {
        let dir = tempdir().unwrap();
        let lib = ArchiveLibrary::open(dir.path().join("archive")).unwrap();
        let first = ingest_ply(&lib, dir.path(), "a.ply");
        lib.write_spz(&first.meta.id, b"old-spz").unwrap();
        let ply = dir.path().join("b.ply");
        fs::write(&ply, b"ply\nnew\n").unwrap();
        lib.ingest(IngestRequest {
            ply: &ply,
            frames_dir: None,
            source: Path::new("/clips/Brandenburg.MOV"),
            source_kind: "video",
            settings: None,
            frame_count: 9,
            geo: None,
            reuse_id: Some(first.meta.id.clone()),
        })
        .unwrap();
        assert!(!Path::new(&first.dir).join(OUTPUT_SPZ).is_file());
    }

    #[test]
    fn write_spz_rejects_empty_bytes() {
        let dir = tempdir().unwrap();
        let lib = ArchiveLibrary::open(dir.path().join("archive")).unwrap();
        let entry = ingest_ply(&lib, dir.path(), "scene.ply");
        let err = lib.write_spz(&entry.meta.id, b"").unwrap_err();
        assert!(err.to_string().contains("no data"));
    }

    #[test]
    fn export_spz_copies_cache() {
        let dir = tempdir().unwrap();
        let lib = ArchiveLibrary::open(dir.path().join("archive")).unwrap();
        let entry = ingest_ply(&lib, dir.path(), "scene.ply");
        lib.write_spz(&entry.meta.id, b"spz-bytes").unwrap();
        let dest = dir.path().join("share").join("gate.spz");
        lib.export_spz(&entry.meta.id, &dest).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"spz-bytes");
    }

    #[test]
    fn export_spz_requires_cache() {
        let dir = tempdir().unwrap();
        let lib = ArchiveLibrary::open(dir.path().join("archive")).unwrap();
        let entry = ingest_ply(&lib, dir.path(), "scene.ply");
        let err = lib
            .export_spz(&entry.meta.id, &dir.path().join("out.spz"))
            .unwrap_err();
        assert!(err.to_string().contains("scene.spz is missing"));
    }

    #[test]
    fn zip_export_does_not_include_spz_cache() {
        let dir = tempdir().unwrap();
        let lib = ArchiveLibrary::open(dir.path().join("archive")).unwrap();
        let entry = ingest_ply(&lib, dir.path(), "scene.ply");
        lib.write_spz(&entry.meta.id, b"spz-bytes").unwrap();
        let zip_path = dir.path().join("scene.3dgs");
        lib.export_3dgs(&entry.meta.id, &zip_path).unwrap();
        let file = fs::File::open(&zip_path).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(names.contains(&OUTPUT_PLY.to_string()));
        assert!(!names.iter().any(|name| name.ends_with(OUTPUT_SPZ)));
    }

    #[test]
    fn drop_ply_keeps_spz_and_updates_size() {
        let dir = tempdir().unwrap();
        let lib = ArchiveLibrary::open(dir.path().join("archive")).unwrap();
        let entry = ingest_ply(&lib, dir.path(), "scene.ply");
        lib.write_spz(&entry.meta.id, b"spz-bytes").unwrap();
        let converted = lib.drop_uncompressed_ply(&entry.meta.id).unwrap();
        assert!(!converted.has_ply);
        assert!(converted.ply_path.ends_with(OUTPUT_SPZ));
        assert_eq!(converted.meta.ply_bytes, 9);
        assert!(!Path::new(&entry.dir).join(OUTPUT_PLY).is_file());
        assert!(Path::new(&entry.dir).join(OUTPUT_SPZ).is_file());
        assert!(lib.spz_is_fresh(&entry.meta.id).unwrap());
    }

    #[test]
    fn drop_ply_without_spz_fails() {
        let dir = tempdir().unwrap();
        let lib = ArchiveLibrary::open(dir.path().join("archive")).unwrap();
        let entry = ingest_ply(&lib, dir.path(), "scene.ply");
        let err = lib.drop_uncompressed_ply(&entry.meta.id).unwrap_err();
        assert!(err.to_string().contains("scene.spz is missing"));
        assert!(Path::new(&entry.ply_path).is_file());
    }

    #[test]
    fn drop_ply_is_idempotent_when_already_spz() {
        let dir = tempdir().unwrap();
        let lib = ArchiveLibrary::open(dir.path().join("archive")).unwrap();
        let entry = ingest_ply(&lib, dir.path(), "scene.ply");
        lib.write_spz(&entry.meta.id, b"spz-bytes").unwrap();
        lib.drop_uncompressed_ply(&entry.meta.id).unwrap();
        let again = lib.drop_uncompressed_ply(&entry.meta.id).unwrap();
        assert!(!again.has_ply);
        assert_eq!(
            fs::read(Path::new(&entry.dir).join(OUTPUT_SPZ)).unwrap(),
            b"spz-bytes"
        );
    }

    #[test]
    fn zip_roundtrip_preserves_spz_only_entry() {
        let dir = tempdir().unwrap();
        let lib = ArchiveLibrary::open(dir.path().join("archive")).unwrap();
        let entry = ingest_ply(&lib, dir.path(), "scene.ply");
        lib.write_spz(&entry.meta.id, b"spz-bytes").unwrap();
        lib.drop_uncompressed_ply(&entry.meta.id).unwrap();
        let zip_path = dir.path().join("scene.3dgs");
        lib.export_3dgs(&entry.meta.id, &zip_path).unwrap();
        let file = fs::File::open(&zip_path).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(names.contains(&OUTPUT_SPZ.to_string()));
        assert!(!names.contains(&OUTPUT_PLY.to_string()));
        drop(archive);
        let other = ArchiveLibrary::open(dir.path().join("other")).unwrap();
        let imported = other.import_3dgs(&zip_path).unwrap();
        assert!(!imported.has_ply);
        assert_eq!(
            fs::read(Path::new(&imported.dir).join(OUTPUT_SPZ)).unwrap(),
            b"spz-bytes"
        );
    }
}
