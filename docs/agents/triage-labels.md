# Triage labels

| Skill role | Beads label | Meaning |
| --- | --- | --- |
| needs-triage | needs-triage | Maintainer needs to evaluate |
| needs-info | needs-info | Waiting on reporter |
| ready-for-agent | ready-for-agent | Fully specified for autonomous implementation |
| ready-for-human | ready-for-human | Requires human implementation |
| wontfix | wontfix | Will not be actioned |

Use these labels separately from Beads status and dependencies.
Labels are applied on demand; no separate registration is required.

Keep at most one triage-role label per issue. When transitioning, remove
the previous role and add the next, preserving unrelated labels:

`bd update ISSUE_ID --remove-label needs-triage --add-label ready-for-agent --json`

Find triage candidates with `bd list --status open --label needs-triage --json`.
Find unblocked agent-ready work with `bd ready --label ready-for-agent --json`.
An unblocked issue is not necessarily fully specified.
