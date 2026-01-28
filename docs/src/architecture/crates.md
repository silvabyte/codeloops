# Crate Structure

Codeloops is organized as a Rust workspace with multiple crates. This document details each crate's purpose and key types.

## Overview

| Crate | Type | Purpose |
|-------|------|---------|
| `codeloops` | Binary | CLI, API server, session viewer |
| `codeloops-core` | Library | Loop orchestration |
| `codeloops-agent` | Library | Agent abstraction |
| `codeloops-critic` | Library | Critic evaluation |
| `codeloops-git` | Library | Git diff capture |
| `codeloops-logging` | Library | Logging and session writing |
| `codeloops-sessions` | Library | Session reading and parsing |

## Dependency Graph

```
                    ┌─────────────────┐
                    │   codeloops     │  (binary)
                    │                 │
                    └────────┬────────┘
                             │
           ┌─────────────────┼─────────────────┐
           │                 │                 │
           ▼                 ▼                 ▼
┌──────────────────┐ ┌──────────────┐ ┌─────────────────┐
│ codeloops-core   │ │codeloops-    │ │ codeloops-      │
│                  │ │sessions      │ │ logging         │
└────────┬─────────┘ └──────────────┘ └─────────────────┘
         │
    ┌────┴────┬─────────────┐
    │         │             │
    ▼         ▼             ▼
┌────────┐ ┌────────┐ ┌──────────────┐
│agent   │ │critic  │ │    git       │
└────────┘ └────────┘ └──────────────┘
```

## codeloops (Binary Crate)

**Location**: `crates/codeloops/`

The main CLI binary. Entry point for all user interactions.

### Key Files

| File | Purpose |
|------|---------|
| `main.rs` | CLI entry, argument parsing, command dispatch |
| `config.rs` | Configuration loading (global + project) |
| `sessions.rs` | Session commands (list, show, diff, stats) |
| `init.rs` | Interactive setup |
| `ui.rs` | Web UI launcher |
| `api/mod.rs` | API server router |
| `api/sessions.rs` | Session API endpoints |
| `api/stats.rs` | Statistics API endpoint |
| `api/sse.rs` | Server-Sent Events for live updates |

### Key Types

```rust
// CLI argument structure
struct Cli {
    prompt: Option<String>,
    prompt_file: Option<PathBuf>,
    working_dir: Option<PathBuf>,
    agent: Option<AgentChoice>,
    actor_agent: Option<AgentChoice>,
    critic_agent: Option<AgentChoice>,
    max_iterations: Option<usize>,
    // ...
}

enum Commands {
    Run,
    Sessions(SessionsAction),
    Ui(UiArgs),
    Init,
}

// Configuration types
struct GlobalConfig {
    defaults: DefaultsConfig,
}

struct ProjectConfig {
    agent: Option<String>,
    model: Option<String>,
    actor: Option<RoleConfig>,
    critic: Option<RoleConfig>,
}
```

## codeloops-core (Library Crate)

**Location**: `crates/codeloops-core/`

Core loop orchestration logic.

### Key Files

| File | Purpose |
|------|---------|
| `lib.rs` | Crate root, re-exports |
| `loop_runner.rs` | Main loop execution |
| `context.rs` | Loop context and iteration records |
| `outcome.rs` | Loop outcomes (Success, Failed, etc.) |
| `error.rs` | Error types |

### Key Types

```rust
// Loop orchestrator
pub struct LoopRunner {
    actor: Arc<dyn Agent>,
    critic: Arc<dyn Agent>,
    diff_capture: DiffCapture,
    logger: Logger,
    session_writer: SessionWriter,
    interrupt: Arc<AtomicBool>,
}

// Shared context across iterations
pub struct LoopContext {
    pub prompt: String,
    pub working_dir: PathBuf,
    pub iteration: usize,
    pub history: Vec<IterationRecord>,
    pub max_iterations: Option<usize>,
    pub last_feedback: Option<String>,
}

// Record of one iteration
pub struct IterationRecord {
    pub iteration_number: usize,
    pub actor_output: String,
    pub actor_stderr: String,
    pub actor_exit_code: i32,
    pub actor_duration_secs: f64,
    pub git_diff: String,
    pub git_files_changed: usize,
    pub critic_output: String,
    pub critic_decision: String,
    pub timestamp: DateTime<Utc>,
}

// Terminal states
pub enum LoopOutcome {
    Success { iterations, summary, confidence, history, duration },
    MaxIterationsReached { iterations, history, duration },
    UserInterrupted { iterations, history, duration },
    Failed { iterations, error, history, duration },
}
```

