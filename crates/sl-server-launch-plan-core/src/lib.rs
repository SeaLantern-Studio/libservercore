//! Mode-aware local launch planning built on startup scan and server flow primitives.

#![forbid(unsafe_code)]

use std::path::PathBuf;

use server_flow::{
    JavaCommandSpec, JavaEnvSpec, LocalLaunchEntry, LocalLaunchSpec, PortProbeKind,
    PortProbeSpec, ScriptLaunchSpec, StarterInstallSpec, StartupMode,
};
use server_startup_scan_core::{resolve_mode_aware_launch_target, ResolvedLaunchTarget};

const STARTER_INSTALL_ARGS_STANDARD: &[&str] = &["--install-server", "."];
const STARTER_INSTALL_ARGS_FORGE_LIKE: &[&str] = &["--installServer", "."];
const STARTER_INSTALL_ARGS_NEOFORGE_LIKE: &[&str] = &["--install-server", ".", "--server-starter"];

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalLaunchPlanInput {
    pub server_id: String,
    pub server_root: PathBuf,
    pub startup_mode: String,
    pub configured_startup_path: Option<String>,
    pub custom_command: Option<String>,
    pub java_path: Option<PathBuf>,
    pub java_home: Option<PathBuf>,
    pub java_bin_dir: Option<PathBuf>,
    pub jvm_args: Vec<String>,
    pub add_nogui: bool,
    pub port: Option<u16>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalLaunchPlan {
    pub resolution: ResolvedLaunchTarget,
    pub spec: LocalLaunchSpec,
}

pub fn plan_local_launch(input: LocalLaunchPlanInput) -> Result<LocalLaunchPlan, String> {
    if !input.server_root.is_dir() {
        return Err(format!(
            "Server root is not a directory: {}",
            input.server_root.display()
        ));
    }

    let resolution = resolve_mode_aware_launch_target(
        &input.startup_mode,
        input.configured_startup_path.as_deref(),
        input.custom_command.as_deref(),
        &input.server_root,
    )?;
    let startup_mode = parse_startup_mode(&resolution.startup_mode)?;
    let java_env = build_java_env(input.java_home.clone(), input.java_bin_dir.clone());
    let java_spec = build_java_command_spec(
        input.java_path.clone(),
        java_env.clone(),
        input.jvm_args.clone(),
        input.add_nogui,
    );

    let entry = match startup_mode {
        StartupMode::Jar => {
            let jar_path = resolution
                .preferred_jar_path
                .clone()
                .or_else(|| input.configured_startup_path.clone())
                .ok_or_else(|| "Missing jar path for jar startup mode".to_string())?;
            let java = java_spec.ok_or_else(|| "Java path is required for jar startup mode".to_string())?;
            LocalLaunchEntry::DirectJar {
                jar_path: PathBuf::from(jar_path),
                java,
            }
        }
        StartupMode::Starter => {
            let installer_jar = resolution
                .preferred_jar_path
                .clone()
                .or_else(|| input.configured_startup_path.clone())
                .ok_or_else(|| "Missing installer jar path for starter startup mode".to_string())?;
            let java = java_spec
                .clone()
                .ok_or_else(|| "Java path is required for starter startup mode".to_string())?;
            let installer_path = PathBuf::from(installer_jar);
            let starter_core_key = infer_starter_core_key(&resolution, input.configured_startup_path.as_deref());
            LocalLaunchEntry::StarterInstall(StarterInstallSpec {
                installer_jar: installer_path.clone(),
                java: java.clone(),
                install_args: starter_install_args(starter_core_key.as_deref()),
                followup: Box::new(LocalLaunchEntry::DirectJar {
                    jar_path: installer_path,
                    java,
                }),
            })
        }
        StartupMode::Bat | StartupMode::Sh | StartupMode::Ps1 => {
            let script_path = input
                .configured_startup_path
                .clone()
                .ok_or_else(|| "Missing script path for script startup mode".to_string())?;
            LocalLaunchEntry::Script(ScriptLaunchSpec {
                startup_mode,
                script_path: PathBuf::from(script_path),
                java_env,
                args_file: None,
                trailing_args: if input.add_nogui {
                    vec!["nogui".to_string()]
                } else {
                    Vec::new()
                },
            })
        }
        StartupMode::Custom => LocalLaunchEntry::CustomCommand {
            command: input
                .custom_command
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| "Missing custom command for custom startup mode".to_string())?,
            startup_mode,
            java_env,
        },
        StartupMode::Exe => LocalLaunchEntry::Executable {
            executable_path: PathBuf::from(
                input
                    .configured_startup_path
                    .clone()
                    .ok_or_else(|| "Missing executable path for exe startup mode".to_string())?,
            ),
            args: if input.add_nogui {
                vec!["nogui".to_string()]
            } else {
                Vec::new()
            },
        },
    };

    let mut spec = LocalLaunchSpec::new(input.server_id, input.server_root, entry);
    if let Some(port) = input.port {
        spec = spec.with_port_probe(PortProbeSpec {
            port,
            kind: PortProbeKind::TcpListening,
        });
    }

    Ok(LocalLaunchPlan { resolution, spec })
}

