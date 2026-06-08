use serde_json::{json, Value};
use crate::parser::{LogEntry, LogLevel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
    JsonLines,
    Csv,
    Tsv,
    PrettyJson,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "text" | "txt" => Ok(OutputFormat::Text),
            "json" => Ok(OutputFormat::Json),
            "jsonl" | "json-lines" => Ok(OutputFormat::JsonLines),
            "csv" => Ok(OutputFormat::Csv),
            "tsv" => Ok(OutputFormat::Tsv),
            "pretty-json" | "pretty" => Ok(OutputFormat::PrettyJson),
            _ => Err(format!("Unknown output format: {}", s)),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            OutputFormat::Text => "text",
            OutputFormat::Json => "json",
            OutputFormat::JsonLines => "jsonl",
            OutputFormat::Csv => "csv",
            OutputFormat::Tsv => "tsv",
            OutputFormat::PrettyJson => "pretty-json",
        }
    }
}

pub struct FormatConverter {
    output_format: OutputFormat,
    include_raw: bool,
}

impl Default for FormatConverter {
    fn default() -> Self {
        FormatConverter {
            output_format: OutputFormat::Text,
            include_raw: false,
        }
    }
}

impl FormatConverter {
    pub fn new(output_format: OutputFormat) -> Self {
        FormatConverter {
            output_format,
            include_raw: false,
        }
    }

    pub fn include_raw(mut self, include_raw: bool) -> Self {
        self.include_raw = include_raw;
        self
    }

    pub fn convert_entry(&self, entry: &LogEntry) -> String {
        match self.output_format {
            OutputFormat::Text => self.to_text(entry),
            OutputFormat::Json => self.to_json(entry),
            OutputFormat::JsonLines => self.to_json_line(entry),
            OutputFormat::Csv => self.to_csv(entry),
            OutputFormat::Tsv => self.to_tsv(entry),
            OutputFormat::PrettyJson => self.to_pretty_json(entry),
        }
    }

    pub fn convert_entries(&self, entries: &[LogEntry]) -> String {
        match self.output_format {
            OutputFormat::Json | OutputFormat::PrettyJson => {
                let json_entries: Vec<Value> = entries
                    .iter()
                    .map(|e| self.entry_to_json_value(e))
                    .collect();

                if self.output_format == OutputFormat::PrettyJson {
                    serde_json::to_string_pretty(&json_entries).unwrap_or_default()
                } else {
                    serde_json::to_string(&json_entries).unwrap_or_default()
                }
            }
            OutputFormat::Csv => {
                let mut result = String::from("timestamp,level,module,message\n");
                for entry in entries {
                    result.push_str(&self.to_csv(entry));
                    result.push('\n');
                }
                result
            }
            OutputFormat::Tsv => {
                let mut result = String::from("timestamp\tlevel\tmodule\tmessage\n");
                for entry in entries {
                    result.push_str(&self.to_tsv(entry));
                    result.push('\n');
                }
                result
            }
            _ => {
                let mut result = String::new();
                for entry in entries {
                    result.push_str(&self.convert_entry(entry));
                    result.push('\n');
                }
                result
            }
        }
    }

    fn to_text(&self, entry: &LogEntry) -> String {
        let mut parts = Vec::new();

        if let Some(ref ts) = entry.timestamp {
            parts.push(format!("[{}]", ts));
        }

        if let Some(level) = entry.level {
            parts.push(format!("[{:>7}]", level.as_str()));
        }

        if let Some(ref module) = entry.module {
            parts.push(format!("<{}>", module));
        }

        if !parts.is_empty() {
            format!("{} {}", parts.join(" "), entry.message)
        } else {
            entry.message.clone()
        }
    }

