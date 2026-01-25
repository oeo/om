# om - Complete Implementation Plan (Simplified)

## Overview
Build `om` - a CLI tool that scores project files by importance and feeds optimal context to LLMs.

**Simplified for MVP:**
- No upgrade command (add later when published)
- No shell completions (add later)
- No JSON output (not needed for LLM use case)
- Sessions OPTIONAL for `om cat` (but recommended for deduplication)

## Prerequisites
- Rust toolchain (cargo, rustc)
- Git installed (required for `git ls-files` command)

## Key Features
- File importance scoring (1-10)
- Session-based deduplication (hash tracking to prevent re-reading unchanged files)
- .omignore support (local and global)
- Tree and flat output modes
- LLM-optimized output

## Session Design
- Sessions are OPTIONAL for `om cat` (recommended for deduplication)
- `om session` is smart:
  - If `OM_SESSION` NOT set: creates new session and outputs export command
  - If `OM_SESSION` already set: confirms session is active
- Use with `eval $(om session)` to automatically create/check session
- Can also use `--session` flag to override
- If no session: files are output normally (no deduplication)
- If session provided: tracks and skips unchanged files

## Execution Strategy
Execute steps sequentially. Each step is self-contained and testable. No step depends on future steps.

---

## Phase 1: Project Foundation

### Step 1: Initialize Cargo Project
**Goal:** Create basic Rust project structure with all dependencies

**Actions:**
1. Verify we're in `/Users/taky/www/om`
2. Run `cargo init --name om` (if not already done)
3. Update `Cargo.toml` with complete dependencies and metadata
4. Create `.gitignore` for Rust

**Cargo.toml content:**
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

**.gitignore content:**
```
/target/
Cargo.lock
/releases/
.omignore
```

**Files Created:**
- `Cargo.toml`
- `.gitignore`

**Verification:**
- `cargo build` succeeds

---

## Phase 2: Core Modules (Bottom-Up)

### Step 2: Git Module (`src/git.rs`)
**Goal:** Wrapper for git commands to list files and find repo root

**Implementation:**
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

**Error Handling:**
- If git not found: "git is not installed or not in PATH"
- If not a git repo: "not a git repository"

**Files Created:**
- `src/git.rs`

**Verification:**
- Module compiles
- Clear error messages

---

### Step 3: Session Module (`src/session.rs`)
**Goal:** Track file hashes to prevent re-reading unchanged files

**Implementation:** (See SPEC.md lines 284-404)

**Key Functions:**
- `load(name: &str)` - Load or create session
- `save()` - Persist to `~/.om/sessions/{name}.json`
- `was_read(path, hash)` - Check if file unchanged
- `mark_read(path, hash)` - Track file
- `compute_hash(content)` - SHA256 hex string
- `generate_id()` - Create `sess-{timestamp}` ID
- `list_all()` - List all sessions
- `show()` - Show files in session
- `clear(name)` - Delete session

**Files Created:**
- `src/session.rs`

**Verification:**
- Module compiles
- Hash computation is consistent

---

### Step 4: Ignore Module (`src/ignore.rs`)
**Goal:** Parse .omignore files (local and global) and check if paths match

**Implementation:** (See SPEC.md lines 478-557)

**Logic:**
- Load `~/.omignore` (global) if exists
- Load `.omignore` (local) if exists
- Parse glob patterns (skip comments/empty lines)
- Match against full path, filename, components

**Files Created:**
- `src/ignore.rs`

**Verification:**
- Compiles independently
- Pattern matching works

---

### Step 5: Scorer Module (`src/scorer.rs`)
**Goal:** Score files 1-10 based on importance using pattern matching

**Implementation:** (See SPEC.md lines 561-808)

**Scoring Rules:**
- Entry points (main.*, index.*, etc.): 10
- README: 10
- Config files (config.*, settings.*): 9
- Project files (Cargo.toml, package.json, etc.): 8
- Test files: 5
- Generated files (*.lock, *.min.js, etc.): 2
- Base score: 7, with directory/depth modifiers

**Unit Tests:**
```rust
#[cfg(test)]
mod tests {
    #[test] fn test_entry_points() { ... }
    #[test] fn test_readme() { ... }
    #[test] fn test_config() { ... }
    #[test] fn test_project_files() { ... }
    #[test] fn test_test_files() { ... }
    #[test] fn test_generated() { ... }
    #[test] fn test_core_dir_boost() { ... }
    #[test] fn test_test_dir_penalty() { ... }
}
```

**Files Created:**
- `src/scorer.rs`

**Verification:**
- `cargo test` runs scorer tests
- All tests pass

