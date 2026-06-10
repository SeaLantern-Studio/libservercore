# sl-server-flavor-core

`sl-server-flavor-core` provides a small, publishable model for resolving Minecraft server flavor capabilities.

It builds on `sl-server-core-taxonomy` for normalized core-key handling.

It is designed for host applications that need to answer questions such as:

- Is this server Bukkit-like, Forge-like, Fabric-like, proxy-like, Bedrock-like, wrapper-driven, or a native executable?
- Is this Java Edition or Bedrock Edition, and is it a game server, proxy, or wrapper?
- Should the default extension surface be plugins, mods, datapacks, or wrapper plugins?
- Is `starter` a valid default startup mode?
- Should control prefer stdin, RCON, Docker stdio, or a wrapper console?

## Example

```rust
use server_flavor_core::{
    resolve_server_flavor_profile, ControlChannel, FlavorResolutionInput, ServerExtensionKind,
    ServerFlavorKind, StartupMode, WrapperKind,
};

let profile = resolve_server_flavor_profile(&FlavorResolutionInput {
    core_key: Some("folia"),
    runtime_kind: Some("local"),
    startup_mode: Some("jar"),
    wrapper_kind: None,
    has_pumpkin_config: false,
});

assert_eq!(profile.flavor_kind, ServerFlavorKind::BukkitLike);
assert_eq!(profile.default_startup_mode, Some(StartupMode::Jar));
assert_eq!(profile.default_extension_kind, Some(ServerExtensionKind::Plugin));
assert_eq!(profile.preferred_control_channel, ControlChannel::Stdin);

let vanilla = resolve_server_flavor_profile(&FlavorResolutionInput {
    core_key: Some("vanilla"),
    runtime_kind: Some("local"),
    startup_mode: Some("jar"),
    wrapper_kind: None,
    has_pumpkin_config: false,
});

assert_eq!(vanilla.flavor_kind, ServerFlavorKind::VanillaLike);
assert_eq!(vanilla.default_extension_kind, Some(ServerExtensionKind::Datapack));

let bedrock = resolve_server_flavor_profile(&FlavorResolutionInput {
    core_key: Some("bds"),
    runtime_kind: Some("local"),
    startup_mode: Some("exe"),
    wrapper_kind: None,
    has_pumpkin_config: false,
});

assert_eq!(bedrock.flavor_kind, ServerFlavorKind::BedrockLike);
assert_eq!(bedrock.default_startup_mode, Some(StartupMode::Exe));
assert_eq!(bedrock.default_extension_kind, Some(ServerExtensionKind::Addon));
assert_eq!(bedrock.edition, server_flavor_core::ServerEdition::Bedrock);

let wrapped = resolve_server_flavor_profile(&FlavorResolutionInput {
    core_key: Some("paper"),
    runtime_kind: Some("local"),
    startup_mode: Some("custom"),
    wrapper_kind: Some(WrapperKind::Mcdr),
    has_pumpkin_config: false,
});

assert_eq!(wrapped.flavor_kind, ServerFlavorKind::WrappedServer);
assert_eq!(wrapped.server_role, server_flavor_core::ServerRole::Wrapper);
```

## Design Notes

This crate intentionally models flavor as a derived capability profile instead of making raw `core_type` strings carry every downstream behavior decision.

The current model covers both Java Edition and Bedrock Edition server families, including proxy forks, native binaries, and Bedrock wrapper ecosystems such as LiteLoaderBDS, LeviLamina, and BDSX.
