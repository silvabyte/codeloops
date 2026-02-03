generally after 4-6 exchanges - arbitrary and WRONG

- when ai finally generates an initial prompt, the ui should automatically open the side panel to show it.
  right now it does not

type here is disabled when agent is respoding

the terminal ui could use use some tlc

- cl-nfb Work type button affordance (P1) - ✅ addressed

refactor codeloops-sessions to also use the codeloops-db sqlite database instead of ndjson

It will often ask me A, B, C questions:

- if its a 'pick one' question, it should provide a UI to make it easy to just quickly pick it
- if its a pick multiple, it should provide a ui to allow me to select more than one option
- it should also allow me to add custom input where applicable in the options
from user pov, the pain this solves is i have to retype the entire option it gives me...

- the prompt draft closes when i enter a new message
- there isnt any visual ux feedback in the conversation

- should be able to kick off a verification agent to determine if the work as properly done

tech debt:

- the api fetching appears to be sprawling in the ui
- shouldnt the rest api be its own crate?
- should merge all frontend code into a single folder and use bun monorepo pattern in root to manage them

bugs:
if i refresh during an agent response... the response never lands and we lose progress. this should finish even if i regresh the page

---------

ive had to manually prompt the agent to scrutinize the prompt implementation details for accuracy and quality.

- each time it had to make corrections

for example:
  File paths - Now reference actual locations (codeloops-db/src/lib.rs, codeloops-db/src/prompts.rs, etc.)
Serde convention - Rust uses snake_case with #[serde(rename_all = "camelCase")] for JSON
Type locations - Types go in ui/src/lib/prompt-session.ts, not a separate types file
Route syntax - Uses {id} pattern matching axum conventions
Added concrete algorithm - The resolve_chain implementation with visited set for deduplication
Specified all new endpoints - Both PUT for parents and GET for resolved content
Added index on parent_id - For efficient reverse lookups when deleting parents

another example:

Key findings:

The file is ui/src/hooks/usePromptSession.ts (not a page file - the hook contains the state machine)
The page component is ui/src/pages/PromptBuilder.tsx
Line 327: previewOpen: false - this is where the initial prompt generation transitions to ready state with previewOpen: false
Line 453: For follow-up messages, previewOpen is preserved from the previous state (correct behavior)
SessionStatePayload in ui/src/lib/prompt-session.ts:23-27 also uses previewOpen: boolean
StoredSession interface at line 150-156 also has previewOpen: boolean
The changes need to touch:

usePromptSession.ts: Main state machine, PromptSessionState, PromptSession, StoredSession, deriveSession, the transition at line 327
prompt-session.ts: SessionStatePayload type
PromptBuilder.tsx: How it derives previewOpen on line 93
I've updated the prompt with the correct file paths and accurate line numbers. The key changes:

Correct files: usePromptSession.ts (hook with state machine), prompt-session.ts (types), PromptBuilder.tsx (page component)
Specific locations: Line 327 for initial prompt → ready transition, line 453 for follow-up message preservation
Added localStorage migration: Since existing sessions store previewOpen: boolean, we need to handle the migration to the new enum
Updated all type locations: PromptSessionState, PromptSession, StoredSession, and SessionStatePayload

another example:
Critical corrections needed:

Two crates involved, not one:

codeloops-logging::SessionWriter - handles writing sessions (JSONL files)
codeloops-sessions::SessionStore - handles reading sessions
API naming was wrong: SessionStore, not SessionManager

Methods were wrong:

Reading: list(), get(), get_diff(), stats(), agentic_metrics(), active_sessions()
Writing: write_start(), write_iteration(), write_end()
Schema was incomplete: The Iteration struct has many fields the prompt glossed over

Parent prompt doesn't exist - need to either create it first or remove that reference

Here's the corrected prompt:

Key improvements:

Correctly identifies both crates that need modification
Accurate API documentation from actual code
Complete schema matching all fields in the types
Realistic implementation order (read path first, then write path)
Removed non-existent parent prompt reference
