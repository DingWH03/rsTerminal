//! Interactive SSH PTY plus optional shared SFTP + status agent on one TCP connection.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use russh::client::{self, Handle, KeyboardInteractiveAuthResponse};
use russh::keys::{PrivateKeyWithHashAlg, decode_secret_key, load_secret_key};
use russh::{Channel, ChannelMsg, Disconnect};
use tokio::sync::mpsc::unbounded_channel;
use tokio::time::timeout;

use crate::connection::sftp_endpoint::{
    SftpEndpoint, SftpRequest, SftpStatus, mark_connected, mark_error, reply_sftp_gone,
};
use crate::connection::sftp_mux;
use crate::connection::{
    ConnIn, ConnOut, ConnectionHandle, ConnectionState, RepaintNotifier, SshConnectParams,
    emit_conn_data, ssh_auth::ResolvedSshAuth, ssh_keys,
};
use crate::remote::AGENT_SCRIPT;
use crate::remote::protocol::{AgentToClient, push_bytes};
use crate::remote::status::{MetricsEvent, SessionMetrics};

pub use crate::config::SSH_OSC7_PROMPT_COMMAND;

/// Result of opening a multiplexed SSH session (PTY + shared SFTP + status agent).
pub struct SshConnectOutcome {
    pub handle: ConnectionHandle,
    pub metrics: SessionMetrics,
    /// Shared-session SFTP endpoint; wrap with `fs::SftpClient::from_endpoint` at app boundary.
    pub sftp_endpoint: SftpEndpoint,
}

struct SshClient;

impl client::Handler for SshClient {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

pub fn connect_ssh(
    params: &SshConnectParams,
    auth: ResolvedSshAuth,
    rows: u16,
    cols: u16,
) -> Result<ConnectionHandle, String> {
    Ok(connect_ssh_session(params, auth, rows, cols)?.handle)
}

/// Preferred entry: returns PTY handle, metrics bus, and shared-session SFTP.
pub fn connect_ssh_session(
    params: &SshConnectParams,
    auth: ResolvedSshAuth,
    rows: u16,
    cols: u16,
) -> Result<SshConnectOutcome, String> {
    if params.host.is_empty() {
        return Err("SSH host not configured".to_string());
    }
    if auth.username.is_empty() {
        return Err("SSH user not configured".to_string());
    }
    let host = params.host.clone();
    let port = params.port;
    let session_tag = params.session_tag.clone();
    let env_vars = params.env_vars.clone();

    let (to_conn_tx, to_conn_rx) = mpsc::channel::<ConnOut>();
    let (from_conn_tx, from_conn_rx) = mpsc::channel::<ConnIn>();

    let metrics = SessionMetrics::new();
    let metrics_thread = metrics.clone();

    let repaint = RepaintNotifier::default();
    let (sftp_endpoint, sftp_rx) = SftpEndpoint::new(repaint.clone());
    let sftp_status_thread = sftp_endpoint.status.clone();

    let host_clone = host.clone();
    let from_tx = from_conn_tx.clone();
    let ssh_repaint = repaint.clone();

    let thread = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");

        if let Err(msg) = rt.block_on(run_ssh_session(
            &host_clone,
            port,
            auth,
            rows,
            cols,
            &env_vars,
            &session_tag,
            to_conn_rx,
            from_tx,
            ssh_repaint,
            metrics_thread,
            sftp_rx,
            sftp_status_thread,
        )) {
            let _ = from_conn_tx.send(ConnIn::StateChanged(ConnectionState::Error(msg)));
        }
    });

    Ok(SshConnectOutcome {
        handle: ConnectionHandle::new(
            to_conn_tx,
            from_conn_rx,
            thread,
            std::thread::spawn(|| {}),
            repaint,
        ),
        metrics,
        sftp_endpoint,
    })
}

