// Composio integration module
//
// This module provides integration with Composio's managed OAuth platform,
// supporting both REST API (v2/v3) and MCP (Model Context Protocol) access.

pub mod health;
pub mod onboarding;
pub mod rest_blocking;
#[cfg(test)]
mod rest_blocking_tests;
pub mod rest_client;
pub mod validation;

pub use health::{check_mcp_health, McpHealthCheck};
pub use onboarding::{CliOnboarding, ComposioOnboarding, OnboardingUx, ServerOnboarding};
pub use rest_blocking::ComposioRestBlockingClient;
pub use rest_client::{
    ComposioAuthConfig, ComposioConnectedAccount, ComposioConnectionLink, ComposioRestClient,
    ComposioToolkitRef,
};
pub use validation::{normalize_toolkit_slug, validate_mcp_config};
