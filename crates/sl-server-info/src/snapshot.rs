use std::time::Instant;

use regex::Regex;
use server_core_taxonomy::CoreFamily;

use crate::model::{
    InfoFact, InfoValue, OnlinePlayersSnapshot, ProbeWarning, ServerInfoError, ServerInfoSnapshot,
};
use crate::rcon::{RconEndpoint, RconProbeOptions};

pub fn probe_snapshot(
    endpoint: &RconEndpoint,
    options: &RconProbeOptions,
) -> Result<ServerInfoSnapshot, ServerInfoError> {
    probe_snapshot_with_runner(options, |command| {
        run_rcon_command(endpoint, options, command)
    })
}

pub(crate) fn probe_snapshot_with_runner<F>(
    options: &RconProbeOptions,
    mut runner: F,
) -> Result<ServerInfoSnapshot, ServerInfoError>
where
    F: FnMut(&str) -> Result<String, ServerInfoError>,
{
    let started = Instant::now();
    let list_output = runner("list")?;
    let latency_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let players = parse_list_output(&list_output)?;

    let mut facts = Vec::new();
    let mut warnings = Vec::new();

    if is_bukkit_like(options.core_key.as_deref()) {
        match runner("version") {
            Ok(version_output) => facts.push(InfoFact {
                namespace: "generic",
                key: "software_text",
                value: InfoValue::Text(version_output.trim().to_string()),
                source_command: "version",
            }),
            Err(error) => warnings.push(ProbeWarning::new("snapshot", None, error.to_string())),
        }

        match runner("plugins") {
            Ok(plugins_output) => {
                let trimmed = plugins_output.trim().to_string();
                facts.push(InfoFact {
                    namespace: "bukkit",
                    key: "plugins_text",
                    value: InfoValue::Text(trimmed.clone()),
                    source_command: "plugins",
                });

                let plugin_names = parse_plugin_names(&trimmed);
                if !plugin_names.is_empty() {
                    facts.push(InfoFact {
                        namespace: "bukkit",
                        key: "plugin_names",
                        value: InfoValue::StringList(plugin_names),
                        source_command: "plugins",
                    });
                }
            }
            Err(error) => warnings.push(ProbeWarning::new("snapshot", None, error.to_string())),
        }
    }

    Ok(ServerInfoSnapshot {
        reachable: true,
        latency_ms: Some(latency_ms),
        players: Some(players),
        facts,
        warnings,
    })
}

pub(crate) fn run_rcon_command(
    endpoint: &RconEndpoint,
    options: &RconProbeOptions,
    command: &str,
) -> Result<String, ServerInfoError> {
    let address = endpoint.address();
    let password = endpoint.password.clone();
    let command_text = command.to_string();
    let connect_timeout = std::time::Duration::from_millis(options.connect_timeout_ms.max(1));
    let read_timeout = std::time::Duration::from_millis(options.read_timeout_ms.max(1));

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| {
            ServerInfoError::Connection(format!("failed to build tokio runtime: {error}"))
        })?;

    runtime.block_on(async move {
        let connection = tokio::time::timeout(connect_timeout, async {
            rcon::Connection::builder()
                .enable_minecraft_quirks(true)
                .connect(address.as_str(), password.as_str())
                .await
        })
        .await
        .map_err(|_| ServerInfoError::Connection(format!("timed out connecting to {address}")))?;

        let mut connection =
            connection.map_err(|error| map_connect_error(&address, &error.to_string()))?;
        tokio::time::timeout(read_timeout, connection.cmd(command_text.as_str()))
            .await
            .map_err(|_| {
                ServerInfoError::Command(format!("timed out running '{command_text}' via RCON"))
            })?
            .map_err(|error| {
                ServerInfoError::Command(format!(
                    "failed to run '{command_text}' via RCON: {error}"
                ))
            })
    })
}

