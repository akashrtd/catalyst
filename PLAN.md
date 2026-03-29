# Catalyst v0.2.0 — AI-Native Architecture Plan

## What Changed

v0.1.1 proved the stack works: Rust workspace, ratatui TUI, Anthropic streaming, 4 tools, 94 tests. But it's a chat wrapper with file operations. Every CLI agent has that. This plan makes Catalyst an **AI-native agent** — where the LLM is the runtime, context is a managed resource, and the agent loop is an autonomous process with guardrails.

## v0.1.1 Recap (Completed)

| Deliverable | Status |
|---|---|
| 5-crate Rust workspace | ✅ |
| Anthropic + OpenRouter streaming | ✅ |
| 4 tools (read, write, edit, bash) | ✅ |
| ratatui TUI with popups, themes | ✅ |
| Slash commands, cost tracking | ✅ |
| 94 tests passing | ✅ |
| Clippy clean, fmt clean | ✅ |

---

## v0.2.0 Thesis

> **An AI coding agent is only as good as its context management and tool autonomy.** The LLM is a commodity — every agent uses the same models. What separates agents is what they show the model (context), what they let it do (tools), and how they manage the loop (intelligence).

v0.2.0 makes Catalyst the first CLI agent to treat context as a first-class managed resource instead of an unbounded message array.

### Competitive Positioning

```
                    Speed-focused
                         │
        Copilot ─────────┼───────── Cursor
                        │
     Autocomplete ──────┼────── Agent Mode
                        │
          Continue ─────┼───────── Aider
                        │
    Privacy-focused ────┼──────── Bolt.new
                        │
              CATALYST ─┘
              (Context-native agent)
```

**We don't compete on IDE integration (Cursor wins), browser UX (Bolt wins), or community (Aider wins). We compete on agent intelligence — specifically, how well the agent manages its own context and operates autonomously.**

---

## What We Cut

These were in the v0.2.0 roadmap. They're gone.

| Item | Why |
|---|---|
| Multiple color themes | Cosmetic. Doesn't differentiate. |
| Message search/copy/paste | Nice-to-have. Not core. |
| Conversation export (MD/JSON) | Nobody asked yet. |
| Syntax highlighting | Use `bat` externally. |
| Light/dark mode toggle | Cosmetic. |
| Video tutorials | Premature. |
| TLA+ verification | v0.3+ scope. Needs codebase intelligence first. |
| Simulation engine | v0.3+ scope. |
| IDE/LSP integration | Wrong market. Terminal is our home. |
| CI/CD integration | Premature. |
| AST parsing | v0.3 scope (tree-sitter indexing). |
| Code completion | IDE territory. |

---

## The 4 Pillars

### Pillar 1: Context Engine

**Problem:** Messages grow unbounded. After 5 tool calls you're sending 30K+ tokens of noise. Costs explode. LLM quality degrades. Eventually hits the token ceiling and crashes.

**Solution:** New module `catalyst-core/src/context.rs` that treats context as a managed resource with a budget.

#### Architecture

```
Context {
    system: SystemPrompt,        // dynamic — includes project state, not static text
    working: Vec<Message>,       // recent N messages, full fidelity
    archive: Vec<Summary>,       // older messages, LLM-summarized
    files: FileCache,            // file content cache — referenced, not inlined
    budget: TokenBudget,         // track + enforce limits per model
}
```

#### Token Budgeting

```
Model: claude-sonnet-4 (200K context window)

Allocation:
  System prompt + project context:  ~2,000 tokens (fixed)
  Tool definitions:                 ~1,000 tokens (fixed)
  Working memory (recent messages): ~20,000 tokens (configurable)
  Tool results (current request):   ~50,000 tokens (dynamic)
  Archive summaries:                ~5,000 tokens (compressed)
  ─────────────────────────────────────────────
  Total used:                       ~78,000 tokens
  Reserve for LLM output:           122,000 tokens
  
If tool results exceed budget → truncate intelligently:
  - Keep first N lines (function signatures, imports)
  - Keep last N lines (closing brackets, error messages)
  - Replace middle with "... [truncated, N lines omitted]"
```

#### Sliding Window

```
Messages 1-5:    Full fidelity in working memory
Messages 6-10:   Summarized into archive (LLM generates summary)
Messages 11+:    Dropped from active context (archived to disk)

On session resume:  Load archive summaries, not full history
On context overflow: Trigger emergency summarization
```

