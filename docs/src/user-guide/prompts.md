# Writing Prompts

The quality of your prompt directly affects how well codeloops completes your task. This guide covers how to write effective prompts.

## Prompt Basics

### Using prompt.md

The default way to provide a prompt is via a `prompt.md` file in your working directory:

```bash
# Create prompt.md
echo "Fix the authentication bug" > prompt.md

# Run codeloops (reads prompt.md automatically)
codeloops
```

### Using --prompt Flag

For quick tasks, pass the prompt inline:

```bash
codeloops --prompt "Add a --verbose flag to the CLI"
```

### Using --prompt-file

Point to a different file:

```bash
codeloops --prompt-file tasks/feature-x.md
```

## Anatomy of a Good Prompt

Effective prompts have four components:

### 1. Clear Objective

State exactly what you want done:

```markdown
Add input validation to the user registration endpoint.
```

Not:
```markdown
Improve the user registration.
```

### 2. Context

Provide relevant information about your codebase:

```markdown
The registration endpoint is in src/api/users.rs, specifically the
`register` function that handles POST /api/users/register.

The User struct is defined in src/models/user.rs.
```

### 3. Requirements

List specific requirements:

```markdown
Requirements:
- Email must be a valid email format (use the `validator` crate)
- Password must be at least 8 characters
- Password must contain at least one uppercase letter
- Password must contain at least one number
```

### 4. Acceptance Criteria

Define what "done" looks like:

```markdown
The task is complete when:
- Invalid emails return a 400 error with message "Invalid email format"
- Short passwords return a 400 error with message "Password must be at least 8 characters"
- Weak passwords return a 400 error with specific feedback
- Valid inputs proceed to registration
```

## Prompt Templates

### Bug Fix

```markdown
## Bug Description
[Describe the bug and how to reproduce it]

## Expected Behavior
[What should happen instead]

## Location
[File path and function name if known]

## Additional Context
[Any relevant information]
```

Example:

```markdown
## Bug Description
The login endpoint returns 500 when the email contains a plus sign
(e.g., user+tag@example.com).

## Expected Behavior
Plus signs should be valid in email addresses per RFC 5321.

## Location
src/api/auth.rs, `login` function

## Additional Context
The validation regex is likely too strict.
```

### New Feature

```markdown
## Feature Description
[What the feature should do]

## Requirements
[List of specific requirements]

## Affected Files
[Known files that need changes, if any]

## Acceptance Criteria
[How to know when it's done]
```

Example:

```markdown
## Feature Description
Add a --dry-run flag to the deploy command that shows what would be
deployed without actually deploying.

## Requirements
- Add --dry-run boolean flag to DeployArgs struct
- When set, print deployment plan instead of executing
- Include file list, target environment, and estimated duration

## Affected Files
- src/commands/deploy.rs (add flag and logic)
- src/lib.rs (if shared types needed)

## Acceptance Criteria
- Running `app deploy --dry-run` shows the plan
- Running `app deploy` without the flag deploys normally
- Help text describes the new flag
```

### Refactoring

```markdown
## Goal
[What the refactoring achieves]

## Current State
[How the code works now]

## Desired State
[How the code should work after]

## Constraints
- [Any constraints to follow]
- [Behaviors to preserve]
```

Example:

```markdown
## Goal
Extract the database connection logic into a reusable module.

## Current State
Database connection code is duplicated across:
- src/api/users.rs
- src/api/posts.rs
- src/api/comments.rs

Each file creates its own connection pool.

## Desired State
- Single src/db/mod.rs module with connection pool
- Shared pool instance across all API modules
- Connection configuration from environment variables

## Constraints
- Must maintain backward compatibility
- No changes to API behavior
- Tests must continue to pass
```

### Adding Tests

```markdown
## Target Code
[What needs tests]

## Test Requirements
[What to test]

## Test Location
[Where tests should go]
```

Example:

