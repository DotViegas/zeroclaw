# Composio Rust SDK

> A minimal, type-safe Rust SDK for integrating AI agents with 1000+ external services through the Composio platform.

[![Crates.io](https://img.shields.io/crates/v/composio-sdk.svg)](https://crates.io/crates/composio-sdk)
[![Documentation](https://docs.rs/composio-sdk/badge.svg)](https://docs.rs/composio-sdk)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.70%2B-blue.svg)](https://www.rust-lang.org)
[![GitHub](https://img.shields.io/badge/github-DotViegas%2Fcomposio--sdk--rust-blue)](https://github.com/DotViegas/composio-sdk-rust)

## What is Composio?

Composio is a platform that connects AI agents to external services like GitHub, Gmail, Slack, and 1000+ other applications. Instead of building integrations for each service yourself, Composio provides:

- **Universal API**: One SDK to access all services
- **Authentication Management**: OAuth, API keys, and other auth methods handled automatically
- **Tool Discovery**: AI agents can discover and use tools at runtime
- **Sandboxed Execution**: Safe environment for running code and commands
- **Event Triggers**: React to events from connected services

Think of it as a "universal adapter" that lets your AI agent interact with any external service through a consistent interface.

## Why This SDK?

This is a **pure Rust implementation** of the Composio SDK, designed for:

- **Performance**: Native async/await with Tokio, minimal memory footprint (~2 MB)
- **Type Safety**: Compile-time guarantees with Rust's type system
- **Reliability**: Automatic retries, comprehensive error handling
- **Self-Contained**: All dependencies bundled, no external setup required
- **Production Ready**: Built for real-world applications with proper error handling and logging

## How It Works

### The Big Picture

![Composio SDK Architecture](architecture.png)

The architecture shows how your AI agent interacts with external services through the Composio SDK:

1. **Your AI Agent** uses the Composio Rust SDK
2. **SDK Components** provide sessions, meta tools, and wizard guidance
3. **Composio Platform** handles authentication and routing
4. **External Services** (GitHub, Gmail, Slack, 1000+ apps) are accessed through a unified interface

### Core Concepts

#### 1. Sessions (User Isolation)

Every user gets their own session. This ensures:
- User A's GitHub credentials don't mix with User B's
- Each user can connect different accounts (work email vs personal email)
- Tools execute with the correct user's permissions

```rust
// Create a session for a specific user
let session = client
    .create_session("user_123")
    .toolkits(vec!["github", "gmail"])
    .send()
    .await?;
```

#### 2. Meta Tools (Runtime Discovery)

Instead of hardcoding which tools your agent can use, meta tools let the agent discover and use tools dynamically:

- **COMPOSIO_SEARCH_TOOLS**: "Find me tools to send emails"
- **COMPOSIO_MANAGE_CONNECTIONS**: "Connect my Gmail account"
- **COMPOSIO_MULTI_EXECUTE_TOOL**: "Run these 5 tools in parallel"
- **COMPOSIO_REMOTE_WORKBENCH**: "Run this Python code in a sandbox"
- **COMPOSIO_REMOTE_BASH_TOOL**: "Execute this bash command safely"

This is powerful because your agent can adapt to new tasks without code changes.

#### 3. Native Rust Meta Tools

We've implemented 4 of the 5 meta tools in **pure Rust** (no Python dependencies):

```rust
use composio_sdk::meta_tools::*;

// Search for tools
let search = ToolSearch::new(Arc::new(client));
let tools = search.search("send email", &session_id).await?;

// Execute multiple tools in parallel
let executor = MultiExecutor::new(Arc::new(client));
let results = executor.execute_parallel(&session_id, tool_calls).await?;

// Manage OAuth connections
let manager = ConnectionManager::new(Arc::new(client));
let is_connected = manager.is_connected(&session_id, "github").await?;

// Execute bash commands
let bash = BashExecutor::new();
let result = bash.execute("ls -la").await?;
```

Only the Workbench uses remote Python execution (by design, for data processing).

#### 4. Wizard Instructions (AI Guidance)

The SDK includes bundled "Skills" - best practices and patterns for using Composio effectively. The wizard module generates instructions for AI agents:

```rust
use composio_sdk::wizard::generate_wizard_instructions;

// Generate instructions for GitHub integration
let instructions = generate_wizard_instructions(Some("github"))?;

// Your AI agent reads these instructions to learn:
// - How to create sessions correctly
// - How to handle authentication
// - Common pitfalls to avoid
// - Toolkit-specific best practices
```

This helps AI agents use Composio correctly without trial and error.

## Architecture

### Module Structure

```
composio-sdk/
├── src/
│   ├── client.rs           # HTTP client, API communication
│   ├── session.rs          # Session management
│   ├── config.rs           # Configuration
│   ├── error.rs            # Error types
│   ├── retry.rs            # Retry logic with exponential backoff
│   │
│   ├── models/             # Data structures
│   │   ├── request.rs      # API request types
│   │   ├── response.rs     # API response types
│   │   └── enums.rs        # Enumerations
│   │
│   ├── meta_tools/         # Native Rust implementations
│   │   ├── search.rs       # Tool discovery
│   │   ├── multi_executor.rs  # Parallel execution
│   │   ├── connections.rs  # OAuth management
│   │   ├── bash.rs         # Command execution
│   │   └── workbench.rs    # Python sandbox (hybrid)
│   │
│   └── wizard/             # AI guidance system
│       ├── skills.rs       # Skills extraction
│       ├── generator.rs    # Instruction generation
│       └── validator.rs    # Pattern validation
│
└── skills/                 # Bundled best practices (33 files)
    ├── AGENTS.md           # Consolidated reference
    ├── SKILL.md            # Metadata
    └── rules/              # 31 rule files
```

### Data Flow

```
1. Your Code
   ↓
2. ComposioClient (HTTP client with retry logic)
   ↓
3. Session (user-scoped context)
   ↓
4. Meta Tools (discovery, execution, auth)
   ↓
5. Composio API (backend.composio.dev)
   ↓
6. External Services (GitHub, Gmail, etc.)
```

### Key Design Decisions

**Why Sessions?**
- Isolates users from each other
- Manages authentication per user
- Provides consistent context for tool execution

**Why Meta Tools?**
- Enables runtime tool discovery
- Reduces context window usage (only 5 tools vs 1000+)
- Allows agents to adapt to new tasks

**Why Native Rust?**
- Better performance (no Python overhead)
- Easier deployment (single binary)
- Type safety at compile time
- Smaller memory footprint

**Why Bundled Skills?**
- No external dependencies
- Always available at compile time
- Consistent behavior across installations
- Helps AI agents learn best practices

## Quick Start

### Installation

```toml
[dependencies]
composio-sdk = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
```

### Basic Usage

```rust
use composio_sdk::{ComposioClient, ComposioError};

#[tokio::main]
async fn main() -> Result<(), ComposioError> {
    // 1. Create client
    let client = ComposioClient::builder()
        .api_key(std::env::var("COMPOSIO_API_KEY")?)
        .build()?;

    // 2. Create session for a user
    let session = client
        .create_session("user_123")
        .toolkits(vec!["github", "gmail"])
        .send()
        .await?;

    // 3. Execute a tool
    let result = session
        .execute_tool(
            "GITHUB_CREATE_ISSUE",
            serde_json::json!({
                "owner": "composio",
                "repo": "composio",
                "title": "Test issue",
                "body": "Created via Rust SDK"
            })
        )
        .await?;

    println!("Issue created: {:?}", result.data);
    Ok(())
}
```

### With Meta Tools

```rust
use composio_sdk::meta_tools::ToolSearch;
use std::sync::Arc;

// Let the agent discover tools at runtime
let search = ToolSearch::new(Arc::new(client));
let tools = search.search("create GitHub issue", &session_id).await?;

// Agent now knows which tools to use
for tool in tools {
    println!("Found: {} - {}", tool.slug, tool.description);
}
```

### With Wizard Instructions

```rust
use composio_sdk::wizard::generate_wizard_instructions;

// Generate instructions for your AI agent
let instructions = generate_wizard_instructions(Some("github"))?;

// Feed these instructions to your AI agent
// The agent learns best practices automatically
```

## Real-World Example

Here's how you might build a GitHub automation agent:

```rust
use composio_sdk::{ComposioClient, meta_tools::*};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    let client = Arc::new(ComposioClient::builder()
        .api_key(std::env::var("COMPOSIO_API_KEY")?)
        .build()?);
    
    let session = client
        .create_session("user_123")
        .toolkits(vec!["github"])
        .send()
        .await?;
    
    let session_id = session.session_id();
    
    // 1. Check if GitHub is connected
    let conn_manager = ConnectionManager::new(client.clone());
    if !conn_manager.is_connected(session_id, "github").await? {
        // Create auth link for user
        let link = conn_manager.create_auth_link(
            session_id,
            "github",
            Some("https://myapp.com/callback")
        ).await?;
        
        println!("Please connect GitHub: {}", link.redirect_url);
        return Ok(());
    }
    
    // 2. Search for relevant tools
    let search = ToolSearch::new(client.clone());
    let tools = search.search("list GitHub repositories", session_id).await?;
    
    println!("Agent can use these tools:");
    for tool in &tools {
        println!("  - {}: {}", tool.slug, tool.name);
    }
    
    // 3. Execute tools in parallel
    let executor = MultiExecutor::new(client.clone());
    let tool_calls = vec![
        ToolCall {
            tool_slug: "GITHUB_GET_REPOS".to_string(),
            arguments: serde_json::json!({ "owner": "composio" }),
            connected_account_id: None,
        },
        ToolCall {
            tool_slug: "GITHUB_GET_ISSUES".to_string(),
            arguments: serde_json::json!({ 
                "owner": "composio", 
                "repo": "composio" 
            }),
            connected_account_id: None,
        },
    ];
    
    let results = executor.execute_parallel(session_id, tool_calls).await?;
    println!("Executed {} tools successfully", results.successful);
    
    Ok(())
}
```

## Features

### Core Features
- ✅ Session management with user isolation
- ✅ Tool execution (regular and meta tools)
- ✅ Toolkit listing and filtering
- ✅ Authentication link creation
- ✅ Automatic retry with exponential backoff
- ✅ Comprehensive error handling

### Native Rust Meta Tools
- ✅ Tool search and discovery
- ✅ Multi-tool parallel execution
- ✅ Connection management (OAuth)
- ✅ Bash command execution
- ✅ Workbench (hybrid: Rust wrapper + remote Python)

### Wizard System
- ✅ Bundled Skills content (33 files)
- ✅ Instruction generation for AI agents
- ✅ Pattern validation
- ✅ Toolkit-specific guidance

### Performance
- ✅ ~2 MB memory footprint
- ✅ Async/await with Tokio
- ✅ Zero-copy deserialization where possible
- ✅ Efficient Arc-based sharing

## Configuration

Customize client behavior:

```rust
use std::time::Duration;

let client = ComposioClient::builder()
    .api_key("your-api-key")
    .base_url("https://backend.composio.dev/api/v3")
    .timeout(Duration::from_secs(30))
    .max_retries(3)
    .initial_retry_delay(Duration::from_secs(1))
    .max_retry_delay(Duration::from_secs(10))
    .build()?;
```

## Authentication Patterns

### In-Chat Authentication (Default)

The agent automatically prompts users when authentication is needed:

```rust
let session = client
    .create_session("user_123")
    .manage_connections(true)  // Default
    .send()
    .await?;

// Agent will automatically handle auth when needed
```

### Manual Authentication

Pre-authenticate users during onboarding:

```rust
// Create auth link
let link = session
    .create_auth_link("github", Some("https://yourapp.com/callback"))
    .await?;

println!("Redirect user to: {}", link.redirect_url);

// Wait for connection
link.wait_for_connection(Duration::from_secs(300)).await?;
```

## Error Handling

The SDK provides detailed error information:

```rust
use composio_sdk::ComposioError;

match session.execute_tool("INVALID_TOOL", serde_json::json!({})).await {
    Ok(result) => println!("Success: {:?}", result),
    Err(ComposioError::ApiError { status, message, suggested_fix, .. }) => {
        eprintln!("API error ({}): {}", status, message);
        if let Some(fix) = suggested_fix {
            eprintln!("Suggested fix: {}", fix);
        }
    }
    Err(ComposioError::NetworkError(e)) => {
        eprintln!("Network error: {}", e);
    }
    Err(e) => {
        eprintln!("Other error: {}", e);
    }
}
```

## Examples

The SDK includes comprehensive examples:

```bash
# Basic usage
cargo run --example basic_usage

# Authentication flows
cargo run --example authentication

# Meta tools
cargo run --example meta_tools_usage

# Wizard instructions
cargo run --example wizard_instructions

# Tool execution
cargo run --example tool_execution
```

See the [`examples/`](examples/) directory for complete working examples.

## Documentation

- **API Docs**: [docs.rs/composio-sdk](https://docs.rs/composio-sdk)
- **Composio Platform**: [docs.composio.dev](https://docs.composio.dev)
- **Meta Tools Guide**: [README_META_TOOLS.md](README_META_TOOLS.md)
- **Wizard System**: [WIZARD_INSTRUCTIONS.md](WIZARD_INSTRUCTIONS.md)
- **Skills Documentation**: [SKILLS_BUNDLED.md](SKILLS_BUNDLED.md)

## Requirements

- Rust 1.70 or later
- Tokio runtime
- Composio API key ([get one here](https://app.composio.dev))

## Performance

The SDK is optimized for production use:

- **Library size**: 2.45 MB (release build)
- **Runtime overhead**: 112 bytes (client) + 296 bytes (session builder)
- **Initialization time**: ~200 µs (client creation)
- **Memory footprint**: Minimal, suitable for resource-constrained environments

See [MEMORY_FOOTPRINT_REPORT.md](MEMORY_FOOTPRINT_REPORT.md) for detailed analysis.

## Development

```bash
# Clone the repository
git clone https://github.com/DotViegas/composio-sdk-rust.git
cd composio-sdk-rust/composio-sdk

# Set your API key
export COMPOSIO_API_KEY="your_api_key_here"

# Run tests
cargo test

# Run examples
cargo run --example basic_usage

# Build documentation
cargo doc --open
```

See [DEVELOPMENT.md](DEVELOPMENT.md) for detailed development guide.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Support

- [GitHub Issues](https://github.com/DotViegas/composio-sdk-rust/issues)
- [Discord Community](https://discord.gg/composio) (Composio Platform)
- [Documentation](https://docs.composio.dev) (Composio Platform)

## Acknowledgments

This SDK was created by [DotViegas](https://github.com/DotViegas) for [ZeroClaw](https://github.com/zeroclaw), a lightweight Rust AI assistant. It follows the design patterns from the official [Composio Python SDK](https://github.com/ComposioHQ/composio) while providing native Rust implementations for better performance and type safety.

---

**Made with ❤️ for the Rust community**