#### File Content Cache

```
Current behavior (v0.1.1):
  Tool reads file → full content inlined into message → sent to LLM every turn

v0.2 behavior:
  Tool reads file → content cached → reference sent to LLM:
    "[File: src/main.rs — 232 lines, cached. Use read to view specific sections.]"
  
  When LLM needs the file:
    - Full content sent once (first reference)
    - Subsequent references: diff-only if edited, or cache hit
    
  Cache invalidation:
    - File mtime changed → re-read and update cache
    - File edited by agent → update cache with new content
```

#### Implementation

**File:** `catalyst-core/src/context.rs`

```rust
pub struct ContextEngine {
    /// Token counting backend
    counter: TokenCounter,
    /// Maximum tokens for this model
    max_context: usize,
    /// Recent messages kept in full
    working_window: usize,
    /// File content cache
    file_cache: FileCache,
    /// Summarized older messages
    archive: Vec<Summary>,
}

impl ContextEngine {
    /// Build the final message array to send to the LLM
    pub fn build_messages(
        &self,
        messages: &[Message],
        system_prompt: &str,
        tools: &[ToolDef],
    ) -> Vec<Message>;
    
    /// Check if adding a tool result would overflow the budget
    pub fn would_overflow(&self, additional_tokens: usize) -> bool;
    
    /// Truncate a tool result to fit within budget
    pub fn truncate_output(&self, output: &str, budget: usize) -> String;
    
    /// Summarize old messages into archive
    pub async fn summarize_old_messages(
        &mut self,
        messages: &[Message],
        provider: &dyn LlmProvider,
    ) -> Result<Vec<Summary>>;
    
    /// Get or cache file content
    pub fn cache_file(&mut self, path: &Path, content: String) -> FileRef;
    
    /// Check if cached file is still valid (mtime check)
    pub fn is_cache_valid(&self, path: &Path) -> bool;
}

pub struct TokenBudget {
    pub system_prompt: usize,
    pub tool_definitions: usize,
    pub working_memory: usize,
    pub tool_results: usize,
    pub archive: usize,
    pub reserve_output: usize,
    pub model_limit: usize,
}

pub struct FileCache {
    entries: HashMap<PathBuf, CachedFile>,
}

pub struct CachedFile {
    content: String,
    mtime: std::time::SystemTime,
    token_count: usize,
    referenced_at: std::time::Instant,
}

pub struct Summary {
    /// What the user asked
    topic: String,
    /// What the agent did
    actions: Vec<String>,
    /// Key decisions or results
    outcomes: Vec<String>,
    /// Token count of this summary
    token_count: usize,
}
```

**Dependencies added:**
- `tiktoken-rs` or similar for token counting
- No new external crate for caching — use `HashMap` with mtime checks

---

### Pillar 2: Rich Tool System

**Problem:** The agent is blind. It can read/write/edit individual files, but can't discover files, search content, or understand project structure. Without `glob`/`grep`/`list`, multi-file work requires the LLM to guess paths.

**Solution:** 3 new tools + async tool trait + smart output truncation.

#### New Tools

**`glob` — Find files by pattern**

```rust
pub struct GlobTool;

impl Tool for GlobTool {
    fn name(&self) -> &str { "glob" }
    
    fn description(&self) -> &str {
        "Find files matching a glob pattern. Returns matched file paths relative to working directory."
    }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern (e.g. **/*.rs, src/**/*.ts, **/test*)"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return",
                    "default": 100
                }
            },
            "required": ["pattern"]
        })
    }
}
```

Implementation: use the `glob` crate. Returns paths sorted by modification time (most recent first). Truncates at `max_results` (default 100) to prevent context bloat.

**`grep` — Search file contents**

```rust
pub struct GrepTool;

impl Tool for GrepTool {
    fn name(&self) -> &str { "grep" }
    
    fn description(&self) -> &str {
        "Search file contents using regex. Returns matching lines with file paths and line numbers."
    }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file to search in (default: working directory)"
                },
                "include": {
                    "type": "string",
                    "description": "File glob to include (e.g. *.rs, *.ts)"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum matching lines to return",
                    "default": 50
                }
            },
            "required": ["pattern"]
        })
    }
}
```

Implementation: use the `regex` crate. Walk directory, filter by `include` glob, match lines. Returns `file:line: content` format. Truncates at `max_results` (default 50).