fn parse_startup_mode(value: &str) -> Result<StartupMode, String> {
    StartupMode::parse(value).ok_or_else(|| format!("Unsupported startup mode: {value}"))
}

fn build_java_env(
    java_home: Option<PathBuf>,
    java_bin_dir: Option<PathBuf>,
) -> Option<JavaEnvSpec> {
    match (java_home, java_bin_dir) {
        (Some(java_home), Some(java_bin_dir)) => Some(JavaEnvSpec::new(java_home, java_bin_dir)),
        _ => None,
    }
}

fn build_java_command_spec(
    java_path: Option<PathBuf>,
    java_env: Option<JavaEnvSpec>,
    jvm_args: Vec<String>,
    add_nogui: bool,
) -> Option<JavaCommandSpec> {
    java_path.map(|java_path| JavaCommandSpec {
        java_path,
        java_env,
        jvm_args,
        add_nogui,
    })
}

fn infer_starter_core_key(
    resolution: &ResolvedLaunchTarget,
    configured_startup_path: Option<&str>,
) -> Option<String> {
    resolution
        .preferred_jar_path
        .as_deref()
        .or(configured_startup_path)
        .map(detect_core_key_from_path)
        .filter(|value| value != "unknown")
}

fn detect_core_key_from_path(path: &str) -> String {
    let lowered = path.to_ascii_lowercase();
    if lowered.contains("arclight") && lowered.contains("neoforge") {
        "arclight-neoforge".to_string()
    } else if lowered.contains("arclight") && lowered.contains("forge") {
        "arclight-forge".to_string()
    } else if lowered.contains("neoforge") {
        "neoforge".to_string()
    } else if lowered.contains("catserver") {
        "catserver".to_string()
    } else if lowered.contains("mohist") {
        "mohist".to_string()
    } else if lowered.contains("forge") {
        "forge".to_string()
    } else {
        "unknown".to_string()
    }
}

