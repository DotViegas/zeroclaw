//! Example: Retrieving meta tools schemas
//!
//! This example demonstrates how to retrieve the complete schemas for all meta tools
//! available in a session. Meta tools are special tools provided by Composio for
//! runtime tool discovery, connection management, and advanced operations.
//!
//! Run with:
//! ```bash
//! COMPOSIO_API_KEY=your_key cargo run --example meta_tools_schema
//! ```

use composio_sdk::ComposioClient;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = env::var("COMPOSIO_API_KEY")
        .expect("COMPOSIO_API_KEY environment variable must be set");

    // Initialize client
    let client = ComposioClient::builder().api_key(api_key).build()?;

    println!("Creating session for user...");

    // Create a session for a user
    let session = client.create_session("user_123").send().await?;

    println!("Session created: {}", session.session_id());
    println!("MCP URL: {}", session.mcp_url());
    println!();

    // Get meta tools schemas
    println!("Retrieving meta tools schemas...");
    let meta_tools = session.get_meta_tools().await?;

    println!("Found {} meta tools:\n", meta_tools.len());

    // Display information about each meta tool
    for (index, tool) in meta_tools.iter().enumerate() {
        println!("{}. {}", index + 1, tool.slug);
        println!("   Name: {}", tool.name);
        println!("   Description: {}", tool.description);
        println!("   Toolkit: {}", tool.toolkit);
        println!("   Version: {}", tool.version);
        println!("   No Auth Required: {}", tool.no_auth);
        println!("   Deprecated: {}", tool.is_deprecated);

        // Display tags if any
        if !tool.tags.is_empty() {
            println!("   Tags: {}", tool.tags.join(", "));
        }

        // Display available versions
        if !tool.available_versions.is_empty() {
            println!(
                "   Available Versions: {}",
                tool.available_versions.join(", ")
            );
        }

        // Display input parameters schema
        println!("   Input Parameters:");
        println!(
            "{}",
            serde_json::to_string_pretty(&tool.input_parameters)?
                .lines()
                .map(|line| format!("      {}", line))
                .collect::<Vec<_>>()
                .join("\n")
        );

        // Display output parameters schema
        println!("   Output Parameters:");
        println!(
            "{}",
            serde_json::to_string_pretty(&tool.output_parameters)?
                .lines()
                .map(|line| format!("      {}", line))
                .collect::<Vec<_>>()
                .join("\n")
        );

        println!();
    }

    // Example: Find a specific meta tool by slug
    println!("Looking for COMPOSIO_SEARCH_TOOLS...");
    if let Some(search_tool) = meta_tools
        .iter()
        .find(|t| t.slug == "COMPOSIO_SEARCH_TOOLS")
    {
        println!("Found COMPOSIO_SEARCH_TOOLS:");
        println!("  Description: {}", search_tool.description);
        println!("  This tool helps discover relevant tools across 1000+ apps");
        println!("  Input schema: {}", search_tool.input_parameters);
    } else {
        println!("COMPOSIO_SEARCH_TOOLS not found in this session");
    }

    println!("\n✓ Meta tools schemas retrieved successfully!");

    Ok(())
}
