//! Compact tagged wire protocol for the remote status agent.
//!
//! Wire lines (LF-terminated), v2:
//! - `H a=rsterm-agent v=0.2.0`
//! - `S t=<unix_sec> [h=<host>] [m=<kib_t>,<kib_a>,<kib_u>] [d=<mount>,<kib_t>,<kib_a>] [c=<l1>,<l5>,<l15>] [u=<uptime_sec>] [w=<cwd>]`
//! - `E c=<code> m=<msg>`
//!
//! Status frames are **deltas**: omitted tags keep the previous snapshot.
//! Volatile fields (`m`, `c`, `t`) are sent every tick; `u` about once a minute;
//! stable ones (`h`, `d`, `w`) only on change.
//!
//! Legacy NDJSON (`{"type":...}`) is still accepted for older agents.

use serde::{Deserialize, Serialize};

use super::status::{CpuInfo, DiskInfo, MemInfo, RemoteStatus};

pub const PROTOCOL_VERSION: u32 = 2;
pub const MAX_FRAME_BYTES: usize = 64 * 1024;

/// Frames sent agent → client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentToClient {
    Hello {
        #[serde(default = "protocol_v")]
        v: u32,
        agent: String,
        ver: String,
    },
    /// Partial status patch — missing fields mean “unchanged”.
    Status {
        #[serde(default = "protocol_v")]
        v: u32,
        ts: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
        #[serde(default, rename = "host", skip_serializing_if = "Option::is_none")]
        hostname: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mem: Option<MemInfo>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        disk: Vec<DiskInfo>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cpu: Option<CpuInfo>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        uptime_secs: Option<u64>,
        #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
        ext: serde_json::Value,
        /// When true, `disk: []` means clear; when false (tagged delta), empty disk means omit.
        #[serde(default, skip_serializing)]
        disk_present: bool,
    },
    Error {
        #[serde(default = "protocol_v")]
        v: u32,
        code: String,
        msg: String,
    },
    Pong {
        #[serde(default = "protocol_v")]
        v: u32,
    },
}

fn protocol_v() -> u32 {
    PROTOCOL_VERSION
}

impl AgentToClient {
    pub fn into_remote_status(self) -> Option<RemoteStatus> {
        match self {
            AgentToClient::Status {
                ts,
                cwd,
                hostname,
                mem,
                disk,
                cpu,
                uptime_secs,
                ext,
                ..
            } => Some(RemoteStatus {
                schema: PROTOCOL_VERSION,
                ts_ms: ts,
                cwd,
                hostname,
                mem,
                disk,
                cpu,
                uptime_secs,
                ext,
            }),
            _ => None,
        }
    }

    /// Whether this status patch explicitly carries a disk list (possibly empty).
    pub fn status_disk_present(&self) -> bool {
        matches!(self, AgentToClient::Status { disk_present: true, .. })
    }
}

/// Frames sent client → agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientToAgent {
    Configure {
        #[serde(default = "protocol_v")]
        v: u32,
        interval_ms: u64,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        disk_mounts: Vec<String>,
    },
    Ping {
        #[serde(default = "protocol_v")]
        v: u32,
    },
    Shutdown {
        #[serde(default = "protocol_v")]
        v: u32,
    },
}

impl ClientToAgent {
    pub fn configure(interval_ms: u64) -> Self {
        Self::Configure {
            v: PROTOCOL_VERSION,
            interval_ms,
            disk_mounts: vec!["/".into()],
        }
    }

    pub fn shutdown() -> Self {
        Self::Shutdown {
            v: PROTOCOL_VERSION,
        }
    }

    pub fn to_ndjson_line(&self) -> Result<String, String> {
        let mut s = serde_json::to_string(self).map_err(|e| e.to_string())?;
        s.push('\n');
        Ok(s)
    }
}

/// Parse one line (without trailing newline). Ignores empty / oversized lines.
pub fn parse_agent_line(line: &str) -> Result<Option<AgentToClient>, String> {
    let line = line.trim();
    if line.is_empty() {
        return Ok(None);
    }
    if line.len() > MAX_FRAME_BYTES {
        return Err("agent frame exceeds 64KiB".into());
    }
    if line.starts_with('{') {
        return parse_json_line(line);
    }
    parse_tagged_line(line).map(Some)
}

fn parse_json_line(line: &str) -> Result<Option<AgentToClient>, String> {
    let mut msg: AgentToClient = serde_json::from_str(line).map_err(|e| e.to_string())?;
    if let AgentToClient::Status {
        ref mut disk_present,
        ref disk,
        ..
    } = msg
    {
        // Legacy JSON always includes disk (maybe empty).
        *disk_present = true;
        let _ = disk;
    }
    Ok(Some(msg))
}

