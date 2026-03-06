//! Response models from Composio API

use serde::Deserialize;

use super::enums::AuthScheme;
use super::request::SessionConfig;

/// Response from session creation
#[derive(Debug, Clone, Deserialize)]
pub struct SessionResponse {
    pub session_id: String,
    pub mcp: McpInfo,
    pub tool_router_tools: Vec<String>,
    pub config: SessionConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assistive_prompt: Option<String>,
}

/// MCP server information
#[derive(Debug, Clone, Deserialize)]
pub struct McpInfo {
    pub url: String,
}

/// Tool schema information
#[derive(Debug, Clone, Deserialize)]
pub struct ToolSchema {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub toolkit: String,
    pub input_parameters: serde_json::Value,
    pub output_parameters: serde_json::Value,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub version: String,
    #[serde(default)]
    pub available_versions: Vec<String>,
    #[serde(default)]
    pub is_deprecated: bool,
    #[serde(default)]
    pub no_auth: bool,
}

/// Response from tool execution
#[derive(Debug, Clone, Deserialize)]
pub struct ToolExecutionResponse {
    pub data: serde_json::Value,
    pub error: Option<String>,
    pub log_id: String,
}

/// Response from meta tool execution
pub type MetaToolExecutionResponse = ToolExecutionResponse;

/// Response from listing toolkits
#[derive(Debug, Clone, Deserialize)]
pub struct ToolkitListResponse {
    pub items: Vec<ToolkitInfo>,
    pub next_cursor: Option<String>,
    pub total_pages: u32,
    pub current_page: u32,
    pub total_items: u32,
}

/// Information about a toolkit
#[derive(Debug, Clone, Deserialize)]
pub struct ToolkitInfo {
    pub name: String,
    pub slug: String,
    pub enabled: bool,
    pub is_no_auth: bool,
    pub composio_managed_auth_schemes: Vec<AuthScheme>,
    pub meta: ToolkitMeta,
    pub connected_account: Option<ConnectedAccountInfo>,
}

/// Metadata about a toolkit
#[derive(Debug, Clone, Deserialize)]
pub struct ToolkitMeta {
    pub logo: String,
    pub description: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub tools_count: u32,
    #[serde(default)]
    pub triggers_count: u32,
    #[serde(default)]
    pub version: String,
}

/// Information about a connected account
#[derive(Debug, Clone, Deserialize)]
pub struct ConnectedAccountInfo {
    pub id: String,
    pub status: String,
    pub created_at: String,
}

/// Response from creating an auth link
#[derive(Debug, Clone, Deserialize)]
pub struct LinkResponse {
    pub link_token: String,
    pub redirect_url: String,
    pub connected_account_id: Option<String>,
}

/// Error response from API
#[derive(Debug, Clone, Deserialize)]
pub struct ErrorResponse {
    pub message: String,
    pub code: Option<String>,
    pub slug: Option<String>,
    pub status: u16,
    pub request_id: Option<String>,
    pub suggested_fix: Option<String>,
    pub errors: Option<Vec<crate::error::ErrorDetail>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_session_response_deserialization() {
        let json = r#"{
            "session_id": "sess_abc123",
            "mcp": {
                "url": "https://mcp.composio.dev/sess_abc123"
            },
            "tool_router_tools": [
                "COMPOSIO_SEARCH_TOOLS",
                "COMPOSIO_MULTI_EXECUTE_TOOL"
            ],
            "config": {
                "user_id": "user_123"
            }
        }"#;

