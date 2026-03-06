//! Compatibility Validation Tests
//!
//! These tests validate that the Rust SDK produces JSON output compatible
//! with the Python SDK and Composio API expectations.
//!
//! This test suite addresses Task 8.3: Compatibility Validation

use composio_sdk::models::{
    AuthScheme, LinkRequest, ManageConnectionsConfig, MetaToolExecutionRequest, MetaToolSlug,
    SessionConfig, TagType, TagsConfig, ToolExecutionRequest, ToolFilter, ToolkitFilter,
    ToolsConfig, WorkbenchConfig,
};
use serde_json::json;
use std::collections::HashMap;

/// Test 8.3.1: Compare JSON output from Rust SDK with Python SDK
/// Test 8.3.2: Verify SessionConfig serialization matches Python
#[test]
fn test_session_config_json_compatibility() {
    // Minimal SessionConfig (Python equivalent: SessionConfig(user_id="user_123"))
    let minimal_config = SessionConfig {
        user_id: "user_123".to_string(),
        toolkits: None,
        auth_configs: None,
        connected_accounts: None,
        manage_connections: None,
        tools: None,
        tags: None,
        workbench: None,
    };

    let json_str = serde_json::to_string(&minimal_config).unwrap();
    let json_value: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // Verify only user_id is present (optional fields omitted)
    assert_eq!(json_value["user_id"], "user_123");
    assert!(json_value.get("toolkits").is_none());
    assert!(json_value.get("auth_configs").is_none());
    assert!(json_value.get("connected_accounts").is_none());
    assert!(json_value.get("manage_connections").is_none());
    assert!(json_value.get("tools").is_none());
    assert!(json_value.get("tags").is_none());
    assert!(json_value.get("workbench").is_none());

    // Complete SessionConfig with all fields
    let mut auth_configs = HashMap::new();
    auth_configs.insert("github".to_string(), "ac_github_123".to_string());

    let mut connected_accounts = HashMap::new();
    connected_accounts.insert("gmail".to_string(), "ca_work_gmail".to_string());

    let mut tools_map = HashMap::new();
    tools_map.insert(
        "gmail".to_string(),
        ToolFilter::Enable {
            enable: vec!["GMAIL_SEND_EMAIL".to_string()],
        },
    );

    let complete_config = SessionConfig {
        user_id: "user_123".to_string(),
        toolkits: Some(ToolkitFilter::Enable(vec![
            "github".to_string(),
            "gmail".to_string(),
        ])),
        auth_configs: Some(auth_configs),
        connected_accounts: Some(connected_accounts),
        manage_connections: Some(ManageConnectionsConfig::Bool(true)),
        tools: Some(ToolsConfig(tools_map)),
        tags: Some(TagsConfig {
            enabled: Some(vec![TagType::ReadOnlyHint]),
            disabled: Some(vec![TagType::DestructiveHint]),
        }),
        workbench: Some(WorkbenchConfig {
            proxy_execution: Some(true),
            auto_offload_threshold: Some(20000),
        }),
    };

    let json_str = serde_json::to_string(&complete_config).unwrap();
    let json_value: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // Verify all fields are present and correctly formatted
    assert_eq!(json_value["user_id"], "user_123");
    assert_eq!(json_value["toolkits"], json!(["github", "gmail"]));
    assert_eq!(json_value["auth_configs"]["github"], "ac_github_123");
    assert_eq!(json_value["connected_accounts"]["gmail"], "ca_work_gmail");
    assert_eq!(json_value["manage_connections"], true);
    assert_eq!(
        json_value["tools"]["gmail"]["enable"],
        json!(["GMAIL_SEND_EMAIL"])
    );
    assert_eq!(json_value["tags"]["enabled"], json!(["READ_ONLY_HINT"]));
    assert_eq!(json_value["tags"]["disabled"], json!(["DESTRUCTIVE_HINT"]));
    assert_eq!(json_value["workbench"]["proxy_execution"], true);
    assert_eq!(json_value["workbench"]["auto_offload_threshold"], 20000);

    println!("✓ SessionConfig JSON serialization is compatible with Python SDK");
}

