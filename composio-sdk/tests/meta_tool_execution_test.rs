//! Integration tests for meta tool execution

use composio_sdk::{ComposioClient, ComposioError, MetaToolSlug};
use serde_json::json;

#[tokio::test]
async fn test_execute_meta_tool_search_tools() {
    // This test requires a valid API key and will make real API calls
    // Skip if COMPOSIO_API_KEY is not set
    let api_key = match std::env::var("COMPOSIO_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("Skipping test: COMPOSIO_API_KEY not set");
            return;
        }
    };

    let client = ComposioClient::builder()
        .api_key(api_key)
        .build()
        .expect("Failed to build client");

    // Create a session
    let session = client
        .create_session("test_user_meta_tool")
        .toolkits(vec!["github"])
        .send()
        .await
        .expect("Failed to create session");

    // Execute COMPOSIO_SEARCH_TOOLS meta tool
    let result = session
        .execute_meta_tool(
            MetaToolSlug::ComposioSearchTools,
            json!({
                "query": "create a GitHub issue"
            }),
        )
        .await;

    match result {
        Ok(response) => {
            println!("Meta tool execution successful");
            println!("Data: {:?}", response.data);
            println!("Log ID: {}", response.log_id);
            
            // Verify response structure
            assert!(!response.log_id.is_empty());
            
            if let Some(error) = response.error {
                println!("Warning: Tool execution returned error: {}", error);
            }
        }
        Err(e) => {
            println!("Meta tool execution failed: {:?}", e);
            // Don't fail the test if it's a connection/auth issue
            match e {
                ComposioError::ApiError { status, .. } if status == 401 || status == 403 => {
                    println!("Skipping test due to authentication issue");
                }
                _ => panic!("Unexpected error: {:?}", e),
            }
        }
    }
}

#[tokio::test]
async fn test_execute_meta_tool_multi_execute() {
    // This test requires a valid API key and will make real API calls
    // Skip if COMPOSIO_API_KEY is not set
    let api_key = match std::env::var("COMPOSIO_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("Skipping test: COMPOSIO_API_KEY not set");
            return;
        }
    };

    let client = ComposioClient::builder()
        .api_key(api_key)
        .build()
        .expect("Failed to build client");

    // Create a session
    let session = client
        .create_session("test_user_multi_execute")
        .toolkits(vec!["github"])
        .send()
        .await
        .expect("Failed to create session");

    // Execute COMPOSIO_MULTI_EXECUTE_TOOL meta tool
    let result = session
        .execute_meta_tool(
            MetaToolSlug::ComposioMultiExecuteTool,
            json!({
                "tools": [
                    {
                        "tool_slug": "GITHUB_GET_REPOS",
                        "arguments": {"owner": "composio"}
                    }
                ]
            }),
        )
        .await;

    match result {
        Ok(response) => {
            println!("Multi-execute meta tool successful");
            println!("Data: {:?}", response.data);
            println!("Log ID: {}", response.log_id);
            
            // Verify response structure
            assert!(!response.log_id.is_empty());
            
            if let Some(error) = response.error {
                println!("Warning: Tool execution returned error: {}", error);
            }
        }
        Err(e) => {
            println!("Multi-execute meta tool failed: {:?}", e);
            // Don't fail the test if it's a connection/auth issue
            match e {
                ComposioError::ApiError { status, .. } if status == 401 || status == 403 => {
                    println!("Skipping test due to authentication issue");
                }
                _ => panic!("Unexpected error: {:?}", e),
            }
        }
    }
}

#[tokio::test]
async fn test_execute_meta_tool_manage_connections() {
    // This test requires a valid API key and will make real API calls
    // Skip if COMPOSIO_API_KEY is not set
    let api_key = match std::env::var("COMPOSIO_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("Skipping test: COMPOSIO_API_KEY not set");
            return;
        }
    };

    let client = ComposioClient::builder()
        .api_key(api_key)
        .build()
        .expect("Failed to build client");

    // Create a session with manage_connections enabled
    let session = client
        .create_session("test_user_manage_connections")
        .toolkits(vec!["github"])
        .manage_connections(true)
        .send()
        .await
        .expect("Failed to create session");

    // Execute COMPOSIO_MANAGE_CONNECTIONS meta tool
    let result = session
        .execute_meta_tool(
            MetaToolSlug::ComposioManageConnections,
            json!({
                "toolkit": "github"
            }),
        )
        .await;

    match result {
        Ok(response) => {
            println!("Manage connections meta tool successful");
            println!("Data: {:?}", response.data);
            println!("Log ID: {}", response.log_id);
            
            // Verify response structure
            assert!(!response.log_id.is_empty());
            
            if let Some(error) = response.error {
                println!("Warning: Tool execution returned error: {}", error);
            }
        }
        Err(e) => {
            println!("Manage connections meta tool failed: {:?}", e);
            // Don't fail the test if it's a connection/auth issue
            match e {
                ComposioError::ApiError { status, .. } if status == 401 || status == 403 => {
                    println!("Skipping test due to authentication issue");
                }
                _ => panic!("Unexpected error: {:?}", e),
            }
        }
    }
}