---

## Phase 3: Output Modules

### Step 6: Tree Module (`src/tree.rs`)
**Goal:** Display files as tree or flat list with scores/colors

**Implementation:** (See SPEC.md lines 812-963)

**Functions:**
- `run(args)` - Main entry point
- `print_flat()` - Flat sorted list
- `print_tree()` - Tree with nested structure
- `build_tree()` - Convert flat list to tree
- `print_node()` - Recursive tree printer
- `get_max_score()` - For directory sorting

**Display:**
- Flat: `{score:2}  {path:60} {reason}`
- Tree: `├── {name} [{score}] {reason}`
- Colors: green (8-10), yellow (5-7), dimmed (1-4)

**Files Created:**
- `src/tree.rs`

**Verification:**
- Compiles with TreeArgs
- Visual output looks good

---

### Step 7: Cat Module (`src/cat.rs`)
**Goal:** Output file contents with headers, filter by score, optional session for deduplication

**Implementation:** (See SPEC.md lines 990-1140)

**Session Handling:**
```rust
// Get session name from flag or env var (optional)
let session_name = args.session
    .or_else(|| std::env::var("OM_SESSION").ok());
```

**Logic:**
- If session provided: load it
- For each file:
  - Read content
  - If session exists:
    - Compute hash
    - Check `was_read(path, hash)`
    - If already read: skip with message
    - Else: output and `mark_read()`
  - If no session: just output
- If session exists: save it

**Output Header:**
```
# Project: {name}
# Session: {session_id}  (only if session provided)
# Files: {count} (score >= {level})
# Skipped: {binary_count} (binary or too large)
# Skipped: {session_count} (already read in session)  (only if session used)
# Total lines: {total}
```

**Files Created:**
- `src/cat.rs`

**Verification:**
- Works without session (normal output)
- Works with session (deduplication)
- Session tracking works correctly

---

### Step 8: Init Module (`src/init.rs`)
**Goal:** Create .omignore with sensible defaults

**Implementation:** (See SPEC.md lines 1155-1219)

**Default Template:**
Lock files, generated files, build output, changelogs, editor configs

**Files Created:**
- `src/init.rs`

**Verification:**
- Creates .omignore correctly
- --global flag works

---

## Phase 4: CLI Layer

### Step 9: CLI Module (`src/cli.rs`)
**Goal:** Define all CLI arguments and commands using clap

**Implementation:** (See SPEC.md lines 120-233)

**Commands:**
- `Tree(TreeArgs)` - Show tree
- `Cat(CatArgs)` - Output files (optional session)
- `Init(InitArgs)` - Create .omignore
- `Session(SessionArgs)` - Manage sessions

**SessionArgs:**
- `command: Option<SessionCommand>` - Optional subcommand
- If `None`: smart init (create session if not set, or confirm if already set)

**SessionCommand:**
- `List` - List all sessions
- `Show(name)` - Show files in session
- `Clear(name)` - Clear session

**Files Created:**
- `src/cli.rs`

**Verification:**
- All argument types defined
- Compiles with clap

---

### Step 10: Session Command Module (`src/session_cmd.rs`)
**Goal:** Implement session management commands

**Implementation:** (See SPEC.md lines 414-443)

**`om session` (smart init) behavior:**
- If `OM_SESSION` NOT set:
  ```
  export OM_SESSION=sess-1706112622; echo 'Session created: sess-1706112622'
  ```
- If `OM_SESSION` already set:
  ```
  echo 'Session already active: sess-1706112622'
  ```

**Usage:**
```bash
eval $(om session)
# Automatically creates session if needed, or confirms if exists
```

**`om session list` output:**
```
Sessions:
  sess-1706112622
  sess-1706112890
```

**Files Created:**
- `src/session_cmd.rs`

**Verification:**
- `om session init` creates unique ID
- List/show/clear work

---

### Step 11: Library Module (`src/lib.rs`)
**Goal:** Expose public API for programmatic use

**Implementation:** (See SPEC.md lines 1223-1256)

**Exports:**
- `ls_files`, `repo_root` from git
- `IgnorePatterns` from ignore
- `score_file`, `score_files`, `ScoredFile` from scorer
- `Session` from session
- `get_context()` - High-level API

**Files Created:**
- `src/lib.rs`

**Verification:**
- Compiles
- Public API is clean

---

### Step 12: Main Module (`src/main.rs`)
**Goal:** Entry point that dispatches to commands

**Implementation:** (See SPEC.md lines 84-116)

