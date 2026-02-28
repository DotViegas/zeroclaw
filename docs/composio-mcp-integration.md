# Composio MCP Integration Guide

Last verified: **February 26, 2026**.

## Overview

ZeroClaw integrates with Composio's Model Context Protocol (MCP) to provide seamless access to 1000+ OAuth-connected apps without managing raw API keys. This guide covers setup, usage patterns, and troubleshooting.

## Integration Patterns

ZeroClaw supports two Composio integration patterns:

### Pattern 1: Direct Tools (Legacy)

Uses individual tools per action (e.g., `GMAIL_SEND_EMAIL`, `GITHUB_CREATE_ISSUE`).

**When to use:**
- Simple, single-action workflows
- Direct control over specific actions
- Backward compatibility with existing configs

**Configuration:**
```toml
[composio]
enabled = true
api_key = "your_composio_api_key"
```

### Pattern 2: Natural Language (Recommended)

Uses `composio_nl` tool with meta-tools workflow for natural language queries.

**When to use:**
- Complex, multi-step workflows
- Natural language interaction
- Automatic tool discovery
- Simplified OAuth handling

**Configuration:**
```toml
[composio]
enabled = true
api_key = "your_composio_api_key"

[composio.mcp]
enabled = true
mcp_url = "https://backend.composio.dev/mcp?user_id=default"
user_id = "default"
```

**Key differences:**

| Feature | Direct Tools | Natural Language |
|---------|-------------|------------------|
| Tool invocation | Specific action names | Natural language query |
| Tool discovery | Manual lookup | Automatic via SEARCH |
| OAuth handling | Manual connection check | Automatic via MANAGE |
| Execution | Single tool call | Meta-tools workflow |
| Complexity | Higher (need exact names) | Lower (describe intent) |

## Features

- **Natural Language Access**: Use `composio_nl` tool with plain English queries
- **Meta-Tools Workflow**: Automatic tool discovery, connection management, and execution
- **Automatic MCP URL Generation**: No manual server setup required
- **Interactive Toolkit Selection**: Choose from popular apps during onboarding
- **Automatic OAuth Handling**: Browser auto-opens, polls for connection
- **Intelligent Retry**: Automatically retries tools after OAuth completion
- **Multi-Mode Support**: CLI, headless, and server modes
- **Session Management**: Efficient session reuse with 30-minute TTL

## Quick Start

### 1. Run the Onboarding Wizard

```bash
zeroclaw onboard
```

Select "Composio (managed OAuth)" when prompted for tool mode.

### 2. Configure MCP Integration

When asked "Enable Composio MCP integration?", select **Yes** (recommended).

### 3. Select Toolkits

Choose which apps you want to enable:
- ✅ Gmail - Email management
- ✅ GitHub - Repository management
- ✅ Slack - Team communication
- ✅ Dropbox - File storage
- ✅ Notion - Note-taking & docs
- ✅ Google Calendar - Scheduling

### 4. Connect a Toolkit (Optional)

The wizard will offer to connect a toolkit immediately:
- Browser opens OAuth authorization page
- Authorize the app
- Wizard confirms connection is ACTIVE
- You're ready to use the toolkit!

### 5. Start Using Tools

```bash
zeroclaw agent
```

#### Using Natural Language (Recommended)

In chat:
```
You: Send an email to user@example.com with subject "Hello"
```

Behind the scenes, `composio_nl` tool:
1. **SEARCH**: Discovers Gmail tools via `COMPOSIO_SEARCH_TOOLS`
2. **MANAGE**: Checks connection via `COMPOSIO_MANAGE_CONNECTIONS`
3. **EXECUTE**: Sends email via `COMPOSIO_MULTI_EXECUTE_TOOL`

If Gmail isn't connected yet, ZeroClaw will:
1. Detect OAuth is needed
2. Open browser with OAuth page
3. Wait for authorization
4. Retry the tool automatically
5. Send the email

#### Using Direct Tools (Legacy)

