//! Request models for Composio API
//!
//! This module contains all request body structures used when making API calls
//! to the Composio Tool Router. These models are serialized to JSON and sent
//! in HTTP request bodies.
//!
//! # Main Request Types
//!
//! - [`SessionConfig`] - Configuration for creating a Tool Router session
//! - [`ToolExecutionRequest`] - Request to execute a tool
//! - [`MetaToolExecutionRequest`] - Request to execute a meta tool
//! - [`LinkRequest`] - Request to create an authentication link
//!
//! # Configuration Types
//!
//! - [`ToolkitFilter`] - Enable or disable specific toolkits
//! - [`ToolsConfig`] - Per-toolkit tool filtering
//! - [`ToolFilter`] - Enable or disable specific tools within a toolkit
//! - [`TagsConfig`] - Tag-based tool filtering (readOnlyHint, destructiveHint, etc.)
//! - [`WorkbenchConfig`] - Workbench execution settings
//! - [`ManageConnectionsConfig`] - Connection management settings
//!
//! # Example
//!
//! ```rust
//! use composio_sdk::models::{SessionConfig, ToolkitFilter};
//!
//! let config = SessionConfig {
//!     user_id: "user_123".to_string(),
//!     toolkits: Some(ToolkitFilter::Enable(vec!["github".to_string()])),
//!     auth_configs: None,
//!     connected_accounts: None,
//!     manage_connections: None,
//!     tools: None,
//!     tags: None,
//!     workbench: None,
//! };
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::enums::{MetaToolSlug, TagType};

/// Configuration for creating a Tool Router session
///
/// This struct defines all the options available when creating a new session.
/// Sessions provide scoped access to tools and toolkits for a specific user.
///
/// # Fields
///
/// * `user_id` - User identifier for session isolation (required)
/// * `toolkits` - Optional toolkit filter (enable or disable specific toolkits)
/// * `auth_configs` - Optional per-toolkit auth config overrides
/// * `connected_accounts` - Optional per-toolkit connected account selection
/// * `manage_connections` - Optional connection management configuration
/// * `tools` - Optional per-toolkit tool filtering
/// * `tags` - Optional tag-based tool filtering
/// * `workbench` - Optional workbench configuration
///
/// # Example
///
/// ```rust
/// use composio_sdk::models::{SessionConfig, ToolkitFilter};
/// use std::collections::HashMap;
///
/// let config = SessionConfig {
///     user_id: "user_123".to_string(),
///     toolkits: Some(ToolkitFilter::Enable(vec!["github".to_string(), "gmail".to_string()])),
///     auth_configs: {
///         let mut map = HashMap::new();
///         map.insert("github".to_string(), "ac_custom_config".to_string());
///         Some(map)
///     },
///     connected_accounts: None,
///     manage_connections: None,
///     tools: None,
///     tags: None,
///     workbench: None,
///  };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toolkits: Option<ToolkitFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_configs: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connected_accounts: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manage_connections: Option<ManageConnectionsConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<TagsConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workbench: Option<WorkbenchConfig>,
}

/// Configuration for connection management
///
/// Controls whether the agent automatically prompts users with Connect Links
/// during chat when authentication is needed (in-chat authentication).
///
/// # Variants
///
/// * `Bool(bool)` - Simple boolean flag (true = enabled, false = disabled)
/// * `Detailed` - Detailed configuration with additional options
///
/// # Example
///
/// ```rust
/// use composio_sdk::models::ManageConnectionsConfig;
///
/// // Simple boolean
/// let simple = ManageConnectionsConfig::Bool(true);
///
/// // Detailed configuration
/// let detailed = ManageConnectionsConfig::Detailed {
///     enabled: true,
///     enable_wait_for_connections: Some(true),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ManageConnectionsConfig {
    /// Simple boolean flag
    Bool(bool),
    /// Detailed configuration
    Detailed {
        enabled: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        enable_wait_for_connections: Option<bool>,
    },
}

/// Toolkit filter for enabling or disabling toolkits
///
/// Controls which toolkits are accessible in a session. By default, all toolkits
/// are accessible via COMPOSIO_SEARCH_TOOLS. Use this filter to restrict access.
///
/// # Variants
///
/// * `Enable(Vec<String>)` - Only allow specified toolkits (allowlist)
/// * `Disable { disable: Vec<String> }` - Allow all except specified toolkits (denylist)
///
/// # Example
///
/// ```rust
/// use composio_sdk::models::ToolkitFilter;
///
/// // Enable only GitHub and Gmail
/// let enable = ToolkitFilter::Enable(vec!["github".to_string(), "gmail".to_string()]);
///
/// // Disable Exa and Firecrawl
/// let disable = ToolkitFilter::Disable {
///     disable: vec!["exa".to_string(), "firecrawl".to_string()],
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolkitFilter {
    Enable(Vec<String>),
    Disable { disable: Vec<String> },
}

