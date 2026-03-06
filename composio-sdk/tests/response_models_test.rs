//! Tests for response model deserialization

use composio_sdk::models::{
    ConnectedAccountInfo, ErrorResponse, LinkResponse, McpInfo, MetaToolExecutionResponse,
    SessionResponse, ToolExecutionResponse, ToolSchema, ToolkitInfo, ToolkitListResponse,
    ToolkitMeta,
};
use serde_json::json;

#[test]
fn test_mcp_info_deserialization() {
    let json = json!({
        "url": "https://mcp.composio.dev/session/sess_123"
    });

    let mcp_info: McpInfo = serde_json::from_value(json).unwrap();
    assert_eq!(mcp_info.url, "https://mcp.composio.dev/session/sess_123");
}

#[test]
fn test_tool_schema_deserialization_minimal() {
    let json = json!({
        "slug": "GITHUB_CREATE_ISSUE",
        "name": "Create Issue",
        "description": "Create a new issue in a GitHub repository",
        "toolkit": "github",
        "input_parameters": {
            "type": "object",
            "properties": {
                "title": {"type": "string"},
                "body": {"type": "string"}
            }
        },
        "output_parameters": {
            "type": "object",
            "properties": {
                "id": {"type": "number"}
            }
        },
        "version": "1.0.0"
    });

    let tool_schema: ToolSchema = serde_json::from_value(json).unwrap();
    assert_eq!(tool_schema.slug, "GITHUB_CREATE_ISSUE");
    assert_eq!(tool_schema.name, "Create Issue");
    assert_eq!(tool_schema.toolkit, "github");
    assert_eq!(tool_schema.version, "1.0.0");
    assert_eq!(tool_schema.scopes.len(), 0); // Default empty
    assert_eq!(tool_schema.tags.len(), 0); // Default empty
    assert!(!tool_schema.is_deprecated); // Default false
    assert!(!tool_schema.no_auth); // Default false
}

#[test]
fn test_tool_schema_deserialization_complete() {
    let json = json!({
        "slug": "GITHUB_CREATE_ISSUE",
        "name": "Create Issue",
        "description": "Create a new issue in a GitHub repository",
        "toolkit": "github",
        "input_parameters": {
            "type": "object",
            "properties": {
                "title": {"type": "string"},
                "body": {"type": "string"}
            }
        },
        "output_parameters": {
            "type": "object",
            "properties": {
                "id": {"type": "number"}
            }
        },
        "scopes": ["repo", "write:org"],
        "tags": ["readOnlyHint", "idempotentHint"],
        "version": "1.0.0",
        "available_versions": ["1.0.0", "0.9.0"],
        "is_deprecated": false,
        "no_auth": false
    });

    let tool_schema: ToolSchema = serde_json::from_value(json).unwrap();
    assert_eq!(tool_schema.slug, "GITHUB_CREATE_ISSUE");
    assert_eq!(tool_schema.scopes, vec!["repo", "write:org"]);
    assert_eq!(tool_schema.tags, vec!["readOnlyHint", "idempotentHint"]);
    assert_eq!(tool_schema.available_versions, vec!["1.0.0", "0.9.0"]);
    assert!(!tool_schema.is_deprecated);
    assert!(!tool_schema.no_auth);
}

#[test]
fn test_session_response_deserialization() {
    let json = json!({
        "session_id": "sess_abc123",
        "mcp": {
            "url": "https://mcp.composio.dev/session/sess_abc123"
        },
        "tool_router_tools": [
            "COMPOSIO_SEARCH_TOOLS",
            "COMPOSIO_MULTI_EXECUTE_TOOL"
        ],
        "config": {
            "user_id": "user_123"
        },
        "assistive_prompt": "You can use these tools to help the user"
    });

    let session_response: SessionResponse = serde_json::from_value(json).unwrap();
    assert_eq!(session_response.session_id, "sess_abc123");
    assert_eq!(
        session_response.mcp.url,
        "https://mcp.composio.dev/session/sess_abc123"
    );
    assert_eq!(session_response.tool_router_tools.len(), 2);
    assert_eq!(
        session_response.tool_router_tools[0],
        "COMPOSIO_SEARCH_TOOLS"
    );
    assert_eq!(
        session_response.assistive_prompt,
        Some("You can use these tools to help the user".to_string())
    );
}

#[test]
fn test_tool_execution_response_success() {
    let json = json!({
        "data": {
            "issue_id": 123,
            "url": "https://github.com/owner/repo/issues/123"
        },
        "error": null,
        "log_id": "log_xyz789"
    });

    let response: ToolExecutionResponse = serde_json::from_value(json).unwrap();
    assert!(response.data.is_object());
    assert_eq!(response.error, None);
    assert_eq!(response.log_id, "log_xyz789");
}

#[test]
fn test_tool_execution_response_error() {
    let json = json!({
        "data": null,
        "error": "Failed to create issue: Repository not found",
        "log_id": "log_xyz789"
    });

    let response: ToolExecutionResponse = serde_json::from_value(json).unwrap();
    assert!(response.data.is_null());
    assert_eq!(
        response.error,
        Some("Failed to create issue: Repository not found".to_string())
    );
    assert_eq!(response.log_id, "log_xyz789");
}

#[test]
fn test_meta_tool_execution_response() {
    let json = json!({
        "data": {
            "tools": [
                {"slug": "GITHUB_CREATE_ISSUE", "name": "Create Issue"}
            ]
        },
        "error": null,
        "log_id": "log_meta123"
    });

    let response: MetaToolExecutionResponse = serde_json::from_value(json).unwrap();
    assert!(response.data.is_object());
    assert_eq!(response.error, None);
    assert_eq!(response.log_id, "log_meta123");
}

