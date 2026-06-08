use regex::Regex;
use crate::parser::LogEntry;
use crate::color;

#[derive(Debug, Clone)]
pub struct Searcher {
    patterns: Vec<SearchPattern>,
    case_sensitive: bool,
    use_regex: bool,
    highlight: bool,
}

#[derive(Debug, Clone)]
struct SearchPattern {
    text: String,
    regex: Option<Regex>,
    is_inverted: bool,
}

impl Searcher {
    pub fn new(patterns: Vec<String>) -> Self {
        let search_patterns = patterns
            .iter()
            .map(|p| SearchPattern {
                text: p.clone(),
                regex: None,
                is_inverted: false,
            })
            .collect();

        Searcher {
            patterns: search_patterns,
            case_sensitive: false,
            use_regex: false,
            highlight: true,
        }
    }

    pub fn case_sensitive(mut self, case_sensitive: bool) -> Self {
        self.case_sensitive = case_sensitive;
        self
    }

    pub fn use_regex(mut self, use_regex: bool) -> Self {
        self.use_regex = use_regex;
        self
    }

    pub fn highlight(mut self, highlight: bool) -> Self {
        self.highlight = highlight;
        self
    }

    pub fn build(&mut self) -> Result<(), String> {
        for pattern in &mut self.patterns {
            if self.use_regex {
                let re = if self.case_sensitive {
                    Regex::new(&pattern.text)
                } else {
                    Regex::new(&format!("(?i){}", pattern.text))
                };
                pattern.regex = Some(re.map_err(|e| format!("Regex error: {}", e))?);
            }
        }
        Ok(())
    }

    pub fn matches(&self, text: &str) -> bool {
        if self.patterns.is_empty() {
            return true;
        }

        for pattern in &self.patterns {
            let matched = if self.use_regex {
                pattern.regex.as_ref().map(|re| re.is_match(text)).unwrap_or(false)
            } else if self.case_sensitive {
                text.contains(&pattern.text)
            } else {
                text.to_lowercase().contains(&pattern.text.to_lowercase())
            };

            if !pattern.is_inverted && matched {
                return true;
            }
            if pattern.is_inverted && !matched {
                return true;
            }
        }

        false
    }

    pub fn matches_entry(&self, entry: &LogEntry) -> bool {
        self.matches(&entry.raw)
    }

    pub fn highlight_text(&self, text: &str) -> String {
        if !self.highlight || self.patterns.is_empty() {
            return text.to_string();
        }

        let mut result = text.to_string();

        for pattern in &self.patterns {
            if self.use_regex {
                if let Some(ref re) = pattern.regex {
                    result = re
                        .replace_all(&result, |caps: &regex::Captures| {
                            caps.get(0)
                                .map(|m| color::red_bold(m.as_str()))
                                .unwrap_or_default()
                        })
                        .to_string();
                }
            } else {
                let search_text = &pattern.text;
                if self.case_sensitive {
                    result = result.replace(search_text, &color::red_bold(search_text));
                } else {
                    result = highlight_case_insensitive(&result, search_text);
                }
            }
        }

        result
    }

    pub fn highlight_entry(&self, entry: &LogEntry) -> String {
        self.highlight_text(&entry.raw)
    }

    pub fn search_entries<'a>(&self, entries: &'a [LogEntry]) -> Vec<&'a LogEntry> {
        entries
            .iter()
            .filter(|e| self.matches_entry(e))
            .collect()
    }

    pub fn search_lines(&self, lines: &[String]) -> Vec<(usize, String)> {
        lines
            .iter()
            .enumerate()
            .filter(|(_, line)| self.matches(line))
            .map(|(i, line)| (i, self.highlight_text(line)))
            .collect()
    }
}

fn highlight_case_insensitive(text: &str, pattern: &str) -> String {
    if pattern.is_empty() {
        return text.to_string();
    }

    let pattern_lower = pattern.to_lowercase();
    let text_lower = text.to_lowercase();
    let mut result = String::new();
    let mut last_end = 0;
    let pattern_len = pattern.len();

    while let Some(start) = text_lower[last_end..].find(&pattern_lower) {
        let abs_start = last_end + start;
        let abs_end = abs_start + pattern_len;

        result.push_str(&text[last_end..abs_start]);
        result.push_str(&color::red_bold(&text[abs_start..abs_end]));

        last_end = abs_end;
    }

    result.push_str(&text[last_end..]);
    result
}

#[derive(Debug, Default)]
pub struct SearchResult {
    pub matches: Vec<(usize, String)>,
    pub total_matches: usize,
    pub files_searched: usize,
}

impl SearchResult {
    pub fn new() -> Self {
        SearchResult::default()
    }

    pub fn merge(&mut self, other: &SearchResult) {
        self.matches.extend(other.matches.clone());
        self.total_matches += other.total_matches;
        self.files_searched += other.files_searched;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_search() {
        let searcher = Searcher::new(vec!["error".to_string()]);
        assert!(searcher.matches("This is an ERROR message"));
        assert!(searcher.matches("error occurred"));
        assert!(!searcher.matches("info message"));
    }

    #[test]
    fn test_case_sensitive_search() {
        let mut searcher = Searcher::new(vec!["ERROR".to_string()]);
        searcher = searcher.case_sensitive(true);
        assert!(searcher.matches("ERROR happened"));
        assert!(!searcher.matches("error happened"));
    }

    #[test]
    fn test_regex_search() {
        let mut searcher = Searcher::new(vec![r"\d{3}-\d{4}".to_string()]);
        searcher = searcher.use_regex(true);
        searcher.build().unwrap();
        assert!(searcher.matches("Phone: 123-4567"));
        assert!(!searcher.matches("Phone: 12-345"));
    }

    #[test]
    fn test_search_entries() {
        let entries = vec![
            LogEntry::new("INFO: starting application".to_string(), 0),
            LogEntry::new("ERROR: something went wrong".to_string(), 1),
            LogEntry::new("DEBUG: processing data".to_string(), 2),
        ];

        let searcher = Searcher::new(vec!["error".to_string()]);
        let results = searcher.search_entries(&entries);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].line_index, 1);
    }
}
