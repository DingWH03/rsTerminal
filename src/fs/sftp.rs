use std::future::Future;
use std::path::Path;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use russh::client::{self, Handle, KeyboardInteractiveAuthResponse};
use russh::keys::{decode_secret_key, load_secret_key, PrivateKeyWithHashAlg};
use russh_sftp::client::SftpSession;
use tokio::time::timeout;

use crate::connection::sftp_endpoint::{
    mark_connected, mark_error, SftpDirEntry, SftpEndpoint, SftpProgress, SftpRequest, SftpStatInfo,
    SftpStatus,
};
use crate::connection::sftp_mux;
use crate::connection::ssh_auth::ResolvedSshAuth;
use crate::connection::ssh_keys;
use crate::connection::RepaintNotifier;
use crate::fs::entry_info::EntryInfo;
use crate::fs::transfer_progress::ByteProgress;
use crate::fs::FileEntry;

pub struct SftpClient {
    req_tx: mpsc::Sender<SftpRequest>,
    _thread: JoinHandle<()>,
    status: Arc<Mutex<SftpStatus>>,
    /// Kept so callers can refresh the notifier context later if needed.
    #[allow(dead_code)]
    repaint: Option<RepaintNotifier>,
}

struct SftpSshClient;

impl client::Handler for SftpSshClient {
    type Error = russh::Error;

    fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKey,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        async { Ok(true) }
    }
}

impl SftpProgress for ByteProgress {
    fn is_cancelled(&self) -> bool {
        ByteProgress::is_cancelled(self)
    }

    fn add_bytes(&self, n: u64, label: &str) {
        ByteProgress::add_bytes(self, n, label)
    }

    fn set_label(&self, label: &str) {
        ByteProgress::set_label(self, label)
    }
}

fn file_entry_from(e: SftpDirEntry) -> FileEntry {
    FileEntry {
        name: e.name,
        is_dir: e.is_dir,
        size: e.size,
        modified: e.modified,
    }
}

fn entry_info_from(s: SftpStatInfo) -> EntryInfo {
    EntryInfo {
        path: s.path,
        name: s.name,
        kind: s.kind,
        size: s.size,
        permissions: s.permissions,
        modified: s.modified,
    }
}

fn as_progress(p: Option<Arc<ByteProgress>>) -> Option<Arc<dyn SftpProgress>> {
    p.map(|p| p as Arc<dyn SftpProgress>)
}

impl SftpClient {
    pub fn connect(
        host: impl Into<String>,
        port: u16,
        auth: ResolvedSshAuth,
        repaint: Option<RepaintNotifier>,
    ) -> Result<Self, String> {
        let host = host.into();
        if auth.username.is_empty() {
            return Err("SSH user not configured".to_string());
        }

        let (req_tx, req_rx) = mpsc::channel();
        let status: Arc<Mutex<SftpStatus>> = Arc::new(Mutex::new(SftpStatus::Connecting));
        let thread_status = status.clone();
        let thread_repaint = repaint.clone();
        let thread = thread::spawn(move || {
            if let Err(e) = sftp_worker(
                &host,
                port,
                auth,
                req_rx,
                thread_status,
                thread_repaint,
            ) {
                log::error!("SFTP worker ended: {e}");
            }
        });

        Ok(Self {
            req_tx,
            _thread: thread,
            status,
            repaint,
        })
    }

    /// Wrap a shared-session SFTP endpoint from `connection::ssh`.
    pub fn from_endpoint(endpoint: SftpEndpoint) -> Self {
        Self {
            req_tx: endpoint.request_tx,
            _thread: thread::spawn(|| {}),
            status: endpoint.status,
            repaint: Some(endpoint.repaint),
        }
    }

    pub fn is_connected(&self) -> bool {
        matches!(*self.status.lock().unwrap(), SftpStatus::Connected)
    }

    pub fn is_connecting(&self) -> bool {
        matches!(*self.status.lock().unwrap(), SftpStatus::Connecting)
    }

