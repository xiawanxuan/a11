use clap::{Parser, Subcommand};
use colored::*;
use anyhow::{Result, Context};
use std::path::PathBuf;
use std::str::FromStr;
use walkdir::WalkDir;

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

        #[arg(required = true, help = "Search patterns")]
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
    }
}

fn collect_files(path: &std::path::Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if path.is_file() {
        files.push(path.to_path_buf());
    } else if path.is_dir() {
        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                files.push(entry.path().to_path_buf());
            }
        }
    }

    files.sort();
    files
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
            println!("\n{}", format!("=== {} ===", file_path.display()).bold());
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
            println!("\n{}", format!("=== {} ===", file_path.display()).bold());
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

    eprintln!("\nMatched {} lines", matched_count.to_string().green().bold());
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
                        println!("\n{}", format!("=== {} ===", file_path.display()).bold());
                        first_file = false;
                    }

                    let highlighted = searcher.highlight_text(line);
                    if show_line_numbers {
                        println!("{}: {}", (line_num + 1).to_string().cyan(), highlighted);
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
        total_matches.to_string().green().bold(),
        files_with_matches.to_string().yellow().bold(),
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

fn colorize_output(entry: &LogEntry, output: &str, format: converter::OutputFormat) -> ColoredString {
    match format {
        converter::OutputFormat::Text => {
            if let Some(level) = entry.level {
                match level {
                    LogLevel::Trace => output.dimmed(),
                    LogLevel::Debug => output.cyan(),
                    LogLevel::Info => output.green(),
                    LogLevel::Warn => output.yellow(),
                    LogLevel::Error => output.red(),
                    LogLevel::Fatal => output.magenta().bold(),
                }
            } else {
                output.normal()
            }
        }
        _ => output.normal(),
    }
}
