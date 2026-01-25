# om - Full Spec

## Overview
Feed context to LLMs. Scores project files by importance and outputs what matters.

**Key Features:**
- File importance scoring (1-10)
- Session-based deduplication (prevents re-reading unchanged files)
- .omignore support (local and global)
- Tree and flat output modes
- LLM-optimized output

**Session Management:**
Sessions are optional but recommended for `om cat`. When used, prevents re-reading unchanged files. Set `OM_SESSION` environment variable or use `--session` flag.

---

## Project Structure

```
om/
├── Cargo.toml
├── Makefile
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── cli.rs
│   ├── git.rs
│   ├── session.rs
│   ├── session_cmd.rs
│   ├── ignore.rs
│   ├── scorer.rs
│   ├── tree.rs
│   ├── cat.rs
│   └── init.rs
├── tests/
│   └── integration.rs
├── README.md
├── PROMPT.md
├── LICENSE
├── .gitignore
└── .omignore.example
```

---

## Cargo.toml

```toml
[package]
name = "om"
version = "0.1.0"
edition = "2021"
description = "Feed context to LLMs. Scores project files by importance."
license = "MIT"
repository = "https://github.com/yourname/om"
keywords = ["llm", "context", "cli", "developer-tools"]
categories = ["command-line-utilities", "development-tools"]
readme = "README.md"

[dependencies]
clap = { version = "4", features = ["derive"] }
colored = "2"
glob = "0.3"
lazy_static = "1.4"
dirs = "5"
sha2 = "0.10"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
tempfile = "3"
assert_cmd = "2"
predicates = "3"

[profile.release]
lto = true
strip = true
codegen-units = 1
```

---

## src/main.rs

```rust
mod cli;
mod git;
mod ignore;
mod scorer;
mod session;
mod tree;
mod cat;
mod init;
mod session_cmd;

use clap::Parser;
use cli::{Cli, Commands};
use std::process;

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Tree(args) => tree::run(args),
        Commands::Cat(args) => cat::run(args),
        Commands::Init(args) => init::run(args),
        Commands::Session(args) => session_cmd::run(args),
    };

    if let Err(e) = result {
        eprintln!("error: {}", e);
        process::exit(1);
    }
}
```

---

## src/cli.rs

```rust
use clap::{Parser, Subcommand, Args};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "om")]
#[command(about = "Feed context to LLMs. Scores project files by importance.")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Show project tree with importance scores
    Tree(TreeArgs),

    /// Output file contents for LLM ingestion (requires session)
    Cat(CatArgs),

    /// Initialize .omignore with sensible defaults
    Init(InitArgs),

    /// Session management
    Session(SessionArgs),
}

#[derive(Args)]
pub struct SessionArgs {
    #[command(subcommand)]
    pub command: Option<SessionCommand>,
}

#[derive(Subcommand)]
pub enum SessionCommand {
    /// List all sessions
    List,

    /// Show files tracked in a session
    Show(SessionShowArgs),

    /// Clear a session
    Clear(SessionClearArgs),
}

#[derive(Args)]
pub struct SessionShowArgs {
    /// Session name
    pub name: String,
}

#[derive(Args)]
pub struct SessionClearArgs {
    /// Session name
    pub name: String,
}

#[derive(Args)]
pub struct TreeArgs {
    /// Root path (default: current directory)
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Minimum score to show (1-10)
    #[arg(short = 's', long = "min-score", default_value = "1")]
    pub min_score: u8,

    /// Maximum depth
    #[arg(short = 'L', long = "depth")]
    pub depth: Option<usize>,

    /// Flat list sorted by score
    #[arg(short = 'f', long = "flat")]
    pub flat: bool,

    /// No color output
    #[arg(long = "no-color")]
    pub no_color: bool,
}

#[derive(Args)]
pub struct CatArgs {
    /// Files to cat, or root path if using --level
    #[arg()]
    pub files: Vec<PathBuf>,

    /// Cat all files with score >= level
    #[arg(short = 'l', long = "level")]
    pub level: Option<u8>,

    /// Root path when using --level
    #[arg(short = 'p', long = "path", default_value = ".")]
    pub path: PathBuf,

    /// No headers between files
    #[arg(long = "no-headers")]
    pub no_headers: bool,

    /// Session name (overrides OM_SESSION env var)
    #[arg(short = 's', long = "session")]
    pub session: Option<String>,
}

#[derive(Args)]
pub struct InitArgs {
    /// Create global ~/.omignore instead of local
    #[arg(long = "global")]
    pub global: bool,

    /// Overwrite existing file
    #[arg(short = 'f', long = "force")]
    pub force: bool,
}
```

