use std::process::Command;
use assert_cmd::prelude::*;
use predicates::prelude::*;

#[test]
fn test_cli_help_command() {
    let mut cmd = Command::cargo_bin("duca").unwrap();
    cmd.arg("--help");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Read Dante's Divine Comedy from your terminal"))
        .stdout(predicate::str::contains("search"))
        .stdout(predicate::str::contains("canto"))
        .stdout(predicate::str::contains("tui"))
        .stdout(predicate::str::contains("parse"));
}

#[test]
fn test_cli_search_command() {
    let mut cmd = Command::cargo_bin("duca").unwrap();
    cmd.args(&["search", "stelle"]);
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Found"))
        .stdout(predicate::str::contains("matches for 'stelle'"))
        .stdout(predicate::str::contains("stelle"));
}

#[test]
fn test_cli_search_with_cantica_filter() {
    let mut cmd = Command::cargo_bin("duca").unwrap();
    cmd.args(&["search", "stelle", "-c", "inferno"]);
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("matches for 'stelle'"))
        .stdout(predicate::str::contains("Inferno"));
}

#[test]
fn test_cli_search_no_matches() {
    let mut cmd = Command::cargo_bin("duca").unwrap();
    cmd.args(&["search", "xyznomatch123"]);
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No matches found"));
}

#[test]
fn test_cli_canto_command() {
    let mut cmd = Command::cargo_bin("duca").unwrap();
    cmd.args(&["canto", "inferno", "1"]);
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Inferno Canto I"))
        .stdout(predicate::str::contains("Nel mezzo del cammin"));
}

#[test]
fn test_cli_invalid_cantica() {
    let mut cmd = Command::cargo_bin("duca").unwrap();
    cmd.args(&["canto", "invalid", "1"]);
    
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Invalid cantica"));
}

#[test]
fn test_cli_invalid_canto_number() {
    let mut cmd = Command::cargo_bin("duca").unwrap();
    cmd.args(&["canto", "inferno", "99"]);
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Canto 99 not found"));
}

#[test]
fn test_cli_paradiso_canto() {
    let mut cmd = Command::cargo_bin("duca").unwrap();
    cmd.args(&["canto", "paradiso", "33"]);
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Paradiso Canto XXXIII"));
}

#[test]
fn test_cli_purgatorio_canto() {
    let mut cmd = Command::cargo_bin("duca").unwrap();
    cmd.args(&["canto", "purgatorio", "1"]);
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Purgatorio Canto I"));
}

#[test]
fn test_cli_search_case_insensitive() {
    let mut cmd = Command::cargo_bin("duca").unwrap();
    cmd.args(&["search", "AMOR"]);
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("matches for 'AMOR'"));
}

#[test]
fn test_cli_search_special_characters() {
    let mut cmd = Command::cargo_bin("duca").unwrap();
    cmd.args(&["search", "città"]);
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("matches for 'città'"));
}

#[test]
fn test_cli_no_subcommand() {
    let mut cmd = Command::cargo_bin("duca").unwrap();
    
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Usage: duca <COMMAND>"));
}

#[test]
fn test_cli_version_info() {
    // Test that the binary can be executed (basic smoke test)
    let mut cmd = Command::cargo_bin("duca").unwrap();
    cmd.arg("--help");
    
    cmd.assert()
        .success();
}

#[test]
fn test_cli_canto_number_boundary() {
    // Test that numbers > 255 are rejected by clap
    let mut cmd = Command::cargo_bin("duca").unwrap();
    cmd.args(&["canto", "inferno", "256"]);
    
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("256 is not in 0..=255"));
}

#[test]
fn test_cli_search_with_regex_special_chars() {
    // Test search with characters that could break regex
    let mut cmd = Command::cargo_bin("duca").unwrap();
    cmd.args(&["search", ".*"]);
    
    // Should not crash, should handle regex escaping
    cmd.assert()
        .success();
}

#[test]
fn test_cli_multiple_word_search() {
    let mut cmd = Command::cargo_bin("duca").unwrap();
    cmd.args(&["search", "mezzo del"]);
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("mezzo del"));
}