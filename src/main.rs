use clap::{Parser, Subcommand};
use anyhow::{Result, Context};
use std::path::{PathBuf, Path};
use std::str::FromStr;

use logalyzer::*;

#[derive(Parser)]
#[command(name = "logalyzer")]
#[command(version, about = "High-performance cross-platform log analysis tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Parse and display log files")]
    Parse {
        #[arg(required = true, help = "Log file or directory path")]
        path: PathBuf,

        #[arg(short = 'f', long, default_value = "auto", help = "Log format: auto, common, json, simple, nginx, apache")]
        format: String,

        #[arg(short = 'n', long, help = "Number of lines to display")]
        lines: Option<usize>,

        #[arg(long, help = "Output format: text, json, jsonl, csv, tsv, pretty-json")]
        output: Option<String>,

        #[arg(long, help = "Include raw log line in output")]
        include_raw: bool,
    },

    #[command(about = "Filter logs by level, module, etc.")]
    Filter {
        #[arg(required = true, help = "Log file path")]
        path: PathBuf,

        #[arg(short = 'l', long, help = "Filter by log levels (comma-separated): trace,debug,info,warn,error,fatal")]
        level: Option<String>,

        #[arg(short = 'm', long, help = "Minimum log level")]
        min_level: Option<String>,

        #[arg(long, help = "Filter by module name")]
        module: Option<String>,

        #[arg(short = 'f', long, default_value = "auto", help = "Log format")]
        format: String,

        #[arg(long, help = "Output format")]
        output: Option<String>,

        #[arg(long, help = "Case sensitive filtering")]
        case_sensitive: bool,
    },

    #[command(about = "Search for keywords in log files")]
    Search {
        #[arg(required = true, help = "Log file or directory path")]
        path: PathBuf,

        #[arg(required = true, trailing_var_arg = true, help = "Search patterns")]
        patterns: Vec<String>,

        #[arg(short = 'i', long, help = "Case insensitive search (default)")]
        ignore_case: bool,

        #[arg(short = 's', long, help = "Case sensitive search")]
        case_sensitive: bool,

        #[arg(short = 'r', long, help = "Use regex patterns")]
        regex: bool,

        #[arg(short = 'l', long, help = "Show line numbers")]
        line_numbers: bool,

        #[arg(long, help = "Count matches only")]
        count: bool,

        #[arg(short = 'n', long, help = "Maximum number of matches to show")]
        max_matches: Option<usize>,
    },

    #[command(about = "Show log statistics")]
    Stats {
        #[arg(required = true, help = "Log file or directory path")]
        path: PathBuf,

        #[arg(short = 'f', long, default_value = "auto", help = "Log format")]
        format: String,

        #[arg(long, help = "Output as JSON")]
        json: bool,
    },

    #[command(about = "Convert log format")]
    Convert {
        #[arg(required = true, help = "Input log file path")]
        input: PathBuf,

        #[arg(required = true, short = 'o', long, help = "Output file path")]
        output: PathBuf,

        #[arg(short = 'f', long, default_value = "auto", help = "Input log format")]
        from_format: String,

        #[arg(short = 't', long, default_value = "json", help = "Output format: json, jsonl, csv, tsv, text")]
        to_format: String,

        #[arg(long, help = "Include raw log line in output")]
        include_raw: bool,
    },

    #[command(about = "Show first N lines of log file")]
    Head {
        #[arg(required = true, help = "Log file path")]
        path: PathBuf,

        #[arg(short = 'n', long, default_value_t = 10, help = "Number of lines")]
        lines: usize,
    },

    #[command(about = "Show last N lines of log file")]
    Tail {
        #[arg(required = true, help = "Log file path")]
        path: PathBuf,

        #[arg(short = 'n', long, default_value_t = 10, help = "Number of lines")]
        lines: usize,
    },

    #[command(about = "Count lines in log file")]
    Count {
        #[arg(required = true, help = "Log file or directory path")]
        path: PathBuf,
    },

    #[command(about = "Parse logs with custom rules")]
    CustomParse {
        #[arg(required = true, help = "Log file path")]
        path: PathBuf,

        #[arg(required = true, short = 'r', long, help = "Custom rule JSON file")]
        rule: PathBuf,

        #[arg(short = 'n', long, help = "Number of lines to display")]
        lines: Option<usize>,

        #[arg(long, help = "Output format")]
        output: Option<String>,
    },

    #[command(about = "Monitor log file in real-time with alerts")]
    Monitor {
        #[arg(required = true, help = "Log file path")]
        path: PathBuf,

        #[arg(short = 'f', long, default_value = "auto", help = "Log format")]
        format: String,

        #[arg(short = 'l', long, help = "Alert on log level: trace,debug,info,warn,error,fatal")]
        alert_level: Option<String>,

        #[arg(short = 'p', long, default_value_t = 500, help = "Poll interval in milliseconds")]
        poll_interval: u64,

        #[arg(long, help = "Filter by minimum log level")]
        min_level: Option<String>,

        #[arg(long, help = "Search pattern to filter")]
        pattern: Option<String>,

        #[arg(long, help = "Don't follow new lines")]
        no_follow: bool,
    },

    #[command(about = "Generate aggregation report")]
    Report {
        #[arg(required = true, help = "Log file or directory path")]
        path: PathBuf,

        #[arg(short = 'f', long, default_value = "auto", help = "Log format")]
        format: String,

        #[arg(short = 'g', long, help = "Aggregate fields (comma-separated): level,module,hour,minute,status")]
        group_by: Option<String>,

        #[arg(short = 'o', long, help = "Output format: text,json,html,markdown")]
        output_format: Option<String>,

        #[arg(short = 'O', long, help = "Output file path")]
        output_file: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Parse { path, format, lines, output, include_raw } => {
            cmd_parse(&path, &format, lines, output.as_deref(), include_raw)
        }
        Commands::Filter { path, level, min_level, module, format, output, case_sensitive } => {
            cmd_filter(&path, level.as_deref(), min_level.as_deref(), module.as_deref(), &format, output.as_deref(), case_sensitive)
        }
        Commands::Search { path, patterns, ignore_case: _, case_sensitive, regex, line_numbers, count, max_matches } => {
            cmd_search(&path, &patterns, case_sensitive, regex, line_numbers, count, max_matches)
        }
        Commands::Stats { path, format, json } => {
            cmd_stats(&path, &format, json)
        }
        Commands::Convert { input, output, from_format, to_format, include_raw } => {
            cmd_convert(&input, &output, &from_format, &to_format, include_raw)
        }
        Commands::Head { path, lines } => {
            cmd_head(&path, lines)
        }
        Commands::Tail { path, lines } => {
            cmd_tail(&path, lines)
        }
        Commands::Count { path } => {
            cmd_count(&path)
        }
        Commands::CustomParse { path, rule, lines, output } => {
            cmd_custom_parse(&path, &rule, lines, output.as_deref())
        }
        Commands::Monitor { path, format, alert_level, poll_interval, min_level, pattern, no_follow } => {
            cmd_monitor(&path, &format, alert_level.as_deref(), poll_interval, min_level.as_deref(), pattern.as_deref(), !no_follow)
        }
        Commands::Report { path, format, group_by, output_format, output_file } => {
            cmd_report(&path, &format, group_by.as_deref(), output_format.as_deref(), output_file.as_deref())
        }
    }
}