**Logic:**
```rust
fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Tree(args) => tree::run(args),
        Commands::Cat(args) => cat::run(args),
        Commands::Init(args) => init::run(args),
        Commands::Session(cmd) => session_cmd::run(cmd),
    };

    if let Err(e) = result {
        eprintln!("error: {}", e);
        process::exit(1);
    }
}
```

**Files Modified:**
- `src/main.rs`

**Verification:**
- `cargo build --release` succeeds
- `./target/release/om --help` works
- All subcommands listed

---

## Phase 5: Testing

### Step 13: Integration Tests (`tests/integration.rs`)
**Goal:** Test full workflows with real git repos

**Test Structure:**
```rust
fn setup_test_repo() -> TempDir {
    // Create temp dir
    // git init
    // Create test files (README.md, main.py, config.toml, src/, tests/)
    // git add .
    // return temp dir
}

#[test] fn test_tree_basic()
#[test] fn test_tree_flat()
#[test] fn test_tree_min_score()
#[test] fn test_cat_without_session()
#[test] fn test_cat_with_session()
#[test] fn test_session_deduplication()
#[test] fn test_session_smart_init()
#[test] fn test_session_already_active()
#[test] fn test_session_list()
#[test] fn test_session_clear()
#[test] fn test_omignore()
#[test] fn test_not_git_repo()
#[test] fn test_init_local()
#[test] fn test_init_force_overwrite()
```

**Session Tests:**
```rust
#[test]
fn test_cat_without_session() {
    let dir = setup_test_repo();

    // Should work without session (no deduplication)
    Command::cargo_bin("om").unwrap()
        .args(["cat", "-l", "10"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("# Test Project"));
}

#[test]
fn test_session_deduplication() {
    let dir = setup_test_repo();

    // First cat with session
    Command::cargo_bin("om").unwrap()
        .args(["cat", "-l", "10", "--session", "test-sess"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("# Test Project"));

    // Second cat should skip unchanged files
    Command::cargo_bin("om").unwrap()
        .args(["cat", "-l", "10", "--session", "test-sess"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Skipped: 2 (already read)"));
}
```

**Files Created:**
- `tests/integration.rs`

**Verification:**
- `cargo test` passes all tests
- Session tracking works
- Error cases handled

---

## Phase 6: Documentation

### Step 14: README.md
**Goal:** User-facing documentation

**Content:**
```markdown
# om

Feed context to LLMs. Scores project files by importance.

## Install

```bash
cargo install --path .
```

## Usage

### Session Management (Optional, but recommended)

Sessions enable smart deduplication - only reading new or changed files.

```bash
# Create and set session (smart command)
eval $(om session)
# Output: Session created: sess-1706112622

# Run again - confirms session is active
eval $(om session)
# Output: Session already active: sess-1706112622

# List all sessions
om session list

# Show files in session
om session show sess-1706112622