In chat:
```
You: Use GMAIL_SEND_EMAIL to send email to user@example.com
```

Requires knowing exact tool names and manual OAuth management.

## Configuration

### Config File Location

`~/.config/zeroclaw/config.toml` (Linux/macOS)  
`%APPDATA%\zeroclaw\config.toml` (Windows)

### MCP Configuration

```toml
[composio]
enabled = true
api_key = "your_composio_api_key"

[composio.mcp]
enabled = true
mcp_url = "https://backend.composio.dev/mcp?user_id=default"
# OR (Tool Router format - session-based)
# mcp_url = "https://backend.composio.dev/tool_router/trs_xxx/mcp?user_id=default"
user_id = "default"
tools_cache_ttl_secs = 600
```

**Important Notes:**

- **API Key**: Use your general Composio API key (starts with `ak_`), not MCP-specific
- **MCP URL Formats**: Two valid formats are supported:
  - **Direct MCP**: `https://backend.composio.dev/mcp?user_id=default`
  - **Tool Router** (session-based): `https://backend.composio.dev/tool_router/trs_xxx/mcp?user_id=default`
- **No toolkits parameter**: Do NOT include `toolkits` parameter for natural language mode
- **Toolkits**: Managed dynamically by meta-tools workflow

### MCP URL Formats Explained

**Direct MCP Format:**
```
https://backend.composio.dev/mcp?user_id=default
```
- Simpler, direct access to MCP endpoint
- Recommended for most use cases

**Tool Router Format (Session-based):**
```
https://backend.composio.dev/tool_router/trs_xxx/mcp?user_id=default
```
- Uses Tool Router session ID (`trs_xxx`)
- Generated by Composio's MCP session creation
- Provides session-based isolation
- May require session-specific API key

Both formats work with the `composio_nl` tool and meta-tools workflow.

### Configuration Options

| Field | Type | Description | Default |
|-------|------|-------------|---------|
| `enabled` | bool | Enable MCP integration | `false` |
| `mcp_url` | string | MCP endpoint URL (auto-filled by wizard) | `None` |
| `server_id` | string | Legacy: Manual server ID (deprecated) | `None` |
| `user_id` | string | MCP user identifier | `None` |
| `toolkits` | array | Legacy: Enabled toolkit slugs (not used in natural language mode) | `[]` |
| `tools_cache_ttl_secs` | int | Tool list cache duration | `600` |

## Environment Variables

### ZEROCLAW_UI_MODE

Controls onboarding user experience:

- `cli` (default for `zeroclaw agent`) - Interactive CLI with browser auto-open
- `server` (default for `zeroclaw gateway/daemon`) - Returns OAuth URL in error

**Example:**
```bash
export ZEROCLAW_UI_MODE=cli
zeroclaw agent
```

### ZEROCLAW_NO_BROWSER

Disables automatic browser opening in CLI mode:

- `1` or `true` - Print OAuth URL only
- Not set - Auto-open browser

**Example:**
```bash
export ZEROCLAW_NO_BROWSER=1
zeroclaw agent
```

Useful for:
- Headless servers
- SSH sessions
- Docker containers
- CI/CD environments

## Usage Modes

### Natural Language Tool (`composio_nl`)

The `composio_nl` tool provides natural language access to all Composio apps through a meta-tools workflow.

#### How It Works

```
User Query → COMPOSIO_SEARCH_TOOLS → COMPOSIO_MANAGE_CONNECTIONS → COMPOSIO_MULTI_EXECUTE_TOOL → Result
```

1. **SEARCH**: Discovers relevant tools based on natural language query
2. **MANAGE**: Checks OAuth connection status, triggers auth if needed
3. **EXECUTE**: Executes the discovered tool with appropriate parameters

#### Usage Examples

**Example 1: List Gmail Emails**
```
You: Show me my recent emails
Agent: [Uses composio_nl with query="list my gmail emails"]
Result: [Displays top 5 emails in natural language]
```