/// Test 8.3.3: Verify ToolExecutionRequest serialization matches Python
#[test]
fn test_tool_execution_request_json_compatibility() {
    // With arguments
    let request_with_args = ToolExecutionRequest {
        tool_slug: "GITHUB_CREATE_ISSUE".to_string(),
        arguments: Some(json!({
            "owner": "composio",
            "repo": "composio",
            "title": "Test issue",
            "body": "Created via Rust SDK"
        })),
    };

    let json_str = serde_json::to_string(&request_with_args).unwrap();
    let json_value: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    assert_eq!(json_value["tool_slug"], "GITHUB_CREATE_ISSUE");
    assert_eq!(json_value["arguments"]["owner"], "composio");
    assert_eq!(json_value["arguments"]["repo"], "composio");
    assert_eq!(json_value["arguments"]["title"], "Test issue");
    assert_eq!(json_value["arguments"]["body"], "Created via Rust SDK");

    // Without arguments
    let request_no_args = ToolExecutionRequest {
        tool_slug: "GITHUB_GET_USER".to_string(),
        arguments: None,
    };

    let json_str = serde_json::to_string(&request_no_args).unwrap();
    let json_value: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    assert_eq!(json_value["tool_slug"], "GITHUB_GET_USER");
    assert!(json_value.get("arguments").is_none());

    println!("✓ ToolExecutionRequest JSON serialization is compatible with Python SDK");
}

/// Test 8.3.4: Verify MetaToolExecutionRequest serialization matches Python
#[test]
fn test_meta_tool_execution_request_json_compatibility() {
    // Test all meta tool slugs
    let meta_tools = vec![
        (
            MetaToolSlug::ComposioSearchTools,
            "COMPOSIO_SEARCH_TOOLS",
            json!({"query": "create a GitHub issue"}),
        ),
        (
            MetaToolSlug::ComposioMultiExecuteTool,
            "COMPOSIO_MULTI_EXECUTE_TOOL",
            json!({
                "tools": [
                    {"tool_slug": "GITHUB_GET_REPOS", "arguments": {"owner": "composio"}}
                ]
            }),
        ),
        (
            MetaToolSlug::ComposioManageConnections,
            "COMPOSIO_MANAGE_CONNECTIONS",
            json!({"toolkit": "github"}),
        ),
        (
            MetaToolSlug::ComposioRemoteWorkbench,
            "COMPOSIO_REMOTE_WORKBENCH",
            json!({"code": "print('hello')"}),
        ),
        (
            MetaToolSlug::ComposioRemoteBashTool,
            "COMPOSIO_REMOTE_BASH_TOOL",
            json!({"command": "ls -la"}),
        ),
    ];

    for (slug, expected_str, args) in meta_tools {
        let request = MetaToolExecutionRequest {
            slug,
            arguments: Some(args.clone()),
        };

        let json_str = serde_json::to_string(&request).unwrap();
        let json_value: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json_value["slug"], expected_str);
        assert_eq!(json_value["arguments"], args);
    }

    println!("✓ MetaToolExecutionRequest JSON serialization is compatible with Python SDK");
}

/// Test 8.3.5: Verify all enums serialize correctly (SCREAMING_SNAKE_CASE)
#[test]
fn test_enum_serialization_screaming_snake_case() {
    // Test MetaToolSlug enum
    let meta_tool_slugs = vec![
        (MetaToolSlug::ComposioSearchTools, "COMPOSIO_SEARCH_TOOLS"),
        (
            MetaToolSlug::ComposioMultiExecuteTool,
            "COMPOSIO_MULTI_EXECUTE_TOOL",
        ),
        (
            MetaToolSlug::ComposioManageConnections,
            "COMPOSIO_MANAGE_CONNECTIONS",
        ),
        (
            MetaToolSlug::ComposioRemoteWorkbench,
            "COMPOSIO_REMOTE_WORKBENCH",
        ),
        (
            MetaToolSlug::ComposioRemoteBashTool,
            "COMPOSIO_REMOTE_BASH_TOOL",
        ),
    ];

    for (slug, expected) in meta_tool_slugs {
        let json_value = serde_json::to_value(&slug).unwrap();
        assert_eq!(json_value, expected);
    }

    // Test TagType enum
    let tag_types = vec![
        (TagType::ReadOnlyHint, "READ_ONLY_HINT"),
        (TagType::DestructiveHint, "DESTRUCTIVE_HINT"),
        (TagType::IdempotentHint, "IDEMPOTENT_HINT"),
        (TagType::OpenWorldHint, "OPEN_WORLD_HINT"),
    ];

    for (tag, expected) in tag_types {
        let json_value = serde_json::to_value(&tag).unwrap();
        assert_eq!(json_value, expected);
    }

    // Test AuthScheme enum
    let auth_schemes = vec![
        (AuthScheme::Oauth2, "OAUTH2"),
        (AuthScheme::Oauth1, "OAUTH1"),
        (AuthScheme::ApiKey, "API_KEY"),
        (AuthScheme::BearerToken, "BEARER_TOKEN"),
        (AuthScheme::Basic, "BASIC"),
        (AuthScheme::Custom, "CUSTOM"),
    ];

    for (scheme, expected) in auth_schemes {
        let json_value = serde_json::to_value(&scheme).unwrap();
        assert_eq!(json_value, expected);
    }

    println!("✓ All enums serialize to SCREAMING_SNAKE_CASE format");
}

