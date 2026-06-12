# sl-server-launch-plan-core

`sl-server-launch-plan-core` turns startup-mode inputs, startup-scan results, and Java/runtime hints into executable local launch plans backed by `sl-server-flow`.

Current v1 direction:

- resolve mode-aware launch targets using `sl-server-startup-scan-core`
- map `jar`, `starter`, script, custom-command, and native executable modes into `sl-server-flow::LocalLaunchSpec`
- keep planning logic shared so host applications do not re-implement startup branching

## Example

```rust
use server_launch_plan_core::{plan_local_launch, LocalLaunchPlanInput};
use std::path::PathBuf;

let plan = plan_local_launch(LocalLaunchPlanInput {
    server_id: "paper-demo".to_string(),
    server_root: PathBuf::from("E:/servers/paper"),
    startup_mode: "sh".to_string(),
    configured_startup_path: Some("E:/servers/paper/start.sh".to_string()),
    custom_command: None,
    java_path: None,
    java_home: None,
    java_bin_dir: None,
    jvm_args: Vec::new(),
    add_nogui: true,
    port: Some(25565),
})
.unwrap();

assert_eq!(plan.spec.startup_mode().as_str(), "sh");
```