#[test]
fn test_toolkit_meta_deserialization() {
    let json = json!({
        "logo": "https://example.com/github-logo.png",
        "description": "GitHub integration toolkit",
        "categories": ["development", "version-control"],
        "tools_count": 50,
        "triggers_count": 10,
        "version": "2.1.0"
    });

    let toolkit_meta: ToolkitMeta = serde_json::from_value(json).unwrap();
    assert_eq!(toolkit_meta.logo, "https://example.com/github-logo.png");
    assert_eq!(toolkit_meta.description, "GitHub integration toolkit");
    assert_eq!(toolkit_meta.categories, vec!["development", "version-control"]);
    assert_eq!(toolkit_meta.tools_count, 50);
    assert_eq!(toolkit_meta.triggers_count, 10);
    assert_eq!(toolkit_meta.version, "2.1.0");
}

#[test]
fn test_connected_account_info_deserialization() {
    let json = json!({
        "id": "ca_abc123",
        "status": "ACTIVE",
        "created_at": "2024-01-15T10:30:00Z"
    });

    let account_info: ConnectedAccountInfo = serde_json::from_value(json).unwrap();
    assert_eq!(account_info.id, "ca_abc123");
    assert_eq!(account_info.status, "ACTIVE");
    assert_eq!(account_info.created_at, "2024-01-15T10:30:00Z");
}

#[test]
fn test_toolkit_info_deserialization() {
    let json = json!({
        "name": "GitHub",
        "slug": "github",
        "enabled": true,
        "is_no_auth": false,
        "composio_managed_auth_schemes": ["OAUTH2"],
        "meta": {
            "logo": "https://example.com/github-logo.png",
            "description": "GitHub integration",
            "categories": ["development"],
            "tools_count": 50,
            "triggers_count": 10,
            "version": "2.1.0"
        },
        "connected_account": {
            "id": "ca_abc123",
            "status": "ACTIVE",
            "created_at": "2024-01-15T10:30:00Z"
        }
    });

    let toolkit_info: ToolkitInfo = serde_json::from_value(json).unwrap();
    assert_eq!(toolkit_info.name, "GitHub");
    assert_eq!(toolkit_info.slug, "github");
    assert!(toolkit_info.enabled);
    assert!(!toolkit_info.is_no_auth);
    assert_eq!(toolkit_info.composio_managed_auth_schemes.len(), 1);
    assert!(toolkit_info.connected_account.is_some());
}

#[test]
fn test_toolkit_list_response_deserialization() {
    let json = json!({
        "items": [
            {
                "name": "GitHub",
                "slug": "github",
                "enabled": true,
                "is_no_auth": false,
                "composio_managed_auth_schemes": ["OAUTH2"],
                "meta": {
                    "logo": "https://example.com/github-logo.png",
                    "description": "GitHub integration",
                    "categories": ["development"],
                    "tools_count": 50,
                    "triggers_count": 10,
                    "version": "2.1.0"
                },
                "connected_account": null
            }
        ],
        "next_cursor": "cursor_xyz",
        "total_pages": 5,
        "current_page": 1,
        "total_items": 100
    });

    let list_response: ToolkitListResponse = serde_json::from_value(json).unwrap();
    assert_eq!(list_response.items.len(), 1);
    assert_eq!(list_response.next_cursor, Some("cursor_xyz".to_string()));
    assert_eq!(list_response.total_pages, 5);
    assert_eq!(list_response.current_page, 1);
    assert_eq!(list_response.total_items, 100);
}

#[test]
fn test_link_response_deserialization() {
    let json = json!({
        "link_token": "lt_abc123",
        "redirect_url": "https://backend.composio.dev/auth/github?token=lt_abc123",
        "connected_account_id": "ca_xyz789"
    });

    let link_response: LinkResponse = serde_json::from_value(json).unwrap();
    assert_eq!(link_response.link_token, "lt_abc123");
    assert_eq!(
        link_response.redirect_url,
        "https://backend.composio.dev/auth/github?token=lt_abc123"
    );
    assert_eq!(
        link_response.connected_account_id,
        Some("ca_xyz789".to_string())
    );
}

#[test]
fn test_error_response_deserialization_minimal() {
    let json = json!({
        "message": "Tool not found",
        "status": 404
    });

    let error_response: ErrorResponse = serde_json::from_value(json).unwrap();
    assert_eq!(error_response.message, "Tool not found");
    assert_eq!(error_response.status, 404);
    assert_eq!(error_response.code, None);
    assert_eq!(error_response.slug, None);
    assert_eq!(error_response.request_id, None);
    assert_eq!(error_response.suggested_fix, None);
    assert_eq!(error_response.errors, None);
}

#[test]
fn test_error_response_deserialization_complete() {
    let json = json!({
        "message": "Invalid input parameters",
        "code": "INVALID_INPUT",
        "slug": "invalid-input",
        "status": 400,
        "request_id": "req_abc123",
        "suggested_fix": "Check the input parameters and try again",
        "errors": [
            {
                "field": "title",
                "message": "Title is required"
            }
        ]
    });

    let error_response: ErrorResponse = serde_json::from_value(json).unwrap();
    assert_eq!(error_response.message, "Invalid input parameters");
    assert_eq!(error_response.code, Some("INVALID_INPUT".to_string()));
    assert_eq!(error_response.slug, Some("invalid-input".to_string()));
    assert_eq!(error_response.status, 400);
    assert_eq!(error_response.request_id, Some("req_abc123".to_string()));
    assert_eq!(
        error_response.suggested_fix,
        Some("Check the input parameters and try again".to_string())
    );
    assert!(error_response.errors.is_some());
    assert_eq!(error_response.errors.as_ref().unwrap().len(), 1);
}
