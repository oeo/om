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
          96969696         '_) (_`         96969696
             96            /__/ \             69
             69          _(<_   / )_          96
            6969        (____|_/____)        6969
```

# om

[![Crates.io](https://img.shields.io/crates/v/om-context.svg)](https://crates.io/crates/om-context)
[![CI](https://github.com/oeo/om/workflows/CI/badge.svg)](https://github.com/oeo/om/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

`om` feeds your codebase to LLMs without wasting context. It scores every file by importance (1-10), outputs only what matters, and tracks what has already been sent so repeated calls skip unchanged files automatically.

**The workflow problem it solves:** pasting entire codebases into LLM conversations is wasteful and often hits context limits. Manually picking files is tedious. `om` does the selection for you — give it a minimum score and it returns the right files in the right order, deduplicated across the session.

## Install

```bash
cargo install --path .
```

## The typical workflow

```bash
# 1. start a session — enables deduplication across calls
eval $(om session)

# 2. survey the repo
om tree --flat

# 3. feed context to the LLM in layers
om cat -l 9          # entry points, README, config (~critical files)
om cat -l 7          # core source
om cat -l 5          # tests and supporting files

# on subsequent calls, only files that changed since last read are returned
om cat -l 7

# 4. clean up
om session clear $OM_SESSION
```

Sessions store content hashes. If a file hasn't changed since the LLM last saw it, `om cat` skips it. This means you can call `om cat -l 7` repeatedly throughout a conversation and only ever send new information.

## Scoring

Files are scored 1-10. Pass `-l N` to set the minimum.

| Score | Files |
|-------|-------|
| 10 | Entry points (`main.rs`, `index.js`), `README` |
| 9 | Config files (`config.*`, `settings.*`) |
| 8 | Project files (`Cargo.toml`, `package.json`, `Dockerfile`) |
| 7 | Core source |
| 5 | Tests |
| 2 | Generated (`*.lock`, `*.min.js`) |

**Directory modifiers:** `src/`, `core/`, `lib/` (+2) · `api/`, `models/` (+1) · `tests/` (-2) · `vendor/`, `dist/` (-3) · root level (+1) · deep nesting (-2)

## Output formats

```bash
om cat -l 7                  # text (default) — human readable
om cat -l 7 --format xml     # XML with CDATA — optimal for Claude
om cat -l 7 --format json    # JSON — for programmatic consumption
```

XML wraps file contents in `<![CDATA[...]]>` sections, which prevents code from being interpreted as XML or conflicting with prompt structure.

## File filtering

`om cat` only outputs files safe for LLM consumption. It silently drops:

- **Binary files** — detected by MIME type and null-byte probe. Extensionless files (`Makefile`, `LICENSE`) are probed by content, not assumed binary.
- **Invalid UTF-8** — excluded cleanly. No corrupted `\u{FFFD}` characters reach the LLM.
- **Empty files** — zero-byte files produce no output block.

Skipped files are reported by path so the LLM knows what was excluded:

```
# Skipped: 2 binary
#   - assets/logo.png
#   - build/app.wasm
# Skipped: 1 unreadable (read error or invalid UTF-8)
#   - data/legacy.bin
```

JSON and XML output include `skipped_binary_paths` and `skipped_unreadable_paths` arrays.

## Git integration

```bash
om tree --dirty        # only modified/untracked files
om cat --staged        # only staged files
om cat --unstaged      # only unstaged files
```

## Path filtering

Commands default to the current working directory. Use `--git-root` to scan the whole repo.

```bash
cd src/ && om cat -l 7          # only files under src/
om cat -l 7 --git-root          # entire repository
om tree tests                   # explicit path filter
```

## Configuration

`om` loads config from `~/.om/config.toml` (global) then `.om.toml` at the repo root. Repo config wins.

```toml
min_score = 7
depth     = 3
no_color  = false
format    = "text"
no_cache  = false
```

## Agent integration

Add to your agent's system instructions (`~/.claude/CLAUDE.md`):

````markdown
When I say **meditate** or **meditate N**:

```command
Use `om` to ingest the codebase.

1. om tree --flat              # understand structure
2. om cat -l 9                 # entry points, README, config
3. om cat -l 7                 # core source
4. om cat -l 5                 # tests and supporting code

On subsequent calls, om cat only returns files that changed.
```
````

## .omignore

Copy `.omignore.example` to `.omignore` or `~/.omignore`. Supports glob patterns like `.gitignore`.

## License

MIT
