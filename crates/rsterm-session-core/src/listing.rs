//! File listing sort / filter helpers for file-manager panes.

use std::cmp::Ordering;

use rsterm_fs::FileEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileSortKey {
    #[default]
    Name,
    Size,
    Modified,
}

/// Rebuild display `entries` from `all_entries` using filter / sort options.
pub fn recompute_entries(
    all: &[FileEntry],
    filter: &str,
    show_hidden: bool,
    sort_key: FileSortKey,
    sort_asc: bool,
) -> Vec<FileEntry> {
    let filter_lc = filter.trim().to_lowercase();
    let mut out: Vec<FileEntry> = all
        .iter()
        .filter(|e| {
            if !show_hidden && e.name.starts_with('.') {
                return false;
            }
            if filter_lc.is_empty() {
                return true;
            }
            e.name.to_lowercase().contains(&filter_lc)
        })
        .cloned()
        .collect();

    out.sort_by(|a, b| {
        let dir_ord = a.is_dir.cmp(&b.is_dir).reverse();
        if dir_ord != Ordering::Equal {
            return dir_ord;
        }
        let primary = match sort_key {
            FileSortKey::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            FileSortKey::Size => a.size.cmp(&b.size),
            FileSortKey::Modified => a.modified.cmp(&b.modified),
        };
        if sort_asc { primary } else { primary.reverse() }
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsterm_fs::FileEntry;
    #[test]
    fn recompute_basic() {
        let all = vec![
            FileEntry {
                name: "b".into(),
                is_dir: false,
                size: 2,
                modified: None,
            },
            FileEntry {
                name: "a".into(),
                is_dir: true,
                size: 0,
                modified: None,
            },
            FileEntry {
                name: ".h".into(),
                is_dir: false,
                size: 1,
                modified: None,
            },
        ];
        let out = recompute_entries(&all, "", false, FileSortKey::Name, true);
        assert_eq!(out.len(), 2);
        assert!(out[0].is_dir);
    }
}