fn collect_files(path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if path.is_file() {
        files.push(path.to_path_buf());
    } else if path.is_dir() {
        walk_dir(path, &mut files);
    }

    files.sort();
    files
}

fn walk_dir(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_dir(&path, files);
            } else if path.is_file() {
                files.push(path);
            }
        }
    }
}

fn parse_log_format(format: &str) -> Result<parser::LogFormat> {
    parser::LogFormat::from_str(format)
        .map_err(|e| anyhow::anyhow!(e))
}

fn parse_output_format(format: &str) -> Result<converter::OutputFormat> {
    converter::OutputFormat::from_str(format)
        .map_err(|e| anyhow::anyhow!(e))
}

fn cmd_parse(path: &std::path::Path, format_str: &str, lines: Option<usize>, output_format: Option<&str>, include_raw: bool) -> Result<()> {
    let files = collect_files(path);
    if files.is_empty() {
        return Err(anyhow::anyhow!("No files found"));
    }

    let format = parse_log_format(format_str)?;
    let parser = LogParser::new(format);
    let out_fmt = output_format
        .map(|f| parse_output_format(f))
        .transpose()?
        .unwrap_or(converter::OutputFormat::Text);
    let converter = FormatConverter::new(out_fmt).include_raw(include_raw);

    let mut total_lines = 0;
    let max_lines = lines.unwrap_or(usize::MAX);

    for file_path in &files {
        let reader = ChunkReader::new(file_path, 1024 * 1024)?;

        if files.len() > 1 {
            println!("\n{}", color::bold(&format!("=== {} ===", file_path.display())));
        }

        let result = reader.read_lines(|line, _index| {
            if total_lines >= max_lines {
                return Ok(false);
            }

            let entry = parser.parse_line(line, total_lines);
            let output = converter.convert_entry(&entry);
            println!("{}", colorize_output(&entry, &output, out_fmt));

            total_lines += 1;
            Ok(true)
        })?;

        let _ = result;
        if total_lines >= max_lines {
            break;
        }
    }

    Ok(())
}

