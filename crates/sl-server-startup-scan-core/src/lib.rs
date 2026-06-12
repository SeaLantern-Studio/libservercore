//! Shared startup candidate scanning logic for Minecraft server folders and archives.

#![forbid(unsafe_code)]

use std::cmp::Ordering;
use std::io::Read;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use server_core_taxonomy::normalize_core_key;
use tar::Archive;
use zip::ZipArchive;

const STARTUP_SCAN_CORE_KEY_OPTIONS: &[&str] = &[
    "pumpkin", "paper", "purpur", "spigot", "bukkit", "folia", "leaves",
    "pufferfish", "sponge", "arclight_forge", "arclight_neoforge", "mohist",
    "catserver", "neoforge", "forge", "fabric", "quilt", "vanilla", "velocity",
    "bungeecord", "waterfall", "lightfall", "travertine", "flamecord", "tuinity",
    "airplane", "glowstone", "cuberite", "minestom", "bds", "liteloaderbds",
    "levilamina", "bdsx", "allay", "nukkit", "powernukkitx", "pocketmine",
    "endstone",
];

const PREFERRED_SERVER_JAR_PATTERNS: &[&str] = &[
    "server.jar",
    "forge.jar",
    "fabric-server.jar",
    "minecraft_server.jar",
    "paper.jar",
    "spigot.jar",
    "purpur.jar",
];

const INDICATIVE_SERVER_JAR_KEYWORDS: &[&str] = &[
    "server", "forge", "fabric", "neoforge", "mohist", "paper", "spigot", "purpur",
    "bukkit", "catserver", "arclight", "velocity", "waterfall", "bungee", "folia",
    "pufferfish", "leaves", "quilt",
];

const STARTER_MAIN_CLASS_PREFIX: &str = "net.neoforged.serverstarterjar";
const FORGE_SIMPLE_INSTALLER_MAIN_CLASS: &str = "net.minecraftforge.installer.SimpleInstaller";

struct TempExtractDir(PathBuf);

