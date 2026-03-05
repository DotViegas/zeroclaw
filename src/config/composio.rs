//! Composio v3 configuration schema with v3 terminology.
//!
//! This module defines the configuration structure for Composio permanent integration,
//! using v3 terminology exclusively (user_id, connected_account, auth_config).
//! Legacy v1/v2 terminology (entity_id, connection, integration) is supported for
//! backward compatibility with automatic mapping and deprecation warnings.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Composio integration configuration (`[composio]` section).
///
/// Provides access to 1000+ OAuth-connected tools via Composio v3 API.
/// Supports both MCP (Model Context Protocol) and REST API patterns.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ComposioConfig {
    /// Enable Composio integration for 1000+ OAuth tools
    #[serde(default, alias = "enable")]
    pub enabled: bool,

    /// Composio API key (stored encrypted when secrets.encrypt = true)
    #[serde(default)]
    pub api_key: Option<String>,

    /// User ID for Composio v3 session-based architecture (v3 terminology)
    /// Replaces deprecated "entity_id" from v1/v2
    #[serde(default = "default_user_id")]
    pub user_id: String,

    /// Legacy entity_id field for backward compatibility (v1/v2 terminology)
    /// DEPRECATED: Use user_id instead. Will be mapped to user_id internally.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,

    /// MCP (Model Context Protocol) integration configuration
    #[serde(default)]
    pub mcp: McpConfig,

    /// Tool list cache TTL in seconds (default: 3600 = 1 hour)
    #[serde(default = "default_tools_cache_ttl")]
    pub tools_cache_ttl_secs: u64,

    /// OAuth onboarding mode for toolkit authentication
    #[serde(default)]
    pub onboarding_mode: OnboardingMode,

    /// Security configuration for toolkit access control
    #[serde(default)]
    pub security: ComposioSecurityConfig,

    /// Cost tracking configuration
    #[serde(default)]
    pub cost_tracking: CostTrackingConfig,
}

/// MCP (Model Context Protocol) configuration (`[composio.mcp]` section).
///
/// Enables Pattern 2 (meta-tools) for dynamic tool discovery and execution.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpConfig {
    /// Enable MCP integration (Pattern 2: meta-tools)
    #[serde(default)]
    pub enabled: bool,

    /// MCP server ID from Composio dashboard
    #[serde(default)]
    pub server_id: Option<String>,

    /// MCP server URL (alternative to server_id)
    #[serde(default)]
    pub mcp_url: Option<String>,

    /// User ID override for MCP (defaults to composio.user_id)
    #[serde(default)]
    pub user_id: Option<String>,

    /// Optional separate API key for MCP (if different from composio.api_key)
    #[serde(default)]
    pub api_key: Option<String>,

    /// Toolkits enabled in MCP server (e.g., ["gmail", "slack", "github"])
    #[serde(default)]
    pub toolkits: Vec<String>,

    /// Auth config IDs per toolkit (for reference)
    #[serde(default)]
    pub auth_configs: HashMap<String, String>,

    /// TTL for tools list cache in seconds (default: 3600 = 1 hour)
    #[serde(default = "default_tools_cache_ttl")]
    pub tools_cache_ttl_secs: u64,

    /// Callback base URL for web-based OAuth (mode: web_callback)
    #[serde(default)]
    pub callback_base_url: Option<String>,

    /// Local callback listen address (mode: cli_callback_local)
    #[serde(default)]
    pub callback_listen_addr: Option<String>,
}

/// Security configuration for Composio tool access control.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ComposioSecurityConfig {
    /// Allowed toolkits (if set, only these toolkits can be used)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_toolkits: Option<Vec<String>>,

    /// Denied toolkits (these toolkits are blocked even if in allowed list)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub denied_toolkits: Option<Vec<String>>,

    /// Rate limit per user_id (calls per minute)
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_minute: u32,
}

