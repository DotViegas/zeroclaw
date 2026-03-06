//! Meta Tools Example - Composio Rust SDK
//!
//! This example demonstrates the five core meta tools in Composio:
//! 1. COMPOSIO_SEARCH_TOOLS - Discover relevant tools across 1000+ apps
//! 2. COMPOSIO_MULTI_EXECUTE_TOOL - Execute up to 20 tools in parallel
//! 3. COMPOSIO_MANAGE_CONNECTIONS - Handle OAuth and API key authentication
//! 4. COMPOSIO_REMOTE_WORKBENCH - Run Python code in persistent sandbox
//! 5. COMPOSIO_REMOTE_BASH_TOOL - Execute bash commands for file/data processing
//!
//! ## What are Meta Tools?
//!
//! Meta tools are Composio's core tools that enable dynamic tool discovery and execution.
//! Unlike regular tools (like GITHUB_CREATE_ISSUE), meta tools operate at a higher level:
//! - They help agents discover which tools to use
//! - They orchestrate multiple tool executions
//! - They manage authentication flows
//! - They provide computational environments for complex operations
//!
//! All meta tools share context via the session_id, making them work together seamlessly.
//!
//! ## Prerequisites
//!
//! 1. Set your Composio API key:
//!    ```bash
//!    export COMPOSIO_API_KEY="your-api-key-here"
//!    ```
//!
//! 2. Have at least one connected account (for COMPOSIO_MULTI_EXECUTE_TOOL example)
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run --example meta_tools
//! ```
//!
//! ## Reference
//!
//! - Core Concepts: COMPOSIO DOCS/02-COMPOSIO-CORE-CONCEPTS.md (Meta Tools section)
//! - Tool Router API: COMPOSIO DOCS/17-API-REFERENCE-COMPOSIO-TOOL-ROUTER.md

