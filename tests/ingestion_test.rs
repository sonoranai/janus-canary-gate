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