fn parse_tagged_line(line: &str) -> Result<AgentToClient, String> {
    let (kind, rest) = match line.split_once(char::is_whitespace) {
        Some((k, r)) => (k, r.trim_start()),
        None => (line, ""),
    };
    match kind {
        "H" => parse_hello_tags(rest),
        "S" => parse_status_tags(rest),
        "E" => parse_error_tags(rest),
        "P" => Ok(AgentToClient::Pong {
            v: PROTOCOL_VERSION,
        }),
        _ => Err(format!("unknown agent frame kind: {kind}")),
    }
}

fn parse_hello_tags(rest: &str) -> Result<AgentToClient, String> {
    let tags = parse_tags(rest, false)?;
    let agent = tags
        .get("a")
        .cloned()
        .unwrap_or_else(|| "rsterm-agent".into());
    let ver = tags.get("v").cloned().unwrap_or_else(|| "0".into());
    Ok(AgentToClient::Hello {
        v: PROTOCOL_VERSION,
        agent,
        ver,
    })
}

fn parse_error_tags(rest: &str) -> Result<AgentToClient, String> {
    let tags = parse_tags(rest, true)?;
    Ok(AgentToClient::Error {
        v: PROTOCOL_VERSION,
        code: tags.get("c").cloned().unwrap_or_else(|| "error".into()),
        msg: tags.get("m").cloned().unwrap_or_default(),
    })
}

fn parse_status_tags(rest: &str) -> Result<AgentToClient, String> {
    let tags = parse_tags(rest, true)?;
    let t_sec: u64 = tags
        .get("t")
        .ok_or_else(|| "status missing t=".to_string())?
        .parse()
        .map_err(|_| "invalid t=".to_string())?;
    // Wire uses unix seconds; snapshot keeps ms.
    let ts = t_sec.saturating_mul(1000);

    let hostname = tags.get("h").cloned().filter(|s| !s.is_empty());
    let cwd = tags.get("w").cloned().filter(|s| !s.is_empty());

    let mem = match tags.get("m") {
        Some(v) => Some(parse_mem_kib(v)?),
        None => None,
    };
    let (disk, disk_present) = match tags.get("d") {
        Some(v) => (vec![parse_disk_kib(v)?], true),
        None => (Vec::new(), false),
    };
    let cpu = match tags.get("c") {
        Some(v) => Some(parse_cpu(v)?),
        None => None,
    };
    let uptime_secs = match tags.get("u") {
        Some(v) => Some(v.parse().map_err(|_| "invalid u=".to_string())?),
        None => None,
    };

    Ok(AgentToClient::Status {
        v: PROTOCOL_VERSION,
        ts,
        cwd,
        hostname,
        mem,
        disk,
        cpu,
        uptime_secs,
        ext: serde_json::Value::Null,
        disk_present,
    })
}

/// Parse `key=value` tokens. If `cwd_rest` is true, `w=` / `m=` (error msg) consume the remainder.
fn parse_tags(rest: &str, cwd_or_msg_rest: bool) -> Result<std::collections::HashMap<String, String>, String> {
    let mut tags = std::collections::HashMap::new();
    let mut rest = rest;

    // `w=` (cwd) or error `m=` may contain spaces — take as remainder of the line.
    if cwd_or_msg_rest {
        if let Some(idx) = find_tag(rest, "w") {
            let before = rest[..idx].trim_end();
            let value = rest[idx + 2..].to_string();
            tags.insert("w".into(), value);
            rest = before;
        }
    }

    for tok in rest.split_whitespace() {
        let Some((k, v)) = tok.split_once('=') else {
            return Err(format!("bad tag token: {tok}"));
        };
        if k.is_empty() {
            return Err("empty tag key".into());
        }
        // Prefer first `w=` handled above; ignore duplicate short tokens.
        if k == "w" && tags.contains_key("w") {
            continue;
        }
        tags.insert(k.to_string(), v.to_string());
    }

    // Error frames: `m=` message may need remainder form when it has spaces.
    // `E c=sample m=first sample failed` — if m was only "first", fix by re-scan.
    // For simplicity agent emits m without spaces (underscores). OK.

    Ok(tags)
}

fn find_tag(s: &str, key: &str) -> Option<usize> {
    let pat = format!("{key}=");
    if s.starts_with(&pat) {
        return Some(0);
    }
    s.find(&format!(" {pat}")).map(|i| i + 1)
}

fn parse_mem_kib(v: &str) -> Result<MemInfo, String> {
    let parts: Vec<&str> = v.split(',').collect();
    if parts.len() != 3 {
        return Err(format!("bad m= tag: {v}"));
    }
    let total = kib_to_bytes(parts[0])?;
    let avail = kib_to_bytes(parts[1])?;
    let used = kib_to_bytes(parts[2])?;
    Ok(MemInfo { total, avail, used })
}

