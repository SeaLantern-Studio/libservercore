use std::path::Path;

use crate::error::ConfigIoError;
use crate::formats::ConfigFormat;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlDocument {
    raw: String,
}

#[derive(Debug, Clone)]
pub struct TomlDocument {
    inner: toml_edit::DocumentMut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonDocument {
    value: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextDocument {
    raw: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertiesDocument {
    lines: Vec<PropertiesLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PropertiesLine {
    Blank,
    Comment(String),
    Entry {
        key: String,
        separator: char,
        value: String,
    },
    Raw(String),
}

#[derive(Debug, Clone)]
pub enum ConfigDocument {
    Yaml(YamlDocument),
    Toml(TomlDocument),
    Json(JsonDocument),
    Properties(PropertiesDocument),
    Text(TextDocument),
}

pub fn read_config_document(
    format: ConfigFormat,
    content: &str,
) -> Result<ConfigDocument, ConfigIoError> {
    match format {
        ConfigFormat::Yaml => Ok(ConfigDocument::Yaml(YamlDocument {
            raw: content.to_string(),
        })),
        ConfigFormat::Toml => {
            let document = content
                .parse::<toml_edit::DocumentMut>()
                .map_err(|e| ConfigIoError::ParseFailed(e.to_string()))?;
            Ok(ConfigDocument::Toml(TomlDocument { inner: document }))
        }
        ConfigFormat::Json => {
            let value = serde_json::from_str(content)
                .map_err(|e| ConfigIoError::ParseFailed(e.to_string()))?;
            Ok(ConfigDocument::Json(JsonDocument { value }))
        }
        ConfigFormat::Properties => Ok(ConfigDocument::Properties(PropertiesDocument::parse(
            content,
        ))),
        ConfigFormat::Text => Ok(ConfigDocument::Text(TextDocument {
            raw: content.to_string(),
        })),
    }
}

pub fn read_config_file(path: &Path) -> Result<ConfigDocument, ConfigIoError> {
    let format = infer_format_from_path(path)?;
    let content = std::fs::read_to_string(path).map_err(|e| ConfigIoError::Io(e.to_string()))?;
    read_config_document(format, &content)
}

pub fn write_config_document(document: &ConfigDocument) -> Result<String, ConfigIoError> {
    match document {
        ConfigDocument::Yaml(document) => Ok(document.raw.clone()),
        ConfigDocument::Toml(document) => Ok(document.inner.to_string()),
        ConfigDocument::Json(document) => serde_json::to_string_pretty(&document.value)
            .map_err(|e| ConfigIoError::WriteFailed(e.to_string())),
        ConfigDocument::Properties(document) => Ok(document.render()),
        ConfigDocument::Text(document) => Ok(document.raw.clone()),
    }
}

pub fn write_config_file(path: &Path, document: &ConfigDocument) -> Result<(), ConfigIoError> {
    let content = write_config_document(document)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ConfigIoError::Io(e.to_string()))?;
    }
    std::fs::write(path, content).map_err(|e| ConfigIoError::WriteFailed(e.to_string()))
}

fn infer_format_from_path(path: &Path) -> Result<ConfigFormat, ConfigIoError> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .ok_or_else(|| ConfigIoError::UnsupportedFormat(path.display().to_string()))?;
    ConfigFormat::from_extension(extension)
        .ok_or_else(|| ConfigIoError::UnsupportedFormat(path.display().to_string()))
}

impl YamlDocument {
    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn replace_raw(&mut self, raw: impl Into<String>) {
        self.raw = raw.into();
    }
}

impl TomlDocument {
    pub fn inner(&self) -> &toml_edit::DocumentMut {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut toml_edit::DocumentMut {
        &mut self.inner
    }
}

impl JsonDocument {
    pub fn value(&self) -> &serde_json::Value {
        &self.value
    }

    pub fn value_mut(&mut self) -> &mut serde_json::Value {
        &mut self.value
    }
}

impl TextDocument {
    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn replace_raw(&mut self, raw: impl Into<String>) {
        self.raw = raw.into();
    }
}

impl PropertiesDocument {
    fn parse(content: &str) -> Self {
        let mut lines = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                lines.push(PropertiesLine::Blank);
                continue;
            }
            if trimmed.starts_with('#') || trimmed.starts_with('!') {
                lines.push(PropertiesLine::Comment(line.to_string()));
                continue;
            }

            if let Some((index, separator)) =
                line.char_indices().find(|(_, ch)| *ch == '=' || *ch == ':')
            {
                let key = line[..index].trim().to_string();
                let value = line[index + separator.len_utf8()..].to_string();
                lines.push(PropertiesLine::Entry {
                    key,
                    separator,
                    value,
                });
            } else {
                lines.push(PropertiesLine::Raw(line.to_string()));
            }
        }