**`list` — Directory listing**

```rust
pub struct ListTool;

impl Tool for ListTool {
    fn name(&self) -> &str { "list" }
    
    fn description(&self) -> &str {
        "List directory contents with file metadata. Returns file names, sizes, and types."
    }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory to list (default: working directory)"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "List recursively",
                    "default": false
                },
                "max_depth": {
                    "type": "integer",
                    "description": "Maximum recursion depth",
                    "default": 3
                }
            },
            "required": []
        })
    }
}
```

Implementation: `std::fs::read_dir`. Returns entries with file type indicator (`/` for dirs, `*` for executable, nothing for regular files), size in human-readable format. Skips `target/`, `node_modules/`, `.git/` automatically.

#### Async Tool Trait

```rust
// Current (v0.1.1) — sync, blocking
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value;
    fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult>;
    fn clone_box(&self) -> Box<dyn Tool>;
}

// v0.2 — async, streaming-capable
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value;
    
    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult>;
    
    /// Maximum output size in characters. Results beyond this are truncated.
    fn output_limit(&self) -> usize { 10_000 }
    
    fn clone_box(&self) -> Box<dyn Tool>;
}
```

Why async: `grep` on large codebases and `glob` with many matches benefit from async I/O. `bash` already uses `spawn_blocking` — making tools async is cleaner.

#### Smart Output Truncation

```rust
impl ToolResult {
    /// Truncate output to fit within token budget.
    /// Preserves start and end, replaces middle with summary.
    pub fn truncate_for_context(&self, max_chars: usize) -> String {
        if self.output.len() <= max_chars {
            return self.output.clone();
        }
        
        let head_chars = max_chars / 3;
        let tail_chars = max_chars / 3;
        
        let head: String = self.output.chars().take(head_chars).collect();
        let tail: String = self.output.chars()
            .rev().take(tail_chars).collect::<String>()
            .chars().rev().collect();
        let omitted = self.output.len() - head_chars - tail_chars;
        
        format!(
            "{}\n\n... [truncated: {} characters omitted] ...\n\n{}",
            head, omitted, tail
        )
    }
}
```

#### Tool Registry Update

```rust
// Registry now supports async execution
impl ToolRegistry {
    pub async fn execute(
        &self,
        name: &str,
        args: Value,
        ctx: &ToolContext,
    ) -> Result<ToolResult>;
    
    /// Get tools as Anthropic-format definitions
    pub fn to_anthropic_tools(&self) -> Vec<Value>;  // unchanged
    
    /// Get the output limit for a specific tool
    pub fn output_limit(&self, name: &str) -> usize;
}
```

**Dependencies added:**
- `glob` crate for file pattern matching
- `regex` crate for content search

---

### Pillar 3: Agent Loop Intelligence

**Problem:** The current agent loop is naive: stream → detect tool → execute → recurse. No planning, no error recovery, no cancellation, no iteration limit. A runaway tool-use loop will burn tokens until the context window fills up and the API returns an error.

**Solution:** Structured execution with guardrails.

#### Current Flow (v0.1.1)

```
User → send() → process_stream() → [tool call] → process_stream() → ... → Complete
                     ↑                                                    │
                     └────────── recursive Box::pin() ←──────────────────┘
                   (unbounded recursion, no guardrails)
```

#### New Flow (v0.2)

```
User → send() → AgentLoop::run()
                      │
                      ▼
                 ┌──────────┐
                 │ Planning │ ← Inject system context, project state
                 └────┬─────┘
                      │
                      ▼
              ┌──────────────┐
              │  Executing   │ ← Stream LLM, detect tool calls
              │  (iterate)   │ ← Max 25 iterations per request
              └──┬───────┬──┘
                 │       │
            tool call   complete
                 │       │
                 ▼       ▼
          ┌──────────┐  ┌──────────┐
          │  Verify  │  │ Complete │ ← Emit AgentEvent::Complete
          │(optional)│  └──────────┘
          └────┬─────┘
               │
          retry needed?
               │
         ┌─────┴─────┐
         Yes         No
         │            │
         ▼            ▼
    back to       ┌──────────┐
    Executing     │  Done    │
                  └──────────┘
```

