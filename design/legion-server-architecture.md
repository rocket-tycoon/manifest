# Legion Server Architecture

## Overview

Legion Server is a Rust daemon that provides:
1. **HTTP API** - For VSCode extension, CLI, and future web UI
2. **MCP Server** - For AI agents to access task context and report progress

## Project Structure

```
legion-server/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, CLI parsing
│   ├── lib.rs               # Library root
│   ├── config.rs            # Configuration loading
│   ├── db/
│   │   ├── mod.rs
│   │   ├── schema.rs        # SQL schema & migrations
│   │   ├── feature.rs       # Feature CRUD
│   │   ├── criterion.rs     # Acceptance criteria CRUD
│   │   ├── session.rs       # Session CRUD
│   │   └── task.rs          # Task CRUD
│   ├── api/
│   │   ├── mod.rs
│   │   ├── routes.rs        # HTTP route definitions
│   │   ├── handlers/
│   │   │   ├── features.rs
│   │   │   ├── sessions.rs
│   │   │   └── tasks.rs
│   │   └── error.rs         # API error types
│   ├── mcp/
│   │   ├── mod.rs
│   │   ├── server.rs        # MCP server setup
│   │   ├── tools.rs         # Tool definitions
│   │   └── handlers.rs      # Tool handlers
│   └── models/
│       ├── mod.rs
│       ├── feature.rs
│       ├── criterion.rs
│       ├── session.rs
│       └── task.rs
├── migrations/
│   └── 001_initial.sql
└── tests/
    ├── api_tests.rs
    └── mcp_tests.rs
```

## Dependencies (Cargo.toml)

```toml
[package]
name = "legion-server"
version = "0.1.0"
edition = "2024"

[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# HTTP server
axum = "0.8"
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace"] }

# MCP
rmcp = "0.1"  # Rust MCP SDK

# Database
rusqlite = { version = "0.35", features = ["bundled"] }
r2d2 = "0.8"
r2d2_sqlite = "0.27"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# CLI
clap = { version = "4", features = ["derive"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Utils
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2"
anyhow = "1"
directories = "6"  # XDG paths
```

---

## Data Models

### Feature

Features are mutable—they represent the current state of a system capability, not a versioned document.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,  // Hierarchical tree structure
    pub title: String,
    pub state: FeatureState,
    pub story: Option<String>,     // User story / description
    pub details: Option<String>,   // Technical notes (markdown)
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureState {
    Proposed,
    Specified,
    Implemented,
    Deprecated,
}
```

### Criterion (Task Acceptance Criteria)

Criteria belong to tasks within a session. They are ephemeral—deleted when the session completes (but their descriptions are preserved in FeatureHistory).

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Criterion {
    pub id: Uuid,
    pub task_id: Uuid,
    pub description: String,
    pub status: CriterionStatus,
    pub verification: VerificationType,
    pub test_file: Option<String>,
    pub blocked_reason: Option<String>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CriterionStatus {
    Pending,
    Complete,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationType {
    Manual,
    Test,
}
```

### Session

Sessions are ephemeral work containers. When completed, tasks are squashed into a FeatureHistory entry and deleted.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub feature_id: Uuid,
    pub goal: String,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Completed,
    Failed,
}
```

### FeatureHistory

FeatureHistory is an append-only log of implementation work—like `git log` for a feature. It records what was done during each session, not versions of the feature content.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureHistory {
    pub id: Uuid,
    pub feature_id: Uuid,
    pub session_id: Option<Uuid>,
    pub summary: String,                    // What was accomplished
    pub criteria_completed: Vec<String>,    // Which acceptance criteria were completed
    pub files_changed: Vec<String>,         // Files touched during the session
    pub git_commits: Vec<String>,           // Related commit SHAs (future)
    pub author: String,                     // Who/what created this entry
    pub created_at: DateTime<Utc>,
}
```

### Task

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub session_id: Uuid,
    pub title: String,
    pub scope: String,
    pub status: TaskStatus,
    pub agent_type: AgentType,
    pub worktree_path: Option<String>,
    pub branch: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    Claude,
    Gemini,
    Codex,
}
```

---

## HTTP API

Base URL: `http://localhost:3000/api/v1`

### Features

