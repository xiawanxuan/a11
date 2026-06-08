use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::parser::{LogEntry, LogLevel, LogParser};
use crate::filter::LogFilter;
use crate::search::Searcher;
use crate::color;

#[derive(Debug, Clone)]
pub struct AlertRule {
    pub name: String,
    pub pattern: Option<String>,
    pub level: Option<LogLevel>,
    pub min_count: usize,
    pub window_seconds: u64,
    pub cooldown_seconds: u64,
}

impl Default for AlertRule {
    fn default() -> Self {
        AlertRule {
            name: "default".to_string(),
            pattern: None,
            level: Some(LogLevel::Error),
            min_count: 1,
            window_seconds: 60,
            cooldown_seconds: 300,
        }
    }
}

struct AlertState {
    _rule_name: String,
    last_triggered: Option<Instant>,
    hit_times: Vec<Instant>,
}

impl AlertState {
    fn new(rule_name: String) -> Self {
        AlertState {
            _rule_name: rule_name,
            last_triggered: None,
            hit_times: Vec::new(),
        }
    }

    fn check_and_trigger(&mut self, rule: &AlertRule, now: Instant) -> bool {
        self.hit_times.push(now);

        let window = Duration::from_secs(rule.window_seconds);
        self.hit_times.retain(|&t| now.duration_since(t) < window);

        if self.hit_times.len() >= rule.min_count {
            let cooldown = Duration::from_secs(rule.cooldown_seconds);
            let should_trigger = match self.last_triggered {
                None => true,
                Some(last) => now.duration_since(last) >= cooldown,
            };

            if should_trigger {
                self.last_triggered = Some(now);
                self.hit_times.clear();
                return true;
            }
        }

        false
    }
}

pub struct LogMonitor {
    file_path: PathBuf,
    parser: LogParser,
    filter: Option<LogFilter>,
    searcher: Option<Searcher>,
    alert_rules: Vec<AlertRule>,
    alert_states: Vec<AlertState>,
    poll_interval: Duration,
    follow: bool,
    running: Arc<AtomicBool>,
}

impl LogMonitor {
    pub fn new<P: AsRef<Path>>(file_path: P, parser: LogParser) -> Self {
        LogMonitor {
            file_path: file_path.as_ref().to_path_buf(),
            parser,
            filter: None,
            searcher: None,
            alert_rules: Vec::new(),
            alert_states: Vec::new(),
            poll_interval: Duration::from_millis(500),
            follow: true,
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn with_filter(mut self, filter: LogFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn with_searcher(mut self, searcher: Searcher) -> Self {
        self.searcher = Some(searcher);
        self
    }

    pub fn add_alert_rule(&mut self, rule: AlertRule) {
        self.alert_states.push(AlertState::new(rule.name.clone()));
        self.alert_rules.push(rule);
    }

    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    pub fn with_follow(mut self, follow: bool) -> Self {
        self.follow = follow;
        self
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn running_clone(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }

    pub fn start<F>(&mut self, mut callback: F) -> Result<usize, String>
    where
        F: FnMut(&LogEntry, bool),
    {
        let file = File::open(&self.file_path)
            .map_err(|e| format!("Failed to open file: {}", e))?;

        let mut reader = BufReader::new(file);
        let mut line_index = 0;

        if self.alert_states.is_empty() {
            for rule in &self.alert_rules {
                self.alert_states.push(AlertState::new(rule.name.clone()));
            }
        }

        while self.running.load(Ordering::SeqCst) {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    if !self.follow {
                        break;
                    }
                    std::thread::sleep(self.poll_interval);
                    continue;
                }
                Ok(_) => {
                    let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
                    if !trimmed.is_empty() {
                        let entry = self.parser.parse_line(trimmed, line_index);

                        let passes_filter = self.filter
                            .as_ref()
                            .map(|f| f.matches(&entry))
                            .unwrap_or(true);

                        let passes_search = self.searcher
                            .as_ref()
                            .map(|s| s.matches_entry(&entry))
                            .unwrap_or(true);

                        if passes_filter && passes_search {
                            let is_alert = self.check_alerts(&entry);
                            callback(&entry, is_alert);
                        }
                    }
                    line_index += 1;
                    line.clear();
                }
                Err(e) => {
                    return Err(format!("Read error: {}", e));
                }
            }
        }

        Ok(line_index)
    }

    fn check_alerts(&mut self, entry: &LogEntry) -> bool {
        if self.alert_rules.is_empty() {
            return false;
        }

        let now = Instant::now();
        let mut triggered = false;

        for (i, rule) in self.alert_rules.iter().enumerate() {
            let mut match_rule = true;

            if let Some(level) = rule.level {
                if entry.level.map(|l| l.priority() < level.priority()).unwrap_or(true) {
                    match_rule = false;
                }
            }

            if let Some(ref pattern) = rule.pattern {
                if !entry.message.contains(pattern) && !entry.raw.contains(pattern) {
                    match_rule = false;
                }
            }

            if match_rule {
                if i < self.alert_states.len() {
                    if self.alert_states[i].check_and_trigger(rule, now) {
                        triggered = true;
                        self.print_alert(rule, entry);
                    }
                }
            }
        }

        triggered
    }

    fn print_alert(&self, rule: &AlertRule, entry: &LogEntry) {
        eprintln!();
        eprintln!("{}", color::red_bold("╔══════════════════════════════════════════════╗"));
        eprintln!("{}", color::red_bold(&format!("║  ALERT: {:<36}║", rule.name)));
        eprintln!("{}", color::red_bold("╠══════════════════════════════════════════════╣"));
        eprintln!("{}", color::red_bold(&format!("║  Level: {:<35}║",
            entry.level.map(|l| l.as_str()).unwrap_or("UNKNOWN"))));
        if let Some(ref ts) = entry.timestamp {
            eprintln!("{}", color::red_bold(&format!("║  Time: {:<36}║", ts)));
        }
        eprintln!("{}", color::red_bold("╠══════════════════════════════════════════════╣"));
        eprintln!("  {}", entry.message);
        eprintln!("{}", color::red_bold("╚══════════════════════════════════════════════╝"));
        eprintln!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_state_basic() {
        let mut state = AlertState::new("test".to_string());
        let rule = AlertRule {
            name: "test".to_string(),
            pattern: None,
            level: None,
            min_count: 1,
            window_seconds: 60,
            cooldown_seconds: 0,
        };

        let now = Instant::now();
        assert!(state.check_and_trigger(&rule, now));
    }

    #[test]
    fn test_alert_state_cooldown() {
        let mut state = AlertState::new("test".to_string());
        let rule = AlertRule {
            name: "test".to_string(),
            pattern: None,
            level: None,
            min_count: 1,
            window_seconds: 60,
            cooldown_seconds: 10,
        };

        let now = Instant::now();
        assert!(state.check_and_trigger(&rule, now));
        assert!(!state.check_and_trigger(&rule, now));
    }

    #[test]
    fn test_alert_state_window_count() {
        let mut state = AlertState::new("test".to_string());
        let rule = AlertRule {
            name: "test".to_string(),
            pattern: None,
            level: None,
            min_count: 3,
            window_seconds: 60,
            cooldown_seconds: 0,
        };

        let now = Instant::now();
        assert!(!state.check_and_trigger(&rule, now));
        assert!(!state.check_and_trigger(&rule, now));
        assert!(state.check_and_trigger(&rule, now));
    }
}
