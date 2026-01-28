# Best Practices

This guide covers best practices for getting the most out of codeloops.

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

**Solution:** Add explicit acceptance criteria to your prompt.

### Too Many Unrelated Changes

**Problem:** Actor makes changes beyond what was asked.

**Solution:** Be specific about scope in the prompt. Add "Do not modify other files" if needed.

### Ignoring Feedback

**Problem:** Re-running the same prompt hoping for different results.

**Solution:** Read the critic's feedback and adjust your prompt or approach.

### Overly Broad Tasks

**Problem:** Task never completes because scope is too large.

**Solution:** Break into smaller, manageable tasks.

### Missing Context

**Problem:** Actor can't find the right files or uses wrong patterns.

**Solution:** Include file paths and reference existing code patterns.

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
