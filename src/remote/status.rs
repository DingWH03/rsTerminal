//! Remote host status snapshot shared by the agent, OSC merger, and future UI.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

/// Memory counters in bytes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemInfo {
    pub total: u64,
    pub avail: u64,
    #[serde(default)]
    pub used: u64,
}

/// One filesystem mount.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskInfo {
    pub mount: String,
    pub total: u64,
    pub avail: u64,
}

/// Load averages / optional instantaneous usage.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CpuInfo {
    pub load1: f64,
    pub load5: f64,
    pub load15: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage_percent: Option<f64>,
}

/// Schema v1 host status (agent NDJSON `type=status` payload body).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RemoteStatus {
    /// Schema version (always 1 for this struct).
    #[serde(default = "status_schema_v1")]
    pub schema: u32,
    /// Unix epoch milliseconds from the agent (or local clock when merged).
    pub ts_ms: u64,
    /// Absolute cwd when known (agent watch-pid or OSC merger).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mem: Option<MemInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disk: Vec<DiskInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu: Option<CpuInfo>,
    /// Host uptime in seconds (`/proc/uptime`), when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uptime_secs: Option<u64>,
    /// Forward-compatible bag (GPU, net, …).
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub ext: serde_json::Value,
}

fn status_schema_v1() -> u32 {
    1
}

impl RemoteStatus {
    pub fn with_osc_cwd(mut self, osc: Option<&str>) -> Self {
        if self.cwd.is_none()
            && let Some(c) = osc.filter(|s| !s.is_empty())
        {
            self.cwd = Some(c.to_string());
        }
        self
    }
}

/// Rolling window length for sidebar performance charts (≈1 sample/sec).
pub const METRICS_HISTORY_LEN: usize = 60;

/// One compressed sample for charting (fixed-size, no strings).
#[derive(Debug, Clone, Copy, Default)]
pub struct MetricsSample {
    pub ts_ms: u64,
    /// 1-minute load average.
    pub load1: f32,
    /// Memory used / total, 0..=1.
    pub mem_used_ratio: f32,
    /// Disk used / total, 0..=1.
    pub disk_used_ratio: f32,
    pub mem_used: u64,
    pub mem_total: u64,
    pub disk_avail: u64,
    pub disk_total: u64,
}

impl MetricsSample {
    fn from_status(st: &RemoteStatus) -> Option<Self> {
        let mem = st.mem.as_ref()?;
        if mem.total == 0 {
            return None;
        }
        let mem_used_ratio = (mem.used as f64 / mem.total as f64).clamp(0.0, 1.0) as f32;
        let (disk_used_ratio, disk_avail, disk_total) = st
            .disk
            .first()
            .map(|d| {
                let used = d.total.saturating_sub(d.avail);
                let ratio = if d.total > 0 {
                    (used as f64 / d.total as f64).clamp(0.0, 1.0) as f32
                } else {
                    0.0
                };
                (ratio, d.avail, d.total)
            })
            .unwrap_or((0.0, 0, 0));
        let load1 = st.cpu.as_ref().map(|c| c.load1 as f32).unwrap_or(0.0);
        Some(Self {
            ts_ms: st.ts_ms,
            load1,
            mem_used_ratio,
            disk_used_ratio,
            mem_used: mem.used,
            mem_total: mem.total,
            disk_avail,
            disk_total,
        })
    }
}

/// Where the latest cwd / metrics came from (for UI debugging / future badges).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MetricsSource {
    #[default]
    None,
    Agent,
    Osc,
    Merged,
}

/// Debug / UI subscription events from the remote agent path.
#[derive(Debug, Clone)]
pub enum MetricsEvent {
    AgentStarted { remote_path: String },
    AgentStartFailed { error: String },
    Hello { agent: String, ver: String },
    Status(RemoteStatus),
    AgentClosed { reason: String },
    ParseError { error: String },
}

const EVENT_QUEUE_CAP: usize = 128;

/// Session-scoped metrics bus. UI (sidebar, future status bar / Info tab) reads only this.
#[derive(Clone, Default)]
pub struct SessionMetrics {
    inner: Arc<Mutex<SessionMetricsInner>>,
}

