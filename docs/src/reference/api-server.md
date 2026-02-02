# API Server Reference

This document provides a complete reference for the codeloops REST API.

## Overview

The API server provides HTTP endpoints for accessing session data. It's started with `codeloops ui` and runs alongside the web UI.

**Base URL**: `http://localhost:3100` (default)

**Content-Type**: `application/json` (unless noted)

## Starting the Server

```bash
# Start with defaults
codeloops ui

# Custom API port
codeloops ui --api-port 4000

# Development mode
codeloops ui --dev
```

## Endpoints

### List Sessions

List all sessions with optional filtering.

**Request**

```
GET /api/sessions
```

**Query Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `outcome` | string | Filter by outcome: `success`, `failed`, `interrupted`, `max_iterations_reached` |
| `after` | string | Sessions after date (YYYY-MM-DD) |
| `before` | string | Sessions before date (YYYY-MM-DD) |
| `search` | string | Search in prompt text |
| `project` | string | Filter by project name |

**Response**

```json
[
  {
    "id": "2025-01-27T15-30-45Z_a3f2c1",
    "timestamp": "2025-01-27T15:30:45Z",
    "prompt_preview": "Add input validation to the user registration...",
    "working_dir": "/home/user/projects/myapp",
    "project": "myapp",
    "outcome": "success",
    "iterations": 2,
    "duration_secs": 89.4,
    "confidence": 0.95,
    "actor_agent": "Claude Code",
    "critic_agent": "Claude Code"
  }
]
```

**Response Fields**

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Session identifier |
| `timestamp` | string | ISO 8601 start time |
| `prompt_preview` | string | First 256 chars of prompt |
| `working_dir` | string | Absolute path |
| `project` | string | Basename of working_dir |
| `outcome` | string/null | Outcome or null if active |
| `iterations` | integer | Number of iterations |
| `duration_secs` | float/null | Total duration or null if active |
| `confidence` | float/null | Confidence score (0-1) |
| `actor_agent` | string | Actor agent name |
| `critic_agent` | string | Critic agent name |

**Example**

```bash
# List all sessions
curl http://localhost:3100/api/sessions

# Filter by outcome
curl "http://localhost:3100/api/sessions?outcome=success"

# Filter by date range
curl "http://localhost:3100/api/sessions?after=2025-01-01&before=2025-01-31"

# Search prompts
curl "http://localhost:3100/api/sessions?search=authentication"

# Combine filters
curl "http://localhost:3100/api/sessions?outcome=success&project=myapp"
```

### Get Session

Get detailed information for a single session.

**Request**

```
GET /api/sessions/{id}
```

**Path Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string | Session identifier |

**Response**

```json
{
  "id": "2025-01-27T15-30-45Z_a3f2c1",
  "start": {
    "timestamp": "2025-01-27T15:30:45Z",
    "prompt": "Add input validation to the user registration endpoint...",
    "working_dir": "/home/user/projects/myapp",
    "actor_agent": "Claude Code",
    "critic_agent": "Claude Code",
    "actor_model": "sonnet",
    "critic_model": null,
    "max_iterations": 10
  },
  "iterations": [
    {
      "iteration_number": 1,
      "actor_output": "I've added email validation...",
      "actor_stderr": "",
      "actor_exit_code": 0,
      "actor_duration_secs": 45.2,
      "git_diff": "diff --git a/src/api/users.rs...",
      "git_files_changed": 1,
      "critic_decision": "CONTINUE",
      "feedback": "Email validation looks good, but...",
      "timestamp": "2025-01-27T15:31:30Z"
    },
    {
      "iteration_number": 2,
      "actor_output": "I've added password validation...",
      "actor_stderr": "",
      "actor_exit_code": 0,
      "actor_duration_secs": 32.1,
      "git_diff": "diff --git a/src/api/users.rs...",
      "git_files_changed": 1,
      "critic_decision": "DONE",
      "feedback": null,
      "timestamp": "2025-01-27T15:32:02Z"
    }
  ],
  "end": {
    "outcome": "success",
    "iterations": 2,
    "summary": "Input validation has been added...",
    "confidence": 0.95,
    "duration_secs": 89.4,
    "timestamp": "2025-01-27T15:32:14Z"
  }
}
```

**Example**

```bash
curl http://localhost:3100/api/sessions/2025-01-27T15-30-45Z_a3f2c1
```

### Get Session Diff

Get the cumulative git diff for a session.

**Request**

```
GET /api/sessions/{id}/diff
```

**Path Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string | Session identifier |

**Response**

Content-Type: `text/plain`

