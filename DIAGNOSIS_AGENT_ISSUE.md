# Diagnosis: Agent "Action Not Found" Issue

**Date**: 2026-02-26  
**Status**: Root cause identified, fix implemented  
**Issue**: Agent says "action not found" when trying to list Dropbox folder

---

## Root Cause Analysis

### What We Found

1. **Pattern 2 is working correctly**:
   - `COMPOSIO_SEARCH_TOOLS` successfully discovers `DROPBOX_LIST_FILES_IN_FOLDER`
   - Tool schemas are returned with full parameter definitions
   - Connection status correctly shows "not connected"

2. **The problem**: Two-step process confusion
   - Agent calls `composio_dynamic` with query: "list my Dropbox folder"
   - Tool discovers `DROPBOX_LIST_FILES_IN_FOLDER` but has no arguments
   - Tool returns error: "Please provide arguments"
   - Agent interprets this as "action not found" and gives up

3. **Why this happens**:
   - Original implementation required explicit `arguments` parameter
   - Agent doesn't know how to construct Dropbox-specific arguments
   - No automatic argument extraction from natural language

### Test Results

```bash
# COMPOSIO_SEARCH_TOOLS works perfectly
cargo run -- test-tool COMPOSIO_SEARCH_TOOLS '{
  "queries": [{"use_case": "list dropbox folder"}],
  "session": {"id": "trs_Ij9jR5rIS4_7"}
}'

# Returns:
# - primary_tool_slugs: ["DROPBOX_LIST_FILES_IN_FOLDER"]
# - has_active_connection: false
# - tool_schemas: {path: "", limit: 2000, recursive: false}
```

---

## Solution Implemented

### 1. Automatic Argument Extraction

Added `extract_arguments_from_query()` method that:
- Parses natural language queries
- Extracts common patterns (paths, emails, repo names)
- Uses schema defaults when available
- Falls back to sensible defaults

### 2. Improved Tool Description

Changed from:
```
"Dynamic tool discovery and execution for 1000+ apps via Composio"
```

To:
```
"Access 1000+ apps (Dropbox, Gmail, GitHub, Slack, etc.) through Composio. 
Simply describe what you want to do in natural language. 
Examples: 'list my Dropbox folder /Documents', 'send email via Gmail', etc.
The tool will automatically discover the right action and execute it."
```

### 3. Better Error Messages

When arguments can't be extracted:
```
Tool found: DROPBOX_LIST_FILES_IN_FOLDER
Description: Tool to list files and folders...

Required parameters:
{
  "path": {"type": "string", "default": ""},
  "limit": {"type": "integer", "default": 2000},
  ...
}

Tip: You can call this tool again with the 'arguments' parameter, 
or I can help you construct the arguments.
```

---

## How It Works Now

### User Query: "list my Dropbox folder"

1. **Agent calls**: `composio_dynamic(query="list my Dropbox folder")`

2. **Tool Router Session**: Create/reuse session `trs_Ij9jR5rIS4_7`

3. **Discovery**: `COMPOSIO_SEARCH_TOOLS` → finds `DROPBOX_LIST_FILES_IN_FOLDER`

4. **Argument Extraction**: 
   ```rust
   // Detects "list folder" pattern
   // Extracts path (none found, use root)
   // Returns: {"path": "", "recursive": false, "limit": 2000}
   ```

5. **Connection Check**: `COMPOSIO_MANAGE_CONNECTIONS` → not connected

6. **OAuth Flow**: Returns OAuth URL to user

7. **After OAuth**: User retries → tool executes successfully

---

## Argument Extraction Patterns

### Dropbox Operations

```
Query: "list my Dropbox folder /Documents"
Extracted: {"path": "/Documents", "recursive": false, "limit": 2000}

Query: "list my Dropbox folder"
Extracted: {"path": "", "recursive": false, "limit": 2000}
```

### Gmail Operations

```
Query: "send email to john@example.com subject 'Meeting'"
Extracted: {"to": "john@example.com", "subject": "Meeting"}
```

### GitHub Operations

```
Query: "create issue in my-repo"
Extracted: {"repo": "my-repo"}
```

### Fallback

If no pattern matches, uses schema defaults:
```json
{
  "path": "",
  "limit": 2000,
  "recursive": false
}
```

---

## Testing

### Build

```bash
cargo build --release
```

**Result**: ✅ Compiles successfully

### Install

```bash
cargo install --path .
```

### Test with Agent

```bash
zeroclaw agent

# In chat:
User: "list my Dropbox folder"

# Expected flow:
# 1. Agent calls composio_dynamic
# 2. Tool discovers DROPBOX_LIST_FILES_IN_FOLDER
# 3. Tool extracts arguments: {path: "", ...}
# 4. Tool checks connection → not connected
# 5. Tool returns OAuth URL
# 6. User authorizes
# 7. User retries → success
```

---

## Next Steps

### For User

1. **Rebuild and install**:
   ```bash
   cargo install --path .
   ```

2. **Restart agent**:
   ```bash
   zeroclaw channel start
   ```

3. **Test in Telegram**:
   ```
   "list my Dropbox folder"
   ```

4. **Expected behavior**:
   - Agent should use `composio_dynamic` tool
   - Tool should discover Dropbox action
   - Tool should extract arguments automatically
   - Tool should prompt for OAuth if needed
   - After OAuth, tool should execute successfully

### For Debugging

If still not working, check:

1. **Agent logs**: Look for "Pattern 2 detected" message
2. **Tool registration**: Ask agent "What tools do you have?"
3. **Tool execution**: Check for `composio_dynamic` in logs
4. **Argument extraction**: Check logs for "Extracted arguments from query"

---

## Files Modified

- `src/tools/composio_meta.rs`:
  - Added `extract_arguments_from_query()` method
  - Improved tool description
  - Better error messages with guidance
  - Automatic argument extraction for common patterns

---

## Summary

The issue was that the agent couldn't provide arguments in the format the tool expected. The fix adds automatic argument extraction from natural language, making the tool truly "dynamic" and user-friendly.

**Status**: ✅ Fixed and ready for testing
