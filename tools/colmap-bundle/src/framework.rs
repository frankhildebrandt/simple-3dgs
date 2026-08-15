//! Qt / Homebrew framework install-name helpers.
//! System frameworks stay on their absolute `/System` / `/usr` / `/Library` paths.

use std::path::{Path, PathBuf};

const BREW_QT_LIB: &[&str] = &[
    "/opt/homebrew/opt/qtbase/lib",
    "/opt/homebrew/opt/qtsvg/lib",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameworkRef {
    pub name: String,
    pub relative: String,
}

impl FrameworkRef {
    pub fn rpath_install_name(&self) -> String {
        format!("@rpath/{}/{}", self.name, self.relative)
    }

    pub fn bundled_binary(&self, libdir: &Path) -> PathBuf {
        libdir.join(&self.name).join(Path::new(&self.relative))
    }
}

pub fn is_system_install_name(name: &str) -> bool {
    name.starts_with("/usr/")
        || name.starts_with("/System/")
        || name.starts_with("/Library/")
}

/// Parses a non-system `…/QtGui.framework/Versions/A/QtGui` install name.
pub fn parse_framework_ref(install_name: &str) -> Option<FrameworkRef> {
    if is_system_install_name(install_name) {
        return None;
    }
    let (prefix, rest) = install_name.split_once(".framework/")?;
    let name = format!("{}.framework", prefix.rsplit('/').next()?);
    if rest.is_empty() {
        return None;
    }
    Some(FrameworkRef {
        name,
        relative: rest.to_string(),
    })
}

pub fn brew_framework_binary(framework: &FrameworkRef) -> Option<PathBuf> {
    for root in BREW_QT_LIB {
        let candidate = Path::new(root)
            .join(&framework.name)
            .join(Path::new(&framework.relative));
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

pub fn source_framework_dir(install_name: &str) -> Option<PathBuf> {
    if is_system_install_name(install_name) {
        return None;
    }
    let idx = install_name.find(".framework")?;
    let dir = PathBuf::from(format!("{}{}", &install_name[..idx], ".framework"));
    dir.is_dir().then_some(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_homebrew_qt_install_name() {
        let name = "/opt/homebrew/opt/qtbase/lib/QtGui.framework/Versions/A/QtGui";
        let parsed = parse_framework_ref(name).unwrap();
        assert_eq!(parsed.name, "QtGui.framework");
        assert_eq!(parsed.relative, "Versions/A/QtGui");
        assert_eq!(
            parsed.rpath_install_name(),
            "@rpath/QtGui.framework/Versions/A/QtGui"
        );
    }

    #[test]
    fn parses_rpath_qt_install_name() {
        let parsed = parse_framework_ref("@rpath/QtDBus.framework/Versions/A/QtDBus").unwrap();
        assert_eq!(parsed.name, "QtDBus.framework");
        assert_eq!(parsed.relative, "Versions/A/QtDBus");
    }

    #[test]
    fn ignores_plain_dylibs() {
        assert!(parse_framework_ref("@rpath/libcolmap_ui.dylib").is_none());
    }

    #[test]
    fn ignores_system_frameworks() {
        assert!(parse_framework_ref(
            "/System/Library/Frameworks/AppKit.framework/Versions/C/AppKit"
        )
        .is_none());
        assert!(parse_framework_ref("/usr/lib/libSystem.B.dylib").is_none());
        assert!(source_framework_dir(
            "/System/Library/Frameworks/AppKit.framework/Versions/C/AppKit"
        )
        .is_none());
    }

    #[test]
    fn source_framework_dir_cuts_at_framework_suffix() {
        let name = "/opt/homebrew/opt/qtsvg/lib/QtSvg.framework/Versions/A/QtSvg";
        let dir = PathBuf::from("/opt/homebrew/opt/qtsvg/lib/QtSvg.framework");
        if dir.is_dir() {
            assert_eq!(source_framework_dir(name).as_deref(), Some(dir.as_path()));
        } else {
            assert!(source_framework_dir(name).is_none());
        }
    }
}