### Main Loop

```rust
impl LoopRunner {
    pub async fn run(&self, context: LoopContext) -> Result<LoopOutcome> {
        // Write session start
        self.session_writer.write_start(&context)?;

        loop {
            // Check interrupt
            if self.interrupt.load(Ordering::SeqCst) {
                return Ok(LoopOutcome::UserInterrupted { ... });
            }

            // Check max iterations
            if let Some(max) = context.max_iterations {
                if context.iteration >= max {
                    return Ok(LoopOutcome::MaxIterationsReached { ... });
                }
            }

            // Execute iteration
            let result = self.run_iteration(&mut context).await?;

            match result {
                IterationResult::Done { summary, confidence } => {
                    return Ok(LoopOutcome::Success { ... });
                }
                IterationResult::Continue { feedback } => {
                    context.last_feedback = Some(feedback);
                    context.iteration += 1;
                }
            }
        }
    }
}
```

## codeloops-agent (Library Crate)

**Location**: `crates/codeloops-agent/`

Agent abstraction layer.

### Key Files

| File | Purpose |
|------|---------|
| `lib.rs` | Crate root, factory function |
| `traits.rs` | Agent trait definition |
| `claude.rs` | Claude Code agent |
| `opencode.rs` | OpenCode agent |
| `cursor.rs` | Cursor agent |
| `spawner.rs` | Process spawning utilities |
| `output.rs` | Output types |

### Key Types

```rust
// Agent interface
#[async_trait]
pub trait Agent: Send + Sync {
    fn name(&self) -> &str;
    fn agent_type(&self) -> AgentType;

    async fn execute(
        &self,
        prompt: &str,
        config: &AgentConfig,
    ) -> Result<AgentOutput, AgentError>;

    async fn is_available(&self) -> bool;
    fn binary_path(&self) -> &Path;
}

// Agent types
pub enum AgentType {
    ClaudeCode,
    OpenCode,
    Cursor,
}

// Agent configuration
pub struct AgentConfig {
    pub working_dir: PathBuf,
    pub timeout: Option<Duration>,
    pub env_vars: HashMap<String, String>,
    pub model: Option<String>,
}

// Agent execution result
pub struct AgentOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration: Duration,
}

// Factory function
pub fn create_agent(agent_type: AgentType) -> Box<dyn Agent> {
    match agent_type {
        AgentType::ClaudeCode => Box::new(ClaudeCodeAgent::new()),
        AgentType::OpenCode => Box::new(OpenCodeAgent::new()),
        AgentType::Cursor => Box::new(CursorAgent::new()),
    }
}
```

## codeloops-critic (Library Crate)

**Location**: `crates/codeloops-critic/`

Critic evaluation and decision parsing.

### Key Files

| File | Purpose |
|------|---------|
| `lib.rs` | Crate root, re-exports |
| `evaluator.rs` | Critic evaluation logic |
| `decision.rs` | Decision types and parsing |
| `prompts.rs` | Prompt templates for critic |

### Key Types

```rust
// Critic evaluator
pub struct CriticEvaluator {
    agent: Arc<dyn Agent>,
}

// Decision types
pub enum CriticDecision {
    Done {
        summary: String,
        confidence: f64,
    },
    Continue {
        feedback: String,
    },
    Error {
        recovery: String,
    },
}

// Evaluation input
pub struct EvaluationInput {
    pub prompt: String,
    pub actor_output: String,
    pub git_diff: String,
    pub iteration: usize,
}
```

## codeloops-git (Library Crate)

**Location**: `crates/codeloops-git/`

Git operations for diff capture.

### Key Files

| File | Purpose |
|------|---------|
| `lib.rs` | Crate root, re-exports |
| `diff.rs` | Diff capture functionality |
| `status.rs` | Git status utilities |

### Key Types

```rust
// Diff capture utility
pub struct DiffCapture {
    working_dir: PathBuf,
    baseline_commit: Option<String>,
}

// Diff result
pub struct DiffSummary {
    pub diff: String,
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}

impl DiffCapture {
    pub fn new(working_dir: PathBuf) -> Self;
    pub fn capture(&self) -> Result<DiffSummary>;
    pub fn set_baseline(&mut self);
}
```

