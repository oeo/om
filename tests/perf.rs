use assert_cmd::Command;
use std::fs;
use std::time::Instant;
use tempfile::tempdir;

#[test]
fn perf_scan_10k_files_under_5s() {
    // Create synthetic repo with 10k files
    let dir = tempdir().unwrap();
    for i in 0..10_000u32 {
        let file_path = dir.path().join(format!("file{}.txt", i));
        fs::write(file_path, b"test").unwrap();
    }
    // Init git repo so om ls-files works fast
    Command::new("git")
        .current_dir(dir.path())
        .args(["init", "-q"])
        .assert()
        .success();
    Command::new("git")
        .current_dir(dir.path())
        .args(["add", "."])
        .assert()
        .success();

    let start = Instant::now();
    Command::cargo_bin("om")
        .unwrap()
        .current_dir(dir.path())
        .args(["tree", "--jobs", "4", "--flat", "--no-color"])
        .assert()
        .success();
    let elapsed = start.elapsed();
    assert!(elapsed.as_secs_f32() < 5.0, "scan took {:?}", elapsed);
}
