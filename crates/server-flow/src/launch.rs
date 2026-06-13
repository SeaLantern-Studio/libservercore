use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use crate::error::LaunchError;
use crate::spec::{
    ArgsFileSpec, JavaCommandSpec, JavaEnvSpec, LocalLaunchEntry, LocalLaunchSpec,
    ManagedJavaMode, ScriptLaunchSpec, StarterInstallSpec,
};
use server_flavor_core::StartupMode;

pub struct LaunchedLocalProcess {
    pub pid: u32,
    pub child: Child,
}

pub fn build_launch_command(spec: &LocalLaunchSpec) -> Result<Command, LaunchError> {
    if !spec.working_dir.is_dir() {
        return Err(LaunchError::InvalidWorkingDirectory(spec.working_dir.clone()));
    }

    match &spec.entry {
        LocalLaunchEntry::DirectJar { jar_path, java } => {
            build_direct_jar_command(&spec.working_dir, jar_path, java, None)
        }
        LocalLaunchEntry::StarterInstall(starter) => {
            build_starter_install_command(&spec.working_dir, starter)
        }
        LocalLaunchEntry::Script(script) => build_script_command(&spec.working_dir, script),
        LocalLaunchEntry::Executable { executable_path, args } => {
            let mut command = Command::new(executable_path);
            command.args(args);
            Ok(command)
        }
        LocalLaunchEntry::CustomCommand {
            command,
            startup_mode: _,
            java_env,
        } => {
            if command.trim().is_empty() {
                return Err(LaunchError::MissingCustomCommand);
            }
            Ok(build_custom_command(command, java_env.as_ref()))
        }
    }
}

pub fn launch_local_process(spec: &LocalLaunchSpec) -> Result<LaunchedLocalProcess, LaunchError> {
    let mut command = build_launch_command(spec)?;
    command.current_dir(&spec.working_dir);
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    let child = command
        .spawn()
        .map_err(|e| LaunchError::SpawnFailed(e.to_string()))?;
    let pid = child.id();

    Ok(LaunchedLocalProcess { pid, child })
}

fn build_direct_jar_command(
    working_dir: &Path,
    jar_path: &Path,
    java: &JavaCommandSpec,
    installer_args: Option<&[String]>,
) -> Result<Command, LaunchError> {
    let mut command = Command::new(&java.java_path);
    command.current_dir(working_dir);
    for arg in &java.jvm_args {
        command.arg(arg);
    }
    command.arg("-jar");
    command.arg(relative_or_owned(working_dir, jar_path));
    if java.add_nogui {
        command.arg("nogui");
    }
    if let Some(installer_args) = installer_args {
        for arg in installer_args {
            command.arg(arg);
        }
    }
    Ok(command)
}

fn build_starter_install_command(
    working_dir: &Path,
    starter: &StarterInstallSpec,
) -> Result<Command, LaunchError> {
    build_direct_jar_command(
        working_dir,
        &starter.installer_jar,
        &starter.java,
        Some(&starter.install_args),
    )
}

fn build_script_command(
    working_dir: &Path,
    script: &ScriptLaunchSpec,
) -> Result<Command, LaunchError> {
    maybe_write_args_file(working_dir, script.args_file.as_ref())?;

    match script.startup_mode {
        StartupMode::Bat => {
            build_bat_command(
                working_dir,
                &script.script_path,
                script.java_env.as_ref(),
                script.windows_codepage.as_deref(),
                &script.trailing_args,
            )
        }
        StartupMode::Sh => {
            let mut command = Command::new("sh");
            command.arg(relative_or_owned(working_dir, &script.script_path));
            command.args(&script.trailing_args);
            apply_java_process_env(&mut command, script.java_env.as_ref());
            Ok(command)
        }
        StartupMode::Ps1 => {
            let mut command = Command::new("powershell");
            command.args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ]);
            command.arg(relative_or_owned(working_dir, &script.script_path));
            command.args(&script.trailing_args);
            apply_java_process_env(&mut command, script.java_env.as_ref());
            Ok(command)
        }
        StartupMode::Exe | StartupMode::Jar | StartupMode::Starter | StartupMode::Custom => {
            Err(LaunchError::UnsupportedStartupMode(Some(
                script.startup_mode.as_str().to_string(),
            )))
        }
    }
}

