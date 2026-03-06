//! Integration tests for session creation

use composio_sdk::client::ComposioClient;
use composio_sdk::error::ComposioError;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_session_creation_success() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Mock successful session creation response
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "session_id": "sess_abc123",
            "mcp": {
                "url": "https://mcp.composio.dev/sess_abc123"
            },
            "tool_router_tools": [],
            "config": {
                "user_id": "user_123"
            }
        })))
        .mount(&mock_server)
        .await;

    // Create client with mock server URL
    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    // Create session
    let session = client.create_session("user_123").send().await.unwrap();

    // Verify session details
    assert_eq!(session.session_id(), "sess_abc123");
    assert_eq!(
        session.mcp_url(),
        "https://mcp.composio.dev/sess_abc123"
    );
}

#[tokio::test]
async fn test_session_creation_with_toolkits() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "session_id": "sess_xyz789",
            "mcp": {
                "url": "https://mcp.composio.dev/sess_xyz789"
            },
            "tool_router_tools": [],
            "config": {
                "user_id": "user_456",
                "toolkits": ["github", "gmail"]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client
        .create_session("user_456")
        .toolkits(vec!["github", "gmail"])
        .send()
        .await
        .unwrap();

    assert_eq!(session.session_id(), "sess_xyz789");
}

#[tokio::test]
async fn test_session_creation_with_auth_config() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "session_id": "sess_custom",
            "mcp": {
                "url": "https://mcp.composio.dev/sess_custom"
            },
            "tool_router_tools": [],
            "config": {
                "user_id": "user_789",
                "auth_configs": {
                    "github": "ac_custom_oauth"
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client
        .create_session("user_789")
        .auth_config("github", "ac_custom_oauth")
        .send()
        .await
        .unwrap();

    assert_eq!(session.session_id(), "sess_custom");
}

#[tokio::test]
async fn test_session_creation_handles_401_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "message": "Invalid API key",
            "status": 401,
            "code": "UNAUTHORIZED",
            "request_id": "req_error_123"
        })))
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("invalid_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let result = client.create_session("user_123").send().await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ComposioError::ApiError {
            status,
            message,
            code,
            ..
        } => {
            assert_eq!(status, 401);
            assert_eq!(message, "Invalid API key");
            assert_eq!(code, Some("UNAUTHORIZED".to_string()));
        }
        _ => panic!("Expected ApiError"),
    }
}

#[tokio::test]
async fn test_session_creation_handles_400_validation_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "message": "Validation failed",
            "status": 400,
            "code": "VALIDATION_ERROR",
            "errors": [
                {
                    "field": "user_id",
                    "message": "User ID is required"
                }
            ],
            "suggested_fix": "Provide a valid user_id"
        })))
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let result = client.create_session("").send().await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ComposioError::ApiError {
            status,
            message,
            suggested_fix,
            errors,
            ..
        } => {
            assert_eq!(status, 400);
            assert_eq!(message, "Validation failed");
            assert_eq!(suggested_fix, Some("Provide a valid user_id".to_string()));
            assert!(errors.is_some());
        }
        _ => panic!("Expected ApiError"),
    }
}

#[tokio::test]
async fn test_session_creation_retries_on_503() {
    let mock_server = MockServer::start().await;

    // First request fails with 503
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .respond_with(ResponseTemplate::new(503).set_body_json(serde_json::json!({
            "message": "Service temporarily unavailable",
            "status": 503
        })))
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;

    // Third request succeeds
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "session_id": "sess_retry_success",
            "mcp": {
                "url": "https://mcp.composio.dev/sess_retry_success"
            },
            "tool_router_tools": [],
            "config": {
                "user_id": "user_retry"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .max_retries(3)
        .build()
        .unwrap();

    let session = client.create_session("user_retry").send().await.unwrap();

    assert_eq!(session.session_id(), "sess_retry_success");
}

#[tokio::test]
async fn test_session_creation_does_not_retry_on_404() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "message": "Endpoint not found",
            "status": 404
        })))
        .expect(1) // Should only be called once (no retries)
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .max_retries(3)
        .build()
        .unwrap();

    let result = client.create_session("user_123").send().await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ComposioError::ApiError { status, .. } => {
            assert_eq!(status, 404);
        }
        _ => panic!("Expected ApiError"),
    }
}

#[tokio::test]
async fn test_session_creation_with_manage_connections() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "session_id": "sess_manage",
            "mcp": {
                "url": "https://mcp.composio.dev/sess_manage"
            },
            "tool_router_tools": [],
            "config": {
                "user_id": "user_manage",
                "manage_connections": false
            }
        })))
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client
        .create_session("user_manage")
        .manage_connections(false)
        .send()
        .await
        .unwrap();

    assert_eq!(session.session_id(), "sess_manage");
}

#[tokio::test]
async fn test_session_creation_with_connected_account() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "session_id": "sess_account",
            "mcp": {
                "url": "https://mcp.composio.dev/sess_account"
            },
            "tool_router_tools": [],
            "config": {
                "user_id": "user_account",
                "connected_accounts": {
                    "gmail": "ca_work_gmail"
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client
        .create_session("user_account")
        .connected_account("gmail", "ca_work_gmail")
        .send()
        .await
        .unwrap();

    assert_eq!(session.session_id(), "sess_account");
}

#[tokio::test]
async fn test_session_creation_with_all_options() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "session_id": "sess_full",
            "mcp": {
                "url": "https://mcp.composio.dev/sess_full"
            },
            "tool_router_tools": [],
            "config": {
                "user_id": "user_full",
                "toolkits": ["github", "gmail"],
                "auth_configs": {
                    "github": "ac_custom"
                },
                "connected_accounts": {
                    "gmail": "ca_work"
                },
                "manage_connections": true
            }
        })))
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client
        .create_session("user_full")
        .toolkits(vec!["github", "gmail"])
        .auth_config("github", "ac_custom")
        .connected_account("gmail", "ca_work")
        .manage_connections(true)
        .send()
        .await
        .unwrap();

    assert_eq!(session.session_id(), "sess_full");
    assert_eq!(session.mcp_url(), "https://mcp.composio.dev/sess_full");
}
