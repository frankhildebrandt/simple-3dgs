//! Decide which LC_RPATH entries to drop. Duplicate `@rpath/` makes dyld abort.

/// Returns rpath values that should be deleted, in the order `install_name_tool` must see them.
pub fn rpaths_to_delete(existing: &[String]) -> Vec<String> {
    let mut keep: Vec<&str> = Vec::new();
    let mut delete = Vec::new();
    for entry in existing {
        if entry == "@rpath/" || keep.contains(&entry.as_str()) {
            delete.push(entry.clone());
        } else {
            keep.push(entry);
        }
    }
    delete
}

/// Returns wanted rpaths that are not already present, in the given order.
pub fn rpaths_to_add(existing: &[String], wanted: &[&str]) -> Vec<String> {
    wanted
        .iter()
        .filter(|entry| !existing.iter().any(|have| have == *entry))
        .map(|entry| (*entry).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{rpaths_to_add, rpaths_to_delete};

    #[test]
    fn drops_duplicate_and_bare_rpath() {
        let existing = vec![
            "@rpath/".into(),
            "@rpath/".into(),
            "@executable_path/colmap-libs".into(),
            "@executable_path/colmap-libs".into(),
            "@loader_path".into(),
        ];
        assert_eq!(
            rpaths_to_delete(&existing),
            vec![
                "@rpath/",
                "@rpath/",
                "@executable_path/colmap-libs",
            ]
        );
    }

    #[test]
    fn keeps_unique_real_rpaths() {
        let existing = vec![
            "@executable_path/colmap-libs".into(),
            "@executable_path/../Resources/colmap-libs".into(),
        ];
        assert!(rpaths_to_delete(&existing).is_empty());
    }

    #[test]
    fn adds_only_missing_rpaths() {
        let existing = vec!["@executable_path/colmap-libs".into()];
        assert_eq!(
            rpaths_to_add(
                &existing,
                &[
                    "@executable_path/colmap-libs",
                    "@executable_path/../Resources/colmap-libs",
                ]
            ),
            vec!["@executable_path/../Resources/colmap-libs"]
        );
    }
}
