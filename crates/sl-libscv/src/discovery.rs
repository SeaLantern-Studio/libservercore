use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::error::ConfigDiscoveryError;
use crate::formats::ConfigFormat;
use server_core_taxonomy::normalize_core_key;
use server_flavor_core::{
    resolve_server_flavor_profile, ConfigSurfaceKind, ConfigSurfaceOwner, FlavorResolutionInput,
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

const DEFAULT_GENERIC_SCAN_DIRS: &[&str] = &[
    "plugins",
    "mods",
    "config",
    "configs",
    "defaultconfig",
    "defaultconfigs",
    "world/serverconfig",
];

const DEFAULT_EXCLUDED_DIRS: &[&str] = &[
    ".git",
    ".idea",
    ".vscode",
    "cache",
    "caches",
    "crash-reports",
    "libraries",
    "logs",
    "versions",
    "userdata",
];

const DEFAULT_ALLOWED_FORMATS: &[ConfigFormat] = &[
    ConfigFormat::Yaml,
    ConfigFormat::Toml,
    ConfigFormat::Json,
    ConfigFormat::Properties,
    ConfigFormat::Text,
];

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigOwnerScope {
    ServerCore,
    Plugin,
    FallbackDirectory,
    Generic,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigEntrySource {
    ExplicitSurface,
    PluginDiscovery,
    FallbackDirectoryDiscovery,
    GenericRootScan,
    GenericRecursiveScan,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfigEntryConfidence {
    Explicit,
    Likely,
    Heuristic,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigMatchReason {
    ExplicitSurfaceKey(String),
    RootPropertiesFallback,
    PluginDirectoryPattern,
    FallbackDirectoryPattern,
    GenericScanRoot(String),
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigEntry {
    pub key: Option<String>,
    pub owner: ConfigOwnerScope,
    pub format: ConfigFormat,
    pub relative_path: String,
    pub absolute_path: PathBuf,
    pub source: ConfigEntrySource,
    pub confidence: ConfigEntryConfidence,
    pub reason: ConfigMatchReason,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigSurfaceCatalog {
    pub core_key: String,
    pub server_root: PathBuf,
    pub entries: Vec<ConfigEntry>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigCandidateCatalog {
    pub server_root: PathBuf,
    pub entries: Vec<ConfigEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDiscoveryInput {
    pub core_key: String,
    pub server_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericConfigDiscoveryInput {
    pub server_root: PathBuf,
    pub include_root_files: bool,
    pub scan_roots: Vec<String>,
    pub excluded_directories: Vec<String>,
    pub allowed_formats: Vec<ConfigFormat>,
    pub max_depth: Option<usize>,
}

impl ConfigDiscoveryInput {
    pub fn new(core_key: impl Into<String>, server_root: PathBuf) -> Self {
        Self {
            core_key: core_key.into(),
            server_root,
        }
    }
}

impl GenericConfigDiscoveryInput {
    pub fn new(server_root: PathBuf) -> Self {
        Self {
            server_root,
            include_root_files: true,
            scan_roots: DEFAULT_GENERIC_SCAN_DIRS
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
            excluded_directories: DEFAULT_EXCLUDED_DIRS
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
            allowed_formats: DEFAULT_ALLOWED_FORMATS.to_vec(),
            max_depth: Some(8),
        }
    }
}

pub fn discover_config_entries(
    input: &ConfigDiscoveryInput,
) -> Result<ConfigSurfaceCatalog, ConfigDiscoveryError> {
    let normalized_core_key = normalize_core_key(&input.core_key)
        .ok_or_else(|| ConfigDiscoveryError::UnknownCoreKey(input.core_key.clone()))?;
    validate_server_root(&input.server_root)?;

    let profile = resolve_server_flavor_profile(&FlavorResolutionInput {
        core_key: Some(normalized_core_key),
        runtime_kind: None,
        startup_mode: None,
        wrapper_kind: None,
        has_pumpkin_config: false,
    });

    let mut entries = Vec::new();
    let mut seen_paths = BTreeSet::new();

    for surface in &profile.config_surfaces {
        match surface.kind {
            ConfigSurfaceKind::CanonicalFile => {
                let absolute_path = input.server_root.join(surface.relative_path);
                if absolute_path.is_file() {
                    push_entry(
                        &mut entries,
                        &mut seen_paths,
                        ConfigEntry {
                            key: Some(surface.key.to_string()),
                            owner: map_owner(surface.owner),
                            format: surface
                                .format
                                .map(map_format)
                                .expect("canonical file should have format"),
                            relative_path: normalize_relative(&input.server_root, &absolute_path),
                            absolute_path,
                            source: ConfigEntrySource::ExplicitSurface,
                            confidence: ConfigEntryConfidence::Explicit,
                            reason: ConfigMatchReason::ExplicitSurfaceKey(surface.key.to_string()),
                        },
                    );
                }
            }
            ConfigSurfaceKind::PluginDirectory => {
                let plugin_root = input.server_root.join(surface.relative_path);
                if plugin_root.is_dir() {
                    discover_recursive_files(
                        &input.server_root,
                        &plugin_root,
                        ConfigOwnerScope::Plugin,
                        ConfigEntrySource::PluginDiscovery,
                        ConfigEntryConfidence::Likely,
                        ConfigMatchReason::PluginDirectoryPattern,
                        &mut entries,
                        &mut seen_paths,
                        None,
                        DEFAULT_ALLOWED_FORMATS,
                        &default_excluded_directories(),
                        0,
                    )?;
                }
            }
            ConfigSurfaceKind::FallbackDirectory => {
                let config_root = input.server_root.join(surface.relative_path);
                if config_root.is_dir() {
                    discover_recursive_files(
                        &input.server_root,
                        &config_root,
                        ConfigOwnerScope::FallbackDirectory,
                        ConfigEntrySource::FallbackDirectoryDiscovery,
                        ConfigEntryConfidence::Likely,
                        ConfigMatchReason::FallbackDirectoryPattern,
                        &mut entries,
                        &mut seen_paths,
                        None,
                        DEFAULT_ALLOWED_FORMATS,
                        &default_excluded_directories(),
                        0,
                    )?;
                }
            }
        }
    }

    let root_entries = std::fs::read_dir(&input.server_root)
        .map_err(|e| ConfigDiscoveryError::Io(e.to_string()))?;
    for entry in root_entries {
        let entry = entry.map_err(|e| ConfigDiscoveryError::Io(e.to_string()))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(is_readme_like)
        {
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) == Some("properties") {
            push_entry(
                &mut entries,
                &mut seen_paths,
                ConfigEntry {
                    key: None,
                    owner: ConfigOwnerScope::ServerCore,
                    format: ConfigFormat::Properties,
                    relative_path: normalize_relative(&input.server_root, &path),
                    absolute_path: path,
                    source: ConfigEntrySource::FallbackDirectoryDiscovery,
                    confidence: ConfigEntryConfidence::Likely,
                    reason: ConfigMatchReason::RootPropertiesFallback,
                },
            );
        }
    }

    entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

    Ok(ConfigSurfaceCatalog {
        core_key: normalized_core_key.to_string(),
        server_root: input.server_root.clone(),
        entries,
    })
}

pub fn discover_config_candidates(
    input: &GenericConfigDiscoveryInput,
) -> Result<ConfigCandidateCatalog, ConfigDiscoveryError> {
    validate_server_root(&input.server_root)?;

    let allowed_formats = normalize_allowed_formats(&input.allowed_formats);
    let scan_roots = normalize_scan_roots(&input.scan_roots);
    let excluded_directories = normalize_excluded_directories(&input.excluded_directories);
    let mut entries = Vec::new();
    let mut seen_paths = BTreeSet::new();

    if input.include_root_files {
        let root_entries = std::fs::read_dir(&input.server_root)
            .map_err(|e| ConfigDiscoveryError::Io(e.to_string()))?;
        for entry in root_entries {
            let entry = entry.map_err(|e| ConfigDiscoveryError::Io(e.to_string()))?;
            let path = entry.path();
            if !path.is_file() || should_skip_file(&path, &allowed_formats) {
                continue;
            }

            push_entry(
                &mut entries,
                &mut seen_paths,
                ConfigEntry {
                    key: None,
                    owner: ConfigOwnerScope::Generic,
                    format: infer_format(&path).expect("file should have supported format"),
                    relative_path: normalize_relative(&input.server_root, &path),
                    absolute_path: path,
                    source: ConfigEntrySource::GenericRootScan,
                    confidence: ConfigEntryConfidence::Heuristic,
                    reason: ConfigMatchReason::GenericScanRoot(".".to_string()),
                },
            );
        }
    }

    for scan_root in &scan_roots {
        let root = input.server_root.join(scan_root);
        if !root.is_dir() {
            continue;
        }

        discover_recursive_files(
            &input.server_root,
            &root,
            classify_generic_owner(scan_root),
            ConfigEntrySource::GenericRecursiveScan,
            ConfigEntryConfidence::Heuristic,
            ConfigMatchReason::GenericScanRoot(scan_root.clone()),
            &mut entries,
            &mut seen_paths,
            input.max_depth,
            &allowed_formats,
            &excluded_directories,
            0,
        )?;
    }

    entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

    Ok(ConfigCandidateCatalog {
        server_root: input.server_root.clone(),
        entries,
    })
}

fn validate_server_root(server_root: &Path) -> Result<(), ConfigDiscoveryError> {
    if !server_root.exists() || !server_root.is_dir() {
        return Err(ConfigDiscoveryError::InvalidServerRoot(
            server_root.display().to_string(),
        ));
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn discover_recursive_files(
    server_root: &Path,
    root: &Path,
    owner: ConfigOwnerScope,
    source: ConfigEntrySource,
    confidence: ConfigEntryConfidence,
    reason: ConfigMatchReason,
    entries: &mut Vec<ConfigEntry>,
    seen_paths: &mut BTreeSet<String>,
    max_depth: Option<usize>,
    allowed_formats: &[ConfigFormat],
    excluded_directories: &BTreeSet<String>,
    current_depth: usize,
) -> Result<(), ConfigDiscoveryError> {
    if max_depth.is_some_and(|max| current_depth > max) {
        return Ok(());
    }

    for dir_entry in std::fs::read_dir(root).map_err(|e| ConfigDiscoveryError::Io(e.to_string()))? {
        let dir_entry = dir_entry.map_err(|e| ConfigDiscoveryError::Io(e.to_string()))?;
        let path = dir_entry.path();

        if path.is_dir() {
            if should_skip_directory(&path, excluded_directories) {
                continue;
            }
            discover_recursive_files(
                server_root,
                &path,
                owner,
                source,
                confidence,
                reason.clone(),
                entries,
                seen_paths,
                max_depth,
                allowed_formats,
                excluded_directories,
                current_depth + 1,
            )?;
            continue;
        }

        if !path.is_file() || should_skip_file(&path, allowed_formats) {
            continue;
        }

        let format = infer_format(&path).expect("file should have supported format");
        push_entry(
            entries,
            seen_paths,
            ConfigEntry {
                key: None,
                owner,
                format,
                relative_path: normalize_relative(server_root, &path),
                absolute_path: path,
                source,
                confidence,
                reason: reason.clone(),
            },
        );
    }

    Ok(())
}

fn normalize_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn push_entry(entries: &mut Vec<ConfigEntry>, seen_paths: &mut BTreeSet<String>, entry: ConfigEntry) {
    if seen_paths.insert(entry.relative_path.clone()) {
        entries.push(entry);
    }
}

fn map_owner(owner: ConfigSurfaceOwner) -> ConfigOwnerScope {
    match owner {
        ConfigSurfaceOwner::ServerCore => ConfigOwnerScope::ServerCore,
        ConfigSurfaceOwner::Plugin => ConfigOwnerScope::Plugin,
        ConfigSurfaceOwner::FallbackDirectory => ConfigOwnerScope::FallbackDirectory,
    }
}

fn map_format(format: server_flavor_core::ConfigFormat) -> ConfigFormat {
    match format {
        server_flavor_core::ConfigFormat::Yaml => ConfigFormat::Yaml,
        server_flavor_core::ConfigFormat::Toml => ConfigFormat::Toml,
        server_flavor_core::ConfigFormat::Json => ConfigFormat::Json,
        server_flavor_core::ConfigFormat::Properties => ConfigFormat::Properties,
        server_flavor_core::ConfigFormat::Text => ConfigFormat::Text,
    }
}

fn infer_format(path: &Path) -> Option<ConfigFormat> {
    let extension = path.extension().and_then(|value| value.to_str())?;
    ConfigFormat::from_extension(extension)
}

fn should_skip_directory(path: &Path, excluded_directories: &BTreeSet<String>) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(|value| excluded_directories.contains(&value.trim().to_ascii_lowercase()))
        .unwrap_or(false)
}

fn should_skip_file(path: &Path, allowed_formats: &[ConfigFormat]) -> bool {
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return true;
    };
    if is_readme_like(file_name) || is_backup_like(file_name) {
        return true;
    }

    let Some(format) = infer_format(path) else {
        return true;
    };

    !allowed_formats.contains(&format)
}

fn classify_generic_owner(scan_root: &str) -> ConfigOwnerScope {
    let normalized = scan_root.trim().replace('\\', "/").to_ascii_lowercase();
    if normalized == "plugins" {
        ConfigOwnerScope::Plugin
    } else if normalized.contains("config") {
        ConfigOwnerScope::FallbackDirectory
    } else {
        ConfigOwnerScope::Generic
    }
}

fn normalize_allowed_formats(formats: &[ConfigFormat]) -> Vec<ConfigFormat> {
    if formats.is_empty() {
        DEFAULT_ALLOWED_FORMATS.to_vec()
    } else {
        let mut deduped = Vec::new();
        for format in formats {
            if !deduped.contains(format) {
                deduped.push(*format);
            }
        }
        deduped
    }
}

fn normalize_scan_roots(values: &[String]) -> Vec<String> {
    if values.is_empty() {
        DEFAULT_GENERIC_SCAN_DIRS
            .iter()
            .map(|value| (*value).to_string())
            .collect()
    } else {
        values
            .iter()
            .map(|value| value.trim().replace('\\', "/"))
            .filter(|value| !value.is_empty())
            .collect()
    }
}

fn normalize_excluded_directories(values: &[String]) -> BTreeSet<String> {
    if values.is_empty() {
        default_excluded_directories()
    } else {
        normalize_name_set(values)
    }
}

fn normalize_name_set(values: &[String]) -> BTreeSet<String> {
    values
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect()
}

fn default_excluded_directories() -> BTreeSet<String> {
    normalize_name_set(
        &DEFAULT_EXCLUDED_DIRS
            .iter()
            .map(|value| (*value).to_string())
            .collect::<Vec<_>>(),
    )
}

fn is_readme_like(file_name: &str) -> bool {
    let lowered = file_name.trim().to_ascii_lowercase();
    lowered == "readme" || lowered.starts_with("readme.")
}

fn is_backup_like(file_name: &str) -> bool {
    let lowered = file_name.trim().to_ascii_lowercase();
    lowered.ends_with(".bak") || lowered.ends_with(".old") || lowered.ends_with(".orig")
}

#[cfg(test)]
mod tests {
    use super::{
        discover_config_candidates, discover_config_entries, ConfigDiscoveryInput,
        ConfigEntryConfidence, ConfigEntrySource, ConfigMatchReason, ConfigOwnerScope,
        GenericConfigDiscoveryInput,
    };
    use crate::formats::ConfigFormat;
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
            let path = std::env::temp_dir().join(format!("sl-libscv-{}-{}", prefix, unique));
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
    fn discovers_bukkit_like_core_files_and_plugin_configs() {
        let dir = TestDir::new("paper-discovery");
        std::fs::write(dir.path().join("server.properties"), "motd=hi\n").unwrap();
        std::fs::write(dir.path().join("bukkit.yml"), "settings: {}\n").unwrap();
        std::fs::write(dir.path().join("spigot.yml"), "settings: {}\n").unwrap();
        std::fs::create_dir_all(dir.path().join("config")).unwrap();
        std::fs::write(
            dir.path().join("config").join("paper-global.yml"),
            "chunk-loading: {}\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("plugins").join("Essentials")).unwrap();
        std::fs::write(
            dir.path().join("plugins").join("Essentials").join("config.yml"),
            "spawn: world\n",
        )
        .unwrap();

        let catalog = discover_config_entries(&ConfigDiscoveryInput::new(
            "paper",
            dir.path().to_path_buf(),
        ))
        .expect("catalog should discover");

        assert!(catalog
            .entries
            .iter()
            .any(|entry| entry.relative_path == "server.properties"));
        assert!(catalog
            .entries
            .iter()
            .any(|entry| entry.relative_path == "bukkit.yml"));
        assert!(catalog
            .entries
            .iter()
            .any(|entry| entry.relative_path == "spigot.yml"));
        assert!(catalog
            .entries
            .iter()
            .any(|entry| entry.relative_path == "config/paper-global.yml"));
        assert!(catalog.entries.iter().any(|entry| {
            entry.relative_path == "plugins/Essentials/config.yml"
                && entry.owner == ConfigOwnerScope::Plugin
                && entry.source == ConfigEntrySource::PluginDiscovery
                && entry.confidence == ConfigEntryConfidence::Likely
                && entry.reason == ConfigMatchReason::PluginDirectoryPattern
        }));
    }

    #[test]
    fn excludes_readme_like_files_from_recursive_discovery() {
        let dir = TestDir::new("readme-skip");
        std::fs::create_dir_all(dir.path().join("plugins").join("Demo")).unwrap();
        std::fs::write(dir.path().join("plugins").join("Demo").join("README.txt"), "docs")
            .unwrap();
        std::fs::write(dir.path().join("plugins").join("Demo").join("notes.txt"), "keep")
            .unwrap();

        let catalog = discover_config_entries(&ConfigDiscoveryInput::new(
            "paper",
            dir.path().to_path_buf(),
        ))
        .expect("catalog should discover");

        assert!(catalog
            .entries
            .iter()
            .any(|entry| entry.relative_path == "plugins/Demo/notes.txt"));
        assert!(!catalog
            .entries
            .iter()
            .any(|entry| entry.relative_path == "plugins/Demo/README.txt"));
    }

    #[test]
    fn generic_candidate_scan_finds_non_bukkit_config_like_files() {
        let dir = TestDir::new("generic-candidates");
        std::fs::create_dir_all(dir.path().join("mods").join("demo")).unwrap();
        std::fs::create_dir_all(dir.path().join("world").join("serverconfig")).unwrap();
        std::fs::create_dir_all(dir.path().join("logs")).unwrap();
        std::fs::write(dir.path().join("eula.txt"), "eula=true\n").unwrap();
        std::fs::write(dir.path().join("mods").join("demo").join("demo-server.toml"), "a=1\n")
            .unwrap();
        std::fs::write(
            dir.path().join("world").join("serverconfig").join("forge-server.toml"),
            "b=2\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("logs").join("latest.json"), "{}")
            .unwrap();

        let catalog = discover_config_candidates(&GenericConfigDiscoveryInput::new(
            dir.path().to_path_buf(),
        ))
        .expect("generic candidates should discover");

        assert!(catalog.entries.iter().any(|entry| {
            entry.relative_path == "eula.txt"
                && entry.source == ConfigEntrySource::GenericRootScan
                && entry.owner == ConfigOwnerScope::Generic
                && entry.format == ConfigFormat::Text
        }));
        assert!(catalog.entries.iter().any(|entry| {
            entry.relative_path == "mods/demo/demo-server.toml"
                && entry.source == ConfigEntrySource::GenericRecursiveScan
                && entry.owner == ConfigOwnerScope::Generic
                && entry.reason == ConfigMatchReason::GenericScanRoot("mods".to_string())
        }));
        assert!(catalog.entries.iter().any(|entry| {
            entry.relative_path == "world/serverconfig/forge-server.toml"
                && entry.owner == ConfigOwnerScope::FallbackDirectory
        }));
        assert!(!catalog
            .entries
            .iter()
            .any(|entry| entry.relative_path == "logs/latest.json"));
    }

    #[test]
    fn generic_candidate_scan_skips_backup_files_and_honors_depth_limit() {
        let dir = TestDir::new("generic-filters");
        std::fs::create_dir_all(dir.path().join("config").join("deep").join("nested")).unwrap();
        std::fs::write(dir.path().join("config").join("keep.yml"), "ok: true\n").unwrap();
        std::fs::write(dir.path().join("config").join("keep.yml.bak"), "skip\n").unwrap();
        std::fs::write(
            dir.path()
                .join("config")
                .join("deep")
                .join("nested")
                .join("too-deep.toml"),
            "x=1\n",
        )
        .unwrap();

        let mut input = GenericConfigDiscoveryInput::new(dir.path().to_path_buf());
        input.scan_roots = vec!["config".to_string()];
        input.max_depth = Some(1);

        let catalog = discover_config_candidates(&input).expect("generic candidates should discover");

        assert!(catalog
            .entries
            .iter()
            .any(|entry| entry.relative_path == "config/keep.yml"));
        assert!(!catalog
            .entries
            .iter()
            .any(|entry| entry.relative_path == "config/keep.yml.bak"));
        assert!(!catalog
            .entries
            .iter()
            .any(|entry| entry.relative_path == "config/deep/nested/too-deep.toml"));
    }
}
