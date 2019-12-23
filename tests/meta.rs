//!
//! This module tests basic meta functionality of gltfgen to make sure all cli options are valid
//! and non-conflicting.
//!
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn box_rotate() {
    // Display Help
    let mut cmd = Command::cargo_bin("gltfgen").unwrap();
    cmd.arg("-h")
        .assert()
        .stderr(predicate::eq(b"" as &[u8])) // No stderr
        .success();
    // Display Help with long argument
    let mut cmd = Command::cargo_bin("gltfgen").unwrap();
    cmd.arg("--help")
        .assert()
        .stderr(predicate::eq(b"" as &[u8])) // No stderr
        .success();
    // Display Version
    let mut cmd = Command::cargo_bin("gltfgen").unwrap();
    cmd.arg("-V")
        .assert()
        .stderr(predicate::eq(b"" as &[u8])) // No stderr
        .success();
    // Display Version with long argument
    let mut cmd = Command::cargo_bin("gltfgen").unwrap();
    cmd.arg("--version")
        .assert()
        .stderr(predicate::eq(b"" as &[u8])) // No stderr
        .success();
}
