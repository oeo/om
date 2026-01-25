use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::process::Command as StdCommand;
use tempfile::TempDir;

fn setup_test_repo() -> TempDir {
    let tmp = TempDir::new().unwrap();
    let tmp_path = tmp.path();

    StdCommand::new("git")
        .args(&["init"])
        .current_dir(tmp_path)
        .output()
        .unwrap();

    StdCommand::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .current_dir(tmp_path)
        .output()
        .unwrap();

    StdCommand::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(tmp_path)
        .output()
        .unwrap();

    fs::write(tmp_path.join("README.md"), "# Test Project\n").unwrap();
    fs::write(tmp_path.join("main.rs"), "fn main() {}\n").unwrap();
    fs::write(tmp_path.join("lib.rs"), "pub fn foo() {}\n").unwrap();
    fs::write(tmp_path.join("Cargo.toml"), "[package]\n").unwrap();

    fs::create_dir(tmp_path.join("src")).unwrap();
    fs::write(tmp_path.join("src/handler.rs"), "pub fn handle() {}\n").unwrap();
    fs::write(tmp_path.join("src/utils.rs"), "pub fn util() {}\n").unwrap();

    fs::create_dir(tmp_path.join("tests")).unwrap();
    fs::write(
        tmp_path.join("tests/test_main.rs"),
        "#[test]\nfn test() {}\n",
    )
    .unwrap();

    fs::create_dir(tmp_path.join("vendor")).unwrap();
    fs::write(tmp_path.join("vendor/lib.rs"), "pub fn vendor() {}\n").unwrap();

    StdCommand::new("git")
        .args(&["add", "-A"])
        .current_dir(tmp_path)
        .output()
        .unwrap();

    StdCommand::new("git")
        .args(&["commit", "-m", "initial"])
        .current_dir(tmp_path)
        .output()
        .unwrap();

    tmp
}

#[test]
fn test_tree_basic() {
    let tmp = setup_test_repo();

    let mut cmd = Command::cargo_bin("om").unwrap();
    cmd.arg("tree")
        .arg("--path")
        .arg(tmp.path())
        .arg("--no-color")
        .assert()
        .success()
        .stdout(predicate::str::contains("README.md"))
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn test_tree_flat() {
    let tmp = setup_test_repo();

    let mut cmd = Command::cargo_bin("om").unwrap();
    cmd.arg("tree")
        .arg("--path")
        .arg(tmp.path())
        .arg("--flat")
        .arg("--no-color")
        .assert()
        .success()
        .stdout(predicate::str::contains("10 README.md"))
        .stdout(predicate::str::contains("10 main.rs"));
}

#[test]
fn test_tree_min_score() {
    let tmp = setup_test_repo();

    let mut cmd = Command::cargo_bin("om").unwrap();
    cmd.arg("tree")
        .arg("--path")
        .arg(tmp.path())
        .arg("--min-score")
        .arg("8")
        .arg("--flat")
        .arg("--no-color")
        .assert()
        .success()
        .stdout(predicate::str::contains("README.md"))
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn test_cat_without_session() {
    let tmp = setup_test_repo();

    let mut cmd = Command::cargo_bin("om").unwrap();
    cmd.arg("cat")
        .arg("--path")
        .arg(tmp.path())
        .arg("--level")
        .arg("10")
        .assert()
        .success()
        .stdout(predicate::str::contains("# File: README.md"))
        .stdout(predicate::str::contains("# Test Project"));
}

#[test]
fn test_cat_with_session() {
    let tmp = setup_test_repo();
    let session_name = format!("test-session-{}", std::process::id());

    let mut cmd = Command::cargo_bin("om").unwrap();
    cmd.arg("cat")
        .arg("--path")
        .arg(tmp.path())
        .arg("--level")
        .arg("10")
        .arg("--session")
        .arg(&session_name)
        .assert()
        .success()
        .stdout(predicate::str::contains("# Session:"));

    let mut cmd2 = Command::cargo_bin("om").unwrap();
    cmd2.arg("cat")
        .arg("--path")
        .arg(tmp.path())
        .arg("--level")
        .arg("10")
        .arg("--session")
        .arg(&session_name)
        .assert()
        .success()
        .stdout(predicate::str::contains("unchanged (session)"));

    let mut cmd3 = Command::cargo_bin("om").unwrap();
    cmd3.arg("session")
        .arg("clear")
        .arg(&session_name)
        .assert()
        .success();
}

#[test]
fn test_session_smart_init() {
    let mut cmd = Command::cargo_bin("om").unwrap();
    cmd.arg("session")
        .assert()
        .success()
        .stdout(predicate::str::contains("export OM_SESSION="));
}

#[test]
fn test_session_list() {
    let mut cmd = Command::cargo_bin("om").unwrap();
    cmd.arg("session").arg("list").assert().success();
}

#[test]
fn test_session_show() {
    let tmp = setup_test_repo();
    let session_name = format!("test-show-{}", std::process::id());

    let mut cmd = Command::cargo_bin("om").unwrap();
    cmd.arg("cat")
        .arg("--path")
        .arg(tmp.path())
        .arg("--level")
        .arg("10")
        .arg("--session")
        .arg(&session_name)
        .assert()
        .success();

    let mut cmd2 = Command::cargo_bin("om").unwrap();
    cmd2.arg("session")
        .arg("show")
        .arg(&session_name)
        .assert()
        .success()
        .stdout(predicate::str::contains("tracked files"));

    let mut cmd3 = Command::cargo_bin("om").unwrap();
    cmd3.arg("session")
        .arg("clear")
        .arg(&session_name)
        .assert()
        .success();
}

#[test]
fn test_init_local() {
    let tmp = setup_test_repo();
    let omignore = tmp.path().join(".omignore");

    let mut cmd = Command::cargo_bin("om").unwrap();
    cmd.arg("init")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created local .omignore"));

    assert!(omignore.exists());
}

#[test]
fn test_init_force() {
    let tmp = setup_test_repo();
    let omignore = tmp.path().join(".omignore");

    fs::write(&omignore, "existing content\n").unwrap();

    let mut cmd = Command::cargo_bin("om").unwrap();
    cmd.arg("init")
        .arg("--force")
        .current_dir(tmp.path())
        .assert()
        .success();

    let content = fs::read_to_string(&omignore).unwrap();
    assert!(content.contains("Lock files"));
}

#[test]
fn test_omignore_filtering() {
    let tmp = setup_test_repo();
    let omignore = tmp.path().join(".omignore");

    fs::write(&omignore, "*.lock\n*-lock.*\nvendor/\n").unwrap();
    fs::write(tmp.path().join("package-lock.json"), "{}").unwrap();

    StdCommand::new("git")
        .args(&["add", "-A"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    StdCommand::new("git")
        .args(&["commit", "-m", "add lock"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let mut cmd = Command::cargo_bin("om").unwrap();
    cmd.arg("tree")
        .arg("--path")
        .arg(tmp.path())
        .arg("--flat")
        .arg("--no-color")
        .assert()
        .success()
        .stdout(predicate::str::contains("package-lock.json").not())
        .stdout(predicate::str::contains("vendor").not());
}

#[test]
fn test_not_git_repo() {
    let tmp = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("om").unwrap();
    cmd.arg("tree")
        .arg("--path")
        .arg(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not a git repository"));
}
