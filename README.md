# libservercore

Core Rust crates for Minecraft server taxonomy, flavor modeling, and related shared logic.

This repository is intended to host publishable crates that can be consumed by application projects through Cargo.

## Crates

- `sl-server-core-taxonomy`: normalize Minecraft server core keys and provide coarse taxonomy helpers.
- `sl-server-flavor-core`: resolve high-level server flavor and capability profiles from core type, runtime kind, startup mode, and wrapper hints.

The current taxonomy intentionally covers vanilla, Bukkit-family forks such as Paper/Folia/Pufferfish, proxy forks such as BungeeCord/Waterfall/Velocity/Travertine/FlameCord, Forge/Fabric ecosystems, mixed plugin+mod hybrids, native executables such as Pumpkin/Cuberite, and Bedrock server lines such as Bedrock Dedicated Server, LiteLoaderBDS, LeviLamina, BDSX, Allay, and Nukkit.

## Status

This repository is in early bootstrap. The first crate focuses on flavor modeling so host applications can stop scattering `if core_type == ...` logic across provisioning, startup, and extension-management flows.

## Publishing

These crates are intended to be published to crates.io and then consumed by application repositories through normal Cargo dependencies.

Current publish order:

1. `sl-server-core-taxonomy`
2. `sl-server-flavor-core`
