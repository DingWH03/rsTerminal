//! Path-bar autocomplete: parse typed paths, list parent dirs in the background.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::TryRecvError;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use rsterm_fs::local;
use rsterm_fs::{FileEntry, home_dir};
use rsterm_session_core::{FileActivePane, PathAutocompleteResultSlot, PathAutocompleteState};

const CACHE_SIZE: usize = 8;
const DEBOUNCE: Duration = Duration::from_millis(80);
const MAX_SUGGESTIONS: usize = 30;

/// Parsed parent directory and trailing name prefix from the path bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAutocomplete {
    pub parent: String,
    pub prefix: String,
}

/// Split `raw` into the directory to list and the incomplete final segment.
pub fn parse_path_input(raw: &str, cwd: &str, remote: bool) -> Option<ParsedAutocomplete> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    if remote {
        parse_remote(raw, cwd)
    } else {
        parse_local(raw, cwd)
    }
}

fn parse_local(raw: &str, cwd: &str) -> Option<ParsedAutocomplete> {
    let (parent_part, prefix) = split_parent_prefix(raw, false);
    let parent = if parent_part.is_empty() {
        if raw.starts_with('/') || raw.starts_with('\\') || raw.starts_with('~') {
            expand_local_segment(raw, cwd)?
        } else {
            PathBuf::from(cwd)
        }
    } else {
        expand_local_segment(&parent_part, cwd)?
    };
    Some(ParsedAutocomplete {
        parent: parent.display().to_string(),
        prefix,
    })
}

fn expand_local_segment(segment: &str, cwd: &str) -> Option<PathBuf> {
    let trimmed = segment.trim_end_matches(['/', '\\']);
    if trimmed.is_empty() {
        return Some(PathBuf::from(if segment.starts_with('/') {
            "/"
        } else {
            cwd
        }));
    }
    let path = if trimmed.starts_with('~') {
        let home = home_dir();
        if trimmed == "~" {
            home
        } else if let Some(rest) = trimmed
            .strip_prefix("~/")
            .or_else(|| trimmed.strip_prefix("~\\"))
        {
            home.join(rest)
        } else {
            PathBuf::from(trimmed)
        }
    } else if Path::new(trimmed).is_absolute() {
        PathBuf::from(trimmed)
    } else {
        PathBuf::from(cwd).join(trimmed)
    };
    Some(path)
}

fn parse_remote(raw: &str, cwd: &str) -> Option<ParsedAutocomplete> {
    let (parent_part, prefix) = split_parent_prefix(raw, true);
    let parent = if parent_part.is_empty() {
        if raw.starts_with('/') {
            "/".to_string()
        } else {
            normalize_remote_path(cwd)
        }
    } else if parent_part.starts_with('/') {
        normalize_remote_path(parent_part.trim_end_matches('/'))
    } else {
        let base = cwd.trim_end_matches('/');
        let rel = parent_part.trim_end_matches('/');
        normalize_remote_path(&format!("{base}/{rel}"))
    };
    Some(ParsedAutocomplete { parent, prefix })
}

fn normalize_remote_path(path: &str) -> String {
    if path.is_empty() || path == "/" {
        return "/".to_string();
    }
    let mut parts = Vec::new();
    for p in path.split('/').filter(|s| !s.is_empty() && *s != ".") {
        if p == ".." {
            parts.pop();
        } else {
            parts.push(p);
        }
    }
    if parts.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", parts.join("/"))
    }
}

fn split_parent_prefix(path: &str, remote: bool) -> (String, String) {
    let sep = if remote {
        path.rfind('/')
    } else {
        path.rfind(['/', '\\'])
    };
    match sep {
        Some(i) => (path[..=i].to_string(), path[i + 1..].to_string()),
        None => (String::new(), path.to_string()),
    }
}

