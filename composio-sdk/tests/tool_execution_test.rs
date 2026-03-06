//! Integration tests for tool execution

use composio_sdk::client::ComposioClient;
use composio_sdk::error::ComposioError;
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_execute_tool_success() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_123",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {
                "user_id": "user_123"
            }
        })))
        .mount(&mock_server)
        .await;

    // Mock tool execution
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_123/execute"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "issue_number": 42,
                "url": "https://github.com/composio/composio/issues/42"
            },
            "error": null,
            "log_id": "log_abc123"
        })))
        .mount(&mock_server)
        .await;

    // Create client and session
    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client.create_session("user_123").send().await.unwrap();

    // Execute tool
    let result = session
        .execute_tool(
            "GITHUB_CREATE_ISSUE",
            json!({
                "owner": "composio",
                "repo": "composio",
                "title": "Test issue",
                "body": "Created via Rust SDK"
            }),
        )
        .await
        .unwrap();

    // Verify response
    assert_eq!(result.log_id, "log_abc123");
    assert!(result.error.is_none());
    assert_eq!(result.data["issue_number"], 42);
    assert_eq!(
        result.data["url"],
        "https://github.com/composio/composio/issues/42"
    );
}

#[tokio::test]
async fn test_execute_tool_with_error_in_response() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_123",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {
                "user_id": "user_123"
            }
        })))
        .mount(&mock_server)
        .await;

    // Mock tool execution with error
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_123/execute"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {},
            "error": "Repository not found",
            "log_id": "log_error123"
        })))
        .mount(&mock_server)
        .await;

    // Create client and session
    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client.create_session("user_123").send().await.unwrap();

    // Execute tool
    let result = session
        .execute_tool(
            "GITHUB_GET_REPOS",
            json!({
                "owner": "nonexistent"
            }),
        )
        .await
        .unwrap();

    // Verify response contains error
    assert_eq!(result.log_id, "log_error123");
    assert_eq!(result.error, Some("Repository not found".to_string()));
}

#[tokio::test]
async fn test_execute_tool_http_error_404() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_123",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {
                "user_id": "user_123"
            }
        })))
        .mount(&mock_server)
        .await;

    // Mock tool execution with 404 error
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_123/execute"))
        .and(header("x-api-key", "test_key"))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(json!({
                "message": "Tool not found",
                "status": 404,
                "code": "TOOL_NOT_FOUND",
                "slug": "tool-not-found",
                "request_id": "req_123",
                "suggested_fix": "Check the tool slug and ensure it's available in this session"
            })),
        )
        .mount(&mock_server)
        .await;

    // Create client and session
    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client.create_session("user_123").send().await.unwrap();

    // Execute tool
    let result = session
        .execute_tool("INVALID_TOOL", json!({}))
        .await;

    // Verify error
    assert!(result.is_err());
    match result.unwrap_err() {
        ComposioError::ApiError {
            status,
            message,
            code,
            slug,
            request_id,
            suggested_fix,
            ..
        } => {
            assert_eq!(status, 404);
            assert_eq!(message, "Tool not found");
            assert_eq!(code, Some("TOOL_NOT_FOUND".to_string()));
            assert_eq!(slug, Some("tool-not-found".to_string()));
            assert_eq!(request_id, Some("req_123".to_string()));
            assert!(suggested_fix.is_some());
        }
        _ => panic!("Expected ApiError"),
    }
}

#[tokio::test]
async fn test_execute_tool_http_error_401() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_123",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {
                "user_id": "user_123"
            }
        })))
        .mount(&mock_server)
        .await;

    // Mock tool execution with 401 error (no connected account)
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_123/execute"))
        .and(header("x-api-key", "test_key"))
        .respond_with(
            ResponseTemplate::new(401).set_body_json(json!({
                "message": "No connected account found for this toolkit",
                "status": 401,
                "code": "NO_CONNECTED_ACCOUNT",
                "slug": "no-connected-account",
                "request_id": "req_456",
                "suggested_fix": "User needs to authenticate with the toolkit first"
            })),
        )
        .mount(&mock_server)
        .await;

    // Create client and session
    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client.create_session("user_123").send().await.unwrap();

    // Execute tool
    let result = session
        .execute_tool("GITHUB_CREATE_ISSUE", json!({}))
        .await;

    // Verify error
    assert!(result.is_err());
    match result.unwrap_err() {
        ComposioError::ApiError {
            status,
            message,
            code,
            ..
        } => {
            assert_eq!(status, 401);
            assert_eq!(message, "No connected account found for this toolkit");
            assert_eq!(code, Some("NO_CONNECTED_ACCOUNT".to_string()));
        }
        _ => panic!("Expected ApiError"),
    }
}