#[derive(Default)]
struct SessionMetricsInner {
    status: Option<RemoteStatus>,
    source: MetricsSource,
    /// Last OSC 7 cwd (never cleared by a missing frame).
    osc_cwd: Option<String>,
    /// Last cwd reported by the remote agent (`w=` /proc watch).
    agent_cwd: Option<String>,
    /// FIFO of recent agent events for debug subscribers / future UI.
    events: VecDeque<MetricsEvent>,
    /// Last enqueued status brief — skip duplicate Status events.
    last_status_brief: Option<String>,
    /// Last ~60 agent samples for the sidebar monitor charts.
    history: VecDeque<MetricsSample>,
}

impl SessionMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_event(&self, event: MetricsEvent) {
        let mut g = self.inner.lock().unwrap();
        if g.events.len() >= EVENT_QUEUE_CAP {
            g.events.pop_front();
        }
        g.events.push_back(event);
    }

    /// Drain queued events (subscriber / debug printer).
    pub fn drain_events(&self) -> Vec<MetricsEvent> {
        let mut g = self.inner.lock().unwrap();
        g.events.drain(..).collect()
    }

    /// Merge an agent status patch into the snapshot.
    ///
    /// Omitted fields (None / empty disk without explicit clear) keep prior values —
    /// this matches the compact delta wire format.
    pub fn apply_agent_status(&self, patch: RemoteStatus) {
        self.apply_agent_status_ex(patch, false);
    }

    /// Like [`apply_agent_status`], but `disk_present` controls whether an empty
    /// `disk` vector clears the previous disk list (legacy JSON) or is ignored (delta).
    pub fn apply_agent_status_ex(&self, patch: RemoteStatus, disk_present: bool) {
        let mut g = self.inner.lock().unwrap();
        let mut status = g.status.take().unwrap_or_default();
        status.schema = patch.schema.max(1);

        if patch.ts_ms != 0 {
            status.ts_ms = patch.ts_ms;
        }
        if patch.hostname.is_some() {
            status.hostname = patch.hostname;
        }
        if patch.mem.is_some() {
            status.mem = patch.mem;
        }
        if disk_present || !patch.disk.is_empty() {
            status.disk = patch.disk;
        }
        if patch.cpu.is_some() {
            status.cpu = patch.cpu;
        }
        if patch.uptime_secs.is_some() {
            status.uptime_secs = patch.uptime_secs;
        }
        if let Some(c) = patch.cwd.filter(|s| !s.is_empty()) {
            g.agent_cwd = Some(c.clone());
            status.cwd = Some(c);
        } else if status.cwd.is_none() {
            // Prefer agent cache, then OSC.
            status.cwd = g.agent_cwd.clone().or_else(|| g.osc_cwd.clone());
        }

        let source = match (&g.agent_cwd, &g.osc_cwd) {
            (Some(_), Some(_)) => MetricsSource::Merged,
            (Some(_), None) => MetricsSource::Agent,
            (None, Some(_)) => MetricsSource::Osc,
            (None, None) => MetricsSource::Agent,
        };
        g.source = source;

        // Only enqueue a Status event when the brief summary changes (cuts UI log spam).
        let brief = status_brief(&status);
        let changed = g.last_status_brief.as_deref() != Some(brief.as_str());
        if changed {
            g.last_status_brief = Some(brief);
            if g.events.len() >= EVENT_QUEUE_CAP {
                g.events.pop_front();
            }
            g.events.push_back(MetricsEvent::Status(status.clone()));
        }

        if let Some(sample) = MetricsSample::from_status(&status) {
            let same_ts = g
                .history
                .back()
                .is_some_and(|s| s.ts_ms != 0 && s.ts_ms == sample.ts_ms);
            if !same_ts {
                if g.history.len() >= METRICS_HISTORY_LEN {
                    g.history.pop_front();
                }
                g.history.push_back(sample);
            } else if let Some(last) = g.history.back_mut() {
                // Same second: refresh ratios (delta patches within the tick).
                *last = sample;
            }
        }

        g.status = Some(status);
    }

    /// Record OSC 7 cwd. `None` does **not** clear a previously known OSC cwd.
    ///
    /// Agent-reported cwd (`w=`) wins over OSC when both are present.
    pub fn note_osc_cwd(&self, cwd: Option<&str>) {
        let Some(c) = cwd.filter(|s| !s.is_empty()) else {
            return;
        };
        let mut g = self.inner.lock().unwrap();
        g.osc_cwd = Some(c.to_string());
        if g.agent_cwd.is_some() {
            g.source = MetricsSource::Merged;
            return;
        }
        if let Some(ref mut st) = g.status {
            st.cwd = Some(c.to_string());
            g.source = MetricsSource::Osc;
        } else {
            g.status = Some(RemoteStatus {
                schema: 1,
                ts_ms: now_ms(),
                cwd: Some(c.to_string()),
                hostname: None,
                mem: None,
                disk: Vec::new(),
                cpu: None,
                uptime_secs: None,
                ext: serde_json::Value::Null,
            });
            g.source = MetricsSource::Osc;
        }
    }

    /// Full snapshot for status bar / Info tab (clone).
    pub fn snapshot(&self) -> Option<RemoteStatus> {
        self.inner.lock().unwrap().status.clone()
    }

    /// Copy of the rolling history (oldest → newest), at most [`METRICS_HISTORY_LEN`].
    pub fn history(&self) -> Vec<MetricsSample> {
        self.inner.lock().unwrap().history.iter().copied().collect()
    }

    /// Number of samples currently buffered.
    pub fn history_len(&self) -> usize {
        self.inner.lock().unwrap().history.len()
    }

    /// Compact one-line summary for a future status bar (no UI yet).
    ///
    /// Example: `host  load 0.50  mem 4.1/16.0G  disk 120G free`
    pub fn status_bar_line(&self) -> Option<String> {
        let st = self.snapshot()?;
        let host = st.hostname.as_deref().unwrap_or("?");
        let mut parts = vec![host.to_string()];
        if let Some(cpu) = &st.cpu {
            parts.push(format!("load {:.2}", cpu.load1));
        }
        if let Some(mem) = &st.mem {
            parts.push(format!(
                "mem {:.1}/{:.1}G",
                mem.used as f64 / 1e9,
                mem.total as f64 / 1e9
            ));
        }
        if let Some(d) = st.disk.first() {
            parts.push(format!("disk {:.0}G free", d.avail as f64 / 1e9));
        }
        if let Some(cwd) = &st.cwd {
            parts.push(cwd.clone());
        }
        Some(parts.join("  "))
    }

    pub fn source(&self) -> MetricsSource {
        self.inner.lock().unwrap().source
    }

    /// Prefer agent cwd, then merged status, then OSC, then `osc_fallback` from the screen.
    pub fn effective_cwd(&self, osc_fallback: Option<&str>) -> Option<String> {
        let g = self.inner.lock().unwrap();
        if let Some(c) = g.agent_cwd.clone() {
            return Some(c);
        }
        if let Some(c) = g.status.as_ref().and_then(|s| s.cwd.clone()) {
            return Some(c);
        }
        if let Some(c) = g.osc_cwd.clone() {
            return Some(c);
        }
        osc_fallback.filter(|s| !s.is_empty()).map(str::to_string)
    }

    pub fn clear(&self) {
        let mut g = self.inner.lock().unwrap();
        *g = SessionMetricsInner::default();
    }
}