impl TempExtractDir {
    fn new(prefix: &str) -> Result<Self, String> {
        let path = std::env::temp_dir().join(format!("{}_{}", prefix, uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path)
            .map_err(|e| format!("Failed to create temp extract dir: {e}"))?;
        Ok(Self(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempExtractDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupSourceKind {
    Folder,
    Archive,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StartupScanConfidence {
    Explicit,
    Likely,
    Heuristic,
    Unknown,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupCandidateKind {
    Jar,
    Starter,
    Script,
    NativeExecutable,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedStartupCoreInfo {
    pub detected_core_key: Option<String>,
    pub display_hint: String,
    pub main_class: Option<String>,
    pub startup_path: Option<String>,
    pub confidence: StartupScanConfidence,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupCandidateItem {
    pub id: String,
    pub mode: String,
    pub label: String,
    pub detail: String,
    pub path: String,
    pub resolved_target_path: Option<String>,
    pub recommended_rank: u8,
    pub kind: StartupCandidateKind,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupScanResult {
    pub parsed_core: ParsedStartupCoreInfo,
    pub candidates: Vec<StartupCandidateItem>,
    pub core_key_options: Vec<String>,
    pub mc_version_options: Vec<String>,
    pub detected_mc_version: Option<String>,
    pub mc_version_detection_failed: bool,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedLaunchTarget {
    pub startup_mode: String,
    pub launch_target: String,
    pub preferred_jar_path: Option<String>,
    pub startup_filename: String,
}

pub fn scan_startup_candidates(
    source_path: &str,
    source_kind: StartupSourceKind,
    mc_version_options: &[&str],
) -> Result<StartupScanResult, String> {
    let source = Path::new(source_path);
    if !source.exists() {
        return Err(format!("Path does not exist: {source_path}"));
    }

    match source_kind {
        StartupSourceKind::Folder => scan_folder_source(source, mc_version_options),
        StartupSourceKind::Archive => scan_archive_source(source_path, mc_version_options),
    }
}

pub fn resolve_preferred_jar_path(
    startup_mode: &str,
    configured_startup_path: Option<&str>,
    server_root: &Path,
) -> Result<Option<String>, String> {
    if !startup_mode_prefers_direct_jar(startup_mode) {
        return Ok(None);
    }

    let Some(configured_startup_path) = configured_startup_path.map(str::trim) else {
        return Ok(None);
    };
    if configured_startup_path.is_empty() {
        return Ok(None);
    }

    let startup_path_obj = Path::new(configured_startup_path);
    if startup_path_obj
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("jar"))
    {
        Ok(Some(configured_startup_path.to_string()))
    } else {
        find_server_jar_checked(server_root).map(Some)
    }
}

pub fn resolve_direct_jar_launch_target(server_root: &Path, jar_path: &str) -> String {
    let jar_path_obj = Path::new(jar_path);
    if jar_path_obj.parent() == Some(server_root) {
        return jar_path_obj
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| jar_path.to_string());
    }

    jar_path.to_string()
}

pub fn resolve_launch_target(
    startup_mode: &str,
    preferred_jar_path: Option<&str>,
    configured_startup_path: Option<&str>,
    custom_command: Option<&str>,
    startup_filename: &str,
) -> String {
    if let Some(preferred_jar_path) = preferred_jar_path {
        return preferred_jar_path.to_string();
    }

    match normalize_startup_mode(startup_mode).as_str() {
        "jar" | "starter" => configured_startup_path.unwrap_or_default().to_string(),
        "custom" => custom_command.unwrap_or_default().to_string(),
        _ => startup_filename.to_string(),
    }
}

pub fn resolve_mode_aware_launch_target(
    startup_mode: &str,
    configured_startup_path: Option<&str>,
    custom_command: Option<&str>,
    server_root: &Path,
) -> Result<ResolvedLaunchTarget, String> {
    let normalized_mode = normalize_startup_mode(startup_mode);
    let preferred_jar_path = resolve_preferred_jar_path(
        &normalized_mode,
        configured_startup_path,
        server_root,
    )?;
    let startup_filename = configured_startup_path
        .and_then(|value| Path::new(value).file_name())
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    let launch_target = resolve_launch_target(
        &normalized_mode,
        preferred_jar_path.as_deref(),
        configured_startup_path,
        custom_command,
        &startup_filename,
    );

    Ok(ResolvedLaunchTarget {
        startup_mode: normalized_mode,
        launch_target,
        preferred_jar_path,
        startup_filename,
    })
}

fn scan_folder_source(
    source: &Path,
    mc_version_options: &[&str],
) -> Result<StartupScanResult, String> {
    let entries = collect_folder_entries_checked(source)?;
    let mut candidates = Vec::new();
    let mut detected_core: Option<(u8, bool, String, ParsedStartupCoreInfo)> = None;

    for path in entries {
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let full_path = path.to_string_lossy().to_string();

        if is_pumpkin_executable(&path) {
            let parsed_info = ParsedStartupCoreInfo {
                detected_core_key: Some("pumpkin".to_string()),
                display_hint: "pumpkin".to_string(),
                main_class: None,
                startup_path: Some(full_path.clone()),
                confidence: StartupScanConfidence::Explicit,
            };

            update_detected_core(&mut detected_core, 1, &filename, &parsed_info);

            candidates.push(StartupCandidateItem {
                id: format!("custom-{filename}"),
                mode: "custom".to_string(),
                label: "Pumpkin".to_string(),
                detail: "Pumpkin executable".to_string(),
                path: full_path,
                resolved_target_path: None,
                recommended_rank: 1,
                kind: StartupCandidateKind::NativeExecutable,
            });
            continue;
        }

        if extension == "jar" {
            let parsed = parse_startup_core_from_jar_path(&full_path)
                .map_err(|error| format!("Failed to scan startup candidates: {error}"))?;
            let is_starter = is_starter_candidate(&parsed);
            let is_server_jar = filename.eq_ignore_ascii_case("server.jar");
            let recommended_rank = if is_starter {
                1
            } else if is_server_jar {
                3
            } else {
                4
            };
            let label = if is_forge_like_installer_main_class(&parsed) {
                "Installer".to_string()
            } else if is_starter {
                "Starter".to_string()
            } else if is_server_jar {
                "server.jar".to_string()
            } else {
                filename.clone()
            };

            update_detected_core(&mut detected_core, recommended_rank, &label, &parsed);

            candidates.push(StartupCandidateItem {
                id: format!("jar-{filename}"),
                mode: if is_starter { "starter" } else { "jar" }.to_string(),
                label,
                detail: startup_detail(&parsed),
                path: full_path,
                resolved_target_path: None,
                recommended_rank,
                kind: if is_starter {
                    StartupCandidateKind::Starter
                } else {
                    StartupCandidateKind::Jar
                },
            });
            continue;
        }

        if matches!(extension.as_str(), "bat" | "cmd" | "sh" | "ps1") {
            let script_target = parse_script_startup_target(&path)?;
            if let Some(parsed_target) = script_target
                .as_ref()
                .and_then(|target| build_parsed_core_from_script_target(source, target).ok())
            {
                update_detected_core(&mut detected_core, 2, &filename, &parsed_target);
            }

            candidates.push(StartupCandidateItem {
                id: format!("{extension}-{filename}"),
                mode: if extension == "cmd" { "bat" } else { &extension }.to_string(),
                label: filename,
                detail: script_detail(script_target.as_deref()),
                path: full_path,
                resolved_target_path: script_target,
                recommended_rank: 2,
                kind: StartupCandidateKind::Script,
            });
        }
    }

    candidates.sort_by(|left, right| {
        left.recommended_rank
            .cmp(&right.recommended_rank)
            .then_with(|| left.label.cmp(&right.label))
    });

    let parsed_core = detected_core
        .map(|(_, _, _, parsed)| parsed)
        .unwrap_or_else(unknown_parsed_core_info);

    Ok(build_result(
        parsed_core,
        candidates,
        None,
        false,
        mc_version_options,
    ))
}

fn scan_archive_source(
    source_path: &str,
    mc_version_options: &[&str],
) -> Result<StartupScanResult, String> {
    let source = Path::new(source_path);

    if source.is_file() {
        let extension = source
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .unwrap_or_default();

        if extension == "jar" {
            let parsed = parse_startup_core_from_jar_path(source_path)?;
            let is_starter = is_starter_candidate(&parsed);
            let label = if is_forge_like_installer_main_class(&parsed) {
                "Installer"
            } else if is_starter {
                "Starter"
            } else {
                "server.jar"
            };

            return Ok(build_result(
                parsed.clone(),
                vec![StartupCandidateItem {
                    id: format!("archive-{}", if is_starter { "starter" } else { "jar" }),
                    mode: if is_starter { "starter" } else { "jar" }.to_string(),
                    label: label.to_string(),
                    detail: startup_detail(&parsed),
                    path: source_path.to_string(),
                    resolved_target_path: None,
                    recommended_rank: if is_starter { 1 } else { 3 },
                    kind: if is_starter {
                        StartupCandidateKind::Starter
                    } else {
                        StartupCandidateKind::Jar
                    },
                }],
                None,
                false,
                mc_version_options,
            ));
        }

        if is_pumpkin_executable(source) {
            return Ok(build_result(
                ParsedStartupCoreInfo {
                    detected_core_key: Some("pumpkin".to_string()),
                    display_hint: "pumpkin".to_string(),
                    main_class: None,
                    startup_path: Some(source_path.to_string()),
                    confidence: StartupScanConfidence::Explicit,
                },
                vec![StartupCandidateItem {
                    id: "archive-custom-pumpkin".to_string(),
                    mode: "custom".to_string(),
                    label: "Pumpkin".to_string(),
                    detail: "Pumpkin executable".to_string(),
                    path: source_path.to_string(),
                    resolved_target_path: None,
                    recommended_rank: 1,
                    kind: StartupCandidateKind::NativeExecutable,
                }],
                None,
                false,
                mc_version_options,
            ));
        }
    }

    let mut temp_extract_dir: Option<TempExtractDir> = None;

    let inspect_root = if source.is_file() {
        let temp_dir = TempExtractDir::new("sl_server_startup_scan")?;
        extract_modpack_archive(source, temp_dir.path())?;
        let root_dir = resolve_extracted_root_checked(temp_dir.path())?;
        temp_extract_dir = Some(temp_dir);
        root_dir
    } else if source.is_dir() {
        source.to_path_buf()
    } else {
        return Err("Invalid archive source".to_string());
    };

    let mut parsed = parse_startup_core_from_directory(&inspect_root)?;
    if let (Some(temp_dir), Some(startup_path)) = (temp_extract_dir.as_ref(), parsed.startup_path.clone()) {
        parsed.startup_path = Some(to_relative_archive_path(temp_dir.path(), &startup_path)?);
    }

    let mut candidates = Vec::new();
    if let Some(startup_path) = parsed.startup_path.clone() {
        let is_starter = is_starter_candidate(&parsed);
        let label = if is_forge_like_installer_main_class(&parsed) {
            "Installer"
        } else if is_starter {
            "Starter"
        } else {
            "server.jar"
        };

        candidates.push(StartupCandidateItem {
            id: format!("archive-{}", if is_starter { "starter" } else { "jar" }),
            mode: if is_starter { "starter" } else { "jar" }.to_string(),
            label: label.to_string(),
            detail: startup_detail(&parsed),
            path: startup_path,
            resolved_target_path: None,
            recommended_rank: if is_starter { 1 } else { 3 },
            kind: if is_starter {
                StartupCandidateKind::Starter
            } else {
                StartupCandidateKind::Jar
            },
        });
    }

    Ok(build_result(
        parsed,
        candidates,
        None,
        false,
        mc_version_options,
    ))
}

fn parse_startup_core_from_directory(root: &Path) -> Result<ParsedStartupCoreInfo, String> {
    let detected_jar = match find_server_jar_checked(root) {
        Ok(path) => Some(path),
        Err(error) if error == "No JAR file found in server root" => None,
        Err(error) => return Err(error),
    };

    if let Some(jar_path) = detected_jar {
        return parse_startup_core_from_jar_path(&jar_path);
    }

    Ok(unknown_parsed_core_info())
}

fn build_result(
    parsed_core: ParsedStartupCoreInfo,
    candidates: Vec<StartupCandidateItem>,
    detected_mc_version: Option<String>,
    mc_version_detection_failed: bool,
    mc_version_options: &[&str],
) -> StartupScanResult {
    StartupScanResult {
        parsed_core,
        candidates,
        core_key_options: STARTUP_SCAN_CORE_KEY_OPTIONS
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        mc_version_options: mc_version_options
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        detected_mc_version,
        mc_version_detection_failed,
    }
}

fn build_parsed_core_from_script_target(
    server_root: &Path,
    target: &str,
) -> Result<ParsedStartupCoreInfo, String> {
    let normalized = target.trim();
    if normalized.is_empty() {
        return Ok(unknown_parsed_core_info());
    }

    let target_path = resolve_script_target_path(server_root, normalized);
    let extension = target_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if extension == "jar" {
        return parse_startup_core_from_jar_path(&target_path.to_string_lossy());
    }

    if is_pumpkin_executable(&target_path) {
        return Ok(ParsedStartupCoreInfo {
            detected_core_key: Some("pumpkin".to_string()),
            display_hint: "pumpkin".to_string(),
            main_class: None,
            startup_path: Some(target_path.to_string_lossy().to_string()),
            confidence: StartupScanConfidence::Likely,
        });
    }

    let display_hint = detect_core_display_hint_from_filename(normalized);
    let detected_core_key = if display_hint == "unknown" {
        None
    } else {
        Some(display_hint.clone())
    };

    Ok(ParsedStartupCoreInfo {
        detected_core_key,
        display_hint,
        main_class: None,
        startup_path: Some(target_path.to_string_lossy().to_string()),
        confidence: StartupScanConfidence::Heuristic,
    })
}

fn startup_mode_prefers_direct_jar(startup_mode: &str) -> bool {
    matches!(normalize_startup_mode(startup_mode).as_str(), "jar" | "starter")
}

fn normalize_startup_mode(startup_mode: &str) -> String {
    match startup_mode.trim().to_ascii_lowercase().as_str() {
        "jar" => "jar".to_string(),
        "starter" => "starter".to_string(),
        "custom" => "custom".to_string(),
        "bat" | "cmd" => "bat".to_string(),
        "sh" => "sh".to_string(),
        "ps1" => "ps1".to_string(),
        _ => "jar".to_string(),
    }
}

fn script_detail(target: Option<&str>) -> String {
    match target {
        Some(target) => format!("Script -> {target}"),
        None => "Script".to_string(),
    }
}

fn parse_script_startup_target(script_path: &Path) -> Result<Option<String>, String> {
    let content = std::fs::read_to_string(script_path)
        .map_err(|e| format!("Failed to read script file {}: {e}", script_path.display()))?;

    for line in content.lines() {
        if let Some(target) = extract_target_from_script_line(line) {
            return Ok(Some(target));
        }
    }

    Ok(None)
}

fn extract_target_from_script_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let lowered = trimmed.to_ascii_lowercase();
    if lowered.starts_with("#") || lowered.starts_with("rem ") || lowered.starts_with("::") {
        return None;
    }

    if let Some(target) = extract_java_jar_target(trimmed) {
        return Some(target);
    }

    extract_called_script_target(trimmed)
}

fn extract_java_jar_target(line: &str) -> Option<String> {
    let tokens = tokenize_script_line(line);
    for window in tokens.windows(2) {
        if window[0].eq_ignore_ascii_case("-jar") {
            return Some(clean_script_token(&window[1]));
        }
    }
    None
}

fn extract_called_script_target(line: &str) -> Option<String> {
    let tokens = tokenize_script_line(line);
    let first = tokens.first()?;
    let first_lower = first.to_ascii_lowercase();
    if first_lower == "&"
        || first_lower == "call"
        || first_lower == "start"
        || first_lower == "./start.sh"
        || first_lower == "./run.sh"
        || first_lower == "sh"
        || first_lower == "bash"
        || first_lower == "powershell"
    {
        let target = tokens.get(1)?;
        return Some(clean_script_token(target));
    }

    let cleaned = clean_script_token(first);
    if has_startup_like_extension(&cleaned) || cleaned.ends_with(".jar") || cleaned.contains("pumpkin") {
        return Some(cleaned);
    }

    None
}

fn tokenize_script_line(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;

    for ch in line.chars() {
        if let Some(active) = quote {
            if ch == active {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }

        match ch {
            '"' | '\'' => {
                quote = Some(ch);
            }
            ' ' | '\t' => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn clean_script_token(token: &str) -> String {
    token
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim_end_matches(',')
        .trim_end_matches(';')
        .to_string()
}

fn has_startup_like_extension(path: &str) -> bool {
    let lowered = path.to_ascii_lowercase();
    lowered.ends_with(".bat")
        || lowered.ends_with(".cmd")
        || lowered.ends_with(".sh")
        || lowered.ends_with(".ps1")
        || lowered.ends_with(".exe")
}

fn resolve_script_target_path(server_root: &Path, target: &str) -> PathBuf {
    let candidate = Path::new(target);
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        server_root.join(candidate)
    }
}

fn parse_startup_core_from_jar_path(jar_path: &str) -> Result<ParsedStartupCoreInfo, String> {
    let display_hint = detect_core_display_hint_from_filename(jar_path);
    let main_class = read_jar_main_class_checked(jar_path)?;
    let detected_core_key = detect_core_key_from_filename_and_main_class(jar_path, main_class.as_deref());
    let confidence = if detected_core_key.is_some() && main_class.is_some() {
        StartupScanConfidence::Explicit
    } else if detected_core_key.is_some() {
        StartupScanConfidence::Likely
    } else {
        StartupScanConfidence::Unknown
    };

    Ok(ParsedStartupCoreInfo {
        detected_core_key,
        display_hint,
        main_class,
        startup_path: Some(jar_path.to_string()),
        confidence,
    })
}

fn unknown_parsed_core_info() -> ParsedStartupCoreInfo {
    ParsedStartupCoreInfo {
        detected_core_key: None,
        display_hint: "unknown".to_string(),
        main_class: None,
        startup_path: None,
        confidence: StartupScanConfidence::Unknown,
    }
}

fn collect_folder_entries_checked(source: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(source).map_err(|e| format!("Failed to read directory: {e}"))?;
    let mut paths = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {e}"))?;
        let path = entry.path();
        if path.is_file() {
            paths.push(path);
        }
    }

    Ok(paths)
}

fn update_detected_core(
    detected_core: &mut Option<(u8, bool, String, ParsedStartupCoreInfo)>,
    recommended_rank: u8,
    label: &str,
    parsed: &ParsedStartupCoreInfo,
) {
    let is_unknown = parsed.detected_core_key.is_none();
    let normalized_label = label.to_ascii_lowercase();
    let should_replace = detected_core
        .as_ref()
        .map(|(best_rank, best_unknown, best_label, _)| {
            (recommended_rank, is_unknown, normalized_label.clone())
                < (*best_rank, *best_unknown, best_label.clone())
        })
        .unwrap_or(true);

    if should_replace {
        *detected_core = Some((recommended_rank, is_unknown, normalized_label, parsed.clone()));
    }
}

fn is_pumpkin_executable(path: &Path) -> bool {
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    filename.contains("pumpkin") && (extension == "exe" || extension.is_empty())
}

fn is_starter_candidate(parsed: &ParsedStartupCoreInfo) -> bool {
    is_starter_main_class(parsed) || is_forge_like_installer_main_class(parsed)
}

fn is_starter_main_class(parsed: &ParsedStartupCoreInfo) -> bool {
    parsed
        .main_class
        .as_deref()
        .map(|value| value.starts_with(STARTER_MAIN_CLASS_PREFIX))
        .unwrap_or(false)
}

fn is_forge_like_installer_main_class(parsed: &ParsedStartupCoreInfo) -> bool {
    parsed.main_class.as_deref() == Some(FORGE_SIMPLE_INSTALLER_MAIN_CLASS)
}

fn startup_detail(parsed: &ParsedStartupCoreInfo) -> String {
    [Some(format_core_label(
        parsed
            .detected_core_key
            .as_deref()
            .unwrap_or(parsed.display_hint.as_str()),
    )), parsed.main_class.clone()]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ")
}

fn format_core_label(core_key: &str) -> String {
    match core_key {
        "allay" => "AllayMC".to_string(),
        "airplane" => "Airplane".to_string(),
        "arclight_forge" => "Arclight-Forge".to_string(),
        "arclight_neoforge" => "Arclight-NeoForge".to_string(),
        "bds" => "BDS".to_string(),
        "bdsx" => "BDSX".to_string(),
        "bungeecord" => "BungeeCord".to_string(),
        "cuberite" => "Cuberite".to_string(),
        "endstone" => "Endstone".to_string(),
        "flamecord" => "FlameCord".to_string(),
        "levilamina" => "LeviLamina".to_string(),
        "liteloaderbds" => "LiteLoaderBDS".to_string(),
        "minestom" => "Minestom".to_string(),
        "neoforge" => "NeoForge".to_string(),
        "pocketmine" => "PocketMine-MP".to_string(),
        "powernukkitx" => "PowerNukkitX".to_string(),
        "pumpkin" => "Pumpkin".to_string(),
        value => value.to_string(),
    }
}

fn detect_core_display_hint_from_filename(input: &str) -> String {
    let filename = Path::new(input)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(input)
        .to_ascii_lowercase();

    let raw_hint = if filename.contains("arclight") && filename.contains("neoforge") {
        "arclight_neoforge"
    } else if filename.contains("arclight") && filename.contains("forge") {
        "arclight_forge"
    } else if filename.contains("neoforge") {
        "neoforge"
    } else if filename.contains("forge") {
        "forge"
    } else if filename.contains("paper") {
        "paper"
    } else if filename.contains("folia") {
        "folia"
    } else if filename.contains("purpur") {
        "purpur"
    } else if filename.contains("pufferfish") {
        "pufferfish"
    } else if filename.contains("leaves") || filename.contains("leaf") {
        "leaves"
    } else if filename.contains("spigot") {
        "spigot"
    } else if filename.contains("bukkit") || filename.contains("craftbukkit") {
        "bukkit"
    } else if filename.contains("velocity") {
        "velocity"
    } else if filename.contains("waterfall") {
        "waterfall"
    } else if filename.contains("bungeecord") || filename.contains("bungee") {
        "bungeecord"
    } else if filename.contains("fabric") {
        "fabric"
    } else if filename.contains("quilt") {
        "quilt"
    } else if filename.contains("mohist") {
        "mohist"
    } else if filename.contains("catserver") {
        "catserver"
    } else if filename.contains("vanilla") || filename.contains("minecraft_server") {
        "vanilla"
    } else if filename.contains("allay") {
        "allay"
    } else if filename.contains("powernukkitx") || filename.contains("powernukkit") {
        "powernukkitx"
    } else if filename.contains("nukkit") {
        "nukkit"
    } else if filename.contains("liteloader") && filename.contains("bds") {
        "liteloaderbds"
    } else if filename.contains("levilamina") {
        "levilamina"
    } else if filename.contains("bdsx") {
        "bdsx"
    } else if filename.contains("bedrock") {
        "bds"
    } else if filename.contains("pocketmine") {
        "pocketmine"
    } else if filename.contains("endstone") {
        "endstone"
    } else if filename.contains("cuberite") {
        "cuberite"
    } else if filename.contains("minestom") {
        "minestom"
    } else if filename.contains("sponge") {
        "sponge"
    } else {
        "unknown"
    };

    normalize_core_key(raw_hint)
        .map(|value| value.to_string())
        .unwrap_or_else(|| raw_hint.to_string())
}

fn detect_core_key_from_filename_and_main_class(
    jar_path: &str,
    main_class: Option<&str>,
) -> Option<String> {
    let filename_hint = detect_core_display_hint_from_filename(jar_path);
    let main_class_hint = main_class.and_then(core_key_from_main_class);

    match (filename_hint.as_str(), main_class_hint) {
        ("neoforge", Some("forge")) | ("arclight_neoforge", Some("forge")) => {
            Some(filename_hint)
        }
        (_, Some(main_class_hint)) => Some(main_class_hint.to_string()),
        _ if filename_hint != "unknown" => Some(filename_hint),
        _ => None,
    }
}

fn core_key_from_main_class(main_class: &str) -> Option<&'static str> {
    match main_class {
        value if value.starts_with(STARTER_MAIN_CLASS_PREFIX) => Some("neoforge"),
        "net.minecraft.server.MinecraftServer" | "net.minecraft.bundler.Main" => Some("vanilla"),
        "net.minecraft.client.Main" => None,
        FORGE_SIMPLE_INSTALLER_MAIN_CLASS => Some("forge"),
        "net.fabricmc.installer.Main" | "net.fabricmc.installer.ServerLauncher" => Some("fabric"),
        "io.izzel.arclight.server.Launcher" => Some("arclight_forge"),
        "catserver.server.CatServerLaunch" | "foxlaunch.FoxServerLauncher" => Some("catserver"),
        "org.bukkit.craftbukkit.Main" | "org.bukkit.craftbukkit.bootstrap.Main" => Some("bukkit"),
        "io.papermc.paperclip.Main" | "com.destroystokyo.paperclip.Paperclip" => Some("paper"),
        "org.leavesmc.leavesclip.Main" => Some("leaves"),
        "net.md_5.bungee.Bootstrap" => Some("lightfall"),
        "com.mohistmc.MohistMCStart" | "com.mohistmc.MohistMC" => Some("mohist"),
        "com.velocitypowered.proxy.Velocity" => Some("velocity"),
        _ => None,
    }
}

fn read_jar_main_class_checked(jar_path: &str) -> Result<Option<String>, String> {
    let file = std::fs::File::open(jar_path).map_err(|e| format!("Failed to open JAR file: {e}"))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Failed to parse JAR archive: {e}"))?;
    let mut manifest = archive
        .by_name("META-INF/MANIFEST.MF")
        .map_err(|e| format!("Failed to read JAR manifest: {e}"))?;

    let mut bytes = Vec::new();
    manifest
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Failed to read JAR manifest content: {e}"))?;
    let content = String::from_utf8_lossy(&bytes).to_string();

    Ok(find_manifest_main_class(&content))
}

fn find_manifest_main_class(manifest_content: &str) -> Option<String> {
    let mut current_key = String::new();
    let mut current_value = String::new();

    let flush_entry = |key: &str, value: &str| {
        if key.eq_ignore_ascii_case("Main-Class") {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        } else {
            None
        }
    };

    for line in manifest_content.lines() {
        if line.is_empty() {
            if let Some(value) = flush_entry(&current_key, &current_value) {
                return Some(value);
            }
            current_key.clear();
            current_value.clear();
            continue;
        }

        if line.starts_with(' ') {
            current_value.push_str(line.trim_start());
            continue;
        }

        if let Some(value) = flush_entry(&current_key, &current_value) {
            return Some(value);
        }

        if let Some((key, value)) = line.split_once(':') {
            current_key.clear();
            current_key.push_str(key.trim());
            current_value.clear();
            current_value.push_str(value.trim());
        } else {
            current_key.clear();
            current_value.clear();
        }
    }

    flush_entry(&current_key, &current_value)
}

fn extract_modpack_archive(archive_path: &Path, target_dir: &Path) -> Result<(), String> {
    let lower_name = archive_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_ascii_lowercase())
        .unwrap_or_default();

    if lower_name.ends_with(".zip") {
        let file = std::fs::File::open(archive_path)
            .map_err(|e| format!("Failed to open archive file: {e}"))?;
        let mut archive = ZipArchive::new(file)
            .map_err(|e| format!("Failed to parse ZIP archive: {e}"))?;
        return extract_zip_archive(&mut archive, target_dir);
    }

    if lower_name.ends_with(".tar.gz") || lower_name.ends_with(".tgz") {
        let file = std::fs::File::open(archive_path)
            .map_err(|e| format!("Failed to open archive file: {e}"))?;
        let decoder = GzDecoder::new(file);
        return extract_tar_archive(decoder, target_dir);
    }

    if lower_name.ends_with(".tar") {
        let file = std::fs::File::open(archive_path)
            .map_err(|e| format!("Failed to open archive file: {e}"))?;
        return extract_tar_archive(file, target_dir);
    }

    Err("Unsupported archive format; only .zip, .tar, .tar.gz, and .tgz are supported".to_string())
}

fn resolve_extracted_root_checked(extract_dir: &Path) -> Result<PathBuf, String> {
    let entries = std::fs::read_dir(extract_dir)
        .map_err(|error| format!("Failed to read extract directory: {error}"))?;
    let mut directories = Vec::new();
    let mut file_count = 0;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read extract directory entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            directories.push(path);
        } else {
            file_count += 1;
        }
    }

    if file_count == 0 && directories.len() == 1 {
        return Ok(directories.remove(0));
    }

    Ok(extract_dir.to_path_buf())
}

fn find_server_jar_checked(server_root: &Path) -> Result<String, String> {
    for pattern in PREFERRED_SERVER_JAR_PATTERNS {
        let jar_path = server_root.join(pattern);
        if jar_path.is_file() {
            return Ok(jar_path.to_string_lossy().to_string());
        }
    }

    let entries = std::fs::read_dir(server_root)
        .map_err(|e| format!("Failed to read server root: {e}"))?;
    let mut jar_files = Vec::new();
    let mut invalid_jar_dirs = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read server root entry: {e}"))?;
        let path = entry.path();
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("jar"))
        {
            if path.is_file() {
                jar_files.push(path);
            } else if path.is_dir() {
                invalid_jar_dirs.push(path);
            }
        }
    }

    let selected = if let Some(path) = select_best_server_jar_path(jar_files) {
        path
    } else if let Some(path) = invalid_jar_dirs.into_iter().next() {
        return Err(format!("Detected a directory masquerading as a JAR file: {}", path.display()));
    } else {
        return Err("No JAR file found in server root".to_string());
    };

    Ok(selected.to_string_lossy().to_string())
}

fn select_best_server_jar_path(mut jar_files: Vec<PathBuf>) -> Option<PathBuf> {
    if jar_files.is_empty() {
        return None;
    }

    jar_files.sort_by(|left, right| compare_server_jar_candidates(left, right));
    jar_files.into_iter().next()
}

fn compare_server_jar_candidates(left: &Path, right: &Path) -> Ordering {
    let left_name = left
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let right_name = right
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    server_jar_candidate_rank(&left_name)
        .cmp(&server_jar_candidate_rank(&right_name))
        .then_with(|| left_name.cmp(&right_name))
}

fn server_jar_candidate_rank(file_name: &str) -> u8 {
    if INDICATIVE_SERVER_JAR_KEYWORDS
        .iter()
        .any(|keyword| file_name.contains(keyword))
    {
        0
    } else {
        1
    }
}

fn extract_zip_archive(
    archive: &mut ZipArchive<std::fs::File>,
    target_dir: &Path,
) -> Result<(), String> {
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|e| format!("Failed to read ZIP entry: {e}"))?;
        let enclosed_path = file
            .enclosed_name()
            .ok_or_else(|| "ZIP entry contains an invalid path".to_string())?;
        let out_path = target_dir.join(enclosed_path);

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&out_path)
                .map_err(|e| format!("Failed to create directory: {e}"))?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {e}"))?;
        }

        let mut out_file = std::fs::File::create(&out_path)
            .map_err(|e| format!("Failed to create file: {e}"))?;
        std::io::copy(&mut file, &mut out_file)
            .map_err(|e| format!("Failed to write file: {e}"))?;
    }

