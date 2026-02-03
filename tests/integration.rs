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
        .arg("--git-root")
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
        .arg("--git-root")
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
        .arg("--git-root")
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
        .stdout(predicate::str::contains("FILE: README.md"))
        .stdout(predicate::str::contains("LINES:"))
        .stdout(predicate::str::contains("HASH:"))
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
        .arg("--git-root")
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
        .arg(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not a git repository"));
}

#[test]
fn test_config_file_support() {
    let tmp = setup_test_repo();
    let config_path = tmp.path().join(".om.toml");
    fs::write(&config_path, "min_score = 8\ndepth = 2\nflat = true\n").unwrap();

    let mut cmd = Command::cargo_bin("om").unwrap();
    cmd.current_dir(tmp.path())
        .arg("tree")
        .arg("--no-color")
        .assert()
        .success()
        .stdout(predicate::str::contains("10 README.md"))
        .stdout(predicate::str::contains("10 main.rs"))
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn test_git_status_filtering() {
    let tmp = setup_test_repo();
    let tmp_path = tmp.path();

    fs::write(tmp_path.join("README.md"), "# Modified README\n").unwrap();
    fs::write(tmp_path.join("new_file.txt"), "new file\n").unwrap();
    fs::write(
        tmp_path.join("main.rs"),
        "fn main() { println!(\"staged\"); }\n",
    )
    .unwrap();
    StdCommand::new("git")
        .args(&["add", "main.rs"])
        .current_dir(tmp_path)
        .output()
        .unwrap();

    let mut cmd = Command::cargo_bin("om").unwrap();
    cmd.arg("tree")
        .arg("--git-root")
        .arg(tmp_path)
        .arg("--dirty")
        .arg("--flat")
        .arg("--no-color")
        .assert()
        .success()
        .stdout(predicate::str::contains("README.md"))
        .stdout(predicate::str::contains("new_file.txt"))
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("lib.rs").not());

    let mut cmd_staged = Command::cargo_bin("om").unwrap();
    cmd_staged
        .arg("tree")
        .arg("--git-root")
        .arg(tmp_path)
        .arg("--staged")
        .arg("--flat")
        .arg("--no-color")
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("README.md").not())
        .stdout(predicate::str::contains("new_file.txt").not());

    let mut cmd_unstaged = Command::cargo_bin("om").unwrap();
    cmd_unstaged
        .arg("tree")
        .arg("--git-root")
        .arg(tmp_path)
        .arg("--unstaged")
        .arg("--flat")
        .arg("--no-color")
        .assert()
        .success()
        .stdout(predicate::str::contains("README.md"))
        .stdout(predicate::str::contains("main.rs").not())
        .stdout(predicate::str::contains("new_file.txt").not());
}

#[test]
fn test_tokens_flag() {
    let tmp = setup_test_repo();

    let mut cmd = Command::cargo_bin("om").unwrap();
    cmd.arg("tree")
        .arg("--git-root")
        .arg(tmp.path())
        .arg("--tokens")
        .arg("--flat")
        .arg("--no-color")
        .assert()
        .success()
        .stdout(predicate::str::contains("README.md (4 tokens)"))
        .stdout(predicate::str::contains("main.rs (4 tokens)"));

    let mut cmd_cat = Command::cargo_bin("om").unwrap();
    cmd_cat
        .arg("cat")
        .arg("--path")
        .arg(tmp.path())
        .arg("--tokens")
        .arg("--level")
        .arg("10")
        .assert()
        .success()
        .stdout(predicate::str::contains("TOKENS: 4"));
}

#[test]
fn test_format_json() {
    let tmp = setup_test_repo();

    let mut cmd = Command::cargo_bin("om").unwrap();
    let output = cmd
        .arg("tree")
        .arg("--git-root")
        .arg(tmp.path())
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let v: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        v["project"],
        tmp.path().file_name().unwrap().to_str().unwrap()
    );
    assert!(v["files"].is_array());

    let mut cmd_cat = Command::cargo_bin("om").unwrap();
    let output_cat = cmd_cat
        .arg("cat")
        .arg("--path")
        .arg(tmp.path())
        .arg("--format")
        .arg("json")
        .arg("--level")
        .arg("10")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let v_cat: serde_json::Value = serde_json::from_slice(&output_cat).unwrap();
    assert!(v_cat["files"].is_array());
    assert!(v_cat["files"][0]["content"].is_string());
}

#[test]
fn test_format_xml() {
    let tmp = setup_test_repo();

    let mut cmd = Command::cargo_bin("om").unwrap();
    cmd.arg("tree")
        .arg("--git-root")
        .arg(tmp.path())
        .arg("--format")
        .arg("xml")
        .assert()
        .success()
        .stdout(predicate::str::contains("<?xml"))
        .stdout(predicate::str::contains("<codebase>"))
        .stdout(predicate::str::contains("<file path=\"README.md\""));

    let mut cmd_cat = Command::cargo_bin("om").unwrap();
    cmd_cat
        .arg("cat")
        .arg("--path")
        .arg(tmp.path())
        .arg("--format")
        .arg("xml")
        .arg("--level")
        .arg("10")
        .assert()
        .success()
        .stdout(predicate::str::contains("<?xml"))
        .stdout(predicate::str::contains("<content><![CDATA[# Test Project"));
}

#[test]
fn test_git_status_flags_json() {
    let tmp = setup_test_repo();
    let tmp_path = tmp.path();

    fs::write(tmp_path.join("dirty.rs"), "fn dirty() {}\n").unwrap();

    let mut cmd = Command::cargo_bin("om").unwrap();
    let output = cmd
        .arg("tree")
        .arg("--git-root")
        .arg(tmp_path)
        .arg("--dirty")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let v: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let files = v["files"].as_array().unwrap();
    assert!(files.iter().any(|f| f["path"] == "dirty.rs"));
    assert!(!files.iter().any(|f| f["path"] == "lib.rs"));
}

#[test]
fn test_config_overrides() {
    let tmp = setup_test_repo();
    let config_path = tmp.path().join(".om.toml");
    fs::write(&config_path, "min_score = 10\n").unwrap();

    let mut cmd = Command::cargo_bin("om").unwrap();
    cmd.current_dir(tmp.path())
        .arg("tree")
        .arg("--no-color")
        .arg("--flat")
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml").not());

    let mut cmd_override = Command::cargo_bin("om").unwrap();
    cmd_override
        .current_dir(tmp.path())
        .arg("tree")
        .arg("--min-score")
        .arg("8")
        .arg("--no-color")
        .arg("--flat")
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn test_rayon_jobs() {
    let tmp = setup_test_repo();

    let mut cmd = Command::cargo_bin("om").unwrap();
    cmd.arg("tree")
        .arg("--git-root")
        .arg(tmp.path())
        .arg("--jobs")
        .arg("2")
        .assert()
        .success();
}
