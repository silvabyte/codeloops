# The Actor-Critic Loop

This document explains the core feedback loop that drives codeloops.

## Concept

The actor-critic pattern comes from reinforcement learning, where:
- The **actor** takes actions (makes code changes)
- The **critic** evaluates those actions (reviews the changes)

In codeloops:
- The actor is a coding agent executing your task
- The critic is another agent instance evaluating the work
- Feedback flows from critic to actor until the task is complete

## State Machine

```
                    ┌──────────────────┐
                    │      START       │
                    └────────┬─────────┘
                             │
                             ▼
                    ┌──────────────────┐
              ┌────▶│ ACTOR_EXECUTING  │◀────┐
              │     └────────┬─────────┘     │
              │              │               │
              │              ▼               │
              │     ┌──────────────────┐     │
              │     │ CAPTURING_DIFF   │     │
              │     └────────┬─────────┘     │
              │              │               │
              │              ▼               │
              │     ┌──────────────────┐     │
              │     │CRITIC_EVALUATING │     │
              │     └────────┬─────────┘     │
              │              │               │
              │   ┌──────────┼──────────┐    │
              │   │          │          │    │
              │   ▼          ▼          ▼    │
              │ ┌────┐   ┌────────┐  ┌─────┐ │
              │ │DONE│   │CONTINUE│  │ERROR│ │
              │ └──┬─┘   └───┬────┘  └──┬──┘ │
              │    │         │          │    │
              │    │         │ feedback │    │
              │    │         └──────────┼────┘
              │    │                    │
              │    │     recovery       │
              │    │     suggestion     │
              │    │         ┌──────────┘
              │    │         │
              │    ▼         ▼
              │ ┌──────┐  ┌──────────────┐
              │ │ END  │  │ FEED_BACK    │
              │ └──────┘  └──────┬───────┘
              │                  │
              └──────────────────┘
```

## Decision Types

The critic returns one of three decisions:

### DONE

The task is complete. The critic has determined that:
- All requirements in the prompt are met
- The implementation is correct
- No further changes are needed

Response includes:
- Summary of what was accomplished
- Confidence score (0.0 to 1.0)

### CONTINUE

More work is needed. The critic has determined that:
- The prompt requirements are not fully met
- There are issues that need addressing
- Additional changes are required

Response includes:
- Feedback explaining what's missing or wrong
- Guidance for the next iteration

### ERROR

Something went wrong. This occurs when:
- The actor's exit code indicates failure
- The actor produced error output
- The changes broke something

Response includes:
- Analysis of what went wrong
- Recovery suggestion for the actor

## Iteration Flow

### 1. Actor Execution

The actor receives:
- Original prompt (always)
- Previous feedback (if CONTINUE or ERROR)

```
┌─────────────────────────────────────────────────┐
│                 Actor Input                     │
├─────────────────────────────────────────────────┤
│ Original Prompt:                                │
│   Add input validation to the login endpoint.   │
│                                                 │
│ Previous Feedback (if any):                     │
│   The email validation is good, but password    │
│   validation is missing. Please add checks for  │
│   minimum length and required characters.       │
└─────────────────────────────────────────────────┘
```

The actor executes with full filesystem access in the working directory.

### 2. Diff Capture

After the actor completes, codeloops captures:
- Git diff (all changes since session start)
- Number of files changed
- Actor stdout and stderr
- Actor exit code
- Execution duration

### 3. Critic Evaluation

The critic receives:
- Original prompt
- Actor's output (stdout)
- Git diff of changes
- Iteration number
- Previous history (summarized)

```
┌─────────────────────────────────────────────────┐
│                 Critic Input                    │
├─────────────────────────────────────────────────┤
│ Task Prompt:                                    │
│   Add input validation to the login endpoint.   │
│                                                 │
│ Actor Output:                                   │
│   I've added email validation using regex and   │
│   password length checking...                   │
│                                                 │
│ Git Diff:                                       │
│   diff --git a/src/auth.rs b/src/auth.rs        │
│   +    if !is_valid_email(&email) { ... }       │
│   +    if password.len() < 8 { ... }            │
│                                                 │
│ Iteration: 1                                    │
└─────────────────────────────────────────────────┘
```

### 4. Decision Parsing

The critic's response is parsed to extract:
- Decision (DONE, CONTINUE, or ERROR)
- Feedback or summary text
- Confidence score (for DONE)

