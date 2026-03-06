//! Tests for request models serialization
//!
//! These tests verify that the Rust SDK request models serialize to JSON
//! in a format compatible with the Python SDK and Composio API.

use composio_sdk::models::{
    LinkRequest, ManageConnectionsConfig, MetaToolExecutionRequest, MetaToolSlug, SessionConfig,
    TagType, TagsConfig, ToolExecutionRequest, ToolFilter, ToolkitFilter, ToolsConfig,
    WorkbenchConfig,
};
use serde_json::json;
use std::collections::HashMap;

#[test]
fn test_session_config_minimal() {
    let config = SessionConfig {
        user_id: "user_123".to_string(),
        toolkits: None,
        auth_configs: None,
        connected_accounts: None,
        manage_connections: None,
        tools: None,
        tags: None,
        workbench: None,
    };

    let json = serde_json::to_value(&config).unwrap();
    assert_eq!(json["user_id"], "user_123");
    // Optional fields should not be present
    assert!(json.get("toolkits").is_none());
    assert!(json.get("auth_configs").is_none());
}

#[test]
fn test_session_config_with_enabled_toolkits() {
    let config = SessionConfig {
        user_id: "user_123".to_string(),
        toolkits: Some(ToolkitFilter::Enable(vec![
            "github".to_string(),
            "gmail".to_string(),
        ])),
        auth_configs: None,
        connected_accounts: None,
        manage_connections: None,
        tools: None,
        tags: None,
        workbench: None,
    };

    let json = serde_json::to_value(&config).unwrap();
    assert_eq!(json["user_id"], "user_123");
    assert_eq!(json["toolkits"], json!(["github", "gmail"]));
}

#[test]
fn test_session_config_with_disabled_toolkits() {
    let config = SessionConfig {
        user_id: "user_123".to_string(),
        toolkits: Some(ToolkitFilter::Disable {
            disable: vec!["exa".to_string(), "firecrawl".to_string()],
        }),
        auth_configs: None,
        connected_accounts: None,
        manage_connections: None,
        tools: None,
        tags: None,
        workbench: None,
    };

    let json = serde_json::to_value(&config).unwrap();
    assert_eq!(json["user_id"], "user_123");
    assert_eq!(
        json["toolkits"],
        json!({
            "disable": ["exa", "firecrawl"]
        })
    );
}

#[test]
fn test_session_config_with_auth_configs() {
    let mut auth_configs = HashMap::new();
    auth_configs.insert("github".to_string(), "ac_github_123".to_string());
    auth_configs.insert("gmail".to_string(), "ac_gmail_456".to_string());

    let config = SessionConfig {
        user_id: "user_123".to_string(),
        toolkits: None,
        auth_configs: Some(auth_configs),
        connected_accounts: None,
        manage_connections: None,
        tools: None,
        tags: None,
        workbench: None,
    };

    let json = serde_json::to_value(&config).unwrap();
    assert_eq!(json["auth_configs"]["github"], "ac_github_123");
    assert_eq!(json["auth_configs"]["gmail"], "ac_gmail_456");
}

#[test]
fn test_session_config_with_connected_accounts() {
    let mut connected_accounts = HashMap::new();
    connected_accounts.insert("gmail".to_string(), "ca_work_gmail".to_string());

    let config = SessionConfig {
        user_id: "user_123".to_string(),
        toolkits: None,
        auth_configs: None,
        connected_accounts: Some(connected_accounts),
        manage_connections: None,
        tools: None,
        tags: None,
        workbench: None,
    };

    let json = serde_json::to_value(&config).unwrap();
    assert_eq!(json["connected_accounts"]["gmail"], "ca_work_gmail");
}

#[test]
fn test_session_config_with_manage_connections() {
    let config = SessionConfig {
        user_id: "user_123".to_string(),
        toolkits: None,
        auth_configs: None,
        connected_accounts: None,
        manage_connections: Some(ManageConnectionsConfig::Bool(true)),
        tools: None,
        tags: None,
        workbench: None,
    };

    let json = serde_json::to_value(&config).unwrap();
    assert_eq!(json["manage_connections"], true);
}

