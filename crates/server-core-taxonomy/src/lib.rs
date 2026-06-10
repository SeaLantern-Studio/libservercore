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
            Some("vanilla") => Self::VanillaLike,
            Some("paper") | Some("folia") | Some("spigot") | Some("purpur")
            | Some("pufferfish") | Some("leaves") | Some("tuinity") | Some("airplane")
            | Some("glowstone") | Some("bukkit") => Self::BukkitLike,
            Some("forge") | Some("neoforge") => Self::ForgeLike,
            Some("fabric") | Some("quilt") => Self::FabricLike,
            Some("velocity") | Some("lightfall") | Some("travertine") | Some("flamecord") => {
                Self::ProxyLike
            }
            Some("arclight")
            | Some("arclight_forge")
            | Some("arclight_neoforge")
            | Some("mohist")
            | Some("catserver") => Self::MixedExtension,
            Some("pumpkin") | Some("cuberite") => Self::NativeExecutable,
            Some(_) | None => Self::Unknown,
        }
    }
}

pub fn normalize_core_key(value: &str) -> Option<&'static str> {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "paper" => Some("paper"),
        "folia" => Some("folia"),
        "spigot" => Some("spigot"),
        "purpur" => Some("purpur"),
        "pufferfish" => Some("pufferfish"),
        "leaves" => Some("leaves"),
        "tuinity" => Some("tuinity"),
        "airplane" | "paperairplane" | "paper-airplane" => Some("airplane"),
        "glowstone" => Some("glowstone"),
        "bukkit" | "craftbukkit" => Some("bukkit"),
        "forge" => Some("forge"),
        "neoforge" => Some("neoforge"),
        "fabric" => Some("fabric"),
        "quilt" => Some("quilt"),
        "velocity" => Some("velocity"),
        "bungeecord" | "waterfall" | "lightfall" => Some("lightfall"),
        "travertine" => Some("travertine"),
        "flamecord" => Some("flamecord"),
        "arclight" => Some("arclight"),
        "arclight-forge" | "arclight_forge" => Some("arclight_forge"),
        "arclight-neoforge" | "arclight_neoforge" => Some("arclight_neoforge"),
        "mohist" => Some("mohist"),
        "catserver" => Some("catserver"),
        "pumpkin" => Some("pumpkin"),
        "cuberite" => Some("cuberite"),
        "minestom" => Some("minestom"),
        "sponge" => Some("sponge"),
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
        assert_eq!(normalize_core_key("Paper-Airplane"), Some("airplane"));
        assert_eq!(
            normalize_core_key("Arclight-Neoforge"),
            Some("arclight_neoforge")
        );
        assert_eq!(normalize_core_key("Folia"), Some("folia"));
        assert_eq!(normalize_core_key("FlameCord"), Some("flamecord"));
        assert_eq!(normalize_core_key("unknown-core"), None);
    }

    #[test]
    fn classifies_known_core_families() {
        assert_eq!(
            CoreFamily::from_core_key("vanilla"),
            CoreFamily::VanillaLike
        );
        assert_eq!(CoreFamily::from_core_key("paper"), CoreFamily::BukkitLike);
        assert_eq!(CoreFamily::from_core_key("folia"), CoreFamily::BukkitLike);
        assert_eq!(
            CoreFamily::from_core_key("glowstone"),
            CoreFamily::BukkitLike
        );
        assert_eq!(CoreFamily::from_core_key("forge"), CoreFamily::ForgeLike);
        assert_eq!(CoreFamily::from_core_key("fabric"), CoreFamily::FabricLike);
        assert_eq!(CoreFamily::from_core_key("velocity"), CoreFamily::ProxyLike);
        assert_eq!(
            CoreFamily::from_core_key("travertine"),
            CoreFamily::ProxyLike
        );
        assert_eq!(
            CoreFamily::from_core_key("pumpkin"),
            CoreFamily::NativeExecutable
        );
        assert_eq!(
            CoreFamily::from_core_key("cuberite"),
            CoreFamily::NativeExecutable
        );
        assert_eq!(
            CoreFamily::from_core_key("arclight"),
            CoreFamily::MixedExtension
        );
        assert_eq!(CoreFamily::from_core_key("minestom"), CoreFamily::Unknown);
        assert_eq!(CoreFamily::from_core_key("sponge"), CoreFamily::Unknown);
        assert_eq!(CoreFamily::from_core_key("mystery"), CoreFamily::Unknown);
    }
}
