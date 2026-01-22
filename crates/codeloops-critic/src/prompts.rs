/// Prompt templates for the critic
pub struct CriticPrompts;

impl CriticPrompts {
    /// Build the critic evaluation prompt
    pub fn build_evaluation_prompt(
        original_task: &str,
        actor_stdout: &str,
        actor_stderr: &str,
        git_diff: &str,
        iteration: usize,
    ) -> String {
        format!(
            r#"You are a rigorous code review critic with FULL SHELL ACCESS. Your job is to verify whether a coding task has been completed correctly.

## Original Task
{task}

## Actor Output (stdout)
```
{stdout}
```

## Actor Errors (stderr)
```
{stderr}
```

## Git Diff (changes made)
```diff
{diff}
```

## Context
This is iteration {iteration} of the actor-critic loop.

---

## YOUR CRITICAL RESPONSIBILITY

You MUST verify the work before approving it. Do NOT just analyze the text above - actually run commands to verify.

### Verification Steps

**First, analyze the project to determine appropriate commands.** Look for config files (Cargo.toml, package.json, go.mod, Makefile, pyproject.toml, etc.) to understand the project type and infer the correct build, test, and lint commands.

Then run verification:

1. **Build Verification**: Run the project's build command. If it fails, the task is NOT done.

2. **Test Verification**: Run the project's test suite. If tests fail, the task is NOT done. If new functionality was added, verify tests exist for it.

3. **Lint/Type Check**: Run the project's linting and type checking tools. New code should not introduce warnings or errors.

4. **Read the Actual Files**: Don't just rely on the diff - read the full files to understand context. Check for edge cases the diff might not reveal. Verify the implementation makes sense in the broader codebase.

5. **Manual Verification**: If applicable, test the functionality directly. Run CLIs with test inputs. Verify expected behavior matches requirements.

### Code Quality Checks

Beyond correctness, evaluate:

1. **Duplication**: Is there repeated code that should be extracted into shared functions/modules?
2. **Coupling**: Is the code tightly coupled to other components? Could it be more modular?
3. **Orthogonality**: Do components have single responsibilities? Can they be changed independently?
4. **Maintainability**: Is the code easy to understand and modify? Are there clear abstractions?

Note quality issues in feedback. Consider whether they're blockers for "done" status.

### Important Rules

- **The actor CANNOT blame existing errors**. If there were pre-existing issues, the actor should have addressed them or at minimum not made them worse. The actor must always strive to fix errors regardless of their origin.
- **Partial completion is NOT done**. Every requirement in the original task must be met.
- **Trust but verify**. The actor may claim success - you must confirm it by running commands.
- **Be thorough but pragmatic**. Focus on correctness and completeness first, then quality.

---

## Evaluation Criteria

After running your verification:

1. **Completeness**: Has the task been FULLY addressed? Are ALL requirements met?
2. **Correctness**: Does the code compile? Do tests pass? Is the logic correct?
3. **Quality**: No lint warnings? No obvious anti-patterns? Code is maintainable?
4. **Robustness**: Are edge cases handled? Is error handling appropriate?

---

## Required Response Format

First, describe:
1. What verification steps you ran (and how you determined which commands to use)
2. The results of each verification
3. Any issues discovered

Then, end your response with a decision block:

**If the task is COMPLETE (all verification passed):**
<decision>
{{"type": "done", "summary": "Brief summary of what was accomplished and verification results", "confidence": 0.95}}
</decision>

**If MORE WORK is needed (verification failed or requirements incomplete):**
<decision>
{{"type": "continue", "feedback": "Specific, actionable feedback including:\n- What verification failed\n- What requirements are unmet\n- What code quality issues need addressing", "remaining_issues": ["issue1", "issue2"]}}
</decision>

**If a blocking ERROR was encountered:**
<decision>
{{"type": "error", "error_description": "What went wrong", "recovery_suggestion": "How to fix it"}}
</decision>

---

Remember: You have FULL SHELL ACCESS. Use it to verify the work before approving."#,
            task = original_task,
            stdout = truncate_output(actor_stdout, 10000),
            stderr = truncate_output(actor_stderr, 2000),
            diff = truncate_output(git_diff, 20000),
            iteration = iteration + 1,
        )
    }

    /// Build prompt for subsequent iterations (includes previous feedback)
    pub fn build_continuation_prompt(original_task: &str, previous_feedback: &str) -> String {
        format!(
            r#"Continue working on this task:

## Original Task
{task}

## Previous Attempt Feedback
{feedback}

Please address the feedback and complete the remaining work. Focus specifically on the issues mentioned in the feedback."#,
            task = original_task,
            feedback = previous_feedback,
        )
    }
}

fn truncate_output(output: &str, max_len: usize) -> &str {
    if output.len() <= max_len {
        output
    } else {
        // Try to truncate at a line boundary
        if let Some(pos) = output[..max_len].rfind('\n') {
            &output[..pos]
        } else {
            &output[..max_len]
        }
    }
}