#[test]
fn test_tools_config_with_enable() {
    let mut tools_map = HashMap::new();
    tools_map.insert(
        "gmail".to_string(),
        ToolFilter::Enable {
            enable: vec!["GMAIL_SEND_EMAIL".to_string(), "GMAIL_READ_EMAIL".to_string()],
        },
    );

    let tools_config = ToolsConfig(tools_map);
    let json = serde_json::to_value(&tools_config).unwrap();

    assert_eq!(
        json["gmail"],
        json!({
            "enable": ["GMAIL_SEND_EMAIL", "GMAIL_READ_EMAIL"]
        })
    );
}

#[test]
fn test_tools_config_with_disable() {
    let mut tools_map = HashMap::new();
    tools_map.insert(
        "github".to_string(),
        ToolFilter::Disable {
            disable: vec!["GITHUB_DELETE_REPO".to_string()],
        },
    );

    let tools_config = ToolsConfig(tools_map);
    let json = serde_json::to_value(&tools_config).unwrap();

    assert_eq!(
        json["github"],
        json!({
            "disable": ["GITHUB_DELETE_REPO"]
        })
    );
}

#[test]
fn test_tools_config_with_shorthand_list() {
    let mut tools_map = HashMap::new();
    tools_map.insert(
        "slack".to_string(),
        ToolFilter::EnableList(vec![
            "SLACK_SEND_MESSAGE".to_string(),
            "SLACK_LIST_CHANNELS".to_string(),
        ]),
    );

    let tools_config = ToolsConfig(tools_map);
    let json = serde_json::to_value(&tools_config).unwrap();

    assert_eq!(json["slack"], json!(["SLACK_SEND_MESSAGE", "SLACK_LIST_CHANNELS"]));
}

#[test]
fn test_tags_config_with_enabled() {
    let tags_config = TagsConfig {
        enabled: Some(vec![TagType::ReadOnlyHint]),
        disabled: None,
    };

    let json = serde_json::to_value(&tags_config).unwrap();
    assert_eq!(json["enabled"], json!(["READ_ONLY_HINT"]));
    assert!(json.get("disabled").is_none());
}

#[test]
fn test_tags_config_with_disabled() {
    let tags_config = TagsConfig {
        enabled: None,
        disabled: Some(vec![TagType::DestructiveHint, TagType::OpenWorldHint]),
    };

    let json = serde_json::to_value(&tags_config).unwrap();
    assert!(json.get("enabled").is_none());
    assert_eq!(json["disabled"], json!(["DESTRUCTIVE_HINT", "OPEN_WORLD_HINT"]));
}

#[test]
fn test_tags_config_with_both() {
    let tags_config = TagsConfig {
        enabled: Some(vec![TagType::ReadOnlyHint, TagType::IdempotentHint]),
        disabled: Some(vec![TagType::DestructiveHint]),
    };

    let json = serde_json::to_value(&tags_config).unwrap();
    assert_eq!(json["enabled"], json!(["READ_ONLY_HINT", "IDEMPOTENT_HINT"]));
    assert_eq!(json["disabled"], json!(["DESTRUCTIVE_HINT"]));
}

#[test]
fn test_workbench_config() {
    let workbench = WorkbenchConfig {
        proxy_execution: Some(true),
        auto_offload_threshold: Some(20000),
    };

    let json = serde_json::to_value(&workbench).unwrap();
    assert_eq!(json["proxy_execution"], true);
    assert_eq!(json["auto_offload_threshold"], 20000);
}

#[test]
fn test_workbench_config_optional_fields() {
    let workbench = WorkbenchConfig {
        proxy_execution: Some(false),
        auto_offload_threshold: None,
    };

    let json = serde_json::to_value(&workbench).unwrap();
    assert_eq!(json["proxy_execution"], false);
    assert!(json.get("auto_offload_threshold").is_none());
}

