//! Simple Session Test
//!
//! Test creating a session without any configuration.

use composio_sdk::ComposioClient;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("COMPOSIO_API_KEY")
        .expect("COMPOSIO_API_KEY environment variable must be set");

    println!("Creating client...");
    let client = ComposioClient::builder().api_key(api_key).build()?;

    println!("Creating session without toolkits...");
    match client.create_session("trs_zrVX9OXGc_4H").send().await {
        Ok(session) => {
            println!("✅ SUCCESS!");
            println!("  Session ID: {}", session.session_id());
            println!("  MCP URL: {}", session.mcp_url());
            println!("  Tools: {}", session.tools().len());
        }
        Err(e) => {
            println!("❌ ERROR: {:?}", e);
        }
    }

    Ok(())
}
