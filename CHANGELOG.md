# Changelog

## Unreleased

- Fix `sponge` and `minestom` to resolve to stable embedded-Java taxonomy and flavor profiles instead of falling back to unknown semantics.
- Canonicalize startup-scan API output to SeaLantern-style Arclight keys (`arclight-forge`, `arclight-neoforge`) instead of leaking internal underscore keys.
- Add explicit Windows `.bat` codepage support to `sl-server-flow` script launch specs so host applications can preserve console encoding behavior.

- Bootstrap workspace for publishable Minecraft server modeling crates.
- Add `sl-server-core-taxonomy` for normalized core keys and coarse family classification.
- Add `sl-server-flavor-core` for derived flavor capability profiles.
- Extend taxonomy coverage for additional forks and standalone servers including Folia, Pufferfish, Tuinity, Airplane, Glowstone, Travertine, FlameCord, Cuberite, Minestom, and Sponge.
- Model `vanilla` explicitly as `VanillaLike` instead of folding it into the Bukkit family.
- Keep proxy canonical keys distinct, including `bungeecord`, `waterfall`, and `lightfall`, instead of normalizing them all to one fork name.
- Add first-class Bedrock modeling for `bds`, `liteloaderbds`, `levilamina`, `bdsx`, `allay`, `nukkit`, `powernukkitx`, `pocketmine`, and `endstone`.
- Add stable public flavor metadata for `edition` and `server_role` so host applications do not need to infer semantics from `display_key`.