fn cmd_filter(path: &std::path::Path, levels: Option<&str>, min_level: Option<&str>, module: Option<&str>, format_str: &str, output_format: Option<&str>, case_sensitive: bool) -> Result<()> {
    let files = collect_files(path);
    if files.is_empty() {
        return Err(anyhow::anyhow!("No files found"));
    }

    let format = parse_log_format(format_str)?;
    let parser = LogParser::new(format);

    let mut filter = LogFilter::new().case_sensitive(case_sensitive);

    if let Some(level_str) = levels {
        let level_list: Result<Vec<LogLevel>, _> = level_str
            .split(',')
            .map(|s| LogLevel::from_str(s.trim()).map_err(|e| anyhow::anyhow!(e)))
            .collect();
        filter = filter.with_levels(level_list?);
    }

    if let Some(min_lvl) = min_level {
        let level = LogLevel::from_str(min_lvl).map_err(|e| anyhow::anyhow!(e))?;
        filter = filter.with_min_level(level);
    }

    if let Some(mod_str) = module {
        filter = filter.with_module(mod_str.to_string());
    }

    let out_fmt = output_format
        .map(|f| parse_output_format(f))
        .transpose()?
        .unwrap_or(converter::OutputFormat::Text);
    let converter = FormatConverter::new(out_fmt);

    let mut matched_count = 0;

    for file_path in &files {
        let reader = ChunkReader::new(file_path, 1024 * 1024)?;

        if files.len() > 1 {
            println!("\n{}", color::bold(&format!("=== {} ===", file_path.display())));
        }

        reader.read_lines(|line, index| {
            let entry = parser.parse_line(line, index);
            if filter.matches(&entry) {
                let output = converter.convert_entry(&entry);
                println!("{}", colorize_output(&entry, &output, out_fmt));
                matched_count += 1;
            }
            Ok(true)
        })?;
    }

    eprintln!("\nMatched {} lines", color::green_bold(&matched_count.to_string()));
    Ok(())
}

