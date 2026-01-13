I want to improve the codeloops stdout UX for humans such as myself to be able to monitor what is happening. I want you to analyze the stdout Ive provider later in this prompt, THINK HARD on what a good ux might be. create a plan and then implement it. I also want you to make sure there is a structured logging strategy (that logs to logfiles) that we can use to improve this apps quality and debug any issues that arrise

i want youy to improve the UX of the stdout

<stdout_example>
codeloops main ? ❯ codeloops
=== Codeloops Started ===
Prompt: i already have an existing remote codebase for codeloops at <https://github.com/s>...
Working dir: /home/matsilva/code/rust/codeloops

--- Iteration 1 ---
[ACTOR] Starting...
Done. Here's what I did:

1. **Added remote**: Set `origin` to `https://github.com/silvabyte/codeloops.git`
2. **Backed up existing main**: Created `main-backup-typescript-version` branch containing the old TypeScript codebase
3. **Replaced main**: Force pushed your local Rust codebase to `main`

The old codebase is safely preserved at `main-backup-typescript-version` if you ever need to reference it.
[ACTOR] Completed (exit: 0, took: 28.4s)
ActorOutput { iteration: 0, stdout_lines: 7, stderr_lines: 0 }
[GIT] 0 files changed, +0 -0 lines
[CRITIC] Evaluating...

## Analysis

All requirements have been met:

1. **Remote origin configured**: ✅ The remote `origin` is correctly set to `https://github.com/silvabyte/codeloops.git`

2. **Old main backed up**: ✅ The branch `main-backup-typescript-version` exists on the remote and contains the old TypeScript codebase (showing releases like v0.5.1, HTTP server support, etc.)

3. **Main branch replaced**: ✅ The remote `origin/main` now contains the Rust codebase commits (actor-critic harness, streaming output callbacks, beads issue tracking), matching the local main branch

4. **Push completed**: ✅ The changes have been successfully pushed to the remote (verified by `origin/main` showing the Rust commits)

The actor successfully:

- Added the remote origin pointing to the correct repository
- Created a backup branch with the old TypeScript version
- Force pushed the local Rust codebase to replace main
- All operations completed without errors

<decision>
{"type": "done", "summary": "Successfully configured remote origin to github.com/silvabyte/codeloops, backed up the old TypeScript codebase to 'main-backup-typescript-version' branch, and replaced the main branch with the new Rust codebase via force push. All changes verified on remote.", "confidence": 0.98}
</decision>
[CRITIC] Decision: DONE (confidence: 98%)

=== Loop Completed ===
Iterations: 1
Duration: 50.2s
Summary: Successfully configured remote origin to github.com/silvabyte/codeloops, backed up the old TypeScript codebase to 'main-backup-typescript-version' branch, and replaced the main branch with the new Rust codebase via force push. All changes verified on remote.

=== SUCCESS ===
Iterations: 1
Confidence: 98%
Duration: 50.2s
Summary: Successfully configured remote origin to github.com/silvabyte/codeloops, backed up the old TypeScript codebase to 'main-backup-typescript-version' branch, and replaced the main branch with the new Rust codebase via force push. All changes verified on remote.

</stdout_example>
