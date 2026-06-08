use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl LogLevel {
    pub fn all() -> Vec<LogLevel> {
        vec![
            LogLevel::Trace,
            LogLevel::Debug,
            LogLevel::Info,
            LogLevel::Warn,
            LogLevel::Error,
            LogLevel::Fatal,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Fatal => "FATAL",
        }
    }

    pub fn priority(&self) -> u8 {
        match self {
            LogLevel::Trace => 0,
            LogLevel::Debug => 1,
            LogLevel::Info => 2,
            LogLevel::Warn => 3,
            LogLevel::Error => 4,
            LogLevel::Fatal => 5,
        }
    }
}

impl FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "TRACE" => Ok(LogLevel::Trace),
            "DEBUG" => Ok(LogLevel::Debug),
            "INFO" => Ok(LogLevel::Info),
            "WARN" | "WARNING" => Ok(LogLevel::Warn),
            "ERROR" | "ERR" => Ok(LogLevel::Error),
            "FATAL" | "CRITICAL" => Ok(LogLevel::Fatal),
            _ => Err(format!("Unknown log level: {}", s)),
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: Option<String>,
    pub level: Option<LogLevel>,
    pub message: String,
    pub module: Option<String>,
    pub line_number: Option<u32>,
    pub raw: String,
    pub line_index: usize,
}