# Clear session
om session clear sess-1706112622
```

### Tree view

```bash
om tree                   # full tree
om tree -s 7              # score >= 7
om tree -L 2              # max depth 2
om tree -f                # flat list
om tree --no-color        # no colors
```

### Cat files

```bash
# Without session (normal output, no deduplication)
om cat -l 7               # cat files score >= 7
om cat -l 9               # critical files only
om cat README.md          # specific file
om cat src/*.rs           # multiple files

# With session (smart deduplication)
eval $(om session)        # create and set session
om cat -l 7               # first time: outputs files
om cat -l 5               # subsequent: skips unchanged files

# With explicit session (overrides OM_SESSION)
om cat -l 7 --session my-session
```

### Init .omignore

```bash
om init                   # create .omignore
om init --global          # create ~/.omignore
om init -f                # force overwrite
```

## Scores

| Score | Meaning | Examples |
|-------|---------|----------|
| 10 | Entry points, README | `main.rs`, `index.ts`, `README.md` |
| 9 | Config | `config.toml`, `settings.py` |
| 8 | Project files | `Cargo.toml`, `package.json` |
| 7 | Regular source | Most code files |
| 5 | Tests | `test_*.py`, `*.spec.ts` |
| 3-4 | Docs, scripts | `docs/`, `scripts/` |
| 1-2 | Generated | `*.lock`, `*.min.js` |

## LLM Integration

See [PROMPT.md](PROMPT.md) for system prompt.

## License

MIT
```

**Files Created:**
- `README.md`

**Verification:**
- All commands documented
- Session workflow clear

---

### Step 15: PROMPT.md
**Goal:** LLM system prompt and meditation protocol

**Content:**
````markdown
# om - LLM System Prompt

## System Prompt for LLM

```
You have access to `om` for understanding project structure.

### Session Setup (Recommended for efficient token usage)

Sessions enable smart deduplication - only reading new or changed files.

Before your first `om cat` command:

1. Create and set a session:
   ```
   eval $(om session)
   ```
   Output: `Session created: sess-1706112622`

2. The session is now active. If you run `eval $(om session)` again, it will confirm the session is active.

**Note:** You CAN use `om cat` without a session, but it won't skip already-read files.

### Commands

om tree                # show project structure with scores
om tree -f             # flat list sorted by score
om tree -s N           # filter to score >= N
om cat -l N            # output files scoring >= N (requires session)
om cat <file>          # read specific file (requires session)

### Scores

10 = Entry points, README
9  = Config files
8  = Project definition files
7  = Regular source code
5  = Tests
3-4 = Docs, scripts
1-2 = Generated, lock files

### Session Behavior

When you run `om cat`:
- First time: All files are output
- Subsequent times: Only NEW or CHANGED files are output
- Unchanged files: Skipped with message "(already read in session)"

This prevents wasting tokens re-reading unchanged files.

### Meditation Protocol

**"meditate to level N"** means:

1. `om tree -f` - Get overview
2. `om cat -l N` - Ingest files at level N+
3. Summarize what you learned

**Meditation Levels:**
- Level 9: Quick overview (README, config, project files)
- Level 7: Working context (includes core source)
- Level 5: Deep understanding (includes tests, domain logic)

**Default:** "meditate" = level 7

### Workflow Example

User: "meditate to level 7"

You:
```
om tree -f
om cat -l 7
```

User: "go deeper on the API layer"

You:
```
om cat api/server.ts api/routes.ts
```

### Important

- Use `eval $(om session)` at start of meditation for smart deduplication
- `om session` is idempotent - safe to run multiple times
- Session persists across conversation
- Don't ask permission to read files - just read them
- Can use `om cat` without session if you prefer
```
````

**Files Created:**
- `PROMPT.md`

**Verification:**
- Instructions clear
- Session workflow explained

---

### Step 16: LICENSE
**Goal:** MIT license

**Content:**
```
MIT License

Copyright (c) 2024

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

**Files Created:**
- `LICENSE`

**Verification:**
- Standard MIT text

---

### Step 17: Makefile
**Goal:** Build and release automation

**Content:**
```makefile
.PHONY: help build build-release test lint fmt check install clean release

help:
	@echo "Available targets:"
	@echo "  build         - Build debug binary"
	@echo "  build-release - Build optimized binary"
	@echo "  test          - Run all tests"
	@echo "  lint          - Run clippy"
	@echo "  fmt           - Format code"
	@echo "  check         - Run fmt + lint + test"
	@echo "  install       - Install locally"
	@echo "  clean         - Clean build artifacts"
	@echo "  release       - Build release binaries"

build:
	cargo build

build-release:
	cargo build --release

test:
	cargo test

lint:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

check: fmt lint test

install:
	cargo install --path .

clean:
	cargo clean

release:
	@echo "Building release binary..."
	@mkdir -p releases
	cargo build --release
	tar czf releases/om-macos-$(shell uname -m).tar.gz -C target/release om
	@echo "Release binary created in releases/"
```

**Files Created:**
- `Makefile`

**Verification:**
- `make help` shows targets
- `make check` runs all checks

---

## Phase 7: Additional Files

### Step 18: Example .omignore
**Goal:** Document .omignore usage

**Content:**
```gitignore
# Example .omignore for your project

# Lock files
*.lock
package-lock.json
Cargo.lock

# Generated code
*.generated.*
*_generated.*

# Build output
dist/
build/

# Changelogs
CHANGELOG.md

# Vendor dependencies
vendor/
third_party/
```

**Files Created:**
- `.omignore.example`

**Verification:**
- Patterns are valid

---

## Phase 8: Final Verification

### Step 19: End-to-End Testing
**Goal:** Verify everything works in real usage

**Manual Test Checklist:**

In `/Users/taky/www/om`:
- [ ] `cargo build --release`
- [ ] `./target/release/om --help` - help text
- [ ] `./target/release/om tree` - shows files
- [ ] `./target/release/om tree -f` - flat list
- [ ] `./target/release/om tree -s 8` - filtering
- [ ] `./target/release/om cat -l 7` - works without session
- [ ] `eval $(./target/release/om session)` - creates and sets OM_SESSION
- [ ] `./target/release/om cat -l 9` - outputs files
- [ ] `./target/release/om cat -l 9` - skips already-read
- [ ] `eval $(./target/release/om session)` - confirms session active
- [ ] `./target/release/om session list` - shows sessions
- [ ] `./target/release/om session show sess-...` - shows files
- [ ] `./target/release/om session clear sess-...` - clears
- [ ] `./target/release/om cat -l 7 --session test` - explicit session
- [ ] `./target/release/om init` - creates .omignore
- [ ] Verify colors (green=10, yellow=7)
- [ ] `./target/release/om tree --no-color` - no colors
- [ ] `cargo test` - all tests pass

**Error Cases:**
- [ ] `om tree /tmp` - error if not git repo
- [ ] `om init` twice - error on second

**Verification:**
- All commands work
- Scores make sense
- Session tracking works
- Errors are clear

---

### Step 20: Git Setup
**Goal:** Clean repository ready for commit

**Actions:**
1. Ensure `.gitignore` contains:
   ```
   /target/
   Cargo.lock
   /releases/
   .omignore
   ```
2. Review all files created
3. Verify no build artifacts

**Final File Structure:**
```
om/
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
├── Cargo.toml
├── Makefile
├── README.md
├── PROMPT.md
├── LICENSE
├── .gitignore
├── .omignore.example
├── SPEC.md (existing)
└── IMPLEMENTATION_PLAN.md (this file)
```

**Verification:**
- All files in correct locations
- No temporary files
- Ready for commit

---

## Execution Order Summary

```
Phase 1: Foundation
  └─ Step 1: Cargo.toml + .gitignore

Phase 2: Core (bottom-up, no dependencies)
  ├─ Step 2: git.rs (git command wrappers)
  ├─ Step 3: session.rs (hash tracking)
  ├─ Step 4: ignore.rs (pattern matching)
  └─ Step 5: scorer.rs (scoring algorithm + unit tests)

Phase 3: Commands
  ├─ Step 6: tree.rs
  ├─ Step 7: cat.rs (with REQUIRED session)
  └─ Step 8: init.rs

Phase 4: CLI
  ├─ Step 9: cli.rs (arg definitions)
  ├─ Step 10: session_cmd.rs (session management)
  ├─ Step 11: lib.rs (public API)
  └─ Step 12: main.rs (entry point)

Phase 5: Tests
  └─ Step 13: integration.rs (E2E + session tests)

Phase 6: Docs
  ├─ Step 14: README.md (with session workflow)
  ├─ Step 15: PROMPT.md (with session protocol)
  ├─ Step 16: LICENSE
  └─ Step 17: Makefile

Phase 7: Extras
  └─ Step 18: .omignore.example

Phase 8: Verification
  ├─ Step 19: E2E manual testing
  └─ Step 20: Git setup
```

---

## Critical Notes

1. **Session Design:**
   - `om cat` works WITHOUT session (normal file output)
   - Session OPTIONAL but recommended (enables deduplication)
   - Check `--session` flag first, then `OM_SESSION` env
   - `om session` is smart:
     - If `OM_SESSION` not set: creates session, outputs export command
     - If `OM_SESSION` already set: confirms session active
   - Idempotent: `eval $(om session)` safe to run multiple times

2. **Error Handling:**
   - All errors bubble up as io::Error
   - User-friendly messages
   - Exit code 1 on error

3. **Testing:**
   - Unit tests in scorer.rs
   - Integration tests use real git repos
   - Manual E2E for UX verification

4. **Simplifications (vs original plan):**
   - ❌ No `om upgrade` command
   - ❌ No shell completions
   - ❌ No JSON output
   - ✅ Sessions optional (but recommended)
   - ✅ `eval $(om session)` smart command - idempotent and automatic
   - ✅ Simpler, cleaner design

---

## Monk's Final Verification

Before execution:
- [x] Complete understanding of session requirement
- [x] Know exact error messages
- [x] Dependencies are compatible
- [x] Test strategy is sufficient
- [x] Steps are independent and ordered
- [x] No circular dependencies
- [x] Error cases handled
- [x] Simplified vs bloated

---

## Execution Confidence: 100%

This plan is:
1. **Simplified** - Removed unnecessary features
2. **Complete** - All 20 steps cover MVP
3. **Ordered** - Bottom-up, no forward dependencies
4. **Testable** - Each step verifiable
5. **Detailed** - Implementation specifics included
6. **Unambiguous** - No guesswork required
7. **Focused** - Core LLM use case only

**Key Difference:** Sessions are OPTIONAL but recommended. `eval $(om session)` is a smart, idempotent command that automatically creates or confirms sessions. LLM can use `om cat` immediately without setup, but gets smart deduplication when using sessions.

Ready to execute in a single shot.