async fn run_ssh_session(
    host: &str,
    port: u16,
    auth: ResolvedSshAuth,
    rows: u16,
    cols: u16,
    env_vars: &HashMap<String, String>,
    session_tag: &str,
    to_conn_rx: mpsc::Receiver<ConnOut>,
    from_tx: mpsc::Sender<ConnIn>,
    repaint: RepaintNotifier,
    metrics: SessionMetrics,
    sftp_rx: mpsc::Receiver<SftpRequest>,
    sftp_status: Arc<Mutex<SftpStatus>>,
) -> Result<(), String> {
    // Bridge sync SFTP requests into the async runtime.
    let (sftp_async_tx, mut sftp_async_rx) = unbounded_channel::<SftpRequest>();
    std::thread::spawn(move || {
        while let Ok(msg) = sftp_rx.recv() {
            if sftp_async_tx.send(msg).is_err() {
                break;
            }
        }
    });

    let (out_async_tx, mut out_async_rx) = unbounded_channel::<ConnOut>();
    std::thread::spawn(move || {
        while let Ok(msg) = to_conn_rx.recv() {
            if out_async_tx.send(msg).is_err() {
                break;
            }
        }
    });

    let ssh_config = Arc::new(client::Config {
        keepalive_interval: Some(Duration::from_secs(15)),
        keepalive_max: 3,
        inactivity_timeout: Some(Duration::from_secs(180)),
        nodelay: true,
        ..Default::default()
    });

    let mut handle = timeout(
        Duration::from_secs(20),
        client::connect(ssh_config, (host, port), SshClient),
    )
    .await
    .map_err(|_| format!("SSH connection to {host}:{port} timed out (20s)"))?
    .map_err(|e| e.to_string())?;

    let mut password = auth.password.clone();
    if auth.allow_default_keys {
        password = password
            .or_else(|| std::env::var("SSH_PASSWORD").ok())
            .or_else(|| std::env::var("RSTERMINAL_SSH_PASSWORD").ok());
    }

    timeout(
        Duration::from_secs(30),
        authenticate(&mut handle, &auth.username, &auth, password.as_deref()),
    )
    .await
    .map_err(|_| "SSH authentication timed out (30s)".to_string())?
    .map_err(|e| e.to_string())?;

    // Shared SFTP subsystem (sidebar listing + agent script upload).
    let sftp_session = match sftp_mux::open_sftp_on_handle(&handle).await {
        Ok(s) => {
            mark_connected(&sftp_status);
            repaint.request_repaint();
            Some(s)
        }
        Err(e) => {
            log::warn!("shared SFTP open failed: {e}");
            mark_error(&sftp_status, e);
            repaint.request_repaint();
            None
        }
    };

    let mut channel = timeout(Duration::from_secs(15), handle.channel_open_session())
        .await
        .map_err(|_| "Opening SSH channel timed out".to_string())?
        .map_err(|e| e.to_string())?;

    let cols_u = cols.max(1) as u32;
    let rows_u = rows.max(1) as u32;

    timeout(
        Duration::from_secs(10),
        channel.request_pty(false, "xterm-256color", cols_u, rows_u, 0, 0, &[]),
    )
    .await
    .map_err(|_| "PTY request timed out".to_string())?
    .map_err(|e| e.to_string())?;

    let mut has_prompt_command = false;
    for (key, value) in env_vars {
        if key == "PROMPT_COMMAND" {
            has_prompt_command = true;
        }
        let _ = channel.set_env(true, key, value).await;
    }
    if !has_prompt_command {
        let _ = channel
            .set_env(true, "PROMPT_COMMAND", SSH_OSC7_PROMPT_COMMAND)
            .await;
    }

    timeout(Duration::from_secs(10), channel.request_shell(true))
        .await
        .map_err(|_| "Shell request timed out".to_string())?
        .map_err(|e| e.to_string())?;

    // Status agent after the interactive shell is up. Deploy via stdin (`sh -s`) so we
    // do not depend on SFTP flush visibility or the login shell parsing `VAR=val cmd`
    // (fish/csh break that form and exit with zero stdout).
    let (mut agent_ch, mut agent_buf, agent_remote) =
        match start_status_agent(&handle, sftp_session.as_ref(), session_tag).await {
            Ok((ch, buf, path)) => {
                log::info!(
                    "status agent exec ok deploy={}",
                    path.as_deref().unwrap_or("stdin")
                );
                metrics.push_event(MetricsEvent::AgentStarted {
                    remote_path: path.clone().unwrap_or_else(|| "stdin:-".into()),
                });
                repaint.request_repaint();
                (Some(ch), buf, path)
            }
            Err(e) => {
                log::warn!("status agent start failed: {e}");
                metrics.push_event(MetricsEvent::AgentStartFailed { error: e.clone() });
                repaint.request_repaint();
                (None, String::new(), None)
            }
        };

    let _ = from_tx.send(ConnIn::StateChanged(ConnectionState::Connected));

    let mut clean_close = false;
    let mut sftp_session = sftp_session;
    let mut agent_exit: Option<u32> = None;
    loop {
        tokio::select! {
            msg = channel.wait() => {
                match msg {
                    Some(ChannelMsg::Data { data }) => {
                        emit_conn_data(&from_tx, &repaint, data.to_vec());
                    }
                    Some(ChannelMsg::ExtendedData { data, .. }) => {
                        emit_conn_data(&from_tx, &repaint, data.to_vec());
                    }
                    Some(ChannelMsg::Eof) => {
                        clean_close = true;
                        break;
                    }
                    Some(ChannelMsg::Close) | None => break,
                    _ => {}
                }
            }
            msg = async {
                match agent_ch.as_mut() {
                    Some(ch) => ch.wait().await,
                    None => std::future::pending().await,
                }
            } => {
                match msg {
                    Some(ChannelMsg::Data { data }) | Some(ChannelMsg::ExtendedData { data, .. }) => {
                        ingest_agent_bytes(&mut agent_buf, &data, &metrics, &repaint);
                    }
                    Some(ChannelMsg::ExitStatus { exit_status }) => {
                        log::warn!("status agent exit_status={exit_status}");
                        agent_exit = Some(exit_status);
                    }
                    Some(ChannelMsg::Failure) => {
                        log::warn!("status agent CHANNEL_FAILURE");
                        metrics.push_event(MetricsEvent::AgentClosed {
                            reason: "exec CHANNEL_FAILURE".into(),
                        });
                        repaint.request_repaint();
                        agent_ch = None;
                    }
                    Some(ChannelMsg::Success) => {
                        log::debug!("status agent late CHANNEL_SUCCESS");
                    }
                    Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) | None => {
                        let reason = match agent_exit {
                            Some(code) => format!("exit {code}"),
                            None => "channel closed".into(),
                        };
                        log::warn!("status agent closed: {reason}");
                        metrics.push_event(MetricsEvent::AgentClosed { reason });
                        repaint.request_repaint();
                        agent_ch = None;
                    }
                    other => {
                        log::debug!("status agent msg: {other:?}");
                    }
                }
            }
            out = out_async_rx.recv() => {
                match out {
                    Some(ConnOut::Data(data)) | Some(ConnOut::PortData { port: 0, data }) => {
                        if channel.data(&data[..]).await.is_err() {
                            break;
                        }
                    }
                    Some(ConnOut::PortData { .. }) => {}
                    Some(ConnOut::Resize(rows, cols)) => {
                        let _ = channel
                            .window_change(cols.max(1) as u32, rows.max(1) as u32, 0, 0)
                            .await;
                    }
                    Some(ConnOut::Winch) => {}
                    Some(ConnOut::Close) => {
                        clean_close = true;
                        break;
                    }
                    None => break,
                }
            }
            req = sftp_async_rx.recv() => {
                match req {
                    Some(SftpRequest::Shutdown) | None => {
                        sftp_session = None;
                    }
                    Some(req) => {
                        if let Some(ref sftp) = sftp_session {
                            sftp_mux::apply_sftp_request(sftp, req).await;
                        } else {
                            reply_sftp_gone(req);
                        }
                    }
                }
            }
        }
    }

    if let Some(ch) = agent_ch.take() {
        let _ = ch.close().await;
    }
    if let (Some(sftp), Some(path)) = (sftp_session.as_ref(), agent_remote.as_ref()) {
        let _ = sftp.remove_file(path).await;
    }

    let _ = channel.close().await;
    let _ = handle
        .disconnect(Disconnect::ByApplication, "", "English")
        .await;
    let end = if clean_close {
        ConnectionState::Closed
    } else {
        ConnectionState::Lost("Connection lost.".into())
    };
    let _ = from_tx.send(ConnIn::StateChanged(end));
    Ok(())
}

