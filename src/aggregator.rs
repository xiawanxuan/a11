use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde_json::json;

use crate::parser::{LogEntry, LogLevel};
use crate::color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateField {
    Level,
    Module,
    Hour,
    Minute,
    StatusCode,
}

impl AggregateField {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "level" => Ok(AggregateField::Level),
            "module" => Ok(AggregateField::Module),
            "hour" => Ok(AggregateField::Hour),
            "minute" => Ok(AggregateField::Minute),
            "status" | "statuscode" | "status_code" => Ok(AggregateField::StatusCode),
            _ => Err(format!("Unknown aggregate field: {}", s)),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            AggregateField::Level => "level",
            AggregateField::Module => "module",
            AggregateField::Hour => "hour",
            AggregateField::Minute => "minute",
            AggregateField::StatusCode => "status_code",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Aggregation {
    pub field: AggregateField,
    pub counts: HashMap<String, usize>,
}

impl Aggregation {
    pub fn new(field: AggregateField) -> Self {
        Aggregation {
            field,
            counts: HashMap::new(),
        }
    }

    pub fn add_entry(&mut self, entry: &LogEntry) {
        let key = self.extract_key(entry);
        *self.counts.entry(key).or_insert(0) += 1;
    }

    fn extract_key(&self, entry: &LogEntry) -> String {
        match self.field {
            AggregateField::Level => {
                entry.level.map(|l| l.as_str().to_string()).unwrap_or_else(|| "UNKNOWN".to_string())
            }
            AggregateField::Module => {
                entry.module.clone().unwrap_or_else(|| "unknown".to_string())
            }
            AggregateField::Hour => {
                Self::extract_time_component(entry, 1)
            }
            AggregateField::Minute => {
                Self::extract_time_component(entry, 2)
            }
            AggregateField::StatusCode => {
                Self::extract_status_code(entry)
            }
        }
    }