---

## src/git.rs

```rust
use std::path::Path;
use std::process::Command;
use std::io::{self, ErrorKind};

pub fn ls_files(root: &Path) -> io::Result<Vec<String>> {
    let output = Command::new("git")
        .arg("ls-files")
        .current_dir(root)
        .output()?;

    if !output.status.success() {
        return Err(io::Error::new(
            ErrorKind::Other,
            "not a git repository or git command failed"
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let files: Vec<String> = stdout
        .lines()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    Ok(files)
}

pub fn repo_root(path: &Path) -> io::Result<std::path::PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(path)
        .output()?;

    if !output.status.success() {
        return Err(io::Error::new(ErrorKind::Other, "not in git repo"));
    }

    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(std::path::PathBuf::from(root))
}
```

---

## src/session.rs

```rust
use std::collections::HashMap;
use std::path::PathBuf;
use std::io;
use std::fs;
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Session {
    #[serde(skip)]
    name: String,
    #[serde(skip)]
    session_file: PathBuf,
    hashes: HashMap<String, String>,
}

impl Session {
    /// Load or create session by name
    pub fn load(name: &str) -> io::Result<Self> {
        let session_dir = Self::session_dir()?;
        fs::create_dir_all(&session_dir)?;

        let session_file = session_dir.join(format!("{}.json", name));

        let hashes = if session_file.exists() {
            let content = fs::read_to_string(&session_file)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            HashMap::new()
        };

        Ok(Self {
            name: name.to_string(),
            session_file,
            hashes,
        })
    }

    /// Save session to disk
    pub fn save(&self) -> io::Result<()> {
        let json = serde_json::to_string_pretty(&self.hashes)?;
        fs::write(&self.session_file, json)?;
        Ok(())
    }

    /// Check if file was already read with same hash
    pub fn was_read(&self, path: &str, current_hash: &str) -> bool {
        self.hashes.get(path).map(|h| h == current_hash).unwrap_or(false)
    }

    /// Mark file as read with its hash
    pub fn mark_read(&mut self, path: String, hash: String) {
        self.hashes.insert(path, hash);
    }

    /// Compute SHA256 hash of content
    pub fn compute_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Generate new unique session ID
    pub fn generate_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        format!("sess-{}", timestamp)
    }

    /// List all sessions
    pub fn list_all() -> io::Result<Vec<String>> {
        let session_dir = Self::session_dir()?;

        if !session_dir.exists() {
            return Ok(Vec::new());
        }

        let mut sessions = Vec::new();
        for entry in fs::read_dir(session_dir)? {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".json") {
                    sessions.push(name.trim_end_matches(".json").to_string());
                }
            }
        }

        sessions.sort();
        Ok(sessions)
    }

    /// Show files tracked in session
    pub fn show(&self) -> &HashMap<String, String> {
        &self.hashes
    }

    /// Clear session (delete file)
    pub fn clear(name: &str) -> io::Result<()> {
        let session_dir = Self::session_dir()?;
        let session_file = session_dir.join(format!("{}.json", name));

        if session_file.exists() {
            fs::remove_file(session_file)?;
        }

        Ok(())
    }

    fn session_dir() -> io::Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "cannot find home directory"))?;
        Ok(home.join(".om").join("sessions"))
    }
}
```

---

## src/session_cmd.rs

