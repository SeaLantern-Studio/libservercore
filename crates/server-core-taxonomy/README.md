# sl-server-core-taxonomy

`sl-server-core-taxonomy` provides normalized Minecraft server core keys and coarse taxonomy helpers.

It is intended for host applications and higher-level crates that need a stable normalization layer before applying runtime, flavor, or UI policy.

## Example

```rust
use server_core_taxonomy::{normalize_core_key, CoreFamily};

assert_eq!(normalize_core_key("Paper"), Some("paper"));
assert_eq!(normalize_core_key("Folia"), Some("folia"));
assert_eq!(normalize_core_key("Arclight-Neoforge"), Some("arclight_neoforge"));
assert_eq!(CoreFamily::from_core_key("vanilla"), CoreFamily::VanillaLike);
assert_eq!(CoreFamily::from_core_key("fabric"), CoreFamily::FabricLike);
```
