//! Runtime server information probes, metrics, and log parsing.
//!
//! ```rust
//! use sl_server_info::log::{parse_log_line, DomainEvent, LogLineInput, LogStream};
//!
//! let parsed = parse_log_line(
//!     Some("paper"),
//!     LogLineInput {
//!         raw: "[12:00:00] [Server thread/INFO]: Alex joined the game".to_string(),
//!         stream: LogStream::Stdout,
//!     },
//! );
//!
//! assert_eq!(
//!     parsed.event,
//!     Some(DomainEvent::PlayerJoin {
//!         player: "Alex".to_string(),
//!     })
//! );
//! ```

#![forbid(unsafe_code)]

pub mod log;
pub mod metrics;
pub mod model;
pub mod rcon;
pub mod snapshot;

pub use log::{
    parse_log_line, DomainEvent, LogLevel, LogLineInput, LogStream, ParsedLogLine,
    StructuredLogLine,
};
pub use metrics::{
    probe_performance, probe_performance_detailed, MetricProviderKind, PerformanceProbeOptions,
    PerformanceProbeResult, PerformanceScope, PerformanceScopeKind, PerformanceSnapshot,
};
pub use model::{
    InfoFact, InfoValue, OnlinePlayersSnapshot, ProbeWarning, ServerInfoError, ServerInfoSnapshot,
};
pub use rcon::{RconEndpoint, RconProbeOptions};
pub use snapshot::probe_snapshot;