#[tokio::test]
async fn test_execute_tool_with_retry_on_503() {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    // Start mock server
    let mock_server = MockServer::start().await;

    // Counter for tracking attempts
    let attempt_counter = Arc::new(AtomicU32::new(0));
    let counter_clone = attempt_counter.clone();

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_123",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {
                "user_id": "user_123"
            }
        })))
        .mount(&mock_server)
        .await;

    // Mock tool execution that fails twice then succeeds
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_123/execute"))
        .and(header("x-api-key", "test_key"))
        .respond_with(move |_req: &wiremock::Request| {
            let count = counter_clone.fetch_add(1, Ordering::SeqCst);
            if count < 2 {
                // First two attempts fail with 503
                ResponseTemplate::new(503).set_body_json(json!({
                    "message": "Service temporarily unavailable",
                    "status": 503
                }))
            } else {
                // Third attempt succeeds
                ResponseTemplate::new(200).set_body_json(json!({
                    "data": {"success": true},
                    "error": null,
                    "log_id": "log_retry123"
                }))
            }
        })
        .mount(&mock_server)
        .await;

    // Create client with retry enabled
    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .max_retries(3)
        .build()
        .unwrap();

    let session = client.create_session("user_123").send().await.unwrap();

    // Execute tool - should succeed after retries
    let result = session
        .execute_tool("GITHUB_GET_REPOS", json!({}))
        .await
        .unwrap();

    // Verify it succeeded after retries
    assert_eq!(result.log_id, "log_retry123");
    assert!(result.error.is_none());
    assert_eq!(attempt_counter.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_execute_tool_no_retry_on_400() {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    // Start mock server
    let mock_server = MockServer::start().await;

    // Counter for tracking attempts
    let attempt_counter = Arc::new(AtomicU32::new(0));
    let counter_clone = attempt_counter.clone();

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_123",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {
                "user_id": "user_123"
            }
        })))
        .mount(&mock_server)
        .await;

    // Mock tool execution that always fails with 400
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_123/execute"))
        .and(header("x-api-key", "test_key"))
        .respond_with(move |_req: &wiremock::Request| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(400).set_body_json(json!({
                "message": "Invalid arguments",
                "status": 400,
                "code": "INVALID_ARGUMENTS"
            }))
        })
        .mount(&mock_server)
        .await;

    // Create client with retry enabled
    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .max_retries(3)
        .build()
        .unwrap();

    let session = client.create_session("user_123").send().await.unwrap();

    // Execute tool - should fail immediately without retry
    let result = session
        .execute_tool("GITHUB_CREATE_ISSUE", json!({}))
        .await;

    // Verify it failed without retrying
    assert!(result.is_err());
    assert_eq!(attempt_counter.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_execute_tool_with_empty_arguments() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_123",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {
                "user_id": "user_123"
            }
        })))
        .mount(&mock_server)
        .await;

    // Mock tool execution with empty arguments
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_123/execute"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {"repos": []},
            "error": null,
            "log_id": "log_empty123"
        })))
        .mount(&mock_server)
        .await;

    // Create client and session
    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client.create_session("user_123").send().await.unwrap();

    // Execute tool with empty JSON object
    let result = session
        .execute_tool("GITHUB_LIST_REPOS", json!({}))
        .await
        .unwrap();

    // Verify response
    assert_eq!(result.log_id, "log_empty123");
    assert!(result.error.is_none());
}

#[tokio::test]
async fn test_execute_tool_accepts_string_and_str() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_123",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {
                "user_id": "user_123"
            }
        })))
        .mount(&mock_server)
        .await;

    // Mock tool execution
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_123/execute"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {},
            "error": null,
            "log_id": "log_123"
        })))
        .expect(2) // Expect two calls
        .mount(&mock_server)
        .await;

    // Create client and session
    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client.create_session("user_123").send().await.unwrap();

    // Test with &str
    let _result1 = session
        .execute_tool("GITHUB_GET_REPOS", json!({}))
        .await
        .unwrap();

    // Test with String
    let tool_slug = "GITHUB_GET_REPOS".to_string();
    let _result2 = session
        .execute_tool(tool_slug, json!({}))
        .await
        .unwrap();
}
