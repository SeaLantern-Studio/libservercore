//! Core server flavor modeling and capability resolution.

#![forbid(unsafe_code)]

pub use server_core_taxonomy::{normalize_core_key, CoreFamily};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServerFlavorKind {
    VanillaLike,
    BukkitLike,
    ForgeLike,
    FabricLike,
    EmbeddedJavaLike,
    ProxyLike,
    BedrockLike,
    NativeExecutable,
    WrappedServer,
    Unknown,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServerEdition {
    Java,
    Bedrock,
    Unknown,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServerRole {
    GameServer,
    Proxy,
    Wrapper,
    Unknown,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServerExtensionKind {
    Plugin,
    Mod,
    Datapack,
    Addon,
    McdrPlugin,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StartupMode {
    Jar,
    Exe,
    Bat,
    Sh,
    Ps1,
    Starter,
    Custom,
}

impl StartupMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Jar => "jar",
            Self::Exe => "exe",
            Self::Bat => "bat",
            Self::Sh => "sh",
            Self::Ps1 => "ps1",
            Self::Starter => "starter",
            Self::Custom => "custom",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "jar" => Some(Self::Jar),
            "exe" => Some(Self::Exe),
            "bat" | "cmd" => Some(Self::Bat),
            "sh" => Some(Self::Sh),
            "ps1" => Some(Self::Ps1),
            "starter" => Some(Self::Starter),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlChannel {
    Stdin,
    Rcon,
    DockerStdio,
    WrapperConsole,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecialConfigKind {
    ServerProperties,
    PumpkinToml,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigSurfaceOwner {
    ServerCore,
    Plugin,
    FallbackDirectory,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigFormat {
    Yaml,
    Toml,
    Json,
    Properties,
    Text,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigSurfaceKind {
    CanonicalFile,
    PluginDirectory,
    FallbackDirectory,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConfigSurface {
    pub key: &'static str,
    pub owner: ConfigSurfaceOwner,
    pub kind: ConfigSurfaceKind,
    pub relative_path: &'static str,
    pub format: Option<ConfigFormat>,
    pub recursive: bool,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WrapperKind {
    Mcdr,
    Generic,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeFamily {
    Local,
    DockerItzg,
    Other,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerFlavorProfile {
    pub flavor_kind: ServerFlavorKind,
    pub edition: ServerEdition,
    pub server_role: ServerRole,
    pub display_key: &'static str,
    pub detected_core_key: Option<&'static str>,
    pub default_startup_mode: Option<StartupMode>,
    pub preferred_candidate_modes: Vec<StartupMode>,
    pub requires_java: bool,
    pub supports_starter_install: bool,
    pub supports_custom_wrapper: bool,
    pub extension_kinds: Vec<ServerExtensionKind>,
    pub default_extension_kind: Option<ServerExtensionKind>,
    pub allow_manual_extension_switch: bool,
    pub preferred_control_channel: ControlChannel,
    pub special_config_kinds: Vec<SpecialConfigKind>,
    pub config_surfaces: Vec<ConfigSurface>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FlavorResolutionInput<'a> {
    pub core_key: Option<&'a str>,
    pub runtime_kind: Option<&'a str>,
    pub startup_mode: Option<&'a str>,
    pub wrapper_kind: Option<WrapperKind>,
    pub has_pumpkin_config: bool,
}

impl<'a> FlavorResolutionInput<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_core_key(mut self, core_key: &'a str) -> Self {
        self.core_key = Some(core_key);
        self
    }

    pub fn with_runtime_kind(mut self, runtime_kind: &'a str) -> Self {
        self.runtime_kind = Some(runtime_kind);
        self
    }

    pub fn with_startup_mode(mut self, startup_mode: &'a str) -> Self {
        self.startup_mode = Some(startup_mode);
        self
    }

    pub fn with_wrapper_kind(mut self, wrapper_kind: WrapperKind) -> Self {
        self.wrapper_kind = Some(wrapper_kind);
        self
    }

    pub fn with_pumpkin_config(mut self, has_pumpkin_config: bool) -> Self {
        self.has_pumpkin_config = has_pumpkin_config;
        self
    }
}

impl ServerFlavorProfile {
    pub fn supports_extension_kind(&self, kind: ServerExtensionKind) -> bool {
        self.extension_kinds.contains(&kind)
    }

    pub fn prefers_custom_startup(&self) -> bool {
        self.default_startup_mode == Some(StartupMode::Custom)
    }

    pub fn is_proxy(&self) -> bool {
        self.server_role == ServerRole::Proxy
    }

    pub fn is_wrapper(&self) -> bool {
        self.server_role == ServerRole::Wrapper
    }

    pub fn config_surface(&self, key: &str) -> Option<&ConfigSurface> {
        self.config_surfaces.iter().find(|surface| surface.key == key)
    }
}

pub fn resolve_profile_from_parts(
    core_key: Option<&str>,
    runtime_kind: Option<&str>,
    startup_mode: Option<&str>,
    wrapper_kind: Option<WrapperKind>,
    has_pumpkin_config: bool,
) -> ServerFlavorProfile {
    resolve_server_flavor_profile(&FlavorResolutionInput {
        core_key,
        runtime_kind,
        startup_mode,
        wrapper_kind,
        has_pumpkin_config,
    })
}

pub fn resolve_server_flavor_profile(input: &FlavorResolutionInput<'_>) -> ServerFlavorProfile {
    if let Some(wrapper_kind) = input.wrapper_kind {
        return wrapped_profile(wrapper_kind, input);
    }

    let normalized_core = input.core_key.and_then(normalize_core_key);
    match normalized_core {
        Some("vanilla") => vanilla_like_profile(normalized_core),
        Some("paper") | Some("folia") | Some("spigot") | Some("purpur") | Some("pufferfish")
        | Some("leaves") | Some("tuinity") | Some("airplane") | Some("glowstone")
        | Some("bukkit") => bukkit_like_profile(normalized_core),
        Some("forge") | Some("neoforge") => forge_like_profile(normalized_core),
        Some("fabric") | Some("quilt") => fabric_like_profile(normalized_core),
        Some("sponge") | Some("minestom") => embedded_java_profile(normalized_core),
        Some("velocity") | Some("bungeecord") | Some("waterfall") | Some("lightfall")
        | Some("travertine") | Some("flamecord") => proxy_like_profile(normalized_core),
        Some("bds") => bedrock_dedicated_profile(normalized_core),
        Some("liteloaderbds") | Some("levilamina") | Some("bdsx") => {
            bedrock_wrapped_profile(normalized_core)
        }
        Some("allay") | Some("nukkit") | Some("powernukkitx") => {
            bedrock_java_plugin_profile(normalized_core)
        }
        Some("pocketmine") => bedrock_script_plugin_profile(normalized_core),
        Some("endstone") => bedrock_native_plugin_profile(normalized_core),
        Some("arclight")
        | Some("arclight_forge")
        | Some("arclight_neoforge")
        | Some("mohist")
        | Some("catserver") => mixed_extension_profile(normalized_core),
        Some("pumpkin") => pumpkin_profile(),
        Some("cuberite") => native_executable_profile(Some("cuberite"), "cuberite"),
        Some(_) | None => fallback_profile(input),
    }
}

pub fn runtime_family(runtime_kind: Option<&str>) -> RuntimeFamily {
    match runtime_kind.map(|value| value.trim().to_ascii_lowercase()) {
        Some(value) if value == "local" => RuntimeFamily::Local,
        Some(value) if value == "docker_itzg" => RuntimeFamily::DockerItzg,
        Some(_) => RuntimeFamily::Other,
        None => RuntimeFamily::Local,
    }
}

fn wrapped_profile(
    wrapper_kind: WrapperKind,
    input: &FlavorResolutionInput<'_>,
) -> ServerFlavorProfile {
    let (display_key, extension_kind) = match wrapper_kind {
        WrapperKind::Mcdr => ("mcdr_wrapped", ServerExtensionKind::McdrPlugin),
        WrapperKind::Generic => ("wrapped_server", ServerExtensionKind::Datapack),
    };

    ServerFlavorProfile {
        flavor_kind: ServerFlavorKind::WrappedServer,
        edition: match input.core_key.and_then(normalize_core_key) {
            Some("bds")
            | Some("liteloaderbds")
            | Some("levilamina")
            | Some("bdsx")
            | Some("allay")
            | Some("nukkit")
            | Some("powernukkitx")
            | Some("pocketmine")
            | Some("endstone") => ServerEdition::Bedrock,
            Some(_) => ServerEdition::Java,
            None => ServerEdition::Unknown,
        },
        server_role: ServerRole::Wrapper,
        display_key,
        detected_core_key: input.core_key.and_then(normalize_core_key),
        default_startup_mode: Some(StartupMode::Custom),
        preferred_candidate_modes: vec![StartupMode::Custom],
        requires_java: !matches!(input.core_key.and_then(normalize_core_key), Some("pumpkin")),
        supports_starter_install: false,
        supports_custom_wrapper: true,
        extension_kinds: vec![extension_kind],
        default_extension_kind: Some(extension_kind),
        allow_manual_extension_switch: false,
        preferred_control_channel: ControlChannel::WrapperConsole,
        special_config_kinds: special_configs(input),
        config_surfaces: config_surfaces(input),
    }
}

fn vanilla_like_profile(core_key: Option<&'static str>) -> ServerFlavorProfile {
    ServerFlavorProfile {
        flavor_kind: ServerFlavorKind::VanillaLike,
        edition: ServerEdition::Java,
        server_role: ServerRole::GameServer,
        display_key: "vanilla_like",
        detected_core_key: core_key,
        default_startup_mode: Some(StartupMode::Jar),
        preferred_candidate_modes: vec![StartupMode::Jar, StartupMode::Sh, StartupMode::Bat],
        requires_java: true,
        supports_starter_install: false,
        supports_custom_wrapper: true,
        extension_kinds: vec![ServerExtensionKind::Datapack],
        default_extension_kind: Some(ServerExtensionKind::Datapack),
        allow_manual_extension_switch: false,
        preferred_control_channel: ControlChannel::Stdin,
        special_config_kinds: vec![SpecialConfigKind::ServerProperties],
        config_surfaces: config_surfaces_for_core(core_key),
    }
}

fn bukkit_like_profile(core_key: Option<&'static str>) -> ServerFlavorProfile {
    ServerFlavorProfile {
        flavor_kind: ServerFlavorKind::BukkitLike,
        edition: ServerEdition::Java,
        server_role: ServerRole::GameServer,
        display_key: "bukkit_like",
        detected_core_key: core_key,
        default_startup_mode: Some(StartupMode::Jar),
        preferred_candidate_modes: vec![StartupMode::Jar, StartupMode::Sh, StartupMode::Bat],
        requires_java: true,
        supports_starter_install: false,
        supports_custom_wrapper: true,
        extension_kinds: vec![ServerExtensionKind::Plugin, ServerExtensionKind::Datapack],
        default_extension_kind: Some(ServerExtensionKind::Plugin),
        allow_manual_extension_switch: false,
        preferred_control_channel: ControlChannel::Stdin,
        special_config_kinds: vec![SpecialConfigKind::ServerProperties],
        config_surfaces: config_surfaces_for_core(core_key),
    }
}

fn forge_like_profile(core_key: Option<&'static str>) -> ServerFlavorProfile {
    ServerFlavorProfile {
        flavor_kind: ServerFlavorKind::ForgeLike,
        edition: ServerEdition::Java,
        server_role: ServerRole::GameServer,
        display_key: "forge_like",
        detected_core_key: core_key,
        default_startup_mode: Some(StartupMode::Starter),
        preferred_candidate_modes: vec![StartupMode::Starter, StartupMode::Jar],
        requires_java: true,
        supports_starter_install: true,
        supports_custom_wrapper: true,
        extension_kinds: vec![ServerExtensionKind::Mod, ServerExtensionKind::Datapack],
        default_extension_kind: Some(ServerExtensionKind::Mod),
        allow_manual_extension_switch: false,
        preferred_control_channel: ControlChannel::Stdin,
        special_config_kinds: vec![SpecialConfigKind::ServerProperties],
        config_surfaces: config_surfaces_for_core(core_key),
    }
}

fn fabric_like_profile(core_key: Option<&'static str>) -> ServerFlavorProfile {
    ServerFlavorProfile {
        flavor_kind: ServerFlavorKind::FabricLike,
        edition: ServerEdition::Java,
        server_role: ServerRole::GameServer,
        display_key: "fabric_like",
        detected_core_key: core_key,
        default_startup_mode: Some(StartupMode::Jar),
        preferred_candidate_modes: vec![StartupMode::Jar, StartupMode::Sh, StartupMode::Bat],
        requires_java: true,
        supports_starter_install: false,
        supports_custom_wrapper: true,
        extension_kinds: vec![ServerExtensionKind::Mod, ServerExtensionKind::Datapack],
        default_extension_kind: Some(ServerExtensionKind::Mod),
        allow_manual_extension_switch: false,
        preferred_control_channel: ControlChannel::Stdin,
        special_config_kinds: vec![SpecialConfigKind::ServerProperties],
        config_surfaces: config_surfaces_for_core(core_key),
    }
}

fn embedded_java_profile(core_key: Option<&'static str>) -> ServerFlavorProfile {
    ServerFlavorProfile {
        flavor_kind: ServerFlavorKind::EmbeddedJavaLike,
        edition: ServerEdition::Java,
        server_role: ServerRole::GameServer,
        display_key: "embedded_java_like",
        detected_core_key: core_key,
        default_startup_mode: Some(StartupMode::Jar),
        preferred_candidate_modes: vec![StartupMode::Jar, StartupMode::Sh, StartupMode::Bat],
        requires_java: true,
        supports_starter_install: false,
        supports_custom_wrapper: true,
        extension_kinds: vec![ServerExtensionKind::Plugin, ServerExtensionKind::Datapack],
        default_extension_kind: Some(ServerExtensionKind::Plugin),
        allow_manual_extension_switch: false,
        preferred_control_channel: ControlChannel::Stdin,
        special_config_kinds: vec![SpecialConfigKind::ServerProperties],
        config_surfaces: config_surfaces_for_core(core_key),
    }
}

fn proxy_like_profile(core_key: Option<&'static str>) -> ServerFlavorProfile {
    ServerFlavorProfile {
        flavor_kind: ServerFlavorKind::ProxyLike,
        edition: ServerEdition::Java,
        server_role: ServerRole::Proxy,
        display_key: "proxy_like",
        detected_core_key: core_key,
        default_startup_mode: Some(StartupMode::Jar),
        preferred_candidate_modes: vec![StartupMode::Jar],
        requires_java: true,
        supports_starter_install: false,
        supports_custom_wrapper: false,
        extension_kinds: vec![ServerExtensionKind::Plugin],
        default_extension_kind: Some(ServerExtensionKind::Plugin),
        allow_manual_extension_switch: false,
        preferred_control_channel: ControlChannel::Stdin,
        special_config_kinds: Vec::new(),
        config_surfaces: config_surfaces_for_core(core_key),
    }
}

fn bedrock_dedicated_profile(core_key: Option<&'static str>) -> ServerFlavorProfile {
    ServerFlavorProfile {
        flavor_kind: ServerFlavorKind::BedrockLike,
        edition: ServerEdition::Bedrock,
        server_role: ServerRole::GameServer,
        display_key: "bedrock_dedicated",
        detected_core_key: core_key,
        default_startup_mode: Some(StartupMode::Exe),
        preferred_candidate_modes: vec![
            StartupMode::Exe,
            StartupMode::Bat,
            StartupMode::Ps1,
            StartupMode::Sh,
        ],
        requires_java: false,
        supports_starter_install: false,
        supports_custom_wrapper: true,
        extension_kinds: vec![ServerExtensionKind::Addon],
        default_extension_kind: Some(ServerExtensionKind::Addon),
        allow_manual_extension_switch: false,
        preferred_control_channel: ControlChannel::Stdin,
        special_config_kinds: vec![SpecialConfigKind::ServerProperties],
        config_surfaces: config_surfaces_for_core(core_key),
    }
}

fn bedrock_wrapped_profile(core_key: Option<&'static str>) -> ServerFlavorProfile {
    ServerFlavorProfile {
        flavor_kind: ServerFlavorKind::BedrockLike,
        edition: ServerEdition::Bedrock,
        server_role: ServerRole::GameServer,
        display_key: "bedrock_wrapped",
        detected_core_key: core_key,
        default_startup_mode: Some(StartupMode::Exe),
        preferred_candidate_modes: vec![
            StartupMode::Exe,
            StartupMode::Bat,
            StartupMode::Ps1,
            StartupMode::Sh,
            StartupMode::Custom,
        ],
        requires_java: false,
        supports_starter_install: false,
        supports_custom_wrapper: true,
        extension_kinds: vec![ServerExtensionKind::Plugin, ServerExtensionKind::Addon],
        default_extension_kind: Some(ServerExtensionKind::Plugin),
        allow_manual_extension_switch: true,
        preferred_control_channel: ControlChannel::Stdin,
        special_config_kinds: vec![SpecialConfigKind::ServerProperties],
        config_surfaces: config_surfaces_for_core(core_key),
    }
}

fn bedrock_java_plugin_profile(core_key: Option<&'static str>) -> ServerFlavorProfile {
    ServerFlavorProfile {
        flavor_kind: ServerFlavorKind::BedrockLike,
        edition: ServerEdition::Bedrock,
        server_role: ServerRole::GameServer,
        display_key: "bedrock_plugin_java",
        detected_core_key: core_key,
        default_startup_mode: Some(StartupMode::Jar),
        preferred_candidate_modes: vec![StartupMode::Jar, StartupMode::Sh, StartupMode::Bat],
        requires_java: true,
        supports_starter_install: false,
        supports_custom_wrapper: true,
        extension_kinds: vec![ServerExtensionKind::Plugin],
        default_extension_kind: Some(ServerExtensionKind::Plugin),
        allow_manual_extension_switch: false,
        preferred_control_channel: ControlChannel::Stdin,
        special_config_kinds: Vec::new(),
        config_surfaces: config_surfaces_for_core(core_key),
    }
}

fn bedrock_script_plugin_profile(core_key: Option<&'static str>) -> ServerFlavorProfile {
    ServerFlavorProfile {
        flavor_kind: ServerFlavorKind::BedrockLike,
        edition: ServerEdition::Bedrock,
        server_role: ServerRole::GameServer,
        display_key: "bedrock_plugin_script",
        detected_core_key: core_key,
        default_startup_mode: Some(StartupMode::Custom),
        preferred_candidate_modes: vec![StartupMode::Custom, StartupMode::Bat, StartupMode::Sh],
        requires_java: false,
        supports_starter_install: false,
        supports_custom_wrapper: true,
        extension_kinds: vec![ServerExtensionKind::Plugin],
        default_extension_kind: Some(ServerExtensionKind::Plugin),
        allow_manual_extension_switch: false,
        preferred_control_channel: ControlChannel::Stdin,
        special_config_kinds: Vec::new(),
        config_surfaces: config_surfaces_for_core(core_key),
    }
}

fn bedrock_native_plugin_profile(core_key: Option<&'static str>) -> ServerFlavorProfile {
    ServerFlavorProfile {
        flavor_kind: ServerFlavorKind::BedrockLike,
        edition: ServerEdition::Bedrock,
        server_role: ServerRole::GameServer,
        display_key: "bedrock_plugin_native",
        detected_core_key: core_key,
        default_startup_mode: Some(StartupMode::Exe),
        preferred_candidate_modes: vec![
            StartupMode::Exe,
            StartupMode::Bat,
            StartupMode::Ps1,
            StartupMode::Sh,
            StartupMode::Custom,
        ],
        requires_java: false,
        supports_starter_install: false,
        supports_custom_wrapper: true,
        extension_kinds: vec![ServerExtensionKind::Plugin],
        default_extension_kind: Some(ServerExtensionKind::Plugin),
        allow_manual_extension_switch: false,
        preferred_control_channel: ControlChannel::Stdin,
        special_config_kinds: Vec::new(),
        config_surfaces: config_surfaces_for_core(core_key),
    }
}

fn mixed_extension_profile(core_key: Option<&'static str>) -> ServerFlavorProfile {
    ServerFlavorProfile {
        flavor_kind: ServerFlavorKind::ForgeLike,
        edition: ServerEdition::Java,
        server_role: ServerRole::GameServer,
        display_key: "mixed_extension_server",
        detected_core_key: core_key,
        default_startup_mode: Some(StartupMode::Jar),
        preferred_candidate_modes: vec![StartupMode::Jar, StartupMode::Starter],
        requires_java: true,
        supports_starter_install: matches!(core_key, Some("arclight_neoforge")),
        supports_custom_wrapper: true,
        extension_kinds: vec![
            ServerExtensionKind::Plugin,
            ServerExtensionKind::Mod,
            ServerExtensionKind::Datapack,
        ],
        default_extension_kind: Some(ServerExtensionKind::Plugin),
        allow_manual_extension_switch: true,
        preferred_control_channel: ControlChannel::Stdin,
        special_config_kinds: vec![SpecialConfigKind::ServerProperties],
        config_surfaces: config_surfaces_for_core(core_key),
    }
}

fn pumpkin_profile() -> ServerFlavorProfile {
    native_executable_profile(Some("pumpkin"), "pumpkin")
}

fn native_executable_profile(
    core_key: Option<&'static str>,
    display_key: &'static str,
) -> ServerFlavorProfile {
    let special_config_kinds = if matches!(core_key, Some("pumpkin")) {
        vec![SpecialConfigKind::PumpkinToml]
    } else {
        Vec::new()
    };

    ServerFlavorProfile {
        flavor_kind: ServerFlavorKind::NativeExecutable,
        edition: ServerEdition::Java,
        server_role: ServerRole::GameServer,
        display_key,
        detected_core_key: core_key,
        default_startup_mode: Some(StartupMode::Custom),
        preferred_candidate_modes: vec![StartupMode::Custom, StartupMode::Bat],
        requires_java: false,
        supports_starter_install: false,
        supports_custom_wrapper: true,
        extension_kinds: vec![ServerExtensionKind::Datapack],
        default_extension_kind: Some(ServerExtensionKind::Datapack),
        allow_manual_extension_switch: false,
        preferred_control_channel: ControlChannel::Stdin,
        special_config_kinds,
        config_surfaces: config_surfaces_for_core(core_key),
    }
}

fn fallback_profile(input: &FlavorResolutionInput<'_>) -> ServerFlavorProfile {
    let startup_mode = input.startup_mode.and_then(StartupMode::parse);
    let runtime = runtime_family(input.runtime_kind);
    let preferred_control_channel = match runtime {
        RuntimeFamily::DockerItzg => ControlChannel::Rcon,
        RuntimeFamily::Local | RuntimeFamily::Other => {
            if matches!(startup_mode, Some(StartupMode::Custom)) {
                ControlChannel::WrapperConsole
            } else {
                ControlChannel::Stdin
            }
        }
    };

    ServerFlavorProfile {
        flavor_kind: if matches!(startup_mode, Some(StartupMode::Custom)) {
            ServerFlavorKind::WrappedServer
        } else {
            ServerFlavorKind::Unknown
        },
        edition: ServerEdition::Unknown,
        server_role: if matches!(startup_mode, Some(StartupMode::Custom)) {
            ServerRole::Wrapper
        } else {
            ServerRole::Unknown
        },
        display_key: if matches!(startup_mode, Some(StartupMode::Custom)) {
            "custom_server"
        } else {
            "unknown_server"
        },
        detected_core_key: input.core_key.and_then(normalize_core_key),
        default_startup_mode: startup_mode.or(Some(StartupMode::Jar)),
        preferred_candidate_modes: match startup_mode {
            Some(mode) => vec![mode],
            None => vec![StartupMode::Jar, StartupMode::Custom],
        },
        requires_java: !input.has_pumpkin_config,
        supports_starter_install: false,
        supports_custom_wrapper: true,
        extension_kinds: vec![
            ServerExtensionKind::Plugin,
            ServerExtensionKind::Mod,
            ServerExtensionKind::Datapack,
        ],
        default_extension_kind: None,
        allow_manual_extension_switch: true,
        preferred_control_channel,
        special_config_kinds: special_configs(input),
        config_surfaces: config_surfaces(input),
    }
}

fn config_surfaces(input: &FlavorResolutionInput<'_>) -> Vec<ConfigSurface> {
    config_surfaces_for_core(input.core_key.and_then(normalize_core_key))
}

fn config_surfaces_for_core(core_key: Option<&'static str>) -> Vec<ConfigSurface> {
    let mut surfaces = base_config_surfaces();

    match core_key {
        Some("paper") | Some("folia") | Some("purpur") | Some("pufferfish") | Some("leaves") => {
            surfaces.extend([
                canonical_file("bukkit_yml", "bukkit.yml", ConfigFormat::Yaml),
                canonical_file("spigot_yml", "spigot.yml", ConfigFormat::Yaml),
                canonical_file("paper_yml", "paper.yml", ConfigFormat::Yaml),
                canonical_file("paper_yaml", "paper.yaml", ConfigFormat::Yaml),
                canonical_file("paper_global_yml", "config/paper-global.yml", ConfigFormat::Yaml),
                canonical_file("paper_global_yaml", "config/paper-global.yaml", ConfigFormat::Yaml),
                canonical_file(
                    "paper_world_defaults_yml",
                    "config/paper-world-defaults.yml",
                    ConfigFormat::Yaml,
                ),
                canonical_file(
                    "paper_world_defaults_yaml",
                    "config/paper-world-defaults.yaml",
                    ConfigFormat::Yaml,
                ),
            ]);
        }
        Some("spigot") | Some("bukkit") | Some("glowstone") | Some("tuinity") | Some("airplane") => {
            surfaces.extend([
                canonical_file("bukkit_yml", "bukkit.yml", ConfigFormat::Yaml),
                canonical_file("spigot_yml", "spigot.yml", ConfigFormat::Yaml),
            ]);

            if matches!(core_key, Some("tuinity") | Some("airplane")) {
                surfaces.push(canonical_file("paper_yml", "paper.yml", ConfigFormat::Yaml));
            }
        }
        Some("pumpkin") => {
            surfaces.push(canonical_file("pumpkin_toml", "pumpkin.toml", ConfigFormat::Toml));
        }
        _ => {}
    }

    dedup_config_surfaces(surfaces)
}

fn base_config_surfaces() -> Vec<ConfigSurface> {
    vec![
        canonical_file("server_properties", "server.properties", ConfigFormat::Properties),
        plugin_directory("plugins_root", "plugins"),
        fallback_directory("defaultconfig", "defaultconfig"),
        fallback_directory("defaultconfigs", "defaultconfigs"),
        fallback_directory("config", "config"),
        fallback_directory("configs", "configs"),
    ]
}

fn canonical_file(
    key: &'static str,
    relative_path: &'static str,
    format: ConfigFormat,
) -> ConfigSurface {
    ConfigSurface {
        key,
        owner: ConfigSurfaceOwner::ServerCore,
        kind: ConfigSurfaceKind::CanonicalFile,
        relative_path,
        format: Some(format),
        recursive: false,
    }
}

fn plugin_directory(key: &'static str, relative_path: &'static str) -> ConfigSurface {
    ConfigSurface {
        key,
        owner: ConfigSurfaceOwner::Plugin,
        kind: ConfigSurfaceKind::PluginDirectory,
        relative_path,
        format: None,
        recursive: true,
    }
}

fn fallback_directory(key: &'static str, relative_path: &'static str) -> ConfigSurface {
    ConfigSurface {
        key,
        owner: ConfigSurfaceOwner::FallbackDirectory,
        kind: ConfigSurfaceKind::FallbackDirectory,
        relative_path,
        format: None,
        recursive: true,
    }
}

fn dedup_config_surfaces(surfaces: Vec<ConfigSurface>) -> Vec<ConfigSurface> {
    let mut deduped = Vec::new();

    for surface in surfaces {
        if !deduped.iter().any(|existing: &ConfigSurface| existing.key == surface.key) {
            deduped.push(surface);
        }
    }

    deduped
}

fn special_configs(input: &FlavorResolutionInput<'_>) -> Vec<SpecialConfigKind> {
    let mut kinds = Vec::new();
    if input.has_pumpkin_config
        || matches!(input.core_key.and_then(normalize_core_key), Some("pumpkin"))
    {
        kinds.push(SpecialConfigKind::PumpkinToml);
    }
    if !matches!(input.core_key.and_then(normalize_core_key), Some("pumpkin")) {
        kinds.push(SpecialConfigKind::ServerProperties);
    }
    kinds
}

#[cfg(test)]
mod tests {
    use super::{
        normalize_core_key, resolve_server_flavor_profile, runtime_family, ConfigFormat,
        ConfigSurfaceKind, ConfigSurfaceOwner, ControlChannel, FlavorResolutionInput,
        RuntimeFamily, ServerEdition, ServerExtensionKind, ServerFlavorKind, ServerRole,
        SpecialConfigKind, StartupMode, WrapperKind,
    };

    #[test]
    fn normalizes_known_core_aliases() {
        assert_eq!(normalize_core_key("Paper"), Some("paper"));
        assert_eq!(
            normalize_core_key("Arclight-Neoforge"),
            Some("arclight_neoforge")
        );
        assert_eq!(normalize_core_key("Waterfall"), Some("waterfall"));
        assert_eq!(normalize_core_key("BungeeCord"), Some("bungeecord"));
        assert_eq!(normalize_core_key("Folia"), Some("folia"));
        assert_eq!(normalize_core_key("Paper-Airplane"), Some("airplane"));
        assert_eq!(normalize_core_key("bedrock-dedicated-server"), Some("bds"));
        assert_eq!(normalize_core_key("LiteLoader-BDS"), Some("liteloaderbds"));
        assert_eq!(normalize_core_key("AllayMC"), Some("allay"));
        assert_eq!(normalize_core_key("unknown-core"), None);
    }

    #[test]
    fn resolves_vanilla_like_profile() {
        let profile = resolve_server_flavor_profile(&FlavorResolutionInput {
            core_key: Some("vanilla"),
            ..FlavorResolutionInput::default()
        });

        assert_eq!(profile.flavor_kind, ServerFlavorKind::VanillaLike);
        assert_eq!(profile.edition, ServerEdition::Java);
        assert_eq!(profile.server_role, ServerRole::GameServer);
        assert_eq!(
            profile.default_extension_kind,
            Some(ServerExtensionKind::Datapack)
        );
        assert_eq!(profile.detected_core_key, Some("vanilla"));
    }

    #[test]
    fn resolves_bukkit_like_profile() {
        let profile = resolve_server_flavor_profile(&FlavorResolutionInput {
            core_key: Some("paper"),
            ..FlavorResolutionInput::default()
        });

        assert_eq!(profile.flavor_kind, ServerFlavorKind::BukkitLike);
        assert_eq!(profile.edition, ServerEdition::Java);
        assert_eq!(profile.server_role, ServerRole::GameServer);
        assert_eq!(profile.default_startup_mode, Some(StartupMode::Jar));
        assert_eq!(
            profile.default_extension_kind,
            Some(ServerExtensionKind::Plugin)
        );
        assert!(!profile.allow_manual_extension_switch);
    }

    #[test]
    fn resolves_additional_bukkit_like_forks() {
        for core_key in ["folia", "pufferfish", "tuinity", "airplane", "glowstone"] {
            let profile = resolve_server_flavor_profile(&FlavorResolutionInput {
                core_key: Some(core_key),
                ..FlavorResolutionInput::default()
            });

            assert_eq!(
                profile.flavor_kind,
                ServerFlavorKind::BukkitLike,
                "{core_key}"
            );
            assert_eq!(
                profile.default_extension_kind,
                Some(ServerExtensionKind::Plugin),
                "{core_key}"
            );
            assert_eq!(profile.edition, ServerEdition::Java, "{core_key}");
        }
    }

    #[test]
    fn resolves_embedded_java_profiles() {
        for core_key in ["sponge", "minestom"] {
            let profile = resolve_server_flavor_profile(&FlavorResolutionInput {
                core_key: Some(core_key),
                ..FlavorResolutionInput::default()
            });

            assert_eq!(
                profile.flavor_kind,
                ServerFlavorKind::EmbeddedJavaLike,
                "{core_key}"
            );
            assert_eq!(profile.edition, ServerEdition::Java, "{core_key}");
            assert_eq!(profile.server_role, ServerRole::GameServer, "{core_key}");
            assert_eq!(
                profile.default_extension_kind,
                Some(ServerExtensionKind::Plugin),
                "{core_key}"
            );
            assert!(
                profile.supports_extension_kind(ServerExtensionKind::Plugin),
                "{core_key}"
            );
            assert!(!profile.allow_manual_extension_switch, "{core_key}");
        }
    }

    #[test]
    fn resolves_forge_like_profile_with_starter() {
        let profile = resolve_server_flavor_profile(&FlavorResolutionInput {
            core_key: Some("neoforge"),
            ..FlavorResolutionInput::default()
        });

        assert_eq!(profile.flavor_kind, ServerFlavorKind::ForgeLike);
        assert_eq!(profile.edition, ServerEdition::Java);
        assert_eq!(profile.default_startup_mode, Some(StartupMode::Starter));
        assert!(profile.supports_starter_install);
        assert_eq!(
            profile.default_extension_kind,
            Some(ServerExtensionKind::Mod)
        );
    }

    #[test]
    fn resolves_mixed_extension_profile() {
        let profile = resolve_server_flavor_profile(&FlavorResolutionInput {
            core_key: Some("arclight"),
            ..FlavorResolutionInput::default()
        });

        assert!(profile.allow_manual_extension_switch);
        assert_eq!(profile.server_role, ServerRole::GameServer);
        assert!(profile
            .extension_kinds
            .contains(&ServerExtensionKind::Plugin));
        assert!(profile.extension_kinds.contains(&ServerExtensionKind::Mod));
    }

    #[test]
    fn resolves_pumpkin_as_native_executable() {
        let profile = resolve_server_flavor_profile(&FlavorResolutionInput {
            core_key: Some("pumpkin"),
            startup_mode: Some("custom"),
            has_pumpkin_config: true,
            ..FlavorResolutionInput::default()
        });

        assert_eq!(profile.flavor_kind, ServerFlavorKind::NativeExecutable);
        assert_eq!(profile.edition, ServerEdition::Java);
        assert_eq!(profile.default_startup_mode, Some(StartupMode::Custom));
        assert!(!profile.requires_java);
        assert_eq!(
            profile.special_config_kinds,
            vec![SpecialConfigKind::PumpkinToml]
        );
    }

    #[test]
    fn resolves_cuberite_as_native_executable() {
        let profile = resolve_server_flavor_profile(&FlavorResolutionInput {
            core_key: Some("cuberite"),
            startup_mode: Some("custom"),
            ..FlavorResolutionInput::default()
        });

        assert_eq!(profile.flavor_kind, ServerFlavorKind::NativeExecutable);
        assert_eq!(profile.server_role, ServerRole::GameServer);
        assert_eq!(profile.display_key, "cuberite");
        assert!(!profile.requires_java);
        assert!(profile.special_config_kinds.is_empty());
    }

    #[test]
    fn resolves_additional_proxy_forks() {
        for core_key in ["bungeecord", "waterfall", "travertine", "flamecord"] {
            let profile = resolve_server_flavor_profile(&FlavorResolutionInput {
                core_key: Some(core_key),
                ..FlavorResolutionInput::default()
            });

            assert_eq!(
                profile.flavor_kind,
                ServerFlavorKind::ProxyLike,
                "{core_key}"
            );
            assert_eq!(
                profile.default_extension_kind,
                Some(ServerExtensionKind::Plugin),
                "{core_key}"
            );
            assert!(profile.is_proxy(), "{core_key}");
        }
    }

    #[test]
    fn resolves_bedrock_dedicated_profile() {
        let profile = resolve_server_flavor_profile(&FlavorResolutionInput {
            core_key: Some("bds"),
            ..FlavorResolutionInput::default()
        });

        assert_eq!(profile.flavor_kind, ServerFlavorKind::BedrockLike);
        assert_eq!(profile.edition, ServerEdition::Bedrock);
        assert_eq!(profile.server_role, ServerRole::GameServer);
        assert_eq!(profile.display_key, "bedrock_dedicated");
        assert_eq!(profile.default_startup_mode, Some(StartupMode::Exe));
        assert_eq!(
            profile.default_extension_kind,
            Some(ServerExtensionKind::Addon)
        );
        assert!(!profile.requires_java);
    }

    #[test]
    fn resolves_bedrock_wrappers() {
        for core_key in ["liteloaderbds", "levilamina", "bdsx"] {
            let profile = resolve_server_flavor_profile(&FlavorResolutionInput {
                core_key: Some(core_key),
                ..FlavorResolutionInput::default()
            });

            assert_eq!(
                profile.flavor_kind,
                ServerFlavorKind::BedrockLike,
                "{core_key}"
            );
            assert_eq!(profile.display_key, "bedrock_wrapped", "{core_key}");
            assert_eq!(
                profile.default_extension_kind,
                Some(ServerExtensionKind::Plugin),
                "{core_key}"
            );
            assert_eq!(profile.edition, ServerEdition::Bedrock, "{core_key}");
            assert!(profile.allow_manual_extension_switch, "{core_key}");
            assert!(
                profile
                    .extension_kinds
                    .contains(&ServerExtensionKind::Addon),
                "{core_key}"
            );
        }
    }

    #[test]
    fn resolves_bedrock_java_plugin_servers() {
        for core_key in ["allay", "nukkit", "powernukkitx"] {
            let profile = resolve_server_flavor_profile(&FlavorResolutionInput {
                core_key: Some(core_key),
                ..FlavorResolutionInput::default()
            });

            assert_eq!(
                profile.flavor_kind,
                ServerFlavorKind::BedrockLike,
                "{core_key}"
            );
            assert_eq!(profile.display_key, "bedrock_plugin_java", "{core_key}");
            assert_eq!(
                profile.default_startup_mode,
                Some(StartupMode::Jar),
                "{core_key}"
            );
            assert_eq!(
                profile.default_extension_kind,
                Some(ServerExtensionKind::Plugin),
                "{core_key}"
            );
            assert_eq!(profile.server_role, ServerRole::GameServer, "{core_key}");
            assert!(profile.requires_java, "{core_key}");
        }
    }

    #[test]
    fn resolves_other_bedrock_plugin_servers() {
        let pocketmine = resolve_server_flavor_profile(&FlavorResolutionInput {
            core_key: Some("pocketmine"),
            ..FlavorResolutionInput::default()
        });
        assert_eq!(pocketmine.flavor_kind, ServerFlavorKind::BedrockLike);
        assert_eq!(pocketmine.edition, ServerEdition::Bedrock);
        assert_eq!(pocketmine.display_key, "bedrock_plugin_script");
        assert_eq!(pocketmine.default_startup_mode, Some(StartupMode::Custom));
        assert!(!pocketmine.requires_java);

        let endstone = resolve_server_flavor_profile(&FlavorResolutionInput {
            core_key: Some("endstone"),
            ..FlavorResolutionInput::default()
        });
        assert_eq!(endstone.flavor_kind, ServerFlavorKind::BedrockLike);
        assert_eq!(endstone.edition, ServerEdition::Bedrock);
        assert_eq!(endstone.display_key, "bedrock_plugin_native");
        assert_eq!(endstone.default_startup_mode, Some(StartupMode::Exe));
        assert!(!endstone.requires_java);
    }

    #[test]
    fn resolves_mcdr_wrapper_profile() {
        let profile = resolve_server_flavor_profile(&FlavorResolutionInput {
            core_key: Some("paper"),
            startup_mode: Some("custom"),
            wrapper_kind: Some(WrapperKind::Mcdr),
            ..FlavorResolutionInput::default()
        });

        assert_eq!(profile.flavor_kind, ServerFlavorKind::WrappedServer);
        assert_eq!(profile.server_role, ServerRole::Wrapper);
        assert!(profile.is_wrapper());
        assert_eq!(
            profile.default_extension_kind,
            Some(ServerExtensionKind::McdrPlugin)
        );
        assert_eq!(
            profile.preferred_control_channel,
            ControlChannel::WrapperConsole
        );
    }

    #[test]
    fn falls_back_to_unknown_profile_with_manual_extension_switch() {
        let profile = resolve_server_flavor_profile(&FlavorResolutionInput {
            startup_mode: Some("custom"),
            runtime_kind: Some("local"),
            ..FlavorResolutionInput::default()
        });

        assert_eq!(profile.flavor_kind, ServerFlavorKind::WrappedServer);
        assert_eq!(profile.edition, ServerEdition::Unknown);
        assert_eq!(profile.server_role, ServerRole::Wrapper);
        assert!(profile.allow_manual_extension_switch);
        assert_eq!(
            profile.preferred_control_channel,
            ControlChannel::WrapperConsole
        );
    }

    #[test]
    fn docker_runtime_prefers_rcon_for_unknown_profiles() {
        let profile = resolve_server_flavor_profile(&FlavorResolutionInput {
            runtime_kind: Some("docker_itzg"),
            ..FlavorResolutionInput::default()
        });

        assert_eq!(profile.preferred_control_channel, ControlChannel::Rcon);
    }

    #[test]
    fn runtime_family_classifies_known_runtime_kinds() {
        assert_eq!(runtime_family(Some("local")), RuntimeFamily::Local);
        assert_eq!(
            runtime_family(Some("docker_itzg")),
            RuntimeFamily::DockerItzg
        );
        assert_eq!(runtime_family(Some("mcdr_local")), RuntimeFamily::Other);
        assert_eq!(runtime_family(None), RuntimeFamily::Local);
    }

    #[test]
    fn builder_style_input_constructs_expected_profile() {
        let profile = resolve_server_flavor_profile(
            &FlavorResolutionInput::new()
                .with_core_key("paper")
                .with_runtime_kind("local")
                .with_startup_mode("jar"),
        );

        assert_eq!(profile.flavor_kind, ServerFlavorKind::BukkitLike);
        assert_eq!(profile.edition, ServerEdition::Java);
        assert!(profile.supports_extension_kind(ServerExtensionKind::Plugin));
        assert!(!profile.prefers_custom_startup());
    }

    #[test]
    fn bukkit_like_profile_exposes_server_and_plugin_config_surfaces() {
        let profile = resolve_server_flavor_profile(&FlavorResolutionInput {
            core_key: Some("paper"),
            ..FlavorResolutionInput::default()
        });

        let server_properties = profile
            .config_surface("server_properties")
            .expect("server.properties surface should exist");
        assert_eq!(server_properties.owner, ConfigSurfaceOwner::ServerCore);
        assert_eq!(server_properties.kind, ConfigSurfaceKind::CanonicalFile);
        assert_eq!(server_properties.relative_path, "server.properties");
        assert_eq!(server_properties.format, Some(ConfigFormat::Properties));

        let plugins_root = profile
            .config_surface("plugins_root")
            .expect("plugins root should exist");
        assert_eq!(plugins_root.owner, ConfigSurfaceOwner::Plugin);
        assert_eq!(plugins_root.kind, ConfigSurfaceKind::PluginDirectory);
        assert!(plugins_root.recursive);
    }

    #[test]
    fn paper_like_profile_exposes_paper_specific_config_files() {
        let profile = resolve_server_flavor_profile(&FlavorResolutionInput {
            core_key: Some("paper"),
            ..FlavorResolutionInput::default()
        });

        assert!(profile.config_surface("paper_yml").is_some());
        assert!(profile.config_surface("paper_yaml").is_some());
        assert!(profile.config_surface("paper_global_yml").is_some());
        assert!(profile.config_surface("paper_world_defaults_yml").is_some());
    }

    #[test]
    fn spigot_like_profile_exposes_bukkit_and_spigot_configs() {
        let profile = resolve_server_flavor_profile(&FlavorResolutionInput {
            core_key: Some("spigot"),
            ..FlavorResolutionInput::default()
        });

        assert!(profile.config_surface("bukkit_yml").is_some());
        assert!(profile.config_surface("spigot_yml").is_some());
        assert!(profile.config_surface("paper_yml").is_none());
    }

    #[test]
    fn fallback_profile_keeps_generic_config_roots() {
        let profile = resolve_server_flavor_profile(&FlavorResolutionInput {
            core_key: Some("mystery"),
            ..FlavorResolutionInput::default()
        });

        for key in ["plugins_root", "defaultconfig", "defaultconfigs", "config", "configs"] {
            assert!(profile.config_surface(key).is_some(), "missing surface {key}");
        }
    }
}
