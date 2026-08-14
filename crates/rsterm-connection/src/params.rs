//! Narrow connect parameters — connection layer must not take persist DTOs.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LocalConnectParams {
    pub shell: Option<String>,
    pub working_dir: Option<String>,
    pub env_vars: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct SshConnectParams {
    /// Opaque tag for logging / agent session identity (usually connection id).
    pub session_tag: String,
    pub host: String,
    pub port: u16,
    pub env_vars: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct SerialConnectParams {
    pub port: String,
    pub baud: u32,
}

#[derive(Debug, Clone)]
pub struct BleConnectParams {
    pub device: String,
}
