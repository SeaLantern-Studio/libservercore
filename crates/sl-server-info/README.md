# sl-server-info

`sl-server-info` provides runtime-oriented inspection helpers for Minecraft server hosts.

It is the internal/runtime-facing counterpart to `sl-libscv`:

- `sl-libscv` focuses on file and config surfaces
- `sl-server-info` focuses on live RCON probing, performance metrics, and log semantics

Current v1 direction:

- RCON reachability and lightweight snapshot probing
- optional performance probes backed by provider-specific commands such as `spark health`, `spark tps`, `spark healthreport`, `forge tps`, `tps`, and `mspt`
- stateless log-line parsing into structured lines plus optional domain events

The crate is intentionally synchronous at the public API boundary. It does not manage polling loops, background threads, log storage, or event buses.

## Snapshot Probe

Use `probe_snapshot()` for portable runtime facts such as player count, max player limit, player list, and a small set of Bukkit-like extra facts.

```rust
use sl_server_info::{probe_snapshot, RconEndpoint, RconProbeOptions};

let endpoint = RconEndpoint {
    host: "127.0.0.1".to_string(),
    port: 25575,
    password: "secret".to_string(),
};

let options = RconProbeOptions {
    core_key: Some("paper".to_string()),
    connect_timeout_ms: 5_000,
    read_timeout_ms: 5_000,
};

let _snapshot = probe_snapshot(&endpoint, &options);
```

## Performance Probe

Use `probe_performance()` when only the first successful metrics snapshot matters, or `probe_performance_detailed()` when the caller also needs warnings from failed providers.

```rust
use sl_server_info::{
    probe_performance_detailed, MetricProviderKind, PerformanceProbeOptions, RconEndpoint,
};

let endpoint = RconEndpoint {
    host: "127.0.0.1".to_string(),
    port: 25575,
    password: "secret".to_string(),
};

let options = PerformanceProbeOptions {
    core_key: Some("forge".to_string()),
    connect_timeout_ms: 5_000,
    read_timeout_ms: 5_000,
    providers: vec![
        MetricProviderKind::SparkHealth,
        MetricProviderKind::SparkTps,
        MetricProviderKind::SparkHealthReport,
        MetricProviderKind::ForgeTps,
        MetricProviderKind::PaperTps,
        MetricProviderKind::PaperMspt,
    ],
};

let _result = probe_performance_detailed(&endpoint, &options);
```

The built-in provider order now prefers current spark commands before older aliases:

- `spark health`
- `spark tps`
- `spark healthreport`
- `forge tps`
- `tps`
- `mspt`

The Paper-compatible providers are intentionally later in the fallback chain:

- `tps` provides a stable TPS-only summary for the last 1m, 5m, and 15m windows
- `mspt` provides average/min/max MSPT for the last 5s, 10s, and 1m windows
- both are useful when spark or forge-specific commands are unavailable but the host still exposes the built-in Bukkit/Paper command set

## Log Parsing

Use `parse_log_line()` to convert raw console output into a structured line and an optional domain event.

```rust
use sl_server_info::{parse_log_line, DomainEvent, LogLineInput, LogStream};

let parsed = parse_log_line(
    Some("paper"),
    LogLineInput {
        raw: "[12:00:00] [Server thread/INFO]: Alex joined the game".to_string(),
        stream: LogStream::Stdout,
    },
);

assert_eq!(
    parsed.event,
    Some(DomainEvent::PlayerJoin {
        player: "Alex".to_string(),
    })
);
```