    pub fn connection_error(&self) -> Option<String> {
        match &*self.status.lock().unwrap() {
            SftpStatus::Error(e) => Some(e.clone()),
            _ => None,
        }
    }

    fn call_unit(
        &self,
        build: impl FnOnce(mpsc::SyncSender<Result<(), String>>) -> SftpRequest,
    ) -> Result<(), String> {
        let rx = self.begin_unit(build)?;
        match rx.recv_timeout(Duration::from_secs(120)) {
            Ok(r) => r,
            Err(_) => Err("SFTP operation timed out".to_string()),
        }
    }

    fn begin_unit(
        &self,
        build: impl FnOnce(mpsc::SyncSender<Result<(), String>>) -> SftpRequest,
    ) -> Result<mpsc::Receiver<Result<(), String>>, String> {
        let (tx, rx) = mpsc::sync_channel(1);
        self.req_tx
            .send(build(tx))
            .map_err(|_| "SFTP thread stopped".to_string())?;
        Ok(rx)
    }

    fn begin_string(
        &self,
        build: impl FnOnce(mpsc::SyncSender<Result<String, String>>) -> SftpRequest,
    ) -> Result<mpsc::Receiver<Result<String, String>>, String> {
        let (tx, rx) = mpsc::sync_channel(1);
        self.req_tx
            .send(build(tx))
            .map_err(|_| "SFTP thread stopped".to_string())?;
        Ok(rx)
    }

    pub fn begin_home_dir(&self) -> Result<mpsc::Receiver<Result<String, String>>, String> {
        self.begin_string(|reply| SftpRequest::Home { reply })
    }

    pub fn begin_list_dir(
        &self,
        path: &str,
    ) -> Result<mpsc::Receiver<Result<Vec<FileEntry>, String>>, String> {
        let (tx, rx) = mpsc::sync_channel(1);
        let (inner_tx, inner_rx) = mpsc::sync_channel(1);
        self.req_tx
            .send(SftpRequest::List {
                path: path.to_string(),
                reply: inner_tx,
            })
            .map_err(|_| "SFTP thread stopped".to_string())?;
        thread::spawn(move || {
            let mapped = match inner_rx.recv() {
                Ok(Ok(entries)) => Ok(entries.into_iter().map(file_entry_from).collect()),
                Ok(Err(e)) => Err(e),
                Err(_) => Err("SFTP thread stopped".into()),
            };
            let _ = tx.send(mapped);
        });
        Ok(rx)
    }

    pub fn home_dir(&self) -> Result<String, String> {
        let rx = self.begin_home_dir()?;
        match rx.recv_timeout(Duration::from_secs(120)) {
            Ok(r) => r,
            Err(_) => Err("SFTP operation timed out".to_string()),
        }
    }

    pub fn list_dir(&self, path: &str) -> Result<Vec<FileEntry>, String> {
        let rx = self.begin_list_dir(path)?;
        match rx.recv_timeout(Duration::from_secs(120)) {
            Ok(r) => r,
            Err(_) => Err("SFTP operation timed out".to_string()),
        }
    }

    pub fn upload(&self, local: &Path, remote: &str) -> Result<(), String> {
        self.upload_with_progress(local, remote, None, "")
    }

    pub fn upload_with_progress(
        &self,
        local: &Path,
        remote: &str,
        progress: Option<Arc<ByteProgress>>,
        label: &str,
    ) -> Result<(), String> {
        self.call_unit(|reply| SftpRequest::Upload {
            local: local.to_path_buf(),
            remote: remote.to_string(),
            progress: as_progress(progress),
            label: label.to_string(),
            reply,
        })
    }

    pub fn download(&self, remote: &str, local: &Path) -> Result<(), String> {
        self.download_with_progress(remote, local, None, "")
    }