**Example 2: Create Dropbox Folder**
```
You: Create a folder called Projects in Dropbox
Agent: [Uses composio_nl with query="create dropbox folder /Projects"]
Result: ✓ Created folder /Projects in your Dropbox!
```

**Example 3: Send Slack Message**
```
You: Send a message to #general saying hello
Agent: [Uses composio_nl with query="send slack message to #general: Hello team!"]
Result: ✓ Message sent to #general!
```

**Example 4: Search GitHub Issues**
```
You: Find open issues in my-repo
Agent: [Uses composio_nl with query="search github issues in my-repo"]
Result: [Lists open issues with titles and numbers]
```

#### OAuth Flow with composio_nl

When authentication is needed:

1. Tool returns OAuth URL in response
2. Agent presents link to user:
   ```
   I need to connect to your Gmail first!
   
   🔗 Click here to authorize: [OAuth URL]
   ⏱ Link expires in 10 minutes
   
   Let me know when you've authorized, and I'll retry automatically!
   ```
3. User authorizes in browser
4. Agent automatically retries the same query
5. Tool executes successfully

#### Session Management

- Sessions are created automatically on first use
- Session ID is reused for 30 minutes (TTL)
- After expiration, new session is created transparently
- No manual session management required

### CLI Modes

### Mode A: CLI Auto-Open (Default)

**When:** Running `zeroclaw agent` on desktop

**Behavior:**
1. Tool detects OAuth needed
2. Browser automatically opens OAuth page
3. User authorizes in browser
4. ZeroClaw polls for connection (120s timeout)
5. Tool retries automatically
6. Success!

**Example:**
```bash
zeroclaw agent
# In chat: "Check my Gmail inbox"
# Browser opens → Authorize → Tool executes
```

### Mode D: CLI Print-Only

**When:** Running in headless environment or SSH

**Behavior:**
1. Tool detects OAuth needed
2. OAuth URL printed to console
3. User manually opens URL in browser
4. ZeroClaw polls for connection
5. Tool retries automatically

**Example:**
```bash
export ZEROCLAW_NO_BROWSER=1
zeroclaw agent
# In chat: "Check my Gmail inbox"
# URL printed → User opens manually → Tool executes
```

### Mode C: Server Return Link

**When:** Running `zeroclaw gateway` or `zeroclaw daemon`