fn ingest_agent_bytes(
    line_buf: &mut String,
    data: &[u8],
    metrics: &SessionMetrics,
    repaint: &RepaintNotifier,
) {
    let mut frames = Vec::new();
    if let Err(e) = push_bytes(line_buf, data, &mut frames) {
        log::warn!("agent frame error: {e}");
        metrics.push_event(MetricsEvent::ParseError { error: e });
        repaint.request_repaint();
        return;
    }
    for frame in frames {
        let disk_present = frame.status_disk_present();
        if let Some(st) = frame.clone().into_remote_status() {
            log::debug!(
                "status agent patch ts={} host={:?} mem={} disk={}",
                st.ts_ms,
                st.hostname,
                st.mem.is_some(),
                disk_present
            );
            metrics.apply_agent_status_ex(st, disk_present);
            repaint.request_repaint();
            continue;
        }
        match frame {
            AgentToClient::Hello { agent, ver, .. } => {
                log::info!("remote agent hello: {agent} {ver}");
                metrics.push_event(MetricsEvent::Hello { agent, ver });
                repaint.request_repaint();
            }
            AgentToClient::Error { code, msg, .. } => {
                log::warn!("remote agent error {code}: {msg}");
                metrics.push_event(MetricsEvent::ParseError {
                    error: format!("{code}: {msg}"),
                });
                repaint.request_repaint();
            }
            AgentToClient::Pong { .. } | AgentToClient::Status { .. } => {}
        }
    }
}