pub(crate) fn parse_list_output(output: &str) -> Result<OnlinePlayersSnapshot, ServerInfoError> {
    let re = Regex::new(
        r"There are\s+(\d+)\s+of(?:\s+a)?\s+max(?:imum)?(?:\s+of)?\s+(\d+)\s+players\s+online(?::\s*(.*))?$",
    )
    .expect("list regex should compile");
    let trimmed = output.trim();
    let captures = re.captures(trimmed).ok_or_else(|| {
        ServerInfoError::Parse(format!("failed to parse 'list' output: {trimmed}"))
    })?;

    let online = captures
        .get(1)
        .and_then(|value| value.as_str().parse::<u32>().ok())
        .ok_or_else(|| {
            ServerInfoError::Parse(format!("failed to parse online count from: {trimmed}"))
        })?;
    let max = captures
        .get(2)
        .and_then(|value| value.as_str().parse::<u32>().ok());
    let players = captures
        .get(3)
        .map(|value| parse_player_names(value.as_str()))
        .unwrap_or_default();

    Ok(OnlinePlayersSnapshot {
        online,
        max,
        players,
    })
}

fn parse_player_names(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .collect()
}

fn parse_plugin_names(output: &str) -> Vec<String> {
    let Some((_, list)) = output.split_once(':') else {
        return Vec::new();
    };

    list.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .split_whitespace()
                .next()
                .unwrap_or(value)
                .trim_matches(|ch| ch == '(' || ch == ')' || ch == '[' || ch == ']')
                .to_string()
        })
        .filter(|value| !value.is_empty())
        .collect()
}

fn is_bukkit_like(core_key: Option<&str>) -> bool {
    let Some(core_key) = core_key else {
        return false;
    };

    matches!(
        CoreFamily::from_core_key(core_key),
        CoreFamily::BukkitLike | CoreFamily::MixedExtension
    )
}

fn map_connect_error(address: &str, error: &str) -> ServerInfoError {
    let lowered = error.to_ascii_lowercase();
    if lowered.contains("auth") || lowered.contains("login") || lowered.contains("password") {
        ServerInfoError::Authentication(format!(
            "failed to authenticate with RCON endpoint {address}: {error}"
        ))
    } else {
        ServerInfoError::Connection(format!(
            "failed to connect to RCON endpoint {address}: {error}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_list_output, parse_plugin_names, probe_snapshot_with_runner};
    use crate::model::ServerInfoError;
    use crate::rcon::RconProbeOptions;

    #[test]
    fn parses_zero_player_list_output() {
        let parsed = parse_list_output("There are 0 of a max 20 players online").unwrap();
        assert_eq!(parsed.online, 0);
        assert_eq!(parsed.max, Some(20));
        assert!(parsed.players.is_empty());
    }

    #[test]
    fn parses_list_output_with_names() {
        let parsed =
            parse_list_output("There are 2 of a max 20 players online: Alex, Steve").unwrap();
        assert_eq!(parsed.online, 2);
        assert_eq!(parsed.max, Some(20));
        assert_eq!(parsed.players, vec!["Alex", "Steve"]);
    }

    #[test]
    fn parses_list_output_with_extra_of_variant() {
        let parsed = parse_list_output("There are 1 of a max of 20 players online: Alex").unwrap();
        assert_eq!(parsed.online, 1);
        assert_eq!(parsed.max, Some(20));
        assert_eq!(parsed.players, vec!["Alex"]);
    }

    #[test]
    fn extracts_plugin_names_from_bukkit_output() {
        let names = parse_plugin_names("Server Plugins (2): Essentials 2.0, WorldEdit 7.3");
        assert_eq!(names, vec!["Essentials", "WorldEdit"]);
    }

    #[test]
    fn snapshot_keeps_warnings_when_optional_commands_fail() {
        let options = RconProbeOptions {
            core_key: Some("paper".to_string()),
            ..RconProbeOptions::default()
        };

        let snapshot = probe_snapshot_with_runner(&options, |command| match command {
            "list" => Ok("There are 2 of a max 20 players online: Alex, Steve".to_string()),
            "version" => Err(ServerInfoError::Command("version failed".to_string())),
            "plugins" => Err(ServerInfoError::Command("plugins failed".to_string())),
            other => Err(ServerInfoError::Command(format!(
                "unexpected command: {other}"
            ))),
        })
        .unwrap();

        assert_eq!(snapshot.players.unwrap().online, 2);
        assert_eq!(snapshot.warnings.len(), 2);
    }
}