/// Build display strings for autocomplete popup rows.
pub fn build_suggestions(
    entries: &[FileEntry],
    parsed: &ParsedAutocomplete,
    remote: bool,
    show_hidden: bool,
) -> Vec<String> {
    let prefix_lc = parsed.prefix.to_lowercase();
    let mut names: Vec<&str> = entries
        .iter()
        .filter(|e| e.is_dir)
        .filter(|e| show_hidden || !e.name.starts_with('.'))
        .filter(|e| prefix_lc.is_empty() || e.name.to_lowercase().starts_with(&prefix_lc))
        .map(|e| e.name.as_str())
        .collect();
    names.sort_unstable();
    names.truncate(MAX_SUGGESTIONS);
    names
        .into_iter()
        .map(|n| join_suggestion(&parsed.parent, n, remote))
        .collect()
}

fn join_suggestion(parent: &str, name: &str, remote: bool) -> String {
    if remote {
        if parent == "/" {
            format!("/{name}")
        } else {
            format!("{parent}/{name}")
        }
    } else {
        Path::new(parent).join(name).display().to_string()
    }
}

fn cache_lookup(state: &PathAutocompleteState, parent: &str) -> Option<Vec<FileEntry>> {
    state
        .cache
        .iter()
        .find(|(p, _)| p == parent)
        .map(|(_, e)| e.clone())
}

fn cache_insert(state: &mut PathAutocompleteState, parent: String, entries: Vec<FileEntry>) {
    if let Some(pos) = state.cache.iter().position(|(p, _)| p == &parent) {
        state.cache.remove(pos);
    }
    state.cache.push_front((parent, entries));
    while state.cache.len() > CACHE_SIZE {
        state.cache.pop_back();
    }
}

fn cancel_pending(state: &mut PathAutocompleteState) {
    if let Some(c) = state.cancel.as_ref() {
        c.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    if let Some(handle) = state.local_join.take() {
        let _ = handle.join();
    }
    state.remote_rx = None;
    state.cancel = None;
    state.results = None;
}

/// Schedule a debounced background list for `parent` (only when `input` changed).
pub fn request_path_autocomplete(
    ac: &mut PathAutocompleteState,
    pane: FileActivePane,
    input: &str,
    parsed: &ParsedAutocomplete,
    remote: bool,
    show_hidden: bool,
) {
    if ac.last_request_input == input {
        return;
    }
    ac.last_request_input = input.to_string();
    if ac.active_pane != Some(pane) {
        ac.reset();
        ac.active_pane = Some(pane);
        ac.last_request_input = input.to_string();
    }
    ac.input_generation = ac.input_generation.wrapping_add(1);
    ac.debounce_parent = parsed.parent.clone();
    ac.debounce_generation = ac.input_generation;
    ac.debounce_remote = remote;
    ac.debounce_show_hidden = show_hidden;
    ac.debounce_at = Some(Instant::now() + DEBOUNCE);
    ac.error = None;
}

/// Poll debounce timer and in-flight list operations.
pub fn poll_path_autocomplete(
    ac: &mut PathAutocompleteState,
    pane: FileActivePane,
    remote: bool,
    client: Option<&Arc<rsterm_fs::sftp::SftpClient>>,
) {
    // Debounce: kick fetch when timer fires.
    let kick = ac
        .debounce_at
        .is_some_and(|t| Instant::now() >= t)
        .then(|| {
            (
                ac.debounce_parent.clone(),
                ac.debounce_generation,
                ac.debounce_remote,
                ac.debounce_show_hidden,
            )
        });
    if let Some((parent, generation, debounce_remote, show_hidden)) = kick {
        ac.debounce_at = None;
        kick_fetch(
            ac,
            pane,
            &parent,
            generation,
            debounce_remote,
            show_hidden,
            if remote { client } else { None },
        );
    }

    // Poll remote receiver.
    let remote_done = ac.remote_rx.as_ref().and_then(|rx| rx.try_recv().ok());
    if let Some(result) = remote_done {
        ac.remote_rx = None;
        ac.loading = false;
        apply_fetch_result(ac, result);
    } else if let Some(rx) = ac.remote_rx.as_ref()
        && matches!(rx.try_recv(), Err(TryRecvError::Disconnected))
    {
        ac.remote_rx = None;
        ac.loading = false;
        ac.error = Some("SFTP list failed".into());
    }

    // Poll local thread.
    let local_ready = ac
        .results
        .as_ref()
        .and_then(|results| results.lock().ok())
        .is_some_and(|g| g.is_some());
    if local_ready {
        if let Some(handle) = ac.local_join.take() {
            let _ = handle.join();
        }
        let result = ac
            .results
            .as_ref()
            .and_then(|r| r.lock().ok())
            .and_then(|mut g| g.take());
        ac.results = None;
        ac.cancel = None;
        ac.loading = false;
        if let Some(result) = result {
            apply_fetch_result(ac, result);
        }
    }
}

fn kick_fetch(
    ac: &mut PathAutocompleteState,
    pane: FileActivePane,
    parent: &str,
    generation: u64,
    remote: bool,
    show_hidden: bool,
    client: Option<&Arc<rsterm_fs::sftp::SftpClient>>,
) {
    if ac.parent == parent && !ac.loading && ac.error.is_none() && ac.generation == generation {
        return;
    }

    if let Some(cached) = cache_lookup(ac, parent) {
        ac.parent = parent.to_string();
        ac.generation = generation;
        ac.entries = cached;
        ac.error = None;
        ac.loading = false;
        ac.active_pane = Some(pane);
        let _ = show_hidden;
        return;
    }

    cancel_pending(ac);
    ac.parent = parent.to_string();
    ac.generation = generation;
    ac.entries.clear();
    ac.error = None;
    ac.loading = true;
    ac.active_pane = Some(pane);
    ac.pending_generation = generation;

    if remote {
        let Some(client) = client else {
            ac.loading = false;
            ac.error = Some("Remote not connected".into());
            return;
        };
        match client.begin_list_dir(parent) {
            Ok(rx) => {
                ac.remote_rx = Some(rx);
            }
            Err(e) => {
                ac.loading = false;
                ac.error = Some(e);
            }
        }
    } else {
        let parent_path = PathBuf::from(parent);
        let cancel = Arc::new(AtomicBool::new(false));
        let results: PathAutocompleteResultSlot = Arc::new(Mutex::new(None));
        let cancel_t = Arc::clone(&cancel);
        let results_t = Arc::clone(&results);
        let join = thread::spawn(move || {
            if cancel_t.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }
            let out = local::list_dir(&parent_path);
            if cancel_t.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }
            if let Ok(mut g) = results_t.lock() {
                *g = Some(out);
            }
        });
        ac.cancel = Some(cancel);
        ac.results = Some(results);
        ac.local_join = Some(join);
    }
}

