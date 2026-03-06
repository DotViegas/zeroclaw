//! Integration tests for toolkit listing functionality

use composio_sdk::{ComposioClient, ComposioError};
use serde_json::json;
use wiremock::{
    matchers::{header, method, path, query_param},
    Mock, MockServer, ResponseTemplate,
};

/// Test basic toolkit listing
#[tokio::test]
async fn test_list_toolkits_basic() {
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

    // Mock toolkit listing
    Mock::given(method("GET"))
        .and(path("/tool_router/session/sess_123/toolkits"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [
                {
                    "name": "GitHub",
                    "slug": "github",
                    "enabled": true,
                    "is_no_auth": false,
                    "composio_managed_auth_schemes": ["OAUTH2"],
                    "meta": {
                        "logo": "https://example.com/github.png",
                        "description": "GitHub integration",
                        "categories": ["development"],
                        "tools_count": 50,
                        "triggers_count": 10,
                        "version": "1.0.0"
                    },
                    "connected_account": {
                        "id": "ca_123",
                        "status": "ACTIVE",
                        "created_at": "2024-01-01T00:00:00Z"
                    }
                },
                {
                    "name": "Gmail",
                    "slug": "gmail",
                    "enabled": true,
                    "is_no_auth": false,
                    "composio_managed_auth_schemes": ["OAUTH2"],
                    "meta": {
                        "logo": "https://example.com/gmail.png",
                        "description": "Gmail integration",
                        "categories": ["communication"],
                        "tools_count": 30,
                        "triggers_count": 5,
                        "version": "1.0.0"
                    },
                    "connected_account": null
                }
            ],
            "next_cursor": null,
            "total_pages": 1,
            "current_page": 1,
            "total_items": 2
        })))
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client.create_session("user_123").send().await.unwrap();

    let response = session.list_toolkits().send().await.unwrap();

    assert_eq!(response.items.len(), 2);
    assert_eq!(response.total_items, 2);
    assert_eq!(response.current_page, 1);
    assert_eq!(response.total_pages, 1);
    assert!(response.next_cursor.is_none());

    // Check first toolkit (GitHub)
    let github = &response.items[0];
    assert_eq!(github.name, "GitHub");
    assert_eq!(github.slug, "github");
    assert!(github.enabled);
    assert!(!github.is_no_auth);
    assert_eq!(github.meta.tools_count, 50);
    assert_eq!(github.meta.triggers_count, 10);
    assert!(github.connected_account.is_some());

    let github_account = github.connected_account.as_ref().unwrap();
    assert_eq!(github_account.id, "ca_123");
    assert_eq!(github_account.status, "ACTIVE");

    // Check second toolkit (Gmail)
    let gmail = &response.items[1];
    assert_eq!(gmail.name, "Gmail");
    assert_eq!(gmail.slug, "gmail");
    assert!(gmail.connected_account.is_none());
}

/// Test toolkit listing with limit parameter
#[tokio::test]
async fn test_list_toolkits_with_limit() {
    let mock_server = MockServer::start().await;

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_123",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {"user_id": "user_123"}
        })))
        .mount(&mock_server)
        .await;

    // Mock toolkit listing with limit
    Mock::given(method("GET"))
        .and(path("/tool_router/session/sess_123/toolkits"))
        .and(query_param("limit", "10"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [],
            "next_cursor": "cursor_abc",
            "total_pages": 5,
            "current_page": 1,
            "total_items": 50
        })))
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client.create_session("user_123").send().await.unwrap();

    let response = session.list_toolkits().limit(10).send().await.unwrap();

    assert_eq!(response.total_items, 50);
    assert_eq!(response.current_page, 1);
    assert_eq!(response.total_pages, 5);
    assert_eq!(response.next_cursor, Some("cursor_abc".to_string()));
}

