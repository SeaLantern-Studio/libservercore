use std::path::PathBuf;
use std::time::Duration;

use server_flavor_core::StartupMode;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManagedJavaMode {
    DirectJvmArgs,
    ArgsFileOnly,
    Disabled,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaEnvSpec {
    pub java_home: PathBuf,
    pub java_bin_dir: PathBuf,
}

impl JavaEnvSpec {
    pub fn new(java_home: PathBuf, java_bin_dir: PathBuf) -> Self {
        Self {
            java_home,
            java_bin_dir,
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaCommandSpec {
    pub java_path: PathBuf,
    pub java_env: Option<JavaEnvSpec>,
    pub jvm_args: Vec<String>,
    pub add_nogui: bool,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgsFileSpec {
    pub path: PathBuf,
    pub mode: ManagedJavaMode,
    pub args: Vec<String>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptLaunchSpec {
    pub startup_mode: StartupMode,
    pub script_path: PathBuf,
    pub java_env: Option<JavaEnvSpec>,
    pub windows_codepage: Option<String>,
    pub args_file: Option<ArgsFileSpec>,
    pub trailing_args: Vec<String>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StarterInstallSpec {
    pub installer_jar: PathBuf,
    pub java: JavaCommandSpec,
    pub install_args: Vec<String>,
    pub followup: Box<LocalLaunchEntry>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalLaunchEntry {
    DirectJar {
        jar_path: PathBuf,
        java: JavaCommandSpec,
    },
    StarterInstall(StarterInstallSpec),
    Script(ScriptLaunchSpec),
    Executable {
        executable_path: PathBuf,
        args: Vec<String>,
    },
    CustomCommand {
        command: String,
        startup_mode: StartupMode,
        java_env: Option<JavaEnvSpec>,
    },
}

impl LocalLaunchEntry {
    pub fn startup_mode(&self) -> StartupMode {
        match self {
            Self::DirectJar { .. } => StartupMode::Jar,
            Self::StarterInstall(_) => StartupMode::Starter,
            Self::Script(spec) => spec.startup_mode,
            Self::Executable { .. } => StartupMode::Exe,
            Self::CustomCommand { startup_mode, .. } => *startup_mode,
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortProbeKind {
    TcpListening,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortProbeSpec {
    pub port: u16,
    pub kind: PortProbeKind,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalLaunchSpec {
    pub server_id: String,
    pub working_dir: PathBuf,
    pub entry: LocalLaunchEntry,
    pub port_probe: Option<PortProbeSpec>,
}

impl LocalLaunchSpec {
    pub fn new(
        server_id: impl Into<String>,
        working_dir: PathBuf,
        entry: LocalLaunchEntry,
    ) -> Self {
        Self {
            server_id: server_id.into(),
            working_dir,
            entry,
            port_probe: None,
        }
    }

    pub fn startup_mode(&self) -> StartupMode {
        self.entry.startup_mode()
    }

    pub fn with_port_probe(mut self, port_probe: PortProbeSpec) -> Self {
        self.port_probe = Some(port_probe);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessStopStrategy {
    GracefulThenForce,
    ForceOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GracefulStopSpec {
    pub stdin_line: String,
    pub timeout: Duration,
    pub poll_interval: Duration,
}

impl GracefulStopSpec {
    pub fn stdin_line(stdin_line: impl Into<String>) -> Self {
        Self {
            stdin_line: stdin_line.into(),
            timeout: Duration::from_secs(30),
            poll_interval: Duration::from_millis(250),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_poll_interval(mut self, poll_interval: Duration) -> Self {
        self.poll_interval = poll_interval;
        self
    }
}