Expected critic output format:
```
DECISION: DONE

SUMMARY: Input validation has been added to the login endpoint.
Email addresses are validated using RFC 5321 compliant regex.
Passwords require minimum 8 characters.

CONFIDENCE: 0.95
```

Or for CONTINUE:
```
DECISION: CONTINUE

FEEDBACK: The email validation looks good, but password validation
only checks length. The requirements also specified:
- At least one uppercase letter
- At least one number

Please add these additional password requirements.
```

### 5. Loop Control

Based on the decision:

**DONE**: Session ends successfully
- SessionEnd written with outcome="success"
- Summary and confidence recorded
- Exit code 0

**CONTINUE**: Actor runs again
- Feedback passed to actor
- New iteration begins
- Same prompt + feedback

**ERROR**: Recovery attempted
- Recovery suggestion passed to actor
- New iteration begins
- If repeated errors, may fail session

## Termination Conditions

The loop ends when:

1. **Success**: Critic returns DONE
2. **Max iterations**: Configured limit reached (exit code 1)
3. **Error**: Unrecoverable error occurs (exit code 2)
4. **Interrupt**: User presses Ctrl+C (exit code 130)

## Confidence Scoring

When the critic returns DONE, it provides a confidence score:

| Score | Meaning |
|-------|---------|
| 0.9 - 1.0 | High confidence, all requirements clearly met |
| 0.7 - 0.9 | Good confidence, requirements met with minor uncertainty |
| 0.5 - 0.7 | Moderate confidence, some requirements unclear |
| < 0.5 | Low confidence, task may be incomplete |

The score is recorded but doesn't affect loop behavior. It's informational for users reviewing sessions.

## Feedback Quality

Good critic feedback:
- Specific about what's missing or wrong
- References the original requirements
- Provides actionable guidance
- Prioritizes issues by importance

Example of good feedback:
```
FEEDBACK: The validation is partially implemented:

DONE:
- Email format validation using regex

MISSING:
1. Password minimum length check (required: 8 characters)
2. Password uppercase letter requirement
3. Password digit requirement

Please implement the missing password validations and return
appropriate error messages for each case.
```

## Actor Recovery

When the actor fails (non-zero exit code), the critic provides recovery guidance:

```
DECISION: ERROR

ANALYSIS: The actor encountered a compilation error:
  error[E0599]: no method named `validate_email` found

RECOVERY: The `validate_email` method doesn't exist. You need to
either:
1. Import it from the `validators` crate, or
2. Implement it in src/utils/validation.rs

Check the project's existing validation patterns in src/utils/.
```

The actor then receives this recovery suggestion and attempts to fix the issue.

## Iteration Limits

Without a limit, loops could run indefinitely. Set limits with:

```bash
codeloops --max-iterations 5
```

Or in configuration:
```toml
max_iterations = 5
```

When the limit is reached:
- Outcome is "max_iterations_reached"
- Exit code is 1
- Session is complete but task may be unfinished

## Best Practices

### For Prompts

Clear prompts lead to accurate critic evaluation:
- Include acceptance criteria
- Be specific about requirements
- Define what "done" looks like

### For Iteration Limits

Choose limits based on task complexity:
- Simple fixes: 2-3 iterations
- Medium features: 5-10 iterations
- Complex tasks: Consider breaking into smaller prompts

### For Agent Selection

Consider critic thoroughness:
- More thorough critic = better feedback but slower
- Faster critic = quicker iterations but may miss issues

## Implementation Details

The loop is implemented in `codeloops-core/src/loop_runner.rs`:

```rust
pub async fn run(&self, context: LoopContext) -> Result<LoopOutcome, LoopError> {
    loop {
        // Run actor
        let actor_output = self.actor.execute(&context.build_prompt()).await?;

        // Capture diff
        let diff = self.diff_capture.capture()?;

        // Run critic
        let critic_output = self.critic.evaluate(&actor_output, &diff).await?;

        // Parse decision
        match critic_output.decision {
            Decision::Done { summary, confidence } => {
                return Ok(LoopOutcome::Success { ... });
            }
            Decision::Continue { feedback } => {
                context.set_feedback(feedback);
                continue;
            }
            Decision::Error { recovery } => {
                context.set_feedback(recovery);
                continue;
            }
        }
    }
}
```