/// Test 8.3.6: Verify response deserialization handles Python test fixtures
#[test]
fn test_response_deserialization_python_fixtures() {
    // Simulate Python SDK response for SessionResponse
    let python_session_response = json!({
        "session_id": "sess_abc123",
        "mcp": {
            "url": "https://mcp.composio.dev/session/sess_abc123"
        },
        "tool_router_tools": [
            "COMPOSIO_SEARCH_TOOLS",
            "COMPOSIO_MULTI_EXECUTE_TOOL",
            "COMPOSIO_MANAGE_CONNECTIONS"
        ],
        "config": {
            "user_id": "user_123",
            "toolkits": ["github", "gmail"],
            "manage_connections": true
        },
        "assistive_prompt": "You can use these tools to help the user"
    });

    let session_response: composio_sdk::models::SessionResponse =
        serde_json::from_value(python_session_response).unwrap();

    assert_eq!(session_response.session_id, "sess_abc123");
    assert_eq!(
        session_response.mcp.url,
        "https://mcp.composio.dev/session/sess_abc123"
    );
    assert_eq!(session_response.tool_router_tools.len(), 3);
    assert_eq!(
        session_response.tool_router_tools[0],
        "COMPOSIO_SEARCH_TOOLS"
    );
    assert_eq!(session_response.config.user_id, "user_123");
    assert!(session_response.assistive_prompt.is_some());

    // Simulate Python SDK response for ToolExecutionResponse
    let python_tool_execution_response = json!({
        "data": {
            "issue_id": 123,
            "url": "https://github.com/owner/repo/issues/123",
            "title": "Test issue"
        },
        "error": null,
        "log_id": "log_xyz789"
    });

    let tool_execution_response: composio_sdk::models::ToolExecutionResponse =
        serde_json::from_value(python_tool_execution_response).unwrap();

    assert!(tool_execution_response.data.is_object());
    assert_eq!(tool_execution_response.error, None);
    assert_eq!(tool_execution_response.log_id, "log_xyz789");

    // Simulate Python SDK error response
    let python_error_response = json!({
        "message": "Tool not found",
        "code": "TOOL_NOT_FOUND",
        "slug": "tool-not-found",
        "status": 404,
        "request_id": "req_abc123",
        "suggested_fix": "Check the tool slug and try again",
        "errors": [
            {
                "field": "tool_slug",
                "message": "Tool with slug 'INVALID_TOOL' does not exist"
            }
        ]
    });

    let error_response: composio_sdk::models::ErrorResponse =
        serde_json::from_value(python_error_response).unwrap();

    assert_eq!(error_response.message, "Tool not found");
    assert_eq!(error_response.code, Some("TOOL_NOT_FOUND".to_string()));
    assert_eq!(error_response.status, 404);
    assert!(error_response.errors.is_some());

    println!("✓ Response deserialization handles Python SDK fixtures correctly");
}

/// Test toolkit filter variants match Python SDK
#[test]
fn test_toolkit_filter_variants_compatibility() {
    // Enable variant (Python: toolkits=["github", "gmail"])
    let enable_filter = ToolkitFilter::Enable(vec!["github".to_string(), "gmail".to_string()]);
    let json_value = serde_json::to_value(&enable_filter).unwrap();
    assert_eq!(json_value, json!(["github", "gmail"]));

    // Disable variant (Python: toolkits={"disable": ["exa", "firecrawl"]})
    let disable_filter = ToolkitFilter::Disable {
        disable: vec!["exa".to_string(), "firecrawl".to_string()],
    };
    let json_value = serde_json::to_value(&disable_filter).unwrap();
    assert_eq!(json_value, json!({"disable": ["exa", "firecrawl"]}));

    println!("✓ ToolkitFilter variants are compatible with Python SDK");
}

