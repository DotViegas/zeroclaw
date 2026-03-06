//! Basic Usage Example - Composio Rust SDK
//!
//! This example demonstrates the fundamental features of the Composio SDK:
//! - Client initialization with API key
//! - Session creation for a user
//! - Toolkit filtering (enable/disable)
//! - Tool execution with arguments
//! - Error handling patterns
//!
//! ## Prerequisites
//!
//! 1. Set your Composio API key as an environment variable:
//!    ```bash
//!    export COMPOSIO_API_KEY="your-api-key-here"
//!    ```
//!
//! 2. Ensure you have at least one connected account for the GitHub toolkit
//!    (or modify the example to use a different toolkit you have connected)
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run --example basic_usage
//! ```
//!
//! ## What This Example Shows
//!
//! - **Client Initialization**: Creating a Composio client with your API key
//! - **Session Creation**: Creating a session scoped to a specific user
//! - **Toolkit Filtering**: Enabling specific toolkits or disabling unwanted ones
//! - **Tool Execution**: Executing tools with JSON arguments
//! - **Error Handling**: Handling different error types gracefully
//! - **Session Information**: Accessing session ID and MCP URL

use composio_sdk::{ComposioClient, ComposioError};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Composio Rust SDK - Basic Usage Example ===\n");

    // ========================================================================
    // Step 1: Initialize the Composio Client
    // ========================================================================
    //
    // The client is the main entry point for interacting with the Composio API.
    // It requires an API key which you can get from the Composio dashboard.
    //
    // The builder pattern allows you to customize:
    // - API key (required)
    // - Base URL (optional, defaults to Composio's production API)
    // - Timeout (optional, defaults to 30 seconds)
    // - Retry configuration (optional, defaults to 3 retries with exponential backoff)

    println!("Step 1: Initializing Composio client...");
    
    let client = ComposioClient::builder()
        .api_key(std::env::var("COMPOSIO_API_KEY")?)
        .build()?;
    
    println!("✓ Client initialized successfully\n");

    // ========================================================================
    // Step 2: Create a Session for a User
    // ========================================================================
    //
    // Sessions are the core concept in Composio's Tool Router API.
    // Each session is scoped to a specific user and defines:
    // - Which toolkits are available
    // - Which authentication configs to use
    // - Which connected accounts to use
    // - Whether to enable in-chat connection management
    //
    // Sessions are immutable - create a new session when configuration changes.

    println!("Step 2: Creating a session for user 'demo_user_123'...");
    
    let session = client
        .create_session("demo_user_123")
        .toolkits(vec!["github", "gmail"])  // Enable specific toolkits
        .manage_connections(true)            // Enable in-chat authentication
        .send()
        .await?;
    
    println!("✓ Session created successfully");
    println!("  Session ID: {}", session.session_id());
    println!("  MCP URL: {}", session.mcp_url());
    println!("  Available tools: {} meta tools", session.tools().len());
    println!();

    // ========================================================================
    // Step 3: Toolkit Filtering - Enable Specific Toolkits
    // ========================================================================
    //
    // You can control which toolkits are available in a session.
    // This is useful for:
    // - Limiting the agent's capabilities to specific services
    // - Reducing context size by excluding unnecessary toolkits
    // - Implementing role-based access control

    println!("Step 3: Creating a session with specific toolkits enabled...");
    
    let github_session = client
        .create_session("demo_user_123")
        .toolkits(vec!["github"])  // Only enable GitHub toolkit
        .send()
        .await?;
    
    println!("✓ GitHub-only session created: {}", github_session.session_id());
    println!();

    // ========================================================================
    // Step 4: Toolkit Filtering - Disable Specific Toolkits
    // ========================================================================
    //
    // Alternatively, you can disable specific toolkits while keeping all others.
    // This is useful when you want most toolkits but need to exclude a few.

    println!("Step 4: Creating a session with specific toolkits disabled...");
    
    let filtered_session = client
        .create_session("demo_user_123")
        .disable_toolkits(vec!["exa", "firecrawl"])  // Disable search toolkits
        .send()
        .await?;
    
    println!("✓ Filtered session created: {}", filtered_session.session_id());
    println!();

    // ========================================================================
    // Step 5: Execute a Tool with Arguments
    // ========================================================================
    //
    // Tools are executed within a session context.
    // Each tool requires:
    // - Tool slug (e.g., "GITHUB_GET_REPOS")
    // - Arguments as a JSON object
    //
    // The SDK automatically:
    // - Injects authentication from the user's connected account
    // - Retries on transient failures
    // - Returns structured responses with data and error fields

    println!("Step 5: Executing a tool (GITHUB_GET_REPOS)...");
    
    match github_session
        .execute_tool(
            "GITHUB_GET_REPOS",
            json!({
                "owner": "composio",
                "type": "public"
            }),
        )
        .await
    {
        Ok(result) => {
            println!("✓ Tool executed successfully");
            println!("  Log ID: {}", result.log_id);
            
            // Check if the tool returned an error
            if let Some(error) = result.error {
                println!("  Tool Error: {}", error);
            } else {
                // Pretty-print the result data
                println!("  Result:");
                println!("{}", serde_json::to_string_pretty(&result.data)?);
            }
        }
        Err(e) => {
            eprintln!("✗ Tool execution failed: {}", e);
            handle_error(&e);
        }
    }
    println!();

    // ========================================================================
    // Step 6: Error Handling Patterns
    // ========================================================================
    //
    // The SDK provides detailed error types that help you handle failures:
    // - ApiError: HTTP errors from the Composio API (with status, message, suggested_fix)
    // - NetworkError: Connection issues, timeouts
    // - SerializationError: JSON parsing failures
    // - InvalidInput: Client-side validation errors
    // - ConfigError: Configuration issues
    //
    // Always check for suggested_fix in ApiError - it provides actionable guidance.

    println!("Step 6: Demonstrating error handling...");
    
    // Example: Executing a tool that doesn't exist
    match github_session
        .execute_tool("INVALID_TOOL_SLUG", json!({}))
        .await
    {
        Ok(_) => {
            println!("  Unexpected success");
        }
        Err(e) => {
            println!("✓ Error caught as expected");
            handle_error(&e);
        }
    }
    println!();

    // Example: Executing a tool with missing required arguments
    match github_session
        .execute_tool(
            "GITHUB_CREATE_ISSUE",
            json!({
                // Missing required fields: owner, repo, title
            }),
        )
        .await
    {
        Ok(_) => {
            println!("  Unexpected success");
        }
        Err(e) => {
            println!("✓ Error caught as expected");
            handle_error(&e);
        }
    }
    println!();

    // ========================================================================
    // Step 7: Additional Session Features
    // ========================================================================
    //
    // Sessions provide additional capabilities:
    // - List available toolkits with connection status
    // - Get meta tool schemas
    // - Create authentication links for users
    // - Execute meta tools (COMPOSIO_SEARCH_TOOLS, etc.)

    println!("Step 7: Listing available toolkits...");
    
    match session.list_toolkits().send().await {
        Ok(toolkits) => {
            println!("✓ Found {} toolkits", toolkits.items.len());
            for toolkit in toolkits.items.iter().take(5) {
                let status = if toolkit.connected_account.is_some() {
                    "✓ Connected"
                } else {
                    "○ Not connected"
                };
                println!("  {} - {} ({})", toolkit.slug, toolkit.name, status);
            }
            if toolkits.items.len() > 5 {
                println!("  ... and {} more", toolkits.items.len() - 5);
            }
        }
        Err(e) => {
            eprintln!("✗ Failed to list toolkits: {}", e);
        }
    }
    println!();

    println!("=== Example completed successfully! ===\n");
    
    println!("Next steps:");
    println!("- Check out examples/meta_tools.rs for meta tool usage");
    println!("- Check out examples/auth_link_creation.rs for authentication flows");
    println!("- Check out examples/toolkit_listing.rs for advanced filtering");
    println!("- Read the documentation: cargo doc --open");

    Ok(())
}

