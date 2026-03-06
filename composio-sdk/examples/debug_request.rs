//! Debug HTTP Request
//!
//! This example shows the actual HTTP request being made to help debug authentication issues.

use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("COMPOSIO_API_KEY")
        .expect("COMPOSIO_API_KEY environment variable must be set");

    println!("=== Debug HTTP Request ===");
    println!();
    println!("API Key: {}...{}", &api_key[..10], &api_key[api_key.len()-5..]);
    println!("API Key Length: {}", api_key.len());
    println!();

    // Make a raw HTTP request to see what's happening
    let client = reqwest::Client::new();
    let url = "https://backend.composio.dev/api/v3/tool_router/session";
    
    println!("Making POST request to:");
    println!("  {}", url);
    println!();
    println!("Headers:");
    println!("  x-api-key: {}...{}", &api_key[..10], &api_key[api_key.len()-5..]);
    println!("  Content-Type: application/json");
    println!();

    let body = serde_json::json!({
        "user_id": "trs_zrVX9OXGc_4H",
        "toolkits": ["github", "gmail"]
    });

    println!("Request Body:");
    println!("{}", serde_json::to_string_pretty(&body)?);
    println!();

    let response = client
        .post(url)
        .header("x-api-key", &api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    println!("Response:");
    println!("  Status: {}", response.status());
    println!("  Headers:");
    for (name, value) in response.headers() {
        println!("    {}: {:?}", name, value);
    }
    println!();

    let response_text = response.text().await?;
    println!("Response Body:");
    
    // Try to pretty print JSON if possible
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response_text) {
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("{}", response_text);
    }

    Ok(())
}