    fn entry_to_json_value(&self, entry: &LogEntry) -> Value {
        let mut obj = json!({
            "line_index": entry.line_index,
            "message": entry.message,
        });

        if let Some(ref ts) = entry.timestamp {
            obj["timestamp"] = Value::String(ts.clone());
        }

        if let Some(level) = entry.level {
            obj["level"] = Value::String(level.as_str().to_string());
        }

        if let Some(ref module) = entry.module {
            obj["module"] = Value::String(module.clone());
        }

        if let Some(line_num) = entry.line_number {
            obj["line_number"] = Value::Number(serde_json::Number::from(line_num));
        }

        if self.include_raw {
            obj["raw"] = Value::String(entry.raw.clone());
        }

        obj
    }

    fn to_json(&self, entry: &LogEntry) -> String {
        serde_json::to_string(&self.entry_to_json_value(entry)).unwrap_or_default()
    }

    fn to_pretty_json(&self, entry: &LogEntry) -> String {
        serde_json::to_string_pretty(&self.entry_to_json_value(entry)).unwrap_or_default()
    }

    fn to_json_line(&self, entry: &LogEntry) -> String {
        self.to_json(entry)
    }

    fn to_csv(&self, entry: &LogEntry) -> String {
        let timestamp = entry.timestamp.as_deref().unwrap_or("");
        let level = entry.level.map(|l| l.as_str()).unwrap_or("");
        let module = entry.module.as_deref().unwrap_or("");
        let message = escape_csv_field(&entry.message);

        format!("{},{},{},{}", timestamp, level, module, message)
    }

    fn to_tsv(&self, entry: &LogEntry) -> String {
        let timestamp = entry.timestamp.as_deref().unwrap_or("");
        let level = entry.level.map(|l| l.as_str()).unwrap_or("");
        let module = entry.module.as_deref().unwrap_or("");
        let message = escape_tsv_field(&entry.message);

        format!("{}\t{}\t{}\t{}", timestamp, level, module, message)
    }
}

fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

fn escape_tsv_field(field: &str) -> String {
    field.replace('\t', "    ")
}

pub fn level_to_color(level: Option<LogLevel>) -> String {
    match level {
        Some(LogLevel::Trace) => "gray".to_string(),
        Some(LogLevel::Debug) => "cyan".to_string(),
        Some(LogLevel::Info) => "green".to_string(),
        Some(LogLevel::Warn) => "yellow".to_string(),
        Some(LogLevel::Error) => "red".to_string(),
        Some(LogLevel::Fatal) => "magenta".to_string(),
        None => "white".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_entry() -> LogEntry {
        LogEntry {
            timestamp: Some("2024-01-01T10:00:00Z".to_string()),
            level: Some(LogLevel::Info),
            message: "Hello, world!".to_string(),
            module: Some("app".to_string()),
            line_number: Some(42),
            raw: "INFO Hello, world!".to_string(),
            line_index: 0,
        }
    }

    #[test]
    fn test_to_json() {
        let converter = FormatConverter::new(OutputFormat::Json);
        let entry = create_test_entry();
        let json_str = converter.convert_entry(&entry);
        let parsed: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["level"], "INFO");
        assert_eq!(parsed["message"], "Hello, world!");
        assert_eq!(parsed["timestamp"], "2024-01-01T10:00:00Z");
    }

    #[test]
    fn test_to_csv() {
        let converter = FormatConverter::new(OutputFormat::Csv);
        let entry = create_test_entry();
        let csv = converter.convert_entry(&entry);
        assert!(csv.contains("2024-01-01T10:00:00Z"));
        assert!(csv.contains("INFO"));
    }

    #[test]
    fn test_csv_escaping() {
        let converter = FormatConverter::new(OutputFormat::Csv);
        let entry = LogEntry {
            timestamp: None,
            level: None,
            message: "Hello, \"world\"!".to_string(),
            module: None,
            line_number: None,
            raw: "".to_string(),
            line_index: 0,
        };
        let csv = converter.convert_entry(&entry);
        assert!(csv.contains("\"Hello, \"\"world\"\"!\""));
    }
}
