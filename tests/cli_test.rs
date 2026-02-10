use assert_cmd::Command;
use predicates::prelude::*;

fn cmd() -> Command {
    Command::cargo_bin("canary-gate").unwrap()
}

#[test]
fn cli_no_args_shows_help() {
    cmd()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

#[test]
fn cli_help_flag() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("canary-gate"))
        .stdout(predicate::str::contains("evaluate"))
        .stdout(predicate::str::contains("watch"))
        .stdout(predicate::str::contains("validate"))
        .stdout(predicate::str::contains("explain"))
        .stdout(predicate::str::contains("history"));
}

#[test]
fn cli_version_flag() {
    cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("canary-gate"));
}

#[test]
fn cli_evaluate_requires_config() {
    cmd()
        .arg("evaluate")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--config"));
}

#[test]
fn cli_evaluate_requires_log() {
    cmd()
        .args(["evaluate", "--config", "nonexistent.yaml"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--log"));
}

#[test]
fn cli_validate_requires_config() {
    cmd()
        .arg("validate")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--config"));
}

#[test]
fn cli_unknown_subcommand_rejected() {
    cmd().arg("nonexistent").assert().failure();
}

#[test]
fn cli_evaluate_with_valid_config_and_log() {
    let config = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/golden/scenario_promote/config.yaml");
    let log = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/golden/scenario_promote/canary.log");

    cmd()
        .args([
            "evaluate",
            "--config",
            config.to_str().unwrap(),
            "--log",
            log.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("RECOMMEND_PROMOTE"));
}

#[test]
fn cli_evaluate_rollback_exit_code() {
    let config = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/golden/scenario_rollback/config.yaml");
    let log = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/golden/scenario_rollback/canary.log");

    cmd()
        .args([
            "evaluate",
            "--config",
            config.to_str().unwrap(),
            "--log",
            log.to_str().unwrap(),
        ])
        .assert()
        .code(2)
        .stdout(predicate::str::contains("RECOMMEND_ROLLBACK"));
}

#[test]
fn cli_evaluate_json_output() {
    let config = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/golden/scenario_promote/config.yaml");
    let log = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/golden/scenario_promote/canary.log");

    cmd()
        .args([
            "evaluate",
            "--config",
            config.to_str().unwrap(),
            "--log",
            log.to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"recommendation\""))
        .stdout(predicate::str::contains("\"promote\""));
}

#[test]
fn cli_validate_with_valid_config() {
    let config = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/configs/valid_minimal.yaml");

    cmd()
        .args(["validate", "--config", config.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"));
}

#[test]
fn cli_validate_with_invalid_config() {
    let config = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/configs/invalid_missing_tests.yaml");

    cmd()
        .args(["validate", "--config", config.to_str().unwrap()])
        .assert()
        .code(3);
}
