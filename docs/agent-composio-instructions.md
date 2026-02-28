# Agent Instructions: Using Composio Tools Effectively

## Overview

When using the `composio_nl` tool to interact with external apps (Gmail, GitHub, Slack, etc.), you MUST provide both a natural language query AND the appropriate arguments for the discovered tool.

## Critical Rules

### 1. Always Provide Tool-Specific Arguments

The `composio_nl` tool accepts two parameters:
- `query` (required): Natural language description to discover the right tool
- `arguments` (optional but HIGHLY RECOMMENDED): Tool-specific parameters

**WRONG** (will fail or return incomplete results):
```json
{
  "query": "list my gmail emails from sergio.lechuga@hotmail.com"
}
```

**CORRECT** (will work properly):
```json
{
  "query": "list my gmail emails from sergio.lechuga@hotmail.com",
  "arguments": {
    "user_id": "me",
    "query": "from:sergio.lechuga@hotmail.com",
    "max_results": 10,
    "include_payload": true,
    "include_spam_trash": false
  }
}
```

### 2. Gmail Search Query Format

When searching Gmail, use Gmail's search operators in the `query` argument:

**Common Gmail Search Operators:**
- `from:email@example.com` - Emails from specific sender
- `to:email@example.com` - Emails to specific recipient
- `subject:keyword` - Emails with keyword in subject
- `after:2024/01/01` - Emails after date
- `before:2024/12/31` - Emails before date
- `has:attachment` - Emails with attachments
- `is:unread` - Unread emails
- `is:starred` - Starred emails
- `in:inbox` - Emails in inbox
- `in:sent` - Sent emails

**Examples:**
- Find emails from specific sender: `"from:sergio.lechuga@hotmail.com"`
- Find recent unread emails: `"is:unread after:2024/02/01"`
- Find emails with attachments from sender: `"from:john@example.com has:attachment"`

### 3. Common Tool Arguments by App

#### Gmail (GMAIL_FETCH_EMAILS, GMAIL_LIST_EMAILS)
```json
{
  "user_id": "me",
  "query": "from:sender@example.com",
  "max_results": 10,
  "include_payload": true,
  "include_spam_trash": false,
  "verbose": true
}
```

#### Gmail (GMAIL_SEND_EMAIL)
```json
{
  "to": "recipient@example.com",
  "subject": "Email subject",
  "body": "Email body content",
  "cc": "optional@example.com",
  "bcc": "optional@example.com"
}
```

#### GitHub (GITHUB_LIST_ISSUES)
```json
{
  "owner": "username",
  "repo": "repository-name",
  "state": "open",
  "labels": "bug,enhancement",
  "per_page": 10
}
```

#### GitHub (GITHUB_CREATE_ISSUE)
```json
{
  "owner": "username",
  "repo": "repository-name",
  "title": "Issue title",
  "body": "Issue description",
  "labels": ["bug", "priority-high"]
}
```

#### Slack (SLACK_SEND_MESSAGE)
```json
{
  "channel": "#general",
  "text": "Message content",
  "thread_ts": "optional-thread-timestamp"
}
```

#### Dropbox (DROPBOX_CREATE_FOLDER)
```json
{
  "path": "/Projects/NewFolder",
  "autorename": false
}
```

#### Dropbox (DROPBOX_LIST_FOLDER)
```json
{
  "path": "/Projects",
  "recursive": false,
  "include_deleted": false
}
```

## User Query Translation Examples

### Example 1: Find Latest Email from Specific Sender

**User asks:** "qual foi o último email do sergio.lechuga@hotmail.com?"

**Your tool call:**
```json
{
  "tool": "composio_nl",
  "arguments": {
    "query": "fetch latest gmail email from sergio.lechuga@hotmail.com",
    "arguments": {
      "user_id": "me",
      "query": "from:sergio.lechuga@hotmail.com",
      "max_results": 1,
      "include_payload": true,
      "include_spam_trash": false,
      "verbose": true
    }
  }
}
```

### Example 2: List Recent Emails

**User asks:** "mostre meus últimos 5 emails"

**Your tool call:**
```json
{
  "tool": "composio_nl",
  "arguments": {
    "query": "list my recent gmail emails",
    "arguments": {
      "user_id": "me",
      "query": "in:inbox",
      "max_results": 5,
      "include_payload": true,
      "include_spam_trash": false
    }
  }
}
```

### Example 3: Find Unread Emails from Specific Domain

**User asks:** "quais emails não lidos eu tenho da empresa X?"

**Your tool call:**
```json
{
  "tool": "composio_nl",
  "arguments": {
    "query": "find unread gmail emails from company domain",
    "arguments": {
      "user_id": "me",
      "query": "is:unread from:@companyx.com",
      "max_results": 20,
      "include_payload": true,
      "include_spam_trash": false
    }
  }
}
```

### Example 4: Send Email

**User asks:** "envie um email para joao@example.com dizendo olá"

**Your tool call:**
```json
{
  "tool": "composio_nl",
  "arguments": {
    "query": "send gmail email to joao@example.com",
    "arguments": {
      "to": "joao@example.com",
      "subject": "Olá",
      "body": "Olá! Como vai?"
    }
  }
}
```

## Response Formatting

When presenting results to the user:

1. **Extract key information** from the JSON response
2. **Format in natural language** (don't dump raw JSON)
3. **Highlight important details** (sender, subject, date, snippet)
4. **Be concise** but informative

**WRONG:**
```
Here's the result:
{
  "data": {
    "results": [...]
  }
}
```

**CORRECT:**
```
Encontrei o último email de sergio.lechuga@hotmail.com:

📧 Assunto: GEUS FORA DO AR
📅 Data: 23/02/2026 08:59
👤 De: Sérgio Luiz Lechuga Garcia <sergio.lechuga@hotmail.com>
📝 Mensagem: "Bom dia João! Favor verificar."
```

## Error Handling

### OAuth Required

If you receive an OAuth error, present the authorization link clearly:

```
Para acessar seu Gmail, preciso que você autorize a conexão:

🔗 Clique aqui para autorizar: [URL]

Após autorizar, me avise que tentarei novamente automaticamente.
```

### No Results Found

If no results are found, suggest alternatives:

```
Não encontrei emails de sergio.lechuga@hotmail.com nos últimos resultados.

Possíveis motivos:
- Os emails podem estar em outra pasta (Spam, Arquivados)
- Pode não haver emails recentes deste remetente
- A busca pode ter limitações de data

Quer que eu:
1. Busque em todas as pastas (incluindo Spam)?
2. Amplie o período de busca?
3. Busque por outro critério?
```

## Testing Your Understanding

Before making a tool call, ask yourself:

1. ✅ Did I include the `arguments` parameter?
2. ✅ Did I use Gmail search operators correctly (e.g., `from:`, `to:`, `subject:`)?
3. ✅ Did I set appropriate `max_results` for the user's request?
4. ✅ Did I set `include_payload: true` to get email content?
5. ✅ Will I format the response in natural language for the user?

## Summary

**The key insight:** The `query` parameter is for tool DISCOVERY (finding the right Composio tool), but the `arguments` parameter is for tool EXECUTION (providing the actual parameters the tool needs).

Always provide both for best results!
