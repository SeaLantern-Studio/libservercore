use std::path::{Path, PathBuf};

use crate::error::StateFileError;
use serde::{Deserialize, Serialize};

const WHITELIST_FILE: &str = "whitelist.json";
const BANNED_PLAYERS_FILE: &str = "banned-players.json";
const OPS_FILE: &str = "ops.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhitelistEntry {
    pub uuid: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BannedPlayerEntry {
    pub uuid: String,
    pub name: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub expires: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpEntry {
    pub uuid: String,
    pub name: String,
    #[serde(default)]
    pub level: u32,
    #[serde(default, alias = "bypassesPlayerLimit", alias = "bypass_player_limit")]
    pub bypasses_player_limit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateFileCatalog {
    pub server_root: PathBuf,
    pub whitelist_path: PathBuf,
    pub banned_players_path: PathBuf,
    pub ops_path: PathBuf,
}

pub fn discover_state_files(server_root: impl AsRef<Path>) -> StateFileCatalog {
    let server_root = server_root.as_ref().to_path_buf();
    StateFileCatalog {
        whitelist_path: server_root.join(WHITELIST_FILE),
        banned_players_path: server_root.join(BANNED_PLAYERS_FILE),
        ops_path: server_root.join(OPS_FILE),
        server_root,
    }
}

pub fn read_whitelist(path: impl AsRef<Path>) -> Result<Vec<WhitelistEntry>, StateFileError> {
    read_json_list(path)
}

pub fn read_banned_players(
    path: impl AsRef<Path>,
) -> Result<Vec<BannedPlayerEntry>, StateFileError> {
    read_json_list(path)
}

pub fn read_ops(path: impl AsRef<Path>) -> Result<Vec<OpEntry>, StateFileError> {
    read_json_list(path)
}

fn read_json_list<T>(path: impl AsRef<Path>) -> Result<Vec<T>, StateFileError>
where
    T: serde::de::DeserializeOwned,
{
    let path = path.as_ref();
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(path).map_err(|error| {
        StateFileError::Io(format!("failed to read {}: {}", path.display(), error))
    })?;

    let trimmed = content.trim();
    if trimmed.is_empty() || trimmed == "[]" {
        return Ok(Vec::new());
    }

    serde_json::from_str(trimmed).map_err(|error| {
        StateFileError::ParseFailed(format!("failed to parse {}: {}", path.display(), error))
    })
}

#[cfg(test)]
mod tests {
    use crate::{
        discover_state_files, read_banned_players, read_ops, read_whitelist, BannedPlayerEntry,
        OpEntry, WhitelistEntry,
    };
    use std::path::{Path, PathBuf};
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
            let path = std::env::temp_dir().join(format!("sl-libscv-state-{}-{}", prefix, unique));
            std::fs::create_dir_all(&path).expect("test dir should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn discover_state_files_uses_server_root_conventions() {
        let dir = TestDir::new("discover");
        let catalog = discover_state_files(dir.path());

        assert_eq!(catalog.server_root, dir.path());
        assert_eq!(catalog.whitelist_path, dir.path().join("whitelist.json"));
        assert_eq!(
            catalog.banned_players_path,
            dir.path().join("banned-players.json")
        );
        assert_eq!(catalog.ops_path, dir.path().join("ops.json"));
    }

    #[test]
    fn missing_state_files_return_empty_lists() {
        let dir = TestDir::new("missing");

        let whitelist = read_whitelist(dir.path().join("whitelist.json")).unwrap();
        let banned = read_banned_players(dir.path().join("banned-players.json")).unwrap();
        let ops = read_ops(dir.path().join("ops.json")).unwrap();

        assert_eq!(whitelist, Vec::<WhitelistEntry>::new());
        assert_eq!(banned, Vec::<BannedPlayerEntry>::new());
        assert_eq!(ops, Vec::<OpEntry>::new());
    }

    #[test]
    fn empty_or_empty_array_state_files_return_empty_lists() {
        let dir = TestDir::new("empty");
        let whitelist_path = dir.path().join("whitelist.json");
        let banned_path = dir.path().join("banned-players.json");
        let ops_path = dir.path().join("ops.json");
        std::fs::write(&whitelist_path, "\n  \n").unwrap();
        std::fs::write(&banned_path, "[]").unwrap();
        std::fs::write(&ops_path, "  []  ").unwrap();

        assert!(read_whitelist(&whitelist_path).unwrap().is_empty());
        assert!(read_banned_players(&banned_path).unwrap().is_empty());
        assert!(read_ops(&ops_path).unwrap().is_empty());
    }

    #[test]
    fn ops_reader_accepts_bypass_aliases() {
        let dir = TestDir::new("ops-alias");
        let ops_path = dir.path().join("ops.json");
        std::fs::write(
            &ops_path,
            r#"[
  {"uuid":"1","name":"Alex","level":4,"bypassesPlayerLimit":true},
  {"uuid":"2","name":"Steve","level":2,"bypass_player_limit":false}
]"#,
        )
        .unwrap();

        let entries = read_ops(&ops_path).unwrap();

        assert_eq!(entries.len(), 2);
        assert!(entries[0].bypasses_player_limit);
        assert!(!entries[1].bypasses_player_limit);
    }

    #[test]
    fn invalid_json_returns_parse_error() {
        let dir = TestDir::new("invalid");
        let whitelist_path = dir.path().join("whitelist.json");
        std::fs::write(&whitelist_path, "[").unwrap();

        let error = read_whitelist(&whitelist_path).unwrap_err();
        let rendered = format!("{error:?}");
        assert!(rendered.contains("ParseFailed"));
    }
}
