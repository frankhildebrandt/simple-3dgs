use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    // Tauri copies bundle resources with `fs::copy`, which cannot overwrite 0444
    // Qt files. Drop the dest tree first: unlink needs directory write, not root.
    let dest = colmap_libs_dest();
    let _ = fs::remove_dir_all(&dest);
    tauri_build::build();
    copy_colmap_libs(&dest);
}

fn colmap_libs_src() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"))
        .join("binaries")
        .join("colmap-libs")
}

fn colmap_libs_dest() -> PathBuf {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    manifest.join("target").join(profile).join("colmap-libs")
}

/// Places COLMAP dylibs next to the sidecar for `tauri dev`.
fn copy_colmap_libs(dest: &Path) {
    let src = colmap_libs_src();
    if !src.is_dir() {
        return;
    }
    let _ = copy_dir(&src, dest);
}

fn copy_dir(src: &Path, dest: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        if from.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            let _ = fs::remove_file(&to);
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}
