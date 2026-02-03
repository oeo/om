use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub enum GitError {
    NotInstalled,
    NotARepo,
    CommandFailed(String),
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitError::NotInstalled => write!(f, "git is not installed"),
            GitError::NotARepo => write!(f, "not a git repository"),
            GitError::CommandFailed(msg) => write!(f, "git command failed: {}", msg),
        }
    }
}

impl std::error::Error for GitError {}

#[derive(Debug, Default)]
pub struct GitStatus {
    pub dirty: HashSet<String>,
    pub staged: HashSet<String>,
    pub unstaged: HashSet<String>,
}

pub fn git_status(root: &Path) -> Result<GitStatus, GitError> {
    let output = Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .current_dir(root)
        .output()
        .map_err(|_| GitError::NotInstalled)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not a git repository") {
            return Err(GitError::NotARepo);
        }
        return Err(GitError::CommandFailed(stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut status = GitStatus::default();

    for line in stdout.lines() {
        if line.len() < 4 {
            continue;
        }

        let x = line.chars().next().unwrap();
        let y = line.chars().nth(1).unwrap();
        let file = line[3..].trim().to_string();

        let is_staged = x != ' ' && x != '?';
        let is_unstaged = y != ' ' && y != '?';
        let is_untracked = x == '?' && y == '?';

        if is_staged {
            status.staged.insert(file.clone());
        }
        if is_unstaged {
            status.unstaged.insert(file.clone());
        }
        if is_staged || is_unstaged || is_untracked {
            status.dirty.insert(file);
        }
    }

    Ok(status)
}

pub fn ls_files(root: &Path) -> Result<Vec<PathBuf>, GitError> {
    let output = Command::new("git")
        .arg("ls-files")
        .arg("--cached")
        .arg("--others")
        .arg("--exclude-standard")
        .current_dir(root)
        .output()
        .map_err(|_| GitError::NotInstalled)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not a git repository") {
            return Err(GitError::NotARepo);
        }
        return Err(GitError::CommandFailed(stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let files = stdout
        .lines()
        .map(|line| PathBuf::from(line.trim()))
        .collect();

    Ok(files)
}

pub fn repo_root(path: &Path) -> Result<PathBuf, GitError> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .current_dir(path)
        .output()
        .map_err(|_| GitError::NotInstalled)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not a git repository") {
            return Err(GitError::NotARepo);
        }
        return Err(GitError::CommandFailed(stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let root = PathBuf::from(stdout.trim());

    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_repo_root() {
        let cwd = env::current_dir().unwrap();
        let root = repo_root(&cwd);
        assert!(root.is_ok());
    }

    #[test]
    fn test_ls_files() {
        let cwd = env::current_dir().unwrap();
        let files = ls_files(&cwd);
        assert!(files.is_ok());
    }

    #[test]
    fn test_git_status() {
        let cwd = env::current_dir().unwrap();
        let status = git_status(&cwd);
        assert!(status.is_ok());
    }
}
