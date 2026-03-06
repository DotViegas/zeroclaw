//! Simple API Key Test
//!
//! This example tests if the API key is valid by making a simple request
//! to the Composio API.

use composio_sdk::ComposioClient;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("COMPOSIO_API_KEY")
        .expect("COMPOSIO_API_KEY environment variable must be set");

    println!("Testing API key: {}", &api_key[..10]); // Show first 10 chars only
    println!();

    // Initialize client
    let client = ComposioClient::builder()
        .api_key(&api_key)
        .build()?;

    println!("✓ Client created successfully");
    println!("  Base URL: {}", client.config().base_url);
    println!("  Timeout: {:?}", client.config().timeout);
    println!();

    // Try to create a session with the test user ID
    println!("Attempting to create session...");
    println!("  User ID: trs_zrVX9OXGc_4H");
    println!("  Toolkits: github");
    println!();

    match client
        .create_session("trs_zrVX9OXGc_4H")
        .toolkits(vec!["github"])
        .send()
        .await
    {
        Ok(session) => {
            println!("✅ SUCCESS! Session created:");
            println!("  Session ID: {}", session.session_id());
            println!("  MCP URL: {}", session.mcp_url());
            println!("  Tools available: {}", session.tools().len());
        }
        Err(e) => {
            println!("❌ ERROR: {}", e);
            println!();
            println!("Debug info:");
            println!("  Error type: {:?}", e);
            
            // Check if it's an API error
            if let composio_sdk::error::ComposioError::ApiError { 
                status, 
                message, 
                code,
                slug,
                request_id,
                suggested_fix,
                errors 
            } = e {
                println!("  Status: {}", status);
                println!("  Message: {}", message);
                println!("  Code: {:?}", code);
                println!("  Slug: {:?}", slug);
                println!("  Request ID: {:?}", request_id);
                println!("  Suggested fix: {:?}", suggested_fix);
                println!("  Errors: {:?}", errors);
            }
        }
    }

    Ok(())
}