/// Test toolkit listing with pagination cursor
#[tokio::test]
async fn test_list_toolkits_with_cursor() {
    let mock_server = MockServer::start().await;

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_123",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {"user_id": "user_123"}
        })))
        .mount(&mock_server)
        .await;

    // Mock toolkit listing with cursor
    Mock::given(method("GET"))
        .and(path("/tool_router/session/sess_123/toolkits"))
        .and(query_param("cursor", "cursor_abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [],
            "next_cursor": null,
            "total_pages": 2,
            "current_page": 2,
            "total_items": 30
        })))
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client.create_session("user_123").send().await.unwrap();

    let response = session
        .list_toolkits()
        .cursor("cursor_abc")
        .send()
        .await
        .unwrap();

    assert_eq!(response.current_page, 2);
    assert_eq!(response.total_pages, 2);
    assert!(response.next_cursor.is_none());
}

/// Test toolkit listing filtered by connection status
#[tokio::test]
async fn test_list_toolkits_is_connected() {
    let mock_server = MockServer::start().await;

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_123",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {"user_id": "user_123"}
        })))
        .mount(&mock_server)
        .await;

    // Mock toolkit listing with is_connected filter
    Mock::given(method("GET"))
        .and(path("/tool_router/session/sess_123/toolkits"))
        .and(query_param("is_connected", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [
                {
                    "name": "GitHub",
                    "slug": "github",
                    "enabled": true,
                    "is_no_auth": false,
                    "composio_managed_auth_schemes": ["OAUTH2"],
                    "meta": {
                        "logo": "https://example.com/github.png",
                        "description": "GitHub integration",
                        "categories": ["development"],
                        "tools_count": 50,
                        "triggers_count": 10,
                        "version": "1.0.0"
                    },
                    "connected_account": {
                        "id": "ca_123",
                        "status": "ACTIVE",
                        "created_at": "2024-01-01T00:00:00Z"
                    }
                }
            ],
            "next_cursor": null,
            "total_pages": 1,
            "current_page": 1,
            "total_items": 1
        })))
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client.create_session("user_123").send().await.unwrap();

    let response = session
        .list_toolkits()
        .is_connected(true)
        .send()
        .await
        .unwrap();

    assert_eq!(response.items.len(), 1);
    assert!(response.items[0].connected_account.is_some());
}

/// Test toolkit listing with search query
#[tokio::test]
async fn test_list_toolkits_search() {
    let mock_server = MockServer::start().await;

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_123",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {"user_id": "user_123"}
        })))
        .mount(&mock_server)
        .await;

    // Mock toolkit listing with search
    Mock::given(method("GET"))
        .and(path("/tool_router/session/sess_123/toolkits"))
        .and(query_param("search", "github"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [
                {
                    "name": "GitHub",
                    "slug": "github",
                    "enabled": true,
                    "is_no_auth": false,
                    "composio_managed_auth_schemes": ["OAUTH2"],
                    "meta": {
                        "logo": "https://example.com/github.png",
                        "description": "GitHub integration",
                        "categories": ["development"],
                        "tools_count": 50,
                        "triggers_count": 10,
                        "version": "1.0.0"
                    },
                    "connected_account": null
                }
            ],
            "next_cursor": null,
            "total_pages": 1,
            "current_page": 1,
            "total_items": 1
        })))
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client.create_session("user_123").send().await.unwrap();

    let response = session
        .list_toolkits()
        .search("github")
        .send()
        .await
        .unwrap();

    assert_eq!(response.items.len(), 1);
    assert_eq!(response.items[0].slug, "github");
}

/// Test toolkit listing with specific toolkit slugs
#[tokio::test]
async fn test_list_toolkits_with_slugs() {
    let mock_server = MockServer::start().await;

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_123",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {"user_id": "user_123"}
        })))
        .mount(&mock_server)
        .await;

    // Mock toolkit listing with specific slugs
    Mock::given(method("GET"))
        .and(path("/tool_router/session/sess_123/toolkits"))
        .and(query_param("toolkits", "github,gmail"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [
                {
                    "name": "GitHub",
                    "slug": "github",
                    "enabled": true,
                    "is_no_auth": false,
                    "composio_managed_auth_schemes": ["OAUTH2"],
                    "meta": {
                        "logo": "https://example.com/github.png",
                        "description": "GitHub integration",
                        "categories": ["development"],
                        "tools_count": 50,
                        "triggers_count": 10,
                        "version": "1.0.0"
                    },
                    "connected_account": null
                },
                {
                    "name": "Gmail",
                    "slug": "gmail",
                    "enabled": true,
                    "is_no_auth": false,
                    "composio_managed_auth_schemes": ["OAUTH2"],
                    "meta": {
                        "logo": "https://example.com/gmail.png",
                        "description": "Gmail integration",
                        "categories": ["communication"],
                        "tools_count": 30,
                        "triggers_count": 5,
                        "version": "1.0.0"
                    },
                    "connected_account": null
                }
            ],
            "next_cursor": null,
            "total_pages": 1,
            "current_page": 1,
            "total_items": 2
        })))
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client.create_session("user_123").send().await.unwrap();

    let response = session
        .list_toolkits()
        .toolkits(vec!["github", "gmail"])
        .send()
        .await
        .unwrap();

    assert_eq!(response.items.len(), 2);
    assert_eq!(response.items[0].slug, "github");
    assert_eq!(response.items[1].slug, "gmail");
}

