//! Integration tests for meta tools schema retrieval

use composio_sdk::{ComposioClient, ComposioError};
use serde_json::json;
use wiremock::{
    matchers::{header, method, path},
    Mock, MockServer, ResponseTemplate,
};

#[tokio::test]
async fn test_get_meta_tools_success() {
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

    // Mock get meta tools endpoint
    Mock::given(method("GET"))
        .and(path("/tool_router/session/sess_123/tools"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "slug": "COMPOSIO_SEARCH_TOOLS",
                "name": "Search Tools",
                "description": "Discover relevant tools across 1000+ apps",
                "toolkit": "composio",
                "input_parameters": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query"
                        }
                    },
                    "required": ["query"]
                },
                "output_parameters": {
                    "type": "object",
                    "properties": {
                        "tools": {
                            "type": "array",
                            "items": {"type": "object"}
                        }
                    }
                },
                "scopes": [],
                "tags": ["meta"],
                "version": "1.0.0",
                "available_versions": ["1.0.0"],
                "is_deprecated": false,
                "no_auth": true
            },
            {
                "slug": "COMPOSIO_MULTI_EXECUTE_TOOL",
                "name": "Multi Execute Tool",
                "description": "Execute up to 20 tools in parallel",
                "toolkit": "composio",
                "input_parameters": {
                    "type": "object",
                    "properties": {
                        "tools": {
                            "type": "array",
                            "description": "Array of tools to execute"
                        }
                    },
                    "required": ["tools"]
                },
                "output_parameters": {
                    "type": "object",
                    "properties": {
                        "results": {
                            "type": "array",
                            "items": {"type": "object"}
                        }
                    }
                },
                "scopes": [],
                "tags": ["meta"],
                "version": "1.0.0",
                "available_versions": ["1.0.0"],
                "is_deprecated": false,
                "no_auth": true
            },
            {
                "slug": "COMPOSIO_MANAGE_CONNECTIONS",
                "name": "Manage Connections",
                "description": "Handle OAuth and API key authentication",
                "toolkit": "composio",
                "input_parameters": {
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["create", "list", "delete"]
                        }
                    },
                    "required": ["action"]
                },
                "output_parameters": {
                    "type": "object"
                },
                "scopes": [],
                "tags": ["meta"],
                "version": "1.0.0",
                "available_versions": ["1.0.0"],
                "is_deprecated": false,
                "no_auth": true
            },
            {
                "slug": "COMPOSIO_REMOTE_WORKBENCH",
                "name": "Remote Workbench",
                "description": "Run Python code in persistent sandbox",
                "toolkit": "composio",
                "input_parameters": {
                    "type": "object",
                    "properties": {
                        "code": {
                            "type": "string",
                            "description": "Python code to execute"
                        }
                    },
                    "required": ["code"]
                },
                "output_parameters": {
                    "type": "object",
                    "properties": {
                        "result": {"type": "string"},
                        "stdout": {"type": "string"},
                        "stderr": {"type": "string"}
                    }
                },
                "scopes": [],
                "tags": ["meta"],
                "version": "1.0.0",
                "available_versions": ["1.0.0"],
                "is_deprecated": false,
                "no_auth": true
            },
            {
                "slug": "COMPOSIO_REMOTE_BASH_TOOL",
                "name": "Remote Bash Tool",
                "description": "Execute bash commands for file/data processing",
                "toolkit": "composio",
                "input_parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "Bash command to execute"
                        }
                    },
                    "required": ["command"]
                },
                "output_parameters": {
                    "type": "object",
                    "properties": {
                        "stdout": {"type": "string"},
                        "stderr": {"type": "string"},
                        "exit_code": {"type": "integer"}
                    }
                },
                "scopes": [],
                "tags": ["meta"],
                "version": "1.0.0",
                "available_versions": ["1.0.0"],
                "is_deprecated": false,
                "no_auth": true
            }
        ])))
        .mount(&mock_server)
        .await;

    // Create client
    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    // Create session
    let session = client.create_session("user_123").send().await.unwrap();

    // Get meta tools
    let meta_tools = session.get_meta_tools().await.unwrap();

    // Verify response
    assert_eq!(meta_tools.len(), 5);

    // Verify COMPOSIO_SEARCH_TOOLS
    let search_tool = meta_tools
        .iter()
        .find(|t| t.slug == "COMPOSIO_SEARCH_TOOLS")
        .expect("COMPOSIO_SEARCH_TOOLS not found");
    assert_eq!(search_tool.name, "Search Tools");
    assert_eq!(
        search_tool.description,
        "Discover relevant tools across 1000+ apps"
    );
    assert_eq!(search_tool.toolkit, "composio");
    assert_eq!(search_tool.version, "1.0.0");
    assert!(!search_tool.is_deprecated);
    assert!(search_tool.no_auth);

    // Verify COMPOSIO_MULTI_EXECUTE_TOOL
    let multi_exec_tool = meta_tools
        .iter()
        .find(|t| t.slug == "COMPOSIO_MULTI_EXECUTE_TOOL")
        .expect("COMPOSIO_MULTI_EXECUTE_TOOL not found");
    assert_eq!(multi_exec_tool.name, "Multi Execute Tool");
    assert_eq!(
        multi_exec_tool.description,
        "Execute up to 20 tools in parallel"
    );

    // Verify COMPOSIO_MANAGE_CONNECTIONS
    let manage_conn_tool = meta_tools
        .iter()
        .find(|t| t.slug == "COMPOSIO_MANAGE_CONNECTIONS")
        .expect("COMPOSIO_MANAGE_CONNECTIONS not found");
    assert_eq!(manage_conn_tool.name, "Manage Connections");

    // Verify COMPOSIO_REMOTE_WORKBENCH
    let workbench_tool = meta_tools
        .iter()
        .find(|t| t.slug == "COMPOSIO_REMOTE_WORKBENCH")
        .expect("COMPOSIO_REMOTE_WORKBENCH not found");
    assert_eq!(workbench_tool.name, "Remote Workbench");

    // Verify COMPOSIO_REMOTE_BASH_TOOL
    let bash_tool = meta_tools
        .iter()
        .find(|t| t.slug == "COMPOSIO_REMOTE_BASH_TOOL")
        .expect("COMPOSIO_REMOTE_BASH_TOOL not found");
    assert_eq!(bash_tool.name, "Remote Bash Tool");
}

