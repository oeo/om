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

### Example session (why to use this)

```bash
❯ meditate 8

⏺ I'll begin the level 8 meditation. Let me first ensure the OM session is established and gather the project context.

⏺ Bash(echo $OM_SESSION)
  ⎿  (No content)

⏺ Bash(eval $(om session))
  ⎿  Session created: sess-1769308334

⏺ Bash(om tree --flat)
  ⎿  10 README.md
     10 src/api/server.ts
     10 src/index.ts
     … +134 lines (ctrl+o to expand)

⏺ Bash(om cat -l 8 > /tmp/om-level8.txt)
  ⎿  (No content)

⏺ Bash(wc -l /tmp/om-level8.txt)
  ⎿     15005 /tmp/om-level8.txt

⏺ Bash(wc -c /tmp/om-level8.txt)
  ⎿    432876 /tmp/om-level8.txt

⏺ Level 8 meditation analysis complete.

  Metrics:
  - Lines: 15,005
  - Size: 432,876 bytes (~423 KB)
  - Estimated tokens: ~108,219 tokens
  - Required reads: 8 chunks of 2000 lines each
  - Context consumption: ~10.8% of 1M context window

  Scope: Level 8 includes configuration files, chain definitions, Docker compose files, and project documentation.

  Proceed with full ingestion of all 15,005 lines?

❯ yes

⏺ Read(/tmp/om-level8.txt)
  ⎿  Read 2000 lines

⏺ Read(/tmp/om-level8.txt)
  ⎿  Read 2000 lines

⏺ Read(/tmp/om-level8.txt)
  ⎿  Read 2000 lines

⏺ Read(/tmp/om-level8.txt)
  ⎿  Read 2000 lines

⏺ Read(/tmp/om-level8.txt)
  ⎿  Read 2000 lines

⏺ I need to continue reading the remaining chunks to complete the meditation. Let me continue with the next sections.

⏺ Read(/tmp/om-level8.txt)
  ⎿  Read 2000 lines

⏺ Read(/tmp/om-level8.txt)
  ⎿  Read 2000 lines

⏺ Read(/tmp/om-level8.txt)
  ⎿  Read 1007 lines

⏺ Level 8 meditation complete. I have ingested all 15,005 lines across 57 files.

  Understanding Achieved

  bolt is a proof-of-work blockchain with an account-based model built on Bun and TypeScript. The codebase is well-organized following domain-driven design.

  Architecture Overview

  Core Components:
  - Blockchain: Account model with nonce tracking, SHA-256 PoW, Bitcoin-style difficulty adjustment
  - Storage: LMDB primary backend with memory adapter for testing
  - Networking: TCP binary protocol for data exchange, IPFS used exclusively for peer discovery
  - Mining: GetBlockTemplate (GBT) protocol for pool compatibility
  - Wallet: HD wallet derivation (BIP32/BIP39/BIP44) with path m/44'/1057'/account'/change/index

  Currency:
  - Symbol: BOLT
  - Base unit: watt (W)
  - Precision: 1 BOLT = 100,000,000 watts
  - Max supply: 21M BOLT (2.1 quadrillion watts)
  - Initial reward: 50 BOLT, halving every 210,000 blocks

  Network Architecture:
  TCP (port 8333) - Binary protocol for all blockchain data
  IPFS (port 5001) - Peer discovery only via pubsub
  API (port 7333) - REST endpoints
  Metrics (port 7336) - Prometheus endpoint

  Current Status:
  - 368 passing tests
  - Headers-first blockchain synchronization
  - Advanced chain reorganization with pre-validation
  - Median time validation and timestamp ordering
  - Comprehensive metrics with Prometheus
  - Docker-based multi-node deployment
  - Recent simplification: removed Redis dependency, eliminated unused worker pool

  Key Observations

  Strengths:
  1. Clean separation of concerns across domain folders
  2. Consistent use of Bun-native APIs (CryptoHasher, sleep, etc.)
  3. Proper event-driven architecture
  4. Comprehensive metrics instrumentation
  5. Multiple storage adapters with clean interface

  Areas Requiring Attention (from level 8 context):
  1. Missing addToMempool implementation in LMDB adapter (line 2656-2658 shows it delegates to mempoolStore, but interface mismatch with base adapter)
  2. Incomplete blockAdded event emitter name inconsistency (fires block:added in some places, blockAdded in others)
  3. Several .disabled and .skip test files indicating incomplete test coverage
  4. State reversion not implemented in LMDB store (line 13783)
  5. GetBlockTemplateService references methods that don't exist on StorageAdapter

  I have complete monk-like clarity on the level 8 context. What would you like to address?

✻ Churned for 1m 19s
```

### Example agent prompt (adjust as needed)

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