| Method | Path | Description |
|--------|------|-------------|
| GET | `/features` | List all features |
| GET | `/features/:id` | Get feature by ID |
| POST | `/features` | Create feature |
| PUT | `/features/:id` | Update feature |
| DELETE | `/features/:id` | Delete feature |
| GET | `/features/:id/criteria` | List criteria for feature |
| POST | `/features/:id/criteria` | Add criterion |

### Sessions

| Method | Path | Description |
|--------|------|-------------|
| GET | `/sessions` | List all sessions |
| GET | `/sessions/:id` | Get session by ID |
| POST | `/sessions` | Create session for feature |
| GET | `/sessions/:id/status` | Get session status with task progress |
| POST | `/sessions/:id/complete` | Complete session |

### Tasks

| Method | Path | Description |
|--------|------|-------------|
| GET | `/tasks/:id` | Get task by ID |
| POST | `/sessions/:id/tasks` | Create task in session |
| PUT | `/tasks/:id` | Update task status |

### Criteria

| Method | Path | Description |
|--------|------|-------------|
| PUT | `/criteria/:id` | Update criterion status |

### Health

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check |

---

## HTTP API Details

### Create Session

```http
POST /api/v1/sessions
Content-Type: application/json

{
  "feature_id": "uuid",
  "goal": "Initial implementation",
  "tasks": [
    {
      "title": "Core authentication flow",
      "scope": "Login form, validation, API integration",
      "agent_type": "claude"
    },
    {
      "title": "Session management",
      "scope": "JWT handling, refresh tokens",
      "agent_type": "claude"
    }
  ]
}
```

Response:
```json
{
  "session": {
    "id": "uuid",
    "feature_id": "uuid",
    "goal": "Initial implementation",
    "status": "active",
    "created_at": "2025-01-09T12:00:00Z"
  },
  "tasks": [
    {
      "id": "task-uuid-1",
      "title": "Core authentication flow",
      "status": "pending"
    },
    {
      "id": "task-uuid-2",
      "title": "Session management",
      "status": "pending"
    }
  ]
}
```

### Get Session Status

```http
GET /api/v1/sessions/:id/status
```

Response:
```json
{
  "session": {
    "id": "uuid",
    "goal": "Initial implementation",
    "status": "active"
  },
  "feature": {
    "id": "uuid",
    "title": "User Authentication"
  },
  "tasks": [
    {
      "id": "task-uuid-1",
      "title": "Core authentication flow",
      "status": "running",
      "agent_type": "claude"
    },
    {
      "id": "task-uuid-2",
      "title": "Session management",
      "status": "completed",
      "agent_type": "claude"
    }
  ],
  "criteria_progress": {
    "total": 4,
    "complete": 2,
    "blocked": 0,
    "pending": 2
  }
}
```

---

## MCP Server

The MCP server runs on a Unix socket or TCP port, providing tools for AI agents.

### Connection

Agents connect via:
- **Unix socket**: `~/.legion/mcp.sock` (preferred for local)
- **TCP**: `localhost:3001` (fallback)

### Tools

#### 1. get_task_context

Get full context for assigned task.

```json
{
  "name": "get_task_context",
  "description": "Get the full context for your assigned task including feature details and acceptance criteria",
  "inputSchema": {
    "type": "object",
    "properties": {
      "task_id": {
        "type": "string",
        "description": "The task ID you were assigned"
      }
    },
    "required": ["task_id"]
  }
}
```

Response:
```json
{
  "task": {
    "id": "uuid",
    "title": "Core authentication flow",
    "scope": "Login form, validation, API integration",
    "status": "running"
  },
  "feature": {
    "title": "User Authentication",
    "description": "Users can log in with email and password",
    "technical_notes": "Use bcrypt for password hashing, JWT for sessions",
    "key_files": ["src/auth/login.ts", "src/auth/session.ts"]
  },
  "criteria": [
    {
      "id": "crit-1",
      "text": "User can enter email and password",
      "status": "pending",
      "verification": "manual"
    },
    {
      "id": "crit-2",
      "text": "Validation errors shown inline",
      "status": "pending",
      "verification": "test",
      "test_file": "tests/login.spec.ts"
    }
  ]
}
```

#### 2. mark_criterion_complete

Mark an acceptance criterion as complete.