```diff
diff --git a/src/api/users.rs b/src/api/users.rs
index 1234567..abcdefg 100644
--- a/src/api/users.rs
+++ b/src/api/users.rs
@@ -10,6 +10,18 @@ pub async fn register(data: Json<RegisterRequest>) {
+    // Validate email
+    if !is_valid_email(&data.email) {
+        return Err(ApiError::BadRequest("Invalid email format"));
+    }
+
+    // Validate password
+    if data.password.len() < 8 {
+        return Err(ApiError::BadRequest("Password must be at least 8 characters"));
+    }
```

**Example**

```bash
curl http://localhost:3100/api/sessions/2025-01-27T15-30-45Z_a3f2c1/diff
```

### Get Statistics

Get aggregate statistics across all sessions.

**Request**

```
GET /api/stats
```

**Response**

```json
{
  "total_sessions": 47,
  "success_rate": 0.787,
  "avg_iterations": 2.3,
  "avg_duration_secs": 94.2,
  "sessions_over_time": [
    { "date": "2025-01-27", "count": 5 },
    { "date": "2025-01-26", "count": 8 },
    { "date": "2025-01-25", "count": 12 }
  ],
  "by_project": [
    {
      "project": "myapp",
      "total": 23,
      "success_rate": 0.826
    },
    {
      "project": "api-svc",
      "total": 15,
      "success_rate": 0.733
    }
  ]
}
```

**Response Fields**

| Field | Type | Description |
|-------|------|-------------|
| `total_sessions` | integer | Total number of sessions |
| `success_rate` | float | Success rate (0.0-1.0) |
| `avg_iterations` | float | Average iterations per session |
| `avg_duration_secs` | float | Average session duration |
| `sessions_over_time` | array | Sessions grouped by date |
| `by_project` | array | Statistics per project |

**Example**

```bash
curl http://localhost:3100/api/stats
```

### Live Session Events (SSE)

Stream real-time session events using Server-Sent Events.

**Request**

```
GET /api/sessions/live
```

**Response**

Content-Type: `text/event-stream`

Events are sent as they occur:

```
event: session_created
data: {"id":"2025-01-27T16-00-00Z_b5d3e2","timestamp":"2025-01-27T16:00:00Z","prompt_preview":"Fix the bug...","actor_agent":"Claude Code","critic_agent":"Claude Code"}

event: session_updated
data: {"id":"2025-01-27T16-00-00Z_b5d3e2","iteration":1,"critic_decision":"CONTINUE"}

event: session_completed
data: {"id":"2025-01-27T16-00-00Z_b5d3e2","outcome":"success","iterations":2}
```

**Event Types**

| Event | Description |
|-------|-------------|
| `session_created` | New session started |
| `session_updated` | Iteration completed |
| `session_completed` | Session finished |

**Example (JavaScript)**

```javascript
const eventSource = new EventSource('http://localhost:3100/api/sessions/live');

eventSource.addEventListener('session_created', (e) => {
  const data = JSON.parse(e.data);
  console.log('New session:', data.id);
});

eventSource.addEventListener('session_updated', (e) => {
  const data = JSON.parse(e.data);
  console.log('Session updated:', data.id, 'iteration:', data.iteration);
});

eventSource.addEventListener('session_completed', (e) => {
  const data = JSON.parse(e.data);
  console.log('Session completed:', data.id, 'outcome:', data.outcome);
});
```

**Example (curl)**

```bash
curl -N http://localhost:3100/api/sessions/live
```

---

## Prompt Builder Endpoints

The Prompt Builder feature provides endpoints for creating `prompt.md` files through an AI-guided interview process.

### Get Context

Get the current working directory context for the UI.

**Request**

```
GET /api/context
```

**Response**

```json
{
  "workingDir": "/home/user/projects/myapp",
  "projectName": "myapp"
}
```

**Response Fields**

| Field | Type | Description |
|-------|------|-------------|
| `workingDir` | string | Absolute path to working directory |
| `projectName` | string | Basename of working directory |

**Example**

```bash
curl http://localhost:3100/api/context
```

### Create Prompt Session

Start a new prompt building session.

**Request**

```
POST /api/prompt-session
```

**Request Body**

```json
{
  "workType": "feature",
  "workingDir": "/home/user/projects/myapp"
}
```

**Request Fields**

| Field | Type | Description |
|-------|------|-------------|
| `workType` | string | Work type: `feature`, `defect`, `risk`, `debt`, or `custom` |
| `workingDir` | string | Absolute path to working directory |

**Response**

```json
{
  "sessionId": "prompt-abc123-def456"
}
```

**Example**