/// Cost tracking configuration for Composio API usage.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CostTrackingConfig {
    /// Enable cost tracking
    #[serde(default)]
    pub enabled: bool,

    /// Default pricing tier for cost estimation
    #[serde(default)]
    pub default_pricing_tier: PricingTier,

    /// Daily budget per user in USD (None = unlimited)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub daily_budget_per_user_usd: Option<f64>,

    /// Per-toolkit cost limits in USD (None = unlimited)
    #[serde(default)]
    pub toolkit_cost_limits_usd: HashMap<String, f64>,

    /// Budget warning threshold (0.0 to 1.0, default 0.8 = 80%)
    #[serde(default = "default_budget_warning_threshold")]
    pub budget_warning_threshold: f64,
}

/// Composio pricing tier for cost estimation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum PricingTier {
    /// Free tier: limited calls, no cost
    Free,
    /// Starter tier: $0.001 per call
    #[default]
    Starter,
    /// Professional tier: $0.0005 per call
    Professional,
    /// Enterprise tier: custom pricing (estimated at $0.0003 per call)
    Enterprise,
}

/// OAuth onboarding mode for toolkit authentication.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum OnboardingMode {
    /// CLI: auto-open browser (default)
    #[default]
    CliAutoOpen,

    /// CLI: print OAuth URL to stdout only
    CliPrintOnly,

    /// CLI: start local callback server for OAuth redirect
    CliCallbackLocal,

    /// Web: return OAuth URL to caller (for web UI integration)
    WebCallback,
}

// Default value functions

fn default_user_id() -> String {
    "default_user".into()
}

fn default_tools_cache_ttl() -> u64 {
    3600 // 1 hour
}

fn default_rate_limit() -> u32 {
    60 // 60 calls per minute
}

fn default_budget_warning_threshold() -> f64 {
    0.8 // 80%
}

// Default implementations

impl Default for ComposioConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: None,
            user_id: default_user_id(),
            entity_id: None,
            mcp: McpConfig::default(),
            tools_cache_ttl_secs: default_tools_cache_ttl(),
            onboarding_mode: OnboardingMode::default(),
            security: ComposioSecurityConfig::default(),
            cost_tracking: CostTrackingConfig::default(),
        }
    }
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            server_id: None,
            mcp_url: None,
            user_id: None,
            api_key: None,
            toolkits: Vec::new(),
            auth_configs: HashMap::new(),
            tools_cache_ttl_secs: default_tools_cache_ttl(),
            callback_base_url: None,
            callback_listen_addr: None,
        }
    }
}

impl Default for ComposioSecurityConfig {
    fn default() -> Self {
        Self {
            allowed_toolkits: None,
            denied_toolkits: None,
            rate_limit_per_minute: default_rate_limit(),
        }
    }
}

impl Default for CostTrackingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_pricing_tier: PricingTier::default(),
            daily_budget_per_user_usd: None,
            toolkit_cost_limits_usd: HashMap::new(),
            budget_warning_threshold: default_budget_warning_threshold(),
        }
    }
}

// Helper methods

impl ComposioConfig {
    /// Get the effective user_id, handling legacy entity_id mapping.
    /// Returns (user_id, is_legacy) tuple.
    pub fn effective_user_id(&self) -> (String, bool) {
        if let Some(ref entity_id) = self.entity_id {
            // Legacy entity_id provided - map to user_id
            (entity_id.clone(), true)
        } else {
            // Use v3 user_id
            (self.user_id.clone(), false)
        }
    }

    /// Check if legacy entity_id field is being used.
    pub fn is_using_legacy_entity_id(&self) -> bool {
        self.entity_id.is_some()
    }