/// Configuration for per-toolkit tool filtering
/// Maps toolkit names to their tool filter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsConfig(pub HashMap<String, ToolFilter>);

/// Tool filter for a specific toolkit
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolFilter {
    /// Enable specific tools
    Enable { enable: Vec<String> },
    /// Disable specific tools
    Disable { disable: Vec<String> },
    /// Shorthand: array of tool names to enable
    EnableList(Vec<String>),
}

/// Configuration for tag-based tool filtering
/// Tags are MCP annotation hints for filtering tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagsConfig {
    /// Tags that the tool must have at least one of
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<Vec<TagType>>,
    /// Tags that the tool must NOT have any of
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<Vec<TagType>>,
}

/// Configuration for workbench
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkbenchConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(alias = "proxy_execution_enabled")]
    pub proxy_execution: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_offload_threshold: Option<u32>,
}

/// Request to execute a tool
#[derive(Debug, Clone, Serialize)]
pub struct ToolExecutionRequest {
    pub tool_slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
}

/// Request to execute a meta tool
#[derive(Debug, Clone, Serialize)]
pub struct MetaToolExecutionRequest {
    pub slug: MetaToolSlug,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
}

/// Request to create an authentication link
#[derive(Debug, Clone, Serialize)]
pub struct LinkRequest {
    pub toolkit: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_session_config_minimal_serialization() {
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

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("user_123"));
        assert!(!json.contains("toolkits"));
        assert!(!json.contains("auth_configs"));
    }

    #[test]
    fn test_session_config_with_toolkits_enable() {
        let config = SessionConfig {
            user_id: "user_123".to_string(),
            toolkits: Some(ToolkitFilter::Enable(vec!["github".to_string(), "gmail".to_string()])),
            auth_configs: None,
            connected_accounts: None,
            manage_connections: None,
            tools: None,
            tags: None,
            workbench: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert!(parsed["toolkits"].is_array());
        let toolkits = parsed["toolkits"].as_array().unwrap();
        assert_eq!(toolkits.len(), 2);
    }

    #[test]
    fn test_session_config_with_toolkits_disable() {
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

        let json = serde_json::to_string(&config).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert!(parsed["toolkits"].is_object());
        assert!(parsed["toolkits"]["disable"].is_array());
    }

    #[test]
    fn test_session_config_with_auth_configs() {
        let mut auth_configs = HashMap::new();
        auth_configs.insert("github".to_string(), "ac_custom".to_string());
        
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

        let json = serde_json::to_string(&config).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["auth_configs"]["github"], "ac_custom");
    }

    #[test]
    fn test_session_config_with_manage_connections_bool() {
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

        let json = serde_json::to_string(&config).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["manage_connections"], true);
    }

    #[test]
    fn test_session_config_with_manage_connections_detailed() {
        let config = SessionConfig {
            user_id: "user_123".to_string(),
            toolkits: None,
            auth_configs: None,
            connected_accounts: None,
            manage_connections: Some(ManageConnectionsConfig::Detailed {
                enabled: true,
                enable_wait_for_connections: Some(false),
            }),
            tools: None,
            tags: None,
            workbench: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["manage_connections"]["enabled"], true);
        assert_eq!(parsed["manage_connections"]["enable_wait_for_connections"], false);
    }

    #[test]
    fn test_session_config_with_tools() {
        let mut tools_map = HashMap::new();
        tools_map.insert(
            "github".to_string(),
            ToolFilter::EnableList(vec!["GITHUB_CREATE_ISSUE".to_string()]),
        );
        
        let config = SessionConfig {
            user_id: "user_123".to_string(),
            toolkits: None,
            auth_configs: None,
            connected_accounts: None,
            manage_connections: None,
            tools: Some(ToolsConfig(tools_map)),
            tags: None,
            workbench: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert!(parsed["tools"]["github"].is_array());
    }

    #[test]
    fn test_session_config_with_tags() {
        let config = SessionConfig {
            user_id: "user_123".to_string(),
            toolkits: None,
            auth_configs: None,
            connected_accounts: None,
            manage_connections: None,
            tools: None,
            tags: Some(TagsConfig {
                enabled: Some(vec![TagType::ReadOnlyHint]),
                disabled: Some(vec![TagType::DestructiveHint]),
            }),
            workbench: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert!(parsed["tags"]["enabled"].is_array());
        assert!(parsed["tags"]["disabled"].is_array());
    }

    #[test]
    fn test_session_config_with_workbench() {
        let config = SessionConfig {
            user_id: "user_123".to_string(),
            toolkits: None,
            auth_configs: None,
            connected_accounts: None,
            manage_connections: None,
            tools: None,
            tags: None,
            workbench: Some(WorkbenchConfig {
                proxy_execution: Some(true),
                auto_offload_threshold: Some(1000),
            }),
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["workbench"]["proxy_execution"], true);
        assert_eq!(parsed["workbench"]["auto_offload_threshold"], 1000);
    }

    #[test]
    fn test_toolkit_filter_enable_serialization() {
        let filter = ToolkitFilter::Enable(vec!["github".to_string(), "gmail".to_string()]);
        let json = serde_json::to_string(&filter).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_toolkit_filter_disable_serialization() {
        let filter = ToolkitFilter::Disable {
            disable: vec!["exa".to_string()],
        };
        let json = serde_json::to_string(&filter).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert!(parsed.is_object());
        assert!(parsed["disable"].is_array());
    }

    #[test]
    fn test_tool_filter_enable_serialization() {
        let filter = ToolFilter::Enable {
            enable: vec!["GITHUB_CREATE_ISSUE".to_string()],
        };
        let json = serde_json::to_string(&filter).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert!(parsed.is_object());
        assert!(parsed["enable"].is_array());
    }

    #[test]
    fn test_tool_filter_disable_serialization() {
        let filter = ToolFilter::Disable {
            disable: vec!["GITHUB_DELETE_REPO".to_string()],
        };
        let json = serde_json::to_string(&filter).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert!(parsed.is_object());
        assert!(parsed["disable"].is_array());
    }

    #[test]
    fn test_tool_filter_enable_list_serialization() {
        let filter = ToolFilter::EnableList(vec!["GITHUB_CREATE_ISSUE".to_string()]);
        let json = serde_json::to_string(&filter).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert!(parsed.is_array());
    }

    #[test]
    fn test_tool_execution_request_serialization() {
        let request = ToolExecutionRequest {
            tool_slug: "GITHUB_CREATE_ISSUE".to_string(),
            arguments: Some(serde_json::json!({
                "owner": "composio",
                "repo": "composio",
                "title": "Test issue"
            })),
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["tool_slug"], "GITHUB_CREATE_ISSUE");
        assert!(parsed["arguments"].is_object());
        assert_eq!(parsed["arguments"]["owner"], "composio");
    }

    #[test]
    fn test_tool_execution_request_without_arguments() {
        let request = ToolExecutionRequest {
            tool_slug: "GITHUB_GET_USER".to_string(),
            arguments: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["tool_slug"], "GITHUB_GET_USER");
        assert!(parsed.get("arguments").is_none());
    }

    #[test]
    fn test_meta_tool_execution_request_serialization() {
        let request = MetaToolExecutionRequest {
            slug: MetaToolSlug::ComposioSearchTools,
            arguments: Some(serde_json::json!({
                "query": "create a GitHub issue"
            })),
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["slug"], "COMPOSIO_SEARCH_TOOLS");
        assert!(parsed["arguments"].is_object());
    }

    #[test]
    fn test_link_request_serialization() {
        let request = LinkRequest {
            toolkit: "github".to_string(),
            callback_url: Some("https://example.com/callback".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["toolkit"], "github");
        assert_eq!(parsed["callback_url"], "https://example.com/callback");
    }

    #[test]
    fn test_link_request_without_callback() {
        let request = LinkRequest {
            toolkit: "gmail".to_string(),
            callback_url: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["toolkit"], "gmail");
        assert!(parsed.get("callback_url").is_none());
    }

    #[test]
    fn test_tags_config_serialization() {
        let config = TagsConfig {
            enabled: Some(vec![TagType::ReadOnlyHint, TagType::IdempotentHint]),
            disabled: Some(vec![TagType::DestructiveHint]),
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert!(parsed["enabled"].is_array());
        assert!(parsed["disabled"].is_array());
        assert_eq!(parsed["enabled"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["disabled"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_workbench_config_serialization() {
        let config = WorkbenchConfig {
            proxy_execution: Some(true),
            auto_offload_threshold: Some(500),
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["proxy_execution"], true);
        assert_eq!(parsed["auto_offload_threshold"], 500);
    }

    #[test]
    fn test_workbench_config_partial_serialization() {
        let config = WorkbenchConfig {
            proxy_execution: Some(false),
            auto_offload_threshold: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["proxy_execution"], false);
        assert!(parsed.get("auto_offload_threshold").is_none());
    }
}