use composio_sdk::{ComposioClient, MetaToolSlug};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Composio Rust SDK - Meta Tools Example ===\n");

    // Initialize client and create session
    println!("Initializing Composio client...");
    let client = ComposioClient::builder()
        .api_key(std::env::var("COMPOSIO_API_KEY")?)
        .build()?;

    println!("Creating session for user 'meta_tools_demo'...");
    let session = client
        .create_session("meta_tools_demo")
        .manage_connections(true)
        .send()
        .await?;

    println!("✓ Session created: {}\n", session.session_id());
    println!("Available meta tools: {}\n", session.tools().len());

    // ========================================================================
    // Meta Tool 1: COMPOSIO_SEARCH_TOOLS
    // ========================================================================
    //
    // Purpose: Discover relevant tools across 1000+ apps based on use case
    //
    // This is the most important meta tool - it enables agents to dynamically
    // discover which tools to use without having all tool schemas in context.
    //
    // Key Features:
    // - Natural language search ("create a GitHub issue")
    // - Returns tools with full schemas
    // - Includes connection status for each toolkit
    // - Provides execution plan and guidance
    // - Suggests related tools
    // - Identifies known pitfalls
    //
    // When to use:
    // - Agent needs to find tools for a specific task
    // - You want to reduce context size (only load relevant tools)
    // - Building conversational agents that discover capabilities at runtime
    //
    // Returns:
    // - tools: Array of tool schemas with input/output parameters
    // - connection_status: Which toolkits are connected
    // - execution_plan: Recommended steps to accomplish the task
    // - related_tools: Additional tools that might be useful
    // - known_pitfalls: Common mistakes to avoid

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("1. COMPOSIO_SEARCH_TOOLS - Dynamic Tool Discovery");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("Searching for tools to 'create a GitHub issue'...\n");

    match session
        .execute_meta_tool(
            MetaToolSlug::ComposioSearchTools,
            json!({
                "query": "create a GitHub issue",
                "limit": 3  // Limit results for demo
            }),
        )
        .await
    {
        Ok(result) => {
            println!("✓ Search completed successfully");
            println!("  Log ID: {}\n", result.log_id);

            if let Some(error) = result.error {
                println!("  Error: {}\n", error);
            } else {
                println!("  Search Results:");
                println!("{}\n", serde_json::to_string_pretty(&result.data)?);

                println!("  💡 What you get from COMPOSIO_SEARCH_TOOLS:");
                println!("     • Tool schemas with input/output parameters");
                println!("     • Connection status (which toolkits are authenticated)");
                println!("     • Execution guidance and recommended steps");
                println!("     • Related tools you might also need");
                println!("     • Known pitfalls and common mistakes to avoid");
            }
        }
        Err(e) => {
            eprintln!("✗ Search failed: {}", e);
        }
    }
    println!();

    // ========================================================================
    // Meta Tool 2: COMPOSIO_MULTI_EXECUTE_TOOL
    // ========================================================================
    //
    // Purpose: Execute up to 20 tools in parallel for efficiency
    //
    // This meta tool enables batch execution of multiple tools, which is
    // significantly faster than executing them sequentially.
    //
    // Key Features:
    // - Execute up to 20 tools in a single request
    // - Parallel execution for speed
    // - Each tool can have different arguments
    // - Returns results in the same order as input
    // - Partial success (some tools can succeed while others fail)
    //
    // When to use:
    // - Fetching data from multiple sources simultaneously
    // - Performing bulk operations (e.g., creating multiple issues)
    // - Reducing latency by parallelizing independent operations
    // - Implementing fan-out patterns
    //
    // Example use cases:
    // - Fetch repos from GitHub AND emails from Gmail simultaneously
    // - Create issues in multiple repositories at once
    // - Check status across multiple services in parallel

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("2. COMPOSIO_MULTI_EXECUTE_TOOL - Parallel Tool Execution");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("Executing multiple tools in parallel...\n");

    match session
        .execute_meta_tool(
            MetaToolSlug::ComposioMultiExecuteTool,
            json!({
                "tools": [
                    {
                        "tool_slug": "GITHUB_GET_REPOS",
                        "arguments": {
                            "owner": "composio",
                            "type": "public"
                        }
                    },
                    {
                        "tool_slug": "GITHUB_GET_USER",
                        "arguments": {
                            "username": "composio"
                        }
                    }
                ]
            }),
        )
        .await
    {
        Ok(result) => {
            println!("✓ Multi-execution completed");
            println!("  Log ID: {}\n", result.log_id);

            if let Some(error) = result.error {
                println!("  Error: {}\n", error);
            } else {
                println!("  Results:");
                println!("{}\n", serde_json::to_string_pretty(&result.data)?);

                println!("  💡 Benefits of COMPOSIO_MULTI_EXECUTE_TOOL:");
                println!("     • Execute up to 20 tools in parallel");
                println!("     • Significantly faster than sequential execution");
                println!("     • Partial success handling (some can fail, others succeed)");
                println!("     • Results returned in same order as input");
                println!("     • Reduces overall latency for bulk operations");
            }
        }
        Err(e) => {
            eprintln!("✗ Multi-execution failed: {}", e);
        }
    }
    println!();

    // ========================================================================
    // Meta Tool 3: COMPOSIO_MANAGE_CONNECTIONS
    // ========================================================================
    //
    // Purpose: Handle OAuth and API key authentication flows
    //
    // This meta tool manages the authentication lifecycle for external services.
    // It's automatically used when manage_connections=true in session config.
    //
    // Key Features:
    // - Generate Connect Links for OAuth flows
    // - Check connection status for toolkits
    // - List all connected accounts
    // - Disconnect accounts
    // - Handle re-authentication for expired connections
    //
    // When to use:
    // - Agent needs to authenticate a user to a service
    // - Checking if a toolkit is connected before using it
    // - Building connection management UIs
    // - Handling connection expiry and re-auth
    //
    // In-chat authentication flow:
    // 1. Agent tries to use a tool (e.g., GITHUB_CREATE_ISSUE)
    // 2. COMPOSIO_SEARCH_TOOLS returns connection_status: false
    // 3. Agent calls COMPOSIO_MANAGE_CONNECTIONS to get Connect Link
    // 4. User clicks link and authenticates
    // 5. Agent retries the original tool
    //
    // Note: This is enabled by default with manage_connections=true

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("3. COMPOSIO_MANAGE_CONNECTIONS - Authentication Management");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("Checking connection status and generating auth link...\n");

    match session
        .execute_meta_tool(
            MetaToolSlug::ComposioManageConnections,
            json!({
                "action": "get_connection_link",
                "toolkit": "github",
                "callback_url": "https://your-app.com/auth/callback"
            }),
        )
        .await
    {
        Ok(result) => {
            println!("✓ Connection management completed");
            println!("  Log ID: {}\n", result.log_id);

            if let Some(error) = result.error {
                println!("  Error: {}\n", error);
            } else {
                println!("  Result:");
                println!("{}\n", serde_json::to_string_pretty(&result.data)?);

                println!("  💡 COMPOSIO_MANAGE_CONNECTIONS capabilities:");
                println!("     • Generate OAuth Connect Links for any toolkit");
                println!("     • Check connection status before tool execution");
                println!("     • List all connected accounts for a user");
                println!("     • Disconnect accounts when needed");
                println!("     • Handle re-authentication for expired connections");
                println!("     • Automatically used with manage_connections=true");
            }
        }
        Err(e) => {
            eprintln!("✗ Connection management failed: {}", e);
        }
    }
    println!();

    // ========================================================================
    // Meta Tool 4: COMPOSIO_REMOTE_WORKBENCH
    // ========================================================================
    //
    // Purpose: Run Python code in a persistent Jupyter notebook environment
    //
    // The workbench is a powerful computational environment for complex operations
    // that would be difficult or impossible with individual tools.
    //
    // Key Features:
    // - Persistent Jupyter notebook environment (state preserved across calls)
    // - Pre-installed libraries: pandas, numpy, matplotlib, Pillow, PyTorch, reportlab
    // - Auto-installs additional packages as needed
    // - Built-in helpers:
    //   • run_composio_tool() - Execute any Composio tool
    //   • invoke_llm() - Call LLM for classification/summarization
    //   • upload_local_file() - Upload files to cloud storage
    //   • proxy_execute() - Direct API calls to connected services
    //   • web_search() - Search the web
    //   • smart_file_extract() - Extract text from PDFs/images
    // - Error correction for common mistakes
    //
    // When to use:
    // - Bulk operations (process 100+ items)
    // - Data analysis and reporting (analyze CSV, generate charts)
    // - Multi-step workflows (fetch data → process → upload results)
    // - Complex transformations (image processing, PDF generation)
    // - Operations requiring state (iterative processing)
    //
    // Example use cases:
    // - Analyze sales data from CSV and generate PDF report
    // - Process 500 GitHub issues and categorize them
    // - Fetch emails, extract attachments, analyze content
    // - Generate charts from database queries
    //
    // Note: Workbench is part of the meta tools system (not available with direct execution)

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("4. COMPOSIO_REMOTE_WORKBENCH - Persistent Python Sandbox");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("Executing Python code in workbench...\n");

    match session
        .execute_meta_tool(
            MetaToolSlug::ComposioRemoteWorkbench,
            json!({
                "code": r#"
import pandas as pd
import numpy as np

# Example: Analyze some data
data = {
    'toolkit': ['github', 'gmail', 'slack', 'notion'],
    'tools_count': [150, 45, 80, 60],
    'connected': [True, True, False, True]
}

df = pd.DataFrame(data)

# Calculate statistics
total_tools = df['tools_count'].sum()
connected_toolkits = df[df['connected']]['toolkit'].tolist()

result = {
    'total_tools': int(total_tools),
    'connected_toolkits': connected_toolkits,
    'average_tools_per_toolkit': float(df['tools_count'].mean()),
    'summary': f"Analyzed {len(df)} toolkits with {total_tools} total tools"
}

result
"#
            }),
        )
        .await
    {
        Ok(result) => {
            println!("✓ Workbench execution completed");
            println!("  Log ID: {}\n", result.log_id);

            if let Some(error) = result.error {
                println!("  Error: {}\n", error);
            } else {
                println!("  Result:");
                println!("{}\n", serde_json::to_string_pretty(&result.data)?);

                println!("  💡 COMPOSIO_REMOTE_WORKBENCH capabilities:");
                println!("     • Persistent Jupyter environment (state preserved)");
                println!("     • Pre-installed: pandas, numpy, matplotlib, PyTorch, reportlab");
                println!("     • Auto-installs additional packages");
                println!("     • Built-in helpers: run_composio_tool, invoke_llm, web_search");
                println!("     • Error correction for common mistakes");
                println!("     • Perfect for bulk operations and data analysis");
                println!("     • Can execute any Composio tool via run_composio_tool()");
            }
        }
        Err(e) => {
            eprintln!("✗ Workbench execution failed: {}", e);
        }
    }
    println!();

    // ========================================================================
    // Meta Tool 5: COMPOSIO_REMOTE_BASH_TOOL
    // ========================================================================
    //
    // Purpose: Execute bash commands for file and data processing
    //
    // This meta tool provides a bash shell environment for operations that
    // are easier to express as shell commands than Python code.
    //
    // Key Features:
    // - Execute any bash command
    // - Access to common Unix utilities (grep, awk, sed, curl, jq, etc.)
    // - File system operations
    // - Text processing and data manipulation
    // - Integration with external tools
    //
    // When to use:
    // - File operations (list, search, transform)
    // - Text processing (grep, sed, awk)
    // - Data format conversions (CSV to JSON, etc.)
    // - Calling external CLI tools
    // - Quick data inspection
    //
    // Example use cases:
    // - Search log files for errors
    // - Convert CSV to JSON
    // - Extract specific fields from text files
    // - Download and process files with curl
    // - Parse JSON with jq
    //
    // Security note: Commands run in a sandboxed environment

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("5. COMPOSIO_REMOTE_BASH_TOOL - Bash Command Execution");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("Executing bash commands...\n");

    match session
        .execute_meta_tool(
            MetaToolSlug::ComposioRemoteBashTool,
            json!({
                "command": "echo 'Hello from Composio Workbench!' && date && uname -a"
            }),
        )
        .await
    {
        Ok(result) => {
            println!("✓ Bash execution completed");
            println!("  Log ID: {}\n", result.log_id);

            if let Some(error) = result.error {
                println!("  Error: {}\n", error);
            } else {
                println!("  Result:");
                println!("{}\n", serde_json::to_string_pretty(&result.data)?);

                println!("  💡 COMPOSIO_REMOTE_BASH_TOOL capabilities:");
                println!("     • Execute any bash command");
                println!("     • Access to Unix utilities: grep, awk, sed, curl, jq");
                println!("     • File system operations");
                println!("     • Text processing and data manipulation");
                println!("     • Integration with external CLI tools");
                println!("     • Runs in sandboxed environment for security");
            }
        }
        Err(e) => {
            eprintln!("✗ Bash execution failed: {}", e);
        }
    }
    println!();

    // ========================================================================
    // Meta Tools Working Together
    // ========================================================================
    //
    // The real power of meta tools comes from using them together.
    // They share context via session_id, enabling complex workflows.
    //
    // Example workflow:
    // 1. COMPOSIO_SEARCH_TOOLS - Find tools for "analyze GitHub issues"
    // 2. COMPOSIO_MANAGE_CONNECTIONS - Ensure GitHub is connected
    // 3. COMPOSIO_MULTI_EXECUTE_TOOL - Fetch issues from multiple repos
    // 4. COMPOSIO_REMOTE_WORKBENCH - Analyze issues with pandas
    // 5. COMPOSIO_REMOTE_BASH_TOOL - Generate report file
    //
    // This workflow would be difficult to implement with individual tools,
    // but meta tools make it straightforward.

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Meta Tools Summary");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("The five meta tools work together to enable powerful AI agents:\n");

    println!("1. COMPOSIO_SEARCH_TOOLS");
    println!("   → Discover tools dynamically based on use case");
    println!("   → Reduces context size (only load relevant tools)");
    println!("   → Returns execution guidance and known pitfalls\n");

    println!("2. COMPOSIO_MULTI_EXECUTE_TOOL");
    println!("   → Execute up to 20 tools in parallel");
    println!("   → Significantly faster than sequential execution");
    println!("   → Perfect for bulk operations and data fetching\n");

    println!("3. COMPOSIO_MANAGE_CONNECTIONS");
    println!("   → Handle OAuth and API key authentication");
    println!("   → Generate Connect Links for users");
    println!("   → Check connection status before tool execution\n");

    println!("4. COMPOSIO_REMOTE_WORKBENCH");
    println!("   → Persistent Python sandbox with pandas, numpy, etc.");
    println!("   → Perfect for data analysis and complex transformations");
    println!("   → Can execute any Composio tool via run_composio_tool()\n");

    println!("5. COMPOSIO_REMOTE_BASH_TOOL");
    println!("   → Execute bash commands for file/text processing");
    println!("   → Access to Unix utilities (grep, awk, sed, curl, jq)");
    println!("   → Quick data inspection and format conversions\n");

    println!("All meta tools share context via session_id, enabling complex workflows.");
    println!();

    println!("=== Example completed successfully! ===\n");

    println!("Next steps:");
    println!("- Check out examples/basic_usage.rs for session management");
    println!("- Check out examples/auth_link_creation.rs for authentication flows");
    println!("- Check out examples/toolkit_listing.rs for toolkit discovery");
    println!("- Read COMPOSIO DOCS/02-COMPOSIO-CORE-CONCEPTS.md for meta tools details");
    println!("- Read the documentation: cargo doc --open");

    Ok(())
}