```rust
use crate::cli::{SessionArgs, SessionCommand, SessionShowArgs, SessionClearArgs};
use crate::session::Session;
use std::io;

pub fn run(args: SessionArgs) -> io::Result<()> {
    match args.command {
        None => smart_init(),
        Some(SessionCommand::List) => list(),
        Some(SessionCommand::Show(args)) => show(args),
        Some(SessionCommand::Clear(args)) => clear(args),
    }
}

fn smart_init() -> io::Result<()> {
    // Check if OM_SESSION is already set
    if let Ok(existing) = std::env::var("OM_SESSION") {
        // Session already active
        println!("echo 'Session already active: {}'", existing);
    } else {
        // Create new session
        let session_id = Session::generate_id();
        let session = Session::load(&session_id)?;
        session.save()?;

        // Output export command for eval
        println!("export OM_SESSION={}; echo 'Session created: {}'", session_id, session_id);
    }

    Ok(())
}

fn list() -> io::Result<()> {
    let sessions = Session::list_all()?;
    if sessions.is_empty() {
        println!("No sessions found");
        println!();
        println!("Create a session with:");
        println!("  om session init");
    } else {
        println!("Sessions:");
        for name in sessions {
            println!("  {}", name);
        }
    }
    Ok(())
}

fn show(args: SessionShowArgs) -> io::Result<()> {
    let session = Session::load(&args.name)?;
    let files = session.show();
    if files.is_empty() {
        println!("Session '{}' is empty", args.name);
    } else {
        println!("Session '{}' - {} files:", args.name, files.len());
        for (path, hash) in files {
            println!("  {} [{}]", path, &hash[..8]);
        }
    }
    Ok(())
}

fn clear(args: SessionClearArgs) -> io::Result<()> {
    Session::clear(&args.name)?;
    println!("Cleared session '{}'", args.name);
    Ok(())
}
```

---

## src/ignore.rs

```rust
use glob::Pattern;
use std::fs;
use std::path::Path;

pub struct IgnorePatterns {
    patterns: Vec<Pattern>,
}

impl IgnorePatterns {
    pub fn load(root: &Path) -> Self {
        let mut patterns = Vec::new();

        // Global ~/.omignore
        if let Some(home) = dirs::home_dir() {
            let global = home.join(".omignore");
            if global.exists() {
                patterns.extend(Self::parse_file(&global));
            }
        }

        // Local .omignore
        let local = root.join(".omignore");
        if local.exists() {
            patterns.extend(Self::parse_file(&local));
        }

        Self { patterns }
    }

    fn parse_file(path: &Path) -> Vec<Pattern> {
        let mut patterns = Vec::new();

        if let Ok(content) = fs::read_to_string(path) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                // Exact pattern
                if let Ok(p) = Pattern::new(line) {
                    patterns.push(p);
                }

                // With **/ prefix for matching anywhere
                if !line.starts_with("**/") {
                    if let Ok(p) = Pattern::new(&format!("**/{}", line)) {
                        patterns.push(p);
                    }
                }

                // Directory patterns (ending with /)
                if line.ends_with('/') {
                    let dir = line.trim_end_matches('/');
                    if let Ok(p) = Pattern::new(&format!("{}/**", dir)) {
                        patterns.push(p);
                    }
                    if let Ok(p) = Pattern::new(&format!("**/{}/**", dir)) {
                        patterns.push(p);
                    }
                }
            }
        }

        patterns
    }

    pub fn is_ignored(&self, path: &str) -> bool {
        for pattern in &self.patterns {
            if pattern.matches(path) {
                return true;
            }
        }
        false
    }
}
```

---

## src/scorer.rs

