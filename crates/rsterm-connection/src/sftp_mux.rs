use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use russh::client::{self, Handle};
use russh_sftp::client::SftpSession;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::sftp_endpoint::{SftpDirEntry, SftpProgress, SftpRequest, SftpStatInfo};

fn format_unix_mode(mode: u32) -> String {
    const BITS: [(u32, char); 9] = [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ];
    let mut s = String::with_capacity(9);
    for (bit, ch) in BITS {
        s.push(if mode & bit != 0 { ch } else { '-' });
    }
    format!("{s} ({mode:o})")
}

fn format_time(t: Option<SystemTime>) -> String {
    let Some(ts) = t else {
        return "—".into();
    };
    let Ok(dur) = ts.duration_since(SystemTime::UNIX_EPOCH) else {
        return "—".into();
    };
    let secs = dur.as_secs();
    let days = secs / 86400;
    let time = secs % 86400;
    let h = time / 3600;
    let m = (time % 3600) / 60;
    let s = time % 60;
    let (y, mo, d) = epoch_days_to_ymd(days);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{m:02}:{s:02} UTC")
}

fn epoch_days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    days += 719468;
    let era = days / 146097;
    let doe = days % 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mp < 10 { y } else { y + 1 };
    (y, mo, d)
}

fn metadata_mtime(meta: &russh_sftp::client::fs::Metadata) -> Option<SystemTime> {
    meta.mtime
        .and_then(|t| UNIX_EPOCH.checked_add(Duration::new(t as u64, 0)))
        .or_else(|| meta.modified().ok())
}

/// Open an SFTP subsystem on an already-authenticated SSH handle (shared session).
pub async fn open_sftp_on_handle<H>(handle: &Handle<H>) -> Result<SftpSession, String>
where
    H: client::Handler + Send + 'static,
    H::Error: From<russh::Error> + Send,
{
    let channel = handle
        .channel_open_session()
        .await
        .map_err(|e| e.to_string())?;
    channel
        .request_subsystem(true, "sftp")
        .await
        .map_err(|e| e.to_string())?;
    let sftp = SftpSession::new(channel.into_stream())
        .await
        .map_err(|e| e.to_string())?;
    sftp.set_timeout(120);
    Ok(sftp)
}

pub async fn write_remote_bytes(
    sftp: &SftpSession,
    remote: &str,
    data: &[u8],
) -> Result<(), String> {
    let mut file = sftp.create(remote).await.map_err(|e| e.to_string())?;
    file.write_all(data).await.map_err(|e| e.to_string())?;
    let _ = file.sync_all().await;
    drop(file);
    let _ = sftp
        .set_metadata(
            remote,
            russh_sftp::protocol::FileAttributes {
                permissions: Some(0o700),
                ..Default::default()
            },
        )
        .await;
    Ok(())
}

/// Dispatch one bridged SFTP request on a live session (shared SSH runtime).
pub async fn apply_sftp_request(sftp: &SftpSession, req: SftpRequest) {
    match req {
        SftpRequest::List { path, reply } => {
            let _ = reply.send(list_remote(sftp, &path).await);
        }
        SftpRequest::Upload {
            local,
            remote,
            progress,
            label,
            reply,
        } => {
            let _ =
                reply.send(upload_file(sftp, &local, &remote, progress.as_deref(), &label).await);
        }
        SftpRequest::Download {
            remote,
            local,
            progress,
            label,
            reply,
        } => {
            let _ =
                reply.send(download_file(sftp, &remote, &local, progress.as_deref(), &label).await);
        }
        SftpRequest::Stat { path, reply } => {
            let _ = reply.send(remote_entry_info(sftp, &path).await);
        }
        SftpRequest::PathBytes { path, reply } => {
            let _ = reply.send(remote_path_bytes(sftp, &path).await);
        }
        SftpRequest::Remove {
            path,
            is_dir,
            reply,
        } => {
            let r = if is_dir {
                sftp.remove_dir(&path).await
            } else {
                sftp.remove_file(&path).await
            }
            .map_err(|e| e.to_string());
            let _ = reply.send(r);
        }
        SftpRequest::Mkdir { path, reply } => {
            let _ = reply.send(sftp.create_dir(&path).await.map_err(|e| e.to_string()));
        }
        SftpRequest::Rename { from, to, reply } => {
            let _ = reply.send(sftp.rename(&from, &to).await.map_err(|e| e.to_string()));
        }
        SftpRequest::Home { reply } => {
            let r = match sftp.canonicalize(".").await {
                Ok(p) if !p.is_empty() && p != "." => Ok(p),
                Ok(_) | Err(_) => match sftp.canonicalize("~").await {
                    Ok(p) if !p.is_empty() => Ok(p),
                    _ => Ok("/".to_string()),
                },
            };
            let _ = reply.send(r);
        }
        SftpRequest::WriteBytes {
            remote,
            data,
            reply,
        } => {
            let _ = reply.send(write_remote_bytes(sftp, &remote, &data).await);
        }
        SftpRequest::Shutdown => {}
    }
}