        Self { lines }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.lines.iter().find_map(|line| match line {
            PropertiesLine::Entry {
                key: existing_key,
                value,
                ..
            } if existing_key == key => Some(value.as_str()),
            _ => None,
        })
    }

    pub fn set(&mut self, key: &str, value: impl Into<String>) {
        let value = value.into();

        for line in &mut self.lines {
            if let PropertiesLine::Entry {
                key: existing_key,
                value: existing_value,
                ..
            } = line
            {
                if existing_key == key {
                    *existing_value = value;
                    return;
                }
            }
        }

        self.lines.push(PropertiesLine::Entry {
            key: key.to_string(),
            separator: '=',
            value,
        });
    }

    fn render(&self) -> String {
        let mut rendered = String::new();

        for line in &self.lines {
            match line {
                PropertiesLine::Blank => rendered.push('\n'),
                PropertiesLine::Comment(line) | PropertiesLine::Raw(line) => {
                    rendered.push_str(line);
                    rendered.push('\n');
                }
                PropertiesLine::Entry {
                    key,
                    separator,
                    value,
                } => {
                    rendered.push_str(key);
                    rendered.push(*separator);
                    rendered.push_str(value);
                    rendered.push('\n');
                }
            }
        }

        rendered
    }
}

#[cfg(test)]
mod tests {
    use super::{
        read_config_document, read_config_file, write_config_document, write_config_file,
        ConfigDocument,
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
            let path = std::env::temp_dir().join(format!("sl-libscv-doc-{}-{}", prefix, unique));
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
    fn properties_document_preserves_comments_and_updates_existing_keys() {
        let input = "# comment\nmotd=Hello\n\nview-distance=10\n";
        let mut document = match read_config_document(ConfigFormat::Properties, input)
            .expect("properties should parse")
        {
            ConfigDocument::Properties(document) => document,
            other => panic!("unexpected document: {other:?}"),
        };

        document.set("motd", "Welcome");
        document.set("spawn-protection", "16");

        let output = write_config_document(&ConfigDocument::Properties(document))
            .expect("properties should write");

        assert!(output.contains("# comment\n"));
        assert!(output.contains("motd=Welcome\n"));
        assert!(output.contains("view-distance=10\n"));
        assert!(output.contains("spawn-protection=16\n"));
    }

    #[test]
    fn toml_document_round_trips_with_toml_edit() {
        let input = "title = \"hello\"\n";
        let document = read_config_document(ConfigFormat::Toml, input).expect("toml should parse");
        let output = write_config_document(&document).expect("toml should write");

        assert_eq!(output, input);
    }

    #[test]
    fn yaml_document_keeps_raw_text_for_high_fidelity_path() {
        let input = "# top\nkey: value\n";
        let document = read_config_document(ConfigFormat::Yaml, input).expect("yaml should read");
        let output = write_config_document(&document).expect("yaml should write");

        assert_eq!(output, input);
    }

    #[test]
    fn properties_file_round_trip_updates_real_file_contents() {
        let dir = TestDir::new("properties-file");
        let path = dir
            .path()
            .join("plugins")
            .join("Demo")
            .join("config.properties");
        std::fs::create_dir_all(path.parent().expect("parent should exist")).unwrap();
        std::fs::write(&path, "# note\nname=before\n").unwrap();

        let mut document = match read_config_file(&path).expect("file should read") {
            ConfigDocument::Properties(document) => document,
            other => panic!("unexpected document: {other:?}"),
        };
        assert_eq!(document.get("name"), Some("before"));
        document.set("name", "after");
        document.set("extra", "1");

        write_config_file(&path, &ConfigDocument::Properties(document)).expect("file should write");

        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.contains("# note\n"));
        assert!(written.contains("name=after\n"));
        assert!(written.contains("extra=1\n"));
    }

    #[test]
    fn yaml_file_round_trip_preserves_raw_text() {
        let dir = TestDir::new("yaml-file");
        let path = dir.path().join("plugins").join("Demo").join("config.yml");
        std::fs::create_dir_all(path.parent().expect("parent should exist")).unwrap();
        let input = "# top\nkey: value\n";
        std::fs::write(&path, input).unwrap();

        let document = read_config_file(&path).expect("file should read");
        write_config_file(&path, &document).expect("file should write");

        let written = std::fs::read_to_string(&path).unwrap();
        assert_eq!(written, input);
    }
}