#[tokio::test]
async fn test_get_meta_tools_not_found() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_invalid",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {
                "user_id": "user_123"
            }
        })))
        .mount(&mock_server)
        .await;

    // Mock get meta tools endpoint with 404
    Mock::given(method("GET"))
        .and(path("/tool_router/session/sess_invalid/tools"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "message": "Session not found",
            "status": 404,
            "code": "SESSION_NOT_FOUND"
        })))
        .mount(&mock_server)
        .await;

    // Create client
    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    // Create session
    let session = client.create_session("user_123").send().await.unwrap();

    // Get meta tools should fail
    let result = session.get_meta_tools().await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ComposioError::ApiError { status, message, .. } => {
            assert_eq!(status, 404);
            assert_eq!(message, "Session not found");
        }
        _ => panic!("Expected ApiError"),
    }
}

#[tokio::test]
async fn test_get_meta_tools_with_retry() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_retry",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {
                "user_id": "user_123"
            }
        })))
        .mount(&mock_server)
        .await;

    // First request fails with 503 (retryable)
    Mock::given(method("GET"))
        .and(path("/tool_router/session/sess_retry/tools"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(503).set_body_json(json!({
            "message": "Service temporarily unavailable",
            "status": 503
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second request succeeds
    Mock::given(method("GET"))
        .and(path("/tool_router/session/sess_retry/tools"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "slug": "COMPOSIO_SEARCH_TOOLS",
                "name": "Search Tools",
                "description": "Discover relevant tools",
                "toolkit": "composio",
                "input_parameters": {},
                "output_parameters": {},
                "scopes": [],
                "tags": [],
                "version": "1.0.0",
                "available_versions": ["1.0.0"],
                "is_deprecated": false,
                "no_auth": true
            }
        ])))
        .mount(&mock_server)
        .await;

    // Create client with retry enabled
    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .max_retries(3)
        .build()
        .unwrap();

    // Create session
    let session = client.create_session("user_123").send().await.unwrap();

    // Get meta tools should succeed after retry
    let meta_tools = session.get_meta_tools().await.unwrap();

    assert_eq!(meta_tools.len(), 1);
    assert_eq!(meta_tools[0].slug, "COMPOSIO_SEARCH_TOOLS");
}

#[tokio::test]
async fn test_get_meta_tools_empty_response() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_empty",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {
                "user_id": "user_123"
            }
        })))
        .mount(&mock_server)
        .await;

    // Mock get meta tools endpoint with empty array
    Mock::given(method("GET"))
        .and(path("/tool_router/session/sess_empty/tools"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&mock_server)
        .await;

    // Create client
    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    // Create session
    let session = client.create_session("user_123").send().await.unwrap();

    // Get meta tools
    let meta_tools = session.get_meta_tools().await.unwrap();

    // Verify empty response
    assert_eq!(meta_tools.len(), 0);
}

#[tokio::test]
async fn test_get_meta_tools_unauthorized() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Mock session creation
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_unauth",
            "mcp": {"url": "https://mcp.composio.dev"},
            "tool_router_tools": [],
            "config": {
                "user_id": "user_123"
            }
        })))
        .mount(&mock_server)
        .await;

    // Mock get meta tools endpoint with 401
    Mock::given(method("GET"))
        .and(path("/tool_router/session/sess_unauth/tools"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "message": "Invalid API key",
            "status": 401,
            "code": "UNAUTHORIZED"
        })))
        .mount(&mock_server)
        .await;

    // Create client
    let client = ComposioClient::builder()
        .api_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    // Create session
    let session = client.create_session("user_123").send().await.unwrap();

    // Get meta tools should fail
    let result = session.get_meta_tools().await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ComposioError::ApiError { status, message, .. } => {
            assert_eq!(status, 401);
            assert_eq!(message, "Invalid API key");
        }
        _ => panic!("Expected ApiError"),
    }
}
