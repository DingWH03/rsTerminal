//! Remote status agent protocol, metrics bus, and embedded reporter script.
//!
//! Wire format is compact tagged lines (`H`/`S`/`E`) with delta tags — see [`protocol`].
//!
//! ## UI consumers
//! Future status bar / Info tab should **only** read [`SessionMetrics::snapshot`] /
//! [`SessionMetrics::status_bar_line`] — do not open a second collector.

pub mod protocol;
pub mod status;

pub use protocol::{
    AgentToClient, ClientToAgent, MAX_FRAME_BYTES, PROTOCOL_VERSION, parse_agent_line, push_bytes,
};
pub use status::{
    CpuInfo, DiskInfo, METRICS_HISTORY_LEN, MemInfo, MetricsEvent, MetricsSample, MetricsSource,
    RemoteStatus, SessionMetrics, format_metrics_event,
};

/// POSIX shell agent uploaded to the remote host (no cross-compile).
pub const AGENT_SCRIPT: &str = include_str!("agent.sh");

pub fn agent_remote_path(session_tag: &str) -> String {
    let safe: String = session_tag
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .take(32)
        .collect();
    format!("/tmp/rsterm-agent-{safe}.sh")
}
