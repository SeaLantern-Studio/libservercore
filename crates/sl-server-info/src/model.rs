#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum InfoValue {
    Text(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
    StringList(Vec<String>),
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct InfoFact {
    pub namespace: &'static str,
    pub key: &'static str,
    pub value: InfoValue,
    pub source_command: &'static str,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeWarning {
    pub stage: &'static str,
    pub provider: Option<&'static str>,
    pub message: String,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnlinePlayersSnapshot {
    pub online: u32,
    pub max: Option<u32>,
    pub players: Vec<String>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct ServerInfoSnapshot {
    pub reachable: bool,
    pub latency_ms: Option<u64>,
    pub players: Option<OnlinePlayersSnapshot>,
    pub facts: Vec<InfoFact>,
    pub warnings: Vec<ProbeWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerInfoError {
    UnsupportedTransport(String),
    UnsupportedFamily(String),
    Connection(String),
    Authentication(String),
    Command(String),
    Parse(String),
}

impl std::fmt::Display for ServerInfoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedTransport(message)
            | Self::UnsupportedFamily(message)
            | Self::Connection(message)
            | Self::Authentication(message)
            | Self::Command(message)
            | Self::Parse(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ServerInfoError {}

impl ProbeWarning {
    pub fn new(
        stage: &'static str,
        provider: Option<&'static str>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            stage,
            provider,
            message: message.into(),
        }
    }
}
