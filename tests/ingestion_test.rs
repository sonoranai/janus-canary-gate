use canary_gate::config::LogFormat;
use canary_gate::ingestion::LogReader;
use std::path::Path;

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/logs")
        .join(name)
}

#[test]
fn read_plaintext_log() {
    let reader = LogReader::new(LogFormat::Plaintext);
    let lines = reader
        .read_file(&fixture("plaintext/grpc_healthy_startup.log"))
        .unwrap();
    assert!(!lines.is_empty());
    assert!(lines.iter().all(|l| !l.is_json));
}

#[test]
fn read_json_log() {
    let reader = LogReader::new(LogFormat::Json);
    let lines = reader
        .read_file(&fixture("json/structured_healthy.jsonl"))
        .unwrap();
    assert!(!lines.is_empty());
    assert!(lines.iter().all(|l| l.is_json));
}

#[test]
fn auto_detect_plaintext() {
    let reader = LogReader::new(LogFormat::Auto);
    let lines = reader
        .read_file(&fixture("plaintext/grpc_healthy_startup.log"))
        .unwrap();
    assert!(!lines.is_empty());
    // Plaintext lines should not be detected as JSON
    assert!(lines.iter().all(|l| !l.is_json));
}

#[test]
fn auto_detect_json() {
    let reader = LogReader::new(LogFormat::Auto);
    let lines = reader
        .read_file(&fixture("json/structured_healthy.jsonl"))
        .unwrap();
    assert!(!lines.is_empty());
    // JSON lines should be detected as JSON
    assert!(lines.iter().all(|l| l.is_json));
}

#[test]
fn plaintext_timestamps_extracted() {
    let reader = LogReader::new(LogFormat::Plaintext);
    let lines = reader
        .read_file(&fixture("plaintext/grpc_healthy_startup.log"))
        .unwrap();
    // All lines in this fixture have timestamps
    for line in &lines {
        assert!(
            line.timestamp.is_some(),
            "Expected timestamp for line: {}",
            line.content
        );
    }
}

#[test]
fn json_timestamps_extracted() {
    let reader = LogReader::new(LogFormat::Json);
    let lines = reader
        .read_file(&fixture("json/structured_healthy.jsonl"))
        .unwrap();
    for line in &lines {
        assert!(
            line.timestamp.is_some(),
            "Expected timestamp for line: {}",
            line.content
        );
    }
}

#[test]
fn line_numbers_are_sequential() {
    let reader = LogReader::new(LogFormat::Plaintext);
    let lines = reader
        .read_file(&fixture("plaintext/grpc_healthy_startup.log"))
        .unwrap();
    for (i, line) in lines.iter().enumerate() {
        assert_eq!(line.line_number, i + 1);
    }
}

#[test]
fn order_stability() {
    let reader = LogReader::new(LogFormat::Plaintext);
    let lines1 = reader
        .read_file(&fixture("plaintext/grpc_healthy_startup.log"))
        .unwrap();
    let lines2 = reader
        .read_file(&fixture("plaintext/grpc_healthy_startup.log"))
        .unwrap();
    assert_eq!(lines1.len(), lines2.len());
    for (a, b) in lines1.iter().zip(lines2.iter()) {
        assert_eq!(a.content, b.content);
        assert_eq!(a.line_number, b.line_number);
    }
}

#[test]
fn empty_lines_skipped() {
    let input = "line one\n\n\nline two\n";
    let reader = LogReader::new(LogFormat::Plaintext);
    let lines = reader.read_lines(std::io::Cursor::new(input)).unwrap();
    assert_eq!(lines.len(), 2);
}

#[test]
fn nonexistent_file_returns_error() {
    let reader = LogReader::new(LogFormat::Plaintext);
    let result = reader.read_file(Path::new("/nonexistent/file.log"));
    assert!(result.is_err());
}

#[test]
fn read_file_populates_source() {
    let reader = LogReader::new(LogFormat::Plaintext);
    let path = fixture("plaintext/grpc_healthy_startup.log");
    let lines = reader.read_file(&path).unwrap();
    assert!(!lines.is_empty());
    let expected = path.display().to_string();
    for line in &lines {
        assert_eq!(line.source.as_deref(), Some(expected.as_str()));
    }
}

#[test]
fn read_lines_has_no_source() {
    let input = "2024-01-15T10:00:00Z hello\n2024-01-15T10:00:01Z world\n";
    let reader = LogReader::new(LogFormat::Plaintext);
    let lines = reader.read_lines(std::io::Cursor::new(input)).unwrap();
    assert_eq!(lines.len(), 2);
    for line in &lines {
        assert!(line.source.is_none());
    }
}

#[test]
fn read_files_merges_multiple() {
    let dir = tempfile::tempdir().unwrap();

    let file_a = dir.path().join("a.log");
    let file_b = dir.path().join("b.log");
    std::fs::write(&file_a, "2024-01-15T10:00:00Z line-a\n").unwrap();
    std::fs::write(&file_b, "2024-01-15T10:00:01Z line-b\n").unwrap();

    let reader = LogReader::new(LogFormat::Plaintext);
    let lines = reader
        .read_files(&[file_a.clone(), file_b.clone()])
        .unwrap();

    assert_eq!(lines.len(), 2);
    assert_eq!(
        lines[0].source.as_deref(),
        Some(file_a.display().to_string().as_str())
    );
    assert_eq!(
        lines[1].source.as_deref(),
        Some(file_b.display().to_string().as_str())
    );
}

#[test]
fn read_files_sorts_by_timestamp() {
    let dir = tempfile::tempdir().unwrap();

    // File a has a later timestamp, file b has an earlier one.
    let file_a = dir.path().join("a.log");
    let file_b = dir.path().join("b.log");
    std::fs::write(&file_a, "2024-01-15T10:00:05Z late\n").unwrap();
    std::fs::write(&file_b, "2024-01-15T10:00:01Z early\n").unwrap();

    let reader = LogReader::new(LogFormat::Plaintext);
    let lines = reader.read_files(&[file_a, file_b]).unwrap();

    assert_eq!(lines.len(), 2);
    assert!(lines[0].content.contains("early"));
    assert!(lines[1].content.contains("late"));
}