#[tokio::test]
async fn test_execute_meta_tool_workbench() {
    // This test requires a valid API key and will make real API calls
    // Skip if COMPOSIO_API_KEY is not set
    let api_key = match std::env::var("COMPOSIO_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("Skipping test: COMPOSIO_API_KEY not set");
            return;
        }
    };

    let client = ComposioClient::builder()
        .api_key(api_key)
        .build()
        .expect("Failed to build client");

    // Create a session with workbench enabled
    let session = client
        .create_session("test_user_workbench")
        .workbench(Some(true), Some(1000))
        .send()
        .await
        .expect("Failed to create session");

    // Execute COMPOSIO_REMOTE_WORKBENCH meta tool
    let result = session
        .execute_meta_tool(
            MetaToolSlug::ComposioRemoteWorkbench,
            json!({
                "code": "print('Hello from Rust SDK!')"
            }),
        )
        .await;

    match result {
        Ok(response) => {
            println!("Workbench meta tool successful");
            println!("Data: {:?}", response.data);
            println!("Log ID: {}", response.log_id);
            
            // Verify response structure
            assert!(!response.log_id.is_empty());
            
            if let Some(error) = response.error {
                println!("Warning: Tool execution returned error: {}", error);
            }
        }
        Err(e) => {
            println!("Workbench meta tool failed: {:?}", e);
            // Don't fail the test if it's a connection/auth issue
            match e {
                ComposioError::ApiError { status, .. } if status == 401 || status == 403 => {
                    println!("Skipping test due to authentication issue");
                }
                _ => panic!("Unexpected error: {:?}", e),
            }
        }
    }
}

#[tokio::test]
async fn test_execute_meta_tool_bash() {
    // This test requires a valid API key and will make real API calls
    // Skip if COMPOSIO_API_KEY is not set
    let api_key = match std::env::var("COMPOSIO_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("Skipping test: COMPOSIO_API_KEY not set");
            return;
        }
    };

    let client = ComposioClient::builder()
        .api_key(api_key)
        .build()
        .expect("Failed to build client");

    // Create a session
    let session = client
        .create_session("test_user_bash")
        .send()
        .await
        .expect("Failed to create session");

    // Execute COMPOSIO_REMOTE_BASH_TOOL meta tool
    let result = session
        .execute_meta_tool(
            MetaToolSlug::ComposioRemoteBashTool,
            json!({
                "command": "echo 'Hello from Rust SDK!'"
            }),
        )
        .await;

    match result {
        Ok(response) => {
            println!("Bash meta tool successful");
            println!("Data: {:?}", response.data);
            println!("Log ID: {}", response.log_id);
            
            // Verify response structure
            assert!(!response.log_id.is_empty());
            
            if let Some(error) = response.error {
                println!("Warning: Tool execution returned error: {}", error);
            }
        }
        Err(e) => {
            println!("Bash meta tool failed: {:?}", e);
            // Don't fail the test if it's a connection/auth issue
            match e {
                ComposioError::ApiError { status, .. } if status == 401 || status == 403 => {
                    println!("Skipping test due to authentication issue");
                }
                _ => panic!("Unexpected error: {:?}", e),
            }
        }
    }
}

#[tokio::test]
async fn test_meta_tool_error_handling() {
    // This test requires a valid API key
    let api_key = match std::env::var("COMPOSIO_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("Skipping test: COMPOSIO_API_KEY not set");
            return;
        }
    };

    let client = ComposioClient::builder()
        .api_key(api_key)
        .build()
        .expect("Failed to build client");

    // Create a session
    let session = client
        .create_session("test_user_error")
        .send()
        .await
        .expect("Failed to create session");

    // Execute meta tool with invalid arguments
    let result = session
        .execute_meta_tool(
            MetaToolSlug::ComposioSearchTools,
            json!({}), // Missing required 'query' field
        )
        .await;

    // This should either succeed with an error in the response, or fail with an API error
    match result {
        Ok(response) => {
            println!("Meta tool returned response (may contain error)");
            if let Some(error) = response.error {
                println!("Tool execution error (expected): {}", error);
                assert!(!error.is_empty());
            }
        }
        Err(e) => {
            println!("Meta tool execution failed (expected): {:?}", e);
            // Verify it's an API error
            match e {
                ComposioError::ApiError { status, message, .. } => {
                    println!("API error status: {}, message: {}", status, message);
                    assert!(status >= 400);
                }
                _ => panic!("Expected ApiError, got: {:?}", e),
            }
        }
    }
}
