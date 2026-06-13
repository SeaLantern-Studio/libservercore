use regex::Regex;

use crate::model::{InfoFact, InfoValue, ProbeWarning, ServerInfoError};
use crate::rcon::{RconEndpoint, RconProbeOptions};
use crate::snapshot::run_rcon_command;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricProviderKind {
    SparkHealth,
    SparkTps,
    SparkHealthReport,
    ForgeTps,
    PaperTps,
    PaperMspt,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceScopeKind {
    Global,
    Dimension,
    World,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceScope {
    pub kind: PerformanceScopeKind,
    pub name: Option<String>,
    pub tps: Option<f32>,
    pub mspt: Option<f32>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceSnapshot {
    pub provider: MetricProviderKind,
    pub overall_tps: Option<f32>,
    pub overall_mspt: Option<f32>,
    pub scopes: Vec<PerformanceScope>,
    pub facts: Vec<InfoFact>,
    pub raw_output: String,
    pub warnings: Vec<ProbeWarning>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceProbeResult {
    pub snapshot: Option<PerformanceSnapshot>,
    pub warnings: Vec<ProbeWarning>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerformanceProbeOptions {
    pub core_key: Option<String>,
    pub connect_timeout_ms: u64,
    pub read_timeout_ms: u64,
    pub providers: Vec<MetricProviderKind>,
}

impl Default for PerformanceProbeOptions {
    fn default() -> Self {
        Self {
            core_key: None,
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
        }
    }
}

pub fn probe_performance(
    endpoint: &RconEndpoint,
    options: &PerformanceProbeOptions,
) -> Result<Option<PerformanceSnapshot>, ServerInfoError> {
    Ok(probe_performance_detailed(endpoint, options)?.snapshot)
}

pub fn probe_performance_detailed(
    endpoint: &RconEndpoint,
    options: &PerformanceProbeOptions,
) -> Result<PerformanceProbeResult, ServerInfoError> {
    probe_performance_with_runner(options, |command| {
        run_rcon_command(
            endpoint,
            &RconProbeOptions {
                core_key: options.core_key.clone(),
                connect_timeout_ms: options.connect_timeout_ms,
                read_timeout_ms: options.read_timeout_ms,
            },
            command,
        )
    })
}

pub(crate) fn probe_performance_with_runner<F>(
    options: &PerformanceProbeOptions,
    mut runner: F,
) -> Result<PerformanceProbeResult, ServerInfoError>
where
    F: FnMut(&str) -> Result<String, ServerInfoError>,
{
    let mut warnings = Vec::new();
    for provider in &options.providers {
        let command = provider.command();
        let output = match runner(command) {
            Ok(output) => output,
            Err(error) => {
                warnings.push(ProbeWarning::new(
                    "metrics",
                    Some(provider.name()),
                    error.to_string(),
                ));
                continue;
            }
        };

        match provider.parse(&output) {
            Ok(mut snapshot) => {
                snapshot.warnings.extend(warnings);
                return Ok(PerformanceProbeResult {
                    snapshot: Some(snapshot),
                    warnings: Vec::new(),
                });
            }
            Err(message) => {
                warnings.push(ProbeWarning::new("metrics", Some(provider.name()), message));
            }
        }
    }

    Ok(PerformanceProbeResult {
        snapshot: None,
        warnings,
    })
}

impl MetricProviderKind {
    fn command(self) -> &'static str {
        match self {
            Self::SparkHealth => "spark health",
            Self::SparkTps => "spark tps",
            Self::SparkHealthReport => "spark healthreport",
            Self::ForgeTps => "forge tps",
            Self::PaperTps => "tps",
            Self::PaperMspt => "mspt",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::SparkHealth => "spark_health",
            Self::SparkTps => "spark_tps",
            Self::SparkHealthReport => "spark_healthreport",
            Self::ForgeTps => "forge_tps",
            Self::PaperTps => "paper_tps",
            Self::PaperMspt => "paper_mspt",
        }
    }

    fn parse(self, output: &str) -> Result<PerformanceSnapshot, String> {
        if looks_like_provider_unavailable(output) {
            return Err(format!("provider command unavailable: {}", output.trim()));
        }

        match self {
            Self::SparkHealth => parse_spark_health(output, "spark health"),
            Self::SparkTps => parse_spark_tps(output, "spark tps"),
            Self::SparkHealthReport => parse_spark_healthreport(output),
            Self::ForgeTps => parse_forge_tps(output),
            Self::PaperTps => parse_paper_tps(output),
            Self::PaperMspt => parse_paper_mspt(output),
        }
    }
}

fn parse_paper_tps(output: &str) -> Result<PerformanceSnapshot, String> {
    let cleaned = strip_leading_slash_echo(output);
    let values = capture_all_floats(
        &cleaned,
        r"(?i)TPS\s+from\s+last\s+1m,\s*5m,\s*15m:\s*([0-9]+(?:\.[0-9]+)?)\s*,\s*([0-9]+(?:\.[0-9]+)?)\s*,\s*([0-9]+(?:\.[0-9]+)?)",
    );

    if values.len() != 3 {
        return Err(format!("unable to parse tps output: {}", cleaned.trim()));
    }

    let overall_tps = values.first().copied();
    let facts = vec![
        InfoFact {
            namespace: "paper",
            key: "tps_text",
            value: InfoValue::Text(cleaned.trim().to_string()),
            source_command: "tps",
        },
        InfoFact {
            namespace: "paper",
            key: "tps_averages",
            value: InfoValue::StringList(
                values
                    .iter()
                    .map(|value| format!("{value:.1}"))
                    .collect::<Vec<_>>(),
            ),
            source_command: "tps",
        },
    ];

    Ok(PerformanceSnapshot {
        provider: MetricProviderKind::PaperTps,
        overall_tps,
        overall_mspt: None,
        scopes: vec![PerformanceScope {
            kind: PerformanceScopeKind::Global,
            name: Some("1m/5m/15m".to_string()),
            tps: overall_tps,
            mspt: None,
        }],
        facts,
        raw_output: output.to_string(),
        warnings: Vec::new(),
    })
}

fn parse_paper_mspt(output: &str) -> Result<PerformanceSnapshot, String> {
    let cleaned = strip_leading_slash_echo(output);
    let header_re = Regex::new(
        r"(?i)Server\s+tick\s+times\s*\(\s*avg\s*/\s*min\s*/\s*max\s*\)\s*from\s+last\s+5s\s*,\s*10s\s*,\s*1m:",
    )
    .expect("paper mspt header regex should compile");
    if !header_re.is_match(&cleaned) {
        return Err(format!("unable to parse mspt output: {}", cleaned.trim()));
    }

    let values = capture_all_floats(
        &cleaned,
        r"(?i)([0-9]+(?:\.[0-9]+)?)\s*/\s*([0-9]+(?:\.[0-9]+)?)\s*/\s*([0-9]+(?:\.[0-9]+)?)\s*,\s*([0-9]+(?:\.[0-9]+)?)\s*/\s*([0-9]+(?:\.[0-9]+)?)\s*/\s*([0-9]+(?:\.[0-9]+)?)\s*,\s*([0-9]+(?:\.[0-9]+)?)\s*/\s*([0-9]+(?:\.[0-9]+)?)\s*/\s*([0-9]+(?:\.[0-9]+)?)",
    );

    if values.len() != 9 {
        return Err(format!("unable to parse mspt output: {}", cleaned.trim()));
    }

    let overall_mspt = values.first().copied();
    let facts = vec![
        InfoFact {
            namespace: "paper",
            key: "mspt_text",
            value: InfoValue::Text(cleaned.trim().to_string()),
            source_command: "mspt",
        },
        InfoFact {
            namespace: "paper",
            key: "mspt_windows",
            value: InfoValue::StringList(
                ["5s", "10s", "1m"]
                    .iter()
                    .enumerate()
                    .map(|(index, window)| {
                        let base = index * 3;
                        format!(
                            "{window}:avg={:.1},min={:.1},max={:.1}",
                            values[base], values[base + 1], values[base + 2]
                        )
                    })
                    .collect::<Vec<_>>(),
            ),
            source_command: "mspt",
        },
    ];

    Ok(PerformanceSnapshot {
        provider: MetricProviderKind::PaperMspt,
        overall_tps: None,
        overall_mspt,
        scopes: vec![PerformanceScope {
            kind: PerformanceScopeKind::Global,
            name: Some("5s/10s/1m".to_string()),
            tps: None,
            mspt: overall_mspt,
        }],
        facts,
        raw_output: output.to_string(),
        warnings: Vec::new(),
    })
}

fn parse_spark_health(output: &str, source_command: &'static str) -> Result<PerformanceSnapshot, String> {
    let cleaned = strip_leading_slash_echo(output);
    let overall_tps = capture_first_float(
        &cleaned,
        &[
            r"TPS[:=]\s*([0-9]+(?:\.[0-9]+)?)",
            r"ticks per second[:=]?\s*([0-9]+(?:\.[0-9]+)?)",
            r"current tps[:=]?\s*([0-9]+(?:\.[0-9]+)?)",
        ],
    );
    let overall_mspt = capture_first_float(
        &cleaned,
        &[
            r"MSPT[:=]\s*([0-9]+(?:\.[0-9]+)?)",
            r"milliseconds per tick[:=]?\s*([0-9]+(?:\.[0-9]+)?)",
            r"tick duration[:=]?\s*([0-9]+(?:\.[0-9]+)?)\s*ms",
        ],
    );

    if overall_tps.is_none() && overall_mspt.is_none() {
        return Err(format!(
            "unable to parse {source_command} output: {}",
            cleaned.trim()
        ));
    }

    Ok(PerformanceSnapshot {
        provider: if source_command == "spark tps" {
            MetricProviderKind::SparkTps
        } else {
            MetricProviderKind::SparkHealth
        },
        overall_tps,
        overall_mspt,
        scopes: Vec::new(),
        facts: vec![InfoFact {
            namespace: "spark",
            key: if source_command == "spark tps" {
                "tps_text"
            } else {
                "health_text"
            },
            value: InfoValue::Text(cleaned.trim().to_string()),
            source_command,
        }],
        raw_output: output.to_string(),
        warnings: Vec::new(),
    })
}

fn parse_spark_tps(output: &str, source_command: &'static str) -> Result<PerformanceSnapshot, String> {
    parse_spark_health(output, source_command)
}

fn parse_spark_healthreport(output: &str) -> Result<PerformanceSnapshot, String> {
    let mut parsed = parse_spark_health(output, "spark healthreport")?;
    parsed.provider = MetricProviderKind::SparkHealthReport;
    parsed.facts[0].key = "healthreport_text";
    Ok(parsed)
}

fn parse_forge_tps(output: &str) -> Result<PerformanceSnapshot, String> {
    let cleaned = strip_leading_slash_echo(output);
    let mut scopes = Vec::new();
    let overall_re = Regex::new(
        r"(?i)^overall\s+tps[:=]\s*([0-9]+(?:\.[0-9]+)?)\s+mspt[:=]\s*([0-9]+(?:\.[0-9]+)?)$",
    )
    .expect("overall forge tps regex should compile");
    let scope_re = Regex::new(r"(?i)^(dimension|world)\s+(.+?)\s+tps[:=]\s*([0-9]+(?:\.[0-9]+)?)\s+mspt[:=]\s*([0-9]+(?:\.[0-9]+)?)$")
        .expect("scope forge tps regex should compile");

    for raw_line in cleaned.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(captures) = overall_re.captures(line) {
            let tps = captures
                .get(1)
                .and_then(|value| value.as_str().parse::<f32>().ok());
            let mspt = captures
                .get(2)
                .and_then(|value| value.as_str().parse::<f32>().ok());
            scopes.push(PerformanceScope {
                kind: PerformanceScopeKind::Global,
                name: None,
                tps,
                mspt,
            });
            continue;
        }

        if let Some(captures) = scope_re.captures(line) {
            let kind_text = captures
                .get(1)
                .map(|value| value.as_str())
                .unwrap_or_default();
            let name = captures
                .get(2)
                .map(|value| value.as_str().trim().to_string());
            let tps = captures
                .get(3)
                .and_then(|value| value.as_str().parse::<f32>().ok());
            let mspt = captures
                .get(4)
                .and_then(|value| value.as_str().parse::<f32>().ok());
            let kind = if kind_text.eq_ignore_ascii_case("world") {
                PerformanceScopeKind::World
            } else {
                PerformanceScopeKind::Dimension
            };
            scopes.push(PerformanceScope {
                kind,
                name,
                tps,
                mspt,
            });
        }
    }

    let overall_tps = capture_first_float(
        &cleaned,
        &[
            r"(?i)overall\s+tps[:=]\s*([0-9]+(?:\.[0-9]+)?)",
            r"mean tps[:=]\s*([0-9]+(?:\.[0-9]+)?)",
        ],
    );
    let overall_mspt = capture_first_float(
        &cleaned,
        &[
            r"(?i)overall\s+tps[:=]\s*[0-9]+(?:\.[0-9]+)?\s+mspt[:=]\s*([0-9]+(?:\.[0-9]+)?)",
            r"mean mspt[:=]\s*([0-9]+(?:\.[0-9]+)?)",
        ],
    );

    if overall_tps.is_none() && overall_mspt.is_none() && scopes.is_empty() {
        return Err(format!(
            "unable to parse forge tps output: {}",
            cleaned.trim()
        ));
    }

    Ok(PerformanceSnapshot {
        provider: MetricProviderKind::ForgeTps,
        overall_tps,
        overall_mspt,
        scopes,
        facts: vec![InfoFact {
            namespace: "forge",
            key: "tps_text",
            value: InfoValue::Text(cleaned.trim().to_string()),
            source_command: "forge tps",
        }],
        raw_output: output.to_string(),
        warnings: Vec::new(),
    })
}

fn strip_leading_slash_echo(output: &str) -> String {
    output
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.eq_ignore_ascii_case("/spark healthreport")
                && !trimmed.eq_ignore_ascii_case("spark healthreport")
                && !trimmed.eq_ignore_ascii_case("/spark health")
                && !trimmed.eq_ignore_ascii_case("spark health")
                && !trimmed.eq_ignore_ascii_case("/spark tps")
                && !trimmed.eq_ignore_ascii_case("spark tps")
                && !trimmed.eq_ignore_ascii_case("/forge tps")
                && !trimmed.eq_ignore_ascii_case("forge tps")
                && !trimmed.eq_ignore_ascii_case("/tps")
                && !trimmed.eq_ignore_ascii_case("tps")
                && !trimmed.eq_ignore_ascii_case("/mspt")
                && !trimmed.eq_ignore_ascii_case("mspt")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn capture_all_floats(input: &str, pattern: &str) -> Vec<f32> {
    let re = Regex::new(pattern).expect("metric regex should compile");
    let Some(captures) = re.captures(input) else {
        return Vec::new();
    };

    captures
        .iter()
        .skip(1)
        .flatten()
        .filter_map(|value| value.as_str().parse::<f32>().ok())
        .collect()
}

fn capture_first_float(input: &str, patterns: &[&str]) -> Option<f32> {
    for pattern in patterns {
        let re = Regex::new(pattern).expect("metric regex should compile");
        if let Some(captures) = re.captures(input) {
            if let Some(value) = captures
                .get(1)
                .and_then(|value| value.as_str().parse::<f32>().ok())
            {
                return Some(value);
            }
        }
    }
    None
}

fn looks_like_provider_unavailable(output: &str) -> bool {
    let lowered = output.to_ascii_lowercase();
    lowered.contains("unknown")
        || lowered.contains("unrecognized")
        || lowered.contains("no such command")
        || lowered.contains("permission")
        || lowered.contains("not found")
}

#[cfg(test)]
mod tests {
    use super::{
        parse_forge_tps, parse_paper_mspt, parse_paper_tps, parse_spark_health,
        parse_spark_healthreport, parse_spark_tps,
        probe_performance_with_runner,
        MetricProviderKind, PerformanceProbeOptions,
    };
    use crate::{rcon::RconEndpoint, ServerInfoError};

    #[test]
    fn parses_spark_health_summary() {
        let parsed = parse_spark_health("TPS: 19.97\nMSPT: 12.4\nCPU: healthy", "spark health").unwrap();
        assert_eq!(parsed.provider, MetricProviderKind::SparkHealth);
        assert_eq!(parsed.overall_tps, Some(19.97));
        assert_eq!(parsed.overall_mspt, Some(12.4));
    }

    #[test]
    fn parses_spark_tps_summary_with_command_echo() {
        let parsed = parse_spark_tps("/spark tps\nTPS: 19.98\nMSPT: 10.1", "spark tps").unwrap();
        assert_eq!(parsed.provider, MetricProviderKind::SparkTps);
        assert_eq!(parsed.overall_tps, Some(19.98));
        assert_eq!(parsed.overall_mspt, Some(10.1));
    }

    #[test]
    fn parses_spark_healthreport_summary() {
        let parsed = parse_spark_healthreport("TPS: 19.97\nMSPT: 12.4\nCPU: healthy").unwrap();
        assert_eq!(parsed.overall_tps, Some(19.97));
        assert_eq!(parsed.overall_mspt, Some(12.4));
    }

    #[test]
    fn parses_spark_healthreport_with_command_echo() {
        let parsed =
            parse_spark_healthreport("/spark healthreport\nTPS: 19.97\nMSPT: 12.4").unwrap();
        assert_eq!(parsed.overall_tps, Some(19.97));
        assert_eq!(parsed.overall_mspt, Some(12.4));
    }

    #[test]
    fn parses_forge_tps_summary_and_scopes() {
        let parsed = parse_forge_tps(
            "Overall tps: 20.0 mspt: 15.2\nDimension minecraft:overworld tps: 19.8 mspt: 16.1",
        )
        .unwrap();
        assert_eq!(parsed.overall_tps, Some(20.0));
        assert_eq!(parsed.overall_mspt, Some(15.2));
        assert_eq!(parsed.scopes.len(), 2);
    }

    #[test]
    fn parses_paper_tps_summary() {
        let parsed = parse_paper_tps("TPS from last 1m, 5m, 15m: 20.0, 19.9, 19.8").unwrap();
        assert_eq!(parsed.provider, MetricProviderKind::PaperTps);
        assert_eq!(parsed.overall_tps, Some(20.0));
        assert_eq!(parsed.overall_mspt, None);
    }

    #[test]
    fn parses_paper_mspt_summary_with_command_echo() {
        let parsed = parse_paper_mspt(
            "/mspt\nServer tick times (avg/min/max) from last 5s, 10s, 1m:\n◴ 12.3/10.0/17.1, 13.4/9.8/18.0, 14.5/9.9/20.2",
        )
        .unwrap();
        assert_eq!(parsed.provider, MetricProviderKind::PaperMspt);
        assert_eq!(parsed.overall_mspt, Some(12.3));
        assert_eq!(parsed.overall_tps, None);
    }

    #[test]
    fn performance_probe_options_default_to_known_providers() {
        let options = PerformanceProbeOptions::default();
        assert_eq!(
            options.providers,
            vec![
                MetricProviderKind::SparkHealth,
                MetricProviderKind::SparkTps,
                MetricProviderKind::SparkHealthReport,
                MetricProviderKind::ForgeTps,
                MetricProviderKind::PaperTps,
                MetricProviderKind::PaperMspt,
            ]
        );
    }

    #[test]
    fn endpoint_type_is_constructible_for_probe_usage() {
        let endpoint = RconEndpoint {
            host: "127.0.0.1".to_string(),
            port: 25575,
            password: "secret".to_string(),
        };
        assert_eq!(endpoint.address(), "127.0.0.1:25575");
    }

    #[test]
    fn performance_probe_tries_next_provider_after_failure() {
        let options = PerformanceProbeOptions::default();

        let result = probe_performance_with_runner(&options, |command| match command {
            "spark health" => Ok("Unknown or incomplete command".to_string()),
            "spark tps" => Ok("Unknown or incomplete command".to_string()),
            "spark healthreport" => Ok("Unknown or incomplete command".to_string()),
            "forge tps" => Ok("Overall tps: 20.0 mspt: 15.2".to_string()),
            "tps" => Ok("TPS from last 1m, 5m, 15m: 20.0, 19.9, 19.8".to_string()),
            "mspt" => Ok("Server tick times (avg/min/max) from last 5s, 10s, 1m:\n◴ 12.3/10.0/17.1, 13.4/9.8/18.0, 14.5/9.9/20.2".to_string()),
            other => Err(ServerInfoError::Command(format!(
                "unexpected command: {other}"
            ))),
        })
        .unwrap();

        let snapshot = result.snapshot.unwrap();

        assert_eq!(snapshot.provider, MetricProviderKind::ForgeTps);
        assert!(snapshot
            .warnings
            .iter()
            .any(|warning| warning.provider == Some("spark_health")));
        assert!(snapshot
            .warnings
            .iter()
            .any(|warning| warning.provider == Some("spark_tps")));
        assert!(snapshot
            .warnings
            .iter()
            .any(|warning| warning.provider == Some("spark_healthreport")));
        assert!(!snapshot
            .warnings
            .iter()
            .any(|warning| warning.provider == Some("paper_tps")));
    }

    #[test]
    fn performance_probe_falls_back_to_paper_providers() {
        let options = PerformanceProbeOptions::default();

        let result = probe_performance_with_runner(&options, |command| match command {
            "spark health" => Ok("Unknown or incomplete command".to_string()),
            "spark tps" => Ok("Unknown or incomplete command".to_string()),
            "spark healthreport" => Ok("Unknown or incomplete command".to_string()),
            "forge tps" => Ok("Unknown or incomplete command".to_string()),
            "tps" => Ok("TPS from last 1m, 5m, 15m: 20.0, 19.9, 19.8".to_string()),
            "mspt" => Ok("Unknown or incomplete command".to_string()),
            other => Err(ServerInfoError::Command(format!(
                "unexpected command: {other}"
            ))),
        })
        .unwrap();

        let snapshot = result.snapshot.unwrap();
        assert_eq!(snapshot.provider, MetricProviderKind::PaperTps);
        assert_eq!(snapshot.overall_tps, Some(20.0));
        assert!(snapshot
            .warnings
            .iter()
            .any(|warning| warning.provider == Some("forge_tps")));
    }

    #[test]
    fn performance_probe_returns_none_when_all_providers_fail() {
        let options = PerformanceProbeOptions::default();

        let result = probe_performance_with_runner(&options, |_command| {
            Ok("Unknown or incomplete command".to_string())
        })
        .unwrap();

        assert!(result.snapshot.is_none());
        assert_eq!(result.warnings.len(), 6);
        assert!(result
            .warnings
            .iter()
            .any(|warning| warning.provider == Some("spark_health")));
        assert!(result
            .warnings
            .iter()
            .any(|warning| warning.provider == Some("spark_tps")));
        assert!(result
            .warnings
            .iter()
            .any(|warning| warning.provider == Some("spark_healthreport")));
        assert!(result
            .warnings
            .iter()
            .any(|warning| warning.provider == Some("forge_tps")));
        assert!(result
            .warnings
            .iter()
            .any(|warning| warning.provider == Some("paper_tps")));
        assert!(result
            .warnings
            .iter()
            .any(|warning| warning.provider == Some("paper_mspt")));
    }
}