    pub fn download_with_progress(
        &self,
        remote: &str,
        local: &Path,
        progress: Option<Arc<ByteProgress>>,
        label: &str,
    ) -> Result<(), String> {
        self.call_unit(|reply| SftpRequest::Download {
            remote: remote.to_string(),
            local: local.to_path_buf(),
            progress: as_progress(progress),
            label: label.to_string(),
            reply,
        })
    }

    pub fn entry_info(&self, path: &str) -> Result<EntryInfo, String> {
        let (tx, rx) = mpsc::sync_channel(1);
        self.req_tx
            .send(SftpRequest::Stat {
                path: path.to_string(),
                reply: tx,
            })
            .map_err(|_| "SFTP thread stopped".to_string())?;
        match rx.recv_timeout(Duration::from_secs(120)) {
            Ok(Ok(s)) => Ok(entry_info_from(s)),
            Ok(Err(e)) => Err(e),
            Err(_) => Err("SFTP operation timed out".to_string()),
        }
    }

    pub fn path_bytes(&self, path: &str) -> Result<u64, String> {
        let (tx, rx) = mpsc::sync_channel(1);
        self.req_tx
            .send(SftpRequest::PathBytes {
                path: path.to_string(),
                reply: tx,
            })
            .map_err(|_| "SFTP thread stopped".to_string())?;
        match rx.recv_timeout(Duration::from_secs(120)) {
            Ok(r) => r,
            Err(_) => Err("SFTP operation timed out".to_string()),
        }
    }

    pub fn remove(&self, path: &str, is_dir: bool) -> Result<(), String> {
        self.call_unit(|reply| SftpRequest::Remove {
            path: path.to_string(),
            is_dir,
            reply,
        })
    }

    pub fn mkdir(&self, path: &str) -> Result<(), String> {
        self.call_unit(|reply| SftpRequest::Mkdir {
            path: path.to_string(),
            reply,
        })
    }

    pub fn rename(&self, from: &str, to: &str) -> Result<(), String> {
        self.call_unit(|reply| SftpRequest::Rename {
            from: from.to_string(),
            to: to.to_string(),
            reply,
        })
    }
}

impl Drop for SftpClient {
    fn drop(&mut self) {
        let _ = self.req_tx.send(SftpRequest::Shutdown);
    }
}

fn sftp_worker(
    host: &str,
    port: u16,
    auth: ResolvedSshAuth,
    req_rx: mpsc::Receiver<SftpRequest>,
    status: Arc<Mutex<SftpStatus>>,
    repaint: Option<RepaintNotifier>,
) -> Result<(), String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;

    let notify = || {
        if let Some(r) = &repaint {
            r.request_repaint();
        }
    };

    let sftp = match rt.block_on(async {
        timeout(Duration::from_secs(25), connect_sftp(host, port, &auth)).await
    }) {
        Ok(Ok(sftp)) => {
            mark_connected(&status);
            notify();
            sftp
        }
        Ok(Err(e)) => {
            let msg = format!("SFTP connection failed: {e}");
            mark_error(&status, msg.clone());
            notify();
            drain_requests_with_error(&req_rx, &msg);
            return Err(msg);
        }
        Err(_) => {
            let msg = format!("SFTP connection to {host}:{port} timed out (25s)");
            mark_error(&status, msg.clone());
            notify();
            drain_requests_with_error(&req_rx, &msg);
            return Err(msg);
        }
    };
    sftp.set_timeout(120);

    while let Ok(req) = req_rx.recv() {
        if matches!(req, SftpRequest::Shutdown) {
            break;
        }
        rt.block_on(sftp_mux::apply_sftp_request(&sftp, req));
    }
    Ok(())
}

