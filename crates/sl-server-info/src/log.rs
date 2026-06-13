use regex::Regex;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogStream {
    Stdout,
    Stderr,
    Unknown,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLineInput {
    pub raw: String,
    pub stream: LogStream,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredLogLine {
    pub raw: String,
    pub message: String,
    pub stream: LogStream,
    pub timestamp_text: Option<String>,
    pub level: Option<LogLevel>,
    pub thread: Option<String>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainEvent {
    ServerReady,
    PlayerJoin { player: String },
    PlayerLeave { player: String },
    Chat { player: String, message: String },
    ErrorLike { message: String },
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLogLine {
    pub structured: StructuredLogLine,
    pub event: Option<DomainEvent>,
}

pub fn parse_log_line(_core_key: Option<&str>, input: LogLineInput) -> ParsedLogLine {
    let structured = parse_structured_line(input);
    let event = parse_domain_event(&structured);
    ParsedLogLine { structured, event }
}

fn parse_structured_line(input: LogLineInput) -> StructuredLogLine {
    let raw = input.raw.trim_end_matches(['\r', '\n']).to_string();
    let message = extract_message(&raw).unwrap_or_else(|| raw.clone());
    let timestamp_text = extract_timestamp(&raw);
    let level = extract_level(&raw);
    let thread = extract_thread(&raw);

    StructuredLogLine {
        raw,
        message,
        stream: input.stream,
        timestamp_text,
        level,
        thread,
    }
}

fn parse_domain_event(line: &StructuredLogLine) -> Option<DomainEvent> {
    let message = line.message.trim();
    if message.contains("Done (") && message.contains(")! For help") {
        return Some(DomainEvent::ServerReady);
    }

    let join_re = Regex::new(r"^([A-Za-z0-9_]{1,32}) joined the game$").expect("join regex");
    if let Some(captures) = join_re.captures(message) {
        return Some(DomainEvent::PlayerJoin {
            player: captures.get(1).unwrap().as_str().to_string(),
        });
    }

    let leave_re = Regex::new(r"^([A-Za-z0-9_]{1,32}) left the game$").expect("leave regex");
    if let Some(captures) = leave_re.captures(message) {
        return Some(DomainEvent::PlayerLeave {
            player: captures.get(1).unwrap().as_str().to_string(),
        });
    }

    let chat_re = Regex::new(r"^<([^>]+)>\s*(.+)$").expect("chat regex");
    if let Some(captures) = chat_re.captures(message) {
        return Some(DomainEvent::Chat {
            player: captures.get(1).unwrap().as_str().trim().to_string(),
            message: captures.get(2).unwrap().as_str().trim().to_string(),
        });
    }

    let lowered = message.to_ascii_lowercase();
    if lowered.contains("error") || lowered.contains("exception") || lowered.contains("fatal") {
        return Some(DomainEvent::ErrorLike {
            message: message.to_string(),
        });
    }

    None
}

fn extract_message(raw: &str) -> Option<String> {
    let java_re = Regex::new(
        r"^\[(?P<time>[^\]]+)\]\s*\[(?P<thread>[^/]+)/(?P<level>[A-Z]+)\]:\s*(?P<message>.+)$",
    )
    .expect("java log regex");
    if let Some(captures) = java_re.captures(raw) {
        return Some(captures.name("message").unwrap().as_str().to_string());
    }

    let simple_re = Regex::new(
        r"^(?:\[[^\]]+\]\s*)?(?P<level>TRACE|DEBUG|INFO|WARN|ERROR|FATAL)[:\s-]+(?P<message>.+)$",
    )
    .expect("simple log regex");
    simple_re.captures(raw).and_then(|captures| {
        let level = captures.name("level")?.as_str();
        let message = captures.name("message")?.as_str();
        Some(format!("{level} {message}"))
    })
}

fn extract_timestamp(raw: &str) -> Option<String> {
    let re = Regex::new(r"^\[([^\]]+)\]").expect("timestamp regex");
    re.captures(raw)
        .and_then(|captures| captures.get(1).map(|value| value.as_str().to_string()))
}

fn extract_level(raw: &str) -> Option<LogLevel> {
    if raw.contains("/TRACE]") || raw.starts_with("TRACE") {
        return Some(LogLevel::Trace);
    }
    if raw.contains("/DEBUG]") || raw.starts_with("DEBUG") {
        return Some(LogLevel::Debug);
    }
    if raw.contains("/INFO]") || raw.starts_with("INFO") {
        return Some(LogLevel::Info);
    }
    if raw.contains("/WARN]") || raw.starts_with("WARN") {
        return Some(LogLevel::Warn);
    }
    if raw.contains("/ERROR]") || raw.starts_with("ERROR") {
        return Some(LogLevel::Error);
    }
    if raw.contains("/FATAL]") || raw.starts_with("FATAL") {
        return Some(LogLevel::Fatal);
    }
    None
}

fn extract_thread(raw: &str) -> Option<String> {
    let re = Regex::new(r"^\[[^\]]+\]\s*\[([^/]+)/[A-Z]+\]").expect("thread regex");
    re.captures(raw)
        .and_then(|captures| captures.get(1).map(|value| value.as_str().to_string()))
}

#[cfg(test)]
mod tests {
    use crate::{parse_log_line, DomainEvent, LogLevel, LogLineInput, LogStream};

    #[test]
    fn detects_server_ready_event() {
        let parsed = parse_log_line(
            Some("paper"),
            LogLineInput {
                raw: "[12:00:00] [Server thread/INFO]: Done (1.234s)! For help, type \"help\""
                    .to_string(),
                stream: LogStream::Stdout,
            },
        );
        assert_eq!(parsed.event, Some(DomainEvent::ServerReady));
        assert_eq!(parsed.structured.level, Some(LogLevel::Info));
    }

    #[test]
    fn detects_player_join_leave_chat_and_error() {
        let join = parse_log_line(
            None,
            LogLineInput {
                raw: "Alex joined the game".to_string(),
                stream: LogStream::Stdout,
            },
        );
        assert_eq!(
            join.event,
            Some(DomainEvent::PlayerJoin {
                player: "Alex".to_string()
            })
        );

        let leave = parse_log_line(
            None,
            LogLineInput {
                raw: "Alex left the game".to_string(),
                stream: LogStream::Stdout,
            },
        );
        assert_eq!(
            leave.event,
            Some(DomainEvent::PlayerLeave {
                player: "Alex".to_string()
            })
        );

        let chat = parse_log_line(
            None,
            LogLineInput {
                raw: "<Alex> hello".to_string(),
                stream: LogStream::Stdout,
            },
        );
        assert_eq!(
            chat.event,
            Some(DomainEvent::Chat {
                player: "Alex".to_string(),
                message: "hello".to_string()
            })
        );

        let error = parse_log_line(
            None,
            LogLineInput {
                raw: "ERROR Failed to load something".to_string(),
                stream: LogStream::Stderr,
            },
        );
        assert!(matches!(error.event, Some(DomainEvent::ErrorLike { .. })));
    }

    #[test]
    fn preserves_unmatched_lines_as_structured_output() {
        let parsed = parse_log_line(
            None,
            LogLineInput {
                raw: "random line".to_string(),
                stream: LogStream::Unknown,
            },
        );
        assert_eq!(parsed.event, None);
        assert_eq!(parsed.structured.message, "random line");
    }
}
