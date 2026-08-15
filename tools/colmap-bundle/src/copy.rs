//! Copy Qt frameworks into the bundle without development-only trees.

use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, copy};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

/// Qt `Headers` trees are compile-time only and are not needed at runtime.
pub(crate) fn skip_entry(name: &OsStr) -> bool {
    name == "Headers"
}

/// Copies a framework directory, skipping Headers.
///
/// Qt ships many 0444 files. `fs::copy` preserves that mode and later
/// overwrites fail with EACCES. Replace via unlink + create (0644) instead;
/// that needs directory write permission, not root and not chmod of the source.
pub(crate) fn copy_dir(src: &Path, dest: &Path) -> io::Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        if skip_entry(&entry.file_name()) {
            continue;
        }
        let from = entry.path();
        let to = dest.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_dir(&from, &to)?;
        } else if file_type.is_symlink() {
            let target = fs::read_link(&from)?;
            let _ = fs::remove_file(&to);
            std::os::unix::fs::symlink(target, &to)?;
        } else {
            copy_replace(&from, &to)?;
        }
    }
    Ok(())
}

fn copy_replace(from: &Path, to: &Path) -> io::Result<()> {
    let _ = fs::remove_file(to);
    let mut src = File::open(from).map_err(|err| {
        io::Error::new(err.kind(), format!("open {}: {err}", from.display()))
    })?;
    let mut dst = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o644)
        .open(to)
        .map_err(|err| {
            io::Error::new(err.kind(), format!("create {}: {err}", to.display()))
        })?;
    copy(&mut src, &mut dst).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("copy {} -> {}: {err}", from.display(), to.display()),
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::os::unix::fs::{symlink, PermissionsExt};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "colmap-bundle-copy-{}-{}",
            std::process::id(),
            TEMP_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn set_mode(path: &Path, mode: u32) {
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(mode);
        fs::set_permissions(path, perms).unwrap();
    }

    #[test]
    fn skips_headers_name() {
        assert!(skip_entry(OsStr::new("Headers")));
        assert!(!skip_entry(OsStr::new("Resources")));
        assert!(!skip_entry(OsStr::new("QtGui")));
    }

    #[test]
    fn copy_dir_skips_headers_and_does_not_chmod_source() {
        let root = temp_dir();
        let src = root.join("src");
        let dest = root.join("dest");
        fs::create_dir_all(src.join("Versions/A/Headers")).unwrap();
        fs::create_dir_all(src.join("Versions/A/Resources")).unwrap();
        symlink("Versions/A/Headers", src.join("Headers")).unwrap();

        let privacy = src.join("Versions/A/Resources/PrivacyInfo.xcprivacy");
        let mut file = File::create(&privacy).unwrap();
        file.write_all(b"privacy").unwrap();
        drop(file);
        set_mode(&privacy, 0o444);

        File::create(src.join("Versions/A/Headers/qgui.h")).unwrap();

        copy_dir(&src, &dest).unwrap();

        assert_eq!(fs::metadata(&privacy).unwrap().permissions().mode() & 0o777, 0o444);
        let copied = dest.join("Versions/A/Resources/PrivacyInfo.xcprivacy");
        assert!(copied.is_file());
        assert_eq!(fs::metadata(&copied).unwrap().permissions().mode() & 0o200, 0o200);
        assert!(!dest.join("Headers").exists());
        assert!(!dest.join("Versions/A/Headers").exists());

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn copy_dir_replaces_readonly_dest_without_chmod() {
        let root = temp_dir();
        let src = root.join("src");
        let dest = root.join("dest");
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&dest).unwrap();
        fs::write(src.join("Info.plist"), b"new").unwrap();
        fs::write(dest.join("Info.plist"), b"old").unwrap();
        set_mode(&dest.join("Info.plist"), 0o444);

        copy_dir(&src, &dest).unwrap();

        assert_eq!(fs::read(dest.join("Info.plist")).unwrap(), b"new");
        fs::remove_dir_all(&root).unwrap();
    }
}
