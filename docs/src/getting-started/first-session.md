# Your First Session

This guide provides a detailed walkthrough of a real codeloops session, explaining each step of the actor-critic loop.

## The Task

Let's say you're working on a web application and need to add input validation to a user registration endpoint. The endpoint currently accepts any input without checking for valid email format or password requirements.

## Setting Up

Navigate to your project and create a detailed prompt:

```bash
cd ~/projects/myapp
```

Create `prompt.md`:

```markdown
Add input validation to the user registration endpoint in src/api/users.rs:

Requirements:
- Email must be a valid email format
- Password must be at least 8 characters
- Password must contain at least one uppercase letter and one number
- Return appropriate error messages for validation failures

The endpoint is the `register` function that handles POST /api/users/register.
```

## Running the Loop

Start codeloops:

```bash
codeloops
```

## Iteration 1: Initial Attempt

### Actor Phase

The actor receives your prompt and starts working:

```
[iteration 1]
[actor] Running Claude Code...
```

The actor examines your codebase, finds `src/api/users.rs`, and implements validation. After completion:

```
[actor] Completed in 45.2s (exit code: 0)
[git] 2 files changed, 47 insertions(+), 3 deletions(-)
```

The actor made changes to 2 files and the git diff is captured.

### Critic Phase

The critic receives:
- Your original prompt
- The actor's stdout output
- The git diff showing all changes

The critic evaluates whether the changes meet your requirements:

```
[critic] Evaluating changes...
[critic] Decision: CONTINUE
[critic] Feedback: The email validation is implemented but the password
         validation only checks length. Missing: uppercase letter check
         and number requirement. Also, error messages are generic -
         please provide specific feedback for each validation rule.
```

### What Happened

The actor's first attempt was incomplete. It added email validation and password length checking, but missed:
- Uppercase letter requirement
- Number requirement
- Specific error messages

The critic identified these gaps and provided feedback.

## Iteration 2: Addressing Feedback

### Actor Phase (with Feedback)

The actor now receives the original prompt plus the critic's feedback:

```
[iteration 2]
[actor] Running Claude Code...
```

The actor reads the feedback and addresses each point:

```
[actor] Completed in 32.1s (exit code: 0)
[git] 1 file changed, 28 insertions(+), 12 deletions(-)
```

### Critic Phase

The critic re-evaluates:

```
[critic] Evaluating changes...
[critic] Decision: DONE
[critic] Summary: Input validation is now complete. Email format is
         validated using regex. Password validation checks: minimum
         8 characters, at least one uppercase letter (A-Z), at least
         one digit (0-9). Error messages are specific and helpful.
[critic] Confidence: 0.95
```

## Session Complete

```
[codeloops] Session complete: success (2 iterations)
[codeloops] Duration: 89.4s
[codeloops] Session saved: 2025-01-27T15-30-45Z_a3f2c1
```

The loop ends because the critic returned `DONE`.

## Reviewing the Session

### Using the CLI

View the session details:

```bash
codeloops sessions show 2025-01-27T15-30-45Z_a3f2c1
```

Or use the interactive picker:

```bash
codeloops sessions show
```

### Viewing the Diff

See the cumulative changes across all iterations:

```bash
codeloops sessions diff 2025-01-27T15-30-45Z_a3f2c1
```

### Using the Web UI

For a visual interface:

```bash
codeloops ui
```

Navigate to the session to see:
- Iteration timeline
- Each iteration's diff with syntax highlighting
- Critic feedback trail

## Understanding the Session File

The session is stored at `~/.local/share/codeloops/sessions/2025-01-27T15-30-45Z_a3f2c1.jsonl`:

```json
{"type":"session_start","timestamp":"2025-01-27T15:30:45Z","prompt":"Add input validation...","working_dir":"/home/user/projects/myapp","actor_agent":"Claude Code","critic_agent":"Claude Code"}
{"type":"iteration","iteration_number":1,"actor_output":"...","git_diff":"...","critic_decision":"CONTINUE","feedback":"The email validation is implemented but..."}
{"type":"iteration","iteration_number":2,"actor_output":"...","git_diff":"...","critic_decision":"DONE","feedback":null}
{"type":"session_end","outcome":"success","iterations":2,"summary":"Input validation is now complete...","confidence":0.95,"duration_secs":89.4}
```

## What If It Doesn't Go Well?

### Task Takes Too Many Iterations

If the loop continues for many iterations without completing:

1. **Set a limit**: Use `--max-iterations 5` to cap the attempts
2. **Cancel and reformulate**: Press Ctrl+C and rewrite your prompt with more detail
3. **Review the feedback**: Check what the critic is asking for

### Actor Produces Errors

If the actor exits with an error (non-zero exit code):

1. The critic will suggest recovery steps
2. The actor tries again with recovery guidance
3. If errors persist, the session ends as `failed`

### Critic Always Says CONTINUE

This usually means your prompt is ambiguous. The critic doesn't know when the task is "done" because the requirements aren't clear. Add explicit acceptance criteria to your prompt.

## Tips for Success

1. **Be specific**: Include file paths, function names, and exact requirements
2. **Define "done"**: What criteria must be met for the task to be complete?
3. **Provide context**: Mention relevant parts of your codebase
4. **Start small**: Complex tasks often work better when broken into smaller prompts

## Next Steps

- Learn about [Configuration](../user-guide/configuration.md) to customize agent behavior
- See [Writing Prompts](../user-guide/prompts.md) for prompt best practices
- Explore [Sessions](../user-guide/sessions.md) for session management
