use std::io::BufRead;
use std::path::{Path, PathBuf};

use crate::config::LogFormat;
use crate::error::{Error, Result};

/// A single raw log line with metadata from ingestion.
#[derive(Debug, Clone)]
pub struct RawLogLine {
    /// The raw line content.
    pub content: String,

    /// Line number in the source (1-based).
    pub line_number: usize,

    /// Timestamp extracted from the line, if available.
    pub timestamp: Option<String>,

    /// Whether this line was detected as JSON.
    pub is_json: bool,

    /// Source file path or identifier (e.g. "pod:name"). None for BufRead/API sources.
    pub source: Option<String>,
}

/// Streaming log reader that processes lines without full buffering.
pub struct LogReader {
    format: LogFormat,
}

impl LogReader {
    pub fn new(format: LogFormat) -> Self {
        Self { format }
    }

    /// Read log lines from a file path, streaming line by line.
    pub fn read_file(&self, path: &Path) -> Result<Vec<RawLogLine>> {
        let file = std::fs::File::open(path).map_err(|e| {
            Error::Ingestion(format!("failed to open log file {}: {}", path.display(), e))
        })?;
        let reader = std::io::BufReader::new(file);
        let mut lines = self.read_lines(reader)?;
        let source = path.display().to_string();
        for line in &mut lines {
            line.source = Some(source.clone());
        }
        Ok(lines)
    }

    /// Read and merge log lines from multiple files, sorted by timestamp.
    pub fn read_files(&self, paths: &[PathBuf]) -> Result<Vec<RawLogLine>> {
        let mut all_lines = Vec::new();
        for path in paths {
            let lines = self.read_file(path)?;
            all_lines.extend(lines);
        }
        all_lines.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        Ok(all_lines)
    }

    /// Read log lines resolved from a `LogInput` (single file or directory).
    pub fn read_input(&self, input: LogInput<'_>) -> Result<Vec<RawLogLine>> {
        match input {
            LogInput::SingleFile(path) => self.read_file(path),
            LogInput::Directory { dir, pattern } => {
                let paths = discover_log_files(dir, pattern)?;
                if paths.is_empty() {
                    return Err(Error::Ingestion(format!(
                        "no log files found in {}{}",
                        dir.display(),
                        pattern
                            .map(|p| format!(" matching '{}'", p))
                            .unwrap_or_default()
                    )));
                }
                self.read_files(&paths)
            }
        }
    }

    /// Read log lines from any BufRead source.
    pub fn read_lines<R: BufRead>(&self, reader: R) -> Result<Vec<RawLogLine>> {
        let mut lines = Vec::new();

        for (idx, line_result) in reader.lines().enumerate() {
            let content = line_result
                .map_err(|e| Error::Ingestion(format!("failed to read line {}: {}", idx + 1, e)))?;

            // Skip empty lines
            if content.trim().is_empty() {
                continue;
            }

            let is_json = detect_json(&content, &self.format);
            let timestamp = extract_timestamp(&content, is_json);

            lines.push(RawLogLine {
                content,
                line_number: idx + 1,
                timestamp,
                is_json,
                source: None,
            });
        }

        Ok(lines)
    }
}

/// Describes where to read log lines from.
pub enum LogInput<'a> {
    /// A single log file.
    SingleFile(&'a Path),
    /// All (or glob-filtered) files in a directory.
    Directory {
        dir: &'a Path,
        pattern: Option<&'a str>,
    },
}

/// Discover log files in a directory, optionally filtered by a glob pattern.
///
/// Returns a sorted list of regular files. Only scans the top level of the directory.
pub fn discover_log_files(dir: &Path, pattern: Option<&str>) -> Result<Vec<PathBuf>> {
    let compiled = pattern
        .map(|p| {
            glob::Pattern::new(p)
                .map_err(|e| Error::Ingestion(format!("invalid glob pattern '{}': {}", p, e)))
        })
        .transpose()?;

    let entries = std::fs::read_dir(dir).map_err(|e| {
        Error::Ingestion(format!("failed to read directory {}: {}", dir.display(), e))
    })?;

    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry
            .map_err(|e| Error::Ingestion(format!("failed to read directory entry: {}", e)))?;
        let path = entry.path();

        // Only include regular files
        if !path.is_file() {
            continue;
        }

        // Apply glob filter against the file name
        if let Some(ref pat) = compiled {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if !pat.matches(name) {
                    continue;
                }
            } else {
                continue;
            }
        }

        paths.push(path);
    }

    paths.sort();
    Ok(paths)
}

/// Detect whether a line is JSON based on format config.
fn detect_json(line: &str, format: &LogFormat) -> bool {
    match format {
        LogFormat::Json => true,
        LogFormat::Plaintext => false,
        LogFormat::Auto => {
            let trimmed = line.trim();
            trimmed.starts_with('{') && trimmed.ends_with('}')
        }
    }
}

/// Extract a timestamp from a log line.
fn extract_timestamp(line: &str, is_json: bool) -> Option<String> {
    if is_json {
        // Try to parse as JSON and extract common timestamp fields
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            for key in &["timestamp", "ts", "time", "@timestamp", "datetime"] {
                if let Some(ts) = value.get(key).and_then(|v| v.as_str()) {
                    return Some(ts.to_string());
                }
            }
        }
        None
    } else {
        // For plaintext, try to extract ISO-8601 timestamp from start of line
        let trimmed = line.trim();
        // Match common patterns like "2024-01-15T10:30:00Z" or "2024-01-15 10:30:00"
        if trimmed.len() >= 19 {
            let prefix = &trimmed[..19];
            if prefix.chars().nth(4) == Some('-')
                && prefix.chars().nth(7) == Some('-')
                && (prefix.chars().nth(10) == Some('T') || prefix.chars().nth(10) == Some(' '))
            {
                // Find the end of the timestamp (up to next space or end)
                let ts_end = trimmed[19..]
                    .find(' ')
                    .map(|i| i + 19)
                    .unwrap_or(trimmed.len().min(30));
                return Some(trimmed[..ts_end].to_string());
            }
        }
        None
    }
}
