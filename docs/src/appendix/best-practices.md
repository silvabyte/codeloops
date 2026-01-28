# Best Practices

This guide covers best practices for getting the most out of codeloops, based on patterns observed across real sessions.

## The Planning Workflow

The most successful codeloops sessions follow a two-phase workflow:

### Phase 1: Interactive Exploration

Use your coding agent interactively to explore and plan:

```bash
# Start your agent
opencode  # or claude

# Explore the problem
> Analyze the authentication flow and identify where rate limiting should be added

# Generate a detailed prompt
> /promptmd
```

### Phase 2: Disciplined Execution

Run codeloops with the generated prompt:

```bash
codeloops
```

This workflow leverages your agent's interactive capabilities for exploration, then uses codeloops' actor-critic loop for structured, self-correcting execution.

## What Makes Prompts Succeed in One Iteration

Analysis of successful single-iteration sessions reveals these patterns:

### Include Root Cause Analysis (for bugs)

Don't just describe symptoms—explain why it's happening:

**Good:**
```markdown
## Problem
The daemon logs spam this error every ~6 seconds:
`Telegram poll error: Ctrl-C signal handler already registered`

## Root Cause
The daemon registers a `ctrlc::set_handler` at `crates/butler/src/commands/daemon.rs:79`.
Then it calls `telegram::poll()` which also calls `ctrlc::set_handler()` at line 40.
The `ctrlc` crate only allows one global handler—the second registration fails.

## Fix
Skip the ctrlc registration in `poll()` when called with `once: true`.

## Files
- `crates/butler/src/commands/telegram.rs` — lines 38-42
- `crates/butler/src/commands/daemon.rs` — lines 79-81, 91
```

### Provide Exact File Paths and Line Numbers

Specificity eliminates guesswork:

**Good:**
```markdown
In `run.rs`, guard against empty output before sending to Telegram:

File: `crates/butler/src/commands/run.rs` — lines 100-103

Change:
```rust
OutputTarget::Telegram => {
    let msg = output_result.stdout.trim();
    if msg.is_empty() {
        eprintln!("Warning: playbook produced no output");
    } else {
        tg.send_message(msg).await?;
    }
}
```
```

### Include Schema Definitions for Data Changes

When adding database tables or data structures, include the full schema:

**Good:**
```markdown
## Database Schema

```sql
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    trigger_at TEXT,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_tasks_status_trigger ON tasks(status, trigger_at);
```
```

### Structure Complex Tasks in Phases

Break large features into numbered, dependent tasks:

**Good:**
```markdown
## Phase 1: Foundation Setup

### Task 1.1: Create the database module
Location: `crates/butler-db/`
- Implement `open_db()` with WAL mode
- Add migration helpers

### Task 1.2: Add state helpers
- Implement `get_state(conn, table, key)`
- Implement `set_state(conn, table, key, value)`

## Phase 2: Migration

### Task 2.1: Migrate telegram state
- Replace JSON file reads with database queries
- Update `crates/butler/src/commands/telegram.rs`
```

## What Causes Multiple Iterations

Sessions that require 2-3 iterations typically have these gaps:

### Missing Edge Cases

The critic catches issues the prompt didn't mention:

- Double initialization of global state
- Empty output handling
- Error propagation paths
- Clippy warnings in new code

**Prevention:** Include a "Validation Checklist" in your prompt:

```markdown
## Validation Checklist
- [ ] No clippy warnings in modified files
- [ ] Error cases return appropriate error types
- [ ] Empty/null inputs are handled
- [ ] New code has test coverage
```

### Ambiguous "Done" Criteria

When the critic can't determine if requirements are met, it requests more changes.

**Prevention:** Use explicit acceptance criteria with checkboxes:

```markdown
## Acceptance Criteria
- [ ] `bun scripts/generate-route-config.ts --check` returns exit code 0 when in sync
- [ ] `bun scripts/generate-route-config.ts --check` returns exit code 1 when out of sync
- [ ] Pre-commit hook fails with clear error message when route config is stale
- [ ] All existing tests pass
```

## Writing Effective Prompts

### Be Specific

Vague prompts lead to vague results:

**Bad:**
```markdown
Improve the code.
```

**Good:**
```markdown
Refactor the `calculate_total` function in src/cart.rs to:
1. Use iterators instead of manual loops
2. Handle the case where the cart is empty
3. Add documentation comments
```

### Include Acceptance Criteria

Tell the critic what "done" looks like:

```markdown
## Task
Add rate limiting to the API endpoint.

## Acceptance Criteria
- [ ] Limit to 100 requests per minute per IP
- [ ] Return 429 status code when limit exceeded
- [ ] Include Retry-After header in 429 responses
- [ ] Log rate limit violations
```

### Provide Context

Reference specific files and functions:

```markdown
Add pagination to the `list_users` function in src/api/users.rs.

The function currently returns all users. Change it to:
- Accept `page` and `per_page` query parameters
- Default to page 1 with 20 items per page
- Return total count in the response

Related types are in src/models/user.rs and src/api/types.rs.
```

### One Task Per Prompt

Don't combine unrelated tasks:

**Bad:**
```markdown
Fix the login bug and add a new dashboard page and update the README.
```

**Good:**
Create separate prompts for each:
1. `prompt-login.md`: Fix the login bug
2. `prompt-dashboard.md`: Add dashboard page
3. `prompt-readme.md`: Update README

## Structuring Complex Tasks

### Break Down Large Features

Instead of one large prompt:
```markdown
Implement complete user authentication with registration, login,
logout, password reset, and email verification.
```

Create a series of prompts:
1. User registration
2. User login
3. Logout
4. Password reset (request)
5. Password reset (confirm)
6. Email verification