#[cfg(target_os = "windows")]
fn build_bat_command(
    working_dir: &Path,
    script_path: &Path,
    java_env: Option<&JavaEnvSpec>,
    windows_codepage: Option<&str>,
    trailing_args: &[String],
) -> Result<Command, LaunchError> {
    let script_text = relative_or_owned(working_dir, script_path)
        .to_string_lossy()
        .to_string();
    let prefix = java_env
        .map(|env| build_windows_java_env_prefix(&env.java_home, &env.java_bin_dir))
        .unwrap_or_default();
    let tail = if trailing_args.is_empty() {
        String::new()
    } else {
        format!(" {}", trailing_args.join(" "))
    };
    let call_text = if prefix.is_empty() {
        format!("call \"{}\"{}", escape_for_cmd(&script_text), tail)
    } else {
        format!(
            "{} & call \"{}\"{}",
            prefix,
            escape_for_cmd(&script_text),
            tail
        )
    };
    let cmd_text = if let Some(codepage) = windows_codepage.filter(|value| !value.trim().is_empty()) {
        format!("chcp {}>nul & {}", codepage.trim(), call_text)
    } else {
        call_text
    };

    let mut command = Command::new("cmd");
    command.args(["/d", "/c", &cmd_text]);
    Ok(command)
}

#[cfg(not(target_os = "windows"))]
fn build_bat_command(
    _working_dir: &Path,
    _script_path: &Path,
    _java_env: Option<&JavaEnvSpec>,
    _windows_codepage: Option<&str>,
    _trailing_args: &[String],
) -> Result<Command, LaunchError> {
    Err(LaunchError::UnsupportedStartupMode(Some("bat".to_string())))
}

fn build_custom_command(command: &str, java_env: Option<&JavaEnvSpec>) -> Command {
    #[cfg(target_os = "windows")]
    {
        let mut process = Command::new("cmd");
        process.args(["/d", "/c", command]);
        apply_java_process_env(&mut process, java_env);
        process
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut process = Command::new("sh");
        process.args(["-c", command]);
        apply_java_process_env(&mut process, java_env);
        process
    }
}

fn maybe_write_args_file(
    working_dir: &Path,
    args_file: Option<&ArgsFileSpec>,
) -> Result<(), LaunchError> {
    let Some(args_file) = args_file else {
        return Ok(());
    };
    if !matches!(args_file.mode, ManagedJavaMode::ArgsFileOnly) {
        return Ok(());
    }
    let path = if args_file.path.is_absolute() {
        args_file.path.clone()
    } else {
        working_dir.join(&args_file.path)
    };
    let parent = path
        .parent()
        .ok_or_else(|| LaunchError::MissingArgsFileParent(path.clone()))?;
    std::fs::create_dir_all(parent).map_err(|e| LaunchError::Io(e.to_string()))?;
    let content = if args_file.args.is_empty() {
        String::new()
    } else {
        format!("{}\n", args_file.args.join("\n"))
    };
    std::fs::write(path, content).map_err(|e| LaunchError::Io(e.to_string()))
}

fn apply_java_process_env(command: &mut Command, java_env: Option<&JavaEnvSpec>) {
    let Some(java_env) = java_env else {
        return;
    };

    command.env("JAVA_HOME", &java_env.java_home);
    command.env(
        "PATH",
        prepend_path_entry(
            &java_env.java_bin_dir.to_string_lossy(),
            &std::env::var("PATH").unwrap_or_default(),
            path_separator(),
        ),
    );
}

fn relative_or_owned(base: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(base)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(target_os = "windows")]
fn path_separator() -> &'static str {
    ";"
}

#[cfg(not(target_os = "windows"))]
fn path_separator() -> &'static str {
    ":"
}

fn prepend_path_entry(path_entry: &str, existing_path: &str, separator: &str) -> String {
    if existing_path.is_empty() {
        path_entry.to_string()
    } else {
        format!("{}{}{}", path_entry, separator, existing_path)
    }
}

#[cfg(target_os = "windows")]
fn build_windows_java_env_prefix(java_home: &Path, java_bin_dir: &Path) -> String {
    format!(
        "set \"JAVA_HOME={}\" & set \"PATH={};%PATH%\"",
        java_home.display(),
        java_bin_dir.display()
    )
}

#[cfg(target_os = "windows")]
fn escape_for_cmd(value: &str) -> String {
    value
        .replace('^', "^^")
        .replace('&', "^&")
        .replace('(', "^(")
        .replace(')', "^)")
        .replace('%', "%%")
}

