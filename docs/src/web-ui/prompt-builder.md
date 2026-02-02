# Prompt Builder

The Prompt Builder is a conversational AI-powered interface for creating high-quality `prompt.md` files. It guides you through an interview process tailored to your work type, helping you capture all the necessary context for effective coding sessions.

## Accessing the Prompt Builder

Start the web UI and navigate to the Prompt Builder:

```bash
codeloops ui
```

Then click "Prompt Builder" in the navigation header, or go directly to `/prompt-builder`.

## Work Types

The Prompt Builder supports five work types, each with a tailored interview flow:

### Feature

For new functionality. The AI guides you through:
1. **Problem Definition** - Who is this for? What problem does it solve?
2. **Technical Approach** - What components need to change? What patterns to follow?
3. **Implementation Details** - Specific changes per component, edge cases
4. **Acceptance Criteria** - Definition of done, verification steps

### Defect

For bug fixes. The AI helps capture:
1. **Symptom** - What's happening vs. what should happen? Reproduction steps
2. **Root Cause** - Where in the code? Why is it happening?
3. **Fix Strategy** - How to address it? Which files?
4. **Verification** - Tests to add, regression prevention

### Risk

For security vulnerabilities, performance issues, or technical concerns:
1. **Risk Identification** - What risk? How discovered? Impact?
2. **Current State** - Where does it exist? Current mitigations?
3. **Remediation Plan** - How to address? Files to change?
4. **Validation** - How to verify the fix?

### Debt

For technical debt cleanup:
1. **Current State** - What needs improvement? Why is it problematic?
2. **Target State** - What should it look like? Patterns to follow?
3. **Refactoring Plan** - Files to touch, safe order of operations
4. **Verification** - Tests that must pass, behavior to preserve

### Custom

For anything that doesn't fit the above categories. The AI asks general questions about your goal and helps structure the prompt appropriately.

## Using the Prompt Builder

### 1. Select Work Type

Choose a work type by clicking one of the text links: **Feature**, **Defect**, **Risk**, **Debt**, or **Custom**.

### 2. Have a Conversation

The AI asks probing questions based on your work type. Answer naturally - the AI builds your prompt incrementally based on your responses.

**Tips:**
- Be specific about context (files, functions, behaviors)
- Include reproduction steps for defects
- Mention constraints or non-goals
- Share relevant code snippets if helpful

### 3. Preview Your Prompt

Press `Cmd+P` (Mac) or `Ctrl+P` (Windows/Linux) to toggle the preview panel. The preview shows your `prompt.md` building in real-time.

You can also:
- **Edit** the preview directly for fine-tuning
- Continue chatting to add more context
- The preview updates automatically

### 4. Export

When you're satisfied with your prompt:

| Action | Description |
|--------|-------------|
| **Save** | Writes `prompt.md` to your project directory |
| **Copy** | Copies to clipboard |
| **Download** | Downloads as a file |

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd/Ctrl + Enter` | Send message |
| `Cmd/Ctrl + P` | Toggle preview panel |
| `Escape` | Close preview panel |

## Session Persistence

Your session is automatically saved to browser local storage. If you:
- Refresh the page
- Close and reopen the browser
- Navigate away and return

...your conversation will be restored.

To start fresh, click "Start over" to clear the session.

## Best Practices

### Be Specific

Instead of:
> "Fix the login bug"

Try:
> "Users clicking 'Login' on the /auth page get a 500 error. Expected: redirect to /dashboard. The error appears in the console as 'TypeError: user.id is undefined' in auth.service.ts line 42."

### Provide Context

Mention:
- Relevant files or functions
- Related features or systems
- Previous attempts and why they failed
- Constraints (performance, backwards compatibility, etc.)

### Define Success

Be clear about:
- What "done" looks like
- How to verify the change works
- Tests that should pass
- Edge cases to handle

## Example Session

**Work Type:** Feature

**AI:** Let's design this feature together. Who is this feature for and what problem does it solve?

**You:** Our API team needs rate limiting on the /api/v2 endpoints. Currently any client can hammer the API and there's no protection. They should be limited to 100 requests per minute per API key.

**AI:** Good context. What components will need to change? Are there existing patterns to follow?

**You:** We already have a RateLimiter class in src/middleware/rate-limiter.ts that's used for auth endpoints. We should extend that. The v2 routes are in src/routes/v2/*.ts.

...and so on. The resulting `prompt.md` captures all this context in a structured format ready for a coding session.