**Behavior:**
1. Tool detects OAuth needed
2. Returns special error with OAuth URL
3. HTTP client receives error
4. Client presents URL to end user
5. No automatic polling (client's responsibility)

**Example Response:**
```json
{
  "success": false,
  "error": "COMPOSIO_OAUTH_REQUIRED: https://backend.composio.dev/..."
}
```

## Toolkit Management

### Natural Language Mode (Recommended)

When using `composio_nl`, toolkits are discovered automatically based on your query. No manual toolkit configuration needed.

**Example:**
```
You: List my Gmail emails
# Automatically discovers and uses Gmail toolkit
```

### Direct Tools Mode (Legacy)

### Adding Toolkits

Edit `config.toml`:

```toml
[composio.mcp]
toolkits = ["gmail", "github", "slack", "dropbox"]
```

Then regenerate MCP URL:
```bash
# Re-run wizard or manually update mcp_url
zeroclaw onboard --channels-only  # Preserves other settings
```

### Connecting Toolkits

**During Wizard:**
- Select toolkit when prompted
- Browser opens automatically
- Authorize and wait for confirmation

**During Runtime:**
- Use any tool from the toolkit
- OAuth flow triggers automatically
- Authorize in browser
- Tool retries and succeeds

### Checking Connection Status

Use the Composio dashboard:
https://app.composio.dev/connections

Or use the REST API tool:
```
You: List my connected Composio accounts
```

## Troubleshooting

### Natural Language Mode Issues

#### "No tools found for query"

**Cause:** Query is too vague or app name not included

**Solution:**
1. Be more specific in your query
2. Include app name explicitly (e.g., "gmail", "dropbox", "slack")
3. Describe the action clearly (e.g., "list", "create", "send")

**Example:**
- ❌ Bad: "Show my emails"
- ✅ Good: "List my gmail emails"

#### "OAuth authorization required"

**Cause:** App not connected yet

**Solution:**
1. Click the OAuth link provided
2. Authorize the app in browser
3. Confirm authorization to agent
4. Agent will automatically retry

#### "Tool execution failed"

**Cause:** Invalid parameters or app-specific error

**Solution:**
1. Check error message for details
2. Verify parameters are correct
3. Try rephrasing query with more specific details
4. Check app-specific requirements

### Direct Tools Mode Issues

### "MCP server returned no tools"

**Cause:** MCP URL is invalid or toolkits aren't configured

**Solution:**
1. Check `mcp_url` in config.toml
2. Verify toolkits are listed
3. Re-run wizard to regenerate URL

### "MCP URL format issues"

**Problem:** Diagnostic reports invalid MCP URL format

**Valid URL formats:**

1. **Direct MCP** (recommended):
   ```
   https://backend.composio.dev/mcp?user_id=default
   ```

2. **Tool Router** (session-based):
   ```
   https://backend.composio.dev/tool_router/trs_xxx/mcp?user_id=default
   ```

**Common issues:**

- ❌ Wrong domain: `https://wrong-domain.com/mcp`
- ❌ Has `toolkits` parameter: `...mcp?toolkits=gmail,github&user_id=default`
- ❌ Extra parameters: `...mcp?include_composio_helper_actions=true&user_id=default`

**How to fix:**

1. Open your config file:
   - Linux/macOS: `~/.zeroclaw/config.toml`
   - Windows: `%USERPROFILE%\.zeroclaw\config.toml`

2. Find the `[composio.mcp]` section:
   ```toml
   [composio.mcp]
   enabled = true
   mcp_url = "YOUR_URL_HERE"
   user_id = "default"
   ```

3. Update `mcp_url` to one of the valid formats above

4. Remove these parameters if present:
   - `toolkits=...`
   - `include_composio_helper_actions=...`
   - Any other query parameters except `user_id`

5. Test the fix:
   ```bash
   zeroclaw composio diagnostic-connect --verbose
   ```

**Example fix:**

```toml
# Before (invalid - has toolkits parameter)
mcp_url = "https://backend.composio.dev/tool_router/trs_abc/mcp?toolkits=gmail,github&user_id=default"

# After (valid - removed toolkits)
mcp_url = "https://backend.composio.dev/tool_router/trs_abc/mcp?user_id=default"
```

### "MCP server returned no tools"

**Cause:** MCP URL is invalid or toolkits aren't configured

**Solution:**
1. Check `mcp_url` in config.toml
2. Verify toolkits are listed
3. Re-run wizard to regenerate URL

### "Timeout waiting for connection"

**Cause:** OAuth authorization not completed within timeout (60s wizard, 120s runtime)

**Solution:**
1. Complete OAuth authorization faster
2. Check browser didn't block popup
3. Verify network connectivity
4. Try again - tool will re-trigger OAuth

### "Could not auto-open browser"

**Cause:** Running in headless environment or browser not found

**Solution:**
1. Set `ZEROCLAW_NO_BROWSER=1`
2. Manually open printed URL
3. Or use server mode for HTTP clients

### "Failed to generate MCP URL"

**Cause:** Invalid API key or network issue

**Solution:**
1. Verify Composio API key is correct
2. Check network connectivity
3. Try again or configure manually

### Tool Fails After OAuth

**Cause:** Connection not fully activated or tool parameters incorrect

**Solution:**
1. Check connection status in Composio dashboard
2. Verify tool parameters are correct
3. Wait a few seconds and retry
4. Check tool-specific requirements

### Meta-Tools Architecture

The `composio_nl` tool uses Composio's meta-tools pattern:

#### COMPOSIO_SEARCH_TOOLS

**Purpose:** Discover relevant tools based on natural language query

**Input:**
```json
{
  "queries": ["list my gmail emails"],
  "session": {
    "generate_id": true
  }
}
```

**Output:**
```json
{
  "tools": [
    {
      "tool_slug": "GMAIL_LIST_EMAILS",
      "description": "List emails from Gmail",
      "toolkit": "gmail",
      "input_schema": {...}
    }
  ],
  "session_id": "abc123"
}
```

#### COMPOSIO_MANAGE_CONNECTIONS

**Purpose:** Check OAuth connection status and trigger auth if needed

**Input:**
```json
{
  "toolkit": "gmail",
  "session": {
    "id": "abc123"
  }
}
```

**Output (Connected):**
```json
{
  "status": "ACTIVE",
  "connection_id": "conn_xyz"
}
```

**Output (Needs OAuth):**
```json
{
  "status": "DISCONNECTED",
  "redirect_url": "https://backend.composio.dev/oauth/..."
}
```

#### COMPOSIO_MULTI_EXECUTE_TOOL

**Purpose:** Execute discovered tool with parameters

**Input:**
```json
{
  "tool_slug": "GMAIL_LIST_EMAILS",
  "params": {
    "max_results": 10
  },
  "session": {
    "id": "abc123"
  }
}
```

**Output:**
```json
{
  "result": {
    "emails": [...]
  }
}
```

### Session Management Details

- **TTL**: 30 minutes (1800 seconds)
- **Reuse**: Same session ID used for all tools in workflow
- **Expiration**: New session created automatically after TTL
- **Isolation**: Each user/agent has separate session

### Tool Discovery Algorithm

1. Parse natural language query
2. Extract app name and action intent
3. Call COMPOSIO_SEARCH_TOOLS with query
4. Parse response to extract tool slugs
5. Select most relevant tool based on description
6. Return tool metadata for execution

### Manual MCP URL Configuration

If you prefer manual setup:

1. Create MCP server at https://app.composio.dev/mcp
2. Add toolkits and OAuth configs
3. Copy server ID
4. Update config.toml:

```toml
[composio.mcp]
enabled = true
server_id = "your_server_id"
user_id = "your_user_id"
```

### Custom Polling Timeouts

Currently fixed at:
- Wizard: 60 seconds
- Runtime: 120 seconds

Future versions will support configuration.

### Toolkit Slug Reference

Common toolkit slugs:
- `gmail` - Gmail
- `github` - GitHub
- `slack` - Slack
- `dropbox` - Dropbox
- `notion` - Notion
- `calendar` - Google Calendar
- `drive` - Google Drive
- `sheets` - Google Sheets
- `trello` - Trello
- `asana` - Asana

Full list: https://app.composio.dev/apps

## Agent System Prompt Integration

ZeroClaw automatically injects comprehensive Composio meta-tools protocol instructions into the agent's system prompt when the `composio_nl` tool is available.

### The 4-Step Workflow

The agent is trained to follow this mandatory workflow:

**Step A: Discover Tools (REQUIRED FIRST)**
- Always call `COMPOSIO_SEARCH_TOOLS` with use case description
- Generate session ID on first call: `session: {generate_id: true}`
- Save session_id for all subsequent calls

**Step B: Load Schemas (IF NEEDED)**
- If tools have `schemaRef` or incomplete schema
- Call `COMPOSIO_GET_TOOL_SCHEMAS` with tool slugs
- Never invent parameters

**Step C: Authenticate (IF NEEDED)**
- Before executing, ALWAYS call `COMPOSIO_MANAGE_CONNECTIONS` to verify connection
- NEVER invent or guess OAuth URLs
- ALWAYS extract `redirect_url` from the API response
- If not ACTIVE, show the real OAuth link from response and wait for user
- Only proceed after connection is ACTIVE

**Step D: Execute**
- Use `COMPOSIO_MULTI_EXECUTE_TOOL` with strict schema compliance
- Include session_id, current_step, current_step_metric
- Follow exact input_schema from Step B

### What the Agent Learns

The system prompt ensures the agent:

1. **Never skips discovery**: Always calls SEARCH first
2. **Never invents parameters**: Gets schema before execution
3. **Never executes without auth**: Checks connection status
4. **Never invents OAuth URLs**: Uses redirect_url from COMPOSIO_MANAGE_CONNECTIONS response
5. **Always uses session_id**: Maintains session across calls
6. **Handles pagination**: Iterates through all results when needed
7. **Warns before destruction**: Asks before delete/overwrite operations
8. **Formats naturally**: Presents results in human-readable format with emojis

### Gmail-Specific Instructions

The agent knows Gmail search operators:
- `from:email@example.com` - From sender
- `to:email@example.com` - To recipient  
- `subject:keyword` - Subject contains
- `after:2024/01/01` - After date
- `is:unread` - Unread only
- `has:attachment` - Has attachments

### Example: Complete Flow

**User asks:** "qual foi o último email do sergio.lechuga@hotmail.com?"

**Agent executes:**

1. **COMPOSIO_SEARCH_TOOLS**:
   ```json
   {
     "queries": [{"use_case": "fetch latest gmail email from specific sender"}],
     "session": {"generate_id": true}
   }
   ```

2. **COMPOSIO_GET_TOOL_SCHEMAS** (if needed):
   ```json
   {
     "tool_slugs": ["GMAIL_FETCH_EMAILS"],
     "session": {"id": "abc123"}
   }
   ```

3. **COMPOSIO_MANAGE_CONNECTIONS**:
   ```json
   {
     "toolkits": ["gmail"],
     "session": {"id": "abc123"}
   }
   ```

4. **COMPOSIO_MULTI_EXECUTE_TOOL**:
   ```json
   {
     "tools": [{
       "tool_slug": "GMAIL_FETCH_EMAILS",
       "arguments": {
         "user_id": "me",
         "query": "from:sergio.lechuga@hotmail.com",
         "max_results": 1,
         "include_payload": true
       }
     }],
     "session": {"id": "abc123"},
     "current_step": "FETCHING_EMAILS"
   }
   ```

5. **Formats response naturally**:
   ```
   Encontrei o último email de sergio.lechuga@hotmail.com:
   📧 Assunto: GEUS FORA DO AR
   📅 Data: 23/02/2026 08:59
   👤 De: Sérgio Luiz Lechuga Garcia
   📝 Mensagem: "Bom dia João! Favor verificar."
   ```

### Example: Connecting to Slack

**User asks:** "Poderia conectar ao meu slack também?"

**❌ WRONG (inventing URL):**
```
Agent: "Clique aqui: https://app.composio.dev/app-store/slack"
```

**✅ CORRECT (calling COMPOSIO_MANAGE_CONNECTIONS):**

1. **COMPOSIO_SEARCH_TOOLS**:
   ```json
   {
     "queries": [{"use_case": "connect to slack workspace"}],
     "session": {"generate_id": true}
   }
   ```

2. **COMPOSIO_MANAGE_CONNECTIONS**:
   ```json
   {
     "toolkits": ["slack"],
     "session": {"id": "xyz789"}
   }
   ```

3. **Extract redirect_url from response** and show:
   ```
   Preciso de autorização para conectar ao Slack.
   
   🔗 [Clique aqui para autorizar](https://backend.composio.dev/oauth/slack/xyz789?...)
   
   O link expira em 10 minutos. Depois de autorizar, me avise!
   ```

**Key difference:** The agent MUST call the API to get the real OAuth URL, not invent a generic one.

This integration happens automatically - no manual configuration needed!

## Security Considerations

### API Key Storage

- Composio API key stored in config.toml
- Enable encrypted secrets in wizard for protection
- Never commit config.toml to version control

### OAuth Tokens

- OAuth tokens managed by Composio
- Never stored locally in ZeroClaw
- Revoke access at https://app.composio.dev/connections

### Network Security

- All API calls use HTTPS
- OAuth flows use standard OAuth 2.0
- No credentials transmitted to ZeroClaw servers

## API Reference

### composio_nl Tool

**Tool Name:** `composio_nl`

**Description:** Natural language access to 1000+ apps via Composio MCP

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `query` | string | Yes | Natural language description of action |

**Example Invocations:**

```rust
// List Gmail emails
composio_nl { query: "list my gmail emails" }

// Create Dropbox folder
composio_nl { query: "create dropbox folder /Projects" }

// Send Slack message
composio_nl { query: "send slack message to #general: Hello!" }

// Search GitHub issues
composio_nl { query: "search github issues in my-repo" }
```

**Response Format:**

Success:
```json
{
  "success": true,
  "result": "Natural language summary of result"
}
```

OAuth Required:
```json
{
  "success": false,
  "error": "OAuth authorization required for GMAIL.\n\nPlease click this link to authorize:\nhttps://backend.composio.dev/oauth/...\n\nAfter authorizing, please retry your request.\nThe authorization link expires in 10 minutes."
}
```

Error:
```json
{
  "success": false,
  "error": "Error message with details"
}
```

### Composio REST API Endpoints

Used by ZeroClaw:

- `POST /api/v1/mcp/generate` - Generate MCP URL
- `POST /api/v3/connected_accounts/link` - Get OAuth URL
- `GET /api/v3/connected_accounts` - Check connection status

### MCP Protocol

- `tools/list` - List available tools
- `tools/call` - Execute tool with parameters

## FAQ

**Q: Should I use natural language mode or direct tools?**  
A: Natural language mode (`composio_nl`) is recommended for most use cases. It's simpler, more flexible, and handles OAuth automatically.

**Q: Do I need to configure toolkits manually?**  
A: No, when using `composio_nl`, toolkits are discovered automatically based on your query.

**Q: What's the difference between Pattern 1 and Pattern 2?**  
A: Pattern 1 (direct tools) requires knowing exact tool names. Pattern 2 (natural language) uses `composio_nl` with plain English queries and automatic tool discovery.

**Q: Can I use both patterns simultaneously?**  
A: Yes, but it's recommended to use one pattern consistently for clarity.

**Q: Do I need a Composio account?**  
A: Yes, get a free API key at https://app.composio.dev

**Q: How many apps are supported?**  
A: 1000+ apps including Gmail, GitHub, Slack, Notion, and more

**Q: Can I use multiple toolkits?**  
A: Yes, `composio_nl` can access any toolkit based on your query

**Q: What happens if OAuth expires?**  
A: ZeroClaw will automatically trigger re-authorization

**Q: Can I use this in production?**  
A: Yes, the integration is production-ready

**Q: Does this work offline?**  
A: No, requires internet for OAuth and API calls

**Q: Can I self-host Composio?**  
A: Contact Composio for enterprise options

**Q: What's the session TTL?**  
A: 30 minutes (1800 seconds). Sessions are reused automatically within this window.

**Q: How do I know which tool was used?**  
A: Enable debug logging: `RUST_LOG=zeroclaw=debug zeroclaw agent`

## Support

- ZeroClaw Issues: https://github.com/zeroclaw-labs/zeroclaw/issues
- Composio Docs: https://docs.composio.dev
- Composio Support: https://app.composio.dev/support

## Changelog

### v0.1.6 (February 26, 2026)
- ✅ Natural language tool (`composio_nl`) with meta-tools workflow
- ✅ Automatic tool discovery via COMPOSIO_SEARCH_TOOLS
- ✅ Automatic OAuth management via COMPOSIO_MANAGE_CONNECTIONS
- ✅ Session management with 30-minute TTL
- ✅ SSE client for robust MCP communication
- ✅ Automatic MCP URL generation
- ✅ Interactive toolkit selection
- ✅ Automatic OAuth handling with retry
- ✅ Multi-mode support (CLI/Server)
- ✅ Browser auto-open
- ✅ Connection polling

### Future
- [ ] Configurable session TTL
- [ ] Batch tool execution
- [ ] Tool result caching
- [ ] Connection health monitoring
- [ ] Callback server mode
- [ ] Multi-step workflow optimization
