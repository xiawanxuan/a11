use serde::{Deserialize, Serialize};
use regex::Regex;
use std::collections::HashMap;
use std::str::FromStr;
use crate::parser::{LogEntry, LogLevel};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRule {
    pub name: String,
    pub pattern: String,
    pub timestamp_field: Option<String>,
    pub level_field: Option<String>,
    pub message_field: Option<String>,
    pub module_field: Option<String>,
    pub line_field: Option<String>,
    #[serde(default)]
    pub level_mappings: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct CustomParser {
    rule: CustomRule,
    regex: Regex,
    named_groups: Vec<String>,
}

impl CustomParser {
    pub fn new(rule: CustomRule) -> Result<Self, String> {
        let regex = Regex::new(&rule.pattern)
            .map_err(|e| format!("Invalid regex pattern: {}", e))?;

        let named_groups = regex
            .capture_names()
            .flatten()
            .map(|s| s.to_string())
            .collect();

        Ok(CustomParser {
            rule,
            regex,
            named_groups,
        })
    }

    pub fn from_json(json_str: &str) -> Result<Self, String> {
        let rule: CustomRule = serde_json::from_str(json_str)
            .map_err(|e| format!("Invalid JSON rule: {}", e))?;
        Self::new(rule)
    }

    pub fn rule_name(&self) -> &str {
        &self.rule.name
    }

    pub fn parse_line(&self, line: &str, line_index: usize) -> Option<LogEntry> {
        let caps = self.regex.captures(line)?;

        let mut entry = LogEntry::new(line.to_string(), line_index);

        if let Some(field_name) = &self.rule.timestamp_field {
            if let Some(m) = caps.name(field_name) {
                entry.timestamp = Some(m.as_str().to_string());
            }
        }

        if let Some(field_name) = &self.rule.level_field {
            if let Some(m) = caps.name(field_name) {
                let level_str = m.as_str();
                let mapped = self.rule.level_mappings
                    .get(level_str)
                    .map(|s| s.as_str())
                    .unwrap_or(level_str);
                if let Ok(level) = LogLevel::from_str(mapped) {
                    entry.level = Some(level);
                }
            }
        }

        if let Some(field_name) = &self.rule.message_field {
            if let Some(m) = caps.name(field_name) {
                entry.message = m.as_str().to_string();
            }
        }

        if let Some(field_name) = &self.rule.module_field {
            if let Some(m) = caps.name(field_name) {
                entry.module = Some(m.as_str().to_string());
            }
        }

        if let Some(field_name) = &self.rule.line_field {
            if let Some(m) = caps.name(field_name) {
                if let Ok(num) = m.as_str().parse::<u32>() {
                    entry.line_number = Some(num);
                }
            }
        }

        Some(entry)
    }

    pub fn named_groups(&self) -> &[String] {
        &self.named_groups
    }
}

#[derive(Debug, Default)]
pub struct CustomRuleSet {
    parsers: Vec<CustomParser>,
}

impl CustomRuleSet {
    pub fn new() -> Self {
        CustomRuleSet {
            parsers: Vec::new(),
        }
    }

    pub fn add_rule(&mut self, rule: CustomRule) -> Result<(), String> {
        let parser = CustomParser::new(rule)?;
        self.parsers.push(parser);
        Ok(())
    }

    pub fn load_from_file(path: &std::path::Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read rule file: {}", e))?;

        let rules: Vec<CustomRule> = if content.trim_start().starts_with('[') {
            serde_json::from_str(&content)
                .map_err(|e| format!("Invalid JSON rules: {}", e))?
        } else {
            let rule: CustomRule = serde_json::from_str(&content)
                .map_err(|e| format!("Invalid JSON rule: {}", e))?;
            vec![rule]
        };

        let mut rule_set = Self::new();
        for rule in rules {
            rule_set.add_rule(rule)?;
        }
        Ok(rule_set)
    }

    pub fn parse_line(&self, line: &str, line_index: usize) -> Option<LogEntry> {
        for parser in &self.parsers {
            if let Some(entry) = parser.parse_line(line, line_index) {
                return Some(entry);
            }
        }
        None
    }

    pub fn is_empty(&self) -> bool {
        self.parsers.is_empty()
    }

    pub fn parser_count(&self) -> usize {
        self.parsers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_rule() -> CustomRule {
        CustomRule {
            name: "test".to_string(),
            pattern: r"^\[(?<time>[^\]]+)\]\s+(?<level>\w+)\s+<(?<module>[^>]+)>\s+(?<msg>.*)$".to_string(),
            timestamp_field: Some("time".to_string()),
            level_field: Some("level".to_string()),
            message_field: Some("msg".to_string()),
            module_field: Some("module".to_string()),
            line_field: None,
            level_mappings: HashMap::new(),
        }
    }

    #[test]
    fn test_custom_parser() {
        let parser = CustomParser::new(test_rule()).unwrap();
        let line = "[2024-01-15 10:00:00] INFO <app> test message";
        let entry = parser.parse_line(line, 0).unwrap();

        assert_eq!(entry.timestamp, Some("2024-01-15 10:00:00".to_string()));
        assert_eq!(entry.level, Some(LogLevel::Info));
        assert_eq!(entry.module, Some("app".to_string()));
        assert_eq!(entry.message, "test message");
    }

    #[test]
    fn test_level_mapping() {
        let mut rule = test_rule();
        rule.level_mappings.insert("WARNING".to_string(), "WARN".to_string());

        let parser = CustomParser::new(rule).unwrap();
        let line = "[2024-01-15 10:00:00] WARNING <app> test";
        let entry = parser.parse_line(line, 0).unwrap();

        assert_eq!(entry.level, Some(LogLevel::Warn));
    }

    #[test]
    fn test_no_match() {
        let parser = CustomParser::new(test_rule()).unwrap();
        let entry = parser.parse_line("not a valid log line", 0);
        assert!(entry.is_none());
    }
}