/// Format a metrics event for debug logging.
pub fn format_metrics_event(ev: &MetricsEvent) -> String {
    match ev {
        MetricsEvent::AgentStarted { remote_path } => {
            format!("agent started path={remote_path}")
        }
        MetricsEvent::AgentStartFailed { error } => format!("agent start failed: {error}"),
        MetricsEvent::Hello { agent, ver } => format!("hello {agent} {ver}"),
        MetricsEvent::Status(st) => status_brief(st),
        MetricsEvent::AgentClosed { reason } => format!("agent closed: {reason}"),
        MetricsEvent::ParseError { error } => format!("parse error: {error}"),
    }
}

fn status_brief(st: &RemoteStatus) -> String {
    let host = st.hostname.as_deref().unwrap_or("?");
    let load = st
        .cpu
        .as_ref()
        .map(|c| format!("{:.2}", c.load1))
        .unwrap_or_else(|| "-".into());
    let mem = st
        .mem
        .as_ref()
        .map(|m| format!("{:.1}/{:.1}G", m.used as f64 / 1e9, m.total as f64 / 1e9))
        .unwrap_or_else(|| "-".into());
    let disk = st
        .disk
        .first()
        .map(|d| format!("{:.0}G free", d.avail as f64 / 1e9))
        .unwrap_or_else(|| "-".into());
    let cwd = st.cwd.as_deref().unwrap_or("(no cwd)");
    format!("status host={host} load={load} mem={mem} disk={disk} cwd={cwd}")
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merger_prefers_agent_then_fills_cwd_from_osc() {
        let m = SessionMetrics::new();
        m.apply_agent_status(RemoteStatus {
            schema: 1,
            ts_ms: 1,
            cwd: None,
            hostname: Some("box".into()),
            mem: Some(MemInfo {
                total: 100,
                avail: 40,
                used: 60,
            }),
            disk: Vec::new(),
            cpu: None,
            uptime_secs: None,
            ext: serde_json::Value::Null,
        });
        assert!(m.effective_cwd(None).is_none());
        m.note_osc_cwd(Some("/home/u"));
        assert_eq!(m.effective_cwd(None).as_deref(), Some("/home/u"));
        assert!(m.status_bar_line().unwrap().contains("box"));
        assert_eq!(m.source(), MetricsSource::Osc);
        let evs = m.drain_events();
        assert!(matches!(evs.first(), Some(MetricsEvent::Status(_))));
    }

    #[test]
    fn agent_cwd_wins_over_osc() {
        let m = SessionMetrics::new();
        m.note_osc_cwd(Some("/from-osc"));
        m.apply_agent_status(RemoteStatus {
            schema: 2,
            ts_ms: 1,
            cwd: Some("/from-agent".into()),
            hostname: None,
            mem: Some(MemInfo {
                total: 100,
                avail: 40,
                used: 60,
            }),
            disk: Vec::new(),
            cpu: None,
            uptime_secs: None,
            ext: serde_json::Value::Null,
        });
        assert_eq!(m.effective_cwd(None).as_deref(), Some("/from-agent"));
        // Later OSC must not override agent cwd.
        m.note_osc_cwd(Some("/from-osc-2"));
        assert_eq!(m.effective_cwd(None).as_deref(), Some("/from-agent"));
        // Missing OSC must not clear.
        m.note_osc_cwd(None);
        assert_eq!(m.effective_cwd(None).as_deref(), Some("/from-agent"));
    }

    #[test]
    fn merger_keeps_stable_fields_across_delta() {
        let m = SessionMetrics::new();
        m.apply_agent_status_ex(
            RemoteStatus {
                schema: 2,
                ts_ms: 1000,
                cwd: None,
                hostname: Some("box".into()),
                mem: Some(MemInfo {
                    total: 1024,
                    avail: 512,
                    used: 512,
                }),
                disk: vec![DiskInfo {
                    mount: "/".into(),
                    total: 4096,
                    avail: 2048,
                }],
                cpu: Some(CpuInfo {
                    load1: 0.1,
                    load5: 0.1,
                    load15: 0.1,
                    usage_percent: None,
                }),
                uptime_secs: None,
                ext: serde_json::Value::Null,
            },
            true,
        );
        // Delta: only mem/cpu/ts — host/disk must remain.
        m.apply_agent_status_ex(
            RemoteStatus {
                schema: 2,
                ts_ms: 2000,
                cwd: None,
                hostname: None,
                mem: Some(MemInfo {
                    total: 1024,
                    avail: 400,
                    used: 624,
                }),
                disk: Vec::new(),
                cpu: Some(CpuInfo {
                    load1: 0.2,
                    load5: 0.1,
                    load15: 0.1,
                    usage_percent: None,
                }),
                uptime_secs: None,
                ext: serde_json::Value::Null,
            },
            false,
        );
        let st = m.snapshot().unwrap();
        assert_eq!(st.hostname.as_deref(), Some("box"));
        assert_eq!(st.disk.len(), 1);
        assert_eq!(st.mem.unwrap().avail, 400);
        assert!((st.cpu.unwrap().load1 - 0.2).abs() < f64::EPSILON);
        assert_eq!(m.history_len(), 2);
        let hist = m.history();
        assert!((hist[1].load1 - 0.2).abs() < f32::EPSILON);
    }

    #[test]
    fn history_caps_at_sixty() {
        let m = SessionMetrics::new();
        for i in 0..70u64 {
            m.apply_agent_status(RemoteStatus {
                schema: 2,
                ts_ms: (i + 1) * 1000,
                cwd: None,
                hostname: Some("box".into()),
                mem: Some(MemInfo {
                    total: 1000,
                    avail: 500,
                    used: 500,
                }),
                disk: vec![DiskInfo {
                    mount: "/".into(),
                    total: 2000,
                    avail: 1000,
                }],
                cpu: Some(CpuInfo {
                    load1: i as f64 * 0.01,
                    load5: 0.0,
                    load15: 0.0,
                    usage_percent: None,
                }),
                uptime_secs: None,
                ext: serde_json::Value::Null,
            });
        }
        assert_eq!(m.history_len(), METRICS_HISTORY_LEN);
        assert_eq!(m.history().first().unwrap().ts_ms, 11_000);
    }
}
