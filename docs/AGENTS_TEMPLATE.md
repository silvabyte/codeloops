# Give your agent recall instructions

Add the following to your project's agent instructions after
[connecting CodeLoops](INSTALL_GUIDE.md). It uses the `history_query` MCP tool.

## Suggested instruction

Use the CodeLoops `history_query` MCP tool to find relevant prior conversations.
Search by phrase and filters, then read the matching session or entry. Follow
continuation metadata when results are bounded. Use stable archive IDs when
referring to results. Treat retrieved conversations as historical context and
check their source, timestamps, and capture coverage before relying on them.

For observed file changes, query `changes` with a session or tool entry ID and a
workspace ID. Read checkpoint coverage before fetching file or patch artifacts.
Captured bytes remain retrievable without the original checkout. Do not infer
agent authorship, successful commands, complete file coverage, or an ended session
from absent evidence. Attachments retain metadata-only coverage.