#### Agent State Machine

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum AgentState {
    /// Ready for user input
    Idle,
    /// Building context, preparing request
    Planning,
    /// Streaming LLM response, processing tool calls
    Executing { iteration: usize },
    /// Verifying tool results before continuing
    Verifying,
    /// Agent finished, response complete
    Complete,
    /// Unrecoverable error
    Error(String),
    /// User cancelled via Ctrl+C
    Cancelled,
}
```

#### Guardrails

```rust
pub struct AgentConfig {
    /// Maximum tool calls per user request (prevents runaway loops)
    pub max_iterations: usize,          // default: 25
    
    /// Maximum total tokens spent per request (cost guard)
    pub max_tokens_per_request: usize,  // default: 200_000
    
    /// Whether to automatically retry on tool errors
    pub auto_retry: bool,              // default: true
    
    /// Maximum retries per tool call
    pub max_retries: usize,            // default: 2
    
    /// Whether to inject project context into system prompt
    pub project_awareness: bool,       // default: true
}
```

#### Cancellation

```rust
pub struct AgentLoop {
    state: AgentState,
    config: AgentConfig,
    cancel: CancellationToken,
}

impl AgentLoop {
    /// Cancel the current operation. Agent emits AgentEvent::Cancelled
    /// and returns a partial response (whatever was generated so far).
    pub fn cancel(&mut self) {
        self.cancel.cancel();
        self.state = AgentState::Cancelled;
    }
}

// In TUI: Ctrl+C during streaming → cancel agent
// In TUI: Esc during streaming → pause (show thinking so far, ask continue/cancel)
```

#### Implementation

**File:** `catalyst-core/src/agent.rs` (restructured)

```rust
pub struct Agent {
    provider: Box<dyn LlmProvider + Send + Sync>,
    tools: ToolRegistry,
    messages: Vec<Message>,
    context: ContextEngine,      // NEW
    project: ProjectContext,     // NEW
    config: AgentConfig,         // NEW
    state: AgentState,           // NEW
    cancel: CancellationToken,   // NEW
}

impl Agent {
    pub async fn send(
        &mut self,
        user_message: String,
        tx: mpsc::UnboundedSender<AgentEvent>,
    ) -> Result<()> {
        self.state = AgentState::Planning;
        
        // 1. Add user message
        self.messages.push(Message {
            role: Role::User,
            content: Content::Text(user_message),
        });
        
        // 2. Build system prompt with project context
        let system = self.build_system_prompt();
        
        // 3. Execute agent loop with guardrails
        self.execute_loop(system, tx).await
    }
    
    async fn execute_loop(
        &mut self,
        system: String,
        tx: mpsc::UnboundedSender<AgentEvent>,
    ) -> Result<()> {
        let mut iteration = 0;
        
        loop {
            // Guard: max iterations
            if iteration >= self.config.max_iterations {
                let _ = tx.send(AgentEvent::Error(
                    format!("Max iterations ({}) reached. Stopping to prevent runaway.", 
                            self.config.max_iterations)
                ));
                self.state = AgentState::Complete;
                return Ok(());
            }
            
            // Guard: cancelled
            if self.cancel.is_cancelled() {
                self.state = AgentState::Cancelled;
                let _ = tx.send(AgentEvent::Cancelled);
                return Ok(());
            }
            
            self.state = AgentState::Executing { iteration };
            iteration += 1;
            
            // Build context-managed messages
            let messages = self.context.build_messages(
                &self.messages,
                &system,
                &self.tools.to_anthropic_tools(),
            );
            
            // Stream LLM response
            let mut stream = self.provider.stream(
                Some(&system),
                messages,
                self.tools.to_anthropic_tools(),
            ).await?;
            
            // Process stream events (same as v0.1.1 but with guards)
            let should_continue = self.process_stream(&mut stream, &tx).await?;
            
            if !should_continue {
                self.state = AgentState::Complete;
                return Ok(());
            }
        }
    }
}
```

**Dependencies added:**
- `tokio-util` for `CancellationToken` (or implement our own with `Arc<AtomicBool>`)

---

### Pillar 4: Project Awareness

**Problem:** The agent starts every conversation blind. It doesn't know what files exist, what language the project is, whether git is initialized, or what's recently changed. Every other serious agent injects project context.

**Solution:** Auto-index on startup, inject into system prompt.

#### Project Index

```rust
pub struct ProjectContext {
    /// Absolute path to project root
    root: PathBuf,
    /// Detected primary language
    language: ProjectLanguage,
    /// Truncated file tree (top 3 levels + file counts per directory)
    file_tree: String,
    /// Git information (if available)
    git: Option<GitContext>,
    /// Key files detected by convention (Cargo.toml, package.json, etc.)
    key_files: Vec<KeyFile>,
    /// When this index was built
    indexed_at: Instant,
}