```markdown
## Target Code
The `parse_config` function in src/config.rs

## Test Requirements
- Test valid TOML parsing
- Test missing required fields
- Test invalid field types
- Test default value handling
- Test environment variable override

## Test Location
Add tests to src/config.rs in a `#[cfg(test)]` module
```

## What to Avoid

### Vague Instructions

Bad:
```markdown
Make the code better.
```

Good:
```markdown
Refactor the `process_order` function to reduce cyclomatic complexity.
Extract the validation logic into a separate `validate_order` function.
```

### Multiple Unrelated Tasks

Bad:
```markdown
Fix the login bug and also add a new dashboard page and update the README.
```

Good:
```markdown
Fix the login bug where users with spaces in their password can't log in.
```

(Create separate prompts for the other tasks)

### Missing Context

Bad:
```markdown
Add caching.
```

Good:
```markdown
Add Redis caching to the `get_user` endpoint in src/api/users.rs.

Cache user data for 5 minutes using the user ID as the key.
The Redis connection string is in the REDIS_URL environment variable.
```

### Ambiguous "Done" Criteria

Bad:
```markdown
Improve performance.
```

Good:
```markdown
Optimize the `search_products` function to run in under 100ms for
queries matching up to 1000 products.

Currently it takes 2-3 seconds. Add an index on the `name` field
and use pagination with a limit of 50 results.
```

## Multi-Step Tasks

For complex tasks, break them into smaller prompts:

Instead of:
```markdown
Implement a complete user authentication system with registration,
login, logout, password reset, and email verification.
```

Do this sequence:
1. `prompt-1.md`: Implement user registration
2. `prompt-2.md`: Implement user login
3. `prompt-3.md`: Implement logout
4. `prompt-4.md`: Implement password reset
5. `prompt-5.md`: Implement email verification

Run each with:
```bash
codeloops --prompt-file prompt-1.md
codeloops --prompt-file prompt-2.md
# etc.
```

## How the Critic Uses Your Prompt

The critic evaluates the actor's work against your prompt. It checks:

1. Were the stated requirements met?
2. Does the implementation match the acceptance criteria?
3. Are there obvious issues or gaps?

A clear prompt helps the critic make accurate decisions. Vague prompts lead to the critic either:
- Approving incomplete work (didn't know what to check)
- Continuously requesting changes (unclear when "done")

## Generating Prompts with Agent Commands

You can configure your coding agent to generate `prompt.md` files automatically. This creates a powerful workflow:

1. Use your agent interactively to explore and plan
2. Run a command that writes the plan to `prompt.md`
3. Run codeloops with that prompt for structured execution

### Claude Code

Create a command file at `~/.claude/commands/promptmd.md`:

```markdown
---
description: Writes plan out to prompt.md file
---

## Instructions

1. Write out the plan to a prompt.md file in the cwd.
   a. If prompt.md file exists already, create a backup of it using a timestamp: prompt.timestamp.md
2. This prompt.md file will be used to implement the plan in another working session.
3. Ensure it contains detailed tasks to thoroughly implement the plan.
4. Also always ensure the plan includes adding documentation, tests that actually provide confidence and any other quality assurance checks available in the project/codebase.
5. Make sure the plan also specifies to commit the changes once finished.
```

Then use it in Claude Code:

```
> /promptmd
```

### OpenCode

Create a command file at `~/.config/opencode/command/promptmd.md` with the same content as above.

Then use it in OpenCode:

```
> /promptmd
```

### Workflow Example

```bash
# 1. Start your agent and explore the task
opencode
> Analyze the codebase and figure out how to add user authentication

# 2. Once you have a plan, generate the prompt
> /promptmd

# 3. Exit and run codeloops
codeloops
```

This workflow lets you leverage your agent's interactive exploration for planning, then use codeloops' actor-critic loop for disciplined execution.

## Tips for Success

1. **Be specific**: File paths, function names, exact requirements
2. **Define done**: What criteria must be met?
3. **Provide context**: Relevant code locations and relationships
4. **One task per prompt**: Complex tasks need multiple prompts
5. **Include constraints**: What must be preserved or avoided?