fn apply_fetch_result(ac: &mut PathAutocompleteState, result: Result<Vec<FileEntry>, String>) {
    match result {
        Ok(entries) => {
            cache_insert(ac, ac.parent.clone(), entries.clone());
            if ac.pending_generation == ac.generation {
                ac.entries = entries;
                ac.error = None;
            }
        }
        Err(_) => {
            if ac.pending_generation == ac.generation {
                ac.entries.clear();
                // Parent may not exist yet while the user is still typing.
                ac.error = None;
            }
        }
    }
}

/// Cancel in-flight autocomplete (blur, submit, pane switch).
pub fn cancel_path_autocomplete(ac: &mut PathAutocompleteState) {
    ac.reset();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_local_absolute_parent() {
        let p = parse_path_input("/usr/loc", "/home", false).unwrap();
        assert_eq!(p.parent, "/usr");
        assert_eq!(p.prefix, "loc");
    }

    #[test]
    fn parse_local_tilde() {
        let home = home_dir().display().to_string();
        let p = parse_path_input("~/Doc", "/tmp", false).unwrap();
        assert_eq!(p.parent, home);
        assert_eq!(p.prefix, "Doc");
    }

    #[test]
    fn parse_remote_other_dir() {
        let p = parse_path_input("/var/log/s", "/home/user", true).unwrap();
        assert_eq!(p.parent, "/var/log");
        assert_eq!(p.prefix, "s");
    }

    #[test]
    fn parse_remote_relative() {
        let p = parse_path_input("subdir/foo", "/home/user", true).unwrap();
        assert_eq!(p.parent, "/home/user/subdir");
        assert_eq!(p.prefix, "foo");
    }
}
