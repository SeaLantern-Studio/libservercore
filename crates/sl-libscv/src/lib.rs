//! File-centered server configuration discovery and IO support.

#![forbid(unsafe_code)]

mod discovery;
mod document;
mod error;
mod formats;

pub use discovery::{
    discover_config_candidates, discover_config_entries, ConfigCandidateCatalog,
    ConfigDiscoveryInput, ConfigEntry, ConfigEntryConfidence, ConfigEntrySource,
    ConfigMatchReason, ConfigOwnerScope, ConfigSurfaceCatalog, GenericConfigDiscoveryInput,
};
pub use document::{
    read_config_document, read_config_file, write_config_document, write_config_file,
    ConfigDocument, JsonDocument, PropertiesDocument, TextDocument, TomlDocument, YamlDocument,
};
pub use error::{ConfigDiscoveryError, ConfigIoError};
pub use formats::ConfigFormat;