Run them sequentially, each building on the previous.

### Use Iteration Limits Wisely

Set limits based on task complexity:

| Task Type | Suggested Limit |
|-----------|-----------------|
| Simple fix | 2-3 |
| Small feature | 5 |
| Medium feature | 10 |
| Complex task | Consider breaking down |

```bash
codeloops --max-iterations 5
```

## Agent Selection

### Same Agent for Both Roles

Use the same agent when:
- Starting out (simpler configuration)
- The agent performs well for your use case
- You want predictable behavior

```bash
codeloops --agent claude
```

### Different Agents per Role

Mix agents when:
- You want fast iteration with thorough review
- Different agents excel at different aspects

```bash
# Fast actor, thorough critic
codeloops --actor-agent opencode --critic-agent claude
```

### Agent-Task Matching

| Task Type | Recommended |
|-----------|-------------|
| Complex refactoring | Claude (strong reasoning) |
| Simple fixes | OpenCode (fast) |
| Cursor-centric workflow | Cursor |

## Working with the Feedback Loop

### Trust the Critic

The critic provides valuable feedback. If the loop continues, review the feedback to understand what's missing.

### Know When to Cancel

Cancel and reformulate if:
- Multiple iterations produce similar results
- The critic's feedback seems stuck
- The task scope may be wrong

Press `Ctrl+C` to cancel, then revise your prompt.

### Learn from Sessions

Review completed sessions to improve future prompts:

```bash
codeloops sessions show
```

Look for patterns:
- What feedback was given?
- How many iterations did similar tasks take?
- What made some tasks succeed faster?

## CI/CD Integration

### JSON Output

Use JSON output for machine parsing:

```bash
codeloops --json-output > result.json
```

### Exit Codes

Use exit codes in scripts:

```bash
codeloops --max-iterations 3 || echo "Task incomplete"
```

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Max iterations reached |
| 2 | Failed |
| 130 | Interrupted |

### Configuration Files

Use project configuration for CI consistency:

```toml
# codeloops.toml
agent = "claude"
max_iterations = 5
log_format = "json"
```

### Pipeline Example

```yaml
# .github/workflows/autofix.yml
- name: Run codeloops
  run: |
    codeloops --prompt "${{ github.event.issue.body }}" \
      --max-iterations 3 \
      --json-output > result.json
  continue-on-error: true

- name: Check result
  run: |
    outcome=$(jq -r '.outcome' result.json)
    if [ "$outcome" != "success" ]; then
      echo "Task did not complete successfully"
      exit 1
    fi
```

## Performance Tips

### Minimize Iteration Count

Better prompts = fewer iterations = faster completion.

### Use Appropriate Agents

Faster agents reduce total time when the task is straightforward.

### Limit Scope

Smaller, focused tasks complete faster than large ones.

## Common Pitfalls

### Ambiguous Requirements

**Problem:** Critic keeps requesting changes because requirements are unclear.

**Solution:** Add explicit acceptance criteria to your prompt. Use checkboxes that the critic can verify.

### Too Many Unrelated Changes

**Problem:** Actor makes changes beyond what was asked.

**Solution:** Be specific about scope in the prompt. Add "Do not modify other files" if needed.

### Ignoring Feedback

**Problem:** Re-running the same prompt hoping for different results.

**Solution:** Read the critic's feedback and adjust your prompt or approach. The critic often catches legitimate issues like:
- Double initialization of global state
- Missing error handling
- Clippy warnings

### Overly Broad Tasks

**Problem:** Task never completes because scope is too large.

**Solution:** Break into smaller, manageable tasks. Use the `/promptmd` workflow to plan interactively, then execute each phase with codeloops.

### Missing Context

**Problem:** Actor can't find the right files or uses wrong patterns.

**Solution:** Include file paths and line numbers. Reference existing code patterns by showing examples from the codebase.

### Describing Symptoms Instead of Causes

**Problem:** Bug fix prompts describe what's wrong but not why.

**Solution:** Include root cause analysis. Explain *why* the bug happens, not just *what* happens. This dramatically reduces iterations.

**Bad:**
```markdown
The daemon crashes on startup.
```

**Good:**
```markdown
The daemon crashes on startup because tracing-subscriber is initialized twice:
1. `main.rs:10` calls `tracing_subscriber::fmt().init()`
2. `daemon.rs:66` calls it again, which panics

Fix: Use `try_init()` instead of `init()` in main.rs.
```

### Not Including Quality Assurance Steps

**Problem:** Prompt completes but leaves behind clippy warnings, failing tests, or missing documentation.

**Solution:** Always include QA requirements in your prompt:

```markdown
## Quality Requirements
- Run `cargo clippy` and fix all warnings in modified files
- Ensure all existing tests pass
- Add tests for new functionality
- Update relevant documentation
```

## Security Considerations

### Review Changes

Always review the git diff before committing:

```bash
git diff
```

### Sensitive Files

Don't include sensitive information in prompts. The prompt is logged in the session file.

### Access Control

Agents have full filesystem access in the working directory. Run codeloops in appropriate environments.

## Session Management

### Regular Cleanup

Delete old sessions periodically:

```bash
# Delete sessions older than 30 days
find ~/.local/share/codeloops/sessions -name "*.jsonl" -mtime +30 -delete
```

### Backup Important Sessions

Copy sessions you want to preserve:

```bash
cp ~/.local/share/codeloops/sessions/important-session.jsonl ~/backups/
```

### Analyze Patterns

Use statistics to improve your workflow:

```bash
codeloops sessions stats
```

Look at success rates by project to identify areas for improvement.
