# CodeLoops MCP Server Testing Instructions

## Overview
Test the CodeLoops MCP server after removing the summarization agent to ensure the actor-critic workflow functions correctly without the summarization codepath.

## Setup

1. **Start the MCP server**:
   ```bash
   npm run dev
   ```

2. **Connect via Claude Code** (or another MCP client):
   - Ensure the codeloops MCP server is configured in your MCP settings
   - The server should be available at the configured transport

## Test Scenarios

### 1. Basic Actor-Critic Workflow

Test that the core actor-critic loop works without summarization errors.

**Test Command**:
```typescript
// Call actor_think with a simple task
actor_think({
  thought: "Test basic actor-critic workflow after removing summarization agent",
  projectContext: "/Users/akamat/code/nodecode/codeloops",
  tags: ["test", "workflow-verification"],
  artifacts: []
})
```

**Expected Result**:
- Should return both actor and critic nodes
- No "Summarization failed" errors
- Critic should provide feedback on the actor's thought

### 2. File Modification Task

Test a typical file modification scenario that would have previously triggered summarization.

**Test Command**:
```typescript
actor_think({
  thought: "Update README.md to reflect removal of summarization feature",
  projectContext: "/Users/akamat/code/nodecode/codeloops", 
  tags: ["file-modification", "documentation"],
  artifacts: [
    {
      name: "README.md",
      path: "README.md"
    }
  ]
})
```

**Expected Result**:
- Actor and critic nodes created successfully
- No summarization errors
- Normal actor-critic feedback loop

### 3. Multiple Sequential Thoughts

Test that multiple actor_think calls work correctly without summarization interference.

**Test Commands** (run in sequence):
```typescript
// First thought
actor_think({
  thought: "Planning to refactor config handling",
  projectContext: "/Users/akamat/code/nodecode/codeloops",
  tags: ["planning", "refactor"],
  artifacts: []
})

// Second thought  
actor_think({
  thought: "Implementing config validation improvements",
  projectContext: "/Users/akamat/code/nodecode/codeloops",
  tags: ["implementation", "config"],
  artifacts: [
    {
      name: "config/index.ts",
      path: "src/config/index.ts"
    }
  ]
})

// Third thought
actor_think({
  thought: "Testing config changes work correctly",
  projectContext: "/Users/akamat/code/nodecode/codeloops",
  tags: ["testing", "config"],
  artifacts: []
})
```

**Expected Result**:
- All three actor_think calls succeed
- Each returns actor and critic nodes
- No errors related to summarization
- Normal critic feedback for each thought

### 4. Resume Functionality

Test that the resume function works correctly without summarization data.

**Test Command**:
```typescript
resume({
  projectContext: "/Users/akamat/code/nodecode/codeloops",
  limit: 10
})
```

**Expected Result**:
- Returns recent nodes from the knowledge graph
- May include existing summary nodes (for backward compatibility)
- No errors when reading existing data

### 5. Delete Functionality

Test that node deletion still handles summary nodes correctly.

**Test Commands**:
```typescript
// First create a test node
actor_think({
  thought: "Test node for deletion testing",
  projectContext: "/Users/akamat/code/nodecode/codeloops",
  tags: ["test", "deletion"],
  artifacts: []
})

// Then delete it (get the node ID from the previous response)
delete_thoughts({
  nodeIds: ["<node-id-from-previous-response>"],
  projectContext: "/Users/akamat/code/nodecode/codeloops",
  reason: "Testing deletion after summarization removal"
})
```

**Expected Result**:
- Deletion succeeds
- Backup is created
- Affected summaries are identified (if any exist)
- No errors related to summarization

## Configuration Verification

### 6. Check Configuration

Verify that summarizer config has been properly removed.

**Test Command**:
```typescript
// Check current config doesn't have summarizer
export({
  projectContext: "/Users/akamat/code/nodecode/codeloops",
  limit: 5
})
```

Check the server logs to ensure no summarizer initialization errors.

## Success Criteria

✅ **All tests pass without errors**
✅ **No "Summarization failed" or "Unknown error" messages**  
✅ **Actor-critic workflow completes normally**
✅ **Existing summary nodes can still be read from the knowledge graph**
✅ **Delete operations handle summary nodes correctly**
✅ **Server starts without summarizer-related errors**

## Troubleshooting

If any tests fail:

1. **Check server logs** for detailed error messages
2. **Verify config** doesn't reference summarizer settings
3. **Confirm** all summarization imports have been removed
4. **Test** with a fresh project context if needed

## Additional Notes

- The SummaryNode interface is kept for backward compatibility
- Existing summary nodes in the knowledge graph should still be readable
- The deletion logic continues to handle affected summary nodes
- No new summary nodes will be created going forward