    Ok(())
}

fn extract_tar_archive<R: Read>(reader: R, target_dir: &Path) -> Result<(), String> {
    let mut archive = Archive::new(reader);
    let entries = archive
        .entries()
        .map_err(|e| format!("Failed to read TAR entries: {e}"))?;

    for entry in entries {
        let mut entry = entry.map_err(|e| format!("Failed to parse TAR entry: {e}"))?;
        entry
            .unpack_in(target_dir)
            .map_err(|e| format!("Failed to unpack TAR entry: {e}"))?;
    }

    Ok(())
}

fn to_relative_archive_path(base_dir: &Path, absolute_path: &str) -> Result<String, String> {
    let absolute = Path::new(absolute_path);
    let relative = absolute
        .strip_prefix(base_dir)
        .map_err(|_| format!("Detected startup file is outside the temp extract dir: {absolute_path}"))?;

    if relative.as_os_str().is_empty() {
        return Err("Detected startup file path is empty".to_string());
    }

    Ok(relative.to_string_lossy().replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::{
        detect_core_key_from_filename_and_main_class, resolve_direct_jar_launch_target,
        resolve_launch_target, resolve_mode_aware_launch_target, resolve_preferred_jar_path,
        scan_startup_candidates, StartupSourceKind, STARTUP_SCAN_CORE_KEY_OPTIONS,
    };
    use std::collections::HashSet;
    use std::fs;
    use std::io::Write;
    use zip::write::FileOptions;

    fn startup_scan_temp_dirs() -> HashSet<String> {
        fs::read_dir(std::env::temp_dir())
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                if !path.is_dir() {
                    return None;
                }

                let name = path.file_name()?.to_string_lossy().to_string();
                if name.starts_with("sl_server_startup_scan_") {
                    Some(name)
                } else {
                    None
                }
            })
            .collect()
    }

    fn write_manifest_jar(path: &std::path::Path, manifest: &str) {
        let file = fs::File::create(path).expect("jar file should create");
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("META-INF/MANIFEST.MF", FileOptions::<()>::default())
            .expect("manifest entry should start");
        zip.write_all(manifest.as_bytes())
            .expect("manifest should write");
        zip.finish().expect("jar should finish");
    }

    #[test]
    fn scan_startup_candidates_rejects_missing_source_path() {
        let error = scan_startup_candidates(
            "E:/missing/libservercore-startup-scan",
            StartupSourceKind::Folder,
            &[],
        )
        .expect_err("missing path should fail");

        assert!(error.contains("Path does not exist"));
    }

    #[test]
    fn scan_startup_candidates_collects_and_sorts_script_candidates_for_folder() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        fs::write(dir.path().join("run.sh"), "#!/bin/sh\n").expect("shell script should write");
        fs::write(dir.path().join("start.bat"), "@echo off\n").expect("bat script should write");

        let result = scan_startup_candidates(
            dir.path().to_string_lossy().as_ref(),
            StartupSourceKind::Folder,
            &[],
        )
        .expect("folder scan should succeed");

        assert_eq!(result.candidates.len(), 2);
        assert_eq!(result.candidates[0].mode, "sh");
        assert_eq!(result.candidates[0].recommended_rank, 2);
        assert_eq!(result.parsed_core.detected_core_key, None);
        assert_eq!(result.parsed_core.display_hint, "unknown");
    }

    #[test]
    fn scan_startup_candidates_uses_direct_jar_path_for_archive_source() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        let jar_path = dir.path().join("paper-server.jar");
        write_manifest_jar(
            &jar_path,
            "Manifest-Version: 1.0\r\nMain-Class: io.papermc.paperclip.Main\r\n\r\n",
        );

        let result = scan_startup_candidates(
            jar_path.to_string_lossy().as_ref(),
            StartupSourceKind::Archive,
            &[],
        )
        .expect("archive jar scan should succeed");

        assert_eq!(result.candidates.len(), 1);
        assert_eq!(result.candidates[0].mode, "jar");
        assert_eq!(result.candidates[0].label, "server.jar");
        assert_eq!(result.parsed_core.detected_core_key.as_deref(), Some("paper"));
    }

    #[test]
    fn scan_startup_candidates_recognizes_pumpkin_executable_archive_source() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        let exe_path = dir.path().join("pumpkin-X64-Windows.exe");
        fs::write(&exe_path, b"pumpkin").expect("pumpkin executable should write");

        let result = scan_startup_candidates(
            exe_path.to_string_lossy().as_ref(),
            StartupSourceKind::Archive,
            &[],
        )
        .expect("pumpkin archive scan should succeed");

        assert_eq!(result.parsed_core.detected_core_key.as_deref(), Some("pumpkin"));
        assert_eq!(result.candidates[0].mode, "custom");
        assert_eq!(result.candidates[0].label, "Pumpkin");
    }

    #[test]
    fn scan_startup_candidates_keeps_neoforge_type_for_legacy_simpleinstaller_manifest() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        let jar_path = dir
            .path()
            .join("neoforge-26.1.0.0-alpha.1+snapshot-1-installer.jar");
        write_manifest_jar(
            &jar_path,
            "Manifest-Version: 1.0\r\nMain-Class: net.minecraftforge.installer.SimpleInstaller\r\n\r\n",
        );

        let result = scan_startup_candidates(
            jar_path.to_string_lossy().as_ref(),
            StartupSourceKind::Archive,
            &[],
        )
        .expect("archive jar scan should succeed");

        assert_eq!(result.parsed_core.detected_core_key.as_deref(), Some("neoforge"));
        assert_eq!(result.candidates[0].mode, "starter");
        assert_eq!(result.candidates[0].label, "Installer");
    }

    #[test]
    fn scan_startup_candidates_prefers_known_core_over_unknown_helper_jar() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        write_manifest_jar(
            &dir.path().join("a-helper.jar"),
            "Manifest-Version: 1.0\r\nMain-Class: net.minecraft.client.Main\r\n\r\n",
        );
        write_manifest_jar(
            &dir.path().join("paper-server.jar"),
            "Manifest-Version: 1.0\r\nMain-Class: io.papermc.paperclip.Main\r\n\r\n",
        );

        let result = scan_startup_candidates(
            dir.path().to_string_lossy().as_ref(),
            StartupSourceKind::Folder,
            &[],
        )
        .expect("folder scan should succeed");

        assert_eq!(result.parsed_core.detected_core_key.as_deref(), Some("paper"));
        assert!(result
            .parsed_core
            .startup_path
            .as_deref()
            .is_some_and(|path| path.ends_with("paper-server.jar")));
    }

    #[test]
    fn scan_startup_candidates_cleans_temp_extract_dir_when_archive_scan_fails() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        let archive_path = dir.path().join("broken-modpack.zip");
        fs::write(&archive_path, b"not a real zip archive").expect("broken archive should write");

        let before = startup_scan_temp_dirs();
        let error = scan_startup_candidates(
            archive_path.to_string_lossy().as_ref(),
            StartupSourceKind::Archive,
            &[],
        )
        .expect_err("invalid archive should fail");
        let after = startup_scan_temp_dirs();

        assert!(error.contains("Failed to parse ZIP archive"), "unexpected error: {error}");
        assert_eq!(after, before);
    }

    #[test]
    fn detect_core_key_from_filename_and_main_class_normalizes_aliases() {
        assert_eq!(
            detect_core_key_from_filename_and_main_class(
                "Waterfall.jar",
                Some("net.md_5.bungee.Bootstrap"),
            )
            .as_deref(),
            Some("lightfall")
        );
        assert_eq!(
            detect_core_key_from_filename_and_main_class("Leaf.jar", None).as_deref(),
            Some("leaves")
        );
    }

    #[test]
    fn startup_scan_options_expose_canonical_core_keys() {
        assert!(STARTUP_SCAN_CORE_KEY_OPTIONS.contains(&"leaves"));
        assert!(STARTUP_SCAN_CORE_KEY_OPTIONS.contains(&"pufferfish"));
        assert!(STARTUP_SCAN_CORE_KEY_OPTIONS.contains(&"forge"));
        assert!(STARTUP_SCAN_CORE_KEY_OPTIONS.contains(&"fabric"));
        assert!(STARTUP_SCAN_CORE_KEY_OPTIONS.contains(&"vanilla"));
        assert!(STARTUP_SCAN_CORE_KEY_OPTIONS.contains(&"nukkit"));
        assert!(!STARTUP_SCAN_CORE_KEY_OPTIONS.contains(&"leaf"));
        assert!(!STARTUP_SCAN_CORE_KEY_OPTIONS.contains(&"arclight-fabric"));
    }

    #[test]
    fn scan_startup_candidates_extracts_jar_target_from_bat_script() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        let script_path = dir.path().join("start.bat");
        fs::write(&script_path, "@echo off\njava -jar paperclip.jar nogui\n")
            .expect("bat script should write");
        write_manifest_jar(
            &dir.path().join("paperclip.jar"),
            "Manifest-Version: 1.0\r\nMain-Class: io.papermc.paperclip.Main\r\n\r\n",
        );

        let result = scan_startup_candidates(
            dir.path().to_string_lossy().as_ref(),
            StartupSourceKind::Folder,
            &[],
        )
        .expect("folder scan should succeed");

        let script = result
            .candidates
            .iter()
            .find(|candidate| candidate.mode == "bat")
            .expect("bat candidate should exist");
        assert_eq!(script.resolved_target_path.as_deref(), Some("paperclip.jar"));
        assert!(script.detail.contains("paperclip.jar"));
        assert_eq!(result.parsed_core.detected_core_key.as_deref(), Some("paper"));
    }

    #[test]
    fn scan_startup_candidates_extracts_called_script_target_from_shell_script() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        let script_path = dir.path().join("run.sh");
        fs::write(&script_path, "#!/bin/sh\n./launch.sh nogui\n")
            .expect("shell script should write");

        let result = scan_startup_candidates(
            dir.path().to_string_lossy().as_ref(),
            StartupSourceKind::Folder,
            &[],
        )
        .expect("folder scan should succeed");

        let script = result
            .candidates
            .iter()
            .find(|candidate| candidate.mode == "sh")
            .expect("sh candidate should exist");
        assert_eq!(script.resolved_target_path.as_deref(), Some("./launch.sh"));
        assert!(script.detail.contains("launch.sh"));
    }

    #[test]
    fn scan_startup_candidates_extracts_executable_target_from_ps1_script() {
        let dir = tempfile::tempdir().expect("temp dir should exist");
        let script_path = dir.path().join("start.ps1");
        fs::write(&script_path, "& './pumpkin.exe' nogui\n")
            .expect("powershell script should write");
        fs::write(dir.path().join("pumpkin.exe"), b"pumpkin")
            .expect("pumpkin exe should write");

        let result = scan_startup_candidates(
            dir.path().to_string_lossy().as_ref(),
            StartupSourceKind::Folder,
            &[],
        )
        .expect("folder scan should succeed");

        let script = result
            .candidates
            .iter()
            .find(|candidate| candidate.mode == "ps1")
            .expect("ps1 candidate should exist");
        assert_eq!(script.resolved_target_path.as_deref(), Some("./pumpkin.exe"));
    }

    #[test]
    fn resolve_preferred_jar_path_prefers_configured_jar_for_jar_mode() {
        let resolved = resolve_preferred_jar_path(
            "jar",
            Some("E:/servers/paper/server.jar"),
            std::path::Path::new("E:/servers/paper"),
        )
        .expect("preferred jar should resolve");

        assert_eq!(resolved.as_deref(), Some("E:/servers/paper/server.jar"));
    }

    #[test]
    fn resolve_preferred_jar_path_ignores_script_mode_even_when_configured_path_exists() {
        let resolved = resolve_preferred_jar_path(
            "sh",
            Some("E:/servers/paper/server.jar"),
            std::path::Path::new("E:/servers/paper"),
        )
        .expect("script mode should skip direct jar preference");

        assert_eq!(resolved, None);
    }

    #[test]
    fn resolve_direct_jar_launch_target_uses_filename_for_root_jar() {
        let target = resolve_direct_jar_launch_target(
            std::path::Path::new("E:/servers/fabric-1.20.1"),
            "E:/servers/fabric-1.20.1/server.jar",
        );

        assert_eq!(target, "server.jar");
    }

    #[test]
    fn resolve_direct_jar_launch_target_keeps_nested_or_external_path() {
        let nested = resolve_direct_jar_launch_target(
            std::path::Path::new("E:/servers/fabric-1.20.1"),
            "E:/servers/fabric-1.20.1/libraries/server.jar",
        );
        let external = resolve_direct_jar_launch_target(
            std::path::Path::new("E:/servers/fabric-1.20.1"),
            "E:/srv/shared/server.jar",
        );

        assert_eq!(nested.replace('\\', "/"), "E:/servers/fabric-1.20.1/libraries/server.jar");
        assert_eq!(external.replace('\\', "/"), "E:/srv/shared/server.jar");
    }

    #[test]
    fn resolve_launch_target_matches_mode_shape() {
        assert_eq!(
            resolve_launch_target(
                "jar",
                None,
                Some("E:/servers/paper/server.jar"),
                Some("java -jar custom.jar nogui"),
                "start.sh",
            ),
            "E:/servers/paper/server.jar"
        );
        assert_eq!(
            resolve_launch_target(
                "custom",
                None,
                Some("E:/servers/paper/server.jar"),
                Some("java -jar custom.jar nogui"),
                "start.sh",
            ),
            "java -jar custom.jar nogui"
        );
        assert_eq!(
            resolve_launch_target(
                "sh",
                Some("E:/servers/paper/found.jar"),
                Some("E:/servers/paper/server.jar"),
                Some("java -jar custom.jar nogui"),
                "start.sh",
            ),
            "E:/servers/paper/found.jar"
        );
    }

    #[test]
    fn resolve_mode_aware_launch_target_uses_filename_for_script_mode() {
        let resolved = resolve_mode_aware_launch_target(
            "sh",
            Some("E:/servers/paper/start.sh"),
            None,
            std::path::Path::new("E:/servers/paper"),
        )
        .expect("mode aware resolution should succeed");

        assert_eq!(resolved.startup_mode, "sh");
        assert_eq!(resolved.startup_filename, "start.sh");
        assert_eq!(resolved.preferred_jar_path, None);
        assert_eq!(resolved.launch_target, "start.sh");
    }
}