async fn start_status_agent(
    handle: &Handle<SshClient>,
    _shared_sftp: Option<&russh_sftp::client::SftpSession>,
    _session_tag: &str,
) -> Result<(Channel<client::Msg>, String, Option<String>), String> {
    let mut ch = handle
        .channel_open_session()
        .await
        .map_err(|e| e.to_string())?;

    // OpenSSH always runs client commands as `$SHELL -c "<cmd>"`. Prefix env
    // assignments (`VAR=1 cmd`) are POSIX/bash-only — fish/csh reject them and
    // the channel closes with no stdout. Always wrap with `/bin/sh -c` so the
    // login shell only has to spawn an external command.
    //
    // Feed the script on stdin (`sh -s`) to avoid depending on SFTP write
    // visibility /tmp noexec edge cases.
    let cmd = "/bin/sh -c 'RSTERM_INTERVAL_MS=1000 RSTERM_DISK_MOUNT=/ exec /bin/sh -s'";
    log::info!("status agent exec cmd={cmd}");
    ch.exec(true, cmd).await.map_err(|e| e.to_string())?;
    wait_channel_success(&mut ch, "agent exec").await?;

    ch.data(AGENT_SCRIPT.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    if !AGENT_SCRIPT.ends_with('\n') {
        ch.data(&b"\n"[..]).await.map_err(|e| e.to_string())?;
    }
    ch.eof().await.map_err(|e| e.to_string())?;

    Ok((ch, String::new(), None))
}

/// Drain channel messages until SSH `CHANNEL_SUCCESS` / `CHANNEL_FAILURE`.
async fn wait_channel_success(ch: &mut Channel<client::Msg>, what: &str) -> Result<(), String> {
    loop {
        match timeout(Duration::from_secs(10), ch.wait()).await {
            Err(_) => return Err(format!("{what}: timed out waiting for CHANNEL_SUCCESS")),
            Ok(None) => return Err(format!("{what}: channel closed before reply")),
            Ok(Some(ChannelMsg::Success)) => return Ok(()),
            Ok(Some(ChannelMsg::Failure)) => {
                return Err(format!("{what}: CHANNEL_FAILURE"));
            }
            Ok(Some(ChannelMsg::Eof)) | Ok(Some(ChannelMsg::Close)) => {
                return Err(format!("{what}: closed before reply"));
            }
            Ok(Some(ChannelMsg::ExitStatus { exit_status })) => {
                return Err(format!(
                    "{what}: exited before start (status {exit_status})"
                ));
            }
            Ok(Some(ChannelMsg::Data { data }))
            | Ok(Some(ChannelMsg::ExtendedData { data, .. })) => {
                // Extremely early output before SUCCESS — keep waiting, but log it.
                let preview = String::from_utf8_lossy(&data);
                log::warn!(
                    "{what}: early output before SUCCESS ({} bytes): {:?}",
                    data.len(),
                    preview.chars().take(200).collect::<String>()
                );
            }
            Ok(Some(other)) => {
                log::debug!("{what}: ignoring {other:?} while waiting for SUCCESS");
            }
        }
    }
}

async fn authenticate(
    handle: &mut Handle<SshClient>,
    user: &str,
    auth: &ResolvedSshAuth,
    password: Option<&str>,
) -> Result<(), String> {
    // Prefer in-memory private key from ResolvedSshAuth when present.
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

    if let Some(pw) = password.filter(|p| !p.is_empty()) {
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

    Err("SSH authentication failed (tried public keys, password, and keyboard-interactive)".into())
}

async fn try_keyboard_interactive(
    handle: &mut Handle<SshClient>,
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