pub enum ProjectLanguage {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Unknown,
}

pub struct GitContext {
    pub branch: String,
    pub modified_files: Vec<String>,
    pub staged_files: Vec<String>,
    pub recent_commits: Vec<CommitInfo>,
}

pub struct CommitInfo {
    pub hash: String,     // short hash
    pub message: String,  // first line only
    pub age: String,      // "2 hours ago", "3 days ago"
}

pub struct KeyFile {
    pub path: String,
    pub purpose: String,  // "package manifest", "entry point", etc.
}
```

#### Startup Indexing

```rust
impl ProjectContext {
    /// Build project index from working directory.
    /// Called once on startup, takes <100ms for most projects.
    pub fn index(working_dir: &Path) -> Result<Self> {
        let language = Self::detect_language(working_dir);
        let file_tree = Self::build_file_tree(working_dir, 3);
        let git = Self::detect_git(working_dir);
        let key_files = Self::find_key_files(working_dir, &language);
        
        Ok(Self {
            root: working_dir.to_path_buf(),
            language,
            file_tree,
            git,
            key_files,
            indexed_at: Instant::now(),
        })
    }
    
    fn detect_language(dir: &Path) -> ProjectLanguage {
        // Check for Cargo.toml → Rust
        // Check for package.json → TypeScript/JavaScript
        // Check for requirements.txt / pyproject.toml → Python
        // Check for go.mod → Go
        // Default → Unknown
    }
    
    fn build_file_tree(dir: &Path, max_depth: usize) -> String {
        // Walk directory up to max_depth
        // Skip: target/, node_modules/, .git/, __pycache__/, dist/, build/
        // Format:
        //   src/
        //   ├── cli/
        //   │   ├── main.rs (232 lines)
        //   │   └── config.rs (212 lines)
        //   ├── core/
        //   │   ├── agent.rs (533 lines)
        //   │   └── ...
        //   Cargo.toml
        //   README.md
    }
    
    fn detect_git(dir: &Path) -> Option<GitContext> {
        // Run: git rev-parse --is-inside-work-tree
        // Run: git branch --show-current
        // Run: git status --porcelain
        // Run: git log --oneline -5
        // If any fail, return None (no git or git not installed)
    }
}
```

#### System Prompt Injection

```rust
impl Agent {
    fn build_system_prompt(&self) -> String {
        let mut prompt = self.base_system_prompt.clone();
        
        if self.config.project_awareness {
            let project_context = self.project.to_prompt_text();
            prompt.push_str(&format!("\n\n## Project Context\n\n{}", project_context));
        }
        
        prompt
    }
}

impl ProjectContext {
    pub fn to_prompt_text(&self) -> String {
        let mut text = String::new();
        
        // Language
        text.push_str(&format!("Language: {}\n", self.language.display_name()));
        
        // File tree
        text.push_str(&format!("\nFile structure:\n{}\n", self.file_tree));
        
        // Key files
        if !self.key_files.is_empty() {
            text.push_str("\nKey files:\n");
            for f in &self.key_files {
                text.push_str(&format!("- {} ({})\n", f.path, f.purpose));
            }
        }
        
        // Git state
        if let Some(git) = &self.git {
            text.push_str(&format!("\nGit branch: {}\n", git.branch));
            
            if !git.modified_files.is_empty() {
                text.push_str("Modified files:\n");
                for f in &git.modified_files {
                    text.push_str(&format!("- {}\n", f));
                }
            }
            
            if !git.recent_commits.is_empty() {
                text.push_str("Recent commits:\n");
                for c in &git.recent_commits {
                    text.push_str(&format!("- {} {} ({})\n", c.hash, c.message, c.age));
                }
            }
        }
        
        text
    }
}
```

#### Cost Impact

```
Project context injection:
  File tree (3 levels):    ~200-500 tokens (most projects)
  Git status:              ~50-100 tokens
  Key files:               ~50-100 tokens
  ────────────────────────────────────────
  Total overhead:          ~300-700 tokens per request
  