```rust
use std::collections::{HashSet, HashMap};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ScoredFile {
    pub path: String,
    pub score: u8,
    pub reason: String,
}

lazy_static::lazy_static! {
    static ref ENTRY_POINTS: HashSet<&'static str> = {
        [
            "main.py", "main.go", "main.rs", "main.c", "main.cpp", "main.zig",
            "index.js", "index.ts", "index.jsx", "index.tsx",
            "app.py", "app.js", "app.ts", "app.go",
            "server.py", "server.js", "server.ts", "server.go",
            "cli.py", "cli.js", "cli.ts",
            "mod.rs", "lib.rs",
        ].into_iter().collect()
    };

    static ref PROJECT_FILES: HashMap<&'static str, u8> = {
        [
            ("README.md", 10), ("README", 10), ("README.rst", 10),
            ("Makefile", 8), ("Justfile", 8), ("Taskfile.yml", 8),
            ("Dockerfile", 8), ("docker-compose.yml", 8), ("docker-compose.yaml", 8),
            ("package.json", 8), ("pyproject.toml", 8), ("Cargo.toml", 8),
            ("go.mod", 8), ("Gemfile", 8), ("mix.exs", 8), ("build.gradle", 8),
            ("requirements.txt", 7), ("setup.py", 7), ("setup.cfg", 6),
            (".env.example", 6), (".env.sample", 6),
            ("LICENSE", 4), ("CHANGELOG.md", 5), ("CONTRIBUTING.md", 5),
            (".gitignore", 3), (".dockerignore", 3),
        ].into_iter().collect()
    };

    static ref IMPORTANT_DIRS: HashSet<&'static str> = {
        ["src", "core", "lib", "app", "pkg", "internal", "cmd"].into_iter().collect()
    };

    static ref DOMAIN_DIRS: HashSet<&'static str> = {
        ["api", "server", "client", "models", "services", "handlers",
         "controllers", "routes", "views", "components"].into_iter().collect()
    };

    static ref TEST_DIRS: HashSet<&'static str> = {
        ["test", "tests", "spec", "specs", "__tests__"].into_iter().collect()
    };

    static ref LOW_DIRS: HashSet<&'static str> = {
        ["vendor", "third_party", "fixtures", "mocks", "testdata",
         "docs", "examples", "scripts", "tools", "dist", "build"].into_iter().collect()
    };
}

pub fn score_file(filepath: &str) -> ScoredFile {
    let path = Path::new(filepath);
    let filename = path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let parts: Vec<&str> = filepath.split('/').collect();
    let dirs = &parts[..parts.len().saturating_sub(1)];
    let depth = dirs.len();

    // Entry points
    if ENTRY_POINTS.contains(filename) {
        return ScoredFile {
            path: filepath.to_string(),
            score: 10,
            reason: "entry point".to_string(),
        };
    }

    // Project files
    if let Some(&score) = PROJECT_FILES.get(filename) {
        return ScoredFile {
            path: filepath.to_string(),
            score,
            reason: "project file".to_string(),
        };
    }

    // Config patterns
    let filename_lower = filename.to_lowercase();
    if filename_lower.starts_with("config.") || filename_lower.starts_with("settings.") {
        return ScoredFile {
            path: filepath.to_string(),
            score: 9,
            reason: "config".to_string(),
        };
    }

    // Low patterns
    if filename.ends_with(".lock") ||
       filename.ends_with(".min.js") ||
       filename.ends_with(".min.css") ||
       filename.ends_with(".map") ||
       filename == "__init__.py" ||
       filename.ends_with(".pyc") ||
       filename.ends_with(".mpy") ||
       filename.ends_with(".d.ts") ||
       filename.contains(".generated.") {
        let score = if filename == "__init__.py" { 3 } else { 2 };
        return ScoredFile {
            path: filepath.to_string(),
            score,
            reason: "generated".to_string(),
        };
    }

    // Test patterns
    if filename.starts_with("test_") ||
       filename.ends_with("_test.py") ||
       filename.ends_with("_test.go") ||
       filename.ends_with("_test.rs") ||
       filename.ends_with(".test.js") ||
       filename.ends_with(".test.ts") ||
       filename.ends_with(".spec.js") ||
       filename.ends_with(".spec.ts") {
        return ScoredFile {
            path: filepath.to_string(),
            score: 5,
            reason: "test".to_string(),
        };
    }

    // Base score
    let mut score: i8 = 7;
    let mut reason = String::new();

    // Directory influence
    for dir in dirs {
        let dir_lower = dir.to_lowercase();
        if IMPORTANT_DIRS.contains(dir_lower.as_str()) {
            score = (score + 2).min(10);
            reason = "core".to_string();
            break;
        } else if DOMAIN_DIRS.contains(dir_lower.as_str()) {
            score = (score + 1).min(10);
            reason = "domain".to_string();
            break;
        } else if TEST_DIRS.contains(dir_lower.as_str()) {
            score = (score - 2).max(1);
            reason = "test".to_string();
            break;
        } else if LOW_DIRS.contains(dir_lower.as_str()) {
            score = (score - 3).max(1);
            reason = "peripheral".to_string();
            break;
        }
    }

    // Depth influence
    if depth == 0 {
        score = (score + 1).min(10);
        if reason.is_empty() {
            reason = "root".to_string();
        }
    } else if depth > 4 {
        score = (score - 2).max(1);
        if reason.is_empty() {
            reason = "deep".to_string();
        }
    } else if depth > 2 {
        score = (score - 1).max(1);
    }

    // Extension bonuses
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    if ["proto", "graphql", "gql", "thrift"].contains(&ext) {
        score = (score + 1).min(10);
        reason = "schema".to_string();
    }

    if ["md", "rst"].contains(&ext) && !filename.to_uppercase().starts_with("README") {
        score = (score - 1).max(1);
        if reason.is_empty() {
            reason = "docs".to_string();
        }
    }

    ScoredFile {
        path: filepath.to_string(),
        score: score as u8,
        reason,
    }
}

pub fn score_files(files: Vec<String>) -> Vec<ScoredFile> {
    files.into_iter().map(|f| score_file(&f)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_points() {
        assert_eq!(score_file("main.rs").score, 10);
        assert_eq!(score_file("index.ts").score, 10);
        assert_eq!(score_file("lib.rs").score, 10);
    }

    #[test]
    fn test_readme() {
        assert_eq!(score_file("README.md").score, 10);
    }

    #[test]
    fn test_config() {
        assert_eq!(score_file("config.toml").score, 9);
        assert_eq!(score_file("settings.py").score, 9);
    }

    #[test]
    fn test_project_files() {
        assert_eq!(score_file("Cargo.toml").score, 8);
        assert_eq!(score_file("package.json").score, 8);
    }

    #[test]
    fn test_test_files() {
        assert_eq!(score_file("test_main.py").score, 5);
        assert_eq!(score_file("foo.test.ts").score, 5);
    }

    #[test]
    fn test_generated() {
        assert_eq!(score_file("package.lock").score, 2);
        assert_eq!(score_file("bundle.min.js").score, 2);
    }

    #[test]
    fn test_core_dir_boost() {
        let score = score_file("src/foo.rs").score;
        assert!(score >= 8); // base 7 + 2 for src dir - depth penalty
    }

    #[test]
    fn test_test_dir_penalty() {
        let score = score_file("tests/helper.rs").score;
        assert!(score <= 5);
    }
}
```

