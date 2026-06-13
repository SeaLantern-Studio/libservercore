#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RconEndpoint {
    pub host: String,
    pub port: u16,
    pub password: String,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RconProbeOptions {
    pub core_key: Option<String>,
    pub connect_timeout_ms: u64,
    pub read_timeout_ms: u64,
}

impl RconEndpoint {
    pub fn address(&self) -> String {
        format!("{}:{}", self.host.trim(), self.port)
    }
}

impl Default for RconProbeOptions {
    fn default() -> Self {
        Self {
            core_key: None,
            connect_timeout_ms: 5_000,
            read_timeout_ms: 5_000,
        }
    }
}