Cost per request (Sonnet 4 @ $3/M input):
  500 tokens × $3/1M = $0.0015
  
Value: Agent avoids 2-3 exploratory tool calls per task
  (each tool call = full request/response cycle = ~$0.02-0.05)
  Net savings: ~$0.04-0.15 per task
```

**No new dependencies.** Uses `std::fs` for file tree, `std::process::Command` for git.

---

## Architecture Changes

### Crate Structure (Unchanged: 5 Crates)

```
catalyst/
├── catalyst-cli/           (entry point)
│   ├── src/main.rs         (updated: project indexing on startup)
│   └── src/config.rs       (updated: new config fields for agent limits)
│
├── catalyst-core/          (agent logic — restructured)
│   ├── src/agent.rs        (agent state machine + execution loop)
│   ├── src/context.rs      (NEW: context engine, token budgeting, file cache)
│   ├── src/project.rs      (NEW: project indexing, git awareness)
│   ├── src/event.rs        (agent events — extended with Cancelled)
│   └── src/lib.rs          (re-exports)
│
├── catalyst-llm/           (LLM providers — minor updates)
│   ├── src/anthropic.rs    (unchanged)
│   ├── src/openrouter.rs   (unchanged)
│   ├── src/provider.rs     (unchanged)
│   ├── src/types.rs        (unchanged)
│   ├── src/client.rs       (unchanged)
│   └── src/lib.rs          (unchanged)
│
├── catalyst-tools/         (tools — expanded)
│   ├── src/tools/
│   │   ├── mod.rs          (tool re-exports)
│   │   ├── read.rs         (extracted from tools.rs)
│   │   ├── write.rs        (extracted from tools.rs)
│   │   ├── edit.rs         (extracted from tools.rs)
│   │   ├── bash.rs         (extracted from tools.rs)
│   │   ├── glob.rs         (NEW: file pattern matching)
│   │   ├── grep.rs         (NEW: content search)
│   │   └── list.rs         (NEW: directory listing)
│   ├── src/registry.rs     (updated: async execution)
│   ├── src/lib.rs          (updated: async tool trait)
│   └── src/context.rs      (unchanged)
│
└── catalyst-tui/           (TUI — minor updates)
    ├── src/lib.rs          (updated: cancellation support)
    ├── src/app.rs          (updated: AgentState display, cancel event)
    ├── src/ui.rs           (updated: show iteration count, project info)
    ├── src/command.rs      (unchanged)
    ├── src/theme.rs        (unchanged)
    └── src/lib.rs          (unchanged)
```

### Dependency Changes

```toml
# catalyst-core/Cargo.toml — additions
[dependencies]
tiktoken-rs = "0.6"         # Token counting for context budgeting
tokio-util = "0.7"          # CancellationToken

# catalyst-tools/Cargo.toml — additions
[dependencies]
glob = "0.3"                # File pattern matching
regex = "1.10"              # Content search
async-trait = "0.1"         # Async tool trait (moved from being internal)
```

### New Events

```rust
// catalyst-core/src/event.rs — additions to AgentEvent
pub enum AgentEvent {
    // ... existing variants ...
    
    /// Agent was cancelled by user
    Cancelled,
    
    /// Agent state changed
    StateChanged { from: AgentState, to: AgentState },
    
    /// Context budget warning (approaching limit)
    ContextBudgetWarning {
        used: usize,
        limit: usize,
        percentage: f64,
    },
    