impl LogEntry {
    pub fn new(raw: String, line_index: usize) -> Self {
        LogEntry {
            timestamp: None,
            level: None,
            message: raw.clone(),
            module: None,
            line_number: None,
            raw,
            line_index,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogParser {
    format: LogFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogFormat {
    Auto,
    Common,
    Json,
    Simple,
    Nginx,
    Apache,
}

impl LogFormat {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(LogFormat::Auto),
            "common" => Ok(LogFormat::Common),
            "json" => Ok(LogFormat::Json),
            "simple" => Ok(LogFormat::Simple),
            "nginx" => Ok(LogFormat::Nginx),
            "apache" => Ok(LogFormat::Apache),
            _ => Err(format!("Unknown log format: {}", s)),
        }
    }
}

impl Default for LogParser {
    fn default() -> Self {
        Self::new(LogFormat::Auto)
    }
}

impl LogParser {
    pub fn new(format: LogFormat) -> Self {
        LogParser { format }
    }

    pub fn parse_line(&self, line: &str, line_index: usize) -> LogEntry {
        let raw = line.to_string();
        let mut entry = LogEntry::new(raw, line_index);

        match self.format {
            LogFormat::Auto => self.parse_auto(&mut entry, line),
            LogFormat::Common => {
                self.parse_common(&mut entry, line);
            }
            LogFormat::Json => {
                self.parse_json(&mut entry, line);
            }
            LogFormat::Simple => {
                self.parse_simple(&mut entry, line);
            }
            LogFormat::Nginx => {
                self.parse_nginx(&mut entry, line);
            }
            LogFormat::Apache => {
                self.parse_apache(&mut entry, line);
            }
        }

        entry
    }

    fn parse_auto(&self, entry: &mut LogEntry, line: &str) {
        if line.trim_start().starts_with('{') {
            if self.parse_json(entry, line) {
                return;
            }
        }
        if self.parse_common(entry, line) {
            return;
        }
        if self.parse_simple(entry, line) {
            return;
        }
    }

    fn parse_common(&self, entry: &mut LogEntry, line: &str) -> bool {
        let re = regex::Regex::new(
            r"^\[?(\d{4}[-/]\d{2}[-/]\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:?\d{2})?)\]?\s*[\[\s]?([A-Z]{4,7})[\]\s:]+\s*(.*)$"
        ).unwrap();

        if let Some(caps) = re.captures(line) {
            entry.timestamp = Some(caps.get(1).unwrap().as_str().to_string());
            if let Ok(level) = LogLevel::from_str(caps.get(2).unwrap().as_str()) {
                entry.level = Some(level);
            }
            entry.message = caps.get(3).unwrap().as_str().to_string();
            true
        } else {
            false
        }
    }

    fn parse_json(&self, entry: &mut LogEntry, line: &str) -> bool {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line.trim()) {
            if let Some(obj) = value.as_object() {
                entry.timestamp = obj.get("time")
                    .or_else(|| obj.get("timestamp"))
                    .or_else(|| obj.get("@timestamp"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                entry.level = obj.get("level")
                    .or_else(|| obj.get("log_level"))
                    .or_else(|| obj.get("severity"))
                    .and_then(|v| v.as_str())
                    .and_then(|s| LogLevel::from_str(s).ok());

                entry.message = obj.get("msg")
                    .or_else(|| obj.get("message"))
                    .or_else(|| obj.get("log"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| line.to_string());

                entry.module = obj.get("module")
                    .or_else(|| obj.get("logger"))
                    .or_else(|| obj.get("target"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                entry.line_number = obj.get("line")
                    .or_else(|| obj.get("line_number"))
                    .and_then(|v| v.as_u64())
                    .map(|n| n as u32);

                return true;
            }
        }
        false
    }

    fn parse_simple(&self, entry: &mut LogEntry, line: &str) -> bool {
        let re = regex::Regex::new(r"^(TRACE|DEBUG|INFO|WARN|WARNING|ERROR|ERR|FATAL|CRITICAL)[\s:]+(.*)$").unwrap();

        if let Some(caps) = re.captures(line) {
            if let Ok(level) = LogLevel::from_str(caps.get(1).unwrap().as_str()) {
                entry.level = Some(level);
            }
            entry.message = caps.get(2).unwrap().as_str().to_string();
            true
        } else {
            false
        }
    }

    fn parse_nginx(&self, entry: &mut LogEntry, line: &str) -> bool {
        let re = regex::Regex::new(
            r#"^(\S+)\s+\S+\s+\S+\s+\[([^\]]+)\]\s+"(\S+)\s+(\S+)\s+\S+"\s+(\d{3})\s+(\d+)"#
        ).unwrap();

        if let Some(caps) = re.captures(line) {
            let status = caps.get(5).unwrap().as_str().parse::<u16>().unwrap_or(0);
            entry.level = Some(if status >= 500 {
                LogLevel::Error
            } else if status >= 400 {
                LogLevel::Warn
            } else {
                LogLevel::Info
            });
            entry.timestamp = Some(caps.get(2).unwrap().as_str().to_string());
            entry.message = format!(
                "{} {} {} {}",
                caps.get(1).unwrap().as_str(),
                caps.get(3).unwrap().as_str(),
                caps.get(4).unwrap().as_str(),
                status
            );
            true
        } else {
            false
        }
    }

    fn parse_apache(&self, entry: &mut LogEntry, line: &str) -> bool {
        let re = regex::Regex::new(
            r#"^(\S+)\s+\S+\s+\S+\s+\[([^\]]+)\]\s+"(\S+)\s+(\S+)\s+\S+"\s+(\d{3})\s+(\d+)"#
        ).unwrap();

        if let Some(caps) = re.captures(line) {
            let status = caps.get(5).unwrap().as_str().parse::<u16>().unwrap_or(0);
            entry.level = Some(if status >= 500 {
                LogLevel::Error
            } else if status >= 400 {
                LogLevel::Warn
            } else {
                LogLevel::Info
            });
            entry.timestamp = Some(caps.get(2).unwrap().as_str().to_string());
            entry.message = format!(
                "{} {} {} {}",
                caps.get(1).unwrap().as_str(),
                caps.get(3).unwrap().as_str(),
                caps.get(4).unwrap().as_str(),
                status
            );
            true
        } else {
            false
        }
    }

    pub fn detect_format(&self, sample_lines: &[String]) -> LogFormat {
        let mut scores = std::collections::HashMap::new();
        scores.insert(LogFormat::Json, 0);
        scores.insert(LogFormat::Common, 0);
        scores.insert(LogFormat::Simple, 0);
        scores.insert(LogFormat::Nginx, 0);
        scores.insert(LogFormat::Apache, 0);

        for line in sample_lines.iter().take(20) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if line.starts_with('{') && serde_json::from_str::<serde_json::Value>(line).is_ok() {
                *scores.get_mut(&LogFormat::Json).unwrap() += 1;
            }

            let mut test_entry = LogEntry::new(line.to_string(), 0);
            if self.parse_common(&mut test_entry, line) {
                *scores.get_mut(&LogFormat::Common).unwrap() += 1;
            }
            if self.parse_simple(&mut test_entry, line) {
                *scores.get_mut(&LogFormat::Simple).unwrap() += 1;
            }
            if self.parse_nginx(&mut test_entry, line) {
                *scores.get_mut(&LogFormat::Nginx).unwrap() += 1;
            }
            if self.parse_apache(&mut test_entry, line) {
                *scores.get_mut(&LogFormat::Apache).unwrap() += 1;
            }
        }

        scores.into_iter()
            .max_by_key(|(_, v)| *v)
            .map(|(k, _)| k)
            .unwrap_or(LogFormat::Common)
    }
}