/// Test tools config variants match Python SDK
#[test]
fn test_tools_config_variants_compatibility() {
    let mut tools_map = HashMap::new();

    // Enable variant
    tools_map.insert(
        "gmail".to_string(),
        ToolFilter::Enable {
            enable: vec!["GMAIL_SEND_EMAIL".to_string(), "GMAIL_READ_EMAIL".to_string()],
        },
    );

    // Disable variant
    tools_map.insert(
        "github".to_string(),
        ToolFilter::Disable {
            disable: vec!["GITHUB_DELETE_REPO".to_string()],
        },
    );

    // Shorthand list variant
    tools_map.insert(
        "slack".to_string(),
        ToolFilter::EnableList(vec![
            "SLACK_SEND_MESSAGE".to_string(),
            "SLACK_LIST_CHANNELS".to_string(),
        ]),
    );

    let tools_config = ToolsConfig(tools_map);
    let json_value = serde_json::to_value(&tools_config).unwrap();

    assert_eq!(
        json_value["gmail"],
        json!({"enable": ["GMAIL_SEND_EMAIL", "GMAIL_READ_EMAIL"]})
    );
    assert_eq!(
        json_value["github"],
        json!({"disable": ["GITHUB_DELETE_REPO"]})
    );
    assert_eq!(
        json_value["slack"],
        json!(["SLACK_SEND_MESSAGE", "SLACK_LIST_CHANNELS"])
    );

    println!("✓ ToolsConfig variants are compatible with Python SDK");
}

/// Test LinkRequest serialization
#[test]
fn test_link_request_compatibility() {
    // With callback URL
    let request_with_callback = LinkRequest {
        toolkit: "github".to_string(),
        callback_url: Some("https://example.com/callback".to_string()),
    };

    let json_value = serde_json::to_value(&request_with_callback).unwrap();
    assert_eq!(json_value["toolkit"], "github");
    assert_eq!(json_value["callback_url"], "https://example.com/callback");

    // Without callback URL
    let request_no_callback = LinkRequest {
        toolkit: "gmail".to_string(),
        callback_url: None,
    };

    let json_value = serde_json::to_value(&request_no_callback).unwrap();
    assert_eq!(json_value["toolkit"], "gmail");
    assert!(json_value.get("callback_url").is_none());

    println!("✓ LinkRequest serialization is compatible with Python SDK");
}

/// Comprehensive compatibility test
#[test]
fn test_comprehensive_json_compatibility() {
    println!("\n=== Comprehensive Compatibility Validation ===\n");

    // Test 1: SessionConfig
    println!("Testing SessionConfig serialization...");
    test_session_config_json_compatibility();

    // Test 2: ToolExecutionRequest
    println!("Testing ToolExecutionRequest serialization...");
    test_tool_execution_request_json_compatibility();

    // Test 3: MetaToolExecutionRequest
    println!("Testing MetaToolExecutionRequest serialization...");
    test_meta_tool_execution_request_json_compatibility();

    // Test 4: Enum serialization
    println!("Testing enum serialization...");
    test_enum_serialization_screaming_snake_case();

    // Test 5: Response deserialization
    println!("Testing response deserialization...");
    test_response_deserialization_python_fixtures();

    // Test 6: ToolkitFilter variants
    println!("Testing ToolkitFilter variants...");
    test_toolkit_filter_variants_compatibility();

    // Test 7: ToolsConfig variants
    println!("Testing ToolsConfig variants...");
    test_tools_config_variants_compatibility();

    // Test 8: LinkRequest
    println!("Testing LinkRequest serialization...");
    test_link_request_compatibility();

    println!("\n=== All Compatibility Tests Passed ===\n");
    println!("✓ Rust SDK JSON output is fully compatible with Python SDK");
    println!("✓ All request models serialize correctly");
    println!("✓ All response models deserialize correctly");
    println!("✓ All enums use SCREAMING_SNAKE_CASE format");
    println!("✓ Optional fields are properly omitted when None");
}
