# sl-server-flow

`sl-server-flow` provides local server lifecycle primitives for host applications that need a stable launch, status, and stop flow.

It builds on `sl-server-flavor-core` for startup mode modeling but stays focused on game-external lifecycle concerns.

Current scope:

- local process launch planning and execution
- startup-mode aware command construction
- managed JVM argument insertion for direct JAR launches
- script launch Java environment injection
- process liveness and optional TCP readiness probing
- graceful stop, wait, and process-tree force stop

Out of scope for this crate:

- Docker lifecycle support
- game-internal inspection such as player count, TPS, or detailed RCON status
- product-specific config parsing or state-file protocols

## Example

```rust
use server_flow::{
    GracefulStopSpec, JavaCommandSpec, JavaEnvSpec, LocalLaunchEntry, LocalLaunchSpec,
    ManagedJavaMode, ProcessStopStrategy, ScriptLaunchSpec,
};
use server_flavor_core::StartupMode;

let spec = LocalLaunchSpec::new(
    "paper",
    std::path::PathBuf::from("E:/servers/paper"),
    LocalLaunchEntry::DirectJar {
        jar_path: std::path::PathBuf::from("server.jar"),
        java: JavaCommandSpec {
            java_path: std::path::PathBuf::from("java"),
            java_env: None,
            jvm_args: vec!["-Xmx4096M".into(), "-Xms2048M".into()],
            add_nogui: true,
        },
    },
);

assert_eq!(spec.startup_mode(), StartupMode::Jar);

let script = LocalLaunchEntry::Script(ScriptLaunchSpec {
    startup_mode: StartupMode::Sh,
    script_path: std::path::PathBuf::from("start.sh"),
    java_env: Some(JavaEnvSpec::new(
        std::path::PathBuf::from("/opt/jdk"),
        std::path::PathBuf::from("/opt/jdk/bin"),
    )),
    args_file: None,
    trailing_args: vec!["nogui".into()],
});

assert_eq!(script.startup_mode(), StartupMode::Sh);

let _ = (
    ManagedJavaMode::DirectJvmArgs,
    GracefulStopSpec::stdin_line("stop"),
    ProcessStopStrategy::GracefulThenForce,
);
```
