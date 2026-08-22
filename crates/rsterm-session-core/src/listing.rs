//! File listing sort / filter helpers for file-manager panes.

use std::cmp::Ordering;

use regex::Regex;
use rsterm_fs::FileEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileSortKey {
    #[default]
    Name,
    Size,
    Modified,
}

/// In-memory / recursive name filter options.
#[derive(Debug, Clone, Default)]
pub struct ListingFilter {
    pub query: String,
    pub case_sensitive: bool,
    pub regex: bool,
    pub show_hidden: bool,
}

impl ListingFilter {
    pub fn from_legacy(filter: &str, show_hidden: bool) -> Self {
        Self {
            query: filter.to_string(),
            case_sensitive: false,
            regex: false,
            show_hidden,
        }
    }
}

/// Returns whether `name` matches the filter query (ignores show_hidden).
pub fn name_matches(name: &str, filter: &ListingFilter) -> bool {
    let q = filter.query.trim();
    if q.is_empty() {
        return true;
    }
    if filter.regex {
        let re = if filter.case_sensitive {
            Regex::new(q)
        } else {
            Regex::new(&format!("(?i){q}"))
        };
        match re {
            Ok(re) => re.is_match(name),
            Err(_) => false,
        }
    } else if filter.case_sensitive {
        name.contains(q)
    } else {
        name.to_lowercase().contains(&q.to_lowercase())
    }
}

/// Rebuild display `entries` from `all_entries` using filter / sort options.
pub fn recompute_entries(
    all: &[FileEntry],
    filter: &ListingFilter,
    sort_key: FileSortKey,
    sort_asc: bool,
) -> Vec<FileEntry> {
    let mut out: Vec<FileEntry> = all
        .iter()
        .filter(|e| {
            if !filter.show_hidden && e.name.starts_with('.') {
                return false;
            }
            // Match against the file basename (last path segment) for nested names.
            let base = e.name.rsplit(['/', '\\']).next().unwrap_or(e.name.as_str());
            name_matches(base, filter)
        })
        .cloned()
        .collect();

    sort_entries(&mut out, sort_key, sort_asc);
    out
}

pub fn sort_entries(out: &mut [FileEntry], sort_key: FileSortKey, sort_asc: bool) {
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
        let filter = ListingFilter::from_legacy("", false);
        let out = recompute_entries(&all, &filter, FileSortKey::Name, true);
        assert_eq!(out.len(), 2);
        assert!(out[0].is_dir);
    }

    #[test]
    fn regex_and_case() {
        let filter = ListingFilter {
            query: r"^foo".into(),
            case_sensitive: false,
            regex: true,
            show_hidden: true,
        };
        assert!(name_matches("FooBar", &filter));
        assert!(!name_matches("barFoo", &filter));
    }
}