```json
{
  "name": "mark_criterion_complete",
  "description": "Mark an acceptance criterion as complete",
  "inputSchema": {
    "type": "object",
    "properties": {
      "task_id": { "type": "string" },
      "criterion_id": { "type": "string" },
      "notes": {
        "type": "string",
        "description": "Optional implementation notes"
      }
    },
    "required": ["task_id", "criterion_id"]
  }
}
```

#### 3. report_blocker

Report that a criterion is blocked.

```json
{
  "name": "report_blocker",
  "description": "Report that a criterion is blocked and cannot be completed",
  "inputSchema": {
    "type": "object",
    "properties": {
      "task_id": { "type": "string" },
      "criterion_id": { "type": "string" },
      "reason": { "type": "string" }
    },
    "required": ["task_id", "criterion_id", "reason"]
  }
}
```

#### 4. add_implementation_note

Add a note about implementation progress.

```json
{
  "name": "add_implementation_note",
  "description": "Add a note about what you implemented or discovered",
  "inputSchema": {
    "type": "object",
    "properties": {
      "task_id": { "type": "string" },
      "content": { "type": "string" },
      "files_changed": {
        "type": "array",
        "items": { "type": "string" }
      }
    },
    "required": ["task_id", "content"]
  }
}
```

#### 5. complete_task

Signal that task is complete.

```json
{
  "name": "complete_task",
  "description": "Signal that your task is complete",
  "inputSchema": {
    "type": "object",
    "properties": {
      "task_id": { "type": "string" },
      "summary": { "type": "string" }
    },
    "required": ["task_id", "summary"]
  }
}
```

---

## CLI Interface

```bash
# Start server (foreground)
legion serve

# Start server (daemon mode)
legion serve --daemon

# Check status
legion status

# Stop daemon
legion stop

# Feature management
legion features list
legion features show <id>
legion features create --title "User Auth" --module "Authentication"

# Session management
legion sessions list
legion sessions start <feature-id> --goal "Initial implementation"
legion sessions status <session-id>
```

---

## Data Storage

### Location

- **macOS**: `~/Library/Application Support/legion/`
- **Linux**: `~/.local/share/legion/`
- **Windows**: `%APPDATA%\legion\`

### Files

```
~/.local/share/legion/
├── legion.db          # SQLite database
├── legion.db-wal      # WAL file
├── config.toml        # User configuration
└── mcp.sock           # Unix socket for MCP
```

### Configuration

```toml
# ~/.config/legion/config.toml

[server]
http_port = 3000
mcp_port = 3001

[database]
path = "~/.local/share/legion/legion.db"

[logging]
level = "info"
```

---

## VSCode Extension Changes

The VSCode extension becomes a thin HTTP client:

```typescript
// src/api/client.ts
export class LegionClient {
  constructor(private baseUrl = 'http://localhost:3000/api/v1') {}

  async getFeatures(): Promise<Feature[]> {
    const res = await fetch(`${this.baseUrl}/features`);
    return res.json();
  }

  async createSession(featureId: string, goal: string, tasks: TaskInput[]): Promise<SessionResponse> {
    const res = await fetch(`${this.baseUrl}/sessions`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ feature_id: featureId, goal, tasks })
    });
    return res.json();
  }

  async getSessionStatus(sessionId: string): Promise<SessionStatus> {
    const res = await fetch(`${this.baseUrl}/sessions/${sessionId}/status`);
    return res.json();
  }
}
```

The extension no longer needs:
- `better-sqlite3` (no native modules!)
- Database code
- MCP server code

It only needs:
- HTTP client
- Tree view UI
- Webview panels
- Terminal spawning (still needs VSCode API)

---

## Agent Integration

When spawning an agent terminal, the extension passes:

```bash
# Environment variables
LEGION_TASK_ID=<task-uuid>
LEGION_MCP_URL=http://localhost:3001

# Or for Claude Code specifically:
claude --mcp-server http://localhost:3001 "Work on task <task-id>"
```

The agent then:
1. Calls `get_task_context` with task ID
2. Works on the implementation
3. Reports progress via `mark_criterion_complete`, `add_implementation_note`
4. Signals completion via `complete_task`

---

## Next Steps

1. **Create Rust project** - `cargo new legion-server`
2. **Implement database layer** - Schema, migrations, CRUD operations
3. **Implement HTTP API** - Axum routes and handlers
4. **Implement MCP server** - Tool definitions and handlers
5. **Update VSCode extension** - Replace SQLite with HTTP client
6. **Create Homebrew formula** - For distribution