## codeloops-logging (Library Crate)

**Location**: `crates/codeloops-logging/`

Logging and session file writing.

### Key Files

| File | Purpose |
|------|---------|
| `lib.rs` | Crate root, re-exports |
| `session.rs` | Session JSONL writer |
| `events.rs` | Log event types |

### Key Types

```rust
// Session writer
pub struct SessionWriter {
    file: File,
    id: String,
}

// Session line types
#[derive(Serialize)]
#[serde(tag = "type")]
pub enum SessionLine {
    #[serde(rename = "session_start")]
    SessionStart {
        timestamp: DateTime<Utc>,
        prompt: String,
        working_dir: PathBuf,
        actor_agent: String,
        critic_agent: String,
        actor_model: Option<String>,
        critic_model: Option<String>,
        max_iterations: Option<usize>,
    },
    #[serde(rename = "iteration")]
    Iteration {
        iteration_number: usize,
        actor_output: String,
        actor_stderr: String,
        actor_exit_code: i32,
        actor_duration_secs: f64,
        git_diff: String,
        git_files_changed: usize,
        critic_decision: String,
        feedback: Option<String>,
        timestamp: DateTime<Utc>,
    },
    #[serde(rename = "session_end")]
    SessionEnd {
        outcome: String,
        iterations: usize,
        summary: Option<String>,
        confidence: Option<f64>,
        duration_secs: f64,
        timestamp: DateTime<Utc>,
    },
}

impl SessionWriter {
    pub fn new(sessions_dir: &Path, prompt: &str) -> Result<Self>;
    pub fn write_start(&mut self, ...) -> Result<()>;
    pub fn write_iteration(&mut self, ...) -> Result<()>;
    pub fn write_end(&mut self, ...) -> Result<()>;
}
```

## codeloops-sessions (Library Crate)

**Location**: `crates/codeloops-sessions/`

Session reading, parsing, and querying.

### Key Files

| File | Purpose |
|------|---------|
| `lib.rs` | Crate root, re-exports |
| `store.rs` | Session storage access |
| `parser.rs` | JSONL parsing |
| `types.rs` | Session types |
| `watcher.rs` | File system watcher for live updates |

### Key Types

```rust
// Session store
pub struct SessionStore {
    sessions_dir: PathBuf,
}

// Full session
pub struct Session {
    pub id: String,
    pub start: SessionStart,
    pub iterations: Vec<Iteration>,
    pub end: Option<SessionEnd>,
}

// Session summary (for listings)
pub struct SessionSummary {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub prompt_preview: String,
    pub working_dir: PathBuf,
    pub project: String,
    pub outcome: Option<String>,
    pub iterations: usize,
    pub duration_secs: Option<f64>,
    pub confidence: Option<f64>,
    pub actor_agent: String,
    pub critic_agent: String,
}

// Filter criteria
pub struct SessionFilter {
    pub outcome: Option<String>,
    pub after: Option<DateTime<Utc>>,
    pub before: Option<DateTime<Utc>>,
    pub search: Option<String>,
    pub project: Option<String>,
}

// Statistics
pub struct SessionStats {
    pub total_sessions: usize,
    pub success_rate: f64,
    pub avg_iterations: f64,
    pub avg_duration_secs: f64,
    pub sessions_over_time: Vec<DayCount>,
    pub by_project: Vec<ProjectStats>,
}

impl SessionStore {
    pub fn new() -> Result<Self>;
    pub fn list_sessions(&self, filter: &SessionFilter) -> Result<Vec<SessionSummary>>;
    pub fn load_session(&self, id: &str) -> Result<Session>;
    pub fn get_stats(&self) -> Result<SessionStats>;
}
```

## Which Crate for Which Change?

| Change Type | Crate |
|-------------|-------|
| New CLI command | `codeloops` |
| Configuration options | `codeloops` |
| Loop behavior | `codeloops-core` |
| New agent support | `codeloops-agent` |
| Critic evaluation logic | `codeloops-critic` |
| Git operations | `codeloops-git` |
| Log output format | `codeloops-logging` |
| Session parsing | `codeloops-sessions` |
| API endpoints | `codeloops` (api/) |
| Web UI | `ui/` (separate) |