    /// Migrate legacy configuration to new format.
    /// 
    /// This function performs the following migrations:
    /// 1. Maps entity_id to user_id if entity_id is present
    /// 2. Suggests enabling MCP if only REST API is configured
    /// 3. Returns a migrated config and a list of migration messages
    pub fn migrate(&self) -> (Self, Vec<String>) {
        let mut migrated = self.clone();
        let mut messages = Vec::new();

        // Migration 1: entity_id -> user_id
        if let Some(ref entity_id) = self.entity_id {
            migrated.user_id = entity_id.clone();
            migrated.entity_id = None;
            messages.push(format!(
                "Migrated entity_id '{}' to user_id. Please update your config.toml to use 'user_id' instead of 'entity_id'.",
                entity_id
            ));
        }

        // Migration 2: Suggest MCP if only REST API is configured
        if self.enabled && self.api_key.is_some() && !self.mcp.enabled {
            messages.push(
                "Consider enabling MCP (Pattern 2) for better functionality. \
                 Add [composio.mcp] section with enabled=true and server_id to your config.toml. \
                 Pattern 1 (REST API) is deprecated and will be removed in a future version."
                    .to_string(),
            );
        }

        // Migration 3: Ensure MCP has required fields if enabled
        if migrated.mcp.enabled && migrated.mcp.server_id.is_none() && migrated.mcp.mcp_url.is_none() {
            messages.push(
                "MCP is enabled but neither server_id nor mcp_url is provided. \
                 Please add either composio.mcp.server_id or composio.mcp.mcp_url to your config.toml."
                    .to_string(),
            );
        }

        (migrated, messages)
    }

    /// Apply automatic migrations and print warnings.
    /// Returns the migrated configuration.
    pub fn auto_migrate(&self) -> Self {
        let (migrated, messages) = self.migrate();
        
        if !messages.is_empty() {
            eprintln!("\n⚠ Composio Configuration Migration Warnings:");
            for (i, msg) in messages.iter().enumerate() {
                eprintln!("  {}. {}", i + 1, msg);
            }
            eprintln!();
        }

        migrated
    }

    /// Validate the entire Composio configuration.
    pub fn validate(&self) -> Result<(), String> {
        // Validate MCP config
        self.mcp.validate()?;

        // Validate security config
        self.security.validate()?;

        // Warn if using legacy entity_id
        if self.is_using_legacy_entity_id() {
            eprintln!(
                "WARNING: composio.entity_id is deprecated. Please use composio.user_id instead. \
                 The entity_id field will be removed in a future version."
            );
        }

        Ok(())
    }
}

impl McpConfig {
    /// Get the effective user_id for MCP (override or parent config).
    pub fn effective_user_id(&self, parent_user_id: &str) -> String {
        self.user_id
            .as_ref()
            .cloned()
            .unwrap_or_else(|| parent_user_id.to_string())
    }

    /// Validate MCP configuration.
    /// Returns error if mcp.enabled is true but neither server_id nor mcp_url is provided.
    pub fn validate(&self) -> Result<(), String> {
        if self.enabled {
            if self.server_id.is_none() && self.mcp_url.is_none() {
                return Err(
                    "MCP is enabled but neither server_id nor mcp_url is provided. \
                     Please set either composio.mcp.server_id or composio.mcp.mcp_url."
                        .to_string(),
                );
            }
        }
        Ok(())
    }
}