fn cmd_search(path: &std::path::Path, patterns: &[String], case_sensitive: bool, use_regex: bool, show_line_numbers: bool, count_only: bool, max_matches: Option<usize>) -> Result<()> {
    let files = collect_files(path);
    if files.is_empty() {
        return Err(anyhow::anyhow!("No files found"));
    }

    let mut searcher = Searcher::new(patterns.to_vec())
        .case_sensitive(case_sensitive)
        .use_regex(use_regex)
        .highlight(true);
    searcher.build().map_err(|e| anyhow::anyhow!(e))?;

    let mut total_matches = 0;
    let mut files_with_matches = 0;
    let max = max_matches.unwrap_or(usize::MAX);

    for file_path in &files {
        let reader = ChunkReader::new(file_path, 1024 * 1024)?;
        let mut file_matches = 0;
        let mut first_file = true;

        let result = reader.read_lines(|line, line_num| {
            if total_matches >= max {
                return Ok(false);
            }

            if searcher.matches(line) {
                if count_only {
                    file_matches += 1;
                    total_matches += 1;
                } else {
                    if first_file && files.len() > 1 {
                        println!("\n{}", color::bold(&format!("=== {} ===", file_path.display())));
                        first_file = false;
                    }

                    let highlighted = searcher.highlight_text(line);
                    if show_line_numbers {
                        println!("{}: {}", color::cyan(&(line_num + 1).to_string()), highlighted);
                    } else {
                        println!("{}", highlighted);
                    }
                    file_matches += 1;
                    total_matches += 1;
                }
            }
            Ok(true)
        })?;

        let _ = result;

        if file_matches > 0 {
            files_with_matches += 1;
        }

        if count_only && files.len() > 1 {
            println!("{}: {}", file_path.display(), file_matches);
        }

        if total_matches >= max {
            break;
        }
    }

    eprintln!(
        "\nFound {} matches in {} of {} files",
        color::green_bold(&total_matches.to_string()),
        color::yellow_bold(&files_with_matches.to_string()),
        files.len()
    );

    Ok(())
}

fn cmd_stats(path: &std::path::Path, format_str: &str, output_json: bool) -> Result<()> {
    let files = collect_files(path);
    if files.is_empty() {
        return Err(anyhow::anyhow!("No files found"));
    }

    let format = parse_log_format(format_str)?;
    let parser = LogParser::new(format);
    let mut total_stats = filter::LogStats::new();

    for file_path in &files {
        let reader = ChunkReader::new(file_path, 1024 * 1024)?;

        let result = reader.read_lines(|line, index| {
            let entry = parser.parse_line(line, index);
            total_stats.add_entry(&entry);
            Ok(true)
        })?;

        let _ = result;
    }

    if output_json {
        println!("{}", serde_json::to_string_pretty(&total_stats.to_json())?);
    } else {
        total_stats.print_summary();
    }

    Ok(())
}

fn cmd_convert(input: &std::path::Path, output: &std::path::Path, from_format: &str, to_format: &str, include_raw: bool) -> Result<()> {
    let fmt = parse_log_format(from_format)?;
    let parser = LogParser::new(fmt);

    let out_fmt = parse_output_format(to_format)?;
    let converter = FormatConverter::new(out_fmt).include_raw(include_raw);

    let reader = ChunkReader::new(input, 1024 * 1024)?;

    use std::io::Write;
    let mut out_file = std::fs::File::create(output)
        .with_context(|| format!("Failed to create output file: {}", output.display()))?;

    if out_fmt == converter::OutputFormat::Json || out_fmt == converter::OutputFormat::PrettyJson {
        let mut entries = Vec::new();
        reader.read_lines(|line, index| {
            let entry = parser.parse_line(line, index);
            entries.push(entry);
            Ok(true)
        })?;

        let output_str = converter.convert_entries(&entries);
        writeln!(out_file, "{}", output_str)?;
    } else if out_fmt == converter::OutputFormat::Csv || out_fmt == converter::OutputFormat::Tsv {
        let header = if out_fmt == converter::OutputFormat::Csv {
            "timestamp,level,module,message\n"
        } else {
            "timestamp\tlevel\tmodule\tmessage\n"
        };
        write!(out_file, "{}", header)?;

        reader.read_lines(|line, index| {
            let entry = parser.parse_line(line, index);
            let converted = converter.convert_entry(&entry);
            writeln!(out_file, "{}", converted)?;
            Ok(true)
        })?;
    } else {
        reader.read_lines(|line, index| {
            let entry = parser.parse_line(line, index);
            let converted = converter.convert_entry(&entry);
            writeln!(out_file, "{}", converted)?;
            Ok(true)
        })?;
    }

    eprintln!("Converted to {}", output.display());
    Ok(())
}

