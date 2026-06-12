use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchError {
    UnsupportedStartupMode(Option<String>),
    MissingJavaForMode(&'static str),
    MissingScriptPath,
    MissingJarPath,
    MissingExecutablePath,
    MissingCustomCommand,
    MissingStarterFollowup,
    MissingArgsFileParent(PathBuf),
    InvalidWorkingDirectory(PathBuf),
    Io(String),
    SpawnFailed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusError {
    MissingProcessId,
    ProbeFailed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopError {
    MissingProcessId,
    MissingOwnedHandle,
    GracefulStopUnsupported,
    Io(String),
    ForceKillFailed(String),
}
