//! Core Minecraft server taxonomy and normalized core-key helpers.

#![forbid(unsafe_code)]

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoreFamily {
    VanillaLike,
    BukkitLike,
    ForgeLike,
    FabricLike,
    ProxyLike,
    NativeExecutable,
    MixedExtension,
    Unknown,
}

impl CoreFamily {
    pub fn from_core_key(core_key: &str) -> Self {
        match normalize_core_key(core_key) {
            Some("paper")
            | Some("spigot")
            | Some("purpur")
            | Some("leaves")
            | Some("bukkit")
            | Some("vanilla") => Self::BukkitLike,
            Some("forge") | Some("neoforge") => Self::ForgeLike,
            Some("fabric") | Some("quilt") => Self::FabricLike,
            Some("velocity") | Some("lightfall") => Self::ProxyLike,
            Some("arclight")
            | Some("arclight_forge")
            | Some("arclight_neoforge")
            | Some("mohist")
            | Some("catserver") => Self::MixedExtension,
            Some("pumpkin") => Self::NativeExecutable,
            Some(_) | None => Self::Unknown,
        }
    }
}

pub fn normalize_core_key(value: &str) -> Option<&'static str> {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "paper" => Some("paper"),
        "spigot" => Some("spigot"),
        "purpur" => Some("purpur"),
        "leaves" => Some("leaves"),
        "bukkit" | "craftbukkit" => Some("bukkit"),
        "forge" => Some("forge"),
        "neoforge" => Some("neoforge"),
        "fabric" => Some("fabric"),
        "quilt" => Some("quilt"),
        "velocity" => Some("velocity"),
        "bungeecord" | "waterfall" | "lightfall" => Some("lightfall"),
        "arclight" => Some("arclight"),
        "arclight-forge" | "arclight_forge" => Some("arclight_forge"),
        "arclight-neoforge" | "arclight_neoforge" => Some("arclight_neoforge"),
        "mohist" => Some("mohist"),
        "catserver" => Some("catserver"),
        "pumpkin" => Some("pumpkin"),
        "vanilla" => Some("vanilla"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_core_key, CoreFamily};

    #[test]
    fn normalizes_known_aliases() {
        assert_eq!(normalize_core_key("Paper"), Some("paper"));
        assert_eq!(normalize_core_key("Waterfall"), Some("lightfall"));
        assert_eq!(normalize_core_key("Arclight-Neoforge"), Some("arclight_neoforge"));
        assert_eq!(normalize_core_key("unknown-core"), None);
    }

    #[test]
    fn classifies_known_core_families() {
        assert_eq!(CoreFamily::from_core_key("paper"), CoreFamily::BukkitLike);
        assert_eq!(CoreFamily::from_core_key("forge"), CoreFamily::ForgeLike);
        assert_eq!(CoreFamily::from_core_key("fabric"), CoreFamily::FabricLike);
        assert_eq!(CoreFamily::from_core_key("velocity"), CoreFamily::ProxyLike);
        assert_eq!(CoreFamily::from_core_key("pumpkin"), CoreFamily::NativeExecutable);
        assert_eq!(CoreFamily::from_core_key("arclight"), CoreFamily::MixedExtension);
        assert_eq!(CoreFamily::from_core_key("mystery"), CoreFamily::Unknown);
    }
}

