//! Comprehensive error handling integration tests
//!
//! This test suite validates that the SDK properly handles all HTTP error codes
//! and converts them to appropriate ComposioError variants with full context.

use composio_sdk::{ComposioClient, ComposioError, MetaToolSlug};
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper function to create a mock session
async fn create_mock_session(mock_server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_test",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {
                "user_id": "user_test"
            }
        })))
        .mount(mock_server)
        .await;
}

#[tokio::test]
async fn test_error_400_bad_request() {
    let mock_server = MockServer::start().await;
    create_mock_session(&mock_server).await;

    // Mock 400 error response
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_test/execute"))
        .respond_with(
            ResponseTemplate::new(400).set_body_json(json!({
                "message": "Invalid arguments provided",
                "status": 400,
                "code": "INVALID_ARGUMENTS",
                "slug": "invalid-arguments",
                "request_id": "req_400_test",
                "suggested_fix": "Check the tool's input schema and provide valid arguments",
                "errors": [
                    {
                        "field": "owner",
                        "message": "Owner field is required"
                    }
                ]
            })),
        )
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client.create_session("user_test").send().await.unwrap();

    let result = session
        .execute_tool("GITHUB_CREATE_ISSUE", json!({}))
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ComposioError::ApiError {
            status,
            message,
            code,
            slug,
            request_id,
            suggested_fix,
            errors,
        } => {
            assert_eq!(status, 400);
            assert_eq!(message, "Invalid arguments provided");
            assert_eq!(code, Some("INVALID_ARGUMENTS".to_string()));
            assert_eq!(slug, Some("invalid-arguments".to_string()));
            assert_eq!(request_id, Some("req_400_test".to_string()));
            assert_eq!(
                suggested_fix,
                Some("Check the tool's input schema and provide valid arguments".to_string())
            );
            assert!(errors.is_some());
            let error_details = errors.unwrap();
            assert_eq!(error_details.len(), 1);
            assert_eq!(error_details[0].field, Some("owner".to_string()));
            assert_eq!(error_details[0].message, "Owner field is required");
        }
        _ => panic!("Expected ApiError"),
    }
}

#[tokio::test]
async fn test_error_401_unauthorized() {
    let mock_server = MockServer::start().await;
    create_mock_session(&mock_server).await;

    // Mock 401 error response
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_test/execute"))
        .respond_with(
            ResponseTemplate::new(401).set_body_json(json!({
                "message": "No connected account found for this toolkit",
                "status": 401,
                "code": "NO_CONNECTED_ACCOUNT",
                "slug": "no-connected-account",
                "request_id": "req_401_test",
                "suggested_fix": "User needs to authenticate with GitHub first. Use COMPOSIO_MANAGE_CONNECTIONS to create a connection."
            })),
        )
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client.create_session("user_test").send().await.unwrap();

    let result = session
        .execute_tool("GITHUB_CREATE_ISSUE", json!({}))
        .await;

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
            assert_eq!(status, 401);
            assert_eq!(message, "No connected account found for this toolkit");
            assert_eq!(code, Some("NO_CONNECTED_ACCOUNT".to_string()));
            assert_eq!(slug, Some("no-connected-account".to_string()));
            assert_eq!(request_id, Some("req_401_test".to_string()));
            assert!(suggested_fix.is_some());
            assert!(suggested_fix.unwrap().contains("authenticate"));
        }
        _ => panic!("Expected ApiError"),
    }
}

#[tokio::test]
async fn test_error_403_forbidden() {
    let mock_server = MockServer::start().await;
    create_mock_session(&mock_server).await;

    // Mock 403 error response
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_test/execute"))
        .respond_with(
            ResponseTemplate::new(403).set_body_json(json!({
                "message": "Insufficient permissions to perform this action",
                "status": 403,
                "code": "INSUFFICIENT_PERMISSIONS",
                "slug": "insufficient-permissions",
                "request_id": "req_403_test",
                "suggested_fix": "The connected account needs additional OAuth scopes. Re-authenticate with required permissions."
            })),
        )
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client.create_session("user_test").send().await.unwrap();

    let result = session
        .execute_tool("GITHUB_DELETE_REPO", json!({}))
        .await;

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
            assert_eq!(status, 403);
            assert_eq!(message, "Insufficient permissions to perform this action");
            assert_eq!(code, Some("INSUFFICIENT_PERMISSIONS".to_string()));
            assert_eq!(slug, Some("insufficient-permissions".to_string()));
            assert_eq!(request_id, Some("req_403_test".to_string()));
            assert!(suggested_fix.is_some());
            assert!(suggested_fix.unwrap().contains("permissions"));
        }
        _ => panic!("Expected ApiError"),
    }
}

#[tokio::test]
async fn test_error_404_not_found() {
    let mock_server = MockServer::start().await;
    create_mock_session(&mock_server).await;

    // Mock 404 error response
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_test/execute"))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(json!({
                "message": "Tool not found in this session",
                "status": 404,
                "code": "TOOL_NOT_FOUND",
                "slug": "tool-not-found",
                "request_id": "req_404_test",
                "suggested_fix": "Verify the tool slug is correct and the toolkit is enabled in this session"
            })),
        )
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client.create_session("user_test").send().await.unwrap();

    let result = session
        .execute_tool("INVALID_TOOL_SLUG", json!({}))
        .await;

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
            assert_eq!(message, "Tool not found in this session");
            assert_eq!(code, Some("TOOL_NOT_FOUND".to_string()));
            assert_eq!(slug, Some("tool-not-found".to_string()));
            assert_eq!(request_id, Some("req_404_test".to_string()));
            assert!(suggested_fix.is_some());
            assert!(suggested_fix.unwrap().contains("toolkit"));
        }
        _ => panic!("Expected ApiError"),
    }
}

