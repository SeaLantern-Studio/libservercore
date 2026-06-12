# sl-server-startup-scan-core

`sl-server-startup-scan-core` scans local server folders and archives to produce startup candidates, canonical core-key hints, and recommended startup entrypoints.

Current v1 direction:

- scan direct server folders for jar, script, and native executable startup candidates
- scan archive sources such as `.zip`, `.tar`, `.tar.gz`, and `.tgz`
- infer canonical core keys from filenames and selected manifest main classes
- parse common script launch targets such as `java -jar ...`, `call start.bat`, and `./run.sh`
- rank startup candidates for upper-layer selection UIs and provisioning flows
- resolve mode-aware launch targets for `jar`, `starter`, `custom`, and script-based startup modes

## Example

```rust
use server_startup_scan_core::{scan_startup_candidates, StartupSourceKind};

let result = scan_startup_candidates(
    "E:/servers/paper",
    StartupSourceKind::Folder,
    &[],
)
.unwrap();

assert!(result
    .candidates
    .iter()
    .any(|entry| entry.mode == "jar" || entry.mode == "starter"));

let resolved = server_startup_scan_core::resolve_mode_aware_launch_target(
    "sh",
    Some("E:/servers/paper/start.sh"),
    None,
    std::path::Path::new("E:/servers/paper"),
)
.unwrap();

assert_eq!(resolved.launch_target, "start.sh");
```
