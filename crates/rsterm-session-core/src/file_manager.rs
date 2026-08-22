//! File-manager workspace session state.

use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use rsterm_fs::sftp::SftpClient;
use rsterm_fs::{FileEntry, home_dir};

use crate::listing::{FileSortKey, ListingFilter, recompute_entries};

/// 粘贴目标面板。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasteTarget {
    /// 右侧本地面板
    LocalRight,
    /// 左侧本地面板
    LocalLeft,
    /// 远程 SFTP 面板
    Remote,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PaneSide {
    Left,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FileClipboardMode {
    Copy,
    Cut,
}

#[derive(Clone)]
pub struct FileClipboard {
    pub mode: FileClipboardMode,
    pub from_remote: bool,
    pub paths: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FileManagerMode {
    /// SSH: left remote SFTP, right local disk.
    SshSftp,
    /// Local: both panes use the local filesystem.
    LocalDual,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum FileActivePane {
    #[default]
    Right,
    Remote,
    LeftLocal,
}

pub struct FilePaneState {
    pub cwd: PathBuf,
    /// Editable path bar buffer (synced from `cwd` when not focused).
    pub path_edit: String,
    /// Last `list_dir` result (unfiltered).
    pub all_entries: Vec<FileEntry>,
    /// Derived display list (filter + sort).
    pub entries: Vec<FileEntry>,
    pub selected: HashSet<usize>,
    pub select_mode: bool,
    pub focus_index: Option<usize>,
    pub error: Option<String>,
    pub loading: bool,
    pub sort_key: FileSortKey,
    pub sort_asc: bool,
    pub filter: String,
    pub show_hidden: bool,
    pub filter_case_sensitive: bool,
    pub filter_regex: bool,
    pub filter_recursive: bool,
}

/// Compatibility alias for the pre-phase-6 public file-manager path.
pub use FilePaneState as PaneState;

impl FilePaneState {
    pub fn new_local(start: PathBuf) -> Self {
        let path_edit = start.display().to_string();
        Self {
            cwd: start,
            path_edit,
            all_entries: Vec::new(),
            entries: Vec::new(),
            selected: HashSet::new(),
            select_mode: false,
            focus_index: None,
            error: None,
            loading: true,
            sort_key: FileSortKey::Name,
            sort_asc: true,
            filter: String::new(),
            show_hidden: false,
            filter_case_sensitive: false,
            filter_regex: false,
            filter_recursive: false,
        }
    }

    pub fn listing_filter(&self) -> ListingFilter {
        ListingFilter {
            query: self.filter.clone(),
            case_sensitive: self.filter_case_sensitive,
            regex: self.filter_regex,
            show_hidden: self.show_hidden,
        }
    }

    pub fn sync_path_edit_from_cwd(&mut self) {
        self.path_edit = self.cwd.display().to_string();
    }

    pub fn apply_listing(&mut self, entries: Vec<FileEntry>) {
        self.all_entries = entries;
        self.sync_path_edit_from_cwd();
        self.recompute();
    }

    pub fn recompute(&mut self) {
        if self.filter_recursive && !self.filter.trim().is_empty() {
            // Recursive results are applied asynchronously; keep current entries until then.
            return;
        }
        self.entries = recompute_entries(
            &self.all_entries,
            &self.listing_filter(),
            self.sort_key,
            self.sort_asc,
        );
        self.selected.clear();
        self.focus_index = None;
    }
}

pub use rsterm_fs::TransferSnapshot;

/// Queued or in-flight paste/transfer job.
#[derive(Clone)]
pub struct TransferJob {
    pub id: u64,
    pub label: String,
    pub target: PasteTarget,
    pub clip: FileClipboard,
    pub dest_local: Option<PathBuf>,
    pub remote_cwd: Option<String>,
    pub remote_client: Option<Arc<SftpClient>>,
}

pub struct FileTransferState {
    pub cancel: Arc<AtomicBool>,
    pub snapshot: Arc<Mutex<TransferSnapshot>>,
    pub join: Option<JoinHandle<()>>,
    pub queue: VecDeque<TransferJob>,
    pub current: Option<TransferJob>,
    pub last_failed: Option<TransferJob>,
    pub(crate) next_id: u64,
}

impl Default for FileTransferState {
    fn default() -> Self {
        Self {
            cancel: Arc::new(AtomicBool::new(false)),
            snapshot: Arc::new(Mutex::new(TransferSnapshot::default())),
            join: None,
            queue: VecDeque::new(),
            current: None,
            last_failed: None,
            next_id: 1,
        }
    }
}

pub struct RemotePane {
    pub client: Arc<SftpClient>,
    pub cwd: String,
    /// Editable path bar buffer.
    pub path_edit: String,
    pub all_entries: Vec<FileEntry>,
    pub entries: Vec<FileEntry>,
    pub selected: HashSet<usize>,
    pub select_mode: bool,
    pub focus_index: Option<usize>,
    pub error: Option<String>,
    pub loading: bool,
    pub sort_key: FileSortKey,
    pub sort_asc: bool,
    pub filter: String,
    pub show_hidden: bool,
    pub filter_case_sensitive: bool,
    pub filter_regex: bool,
    pub filter_recursive: bool,
}

impl RemotePane {
    pub fn listing_filter(&self) -> ListingFilter {
        ListingFilter {
            query: self.filter.clone(),
            case_sensitive: self.filter_case_sensitive,
            regex: self.filter_regex,
            show_hidden: self.show_hidden,
        }
    }

    pub fn sync_path_edit_from_cwd(&mut self) {
        self.path_edit = self.cwd.clone();
    }

    pub fn apply_listing(&mut self, entries: Vec<FileEntry>) {
        self.all_entries = entries;
        self.sync_path_edit_from_cwd();
        self.recompute();
    }

    pub fn recompute(&mut self) {
        if self.filter_recursive && !self.filter.trim().is_empty() {
            return;
        }
        self.entries = recompute_entries(
            &self.all_entries,
            &self.listing_filter(),
            self.sort_key,
            self.sort_asc,
        );
        self.selected.clear();
        self.focus_index = None;
    }
}

#[derive(Clone, Default)]
pub struct RenameDialog {
    pub open: bool,
    pub pane: FileActivePane,
    pub new_name: String,
    old_name: String,
}

impl RenameDialog {
    pub fn open_for(&mut self, pane: FileActivePane, name: &str) {
        self.open = true;
        self.pane = pane;
        self.old_name = name.to_string();
        self.new_name = name.to_string();
    }

    pub fn old_name(&self) -> &str {
        &self.old_name
    }
}

#[derive(Clone)]
pub struct InfoLine(pub String, pub String);

#[derive(Clone, Default)]
pub struct InfoDialog {
    pub open: bool,
    pub lines: Vec<InfoLine>,
}

impl InfoDialog {
    pub fn show(&mut self, info: rsterm_fs::entry_info::EntryInfo) {
        use rsterm_fs::transfer_progress::format_bytes;
        self.open = true;
        self.lines = vec![
            InfoLine("Name".into(), info.name),
            InfoLine("Type".into(), info.kind),
            InfoLine("Size".into(), format_bytes(info.size)),
            InfoLine("Permissions".into(), info.permissions),
            InfoLine("Modified".into(), info.modified),
            InfoLine("Path".into(), info.path),
        ];
    }
}

/// Background recursive name search for the active pane.
pub type RecursiveSearchResult = Result<Vec<FileEntry>, String>;

/// Shared slot for a background path-autocomplete directory listing.
pub type PathAutocompleteResultSlot = Arc<Mutex<Option<Result<Vec<FileEntry>, String>>>>;

/// Background directory listing for path-bar autocomplete.
#[derive(Default)]
pub struct PathAutocompleteState {
    pub active_pane: Option<FileActivePane>,
    pub parent: String,
    pub generation: u64,
    pub loading: bool,
    pub entries: Vec<FileEntry>,
    pub error: Option<String>,
    pub input_generation: u64,
    pub debounce_parent: String,
    pub debounce_generation: u64,
    pub debounce_remote: bool,
    pub debounce_show_hidden: bool,
    pub debounce_at: Option<std::time::Instant>,
    pub cache: std::collections::VecDeque<(String, Vec<FileEntry>)>,
    pub cancel: Option<Arc<AtomicBool>>,
    pub results: Option<PathAutocompleteResultSlot>,
    pub local_join: Option<JoinHandle<()>>,
    pub remote_rx: Option<std::sync::mpsc::Receiver<Result<Vec<FileEntry>, String>>>,
    pub pending_generation: u64,
    /// Last `path_edit` value that triggered a debounced fetch.
    pub last_request_input: String,
}

impl PathAutocompleteState {
    pub fn reset(&mut self) {
        if let Some(c) = self.cancel.as_ref() {
            c.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        if let Some(handle) = self.local_join.take() {
            let _ = handle.join();
        }
        self.remote_rx = None;
        self.cancel = None;
        self.results = None;
        self.loading = false;
        self.entries.clear();
        self.error = None;
        self.parent.clear();
        self.active_pane = None;
        self.debounce_at = None;
        self.debounce_parent.clear();
        self.last_request_input.clear();
    }
}

pub struct RecursiveSearchState {
    pub pane: FileActivePane,
    pub cancel: Arc<AtomicBool>,
    pub results: Arc<Mutex<Option<RecursiveSearchResult>>>,
    pub join: Option<JoinHandle<()>>,
}

impl RecursiveSearchState {
    pub fn take_if_done(&mut self) -> Option<RecursiveSearchResult> {
        let ready = {
            let guard = self.results.lock().ok()?;
            guard.is_some()
        };
        if !ready {
            return None;
        }
        if let Some(handle) = self.join.take() {
            let _ = handle.join();
        }
        self.results.lock().ok().and_then(|mut g| g.take())
    }

    pub fn request_cancel(&self) {
        self.cancel
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn is_running(&self) -> bool {
        self.join.is_some() && self.results.lock().map(|g| g.is_none()).unwrap_or(false)
    }
}

pub struct FileManagerSession {
    pub id: String,
    pub title: String,
    /// Saved SSH profile id (for sidebar「新窗口」).
    pub saved_conn_id: Option<String>,
    pub mode: FileManagerMode,
    pub remote: Option<RemotePane>,
    /// Left pane when `mode == LocalDual`.
    pub left_local: Option<FilePaneState>,
    pub right: FilePaneState,
    pub clipboard: Option<FileClipboard>,
    pub status: Option<String>,
    pub rename_dialog: RenameDialog,
    pub info_dialog: InfoDialog,
    pub transfer: FileTransferState,
    /// Anchor index for shift-range selection per pane.
    pub local_anchor: Option<usize>,
    pub right_anchor: Option<usize>,
    pub remote_anchor: Option<usize>,
    pub active_pane: FileActivePane,
    pub recursive_search: Option<RecursiveSearchState>,
    pub path_autocomplete: PathAutocompleteState,
}

impl FileManagerSession {
    /// Sidebar label: left pane path only (remote for SFTP, left local for dual-local).
    pub fn tab_label(&self) -> String {
        match self.mode {
            FileManagerMode::SshSftp => {
                let host = self
                    .title
                    .strip_prefix("Remote: ")
                    .unwrap_or(self.title.as_str());
                self.remote
                    .as_ref()
                    .map(|r| format!("{host}:{}", r.cwd))
                    .unwrap_or_else(|| self.title.clone())
            }
            FileManagerMode::LocalDual => self
                .left_local
                .as_ref()
                .map(|p| p.cwd.display().to_string())
                .unwrap_or_else(|| "File Manager".to_string()),
        }
    }

    pub fn open_ssh(
        host: &str,
        port: u16,
        auth: rsterm_connection::ssh_auth::ResolvedSshAuth,
        saved_conn_id: String,
    ) -> Result<Self, String> {
        let client = SftpClient::connect(host, port, auth, None)?;
        let title = format!("Remote: {host}");
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            saved_conn_id: Some(saved_conn_id),
            mode: FileManagerMode::SshSftp,
            remote: Some(RemotePane {
                client: Arc::new(client),
                cwd: "/".to_string(),
                path_edit: "/".to_string(),
                all_entries: Vec::new(),
                entries: Vec::new(),
                selected: HashSet::new(),
                select_mode: false,
                focus_index: None,
                error: None,
                loading: true,
                sort_key: FileSortKey::Name,
                sort_asc: true,
                filter: String::new(),
                show_hidden: false,
                filter_case_sensitive: false,
                filter_regex: false,
                filter_recursive: false,
            }),
            left_local: None,
            right: FilePaneState::new_local(home_dir()),
            clipboard: None,
            status: None,
            rename_dialog: RenameDialog::default(),
            info_dialog: InfoDialog::default(),
            transfer: FileTransferState::default(),
            local_anchor: None,
            right_anchor: None,
            remote_anchor: None,
            active_pane: FileActivePane::Remote,
            recursive_search: None,
            path_autocomplete: PathAutocompleteState::default(),
        })
    }

    pub fn open_local() -> Self {
        let home = home_dir();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: "File Manager".to_string(),
            saved_conn_id: None,
            mode: FileManagerMode::LocalDual,
            remote: None,
            left_local: Some(FilePaneState::new_local(home.clone())),
            right: FilePaneState::new_local(home),
            clipboard: None,
            status: None,
            rename_dialog: RenameDialog::default(),
            info_dialog: InfoDialog::default(),
            transfer: FileTransferState::default(),
            local_anchor: None,
            right_anchor: None,
            remote_anchor: None,
            active_pane: FileActivePane::LeftLocal,
            recursive_search: None,
            path_autocomplete: PathAutocompleteState::default(),
        }
    }
}