---

## src/tree.rs

```rust
use crate::cli::TreeArgs;
use crate::git;
use crate::ignore::IgnorePatterns;
use crate::scorer::{score_files, ScoredFile};
use colored::*;
use std::collections::BTreeMap;
use std::io;

pub fn run(args: TreeArgs) -> io::Result<()> {
    let files = git::ls_files(&args.path)?;
    let ignore = IgnorePatterns::load(&args.path);

    let files: Vec<String> = files
        .into_iter()
        .filter(|f| !ignore.is_ignored(f))
        .collect();

    let mut scored = score_files(files);
    scored.retain(|f| f.score >= args.min_score);

    if args.flat {
        print_flat(&scored, args.no_color);
    } else {
        print_tree(&scored, args.depth, args.no_color);
    }

    Ok(())
}

fn print_flat(files: &[ScoredFile], no_color: bool) {
    let mut sorted = files.to_vec();
    sorted.sort_by(|a, b| b.score.cmp(&a.score).then(a.path.cmp(&b.path)));

    for f in sorted {
        if no_color {
            println!("{:2}  {:60} {}", f.score, f.path, f.reason);
        } else {
            let score_str = format!("{:2}", f.score);
            let colored_score = match f.score {
                8..=10 => score_str.green(),
                5..=7 => score_str.yellow(),
                _ => score_str.dimmed(),
            };
            println!("{}  {:60} {}", colored_score, f.path, f.reason.dimmed());
        }
    }
}

#[derive(Default)]
struct TreeNode {
    children: BTreeMap<String, TreeNode>,
    file: Option<ScoredFile>,
}

fn build_tree(files: &[ScoredFile]) -> TreeNode {
    let mut root = TreeNode::default();

    for f in files {
        let parts: Vec<&str> = f.path.split('/').collect();
        let mut node = &mut root;

        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                node.children.entry(part.to_string()).or_default().file = Some(f.clone());
            } else {
                node = node.children.entry(part.to_string()).or_default();
            }
        }
    }

    root
}

fn print_tree(files: &[ScoredFile], max_depth: Option<usize>, no_color: bool) {
    let tree = build_tree(files);

    if no_color {
        println!(".");
    } else {
        println!("{}", ".".blue().bold());
    }

    print_node(&tree, "", max_depth, 0, no_color);
}

fn print_node(node: &TreeNode, prefix: &str, max_depth: Option<usize>, depth: usize, no_color: bool) {
    if let Some(max) = max_depth {
        if depth > max {
            return;
        }
    }

    // Separate dirs and files, sort by score
    let mut dirs: Vec<_> = node.children.iter()
        .filter(|(_, n)| n.file.is_none())
        .map(|(name, n)| (name, n, get_max_score(n)))
        .collect();

    let mut files: Vec<_> = node.children.iter()
        .filter(|(_, n)| n.file.is_some())
        .map(|(name, n)| (name, n.file.as_ref().unwrap()))
        .collect();

    dirs.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(b.0)));
    files.sort_by(|a, b| b.1.score.cmp(&a.1.score).then(a.0.cmp(b.0)));

    let total = dirs.len() + files.len();
    let mut idx = 0;

    for (name, child, _) in dirs {
        idx += 1;
        let is_last = idx == total;
        let connector = if is_last { "└── " } else { "├── " };

        if no_color {
            println!("{}{}{}/", prefix, connector, name);
        } else {
            println!("{}{}{}/", prefix, connector, name.blue().bold());
        }

        let new_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });
        print_node(child, &new_prefix, max_depth, depth + 1, no_color);
    }

    for (name, file) in files {
        idx += 1;
        let is_last = idx == total;
        let connector = if is_last { "└── " } else { "├── " };

        if no_color {
            println!("{}{}{} [{}] {}", prefix, connector, name, file.score, file.reason);
        } else {
            let score_str = format!("[{}]", file.score);
            let colored_score = match file.score {
                8..=10 => score_str.green(),
                5..=7 => score_str.yellow(),
                _ => score_str.dimmed(),
            };
            println!("{}{}{} {} {}", prefix, connector, name, colored_score, file.reason.dimmed());
        }
    }
}

fn get_max_score(node: &TreeNode) -> u8 {
    let file_score = node.file.as_ref().map(|f| f.score).unwrap_or(0);
    let child_max = node.children.values().map(get_max_score).max().unwrap_or(0);
    file_score.max(child_max)
}
```

