//! Test Raw Response
//!
//! See the raw JSON response from the API.

use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("COMPOSIO_API_KEY")
        .expect("COMPOSIO_API_KEY environment variable must be set");

    let client = reqwest::Client::new();
    let url = "https://backend.composio.dev/api/v3/tool_router/session";
    
    let body = serde_json::json!({
        "user_id": "trs_zrVX9OXGc_4H"
    });

    let response = client
        .post(url)
        .header("x-api-key", &api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    println!("Status: {}", response.status());
    
    let response_text = response.text().await?;
    
    // Pretty print JSON
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response_text) {
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("{}", response_text);
    }

    Ok(())
}
