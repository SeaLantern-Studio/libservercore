#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigFormat {
    Yaml,
    Toml,
    Json,
    Properties,
    Text,
}

impl ConfigFormat {
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension.trim().to_ascii_lowercase().as_str() {
            "yml" | "yaml" => Some(Self::Yaml),
            "toml" => Some(Self::Toml),
            "json" => Some(Self::Json),
            "properties" => Some(Self::Properties),
            "txt" => Some(Self::Text),
            _ => None,
        }
    }
}
