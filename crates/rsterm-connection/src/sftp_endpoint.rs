use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use super::RepaintNotifier;

#[derive(Clone)]
pub enum SftpStatus {
    Connecting,
    Connected,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct SftpDirEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

impl SftpDirEntry {
    pub fn sort_key(&self) -> (u8, String) {
        (if self.is_dir { 0 } else { 1 }, self.name.to_lowercase())
    }
}

#[derive(Clone, Debug)]
pub struct SftpStatInfo {
    pub path: String,
    pub name: String,
    pub kind: String,
    pub size: u64,
    pub permissions: String,
    pub modified: String,
}

pub trait SftpProgress: Send + Sync {
    fn is_cancelled(&self) -> bool;
    fn add_bytes(&self, n: u64, label: &str);
    fn set_label(&self, label: &str);
}

pub enum SftpRequest {
    List {
        path: String,
        reply: mpsc::SyncSender<Result<Vec<SftpDirEntry>, String>>,
    },
    Upload {
        local: std::path::PathBuf,
        remote: String,
        progress: Option<Arc<dyn SftpProgress>>,
        label: String,
        reply: mpsc::SyncSender<Result<(), String>>,
    },
    Download {
        remote: String,
        local: std::path::PathBuf,
        progress: Option<Arc<dyn SftpProgress>>,
        label: String,
        reply: mpsc::SyncSender<Result<(), String>>,
    },
    Stat {
        path: String,
        reply: mpsc::SyncSender<Result<SftpStatInfo, String>>,
    },
    PathBytes {
        path: String,
        reply: mpsc::SyncSender<Result<u64, String>>,
    },
    Remove {
        path: String,
        is_dir: bool,
        reply: mpsc::SyncSender<Result<(), String>>,
    },
    Mkdir {
        path: String,
        reply: mpsc::SyncSender<Result<(), String>>,
    },
    Rename {
        from: String,
        to: String,
        reply: mpsc::SyncSender<Result<(), String>>,
    },
    Home {
        reply: mpsc::SyncSender<Result<String, String>>,
    },
    /// Write raw bytes (agent script deploy on a shared session).
    #[allow(dead_code)]
    WriteBytes {
        remote: String,
        data: Vec<u8>,
        reply: mpsc::SyncSender<Result<(), String>>,
    },
    Shutdown,
}

pub struct SftpEndpoint {
    pub request_tx: mpsc::Sender<SftpRequest>,
    pub status: Arc<Mutex<SftpStatus>>,
    pub repaint: RepaintNotifier,
}

impl SftpEndpoint {
    pub fn new(repaint: RepaintNotifier) -> (Self, mpsc::Receiver<SftpRequest>) {
        let (request_tx, request_rx) = mpsc::channel();
        let status = new_status_handle();
        (
            Self {
                request_tx,
                status,
                repaint,
            },
            request_rx,
        )
    }
}

pub fn new_status_handle() -> Arc<Mutex<SftpStatus>> {
    Arc::new(Mutex::new(SftpStatus::Connecting))
}

pub fn mark_connected(status: &Arc<Mutex<SftpStatus>>) {
    *status.lock().unwrap() = SftpStatus::Connected;
}

pub fn mark_error(status: &Arc<Mutex<SftpStatus>>, msg: impl Into<String>) {
    *status.lock().unwrap() = SftpStatus::Error(msg.into());
}

pub fn reply_sftp_gone(req: SftpRequest) {
    let err = "shared SFTP unavailable";
    match req {
        SftpRequest::List { reply, .. } => {
            let _ = reply.send(Err(err.into()));
        }
        SftpRequest::Upload { reply, .. }
        | SftpRequest::Download { reply, .. }
        | SftpRequest::Remove { reply, .. }
        | SftpRequest::Mkdir { reply, .. }
        | SftpRequest::Rename { reply, .. }
        | SftpRequest::WriteBytes { reply, .. } => {
            let _ = reply.send(Err(err.into()));
        }
        SftpRequest::Stat { reply, .. } => {
            let _ = reply.send(Err(err.into()));
        }
        SftpRequest::PathBytes { reply, .. } => {
            let _ = reply.send(Err(err.into()));
        }
        SftpRequest::Home { reply } => {
            let _ = reply.send(Err(err.into()));
        }
        SftpRequest::Shutdown => {}
    }
}