async fn list_remote(sftp: &SftpSession, path: &str) -> Result<Vec<SftpDirEntry>, String> {
    let read_dir = sftp.read_dir(path).await.map_err(|e| e.to_string())?;
    let mut entries = Vec::new();
    for entry in read_dir {
        let name = entry.file_name();
        let meta = entry.metadata();
        let is_dir = meta.is_dir();
        entries.push(SftpDirEntry {
            name,
            is_dir,
            size: meta.len(),
            modified: metadata_mtime(&meta),
        });
    }
    entries.sort_by_key(|e| e.sort_key());
    Ok(entries)
}

fn check_cancel(progress: Option<&dyn SftpProgress>) -> Result<(), String> {
    if progress.is_some_and(|p| p.is_cancelled()) {
        Err("Transfer stopped".into())
    } else {
        Ok(())
    }
}

async fn upload_file(
    sftp: &SftpSession,
    local: &Path,
    remote: &str,
    progress: Option<&dyn SftpProgress>,
    label: &str,
) -> Result<(), String> {
    use std::fs::File;
    use std::io::Read;

    check_cancel(progress)?;

    if local.is_dir() {
        let _ = sftp.create_dir(remote).await;
        for item in std::fs::read_dir(local).map_err(|e| e.to_string())? {
            check_cancel(progress)?;
            let item = item.map_err(|e| e.to_string())?;
            let name = item.file_name().to_string_lossy().into_owned();
            let sub_local = local.join(&name);
            let sub_remote = format!("{remote}/{name}");
            let sub_label = format!("Uploading {name}");
            Box::pin(upload_file(
                sftp,
                &sub_local,
                &sub_remote,
                progress,
                &sub_label,
            ))
            .await?;
        }
        return Ok(());
    }

    if let Some(p) = progress {
        p.set_label(label);
    }

    let mut local_f = File::open(local).map_err(|e| e.to_string())?;
    let mut remote_f = sftp.create(remote).await.map_err(|e| e.to_string())?;
    let mut buf = [0u8; 64 * 1024];
    loop {
        check_cancel(progress)?;
        let n = local_f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        remote_f
            .write_all(&buf[..n])
            .await
            .map_err(|e| e.to_string())?;
        if let Some(p) = progress {
            p.add_bytes(n as u64, label);
        }
    }
    let _ = remote_f.shutdown().await;
    Ok(())
}

async fn download_file(
    sftp: &SftpSession,
    remote: &str,
    local: &Path,
    progress: Option<&dyn SftpProgress>,
    label: &str,
) -> Result<(), String> {
    use std::fs::File;
    use std::io::Write;

    check_cancel(progress)?;

    let meta = sftp.metadata(remote).await.map_err(|e| e.to_string())?;
    if meta.is_dir() {
        std::fs::create_dir_all(local).map_err(|e| e.to_string())?;
        let read_dir = sftp.read_dir(remote).await.map_err(|e| e.to_string())?;
        for entry in read_dir {
            check_cancel(progress)?;
            let name = entry.file_name();
            let sub_remote = format!("{remote}/{name}");
            let sub_local = local.join(&name);
            let sub_label = format!("Downloading {name}");
            Box::pin(download_file(
                sftp,
                &sub_remote,
                &sub_local,
                progress,
                &sub_label,
            ))
            .await?;
        }
        return Ok(());
    }

    if let Some(p) = progress {
        p.set_label(label);
    }

    if let Some(parent) = local.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut remote_f = sftp.open(remote).await.map_err(|e| e.to_string())?;
    let mut local_f = File::create(local).map_err(|e| e.to_string())?;
    let mut buf = [0u8; 64 * 1024];
    loop {
        check_cancel(progress)?;
        let n = remote_f.read(&mut buf).await.map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        local_f.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        if let Some(p) = progress {
            p.add_bytes(n as u64, label);
        }
    }
    let _ = remote_f.shutdown().await;
    Ok(())
}

async fn remote_entry_info(sftp: &SftpSession, path: &str) -> Result<SftpStatInfo, String> {
    let stat = sftp.metadata(path).await.map_err(|e| e.to_string())?;
    let name = Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string());
    let is_dir = stat.is_dir();
    let size = if is_dir {
        remote_path_bytes(sftp, path).await?
    } else {
        stat.len()
    };
    let mode = stat.permissions.unwrap_or(0);
    Ok(SftpStatInfo {
        path: path.to_string(),
        name,
        kind: if is_dir {
            "Folder".into()
        } else {
            "File".into()
        },
        size,
        permissions: format_unix_mode(mode),
        modified: format_time(metadata_mtime(&stat)),
    })
}

async fn remote_path_bytes(sftp: &SftpSession, path: &str) -> Result<u64, String> {
    let stat = sftp.metadata(path).await.map_err(|e| e.to_string())?;
    if !stat.is_dir() {
        return Ok(stat.len());
    }
    let mut total = 0u64;
    let read_dir = sftp.read_dir(path).await.map_err(|e| e.to_string())?;
    for entry in read_dir {
        let name = entry.file_name();
        let sub = format!("{path}/{name}");
        total += Box::pin(remote_path_bytes(sftp, &sub)).await?;
    }
    Ok(total)
}