---

## src/cat.rs

```rust
use crate::cli::CatArgs;
use crate::git;
use crate::ignore::IgnorePatterns;
use crate::scorer::{score_files, ScoredFile};
use crate::session::Session;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::Path;

const MAX_SIZE: u64 = 100 * 1024; // 100kb

const BINARY_EXT: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "ico", "webp", "avif", "bmp", "svg",
    "mp3", "mp4", "wav", "ogg", "webm", "mov", "avi",
    "zip", "tar", "gz", "rar", "7z", "bz2",
    "pdf", "doc", "docx", "xls", "xlsx", "ppt",
    "exe", "dll", "so", "dylib", "o", "a",
    "woff", "woff2", "ttf", "otf", "eot",
    "pyc", "pyo", "class", "beam",
    "sqlite", "db",
];

pub fn run(args: CatArgs) -> io::Result<()> {
    // Get session name from flag or env var (optional)
    let session_name = args.session
        .or_else(|| std::env::var("OM_SESSION").ok());

    if let Some(level) = args.level {
        cat_by_level(&args.path, level, args.no_headers, session_name.as_deref())
    } else if !args.files.is_empty() {
        cat_files(&args.files, args.no_headers, session_name.as_deref())
    } else {
        cat_by_level(&args.path, 1, args.no_headers, session_name.as_deref())
    }
}

fn cat_by_level(root: &Path, min_score: u8, no_headers: bool, session_name: Option<&str>) -> io::Result<()> {
    let mut session = session_name.map(|name| Session::load(name)).transpose()?;

    let files = git::ls_files(root)?;
    let ignore = IgnorePatterns::load(root);

    let files: Vec<String> = files
        .into_iter()
        .filter(|f| !ignore.is_ignored(f))
        .collect();

    let mut scored = score_files(files);
    scored.retain(|f| f.score >= min_score);
    scored.sort_by(|a, b| b.score.cmp(&a.score).then(a.path.cmp(&b.path)));

    let project_name = root.canonicalize()
        .ok()
        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
        .unwrap_or_else(|| "project".to_string());

    let mut output_files = Vec::new();
    let mut skipped_binary = 0;
    let mut skipped_session = 0;

    for f in &scored {
        let full_path = root.join(&f.path);
        if !is_text_file(&full_path) {
            skipped_binary += 1;
            continue;
        }

        match fs::read_to_string(&full_path) {
            Ok(content) => {
                // Only check session if we have one
                if let Some(ref sess) = session {
                    let hash = Session::compute_hash(&content);
                    if sess.was_read(&f.path, &hash) {
                        skipped_session += 1;
                        continue;
                    }
                    output_files.push((f, content, Some(hash)));
                } else {
                    output_files.push((f, content, None));
                }
            }
            Err(_) => skipped_binary += 1,
        }
    }

    // Header
    let total_lines: usize = output_files.iter()
        .map(|(_, c, _)| c.lines().count())
        .sum();

    println!("# Project: {}", project_name);
    if let Some(name) = session_name {
        println!("# Session: {}", name);
    }
    println!("# Files: {} (score >= {})", output_files.len(), min_score);
    if skipped_binary > 0 {
        println!("# Skipped: {} (binary or too large)", skipped_binary);
    }
    if skipped_session > 0 {
        println!("# Skipped: {} (already read in session)", skipped_session);
    }
    println!("# Total lines: {}", total_lines);
    println!();

    for (f, content, hash) in output_files {
        if !no_headers {
            println!("{}", "=".repeat(60));
            println!("FILE: {}", f.path);
            println!("SCORE: {}", f.score);
            println!("{}", "=".repeat(60));
        }
        print!("{}", content);
        if !content.ends_with('\n') {
            println!();
        }
        println!();

        // Only mark as read if we have a session
        if let (Some(ref mut sess), Some(h)) = (&mut session, hash) {
            sess.mark_read(f.path.clone(), h);
        }
    }

    if let Some(sess) = session {
        sess.save()?;
    }
    Ok(())
}

fn cat_files(files: &[std::path::PathBuf], no_headers: bool, session_name: Option<&str>) -> io::Result<()> {
    let mut session = session_name.map(|name| Session::load(name)).transpose()?;

    for path in files {
        if !path.exists() {
            eprintln!("warning: {} not found", path.display());
            continue;
        }

        let content = fs::read_to_string(path)?;
        let path_str = path.to_string_lossy().to_string();

        // Only check session if we have one
        if let Some(ref sess) = session {
            let hash = Session::compute_hash(&content);
            if sess.was_read(&path_str, &hash) {
                println!("# Skipped: {} (already read in session)", path_str);
                continue;
            }
        }

        if !no_headers {
            println!("{}", "=".repeat(60));
            println!("FILE: {}", path.display());
            println!("{}", "=".repeat(60));
        }
        print!("{}", content);
        if !content.ends_with('\n') {
            println!();
        }
        println!();

        // Only mark as read if we have a session
        if let Some(ref mut sess) = session {
            let hash = Session::compute_hash(&content);
            sess.mark_read(path_str, hash);
        }
    }

    if let Some(sess) = session {
        sess.save()?;
    }
    Ok(())
}

fn is_text_file(path: &Path) -> bool {
    // Check extension
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        if BINARY_EXT.contains(&ext.to_lowercase().as_str()) {
            return false;
        }
    }

    // Check size
    if let Ok(meta) = fs::metadata(path) {
        if meta.len() > MAX_SIZE || meta.len() == 0 {
            return false;
        }
    } else {
        return false;
    }

    true
}
```

