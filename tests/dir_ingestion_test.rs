use canary_gate::config::LogFormat;
use canary_gate::ingestion::{discover_log_files, LogInput, LogReader};
use std::path::Path;
use tempfile::tempdir;

fn write_log(dir: &Path, name: &str, content: &str) {
    std::fs::write(dir.join(name), content).unwrap();
}

#[test]
fn discover_files_in_directory() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "a.log", "line1\n");
    write_log(dir.path(), "b.log", "line2\n");
    write_log(dir.path(), "c.log", "line3\n");

    let files = discover_log_files(dir.path(), None).unwrap();
    assert_eq!(files.len(), 3);
    // Sorted by name
    assert!(files[0].ends_with("a.log"));
    assert!(files[1].ends_with("b.log"));
    assert!(files[2].ends_with("c.log"));
}

#[test]
fn discover_files_with_glob_pattern() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "webapp-1.log", "line1\n");
    write_log(dir.path(), "webapp-2.log", "line2\n");
    write_log(dir.path(), "other.txt", "line3\n");

    let files = discover_log_files(dir.path(), Some("webapp-*.log")).unwrap();
    assert_eq!(files.len(), 2);
    assert!(files[0].ends_with("webapp-1.log"));
    assert!(files[1].ends_with("webapp-2.log"));
}

#[test]
fn discover_files_empty_dir() {
    let dir = tempdir().unwrap();
    let files = discover_log_files(dir.path(), None).unwrap();
    assert!(files.is_empty());
}

#[test]
fn discover_files_nonexistent_dir() {
    let result = discover_log_files(Path::new("/nonexistent/dir"), None);
    assert!(result.is_err());
}

#[test]
fn discover_files_invalid_glob() {
    let dir = tempdir().unwrap();
    let result = discover_log_files(dir.path(), Some("[invalid"));
    assert!(result.is_err());
}

#[test]
fn read_input_single_file() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("test.log");
    std::fs::write(&file, "2024-01-15T10:00:00Z hello\n").unwrap();

    let reader = LogReader::new(LogFormat::Plaintext);
    let lines = reader.read_input(LogInput::SingleFile(&file)).unwrap();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].content.contains("hello"));
}

#[test]
fn read_input_directory() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "a.log", "2024-01-15T10:00:00Z first\n");
    write_log(dir.path(), "b.log", "2024-01-15T10:00:01Z second\n");

    let reader = LogReader::new(LogFormat::Plaintext);
    let lines = reader
        .read_input(LogInput::Directory {
            dir: dir.path(),
            pattern: None,
        })
        .unwrap();
    assert_eq!(lines.len(), 2);
}

#[test]
fn read_input_directory_with_pattern() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "app-1.log", "2024-01-15T10:00:00Z matched\n");
    write_log(dir.path(), "app-2.log", "2024-01-15T10:00:01Z matched\n");
    write_log(dir.path(), "debug.txt", "should be excluded\n");

    let reader = LogReader::new(LogFormat::Plaintext);
    let lines = reader
        .read_input(LogInput::Directory {
            dir: dir.path(),
            pattern: Some("app-*.log"),
        })
        .unwrap();
    assert_eq!(lines.len(), 2);
    assert!(lines.iter().all(|l| l.content.contains("matched")));
}

#[test]
fn read_input_directory_no_matches() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "other.txt", "no match\n");

    let reader = LogReader::new(LogFormat::Plaintext);
    let result = reader.read_input(LogInput::Directory {
        dir: dir.path(),
        pattern: Some("*.log"),
    });
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("no log files found"));
}

#[test]
fn source_field_tracks_files() {
    let dir = tempdir().unwrap();
    write_log(dir.path(), "alpha.log", "2024-01-15T10:00:00Z from-alpha\n");
    write_log(dir.path(), "beta.log", "2024-01-15T10:00:01Z from-beta\n");

    let reader = LogReader::new(LogFormat::Plaintext);
    let lines = reader
        .read_input(LogInput::Directory {
            dir: dir.path(),
            pattern: None,
        })
        .unwrap();
    assert_eq!(lines.len(), 2);

    let alpha_line = lines
        .iter()
        .find(|l| l.content.contains("from-alpha"))
        .unwrap();
    assert!(alpha_line.source.as_ref().unwrap().contains("alpha.log"));

    let beta_line = lines
        .iter()
        .find(|l| l.content.contains("from-beta"))
        .unwrap();
    assert!(beta_line.source.as_ref().unwrap().contains("beta.log"));
}

#[test]
fn merged_stream_sorted_by_timestamp() {
    let dir = tempdir().unwrap();
    // alpha has the later timestamp, beta has the earlier one.
    write_log(dir.path(), "alpha.log", "2024-01-15T10:00:10Z late\n");
    write_log(dir.path(), "beta.log", "2024-01-15T10:00:01Z early\n");

    let reader = LogReader::new(LogFormat::Plaintext);
    let lines = reader
        .read_input(LogInput::Directory {
            dir: dir.path(),
            pattern: None,
        })
        .unwrap();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].content.contains("early"));
    assert!(lines[1].content.contains("late"));
}