fn starter_install_args(core_key: Option<&str>) -> Vec<String> {
    let args = match core_key {
        Some("neoforge") | Some("arclight-neoforge") => STARTER_INSTALL_ARGS_NEOFORGE_LIKE,
        Some("forge") | Some("arclight-forge") | Some("catserver") | Some("mohist") => {
            STARTER_INSTALL_ARGS_FORGE_LIKE
        }
        _ => STARTER_INSTALL_ARGS_STANDARD,
    };

    args.iter().map(|value| (*value).to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::{plan_local_launch, LocalLaunchPlanInput};
    use server_flow::{LocalLaunchEntry, StartupMode};
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
            let path = std::env::temp_dir().join(format!("sl-launch-plan-{}-{}", prefix, unique));
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

    #[test]
    fn plans_direct_jar_launch_with_port_probe() {
        let dir = TestDir::new("jar");
        let jar_path = dir.path().join("server.jar");
        std::fs::write(&jar_path, b"placeholder").unwrap();

        let plan = plan_local_launch(LocalLaunchPlanInput {
            server_id: "jar-demo".to_string(),
            server_root: dir.path().to_path_buf(),
            startup_mode: "jar".to_string(),
            configured_startup_path: Some(jar_path.to_string_lossy().to_string()),
            custom_command: None,
            java_path: Some(PathBuf::from("java")),
            java_home: None,
            java_bin_dir: None,
            jvm_args: vec!["-Xmx2G".to_string()],
            add_nogui: true,
            port: Some(25565),
        })
        .expect("jar launch plan should build");

        assert_eq!(plan.spec.startup_mode(), StartupMode::Jar);
        assert!(plan.spec.port_probe.is_some());
        match plan.spec.entry {
            LocalLaunchEntry::DirectJar { jar_path, java } => {
                assert!(jar_path.ends_with("server.jar"));
                assert_eq!(java.jvm_args, vec!["-Xmx2G".to_string()]);
                assert!(java.add_nogui);
            }
            other => panic!("unexpected entry: {other:?}"),
        }
    }

    #[test]
    fn plans_script_launch_with_nogui_trailing_arg() {
        let dir = TestDir::new("script");
        let script_path = dir.path().join("start.sh");
        std::fs::write(&script_path, "#!/bin/sh\n").unwrap();

        let plan = plan_local_launch(LocalLaunchPlanInput {
            server_id: "script-demo".to_string(),
            server_root: dir.path().to_path_buf(),
            startup_mode: "sh".to_string(),
            configured_startup_path: Some(script_path.to_string_lossy().to_string()),
            custom_command: None,
            java_path: None,
            java_home: Some(PathBuf::from("/opt/jdk")),
            java_bin_dir: Some(PathBuf::from("/opt/jdk/bin")),
            jvm_args: Vec::new(),
            add_nogui: true,
            port: None,
        })
        .expect("script launch plan should build");

        assert_eq!(plan.spec.startup_mode(), StartupMode::Sh);
        match plan.spec.entry {
            LocalLaunchEntry::Script(script) => {
                assert!(script.script_path.ends_with("start.sh"));
                assert_eq!(script.trailing_args, vec!["nogui".to_string()]);
                assert!(script.java_env.is_some());
            }
            other => panic!("unexpected entry: {other:?}"),
        }
    }

    #[test]
    fn plans_custom_command_launch() {
        let dir = TestDir::new("custom");

        let plan = plan_local_launch(LocalLaunchPlanInput {
            server_id: "custom-demo".to_string(),
            server_root: dir.path().to_path_buf(),
            startup_mode: "custom".to_string(),
            configured_startup_path: None,
            custom_command: Some("java -jar custom.jar nogui".to_string()),
            java_path: None,
            java_home: None,
            java_bin_dir: None,
            jvm_args: Vec::new(),
            add_nogui: false,
            port: None,
        })
        .expect("custom launch plan should build");

        assert_eq!(plan.resolution.launch_target, "java -jar custom.jar nogui");
        match plan.spec.entry {
            LocalLaunchEntry::CustomCommand { command, .. } => {
                assert_eq!(command, "java -jar custom.jar nogui");
            }
            other => panic!("unexpected entry: {other:?}"),
        }
    }

    #[test]
    fn custom_mode_rejects_missing_command() {
        let dir = TestDir::new("custom-missing");

        let error = plan_local_launch(LocalLaunchPlanInput {
            server_id: "custom-demo".to_string(),
            server_root: dir.path().to_path_buf(),
            startup_mode: "custom".to_string(),
            configured_startup_path: None,
            custom_command: None,
            java_path: None,
            java_home: None,
            java_bin_dir: None,
            jvm_args: Vec::new(),
            add_nogui: false,
            port: None,
        })
        .expect_err("custom mode should reject empty command");

        assert!(error.contains("Missing custom command"));
    }

    #[test]
    fn plans_starter_launch_as_starter_entry() {
        let dir = TestDir::new("starter");
        let installer_path = dir.path().join("neoforge-installer.jar");
        std::fs::write(&installer_path, b"placeholder").unwrap();

        let plan = plan_local_launch(LocalLaunchPlanInput {
            server_id: "starter-demo".to_string(),
            server_root: dir.path().to_path_buf(),
            startup_mode: "starter".to_string(),
            configured_startup_path: Some(installer_path.to_string_lossy().to_string()),
            custom_command: None,
            java_path: Some(PathBuf::from("java")),
            java_home: None,
            java_bin_dir: None,
            jvm_args: Vec::new(),
            add_nogui: false,
            port: None,
        })
        .expect("starter launch plan should build");

        assert_eq!(plan.spec.startup_mode(), StartupMode::Starter);
        match plan.spec.entry {
            LocalLaunchEntry::StarterInstall(starter) => {
                assert!(starter.installer_jar.ends_with("neoforge-installer.jar"));
                assert_eq!(
                    starter.install_args,
                    vec![
                        "--install-server".to_string(),
                        ".".to_string(),
                        "--server-starter".to_string()
                    ]
                );
            }
            other => panic!("unexpected entry: {other:?}"),
        }
    }
}
