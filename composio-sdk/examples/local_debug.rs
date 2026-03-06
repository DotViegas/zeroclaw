/// Example demonstrating local debugging capabilities
/// 
/// Run with: cargo run --example local_debug --features local-debug
/// 
/// This example shows:
/// - Request/response logging
/// - Error inspection
/// - Performance profiling
/// - Network debugging

use composio_sdk::{ComposioClient, ComposioError};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debug output (only with local-debug feature)
    #[cfg(feature = "local-debug")]
    {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_target(false)
            .with_thread_ids(true)
            .with_line_number(true)
            .init();
        
        tracing::info!("🔍 Local debugging enabled");
    }

    // Get API key from environment
    let api_key = std::env::var("COMPOSIO_API_KEY")
        .expect("COMPOSIO_API_KEY environment variable not set");

    println!("🚀 Starting Composio SDK Debug Session\n");

    // 1. Client Initialization with Timing
    let start = Instant::now();
    let client = ComposioClient::builder()
        .api_key(api_key)
        .build()?;
    println!("✅ Client initialized in {:?}", start.elapsed());

    // 2. Session Creation with Detailed Logging
    println!("\n📦 Creating session...");
    let start = Instant::now();
    
    let session_result = client
        .create_session("debug_user_123")
        .toolkits(vec!["github"])
        .manage_connections(true)
        .send()
        .await;

    match session_result {
        Ok(session) => {
            println!("✅ Session created in {:?}", start.elapsed());
            println!("   Session ID: {}", session.session_id());
            println!("   MCP URL: {}", session.mcp_url());
            println!("   Tools available: {}", session.tools().len());

            // 3. List Toolkits with Pagination
            println!("\n📋 Listing toolkits...");
            let start = Instant::now();
            
            match session.list_toolkits().limit(5).send().await {
                Ok(toolkits) => {
                    println!("✅ Toolkits listed in {:?}", start.elapsed());
                    println!("   Total items: {}", toolkits.total_items);
                    println!("   Current page: {}", toolkits.current_page);
                    
                    for toolkit in &toolkits.items {
                        println!("   - {} ({})", toolkit.name, toolkit.slug);
                        if let Some(account) = &toolkit.connected_account {
                            println!("     Status: {}", account.status);
                        } else {
                            println!("     Status: Not connected");
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to list toolkits: {}", e);
                    debug_error(&e);
                }
            }

            // 4. Test Tool Execution (will fail without connection, but shows error handling)
            println!("\n🔧 Testing tool execution...");
            let start = Instant::now();
            
            let result = session
                .execute_tool(
                    "GITHUB_GET_REPOS",
                    serde_json::json!({
                        "owner": "composio"
                    })
                )
                .await;

            match result {
                Ok(response) => {
                    println!("✅ Tool executed in {:?}", start.elapsed());
                    println!("   Result: {:?}", response.data);
                }
                Err(e) => {
                    println!("⚠️  Tool execution failed (expected if not authenticated)");
                    debug_error(&e);
                }
            }

            // 5. Test Meta Tool
            println!("\n🔍 Testing meta tool (SEARCH_TOOLS)...");
            let start = Instant::now();
            
            let result = session
                .execute_meta_tool(
                    composio_sdk::MetaToolSlug::ComposioSearchTools,
                    serde_json::json!({
                        "query": "create github issue"
                    })
                )
                .await;

            match result {
                Ok(response) => {
                    println!("✅ Meta tool executed in {:?}", start.elapsed());
                    if let Some(data) = response.data {
                        println!("   Found tools: {}", 
                            data.as_array().map(|a| a.len()).unwrap_or(0));
                    }
                }
                Err(e) => {
                    println!("❌ Meta tool execution failed");
                    debug_error(&e);
                }
            }
        }
        Err(e) => {
            println!("❌ Session creation failed");
            debug_error(&e);
        }
    }

    println!("\n✨ Debug session complete!");
    Ok(())
}

/// Helper function to debug errors in detail
fn debug_error(error: &ComposioError) {
    println!("\n🔍 Error Details:");
    println!("   Type: {}", error);
    
    match error {
        ComposioError::ApiError { 
            status, 
            message, 
            code, 
            slug, 
            request_id, 
            suggested_fix, 
            errors 
        } => {
            println!("   Status: {}", status);
            println!("   Message: {}", message);
            if let Some(code) = code {
                println!("   Code: {}", code);
            }
            if let Some(slug) = slug {
                println!("   Slug: {}", slug);
            }
            if let Some(request_id) = request_id {
                println!("   Request ID: {}", request_id);
            }
            if let Some(fix) = suggested_fix {
                println!("   💡 Suggested Fix: {}", fix);
            }
            if let Some(errors) = errors {
                println!("   Additional Errors:");
                for err in errors {
                    println!("     - {}: {}", err.field, err.message);
                }
            }
        }
        ComposioError::NetworkError(e) => {
            println!("   Network Error: {}", e);
            println!("   💡 Check your internet connection and API endpoint");
        }
        ComposioError::SerializationError(e) => {
            println!("   Serialization Error: {}", e);
            println!("   💡 Check request/response format");
        }
        ComposioError::InvalidInput(msg) => {
            println!("   Invalid Input: {}", msg);
            println!("   💡 Verify your input parameters");
        }
        ComposioError::ConfigError(msg) => {
            println!("   Config Error: {}", msg);
            println!("   💡 Check your API key and configuration");
        }
    }
}