#[test]
fn test_tool_execution_request() {
    let request = ToolExecutionRequest {
        tool_slug: "GITHUB_CREATE_ISSUE".to_string(),
        arguments: Some(json!({
            "owner": "composio",
            "repo": "composio",
            "title": "Test issue",
            "body": "Created via Rust SDK"
        })),
    };

    let json = serde_json::to_value(&request).unwrap();
    assert_eq!(json["tool_slug"], "GITHUB_CREATE_ISSUE");
    assert_eq!(json["arguments"]["owner"], "composio");
    assert_eq!(json["arguments"]["title"], "Test issue");
}

#[test]
fn test_tool_execution_request_no_arguments() {
    let request = ToolExecutionRequest {
        tool_slug: "GITHUB_GET_USER".to_string(),
        arguments: None,
    };

    let json = serde_json::to_value(&request).unwrap();
    assert_eq!(json["tool_slug"], "GITHUB_GET_USER");
    assert!(json.get("arguments").is_none());
}

#[test]
fn test_meta_tool_execution_request() {
    let request = MetaToolExecutionRequest {
        slug: MetaToolSlug::ComposioSearchTools,
        arguments: Some(json!({
            "query": "create a GitHub issue"
        })),
    };

    let json = serde_json::to_value(&request).unwrap();
    assert_eq!(json["slug"], "COMPOSIO_SEARCH_TOOLS");
    assert_eq!(json["arguments"]["query"], "create a GitHub issue");
}

#[test]
fn test_meta_tool_execution_request_multi_execute() {
    let request = MetaToolExecutionRequest {
        slug: MetaToolSlug::ComposioMultiExecuteTool,
        arguments: Some(json!({
            "tools": [
                {
                    "tool_slug": "GITHUB_GET_REPOS",
                    "arguments": {"owner": "composio"}
                },
                {
                    "tool_slug": "GITHUB_GET_ISSUES",
                    "arguments": {"owner": "composio", "repo": "composio"}
                }
            ]
        })),
    };

    let json = serde_json::to_value(&request).unwrap();
    assert_eq!(json["slug"], "COMPOSIO_MULTI_EXECUTE_TOOL");
    assert!(json["arguments"]["tools"].is_array());
}

#[test]
fn test_link_request() {
    let request = LinkRequest {
        toolkit: "github".to_string(),
        callback_url: Some("https://example.com/callback".to_string()),
    };

    let json = serde_json::to_value(&request).unwrap();
    assert_eq!(json["toolkit"], "github");
    assert_eq!(json["callback_url"], "https://example.com/callback");
}

#[test]
fn test_link_request_no_callback() {
    let request = LinkRequest {
        toolkit: "gmail".to_string(),
        callback_url: None,
    };

    let json = serde_json::to_value(&request).unwrap();
    assert_eq!(json["toolkit"], "gmail");
    assert!(json.get("callback_url").is_none());
}

#[test]
fn test_session_config_complete() {
    // Test a complete SessionConfig with all fields populated
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

    let config = SessionConfig {
        user_id: "user_123".to_string(),
        toolkits: Some(ToolkitFilter::Enable(vec!["github".to_string(), "gmail".to_string()])),
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

    let json = serde_json::to_value(&config).unwrap();

    // Verify all fields are present and correct
    assert_eq!(json["user_id"], "user_123");
    assert_eq!(json["toolkits"], json!(["github", "gmail"]));
    assert_eq!(json["auth_configs"]["github"], "ac_github_123");
    assert_eq!(json["connected_accounts"]["gmail"], "ca_work_gmail");
    assert_eq!(json["manage_connections"], true);
    assert_eq!(json["tools"]["gmail"]["enable"], json!(["GMAIL_SEND_EMAIL"]));
    assert_eq!(json["tags"]["enabled"], json!(["READ_ONLY_HINT"]));
    assert_eq!(json["tags"]["disabled"], json!(["DESTRUCTIVE_HINT"]));
    assert_eq!(json["workbench"]["proxy_execution"], true);
    assert_eq!(json["workbench"]["auto_offload_threshold"], 20000);
}