---

## src/init.rs

```rust
use crate::cli::InitArgs;
use std::fs;
use std::io;
use std::path::PathBuf;

const DEFAULT_OMIGNORE: &str = r#"# om ignore file
# Same syntax as .gitignore

# Lock files (usually noise)
*.lock
package-lock.json
yarn.lock
pnpm-lock.yaml
Cargo.lock
poetry.lock
Gemfile.lock

# Generated files
*.generated.*
*_generated.*
*.min.js
*.min.css
*.map
*.d.ts

# Build output
dist/
build/
out/

# Large changelogs
CHANGELOG.md
HISTORY.md

# Editor configs
.editorconfig
.prettierrc*
.eslintrc*
"#;

pub fn run(args: InitArgs) -> io::Result<()> {
    let path = if args.global {
        dirs::home_dir()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "cannot find home directory"))?
            .join(".omignore")
    } else {
        PathBuf::from(".omignore")
    };

    if path.exists() && !args.force {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} already exists (use -f to overwrite)", path.display())
        ));
    }

    fs::write(&path, DEFAULT_OMIGNORE)?;
    println!("Created {}", path.display());

    Ok(())
}
```

---

## src/lib.rs

```rust
//! om - Feed context to LLMs
//!
//! Score project files by importance and output what matters.

mod git;
mod ignore;
mod scorer;
mod session;

pub use git::{ls_files, repo_root};
pub use ignore::IgnorePatterns;
pub use scorer::{score_file, score_files, ScoredFile};
pub use session::Session;

/// Score and filter files from a git repository
pub fn get_context(root: &std::path::Path, min_score: u8) -> std::io::Result<Vec<ScoredFile>> {
    let files = ls_files(root)?;
    let ignore = IgnorePatterns::load(root);

    let files: Vec<String> = files
        .into_iter()
        .filter(|f| !ignore.is_ignored(f))
        .collect();

    let mut scored = score_files(files);
    scored.retain(|f| f.score >= min_score);
    scored.sort_by(|a, b| b.score.cmp(&a.score).then(a.path.cmp(&b.path)));

    Ok(scored)
}
```

---

## Complete Cargo.toml

```toml
[package]
name = "om"
version = "0.1.0"
edition = "2021"
description = "Feed context to LLMs. Scores project files by importance."
license = "MIT"
repository = "https://github.com/yourname/om"
keywords = ["llm", "context", "cli", "developer-tools"]
categories = ["command-line-utilities", "development-tools"]
readme = "README.md"

[dependencies]
clap = { version = "4", features = ["derive"] }
colored = "2"
glob = "0.3"
lazy_static = "1.4"
dirs = "5"
sha2 = "0.10"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
tempfile = "3"
assert_cmd = "2"
predicates = "3"

[profile.release]
lto = true
strip = true
codegen-units = 1
```
