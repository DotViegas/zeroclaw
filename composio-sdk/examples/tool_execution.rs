//! Example demonstrating tool execution with the Composio SDK
//!
//! This example shows how to:
//! - Create a session for a user
//! - Execute a tool with arguments
//! - Handle the response and errors
//!
//! Run with: cargo run --example tool_execution

use composio_sdk::client::ComposioClient;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the Composio client with your API key
    let client = ComposioClient::builder()
        .api_key(std::env::var("COMPOSIO_API_KEY")?)
        .build()?;

    println!("Creating session for user...");

    // Create a session for a specific user with GitHub toolkit enabled
    let session = client
        .create_session("user_123")
        .toolkits(vec!["github"])
        .send()
        .await?;

    println!("Session created: {}", session.session_id());
    println!("MCP URL: {}", session.mcp_url());
    println!();

    // Example 1: Execute a tool to get GitHub repositories
    println!("Example 1: Getting GitHub repositories...");
    match session
        .execute_tool(
            "GITHUB_GET_REPOS",
            json!({
                "owner": "composio"
            }),
        )
        .await
    {
        Ok(result) => {
            println!("✓ Success!");
            println!("  Log ID: {}", result.log_id);
            
            if let Some(error) = result.error {
                println!("  Tool Error: {}", error);
            } else {
                println!("  Result: {}", serde_json::to_string_pretty(&result.data)?);
            }
        }
        Err(e) => {
            eprintln!("✗ Error: {}", e);
        }
    }
    println!();

    // Example 2: Execute a tool to create a GitHub issue
    println!("Example 2: Creating a GitHub issue...");
    match session
        .execute_tool(
            "GITHUB_CREATE_ISSUE",
            json!({
                "owner": "composio",
                "repo": "composio",
                "title": "Test issue from Rust SDK",
                "body": "This issue was created using the Composio Rust SDK as a demonstration."
            }),
        )
        .await
    {
        Ok(result) => {
            println!("✓ Success!");
            println!("  Log ID: {}", result.log_id);
            
            if let Some(error) = result.error {
                println!("  Tool Error: {}", error);
            } else {
                println!("  Result: {}", serde_json::to_string_pretty(&result.data)?);
            }
        }
        Err(e) => {
            eprintln!("✗ Error: {}", e);
            
            // Handle specific error types
            match e {
                composio_sdk::error::ComposioError::ApiError {
                    status,
                    message,
                    suggested_fix,
                    ..
                } => {
                    eprintln!("  Status: {}", status);
                    eprintln!("  Message: {}", message);
                    if let Some(fix) = suggested_fix {
                        eprintln!("  Suggested Fix: {}", fix);
                    }
                }
                _ => {}
            }
        }
    }
    println!();

    // Example 3: Execute a tool with empty arguments
    println!("Example 3: Listing GitHub user's repositories...");
    match session
        .execute_tool("GITHUB_LIST_USER_REPOS", json!({}))
        .await
    {
        Ok(result) => {
            println!("✓ Success!");
            println!("  Log ID: {}", result.log_id);
            
            if let Some(error) = result.error {
                println!("  Tool Error: {}", error);
            } else {
                println!("  Result: {}", serde_json::to_string_pretty(&result.data)?);
            }
        }
        Err(e) => {
            eprintln!("✗ Error: {}", e);
        }
    }

    Ok(())
}
