// Integration tests for authentication link creation

use composio_sdk::ComposioClient;
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_create_auth_link_success() {
    let mock_server = MockServer::start().await;

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_123",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {"user_id": "user_123"}
        })))
        .mount(&mock_server)
        .await;

    // Mock auth link creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_123/link"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "link_token": "lt_abc123",
            "redirect_url": "https://backend.composio.dev/auth/github?token=lt_abc123",
            "connected_account_id": null
        })))
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client.create_session("user_123").send().await.unwrap();

    let link = session.create_auth_link("github", None).await.unwrap();

    assert_eq!(link.link_token, "lt_abc123");
    assert_eq!(
        link.redirect_url,
        "https://backend.composio.dev/auth/github?token=lt_abc123"
    );
    assert_eq!(link.connected_account_id, None);
}

#[tokio::test]
async fn test_create_auth_link_with_callback() {
    let mock_server = MockServer::start().await;

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_456",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {"user_id": "user_456"}
        })))
        .mount(&mock_server)
        .await;

    // Mock auth link creation with callback
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_456/link"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "link_token": "lt_xyz789",
            "redirect_url": "https://backend.composio.dev/auth/gmail?token=lt_xyz789&callback=https%3A%2F%2Fexample.com%2Fcallback",
            "connected_account_id": null
        })))
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client.create_session("user_456").send().await.unwrap();

    let link = session
        .create_auth_link("gmail", Some("https://example.com/callback".to_string()))
        .await
        .unwrap();

    assert_eq!(link.link_token, "lt_xyz789");
    assert!(link.redirect_url.contains("callback="));
    assert_eq!(link.connected_account_id, None);
}

#[tokio::test]
async fn test_create_auth_link_invalid_toolkit() {
    let mock_server = MockServer::start().await;

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_789",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {"user_id": "user_789"}
        })))
        .mount(&mock_server)
        .await;

    // Mock auth link creation with invalid toolkit (400 error)
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_789/link"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "message": "Invalid toolkit slug: invalid_toolkit",
            "code": "INVALID_TOOLKIT",
            "slug": "invalid_toolkit_error",
            "status": 400,
            "request_id": "req_123",
            "suggested_fix": "Use a valid toolkit slug like 'github', 'gmail', or 'slack'",
            "errors": []
        })))
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client.create_session("user_789").send().await.unwrap();

    let result = session.create_auth_link("invalid_toolkit", None).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    match error {
        composio_sdk::ComposioError::ApiError {
            status,
            message,
            code,
            ..
        } => {
            assert_eq!(status, 400);
            assert!(message.contains("Invalid toolkit"));
            assert_eq!(code, Some("INVALID_TOOLKIT".to_string()));
        }
        _ => panic!("Expected ApiError, got {:?}", error),
    }
}

#[tokio::test]
async fn test_create_auth_link_existing_connection() {
    let mock_server = MockServer::start().await;

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_existing",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {"user_id": "user_existing"}
        })))
        .mount(&mock_server)
        .await;

    // Mock auth link creation with existing connection (400 error)
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_existing/link"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "message": "Connected account already exists for toolkit: github",
            "code": "EXISTING_CONNECTION",
            "slug": "existing_connection_error",
            "status": 400,
            "request_id": "req_456",
            "suggested_fix": "Use the existing connected account or disconnect it first",
            "errors": []
        })))
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client
        .create_session("user_existing")
        .send()
        .await
        .unwrap();

    let result = session.create_auth_link("github", None).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    match error {
        composio_sdk::ComposioError::ApiError {
            status,
            message,
            code,
            ..
        } => {
            assert_eq!(status, 400);
            assert!(message.contains("already exists"));
            assert_eq!(code, Some("EXISTING_CONNECTION".to_string()));
        }
        _ => panic!("Expected ApiError, got {:?}", error),
    }
}

#[tokio::test]
async fn test_create_auth_link_with_retry() {
    let mock_server = MockServer::start().await;

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_retry",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {"user_id": "user_retry"}
        })))
        .mount(&mock_server)
        .await;

    // First request fails with 503 (retryable)
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_retry/link"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second request succeeds
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_retry/link"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "link_token": "lt_retry123",
            "redirect_url": "https://backend.composio.dev/auth/slack?token=lt_retry123",
            "connected_account_id": null
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

    let link = session.create_auth_link("slack", None).await.unwrap();

    assert_eq!(link.link_token, "lt_retry123");
    assert!(link.redirect_url.contains("slack"));
}

#[tokio::test]
async fn test_create_auth_link_with_existing_account_id() {
    let mock_server = MockServer::start().await;

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_with_account",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {"user_id": "user_with_account"}
        })))
        .mount(&mock_server)
        .await;

    // Mock auth link creation that returns existing connected_account_id
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_with_account/link"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "link_token": "lt_existing",
            "redirect_url": "https://backend.composio.dev/auth/github?token=lt_existing",
            "connected_account_id": "ca_existing123"
        })))
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client
        .create_session("user_with_account")
        .send()
        .await
        .unwrap();

    let link = session.create_auth_link("github", None).await.unwrap();

    assert_eq!(link.link_token, "lt_existing");
    assert_eq!(
        link.connected_account_id,
        Some("ca_existing123".to_string())
    );
}