        let response: SessionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.session_id, "sess_abc123");
        assert_eq!(response.mcp.url, "https://mcp.composio.dev/sess_abc123");
        assert_eq!(response.tool_router_tools.len(), 2);
        assert_eq!(response.config.user_id, "user_123");
        assert!(response.assistive_prompt.is_none());
    }

    #[test]
    fn test_session_response_with_assistive_prompt() {
        let json = r#"{
            "session_id": "sess_abc123",
            "mcp": {
                "url": "https://mcp.composio.dev/sess_abc123"
            },
            "tool_router_tools": [],
            "config": {
                "user_id": "user_123"
            },
            "assistive_prompt": "Use COMPOSIO_SEARCH_TOOLS to discover tools"
        }"#;

        let response: SessionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(
            response.assistive_prompt,
            Some("Use COMPOSIO_SEARCH_TOOLS to discover tools".to_string())
        );
    }

    #[test]
    fn test_mcp_info_deserialization() {
        let json = r#"{
            "url": "https://mcp.composio.dev/session_123"
        }"#;

        let mcp: McpInfo = serde_json::from_str(json).unwrap();
        assert_eq!(mcp.url, "https://mcp.composio.dev/session_123");
    }

    #[test]
    fn test_tool_schema_deserialization() {
        let json = r#"{
            "slug": "GITHUB_CREATE_ISSUE",
            "name": "Create Issue",
            "description": "Create a new issue in a GitHub repository",
            "toolkit": "github",
            "input_parameters": {
                "type": "object",
                "properties": {
                    "owner": {"type": "string"},
                    "repo": {"type": "string"},
                    "title": {"type": "string"}
                }
            },
            "output_parameters": {
                "type": "object",
                "properties": {
                    "id": {"type": "number"}
                }
            },
            "scopes": ["repo"],
            "tags": ["write"],
            "version": "1.0.0",
            "available_versions": ["1.0.0", "0.9.0"],
            "is_deprecated": false,
            "no_auth": false
        }"#;

        let schema: ToolSchema = serde_json::from_str(json).unwrap();
        assert_eq!(schema.slug, "GITHUB_CREATE_ISSUE");
        assert_eq!(schema.name, "Create Issue");
        assert_eq!(schema.toolkit, "github");
        assert_eq!(schema.scopes.len(), 1);
        assert_eq!(schema.tags.len(), 1);
        assert_eq!(schema.version, "1.0.0");
        assert_eq!(schema.available_versions.len(), 2);
        assert!(!schema.is_deprecated);
        assert!(!schema.no_auth);
    }

    #[test]
    fn test_tool_schema_minimal_deserialization() {
        let json = r#"{
            "slug": "SIMPLE_TOOL",
            "name": "Simple Tool",
            "description": "A simple tool",
            "toolkit": "simple",
            "input_parameters": {},
            "output_parameters": {},
            "version": "1.0.0"
        }"#;

        let schema: ToolSchema = serde_json::from_str(json).unwrap();
        assert_eq!(schema.slug, "SIMPLE_TOOL");
        assert!(schema.scopes.is_empty());
        assert!(schema.tags.is_empty());
        assert!(schema.available_versions.is_empty());
        assert!(!schema.is_deprecated);
        assert!(!schema.no_auth);
    }

    #[test]
    fn test_tool_execution_response_deserialization() {
        let json = r#"{
            "data": {
                "issue_id": 123,
                "url": "https://github.com/owner/repo/issues/123"
            },
            "error": null,
            "log_id": "log_xyz789"
        }"#;

        let response: ToolExecutionResponse = serde_json::from_str(json).unwrap();
        assert!(response.data.is_object());
        assert_eq!(response.data["issue_id"], 123);
        assert!(response.error.is_none());
        assert_eq!(response.log_id, "log_xyz789");
    }

    #[test]
    fn test_tool_execution_response_with_error() {
        let json = r#"{
            "data": null,
            "error": "Failed to create issue: Invalid repository",
            "log_id": "log_error123"
        }"#;

        let response: ToolExecutionResponse = serde_json::from_str(json).unwrap();
        assert!(response.data.is_null());
        assert_eq!(
            response.error,
            Some("Failed to create issue: Invalid repository".to_string())
        );
        assert_eq!(response.log_id, "log_error123");
    }

    #[test]
    fn test_toolkit_list_response_deserialization() {
        let json = r#"{
            "items": [
                {
                    "name": "GitHub",
                    "slug": "github",
                    "enabled": true,
                    "is_no_auth": false,
                    "composio_managed_auth_schemes": ["OAUTH2"],
                    "meta": {
                        "logo": "https://logo.url",
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
            "next_cursor": "cursor_abc",
            "total_pages": 5,
            "current_page": 1,
            "total_items": 100
        }"#;

        let response: ToolkitListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.items.len(), 1);
        assert_eq!(response.next_cursor, Some("cursor_abc".to_string()));
        assert_eq!(response.total_pages, 5);
        assert_eq!(response.current_page, 1);
        assert_eq!(response.total_items, 100);
    }

    #[test]
    fn test_toolkit_info_deserialization() {
        let json = r#"{
            "name": "Gmail",
            "slug": "gmail",
            "enabled": true,
            "is_no_auth": false,
            "composio_managed_auth_schemes": ["OAUTH2"],
            "meta": {
                "logo": "https://gmail.logo",
                "description": "Gmail integration",
                "categories": ["communication"],
                "tools_count": 30,
                "triggers_count": 5,
                "version": "2.0.0"
            },
            "connected_account": null
        }"#;

        let info: ToolkitInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.name, "Gmail");
        assert_eq!(info.slug, "gmail");
        assert!(info.enabled);
        assert!(!info.is_no_auth);
        assert_eq!(info.composio_managed_auth_schemes.len(), 1);
        assert!(info.connected_account.is_none());
    }

    #[test]
    fn test_toolkit_meta_deserialization() {
        let json = r#"{
            "logo": "https://logo.url",
            "description": "Test toolkit",
            "categories": ["test", "development"],
            "tools_count": 25,
            "triggers_count": 3,
            "version": "1.5.0"
        }"#;

        let meta: ToolkitMeta = serde_json::from_str(json).unwrap();
        assert_eq!(meta.logo, "https://logo.url");
        assert_eq!(meta.description, "Test toolkit");
        assert_eq!(meta.categories.len(), 2);
        assert_eq!(meta.tools_count, 25);
        assert_eq!(meta.triggers_count, 3);
        assert_eq!(meta.version, "1.5.0");
    }

    #[test]
    fn test_toolkit_meta_minimal_deserialization() {
        let json = r#"{
            "logo": "https://logo.url",
            "description": "Minimal toolkit"
        }"#;

        let meta: ToolkitMeta = serde_json::from_str(json).unwrap();
        assert_eq!(meta.logo, "https://logo.url");
        assert_eq!(meta.description, "Minimal toolkit");
        assert!(meta.categories.is_empty());
        assert_eq!(meta.tools_count, 0);
        assert_eq!(meta.triggers_count, 0);
        assert_eq!(meta.version, "");
    }

    #[test]
    fn test_connected_account_info_deserialization() {
        let json = r#"{
            "id": "ca_abc123",
            "status": "ACTIVE",
            "created_at": "2024-01-15T10:30:00Z"
        }"#;

        let info: ConnectedAccountInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.id, "ca_abc123");
        assert_eq!(info.status, "ACTIVE");
        assert_eq!(info.created_at, "2024-01-15T10:30:00Z");
    }

    #[test]
    fn test_link_response_deserialization() {
        let json = r#"{
            "link_token": "lt_xyz789",
            "redirect_url": "https://auth.composio.dev/link?token=lt_xyz789",
            "connected_account_id": "ca_existing123"
        }"#;

        let response: LinkResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.link_token, "lt_xyz789");
        assert_eq!(response.redirect_url, "https://auth.composio.dev/link?token=lt_xyz789");
        assert_eq!(response.connected_account_id, Some("ca_existing123".to_string()));
    }

    #[test]
    fn test_link_response_without_connected_account() {
        let json = r#"{
            "link_token": "lt_new456",
            "redirect_url": "https://auth.composio.dev/link?token=lt_new456",
            "connected_account_id": null
        }"#;

        let response: LinkResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.link_token, "lt_new456");
        assert!(response.connected_account_id.is_none());
    }

    #[test]
    fn test_error_response_deserialization() {
        let json = r#"{
            "message": "Validation failed",
            "code": "VALIDATION_ERROR",
            "slug": "validation-failed",
            "status": 400,
            "request_id": "req_abc123",
            "suggested_fix": "Check your input parameters",
            "errors": [
                {
                    "field": "user_id",
                    "message": "User ID is required"
                }
            ]
        }"#;

        let response: ErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.message, "Validation failed");
        assert_eq!(response.code, Some("VALIDATION_ERROR".to_string()));
        assert_eq!(response.slug, Some("validation-failed".to_string()));
        assert_eq!(response.status, 400);
        assert_eq!(response.request_id, Some("req_abc123".to_string()));
        assert_eq!(response.suggested_fix, Some("Check your input parameters".to_string()));
        assert!(response.errors.is_some());
        assert_eq!(response.errors.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_error_response_minimal_deserialization() {
        let json = r#"{
            "message": "Internal server error",
            "status": 500
        }"#;

        let response: ErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.message, "Internal server error");
        assert_eq!(response.status, 500);
        assert!(response.code.is_none());
        assert!(response.slug.is_none());
        assert!(response.request_id.is_none());
        assert!(response.suggested_fix.is_none());
        assert!(response.errors.is_none());
    }

    #[test]
    fn test_auth_scheme_deserialization() {
        let json = r#"["OAUTH2", "API_KEY", "BEARER_TOKEN"]"#;
        let schemes: Vec<AuthScheme> = serde_json::from_str(json).unwrap();
        
        assert_eq!(schemes.len(), 3);
        assert!(matches!(schemes[0], AuthScheme::Oauth2));
        assert!(matches!(schemes[1], AuthScheme::ApiKey));
        assert!(matches!(schemes[2], AuthScheme::BearerToken));
    }

    #[test]
    fn test_meta_tool_execution_response_alias() {
        let json = r#"{
            "data": {"result": "success"},
            "error": null,
            "log_id": "log_meta123"
        }"#;

        let response: MetaToolExecutionResponse = serde_json::from_str(json).unwrap();
        assert!(response.data.is_object());
        assert!(response.error.is_none());
        assert_eq!(response.log_id, "log_meta123");
    }

    #[test]
    fn test_toolkit_list_response_empty_items() {
        let json = r#"{
            "items": [],
            "next_cursor": null,
            "total_pages": 0,
            "current_page": 0,
            "total_items": 0
        }"#;

        let response: ToolkitListResponse = serde_json::from_str(json).unwrap();
        assert!(response.items.is_empty());
        assert!(response.next_cursor.is_none());
        assert_eq!(response.total_pages, 0);
        assert_eq!(response.current_page, 0);
        assert_eq!(response.total_items, 0);
    }

    #[test]
    fn test_session_response_clone() {
        let json = r#"{
            "session_id": "sess_123",
            "mcp": {"url": "https://mcp.url"},
            "tool_router_tools": [],
            "config": {"user_id": "user_123"}
        }"#;

        let response: SessionResponse = serde_json::from_str(json).unwrap();
        let cloned = response.clone();
        
        assert_eq!(response.session_id, cloned.session_id);
        assert_eq!(response.mcp.url, cloned.mcp.url);
    }

    #[test]
    fn test_tool_schema_debug() {
        let json = r#"{
            "slug": "TEST_TOOL",
            "name": "Test",
            "description": "Test tool",
            "toolkit": "test",
            "input_parameters": {},
            "output_parameters": {},
            "version": "1.0.0"
        }"#;

        let schema: ToolSchema = serde_json::from_str(json).unwrap();
        let debug_str = format!("{:?}", schema);
        
        assert!(debug_str.contains("TEST_TOOL"));
        assert!(debug_str.contains("Test"));
    }
}