    fn extract_time_component(entry: &LogEntry, component_index: usize) -> String {
        entry.timestamp.as_ref()
            .and_then(|ts| {
                let parts: Vec<&str> = ts.split(|c| c == ':' || c == ' ' || c == 'T').collect();
                if component_index < parts.len() {
                    Some(parts[component_index].to_string())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "unknown".to_string())
    }

    fn extract_status_code(entry: &LogEntry) -> String {
        use regex::Regex;
        let re = Regex::new(r"\b(\d{3})\b").unwrap();
        re.captures(&entry.message)
            .or_else(|| re.captures(&entry.raw))
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }

    pub fn sorted_by_count(&self) -> Vec<(String, usize)> {
        let mut items: Vec<(String, usize)> = self.counts
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        items.sort_by(|a, b| b.1.cmp(&a.1));
        items
    }

    pub fn total(&self) -> usize {
        self.counts.values().sum()
    }

    pub fn print_table(&self, title: &str) {
        println!("\n{}", color::bold(title));
        println!("{}", "-".repeat(50));

        let items = self.sorted_by_count();
        let total = self.total() as f64;

        for (key, count) in &items {
            let percentage = if total > 0.0 {
                (*count as f64 / total) * 100.0
            } else {
                0.0
            };
            println!("  {:<25} {:>8} ({:>5.1}%)", key, count, percentage);
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReportGenerator {
    aggregations: Vec<Aggregation>,
    total_entries: usize,
    error_entries: usize,
    warning_entries: usize,
    start_time: Option<Instant>,
}

impl ReportGenerator {
    pub fn new() -> Self {
        ReportGenerator {
            aggregations: Vec::new(),
            total_entries: 0,
            error_entries: 0,
            warning_entries: 0,
            start_time: Some(Instant::now()),
        }
    }

    pub fn with_fields(fields: &[AggregateField]) -> Self {
        let mut gen = Self::new();
        for field in fields {
            gen.add_aggregation(Aggregation::new(*field));
        }
        gen
    }

    pub fn add_aggregation(&mut self, agg: Aggregation) {
        self.aggregations.push(agg);
    }

    pub fn add_entry(&mut self, entry: &LogEntry) {
        self.total_entries += 1;

        if let Some(level) = entry.level {
            match level {
                LogLevel::Error | LogLevel::Fatal => self.error_entries += 1,
                LogLevel::Warn => self.warning_entries += 1,
                _ => {}
            }
        }

        for agg in &mut self.aggregations {
            agg.add_entry(entry);
        }
    }

    pub fn add_entries(&mut self, entries: &[LogEntry]) {
        for entry in entries {
            self.add_entry(entry);
        }
    }

    pub fn total_entries(&self) -> usize {
        self.total_entries
    }

    pub fn error_rate(&self) -> f64 {
        if self.total_entries > 0 {
            self.error_entries as f64 / self.total_entries as f64 * 100.0
        } else {
            0.0
        }
    }

    pub fn warning_rate(&self) -> f64 {
        if self.total_entries > 0 {
            self.warning_entries as f64 / self.total_entries as f64 * 100.0
        } else {
            0.0
        }
    }

    pub fn print_summary(&self) {
        let elapsed = self.start_time
            .map(|t| t.elapsed())
            .unwrap_or(Duration::from_secs(0));

        println!("{}", color::bold("╔══════════════════════════════════════════════════════╗"));
        println!("{}", color::bold("║           LOG ANALYSIS REPORT SUMMARY               ║"));
        println!("{}", color::bold("╠══════════════════════════════════════════════════════╣"));
        println!("  Total log entries:      {:>10}", self.total_entries);
        println!("  Error entries:          {:>10} ({:.2}%)", self.error_entries, self.error_rate());
        println!("  Warning entries:        {:>10} ({:.2}%)", self.warning_entries, self.warning_rate());
        println!("  Processing time:        {:>10?}", elapsed);
        println!("{}", color::bold("╚══════════════════════════════════════════════════════╝"));

        for agg in &self.aggregations {
            agg.print_table(&format!("\nAggregation by {}", agg.field.as_str()));
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        let mut aggregations_json = Vec::new();
        for agg in &self.aggregations {
            let mut counts = serde_json::Map::new();
            for (k, v) in &agg.counts {
                counts.insert(k.clone(), json!(v));
            }
            aggregations_json.push(json!({
                "field": agg.field.as_str(),
                "counts": counts,
            }));
        }

        json!({
            "summary": {
                "total_entries": self.total_entries,
                "error_entries": self.error_entries,
                "warning_entries": self.warning_entries,
                "error_rate": self.error_rate(),
                "warning_rate": self.warning_rate(),
            },
            "aggregations": aggregations_json,
        })
    }

    pub fn to_html(&self) -> String {
        let mut html = String::new();
        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str("<title>Log Analysis Report</title>\n");
        html.push_str("<style>\n");
        html.push_str("body { font-family: Arial, sans-serif; margin: 20px; }\n");
        html.push_str("table { border-collapse: collapse; width: 100%; margin-bottom: 20px; }\n");
        html.push_str("th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }\n");
        html.push_str("th { background-color: #4CAF50; color: white; }\n");
        html.push_str(".error { color: red; }\n");
        html.push_str(".warning { color: orange; }\n");
        html.push_str("h1, h2 { color: #333; }\n");
        html.push_str("</style>\n</head>\n<body>\n");

        html.push_str("<h1>Log Analysis Report</h1>\n");
        html.push_str("<h2>Summary</h2>\n");
        html.push_str("<table>\n");
        html.push_str("<tr><th>Metric</th><th>Value</th></tr>\n");
        html.push_str(&format!("<tr><td>Total entries</td><td>{}</td></tr>\n", self.total_entries));
        html.push_str(&format!("<tr><td class=\"error\">Error entries</td><td class=\"error\">{} ({:.2}%)</td></tr>\n",
            self.error_entries, self.error_rate()));
        html.push_str(&format!("<tr><td class=\"warning\">Warning entries</td><td class=\"warning\">{} ({:.2}%)</td></tr>\n",
            self.warning_entries, self.warning_rate()));
        html.push_str("</table>\n");

        for agg in &self.aggregations {
            html.push_str(&format!("<h2>Aggregation by {}</h2>\n", agg.field.as_str()));
            html.push_str("<table>\n");
            html.push_str("<tr><th>Key</th><th>Count</th><th>Percentage</th></tr>\n");

            let items = agg.sorted_by_count();
            let total = agg.total() as f64;

            for (key, count) in &items {
                let percentage = if total > 0.0 {
                    (*count as f64 / total) * 100.0
                } else {
                    0.0
                };
                html.push_str(&format!("<tr><td>{}</td><td>{}</td><td>{:.2}%</td></tr>\n",
                    key, count, percentage));
            }
            html.push_str("</table>\n");
        }

        html.push_str("</body>\n</html>\n");
        html
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str("# Log Analysis Report\n\n");
        md.push_str("## Summary\n\n");
        md.push_str("| Metric | Value |\n");
        md.push_str("|--------|-------|\n");
        md.push_str(&format!("| Total entries | {} |\n", self.total_entries));
        md.push_str(&format!("| Error entries | {} ({:.2}%) |\n", self.error_entries, self.error_rate()));
        md.push_str(&format!("| Warning entries | {} ({:.2}%) |\n", self.warning_entries, self.warning_rate()));
        md.push_str("\n");

        for agg in &self.aggregations {
            md.push_str(&format!("## Aggregation by {}\n\n", agg.field.as_str()));
            md.push_str("| Key | Count | Percentage |\n");
            md.push_str("|-----|-------|------------|\n");

            let items = agg.sorted_by_count();
            let total = agg.total() as f64;

            for (key, count) in &items {
                let percentage = if total > 0.0 {
                    (*count as f64 / total) * 100.0
                } else {
                    0.0
                };
                md.push_str(&format!("| {} | {} | {:.2}% |\n", key, count, percentage));
            }
            md.push_str("\n");
        }

        md
    }
}

impl Default for ReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::LogEntry;

    fn test_entries() -> Vec<LogEntry> {
        vec![
            LogEntry {
                timestamp: Some("2024-01-15 10:00:00".to_string()),
                level: Some(LogLevel::Info),
                message: "GET /api/users 200".to_string(),
                module: Some("api".to_string()),
                line_number: None,
                raw: "INFO GET /api/users 200".to_string(),
                line_index: 0,
            },
            LogEntry {
                timestamp: Some("2024-01-15 10:01:00".to_string()),
                level: Some(LogLevel::Error),
                message: "Database error 500".to_string(),
                module: Some("database".to_string()),
                line_number: None,
                raw: "ERROR Database error 500".to_string(),
                line_index: 1,
            },
            LogEntry {
                timestamp: Some("2024-01-15 11:00:00".to_string()),
                level: Some(LogLevel::Warn),
                message: "Slow query 408".to_string(),
                module: Some("database".to_string()),
                line_number: None,
                raw: "WARN Slow query 408".to_string(),
                line_index: 2,
            },
        ]
    }

    #[test]
    fn test_aggregation_by_level() {
        let mut agg = Aggregation::new(AggregateField::Level);
        for entry in &test_entries() {
            agg.add_entry(entry);
        }

        assert_eq!(*agg.counts.get("INFO").unwrap(), 1);
        assert_eq!(*agg.counts.get("ERROR").unwrap(), 1);
        assert_eq!(*agg.counts.get("WARN").unwrap(), 1);
        assert_eq!(agg.total(), 3);
    }

    #[test]
    fn test_aggregation_by_module() {
        let mut agg = Aggregation::new(AggregateField::Module);
        for entry in &test_entries() {
            agg.add_entry(entry);
        }

        assert_eq!(*agg.counts.get("api").unwrap(), 1);
        assert_eq!(*agg.counts.get("database").unwrap(), 2);
    }

    #[test]
    fn test_report_generator() {
        let mut report = ReportGenerator::with_fields(&[AggregateField::Level]);
        report.add_entries(&test_entries());

        assert_eq!(report.total_entries(), 3);
        assert_eq!(report.error_entries, 1);
        assert_eq!(report.warning_entries, 1);
        assert!((report.error_rate() - 33.33).abs() < 0.1);
    }

    #[test]
    fn test_sorted_by_count() {
        let mut agg = Aggregation::new(AggregateField::Module);
        for entry in &test_entries() {
            agg.add_entry(entry);
        }

        let sorted = agg.sorted_by_count();
        assert_eq!(sorted[0].0, "database");
        assert_eq!(sorted[1].0, "api");
    }

    #[test]
    fn test_to_json() {
        let mut report = ReportGenerator::with_fields(&[AggregateField::Level]);
        report.add_entries(&test_entries());
        let json = report.to_json();
        assert!(json["summary"]["total_entries"].as_i64().unwrap() == 3);
    }

    #[test]
    fn test_to_markdown() {
        let mut report = ReportGenerator::with_fields(&[AggregateField::Level]);
        report.add_entries(&test_entries());
        let md = report.to_markdown();
        assert!(md.contains("# Log Analysis Report"));
        assert!(md.contains("## Summary"));
    }
}