fn cmd_head(path: &std::path::Path, lines: usize) -> Result<()> {
    let reader = ChunkReader::new(path, 1024 * 1024)?;
    let head_lines = reader.read_head(lines)?;
    for line in &head_lines {
        println!("{}", line);
    }
    eprintln!("\nShowing {} of {} lines", head_lines.len(), reader.get_line_count()?);
    Ok(())
}

fn cmd_tail(path: &std::path::Path, lines: usize) -> Result<()> {
    let reader = ChunkReader::new(path, 1024 * 1024)?;
    let tail_lines = reader.read_tail(lines)?;
    for line in &tail_lines {
        println!("{}", line);
    }
    eprintln!("\nShowing last {} of {} lines", tail_lines.len(), reader.get_line_count()?);
    Ok(())
}

fn cmd_count(path: &std::path::Path) -> Result<()> {
    let files = collect_files(path);
    let mut total_lines = 0;

    for file_path in &files {
        let reader = ChunkReader::new(file_path, 1024 * 1024)?;
        let count = reader.get_line_count()?;
        if files.len() > 1 {
            println!("{:>8}  {}", count, file_path.display());
        }
        total_lines += count;
    }

    if files.len() > 1 {
        println!("{:>8}  total", total_lines);
    } else {
        println!("{}", total_lines);
    }

    Ok(())
}

fn colorize_output(entry: &LogEntry, output: &str, format: converter::OutputFormat) -> String {
    match format {
        converter::OutputFormat::Text => {
            if let Some(level) = entry.level {
                match level {
                    LogLevel::Trace => color::dimmed(output),
                    LogLevel::Debug => color::cyan(output),
                    LogLevel::Info => color::green(output),
                    LogLevel::Warn => color::yellow(output),
                    LogLevel::Error => color::red(output),
                    LogLevel::Fatal => color::magenta_bold(output),
                }
            } else {
                output.to_string()
            }
        }
        _ => output.to_string(),
    }
}

fn cmd_custom_parse(path: &Path, rule_path: &Path, lines: Option<usize>, output_format: Option<&str>) -> Result<()> {
    let rule_set = CustomRuleSet::load_from_file(rule_path)
        .map_err(|e| anyhow::anyhow!(e))?;

    if rule_set.is_empty() {
        return Err(anyhow::anyhow!("No custom rules loaded"));
    }

    eprintln!("Loaded {} custom rule(s)", rule_set.parser_count());

    let reader = ChunkReader::new(path, 1024 * 1024)?;
    let out_fmt = output_format
        .map(|f| parse_output_format(f))
        .transpose()?
        .unwrap_or(converter::OutputFormat::Text);
    let converter = FormatConverter::new(out_fmt);

    let mut total = 0;
    let mut matched = 0;
    let max_lines = lines.unwrap_or(usize::MAX);

    reader.read_lines(|line, index| {
        if total >= max_lines {
            return Ok(false);
        }
        total += 1;

        if let Some(entry) = rule_set.parse_line(line, index) {
            let output = converter.convert_entry(&entry);
            println!("{}", colorize_output(&entry, &output, out_fmt));
            matched += 1;
        }
        Ok(true)
    })?;

    eprintln!("\nMatched {} of {} lines with custom rules", matched, total);
    Ok(())
}

