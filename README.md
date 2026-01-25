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
[![Downloads](https://img.shields.io/crates/d/om-context.svg)](https://crates.io/crates/om-context)

Feed optimal context to LLMs. Scores files by importance (1-10). Tracks content hashes to skip unchanged files.

## Install

```bash
cargo install --path .
```

## Usage

```bash
# Start session (enables deduplication)
eval $(om session)

# View structure
om tree                          # tree view with scores
om tree --flat                   # flat list, sorted by score
om tree --min-score 8            # filter threshold

# Read files
om cat -l 9                      # entry points, README, config
om cat -l 7                      # + core source
om cat -l 5                      # + tests
om cat file.rs                   # specific files

# Cleanup
om session clear $OM_SESSION
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
