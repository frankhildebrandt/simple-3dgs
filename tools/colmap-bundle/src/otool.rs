//! Parse `otool -L` and `otool -l` text. The Mach-O tools stay C; this crate only drives them.

pub fn parse_install_names(otool_l_stdout: &str) -> Vec<String> {
    otool_l_stdout
        .lines()
        .skip(1)
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            Some(trimmed.split_whitespace().next()?.to_string())
        })
        .collect()
}

pub fn parse_rpaths(otool_load_stdout: &str) -> Vec<String> {
    let lines: Vec<&str> = otool_load_stdout.lines().collect();
    let mut found = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if !line.contains("LC_RPATH") {
            continue;
        }
        if let Some(path_line) = lines.get(i + 2) {
            if let Some(path) = parse_rpath_line(path_line) {
                found.push(path);
            }
        }
    }
    found
}

fn parse_rpath_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("path ")?;
    let path = rest.split(" (").next()?.trim();
    if path.is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OTOOL_L: &str = "\
colmap:
\t@rpath/libcolmap_ui.dylib (compatibility version 0.0.0, current version 0.0.0)
\t/opt/homebrew/opt/qtsvg/lib/QtSvg.framework/Versions/A/QtSvg (compatibility version 6.0.0, current version 6.11.1)
\t/System/Library/Frameworks/AppKit.framework/Versions/C/AppKit (compatibility version 45.0.0, current version 2685.60.104)
";

    const OTOOL_LOAD: &str = "\
          cmd LC_RPATH
      cmdsize 32
         path @rpath/ (offset 12)
          cmd LC_RPATH
      cmdsize 48
         path @executable_path/colmap-libs (offset 12)
";

    #[test]
    fn parse_install_names_skips_header_and_versions() {
        let names = parse_install_names(OTOOL_L);
        assert_eq!(
            names,
            vec![
                "@rpath/libcolmap_ui.dylib",
                "/opt/homebrew/opt/qtsvg/lib/QtSvg.framework/Versions/A/QtSvg",
                "/System/Library/Frameworks/AppKit.framework/Versions/C/AppKit",
            ]
        );
    }

    #[test]
    fn parse_rpaths_reads_path_field() {
        assert_eq!(
            parse_rpaths(OTOOL_LOAD),
            vec!["@rpath/", "@executable_path/colmap-libs"]
        );
    }
}
