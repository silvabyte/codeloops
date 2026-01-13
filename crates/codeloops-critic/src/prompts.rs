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
            r#"You are a code review critic. Your job is to evaluate whether a coding task has been completed successfully.

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

## Your Evaluation Criteria
1. **Completeness**: Has the task been fully addressed? Are all requirements met?
2. **Correctness**: Do the changes appear correct? Are there obvious bugs or errors?
3. **Quality**: Is the implementation reasonable? No obvious anti-patterns?
4. **Errors**: Were there any errors in the actor's execution that need addressing?

## Required Response Format

After your analysis, you MUST end your response with a decision block in exactly this format:

If the task is COMPLETE:
<decision>
{{"type": "done", "summary": "Brief summary of what was accomplished", "confidence": 0.95}}
</decision>

If MORE WORK is needed:
<decision>
{{"type": "continue", "feedback": "Specific, actionable feedback for the next iteration", "remaining_issues": ["issue1", "issue2"]}}
</decision>

If an ERROR was encountered:
<decision>
{{"type": "error", "error_description": "What went wrong", "recovery_suggestion": "How to fix it"}}
</decision>

Be thorough but fair. Only mark as "done" if the task is genuinely complete.
Do not be overly pedantic about style issues unless they affect functionality.
Focus on whether the core task requirements have been met."#,
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