/// Helper function to handle and display different error types
///
/// This demonstrates best practices for error handling with the Composio SDK.
/// Always check for:
/// - Status code (to determine if retry is appropriate)
/// - Error message (for logging and debugging)
/// - Suggested fix (for actionable guidance to users)
/// - Request ID (for support inquiries)
fn handle_error(error: &ComposioError) {
    match error {
        ComposioError::ApiError {
            status,
            message,
            code,
            slug,
            request_id,
            suggested_fix,
            errors,
        } => {
            println!("  Error Type: API Error");
            println!("  Status Code: {}", status);
            println!("  Message: {}", message);
            
            if let Some(code) = code {
                println!("  Error Code: {}", code);
            }
            
            if let Some(slug) = slug {
                println!("  Error Slug: {}", slug);
            }
            
            if let Some(request_id) = request_id {
                println!("  Request ID: {} (use this when contacting support)", request_id);
            }
            
            if let Some(fix) = suggested_fix {
                println!("  💡 Suggested Fix: {}", fix);
            }
            
            if let Some(errors) = errors {
                if !errors.is_empty() {
                    println!("  Detailed Errors:");
                    for err in errors {
                        if let Some(field) = &err.field {
                            println!("    - {}: {}", field, err.message);
                        } else {
                            println!("    - {}", err.message);
                        }
                    }
                }
            }
            
            // Provide guidance based on status code
            match *status {
                400 => println!("  ℹ️  This is a client error - check your request parameters"),
                401 => println!("  ℹ️  Authentication failed - check your API key"),
                403 => println!("  ℹ️  Access forbidden - check your permissions"),
                404 => println!("  ℹ️  Resource not found - check the tool slug or session ID"),
                429 => println!("  ℹ️  Rate limited - the SDK will automatically retry"),
                500..=599 => println!("  ℹ️  Server error - the SDK will automatically retry"),
                _ => {}
            }
        }
        
        ComposioError::NetworkError(e) => {
            println!("  Error Type: Network Error");
            println!("  Message: {}", e);
            println!("  ℹ️  Check your internet connection - the SDK will automatically retry");
        }
        
        ComposioError::SerializationError(e) => {
            println!("  Error Type: Serialization Error");
            println!("  Message: {}", e);
            println!("  ℹ️  This indicates a JSON parsing issue - check your arguments");
        }
        
        ComposioError::InvalidInput(msg) => {
            println!("  Error Type: Invalid Input");
            println!("  Message: {}", msg);
            println!("  ℹ️  Fix the input validation error and try again");
        }
        
        ComposioError::ConfigError(msg) => {
            println!("  Error Type: Configuration Error");
            println!("  Message: {}", msg);
            println!("  ℹ️  Check your SDK configuration (API key, base URL, etc.)");
        }
    }
}