```bash
curl -X POST http://localhost:3100/api/prompt-session \
  -H "Content-Type: application/json" \
  -d '{"workType": "feature", "workingDir": "/home/user/projects/myapp"}'
```

### Send Message

Send a message to the prompt session and receive AI response via SSE.

**Request**

```
POST /api/prompt-session/{sessionId}/message
```

**Path Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `sessionId` | string | Session identifier from create session |

**Request Body**

```json
{
  "content": "I want to add user authentication"
}
```

**Response**

Content-Type: `text/event-stream`

```
data: {"content":"Let's"}
data: {"content":" design"}
data: {"content":" this"}
data: {"content":" feature."}
data: {"promptDraft":"# Feature: User Authentication\n\n## Problem\n..."}
data: [DONE]
```

**SSE Events**

| Event Data | Description |
|------------|-------------|
| `{"content": "..."}` | Streaming text chunk from AI |
| `{"promptDraft": "..."}` | Updated prompt.md draft |
| `[DONE]` | End of stream marker |

**Special Messages**

Send `__INIT__` as the content to get the initial AI greeting for the selected work type.

**Example**

```bash
# Get initial greeting
curl -X POST "http://localhost:3100/api/prompt-session/prompt-abc123/message" \
  -H "Content-Type: application/json" \
  -d '{"content": "__INIT__"}'

# Send user message
curl -X POST "http://localhost:3100/api/prompt-session/prompt-abc123/message" \
  -H "Content-Type: application/json" \
  -d '{"content": "I want to add input validation for the API"}'
```

### Save Prompt

Save the prompt.md content to disk.

**Request**

```
POST /api/prompt/save
```

**Request Body**

```json
{
  "workingDir": "/home/user/projects/myapp",
  "content": "# Feature: Input Validation\n\n## Problem\n..."
}
```

**Request Fields**

| Field | Type | Description |
|-------|------|-------------|
| `workingDir` | string | Directory to save prompt.md |
| `content` | string | Content to write to prompt.md |

**Response**

```json
{
  "path": "/home/user/projects/myapp/prompt.md"
}
```

**Error Responses**

| Status | Description |
|--------|-------------|
| 500 | Write failed (permission denied, disk full, etc.) |

**Example**

```bash
curl -X POST http://localhost:3100/api/prompt/save \
  -H "Content-Type: application/json" \
  -d '{"workingDir": "/home/user/projects/myapp", "content": "# My Prompt\n\nContent here..."}'
```

---

## Error Responses

### 404 Not Found

```json
{
  "error": "Session not found",
  "id": "invalid-session-id"
}
```

### 400 Bad Request

```json
{
  "error": "Invalid filter parameter",
  "details": "Invalid date format for 'after' parameter"
}
```

### 500 Internal Server Error

```json
{
  "error": "Internal server error",
  "details": "Failed to read session file"
}
```

## CORS

The API server allows cross-origin requests from localhost origins by default, enabling the separate UI dev server to make requests.

Headers included:
- `Access-Control-Allow-Origin: *` (in dev mode)
- `Access-Control-Allow-Methods: GET, OPTIONS`
- `Access-Control-Allow-Headers: Content-Type`

## Health Check

Check if the API server is running:

```bash
curl http://localhost:3100/api/sessions
```

A successful response (even an empty array `[]`) indicates the server is healthy.

## Rate Limiting

There is no built-in rate limiting. The API is designed for local use only.

## Authentication

There is no authentication. The API is intended for local development use.

## Programmatic Usage

### Python

```python
import requests

BASE_URL = "http://localhost:3100"

# List sessions
sessions = requests.get(f"{BASE_URL}/api/sessions").json()

# Get session detail
session = requests.get(f"{BASE_URL}/api/sessions/{sessions[0]['id']}").json()

# Get statistics
stats = requests.get(f"{BASE_URL}/api/stats").json()
```

### JavaScript/TypeScript

```typescript
const BASE_URL = "http://localhost:3100";

// List sessions
const sessions = await fetch(`${BASE_URL}/api/sessions`).then(r => r.json());

// Get session detail
const session = await fetch(`${BASE_URL}/api/sessions/${sessions[0].id}`).then(r => r.json());

// Get statistics
const stats = await fetch(`${BASE_URL}/api/stats`).then(r => r.json());
```

### curl

```bash
# List sessions with jq formatting
curl -s http://localhost:3100/api/sessions | jq

# Get specific session
curl -s http://localhost:3100/api/sessions/2025-01-27T15-30-45Z_a3f2c1 | jq

# Get diff
curl http://localhost:3100/api/sessions/2025-01-27T15-30-45Z_a3f2c1/diff
```
