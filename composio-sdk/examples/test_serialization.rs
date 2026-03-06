//! Test Serialization
//!
//! This example tests how the SessionConfig is being serialized.

use composio_sdk::models::{SessionConfig, ToolkitFilter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Test 1: Simple enable list
    let config1 = SessionConfig {
        user_id: "trs_zrVX9OXGc_4H".to_string(),
        toolkits: Some(ToolkitFilter::Enable(vec!["github".to_string()])),
        auth_configs: None,
        connected_accounts: None,
        manage_connections: None,
        tools: None,
        tags: None,
        workbench: None,
    };

    println!("Test 1: Enable list");
    println!("{}", serde_json::to_string_pretty(&config1)?);
    println!();

    // Test 2: Without toolkits
    let config2 = SessionConfig {
        user_id: "trs_zrVX9OXGc_4H".to_string(),
        toolkits: None,
        auth_configs: None,
        connected_accounts: None,
        manage_connections: None,
        tools: None,
        tags: None,
        workbench: None,
    };

    println!("Test 2: Without toolkits");
    println!("{}", serde_json::to_string_pretty(&config2)?);
    println!();

    Ok(())
}