#[tokio::test]
async fn test_error_500_internal_server_error() {
    let mock_server = MockServer::start().await;
    create_mock_session(&mock_server).await;

    // Mock 500 error response
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_test/execute"))
        .respond_with(
            ResponseTemplate::new(500).set_body_json(json!({
                "message": "Internal server error occurred",
                "status": 500,
                "code": "INTERNAL_SERVER_ERROR",
                "slug": "internal-server-error",
                "request_id": "req_500_test",
                "suggested_fix": "This is a temporary issue. Please retry your request."
            })),
        )
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .max_retries(0) // Disable retries for this test
        .build()
        .unwrap();

    let session = client.create_session("user_test").send().await.unwrap();

    let result = session
        .execute_tool("GITHUB_CREATE_ISSUE", json!({}))
        .await;

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
            assert_eq!(status, 500);
            assert_eq!(message, "Internal server error occurred");
            assert_eq!(code, Some("INTERNAL_SERVER_ERROR".to_string()));
            assert_eq!(slug, Some("internal-server-error".to_string()));
            assert_eq!(request_id, Some("req_500_test".to_string()));
            assert!(suggested_fix.is_some());
            assert!(suggested_fix.unwrap().contains("retry"));
        }
        _ => panic!("Expected ApiError"),
    }
}

#[tokio::test]
async fn test_meta_tool_error_400() {
    let mock_server = MockServer::start().await;
    create_mock_session(&mock_server).await;

    // Mock 400 error for meta tool
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_test/execute_meta"))
        .respond_with(
            ResponseTemplate::new(400).set_body_json(json!({
                "message": "Missing required field: query",
                "status": 400,
                "code": "MISSING_REQUIRED_FIELD",
                "slug": "missing-required-field",
                "request_id": "req_meta_400",
                "suggested_fix": "Provide the 'query' field in arguments"
            })),
        )
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client.create_session("user_test").send().await.unwrap();

    let result = session
        .execute_meta_tool(MetaToolSlug::ComposioSearchTools, json!({}))
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ComposioError::ApiError {
            status,
            message,
            request_id,
            suggested_fix,
            ..
        } => {
            assert_eq!(status, 400);
            assert!(message.contains("query"));
            assert_eq!(request_id, Some("req_meta_400".to_string()));
            assert!(suggested_fix.is_some());
        }
        _ => panic!("Expected ApiError"),
    }
}

#[tokio::test]
async fn test_meta_tool_error_404() {
    let mock_server = MockServer::start().await;
    create_mock_session(&mock_server).await;

    // Mock 404 error for meta tool
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_test/execute_meta"))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(json!({
                "message": "Session not found",
                "status": 404,
                "code": "SESSION_NOT_FOUND",
                "slug": "session-not-found",
                "request_id": "req_meta_404",
                "suggested_fix": "Create a new session or verify the session ID"
            })),
        )
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client.create_session("user_test").send().await.unwrap();

    let result = session
        .execute_meta_tool(
            MetaToolSlug::ComposioSearchTools,
            json!({"query": "test"}),
        )
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ComposioError::ApiError {
            status,
            message,
            request_id,
            ..
        } => {
            assert_eq!(status, 404);
            assert!(message.contains("Session"));
            assert_eq!(request_id, Some("req_meta_404".to_string()));
        }
        _ => panic!("Expected ApiError"),
    }
}

#[tokio::test]
async fn test_error_without_optional_fields() {
    let mock_server = MockServer::start().await;
    create_mock_session(&mock_server).await;

    // Mock error response with minimal fields
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_test/execute"))
        .respond_with(
            ResponseTemplate::new(500).set_body_json(json!({
                "message": "Something went wrong",
                "status": 500
            })),
        )
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .max_retries(0)
        .build()
        .unwrap();

    let session = client.create_session("user_test").send().await.unwrap();

    let result = session
        .execute_tool("GITHUB_CREATE_ISSUE", json!({}))
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ComposioError::ApiError {
            status,
            message,
            code,
            slug,
            request_id,
            suggested_fix,
            errors,
        } => {
            assert_eq!(status, 500);
            assert_eq!(message, "Something went wrong");
            assert!(code.is_none());
            assert!(slug.is_none());
            assert!(request_id.is_none());
            assert!(suggested_fix.is_none());
            assert!(errors.is_none());
        }
        _ => panic!("Expected ApiError"),
    }
}

#[tokio::test]
async fn test_error_with_malformed_json_response() {
    let mock_server = MockServer::start().await;
    create_mock_session(&mock_server).await;

    // Mock error response with invalid JSON
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_test/execute"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .max_retries(0)
        .build()
        .unwrap();

    let session = client.create_session("user_test").send().await.unwrap();

    let result = session
        .execute_tool("GITHUB_CREATE_ISSUE", json!({}))
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ComposioError::ApiError {
            status,
            message,
            code,
            slug,
            request_id,
            suggested_fix,
            errors,
        } => {
            assert_eq!(status, 500);
            assert_eq!(message, "HTTP error 500");
            assert!(code.is_none());
            assert!(slug.is_none());
            assert!(request_id.is_none());
            assert!(suggested_fix.is_none());
            assert!(errors.is_none());
        }
        _ => panic!("Expected ApiError"),
    }
}