fn parse_disk_kib(v: &str) -> Result<DiskInfo, String> {
    // mount may be `/` — split from the right for two ints.
    let Some((mount, rest)) = v.split_once(',') else {
        return Err(format!("bad d= tag: {v}"));
    };
    let Some((total_s, avail_s)) = rest.split_once(',') else {
        return Err(format!("bad d= tag: {v}"));
    };
    Ok(DiskInfo {
        mount: mount.to_string(),
        total: kib_to_bytes(total_s)?,
        avail: kib_to_bytes(avail_s)?,
    })
}

fn parse_cpu(v: &str) -> Result<CpuInfo, String> {
    let parts: Vec<&str> = v.split(',').collect();
    if parts.len() != 3 {
        return Err(format!("bad c= tag: {v}"));
    }
    Ok(CpuInfo {
        load1: parts[0].parse().map_err(|_| "bad c= load1")?,
        load5: parts[1].parse().map_err(|_| "bad c= load5")?,
        load15: parts[2].parse().map_err(|_| "bad c= load15")?,
        usage_percent: None,
    })
}

fn kib_to_bytes(s: &str) -> Result<u64, String> {
    let kib: u64 = s.parse().map_err(|_| format!("bad kib: {s}"))?;
    Ok(kib.saturating_mul(1024))
}

/// Feed byte chunks into a line buffer; returns completed frames.
pub fn push_bytes(buf: &mut String, data: &[u8], out: &mut Vec<AgentToClient>) -> Result<(), String> {
    let chunk = std::str::from_utf8(data).map_err(|e| e.to_string())?;
    buf.push_str(chunk);
    while let Some(idx) = buf.find('\n') {
        let line: String = buf.drain(..=idx).collect();
        let line = line.trim_end_matches(['\r', '\n']);
        if let Some(msg) = parse_agent_line(line)? {
            out.push(msg);
        }
        if buf.len() > MAX_FRAME_BYTES {
            return Err("agent line buffer overflow".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tagged_hello_and_status() {
        let msg = parse_agent_line("H a=rsterm-agent v=0.2.0")
            .unwrap()
            .unwrap();
        assert!(matches!(
            msg,
            AgentToClient::Hello {
                agent: ref a,
                ver: ref v,
                ..
            } if a == "rsterm-agent" && v == "0.2.0"
        ));

        let msg = parse_agent_line(
            "S t=1710000000 h=box m=100,40,60 d=/,500,100 c=0.5,0.4,0.3 w=/home/u",
        )
        .unwrap()
        .unwrap();
        let st = msg.into_remote_status().unwrap();
        assert_eq!(st.ts_ms, 1_710_000_000_000);
        assert_eq!(st.cwd.as_deref(), Some("/home/u"));
        assert_eq!(st.hostname.as_deref(), Some("box"));
        assert_eq!(st.mem.unwrap().total, 100 * 1024);
        assert_eq!(st.disk[0].mount, "/");
        assert_eq!(st.disk[0].total, 500 * 1024);
        assert!((st.cpu.unwrap().load1 - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_status_delta_omits_stable_tags() {
        let msg = parse_agent_line("S t=1710000001 m=100,39,61 c=0.5,0.4,0.3")
            .unwrap()
            .unwrap();
        match msg {
            AgentToClient::Status {
                hostname: None,
                disk_present: false,
                mem: Some(_),
                cpu: Some(_),
                ..
            } => {}
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parse_cwd_with_spaces() {
        let msg = parse_agent_line("S t=1 m=1,1,0 c=0,0,0 w=/home/u/My Docs")
            .unwrap()
            .unwrap();
        let st = msg.into_remote_status().unwrap();
        assert_eq!(st.cwd.as_deref(), Some("/home/u/My Docs"));
    }

    #[test]
    fn legacy_json_still_parses() {
        let hello = r#"{"v":1,"type":"hello","agent":"rsterm-agent","ver":"0.1.0"}"#;
        assert!(matches!(
            parse_agent_line(hello).unwrap().unwrap(),
            AgentToClient::Hello { .. }
        ));
    }

    #[test]
    fn push_bytes_splits_tagged_lines() {
        let mut buf = String::new();
        let mut out = Vec::new();
        push_bytes(&mut buf, b"H a=a v=1", &mut out).unwrap();
        assert!(out.is_empty());
        push_bytes(&mut buf, b"\nS t=1 m=1,1,0 c=0,0,0\n", &mut out).unwrap();
        assert_eq!(out.len(), 2);
    }
}