fn cmd_monitor(
    path: &Path,
    format_str: &str,
    alert_level: Option<&str>,
    poll_ms: u64,
    min_level: Option<&str>,
    pattern: Option<&str>,
    follow: bool,
) -> Result<()> {
    let format = parse_log_format(format_str)?;
    let parser = LogParser::new(format);

    let mut monitor = LogMonitor::new(path, parser)
        .with_poll_interval(std::time::Duration::from_millis(poll_ms))
        .with_follow(follow);

    if let Some(level_str) = min_level {
        let level = LogLevel::from_str(level_str)
            .map_err(|e| anyhow::anyhow!(e))?;
        let filter = LogFilter::new().with_min_level(level);
        monitor = monitor.with_filter(filter);
    }

    if let Some(pat) = pattern {
        let mut searcher = Searcher::new(vec![pat.to_string()]);
        searcher.build().map_err(|e| anyhow::anyhow!(e))?;
        monitor = monitor.with_searcher(searcher);
    }

    if let Some(level_str) = alert_level {
        let level = LogLevel::from_str(level_str)
            .map_err(|e| anyhow::anyhow!(e))?;
        let rule = AlertRule {
            name: format!("{}_alert", level.as_str().to_lowercase()),
            pattern: None,
            level: Some(level),
            min_count: 1,
            window_seconds: 10,
            cooldown_seconds: 30,
        };
        monitor.add_alert_rule(rule);
    }

    eprintln!("Monitoring: {}", path.display());
    eprintln!("Poll interval: {}ms", poll_ms);
    eprintln!("Follow mode: {}", if follow { "on" } else { "off" });
    eprintln!("(Press Ctrl+C to stop)");
    eprintln!();

    let _running = monitor.running_clone();

    let result = monitor.start(|entry, is_alert| {
        if is_alert {
            println!("{} [ALERT] {}",
                color::red_bold("⚠"),
                color::red_bold(&entry.message));
        } else {
            let level_str = entry.level.map(|l| format!("[{:>7}]", l.as_str())).unwrap_or_default();
            let ts = entry.timestamp.as_deref().unwrap_or("");
            println!("{} {} {}", ts, level_str, entry.message);
        }
    });

    match result {
        Ok(count) => {
            eprintln!("\nMonitoring stopped. Processed {} lines.", count);
            Ok(())
        }
        Err(e) => Err(anyhow::anyhow!(e)),
    }
}

fn cmd_report(
    path: &Path,
    format_str: &str,
    group_by: Option<&str>,
    output_format: Option<&str>,
    output_file: Option<&Path>,
) -> Result<()> {
    let files = collect_files(path);
    if files.is_empty() {
        return Err(anyhow::anyhow!("No files found"));
    }

    let format = parse_log_format(format_str)?;
    let parser = LogParser::new(format);

    let fields: Vec<AggregateField> = if let Some(gb) = group_by {
        gb.split(',')
            .map(|s| AggregateField::from_str(s.trim()).map_err(|e| anyhow::anyhow!(e)))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        vec![AggregateField::Level, AggregateField::Module]
    };

    let mut report = ReportGenerator::with_fields(&fields);

    for file_path in &files {
        let reader = ChunkReader::new(file_path, 1024 * 1024)?;

        if files.len() > 1 {
            eprintln!("Processing: {}", file_path.display());
        }

        reader.read_lines(|line, index| {
            let entry = parser.parse_line(line, index);
            report.add_entry(&entry);
            Ok(true)
        })?;
    }

    let out_fmt = output_format.unwrap_or("text");

    let content = match out_fmt {
        "json" => Some(serde_json::to_string_pretty(&report.to_json())?),
        "html" => Some(report.to_html()),
        "markdown" | "md" => Some(report.to_markdown()),
        "text" | _ => {
            report.print_summary();
            None
        }
    };

    if let Some(ref text) = content {
        if let Some(out_path) = output_file {
            std::fs::write(out_path, text)
                .with_context(|| format!("Failed to write report to {}", out_path.display()))?;
            eprintln!("Report written to: {}", out_path.display());
        } else {
            println!("{}", text);
        }
    }

    Ok(())
}
