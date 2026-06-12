# sl-libscv

`sl-libscv` provides file-centered configuration discovery and IO for Minecraft server installations.

It is designed to work from normalized core keys plus the richer config-surface metadata exported by `sl-server-flavor-core`.

Current v1 direction:

- discover server-core config files
- discover generic plugin config files under `plugins/`
- discover fallback config directories such as `config/` and `defaultconfigs/`
- discover generic config candidates for non-Bukkit ecosystems under controlled scan roots such as `mods/` and `world/serverconfig/`
- provide typed file documents for `yaml`, `toml`, `json`, `properties`, and plain text
- support high-fidelity editing, especially for `properties`

## Example

```rust
use sl_libscv::{
    discover_config_entries, read_config_file, write_config_file, ConfigDiscoveryInput,
    ConfigDocument,
};

let input = ConfigDiscoveryInput::new("paper", std::path::PathBuf::from("E:/servers/paper"));
let catalog = discover_config_entries(&input).unwrap();

assert!(catalog.entries.iter().any(|entry| entry.relative_path == "server.properties"));
assert!(catalog.entries.iter().any(|entry| entry.relative_path == "plugins/Essentials/config.yml"));

if let Some(entry) = catalog
    .entries
    .iter()
    .find(|entry| entry.relative_path == "server.properties")
{
    let mut document = read_config_file(&entry.absolute_path).unwrap();
    if let ConfigDocument::Properties(properties) = &mut document {
        properties.set("motd", "Hello from sl-libscv");
    }
    write_config_file(&entry.absolute_path, &document).unwrap();
}
```

## Generic Candidate Discovery

For server types that do not yet have explicit surface mappings, `sl-libscv` can also provide heuristic config candidates without claiming product-level compatibility.

```rust
use sl_libscv::{discover_config_candidates, GenericConfigDiscoveryInput};

let candidates = discover_config_candidates(&GenericConfigDiscoveryInput::new(
    std::path::PathBuf::from("E:/servers/neoforge"),
))
.unwrap();

for entry in candidates.entries {
    println!("{} {:?} {:?}", entry.relative_path, entry.source, entry.reason);
}
```
