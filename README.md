```
          69696969                         69696969
       6969    696969                   696969    6969
     969    69  6969696               6969  6969     696
    969        696969696             696969696969     696
   969        69696969696           6969696969696      696
   696      9696969696969           969696969696       969
    696     696969696969             969696969        969
     696     696  96969      _=_      9696969  69    696
       9696    969696      q(-_-)p      696969    6969
          96969696         '_) (_`         69696969
             96            /__/  \            69
             69          _(<_   / )_          96
            6969        (__\_\_|_/__)        9696
```

# om

[![Crates.io](https://img.shields.io/crates/v/om-context.svg)](https://crates.io/crates/om-context)
[![CI](https://github.com/oeo/om/workflows/CI/badge.svg)](https://github.com/oeo/om/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Feed optimal context to LLMs. Scores files by importance (1-10), tracks content hashes for deduplication, and provides structured formats (XML/JSON) for agent consumption.

## v0.2.0 Features

- **Token Counting**: Precise token counting using `tiktoken-rs` (GPT-4o/GPT-3.5/4 support).
- **Structured Output**: XML and JSON formats designed for LLM agents.
- **Git Awareness**: Filter by `--dirty`, `--staged`, or `--unstaged` status.
- **Configurable**: Global (`~/.om/config.toml`) and project (`.om.toml`) configuration support.
- **High Performance**: Parallel processing with Rayon (scans 10k+ files in <1s).
- **Smart Binary Detection**: MIME-based detection to skip non-text files.

## Install

```bash
cargo install --path .
```

## Usage

```bash
# Start session (enables deduplication)
eval $(om session)

# View structure
om tree                          # tree view with scores (current directory)
om tree --flat                   # flat list, sorted by score
om tree --tokens                 # show token counts per file
om tree --dirty                  # show only modified/untracked files
om tree --format json            # output valid JSON
om tree --jobs 4                 # parallel scanning

# Read files
om cat -l 9                      # entry points, README, config
om cat -l 7                      # + core source
om cat --tokens                  # include token counts in output
om cat --format xml              # output XML (optimal for Claude)
om cat --staged                  # read only staged files

# Cleanup
om session clear $OM_SESSION
```

### Output Formats

`om` supports multiple formats via the `--format` flag:

- `text` (default): Human-readable ASCII tree or flat list.
- `json`: Machine-readable JSON including all metadata.
- `xml`: LLM-optimized XML with CDATA sections (prevents instruction/code mixing).

```bash
om tree --format xml
om cat src/main.rs --format json
```

### Git Integration

Filter your context to only include relevant changes:

```bash
om tree --dirty      # modified, added, or untracked files
om cat --staged      # only what you're about to commit
om cat --unstaged    # local changes not yet staged
```

### Token Counting

Uses `tiktoken-rs` for precise token estimation:

```bash
om tree --tokens
om cat README.md --tokens
```

### Configuration

`om` looks for configuration in:
1. `.om.toml` in the repository root.
2. `~/.om/config.toml` for global defaults.

Example `.om.toml`:
```toml
min_score = 7
depth = 3
no_color = false
format = "text"
```

### Path Filtering

By default, `om tree` respects your current working directory:

```bash
cd src/              # navigate to subdirectory
om tree              # shows only files under src/
om tree --git-root   # override: show entire repository
```

You can also filter by path explicitly:

```bash
om tree src          # show only src/ files
om tree tests        # show only tests/ files
```

Sessions store at `~/.om/sessions/*.json`. List with `ls ~/.om/sessions/`.

## Agent Integration

Add this to your agent's system instructions (e.g., `~/.claude/CLAUDE.md`):

````markdown
# Commands: Project Context with om

When I say **om**, what I mean is:

```command
Use the `om` tool to understand codebase structure and ingest files.

Start a session:
eval $(om session)

Protocol:
1. om tree --flat              # understand structure
2. om cat -l 9                 # entry points, README, config
3. om cat -l 7                 # core source files
4. om cat -l 5                 # tests and supporting code

On subsequent calls:
om cat -l 7                    # only changed files returned

The tool tracks file hashes. Unchanged files are automatically skipped.

Cleanup:
om session clear $OM_SESSION
```

When I say **om to level N**, run `om cat -l N` and summarize what you learned.
````

### Example session

```
❯ om tree --flat --tokens

  SCORE  TOKENS  PATH
  10     150     README.md
  10     850     src/main.rs
  10     420     src/cli.rs
  9      120     Cargo.toml
  7      640     src/tree.rs
  7      580     src/cat.rs

❯ om cat src/main.rs --format xml

<codebase>
  <project>om</project>
  <files>
    <file path="src/main.rs" score="10" lines="72" tokens="850">
      <content><![CDATA[
mod cat;
mod cli;
...
fn main() {
    let cli = Cli::parse();
    ...
}
      ]]></content>
    </file>
  </files>
</codebase>
```

## Scoring

| Score | Files |
|-------|-------|
| 10 | Entry points (main.rs, index.js), README |
| 9 | Config (config.*, settings.*) |
| 8 | Project files (Cargo.toml, package.json, Dockerfile) |
| 7 | Core source |
| 5 | Tests |
| 2 | Generated (*.lock, *.min.js) |

**Modifiers:** Important dirs (+2), domain dirs (+1), test dirs (-2), vendor/dist (-3), root level (+1), deep nesting (-2).

## .omignore

Copy `.omignore.example` to `.omignore` or `~/.omignore`. Supports glob patterns like `.gitignore`.

## License

MIT
