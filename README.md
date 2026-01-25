# om - LLM Context Tool

`om` is a CLI tool that scores project files by importance (1-10) and feeds optimal context to LLMs with smart session-based deduplication.

## What is om?

When working with LLMs on codebases, you want to provide relevant context without overwhelming the model. `om` solves this by:

1. **Scoring files** based on importance (entry points = 10, tests = 5, generated files = 2)
2. **Smart filtering** to show only what matters
3. **Session tracking** to avoid re-reading unchanged files
4. **Tree visualization** to understand project structure at a glance

## Install

```bash
cargo install --path .
```

Or build from source:

```bash
cargo build --release
sudo cp target/release/om /usr/local/bin/
```

## Usage

### Session Management

Sessions track which files have been read to avoid duplication. The smart session command is idempotent:

```bash
eval $(om session)
```

This creates a new session if `OM_SESSION` is not set, or confirms the active session if already set.

```bash
om session list
om session show sess-1234567890
om session clear sess-1234567890
```

### Tree View

Show project structure with importance scores:

```bash
om tree
om tree --flat
om tree --min-score 8
om tree --depth 3
```

### Cat Files

Output file contents, optionally using sessions for deduplication:

```bash
om cat -l 9
om cat -l 7 --session sess-1234567890
om cat src/main.rs src/lib.rs
```

The `-l` (level) flag filters files by minimum score:
- Level 9-10: Entry points, README, config
- Level 7-8: Core source files, project files
- Level 5-6: Supporting code, tests
- Level 1-4: Low priority, generated files

### Without Sessions

All cat commands work without sessions:

```bash
om cat -l 7
```

### With Sessions (Recommended)

```bash
eval $(om session)
om cat -l 9
om cat -l 7
```

Files are tracked by content hash. On subsequent runs, unchanged files are skipped.

### Initialize .omignore

Create a default `.omignore` file:

```bash
om init
om init --global
om init --force
```

## Scoring System

| Score | Files | Examples |
|-------|-------|----------|
| 10 | Entry points, README | main.rs, index.js, README.md |
| 9 | Config files | config.toml, settings.json |
| 8 | Project files | Cargo.toml, package.json, Dockerfile |
| 7 | Core source (base) | src/handler.rs, lib/utils.py |
| 5 | Tests | test_main.rs, foo.test.ts |
| 3 | Init files | __init__.py |
| 2 | Generated files | *.lock, *.min.js, *.d.ts |

Scores are modified by:
- Directory importance (src, core: +2 / vendor, dist: -3)
- Depth (root: +1 / deeply nested: -2)
- File type (schema files: +1 / docs: -1)

## LLM Integration

For LLM system prompts and meditation protocols, see [PROMPT.md](PROMPT.md).

## Requirements

- Git (uses `git ls-files` to discover files)
- Rust 1.70+ (for building)

## Configuration

`.omignore` files use glob patterns:

```gitignore
*.lock
node_modules/
**/dist/**
```

Priority: `~/.omignore` (global) < `.omignore` (local)

## License

MIT
