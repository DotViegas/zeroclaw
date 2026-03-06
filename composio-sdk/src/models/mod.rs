//! Data models for Composio API
//!
//! This module contains all request and response models for the Composio Tool Router API,
//! as well as enums for various API types.
//!
//! # Organization
//!
//! - [`request`] - Request models for API calls
//! - [`response`] - Response models from API calls
//! - [`enums`] - Enums for meta tool slugs, tag types, and auth schemes
//!
//! # Examples
//!
//! ```rust
//! use composio_sdk::models::{SessionConfig, ToolkitFilter, MetaToolSlug};
//!
//! // Create a session configuration
//! let config = SessionConfig {
//!     user_id: "user_123".to_string(),
//!     toolkits: Some(ToolkitFilter::Enable(vec!["github".to_string()])),
//!     auth_configs: None,
//!     connected_accounts: None,
//!     manage_connections: Some(true),
//!     tools: None,
//!     tags: None,
//!     workbench: None,
//! };
//! ```

pub mod enums;
pub mod request;
pub mod response;

// ============================================================================
// Enums
// ============================================================================

/// Meta tool slugs for the 5 core Composio meta tools
pub use enums::MetaToolSlug;

/// Tag types for filtering tools by behavior hints
pub use enums::TagType;

/// Authentication schemes supported by toolkits
pub use enums::AuthScheme;

// ============================================================================
// Request Models
// ============================================================================

/// Configuration for creating a Tool Router session
pub use request::SessionConfig;

/// Configuration for connection management
pub use request::ManageConnectionsConfig;

/// Toolkit filter for enabling or disabling specific toolkits
pub use request::ToolkitFilter;

/// Configuration for per-toolkit tool filtering
pub use request::ToolsConfig;

/// Tool filter for a specific toolkit
pub use request::ToolFilter;

/// Configuration for tag-based tool filtering
pub use request::TagsConfig;

/// Configuration for workbench execution
pub use request::WorkbenchConfig;

/// Request to execute a tool
pub use request::ToolExecutionRequest;

/// Request to execute a meta tool
pub use request::MetaToolExecutionRequest;

/// Request to create an authentication link
pub use request::LinkRequest;

// ============================================================================
// Response Models
// ============================================================================

/// Response from session creation
pub use response::SessionResponse;

/// MCP server information
pub use response::McpInfo;

/// Tool schema information
pub use response::ToolSchema;

/// Response from tool execution
pub use response::ToolExecutionResponse;

/// Response from meta tool execution
pub use response::MetaToolExecutionResponse;

/// Response from listing toolkits
pub use response::ToolkitListResponse;

/// Information about a toolkit
pub use response::ToolkitInfo;

/// Metadata about a toolkit
pub use response::ToolkitMeta;

/// Information about a connected account
pub use response::ConnectedAccountInfo;

/// Response from creating an auth link
pub use response::LinkResponse;

/// Error response from API
pub use response::ErrorResponse;
