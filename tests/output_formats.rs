use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::process::Command as StdCommand;
use tempfile::tempdir;

#[test]
fn test_tree_json_output() {
    let dir = tempdir().unwrap();
    let repo_path = dir.path();

    StdCommand::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    fs::write(repo_path.join("README.md"), "# Test").unwrap();
    fs::write(repo_path.join("main.rs"), "fn main() {}").unwrap();

    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .unwrap();

    let mut cmd = Command::cargo_bin("om").unwrap();
    cmd.current_dir(repo_path)
        .args(["tree", "--format", "json"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(r#""project":"#))
        .stdout(predicate::str::contains(r#""files":"#))
        .stdout(predicate::str::contains(r#""path":"#))
        .stdout(predicate::str::contains(r#""score":"#));

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(serde_json::from_str::<serde_json::Value>(&stdout).is_ok());
}

#[test]
fn test_tree_xml_output() {
    let dir = tempdir().unwrap();
    let repo_path = dir.path();

    StdCommand::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    fs::write(repo_path.join("README.md"), "# Test").unwrap();
    fs::write(repo_path.join("main.rs"), "fn main() {}").unwrap();

    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .unwrap();

    let mut cmd = Command::cargo_bin("om").unwrap();
    cmd.current_dir(repo_path).args(["tree", "--format", "xml"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("<?xml"))
        .stdout(predicate::str::contains("<codebase>"))
        .stdout(predicate::str::contains("<project>"))
        .stdout(predicate::str::contains("<files>"))
        .stdout(predicate::str::contains("<file"));
}

#[test]
fn test_cat_json_output() {
    let dir = tempdir().unwrap();
    let repo_path = dir.path();

    StdCommand::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    fs::write(repo_path.join("test.txt"), "hello world").unwrap();

    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .unwrap();

    let mut cmd = Command::cargo_bin("om").unwrap();
    cmd.current_dir(repo_path)
        .args(["cat", "test.txt", "--format", "json"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(r#""project":"#))
        .stdout(predicate::str::contains(r#""files_shown":"#))
        .stdout(predicate::str::contains(r#""total_lines":"#))
        .stdout(predicate::str::contains(r#""content":"#))
        .stdout(predicate::str::contains("hello world"));

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(serde_json::from_str::<serde_json::Value>(&stdout).is_ok());
}

#[test]
fn test_cat_xml_output() {
    let dir = tempdir().unwrap();
    let repo_path = dir.path();

    StdCommand::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    fs::write(repo_path.join("test.txt"), "hello world").unwrap();

    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .unwrap();

    let mut cmd = Command::cargo_bin("om").unwrap();
    cmd.current_dir(repo_path)
        .args(["cat", "test.txt", "--format", "xml"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("<?xml"))
        .stdout(predicate::str::contains("<codebase>"))
        .stdout(predicate::str::contains("<project>"))
        .stdout(predicate::str::contains("<files_shown>"))
        .stdout(predicate::str::contains("<total_lines>"))
        .stdout(predicate::str::contains("<content>"))
        .stdout(predicate::str::contains("hello world"));
}

#[test]
fn test_invalid_format() {
    let dir = tempdir().unwrap();
    let repo_path = dir.path();

    StdCommand::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    fs::write(repo_path.join("test.txt"), "hello").unwrap();

    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .unwrap();

    let mut cmd = Command::cargo_bin("om").unwrap();
    cmd.current_dir(repo_path)
        .args(["tree", "--format", "invalid"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid format"));
}

#[test]
fn test_default_text_format_unchanged() {
    let dir = tempdir().unwrap();
    let repo_path = dir.path();

    StdCommand::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    fs::write(repo_path.join("README.md"), "# Test").unwrap();

    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .unwrap();

    let mut cmd = Command::cargo_bin("om").unwrap();
    cmd.current_dir(repo_path).args(["tree", "--flat"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("10 README.md"))
        .stdout(predicate::str::contains("<?xml").not())
        .stdout(predicate::str::contains(r#""project""#).not());
}