impl ComposioSecurityConfig {
    /// Validate security configuration.
    pub fn validate(&self) -> Result<(), String> {
        // Check for conflicting allowlist/denylist
        if let (Some(allowed), Some(denied)) = (&self.allowed_toolkits, &self.denied_toolkits) {
            let conflicts: Vec<_> = allowed
                .iter()
                .filter(|toolkit| denied.contains(toolkit))
                .collect();
            if !conflicts.is_empty() {
                return Err(format!(
                    "Toolkits cannot be in both allowed and denied lists: {:?}",
                    conflicts
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Feature: composio-permanent-integration
    // Task 30.1: Unit tests for configuration schema validation

    #[test]
    fn test_default_composio_config() {
        let config = ComposioConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.user_id, "default_user");
        assert_eq!(config.tools_cache_ttl_secs, 3600);
        assert_eq!(config.security.rate_limit_per_minute, 60);
        assert!(!config.mcp.enabled);
    }

    #[test]
    fn test_mcp_validation_fails_when_enabled_without_server_id_or_url() {
        let mut config = McpConfig::default();
        config.enabled = true;
        config.server_id = None;
        config.mcp_url = None;

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("neither server_id nor mcp_url is provided"));
    }

    #[test]
    fn test_mcp_validation_succeeds_with_server_id() {
        let mut config = McpConfig::default();
        config.enabled = true;
        config.server_id = Some("test_server_id".to_string());

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_mcp_validation_succeeds_with_mcp_url() {
        let mut config = McpConfig::default();
        config.enabled = true;
        config.mcp_url = Some("https://mcp.composio.dev".to_string());

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_mcp_validation_succeeds_with_both_server_id_and_url() {
        let mut config = McpConfig::default();
        config.enabled = true;
        config.server_id = Some("test_server_id".to_string());
        config.mcp_url = Some("https://mcp.composio.dev".to_string());

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_mcp_validation_succeeds_when_disabled() {
        let mut config = McpConfig::default();
        config.enabled = false;
        config.server_id = None;
        config.mcp_url = None;

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_effective_user_id_uses_user_id_when_no_entity_id() {
        let config = ComposioConfig {
            user_id: "test_user".to_string(),
            entity_id: None,
            ..Default::default()
        };

        let (effective_id, is_legacy) = config.effective_user_id();
        assert_eq!(effective_id, "test_user");
        assert!(!is_legacy);
    }

    #[test]
    fn test_effective_user_id_uses_entity_id_when_provided() {
        let config = ComposioConfig {
            user_id: "test_user".to_string(),
            entity_id: Some("legacy_entity".to_string()),
            ..Default::default()
        };

        let (effective_id, is_legacy) = config.effective_user_id();
        assert_eq!(effective_id, "legacy_entity");
        assert!(is_legacy);
    }

    #[test]
    fn test_is_using_legacy_entity_id() {
        let config_with_entity = ComposioConfig {
            entity_id: Some("legacy_entity".to_string()),
            ..Default::default()
        };
        assert!(config_with_entity.is_using_legacy_entity_id());

        let config_without_entity = ComposioConfig {
            entity_id: None,
            ..Default::default()
        };
        assert!(!config_without_entity.is_using_legacy_entity_id());
    }

    #[test]
    fn test_mcp_effective_user_id_uses_override() {
        let mcp_config = McpConfig {
            user_id: Some("mcp_user".to_string()),
            ..Default::default()
        };

        let effective_id = mcp_config.effective_user_id("parent_user");
        assert_eq!(effective_id, "mcp_user");
    }

    #[test]
    fn test_mcp_effective_user_id_uses_parent_when_no_override() {
        let mcp_config = McpConfig {
            user_id: None,
            ..Default::default()
        };

        let effective_id = mcp_config.effective_user_id("parent_user");
        assert_eq!(effective_id, "parent_user");
    }

    #[test]
    fn test_security_validation_fails_with_conflicting_allowlist_denylist() {
        let security_config = ComposioSecurityConfig {
            allowed_toolkits: Some(vec!["gmail".to_string(), "slack".to_string()]),
            denied_toolkits: Some(vec!["slack".to_string(), "github".to_string()]),
            rate_limit_per_minute: 60,
        };

        let result = security_config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("cannot be in both allowed and denied lists"));
    }

    #[test]
    fn test_security_validation_succeeds_with_non_conflicting_lists() {
        let security_config = ComposioSecurityConfig {
            allowed_toolkits: Some(vec!["gmail".to_string(), "slack".to_string()]),
            denied_toolkits: Some(vec!["github".to_string(), "notion".to_string()]),
            rate_limit_per_minute: 60,
        };

        assert!(security_config.validate().is_ok());
    }

    #[test]
    fn test_security_validation_succeeds_with_only_allowlist() {
        let security_config = ComposioSecurityConfig {
            allowed_toolkits: Some(vec!["gmail".to_string()]),
            denied_toolkits: None,
            rate_limit_per_minute: 60,
        };

        assert!(security_config.validate().is_ok());
    }

    #[test]
    fn test_security_validation_succeeds_with_only_denylist() {
        let security_config = ComposioSecurityConfig {
            allowed_toolkits: None,
            denied_toolkits: Some(vec!["github".to_string()]),
            rate_limit_per_minute: 60,
        };

        assert!(security_config.validate().is_ok());
    }

    #[test]
    fn test_security_validation_succeeds_with_no_lists() {
        let security_config = ComposioSecurityConfig {
            allowed_toolkits: None,
            denied_toolkits: None,
            rate_limit_per_minute: 60,
        };

        assert!(security_config.validate().is_ok());
    }

    #[test]
    fn test_composio_config_validation_propagates_mcp_errors() {
        let config = ComposioConfig {
            mcp: McpConfig {
                enabled: true,
                server_id: None,
                mcp_url: None,
                ..Default::default()
            },
            ..Default::default()
        };

        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_composio_config_validation_propagates_security_errors() {
        let config = ComposioConfig {
            security: ComposioSecurityConfig {
                allowed_toolkits: Some(vec!["gmail".to_string()]),
                denied_toolkits: Some(vec!["gmail".to_string()]),
                rate_limit_per_minute: 60,
            },
            ..Default::default()
        };

        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_composio_config_validation_succeeds_with_valid_config() {
        let config = ComposioConfig {
            enabled: true,
            api_key: Some("test_key".to_string()),
            user_id: "test_user".to_string(),
            mcp: McpConfig {
                enabled: true,
                server_id: Some("test_server".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_onboarding_mode_default() {
        let mode = OnboardingMode::default();
        assert!(matches!(mode, OnboardingMode::CliAutoOpen));
    }

    #[test]
    fn test_pricing_tier_default() {
        let tier = PricingTier::default();
        assert_eq!(tier, PricingTier::Starter);
    }

    #[test]
    fn test_cost_tracking_config_default() {
        let config = CostTrackingConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.default_pricing_tier, PricingTier::Starter);
        assert_eq!(config.budget_warning_threshold, 0.8);
        assert!(config.daily_budget_per_user_usd.is_none());
    }

    #[test]
    fn test_mcp_config_with_toolkits() {
        let config = McpConfig {
            enabled: true,
            server_id: Some("test_server".to_string()),
            toolkits: vec!["gmail".to_string(), "slack".to_string(), "github".to_string()],
            ..Default::default()
        };

        assert_eq!(config.toolkits.len(), 3);
        assert!(config.toolkits.contains(&"gmail".to_string()));
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_mcp_config_with_auth_configs() {
        let mut auth_configs = HashMap::new();
        auth_configs.insert("gmail".to_string(), "auth_config_1".to_string());
        auth_configs.insert("slack".to_string(), "auth_config_2".to_string());

        let config = McpConfig {
            enabled: true,
            mcp_url: Some("https://mcp.composio.dev".to_string()),
            auth_configs,
            ..Default::default()
        };

        assert_eq!(config.auth_configs.len(), 2);
        assert_eq!(
            config.auth_configs.get("gmail"),
            Some(&"auth_config_1".to_string())
        );
    }

    #[test]
    fn test_security_config_with_custom_rate_limit() {
        let config = ComposioSecurityConfig {
            rate_limit_per_minute: 120,
            ..Default::default()
        };

        assert_eq!(config.rate_limit_per_minute, 120);
    }

    #[test]
    fn test_cost_tracking_with_toolkit_limits() {
        let mut toolkit_limits = HashMap::new();
        toolkit_limits.insert("gmail".to_string(), 10.0);
        toolkit_limits.insert("slack".to_string(), 5.0);

        let config = CostTrackingConfig {
            enabled: true,
            toolkit_cost_limits_usd: toolkit_limits,
            ..Default::default()
        };

        assert_eq!(config.toolkit_cost_limits_usd.len(), 2);
        assert_eq!(
            config.toolkit_cost_limits_usd.get("gmail"),
            Some(&10.0)
        );
    }

    #[test]
    fn test_mcp_config_with_callback_urls() {
        let config = McpConfig {
            enabled: true,
            server_id: Some("test_server".to_string()),
            callback_base_url: Some("https://example.com/callback".to_string()),
            callback_listen_addr: Some("127.0.0.1:8080".to_string()),
            ..Default::default()
        };

        assert_eq!(
            config.callback_base_url,
            Some("https://example.com/callback".to_string())
        );
        assert_eq!(
            config.callback_listen_addr,
            Some("127.0.0.1:8080".to_string())
        );
    }

    #[test]
    fn test_empty_toolkits_list_is_valid() {
        let config = McpConfig {
            enabled: true,
            server_id: Some("test_server".to_string()),
            toolkits: vec![],
            ..Default::default()
        };

        assert!(config.validate().is_ok());
        assert_eq!(config.toolkits.len(), 0);
    }

    #[test]
    fn test_security_config_empty_lists_are_valid() {
        let config = ComposioSecurityConfig {
            allowed_toolkits: Some(vec![]),
            denied_toolkits: Some(vec![]),
            rate_limit_per_minute: 60,
        };

        assert!(config.validate().is_ok());
    }

    // Feature: composio-permanent-integration
    // Task 38.2: Tests for config migration helper

    #[test]
    fn test_migrate_entity_id_to_user_id() {
        let config = ComposioConfig {
            enabled: true,
            user_id: "default_user".to_string(),
            entity_id: Some("legacy_entity".to_string()),
            api_key: Some("test_key".to_string()),
            ..Default::default()
        };

        let (migrated, messages) = config.migrate();
        
        assert_eq!(migrated.user_id, "legacy_entity");
        assert!(migrated.entity_id.is_none());
        assert_eq!(messages.len(), 2); // entity_id migration + MCP suggestion
        assert!(messages[0].contains("Migrated entity_id"));
    }

    #[test]
    fn test_migrate_suggests_mcp_for_rest_only_config() {
        let config = ComposioConfig {
            enabled: true,
            api_key: Some("test_key".to_string()),
            mcp: McpConfig {
                enabled: false,
                ..Default::default()
            },
            ..Default::default()
        };

        let (_, messages) = config.migrate();
        
        assert!(!messages.is_empty());
        assert!(messages.iter().any(|m| m.contains("Consider enabling MCP")));
    }

    #[test]
    fn test_migrate_warns_about_missing_mcp_config() {
        let config = ComposioConfig {
            enabled: true,
            mcp: McpConfig {
                enabled: true,
                server_id: None,
                mcp_url: None,
                ..Default::default()
            },
            ..Default::default()
        };

        let (_, messages) = config.migrate();
        
        assert!(messages.iter().any(|m| m.contains("neither server_id nor mcp_url")));
    }

    #[test]
    fn test_migrate_no_warnings_for_proper_mcp_config() {
        let config = ComposioConfig {
            enabled: true,
            api_key: Some("test_key".to_string()),
            user_id: "test_user".to_string(),
            entity_id: None,
            mcp: McpConfig {
                enabled: true,
                server_id: Some("test_server".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let (migrated, messages) = config.migrate();
        
        assert_eq!(migrated.user_id, "test_user");
        assert!(messages.is_empty());
    }

    #[test]
    fn test_auto_migrate_returns_migrated_config() {
        let config = ComposioConfig {
            enabled: true,
            user_id: "default_user".to_string(),
            entity_id: Some("legacy_entity".to_string()),
            ..Default::default()
        };

        let migrated = config.auto_migrate();
        
        assert_eq!(migrated.user_id, "legacy_entity");
        assert!(migrated.entity_id.is_none());
    }
}