#[cfg(test)]
mod tests {
    use super::{build_launch_command, launch_local_process};
    use crate::spec::{
        ArgsFileSpec, JavaCommandSpec, JavaEnvSpec, LocalLaunchEntry, LocalLaunchSpec,
        ManagedJavaMode, ScriptLaunchSpec,
    };
    use server_flavor_core::StartupMode;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(prefix: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("sl-server-flow-{}-{}", prefix, unique));
            std::fs::create_dir_all(&path).expect("test dir should be created");
            Self { path }
        }

        fn path(&self) -> &std::path::Path {
            &self.path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn probe_java_command() -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            PathBuf::from(std::env::var("ComSpec").unwrap_or_else(|_| "cmd".to_string()))
        }

        #[cfg(not(target_os = "windows"))]
        {
            PathBuf::from("sh")
        }
    }

    #[test]
    fn direct_jar_command_includes_jvm_args_and_nogui() {
        let dir = TestDir::new("jar");
        let spec = LocalLaunchSpec::new(
            "jar-test",
            dir.path().to_path_buf(),
            LocalLaunchEntry::DirectJar {
                jar_path: PathBuf::from("server.jar"),
                java: JavaCommandSpec {
                    java_path: probe_java_command(),
                    java_env: None,
                    jvm_args: vec!["-Xmx4096M".to_string(), "-Xms2048M".to_string()],
                    add_nogui: true,
                },
            },
        );

        let command = build_launch_command(&spec).expect("command should build");
        let args = command
            .get_args()
            .map(|value| value.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert_eq!(
            args,
            vec!["-Xmx4096M", "-Xms2048M", "-jar", "server.jar", "nogui"]
        );
    }

    #[test]
    fn sh_script_command_writes_args_file_and_injects_java_env() {
        let dir = TestDir::new("script");
        let script_path = dir.path().join("start.sh");
        std::fs::write(&script_path, "#!/bin/sh\nexit 0\n").expect("script should write");

        let spec = LocalLaunchSpec::new(
            "script-test",
            dir.path().to_path_buf(),
            LocalLaunchEntry::Script(ScriptLaunchSpec {
                startup_mode: StartupMode::Sh,
                script_path: script_path.clone(),
                java_env: Some(JavaEnvSpec::new(
                    PathBuf::from("/opt/jdk"),
                    PathBuf::from("/opt/jdk/bin"),
                )),
                windows_codepage: None,
                args_file: Some(ArgsFileSpec {
                    path: PathBuf::from("user_jvm_args.txt"),
                    mode: ManagedJavaMode::ArgsFileOnly,
                    args: vec!["-Xmx4096M".to_string(), "-Xms2048M".to_string()],
                }),
                trailing_args: vec!["nogui".to_string()],
            }),
        );

        let command = build_launch_command(&spec).expect("command should build");
        let envs = command
            .get_envs()
            .map(|(key, value)| {
                (
                    key.to_string_lossy().to_string(),
                    value.map(|value| value.to_string_lossy().to_string()),
                )
            })
            .collect::<Vec<_>>();

        assert!(dir.path().join("user_jvm_args.txt").exists());
        assert!(envs.iter().any(|(key, value)| {
            key == "JAVA_HOME" && value.as_deref() == Some("/opt/jdk")
        }));
        assert!(envs.iter().any(|(key, value)| {
            key == "PATH"
                && value
                    .as_deref()
                    .is_some_and(|value| value.starts_with("/opt/jdk/bin"))
        }));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn bat_command_keeps_java_env_inline_in_cmd_text() {
        let dir = TestDir::new("bat");
        let script_path = dir.path().join("launch.bat");
        std::fs::write(&script_path, "@echo off\r\nexit /b 0\r\n").expect("script should write");

        let spec = LocalLaunchSpec::new(
            "bat-test",
            dir.path().to_path_buf(),
            LocalLaunchEntry::Script(ScriptLaunchSpec {
                startup_mode: StartupMode::Bat,
                script_path,
                java_env: Some(JavaEnvSpec::new(
                    PathBuf::from("C:/Java/JDK 21"),
                    PathBuf::from("C:/Java/JDK 21/bin"),
                )),
                windows_codepage: Some("65001".to_string()),
                args_file: None,
                trailing_args: vec!["nogui".to_string()],
            }),
        );

        let command = build_launch_command(&spec).expect("command should build");
        let args = command
            .get_args()
            .map(|value| value.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert_eq!(args[0], "/d");
        assert_eq!(args[1], "/c");
        assert!(args[2].contains("chcp 65001>nul"));
        assert!(args[2].contains("JAVA_HOME=C:/Java/JDK 21"));
        assert!(args[2].contains("PATH=C:/Java/JDK 21/bin;%PATH%"));
        assert!(args[2].contains("call \"launch.bat\" nogui"));
    }

    #[test]
    fn launch_local_process_spawns_short_lived_custom_command() {
        let dir = TestDir::new("custom");

        let spec = LocalLaunchSpec::new(
            "custom-test",
            dir.path().to_path_buf(),
            LocalLaunchEntry::CustomCommand {
                command: "exit 0".to_string(),
                startup_mode: StartupMode::Custom,
                java_env: None,
            },
        );

        let mut launched = launch_local_process(&spec).expect("process should launch");
        let status = launched.child.wait().expect("process should finish");

        assert!(status.success());
    }
}
