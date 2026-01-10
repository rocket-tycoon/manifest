# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

RocketManifest (formerly "Legion") is an MCP server for living feature documentation. It was extracted from the RocketCrew VSCode extension to provide a standalone backend for technical product management.

### Core Philosophy: Features as Living Documentation

Unlike JIRA/Linear which track **work items** that accumulate as closed history, RocketManifest tracks **features** that describe the current state of the system:

| Traditional Tools | RocketManifest |
|-------------------|----------------|
| Issue (work item) | Feature (system capability) |
| Open → Closed → Forgotten | Proposed → Implemented → **Living** |
| Changelog of what happened | Description of what IS |

Features are not work items to be closed. They are living descriptions that evolve with the codebase.

### MCP Server Purpose

AI agents access features through deterministic MCP tools (not grep):
- `get_task_context` - Get assigned task with feature context and criteria
- `mark_criterion_complete` - Check off requirements
- `report_blocker` - Report blocked criteria
- `add_implementation_note` - Document implementation details
- `complete_task` - Signal task completion

Agents are scoped to their assigned task and report progress back.

## Build & Test

```bash
cargo build                    # Debug build
cargo build --release          # Release build
cargo test                     # Run all tests
cargo test db_spec             # Run db specs only
cargo run                      # Start server on port 3000
cargo run -- serve -p 8080     # Start on custom port
```

### BDD Testing with Speculate

Tests use [speculate2](https://crates.io/crates/speculate2) for BDD-style specs:

```rust
speculate! {
    describe "features" {
        before {
            let db = Database::open_memory().expect("...");
            db.migrate().expect("...");
        }

        it "creates a feature" {
            // ...
        }
    }
}
```

Test files: `tests/db_spec.rs`, `tests/api_spec.rs` (39 tests total)

## Architecture

**Stack**: Rust 2024 + Axum 0.8 + SQLite (rusqlite with WAL mode) + Tokio

```
src/
├── main.rs         # CLI (clap) with serve/status/stop subcommands
├── api/
│   ├── mod.rs      # Router setup, all routes under /api/v1
│   └── handlers/   # Request handlers (extract State<Database>)
├── db/
│   ├── mod.rs      # Database wrapper with CRUD operations
│   └── schema.rs   # SQLite schema (embedded, auto-migrated)
├── models/         # Domain types with serde + enums with as_str/from_str
└── mcp/            # MCP server (stub - not yet implemented)
```

### Data Model

Features form a **hierarchical tree** (like a file browser):

```
Authentication/                 <- feature node with context
├── Login/                      <- feature node with context
│   ├── Email + Password        <- leaf (can have sessions)
│   └── OAuth/                  <- feature node
│       ├── Google              <- leaf
│       └── GitHub              <- leaf
└── Session Management          <- leaf
```

**Permanent entities:**
- **Feature**: Self-referential tree via `parent_id`. Any node can have content and criteria. Only **leaf nodes** can have sessions.
- **AcceptanceCriterion**: Belongs to Feature, tracks completion status and verification type (manual/test)
- **FeatureHistory**: Append-only log of implementation sessions (like `git log` for a feature). Records what was done during each session, which criteria were completed, and links to git commits. This is NOT feature versioning—the feature content itself is mutable. History answers "what work was done on this feature and when?"

**Ephemeral entities (exist only during active work):**
- **Session**: One active session per feature at a time. When completed, tasks are squashed into a `feature_history` entry and deleted.
- **Task**: Work unit within Session, assigned to an agent (claude/gemini/codex). Deleted when session completes.

```
Session lifecycle:
1. Create session on leaf feature
2. Create tasks, agents work on criteria
3. Session completes:
   └─> Summary of work + completed criteria → feature_history entry
   └─> Link to git commit(s) recorded
   └─> Task records deleted
   └─> Session marked completed
```

Key methods: `get_root_features()`, `get_children(id)`, `is_leaf(id)`

### API Routes

All routes prefixed with `/api/v1`:
- Features: CRUD at `/features`, `/features/{id}`
  - `/features/roots` - GET root features (no parent)
  - `/features/{id}/children` - GET direct children
  - `/features/{id}/criteria` - GET/POST criteria
- Sessions: POST `/sessions`, GET `/sessions/{id}`, `/sessions/{id}/status`
  - Only allowed on leaf features (returns 500 if feature has children)
- Tasks: GET/PUT `/tasks/{id}`
- Criteria: PUT `/criteria/{id}`

### Database

- Location: `~/.local/share/legion/legion.db` (via `directories` crate)
- Schema auto-migrates on startup via `db.migrate()`
- All IDs stored as TEXT (UUIDs), dates as RFC3339 strings, content as JSON

### Code Patterns

- Enums use manual `as_str()`/`from_str()` for DB serialization (not derive macros)
- `Result<Option<T>>` pattern for get operations (None = not found, Err = DB error)
- Dynamic SQL building for partial updates (UpdateTaskInput, UpdateCriterionInput)
- Database wrapped in `Arc<Mutex<Connection>>` for thread-safe sharing

## Related Projects

- **RocketCrew** (`../RocketCrew`) - VSCode extension that consumes this server's HTTP API
- `../RocketCrew/design/vscode-extension-mvp.md` - Full vision for the TPM workflow
- `design/legion-server-architecture.md` - Server architecture spec (local)