async fn connect_sftp(
    host: &str,
    port: u16,
    auth: &ResolvedSshAuth,
) -> Result<SftpSession, String> {
    let ssh_config = Arc::new(client::Config {
        keepalive_interval: Some(Duration::from_secs(15)),
        keepalive_max: 3,
        inactivity_timeout: Some(Duration::from_secs(180)),
        nodelay: true,
        ..Default::default()
    });
    let mut handle = client::connect(ssh_config, (host, port), SftpSshClient)
        .await
        .map_err(|e| e.to_string())?;

    authenticate(&mut handle, auth).await?;
    sftp_mux::open_sftp_on_handle(&handle).await
}

async fn authenticate(
    handle: &mut Handle<SftpSshClient>,
    auth: &ResolvedSshAuth,
) -> Result<(), String> {
    let user = auth.username.as_str();

    if let Some(pem) = auth
        .private_key_pem
        .as_deref()
        .filter(|p| !p.trim().is_empty())
    {
        let passphrase = auth.key_passphrase.as_deref().filter(|p| !p.is_empty());
        match decode_secret_key(pem, passphrase) {
            Ok(key) => {
                let hash = handle
                    .best_supported_rsa_hash()
                    .await
                    .map_err(|e| e.to_string())?
                    .flatten();
                let key = PrivateKeyWithHashAlg::new(Arc::new(key), hash);
                if handle
                    .authenticate_publickey(user, key)
                    .await
                    .map(|r| r.success())
                    .unwrap_or(false)
                {
                    return Ok(());
                }
            }
            Err(e) => {
                return Err(format!("Failed to parse private key: {e}"));
            }
        }
    }

    if auth.allow_default_keys {
        for path in ssh_keys::default_key_paths() {
            if !path.is_file() {
                continue;
            }
            let Ok(key) = load_secret_key(&path, None) else {
                continue;
            };
            let hash = handle
                .best_supported_rsa_hash()
                .await
                .map_err(|e| e.to_string())?
                .flatten();
            let key = PrivateKeyWithHashAlg::new(Arc::new(key), hash);
            if handle
                .authenticate_publickey(user, key)
                .await
                .map(|r| r.success())
                .unwrap_or(false)
            {
                return Ok(());
            }
        }
    }

    let mut password = auth.password.clone();
    if auth.allow_default_keys {
        password = password.or_else(|| {
            std::env::var("SSH_PASSWORD")
                .ok()
                .filter(|p| !p.is_empty())
        });
    }

    if let Some(pw) = password.as_deref().filter(|p| !p.is_empty()) {
        if handle
            .authenticate_password(user, pw)
            .await
            .map(|r| r.success())
            .unwrap_or(false)
        {
            return Ok(());
        }
        if try_keyboard_interactive(handle, user, pw).await {
            return Ok(());
        }
    }

    if handle
        .authenticate_none(user)
        .await
        .map(|r| r.success())
        .unwrap_or(false)
    {
        return Ok(());
    }

    Err("SFTP authentication failed".into())
}

async fn try_keyboard_interactive(
    handle: &mut Handle<SftpSshClient>,
    user: &str,
    password: &str,
) -> bool {
    let mut resp = match handle
        .authenticate_keyboard_interactive_start(user, None::<String>)
        .await
    {
        Ok(r) => r,
        Err(_) => return false,
    };

    loop {
        match resp {
            KeyboardInteractiveAuthResponse::Success => return true,
            KeyboardInteractiveAuthResponse::Failure { .. } => return false,
            KeyboardInteractiveAuthResponse::InfoRequest { prompts, .. } => {
                let answers: Vec<String> = prompts.iter().map(|_| password.to_string()).collect();
                resp = match handle
                    .authenticate_keyboard_interactive_respond(answers)
                    .await
                {
                    Ok(r) => r,
                    Err(_) => return false,
                };
            }
        }
    }
}

fn drain_requests_with_error(req_rx: &mpsc::Receiver<SftpRequest>, err: &str) {
    while let Ok(req) = req_rx.recv() {
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
}

pub fn join_remote(base: &str, name: &str) -> String {
    if base.ends_with('/') {
        format!("{base}{name}")
    } else {
        format!("{base}/{name}")
    }
}