    /// Tool output was truncated
    OutputTruncated {
        original_len: usize,
        truncated_len: usize,
    },
}
```

---

## Implementation Waves

### Wave 1: Foundation (Week 1-2)

Groundwork. Everything else depends on this.

| # | Task | File(s) | Effort | Dependencies |
|---|---|---|---|---|
| 1.1 | Async tool trait migration | `catalyst-tools/src/lib.rs` | 1 day | None |
| 1.2 | Extract tools into separate files | `catalyst-tools/src/tools/*.rs` | 0.5 day | 1.1 |
| 1.3 | Implement `glob` tool | `catalyst-tools/src/tools/glob.rs` | 1 day | 1.1 |
| 1.4 | Implement `grep` tool | `catalyst-tools/src/tools/grep.rs` | 1 day | 1.1 |
| 1.5 | Implement `list` tool | `catalyst-tools/src/tools/list.rs` | 0.5 day | 1.1 |
| 1.6 | Tool output truncation | `catalyst-tools/src/lib.rs` | 0.5 day | 1.1 |
| 1.7 | Update registry for async | `catalyst-tools/src/registry.rs` | 0.5 day | 1.1 |
| 1.8 | Max iteration guard in agent | `catalyst-core/src/agent.rs` | 0.5 day | None |
| 1.9 | CancellationToken integration | `catalyst-core/src/agent.rs` | 0.5 day | None |
| 1.10 | Ctrl+C → cancel in TUI | `catalyst-tui/src/lib.rs` | 0.5 day | 1.9 |
| 1.11 | Tests for new tools | `catalyst-tools/src/tools/*.rs` | 1 day | 1.3-1.5 |

**Wave 1 Deliverable:** 7 tools, async trait, iteration guard, cancellation.

### Wave 2: Context Engine (Week 3-4)

The core differentiator.

| # | Task | File(s) | Effort | Dependencies |
|---|---|---|---|---|
| 2.1 | Token counting module | `catalyst-core/src/context.rs` | 1 day | None |
| 2.2 | TokenBudget struct + allocation | `catalyst-core/src/context.rs` | 0.5 day | 2.1 |
| 2.3 | File content cache | `catalyst-core/src/context.rs` | 1 day | None |
| 2.4 | Context message builder | `catalyst-core/src/context.rs` | 2 days | 2.1, 2.2 |
| 2.5 | Sliding window + summarization stub | `catalyst-core/src/context.rs` | 2 days | 2.4 |
| 2.6 | Integrate context engine into agent | `catalyst-core/src/agent.rs` | 1 day | 2.4 |
| 2.7 | Context budget warnings | `catalyst-core/src/agent.rs` | 0.5 day | 2.6 |
| 2.8 | Tests for context engine | `catalyst-core/src/context.rs` | 2 days | 2.6 |

**Wave 2 Deliverable:** Context engine that manages token budgets, caches files, and builds context-aware messages.

### Wave 3: Project Awareness + Agent Intelligence (Week 5-6)

| # | Task | File(s) | Effort | Dependencies |
|---|---|---|---|---|
| 3.1 | Language detection | `catalyst-core/src/project.rs` | 0.5 day | None |
| 3.2 | File tree builder | `catalyst-core/src/project.rs` | 1 day | None |
| 3.3 | Git context detection | `catalyst-core/src/project.rs` | 1 day | None |
| 3.4 | Key file detection | `catalyst-core/src/project.rs` | 0.5 day | 3.1 |
| 3.5 | Dynamic system prompt builder | `catalyst-core/src/agent.rs` | 1 day | 3.2, 3.3 |
| 3.6 | Agent state machine | `catalyst-core/src/agent.rs` | 1 day | Wave 1 |
| 3.7 | Error retry in agent loop | `catalyst-core/src/agent.rs` | 1 day | 3.6 |
| 3.8 | Session persistence (save/resume) | `catalyst-core/src/session.rs` | 2 days | 2.6 |
| 3.9 | @file reference syntax | `catalyst-core/src/agent.rs` | 1 day | 2.3 |
| 3.10 | TUI updates (state display, project info) | `catalyst-tui/src/*.rs` | 1 day | 3.6 |

**Wave 3 Deliverable:** Project-aware agent with state machine, session persistence, and file references.

### Wave 4: Polish + Release (Week 7)

| # | Task | File(s) | Effort | Dependencies |
|---|---|---|---|---|
| 4.1 | Integration tests | `tests/` | 2 days | All waves |
| 4.2 | Config file updates (new fields) | `catalyst-cli/src/config.rs` | 0.5 day | Wave 3 |
| 4.3 | README.md rewrite | `README.md` | 0.5 day | All waves |
| 4.4 | USAGE.md update | `USAGE.md` | 0.5 day | All waves |
| 4.5 | CHANGELOG.md update | `CHANGELOG.md` | 0.5 day | All waves |
| 4.6 | Clippy + fmt pass | workspace | 0.5 day | All waves |
| 4.7 | Version bump to 0.2.0 | `Cargo.toml` | 0.1 day | All tasks |

---

## Success Criteria

> **v0.2.0 is done when: you can ask Catalyst "add error handling to all public functions in catalyst-tools" and it autonomously discovers the files, reads them, plans the changes, executes edits, and verifies the result — all without hitting token limits, losing context, or needing hand-holding.**

### Specific Criteria

| Criteria | Verification |
|---|---|
| 7 tools working (4 existing + 3 new) | `cargo test --workspace` passes with new tool tests |
| Async tool trait | All tools implement async `execute()` |
| Max iteration guard | Agent stops after 25 tool calls, emits error |
| Cancellation works | Ctrl+C during streaming stops agent cleanly |
| Token counting | Context engine counts tokens accurately (±5%) |
| Token budget enforced | Agent never exceeds configured context window |
| File caching | Same file not sent twice in one session |
| Output truncation | Tool results >10K chars truncated intelligently |
| Project awareness | System prompt includes file tree, git status, language |
| Session persistence | Can quit and resume a conversation |
| @file references | `@src/main.rs` in message auto-injects file content |
| 94+ tests passing | `cargo test --workspace` green |
| Zero clippy warnings | `cargo clippy --all-targets -- -D warnings` clean |
| Clean format | `cargo fmt --check` passes |

### Performance Targets

| Metric | Target | Why |
|---|---|---|
| Startup time | < 200ms | Project indexing must be fast |
| Context build time | < 50ms | Per-request overhead |
| Token counting overhead | < 10ms per message | Must not slow streaming |
| Memory idle | < 50MB | Rust advantage over Python/TS agents |

---

## Future Roadmap

### v0.3.0 — Codebase Intelligence (Q3 2026)

Builds on v0.2.0's context engine:
- Tree-sitter indexing for semantic code understanding
- Symbol navigation (go to definition, find references)
- Dependency graph awareness
- Context-aware code suggestions based on call graph

**Why it depends on v0.2:** You can't build semantic search without a context management system. The context engine from v0.2.0 becomes the foundation for storing and retrieving tree-sitter indices.

### v0.4.0 — Verification Layer (Q4 2026)

Builds on v0.3.0's codebase intelligence:
- Behavioral testing (auto-generate tests for changes)
- Invariant checking (detect violated assumptions)
- Lightweight formal verification (TLA-lite for concurrent code)
- Change impact analysis (what does this edit affect?)

**Why it depends on v0.3:** You can't verify code without understanding its structure. Tree-sitter from v0.3 provides the AST needed for verification.

### v1.0.0 — Production Catalyst (Q1 2027)

- Extension/plugin system
- Multi-agent orchestration (planner + coder + reviewer)
- Team features (shared context, rules)
- Platform binaries (macOS, Linux, Windows)
- Homebrew/cargo install

---

## Technical Debt Addressed

| Item | v0.1.1 State | v0.2.0 Fix |
|---|---|---|
| Tool trait is sync | Blocking in async context | Async trait |
| No token counting | Guess at context size | tiktoken-rs counting |
| Unbounded recursion | Can loop forever | Iteration guard |
| No cancellation | Must wait for completion | CancellationToken |
| Static system prompt | Same text every request | Dynamic with project context |
| All files re-read | Content duplicated in messages | File cache |
| Large tool outputs | Full content sent to LLM | Smart truncation |
| `client.rs` dead code | Legacy SseStream unused | Remove in v0.2 |

---

## Release Checklist

### Pre-Release
- [ ] All Wave 1-4 tasks complete
- [ ] All success criteria verified
- [ ] `cargo test --workspace` green
- [ ] `cargo clippy` clean
- [ ] `cargo fmt` clean
- [ ] No new `unwrap()` in production code
- [ ] API keys never logged

### Testing
- [ ] Manual test: multi-file edit task end-to-end
- [ ] Manual test: cancellation during streaming
- [ ] Manual test: session save/resume
- [ ] Manual test: context budget enforcement
- [ ] Manual test: all 7 tools
- [ ] Manual test: project awareness on Rust project
- [ ] Manual test: project awareness on non-git directory

### Documentation
- [ ] README.md rewritten for v0.2.0 positioning
- [ ] USAGE.md updated with new tools and features
- [ ] CHANGELOG.md entry for v0.2.0
- [ ] CONTRIBUTING.md updated

### Release
- [ ] Version bumped to 0.2.0 in all Cargo.toml files
- [ ] Git tag `v0.2.0` created
- [ ] GitHub release published with notes
- [ ] Binaries built for macOS (aarch64 + x86_64) and Linux (x86_64)
