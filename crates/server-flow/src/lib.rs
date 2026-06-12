//! Local Minecraft server lifecycle flow primitives and execution support.

#![forbid(unsafe_code)]

mod error;
mod launch;
mod process_tree;
mod spec;
mod status;
mod stop;

pub use error::{LaunchError, StatusError, StopError};
pub use launch::{build_launch_command, launch_local_process, LaunchedLocalProcess};
pub use process_tree::{force_kill_process_tree_by_pid, is_process_alive};
pub use spec::{
    ArgsFileSpec, GracefulStopSpec, JavaCommandSpec, JavaEnvSpec, LocalLaunchEntry,
    LocalLaunchSpec, ManagedJavaMode, PortProbeKind, PortProbeSpec, ProcessStopStrategy,
    ScriptLaunchSpec, StarterInstallSpec,
};
pub use status::{probe_local_status, LocalFlowState, LocalFlowStatus, OwnedProcessHandle, PortProbeStatus};
pub use stop::{stop_local_process, stop_process_by_pid, StopOutcome};

pub use server_flavor_core::StartupMode;
