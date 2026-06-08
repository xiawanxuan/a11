use std::collections::HashMap;
use crate::parser::{LogEntry, LogLevel};

#[derive(Debug, Clone)]
pub struct LogFilter {
    levels: Option<Vec<LogLevel>>,
    min_level: Option<LogLevel>,
    module_filter: Option<String>,
    case_sensitive: bool,
}

impl Default for LogFilter {
    fn default() -> Self {
        LogFilter {
            levels: None,
            min_level: None,
            module_filter: None,
            case_sensitive: false,
        }
    }
}

impl LogFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_levels(mut self, levels: Vec<LogLevel>) -> Self {
        self.levels = Some(levels);
        self
    }

    pub fn with_min_level(mut self, level: LogLevel) -> Self {
        self.min_level = Some(level);
        self
    }

    pub fn with_module(mut self, module: String) -> Self {
        self.module_filter = Some(module);
        self
    }

    pub fn case_sensitive(mut self, case_sensitive: bool) -> Self {
        self.case_sensitive = case_sensitive;
        self
    }

    pub fn matches(&self, entry: &LogEntry) -> bool {
        if let Some(ref levels) = self.levels {
            if let Some(level) = entry.level {
                if !levels.contains(&level) {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(min_level) = self.min_level {
            if let Some(level) = entry.level {
                if level.priority() < min_level.priority() {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(ref module_filter) = self.module_filter {
            if let Some(ref module) = entry.module {
                if self.case_sensitive {
                    if !module.contains(module_filter) {
                        return false;
                    }
                } else {
                    if !module.to_lowercase().contains(&module_filter.to_lowercase()) {
                        return false;
                    }
                }
            } else {
                return false;
            }
        }

        true
    }

    pub fn filter_entries<'a>(&self, entries: &'a [LogEntry]) -> Vec<&'a LogEntry> {
        entries.iter().filter(|e| self.matches(e)).collect()
    }
}

#[derive(Debug, Clone, Default)]
pub struct LogStats {
    pub total_lines: usize,
    pub parsed_lines: usize,
    pub level_counts: HashMap<LogLevel, usize>,
    pub modules: HashMap<String, usize>,
}

impl LogStats {
    pub fn new() -> Self {
        let mut level_counts = HashMap::new();
        for level in LogLevel::all() {
            level_counts.insert(level, 0);
        }
        LogStats {
            total_lines: 0,
            parsed_lines: 0,
            level_counts,
            modules: HashMap::new(),
        }
    }

    pub fn add_entry(&mut self, entry: &LogEntry) {
        self.total_lines += 1;
        if entry.level.is_some() {
            self.parsed_lines += 1;
        }

        if let Some(level) = entry.level {
            *self.level_counts.entry(level).or_insert(0) += 1;
        }

        if let Some(ref module) = entry.module {
            *self.modules.entry(module.clone()).or_insert(0) += 1;
        }
    }

    pub fn merge(&mut self, other: &LogStats) {
        self.total_lines += other.total_lines;
        self.parsed_lines += other.parsed_lines;

        for (level, count) in &other.level_counts {
            *self.level_counts.entry(*level).or_insert(0) += count;
        }

        for (module, count) in &other.modules {
            *self.modules.entry(module.clone()).or_insert(0) += count;
        }
    }

    pub fn print_summary(&self) {
        println!("=== Log Statistics ===");
        println!("Total lines: {}", self.total_lines);
        println!("Parsed lines: {}", self.parsed_lines);
        println!();
        println!("Level breakdown:");

        let mut levels: Vec<(&LogLevel, &usize)> = self.level_counts.iter().collect();
        levels.sort_by_key(|(l, _)| l.priority());

        for (level, count) in &levels {
            let percentage = if self.total_lines > 0 {
                (**count as f64 / self.total_lines as f64) * 100.0
            } else {
                0.0
            };
            println!("  {:>7}: {:>8} ({:>5.1}%)", level.as_str(), count, percentage);
        }

        if !self.modules.is_empty() {
            println!();
            println!("Top modules:");
            let mut modules: Vec<(&String, &usize)> = self.modules.iter().collect();
            modules.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
            for (module, count) in modules.iter().take(10) {
                println!("  {:<30} {}", module, count);
            }
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        let level_counts: std::collections::HashMap<String, usize> = self
            .level_counts
            .iter()
            .map(|(k, v)| (k.as_str().to_string(), *v))
            .collect();

        serde_json::json!({
            "total_lines": self.total_lines,
            "parsed_lines": self.parsed_lines,
            "level_counts": level_counts,
            "modules": self.modules,
        })
    }
}

pub fn compute_stats(entries: &[LogEntry]) -> LogStats {
    let mut stats = LogStats::new();
    for entry in entries {
        stats.add_entry(entry);
    }
    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_entries() -> Vec<LogEntry> {
        vec![
            LogEntry {
                timestamp: Some("2024-01-01 10:00:00".to_string()),
                level: Some(LogLevel::Info),
                message: "info message".to_string(),
                module: Some("app".to_string()),
                line_number: None,
                raw: "INFO info message".to_string(),
                line_index: 0,
            },
            LogEntry {
                timestamp: Some("2024-01-01 10:00:01".to_string()),
                level: Some(LogLevel::Error),
                message: "error message".to_string(),
                module: Some("db".to_string()),
                line_number: None,
                raw: "ERROR error message".to_string(),
                line_index: 1,
            },
            LogEntry {
                timestamp: Some("2024-01-01 10:00:02".to_string()),
                level: Some(LogLevel::Debug),
                message: "debug message".to_string(),
                module: Some("app".to_string()),
                line_number: None,
                raw: "DEBUG debug message".to_string(),
                line_index: 2,
            },
        ]
    }

    #[test]
    fn test_filter_by_level() {
        let entries = create_test_entries();
        let filter = LogFilter::new().with_levels(vec![LogLevel::Error, LogLevel::Info]);
        let filtered = filter.filter_entries(&entries);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_min_level() {
        let entries = create_test_entries();
        let filter = LogFilter::new().with_min_level(LogLevel::Warn);
        let filtered = filter.filter_entries(&entries);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].level, Some(LogLevel::Error));
    }

    #[test]
    fn test_stats_computation() {
        let entries = create_test_entries();
        let stats = compute_stats(&entries);
        assert_eq!(stats.total_lines, 3);
        assert_eq!(stats.parsed_lines, 3);
        assert_eq!(*stats.level_counts.get(&LogLevel::Info).unwrap(), 1);
        assert_eq!(*stats.level_counts.get(&LogLevel::Error).unwrap(), 1);
        assert_eq!(*stats.level_counts.get(&LogLevel::Debug).unwrap(), 1);
    }
}
