//! File-centered server configuration discovery and IO support.
//!
//! ```rust
//! use sl_libscv::state::discover_state_files;
//!
//! let catalog = discover_state_files("E:/servers/paper");
//! assert!(catalog.whitelist_path.ends_with("whitelist.json"));
//! assert!(catalog.banned_players_path.ends_with("banned-players.json"));
//! assert!(catalog.ops_path.ends_with("ops.json"));
//! ```

#![forbid(unsafe_code)]

pub mod discovery;
pub mod document;
pub mod error;
pub mod formats;
pub mod state;

pub use discovery::{
    discover_config_candidates, discover_config_entries, ConfigCandidateCatalog,
    ConfigDiscoveryInput, ConfigEntry, ConfigEntryConfidence, ConfigEntrySource, ConfigMatchReason,
    ConfigOwnerScope, ConfigSurfaceCatalog, GenericConfigDiscoveryInput,
};
pub use document::{
    read_config_document, read_config_file, write_config_document, write_config_file,
    ConfigDocument, JsonDocument, PropertiesDocument, TextDocument, TomlDocument, YamlDocument,
};
pub use error::{ConfigDiscoveryError, ConfigIoError, StateFileError};
pub use formats::ConfigFormat;
pub use state::{
    discover_state_files, read_banned_players, read_ops, read_whitelist, BannedPlayerEntry,
    OpEntry, StateFileCatalog, WhitelistEntry,
};