/// Test toolkit listing with multiple filters combined
#[tokio::test]
async fn test_list_toolkits_combined_filters() {
    let mock_server = MockServer::start().await;

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_123",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {"user_id": "user_123"}
        })))
        .mount(&mock_server)
        .await;

    // Mock toolkit listing with combined filters
    Mock::given(method("GET"))
        .and(path("/tool_router/session/sess_123/toolkits"))
        .and(query_param("limit", "5"))
        .and(query_param("is_connected", "true"))
        .and(query_param("search", "git"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [
                {
                    "name": "GitHub",
                    "slug": "github",
                    "enabled": true,
                    "is_no_auth": false,
                    "composio_managed_auth_schemes": ["OAUTH2"],
                    "meta": {
                        "logo": "https://example.com/github.png",
                        "description": "GitHub integration",
                        "categories": ["development"],
                        "tools_count": 50,
                        "triggers_count": 10,
                        "version": "1.0.0"
                    },
                    "connected_account": {
                        "id": "ca_123",
                        "status": "ACTIVE",
                        "created_at": "2024-01-01T00:00:00Z"
                    }
                }
            ],
            "next_cursor": null,
            "total_pages": 1,
            "current_page": 1,
            "total_items": 1
        })))
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client.create_session("user_123").send().await.unwrap();

    let response = session
        .list_toolkits()
        .limit(5)
        .is_connected(true)
        .search("git")
        .send()
        .await
        .unwrap();

    assert_eq!(response.items.len(), 1);
    assert_eq!(response.items[0].slug, "github");
    assert!(response.items[0].connected_account.is_some());
}

/// Test toolkit listing error handling
#[tokio::test]
async fn test_list_toolkits_error() {
    let mock_server = MockServer::start().await;

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_123",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {"user_id": "user_123"}
        })))
        .mount(&mock_server)
        .await;

    // Mock toolkit listing error
    Mock::given(method("GET"))
        .and(path("/tool_router/session/sess_123/toolkits"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "message": "Session not found",
            "status": 404,
            "code": "SESSION_NOT_FOUND"
        })))
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let session = client.create_session("user_123").send().await.unwrap();

    let result = session.list_toolkits().send().await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ComposioError::ApiError { status, message, .. } => {
            assert_eq!(status, 404);
            assert!(message.contains("Session not found"));
        }
        _ => panic!("Expected ApiError"),
    }
}

/// Test toolkit listing with retry on transient failure
#[tokio::test]
async fn test_list_toolkits_retry() {
    let mock_server = MockServer::start().await;

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_123",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {"user_id": "user_123"}
        })))
        .mount(&mock_server)
        .await;

    // First request fails with 503
    Mock::given(method("GET"))
        .and(path("/tool_router/session/sess_123/toolkits"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second request succeeds
    Mock::given(method("GET"))
        .and(path("/tool_router/session/sess_123/toolkits"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [],
            "next_cursor": null,
            "total_pages": 1,
            "current_page": 1,
            "total_items": 0
        })))
        .mount(&mock_server)
        .await;

    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .max_retries(3)
        .build()
        .unwrap();

    let session = client.create_session("user_123").send().await.unwrap();

    // Should succeed after retry
    let response = session.list_toolkits().send().await.unwrap();

    assert_eq!(response.items.len(), 0);
}
