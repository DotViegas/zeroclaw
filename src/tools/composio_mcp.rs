// Composio MCP Tool Wrapper — converts MCP tools to ZeroClaw tools
//
// This module wraps Composio MCP tools as native ZeroClaw tools, allowing
// seamless integration with the existing tool system.

use super::traits::{Tool, ToolResult};
use crate::composio::ComposioOnboarding;
use crate::mcp::{ComposioMcpClient, McpTool};
use crate::security::policy::{SecurityPolicy, ToolOperation};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// A ZeroClaw tool that wraps a Composio MCP tool
pub struct ComposioMcpTool {
    client: Arc<ComposioMcpClient>,
    tool_name: String,
    description: String,
    schema: Value,
    security: Arc<SecurityPolicy>,
    onboarding: Option<Arc<dyn ComposioOnboarding>>,
}

impl ComposioMcpTool {
    /// Create a new Composio MCP tool wrapper
    ///
    /// # Arguments
    /// * `client` - Shared MCP client for executing tools
    /// * `mcp_tool` - MCP tool definition from the server
    /// * `security` - Security policy for access control
    pub fn new(
        client: Arc<ComposioMcpClient>,
        mcp_tool: McpTool,
        security: Arc<SecurityPolicy>,
    ) -> Self {
        Self::new_with_onboarding(client, mcp_tool, security, None)
    }

    /// Create a new Composio MCP tool wrapper with onboarding support
    ///
    /// # Arguments
    /// * `client` - Shared MCP client for executing tools
    /// * `mcp_tool` - MCP tool definition from the server
    /// * `security` - Security policy for access control
    /// * `onboarding` - Optional onboarding handler for OAuth flows
    pub fn new_with_onboarding(
        client: Arc<ComposioMcpClient>,
        mcp_tool: McpTool,
        security: Arc<SecurityPolicy>,
        onboarding: Option<Arc<dyn ComposioOnboarding>>,
    ) -> Self {
        let description = mcp_tool.description.clone().unwrap_or_else(|| {
            format!(
                "Composio MCP tool: {} (via MCP server)",
                mcp_tool.name
            )
        });

        Self {
            client,
            tool_name: mcp_tool.name,
            description,
            schema: mcp_tool.input_schema,
            security,
            onboarding,
        }
    }

    /// Get the underlying MCP tool name
    pub fn mcp_tool_name(&self) -> &str {
        &self.tool_name
    }

    /// Infer the toolkit slug from the tool name
    ///
    /// Examples:
    /// - "GMAIL_SEND_EMAIL" -> Some("gmail")
    /// - "github-create-issue" -> Some("github")
    /// - "SLACK_POST_MESSAGE" -> Some("slack")
    fn infer_toolkit_slug(&self) -> Option<String> {
        let name = self.tool_name.as_str();
        
        // Try underscore separator first (most common)
        if let Some(prefix) = name.split('_').next() {
            if !prefix.is_empty() {
                let normalized = crate::composio::normalize_toolkit_slug(prefix);
                if !normalized.is_empty() {
                    return Some(normalized);
                }
            }
        }
        
        // Try hyphen separator
        if let Some(prefix) = name.split('-').next() {
            if !prefix.is_empty() {
                let normalized = crate::composio::normalize_toolkit_slug(prefix);
                if !normalized.is_empty() {
                    return Some(normalized);
                }
            }
        }
        
        // Try camelCase (e.g., "gmailSendEmail")
        if name.chars().any(|c| c.is_uppercase()) {
            let mut result = String::new();
            for c in name.chars() {
                if c.is_uppercase() {
                    break;
                }
                result.push(c);
            }
            if !result.is_empty() {
                let normalized = crate::composio::normalize_toolkit_slug(&result);
                if !normalized.is_empty() {
                    return Some(normalized);
                }
            }
        }

        None
    }

    /// Check if an error message indicates OAuth is needed
    fn is_oauth_needed_message(msg: &str) -> bool {
        let lower = msg.to_lowercase();
        lower.contains("disconnected")
            || lower.contains("not connected")
            || lower.contains("authorization")
            || lower.contains("oauth")
            || lower.contains("connected account")
            || lower.contains("connect")
            || lower.contains("authenticate")
            || lower.contains("permission")
            || lower.contains("unauthorized")
    }
}

#[async_trait]
impl Tool for ComposioMcpTool {
    fn name(&self) -> &str {
        &self.tool_name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> Value {
        self.schema.clone()
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        // Security check
        if let Err(error) = self.security.enforce_tool_operation(
            ToolOperation::Act,
            &format!("composio_mcp.{}", self.tool_name),
        ) {
            tracing::warn!(
                tool = self.tool_name,
                error = %error,
                "Tool execution blocked by security policy"
            );
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(error),
            });
        }

        tracing::debug!(
            tool = self.tool_name,
            "Executing Composio MCP tool"
        );

        // First attempt
        let first_attempt = self.client.execute_tool(&self.tool_name, args.clone()).await;

        match first_attempt {
            Ok(result) if !result.is_error() => {
                // Success
                tracing::info!(
                    tool = self.tool_name,
                    "Tool executed successfully"
                );
                return Ok(ToolResult {
                    success: true,
                    output: result.to_output_string(),
                    error: None,
                });
            }
            Ok(result) => {
                // Error from MCP - check if OAuth is needed
                let error_text = result.to_output_string();

                if Self::is_oauth_needed_message(&error_text) {
                    tracing::info!(
                        tool = self.tool_name,
                        "OAuth connection required, attempting onboarding"
                    );

                    // Try onboarding if available
                    if let Some(onboarding) = &self.onboarding {
                        if let Some(toolkit) = self.infer_toolkit_slug() {
                            let entity_id = self.client.user_id();

                            tracing::debug!(
                                tool = self.tool_name,
                                toolkit = toolkit,
                                entity_id = entity_id,
                                "Inferred toolkit from tool name"
                            );

                            // Attempt onboarding
                            match onboarding.ensure_connected(&toolkit, entity_id).await {
                                Ok(()) => {
                                    tracing::info!(
                                        tool = self.tool_name,
                                        toolkit = toolkit,
                                        "OAuth connection established, retrying tool"
                                    );

                                    // RETRY once after successful onboarding
                                    match self.client.execute_tool(&self.tool_name, args).await {
                                        Ok(retry_result) => {
                                            if retry_result.is_error() {
                                                tracing::warn!(
                                                    tool = self.tool_name,
                                                    "Tool failed after OAuth connection"
                                                );
                                                return Ok(ToolResult {
                                                    success: false,
                                                    output: String::new(),
                                                    error: Some(format!(
                                                        "Tool failed after OAuth connection: {}",
                                                        retry_result.to_output_string()
                                                    )),
                                                });
                                            } else {
                                                tracing::info!(
                                                    tool = self.tool_name,
                                                    "Tool succeeded after OAuth connection"
                                                );
                                                return Ok(ToolResult {
                                                    success: true,
                                                    output: retry_result.to_output_string(),
                                                    error: None,
                                                });
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                tool = self.tool_name,
                                                error = %e,
                                                "Retry failed after OAuth connection"
                                            );
                                            return Ok(ToolResult {
                                                success: false,
                                                output: String::new(),
                                                error: Some(format!("Retry failed: {}", e)),
                                            });
                                        }
                                    }
                                }
                                Err(e) => {
                                    // Check for server mode error (OAuth authorization required)
                                    let err_msg = e.to_string();
                                    if err_msg.contains("OAuth authorization required") 
                                        && err_msg.contains("Please click this link to authorize:") {
                                        tracing::debug!(
                                            tool = self.tool_name,
                                            "Returning OAuth URL to client (server mode)"
                                        );
                                        // Pass through the full error message with URL
                                        return Ok(ToolResult {
                                            success: false,
                                            output: String::new(),
                                            error: Some(err_msg),
                                        });
                                    }
                                    // Other onboarding error
                                    tracing::error!(
                                        tool = self.tool_name,
                                        toolkit = toolkit,
                                        error = %e,
                                        "OAuth onboarding failed"
                                    );
                                    // Check if error contains OAuth URL
                                    let err_msg = e.to_string();
                                    if err_msg.contains("Please click this link to authorize:") {
                                        // Pass through the OAuth URL
                                        return Ok(ToolResult {
                                            success: false,
                                            output: String::new(),
                                            error: Some(err_msg),
                                        });
                                    }
                                    return Ok(ToolResult {
                                        success: false,
                                        output: String::new(),
                                        error: Some(format!(
                                            "OAuth onboarding failed for {}: {}",
                                            toolkit,
                                            e
                                        )),
                                    });
                                }
                            }
                        } else {
                            tracing::warn!(
                                tool = self.tool_name,
                                "Could not infer toolkit from tool name"
                            );
                        }
                    } else {
                        tracing::debug!(
                            tool = self.tool_name,
                            "No onboarding handler available"
                        );
                    }
                }

                // Return original error
                tracing::warn!(
                    tool = self.tool_name,
                    "Tool execution failed"
                );
                Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(error_text),
                })
            }
            Err(e) => {
                tracing::error!(
                    tool = self.tool_name,
                    error = %e,
                    "MCP execution failed"
                );
                Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("MCP execution failed: {}", e)),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::SecurityPolicy;

    fn test_security() -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy::default())
    }

    fn test_mcp_tool() -> McpTool {
        McpTool {
            name: "GMAIL_SEND_EMAIL".to_string(),
            description: Some("Send an email via Gmail".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "to": {"type": "string"},
                    "subject": {"type": "string"},
                    "body": {"type": "string"}
                },
                "required": ["to", "subject", "body"]
            }),
        }
    }

    #[test]
    fn composio_mcp_tool_has_correct_name() {
        let client = Arc::new(ComposioMcpClient::new(
            "test_key".to_string(),
            "server_123".to_string(),
            "user_456".to_string(),
        ));

        let tool = ComposioMcpTool::new(client, test_mcp_tool(), test_security());

        assert_eq!(tool.name(), "GMAIL_SEND_EMAIL");
        assert_eq!(tool.mcp_tool_name(), "GMAIL_SEND_EMAIL");
    }

    #[test]
    fn composio_mcp_tool_has_description() {
        let client = Arc::new(ComposioMcpClient::new(
            "test_key".to_string(),
            "server_123".to_string(),
            "user_456".to_string(),
        ));

        let tool = ComposioMcpTool::new(client, test_mcp_tool(), test_security());

        assert_eq!(tool.description(), "Send an email via Gmail");
    }

    #[test]
    fn composio_mcp_tool_has_schema() {
        let client = Arc::new(ComposioMcpClient::new(
            "test_key".to_string(),
            "server_123".to_string(),
            "user_456".to_string(),
        ));

        let tool = ComposioMcpTool::new(client, test_mcp_tool(), test_security());

        let schema = tool.parameters_schema();
        assert!(schema.is_object());
        assert!(schema["properties"]["to"].is_object());
        assert!(schema["properties"]["subject"].is_object());
        assert!(schema["properties"]["body"].is_object());
    }

    #[test]
    fn composio_mcp_tool_uses_default_description_when_missing() {
        let client = Arc::new(ComposioMcpClient::new(
            "test_key".to_string(),
            "server_123".to_string(),
            "user_456".to_string(),
        ));

        let mcp_tool = McpTool {
            name: "TEST_TOOL".to_string(),
            description: None,
            input_schema: serde_json::json!({}),
        };

        let tool = ComposioMcpTool::new(client, mcp_tool, test_security());

        assert!(tool.description().contains("TEST_TOOL"));
        assert!(tool.description().contains("MCP"));
    }

    #[test]
    fn infer_toolkit_slug_handles_underscore_format() {
        let client = Arc::new(ComposioMcpClient::new(
            "test_key".to_string(),
            "server_123".to_string(),
            "user_456".to_string(),
        ));

        let mcp_tool = McpTool {
            name: "GMAIL_SEND_EMAIL".to_string(),
            description: Some("Send email".to_string()),
            input_schema: serde_json::json!({}),
        };

        let tool = ComposioMcpTool::new(client, mcp_tool, test_security());
        assert_eq!(tool.infer_toolkit_slug(), Some("gmail".to_string()));
    }

    #[test]
    fn infer_toolkit_slug_handles_dash_format() {
        let client = Arc::new(ComposioMcpClient::new(
            "test_key".to_string(),
            "server_123".to_string(),
            "user_456".to_string(),
        ));

        let mcp_tool = McpTool {
            name: "github-create-issue".to_string(),
            description: Some("Create issue".to_string()),
            input_schema: serde_json::json!({}),
        };

        let tool = ComposioMcpTool::new(client, mcp_tool, test_security());
        assert_eq!(tool.infer_toolkit_slug(), Some("github".to_string()));
    }

    #[test]
    fn infer_toolkit_slug_handles_camel_case() {
        let client = Arc::new(ComposioMcpClient::new(
            "test_key".to_string(),
            "server_123".to_string(),
            "user_456".to_string(),
        ));

        let mcp_tool = McpTool {
            name: "slackPostMessage".to_string(),
            description: Some("Post message".to_string()),
            input_schema: serde_json::json!({}),
        };

        let tool = ComposioMcpTool::new(client, mcp_tool, test_security());
        assert_eq!(tool.infer_toolkit_slug(), Some("slack".to_string()));
    }

    #[test]
    fn infer_toolkit_slug_normalizes_output() {
        let client = Arc::new(ComposioMcpClient::new(
            "test_key".to_string(),
            "server_123".to_string(),
            "user_456".to_string(),
        ));

        let mcp_tool = McpTool {
            name: "Google_Drive_Upload".to_string(),
            description: Some("Upload file".to_string()),
            input_schema: serde_json::json!({}),
        };

        let tool = ComposioMcpTool::new(client, mcp_tool, test_security());
        assert_eq!(tool.infer_toolkit_slug(), Some("google".to_string()));
    }

    #[test]
    fn infer_toolkit_slug_returns_none_for_invalid() {
        let client = Arc::new(ComposioMcpClient::new(
            "test_key".to_string(),
            "server_123".to_string(),
            "user_456".to_string(),
        ));

        let mcp_tool = McpTool {
            name: "123".to_string(),
            description: Some("Invalid".to_string()),
            input_schema: serde_json::json!({}),
        };

        let tool = ComposioMcpTool::new(client, mcp_tool, test_security());
        assert_eq!(tool.infer_toolkit_slug(), None);
    }

    #[test]
    fn is_oauth_needed_message_detects_common_patterns() {
        assert!(ComposioMcpTool::is_oauth_needed_message("Account disconnected"));
        assert!(ComposioMcpTool::is_oauth_needed_message("Not connected to Gmail"));
        assert!(ComposioMcpTool::is_oauth_needed_message("OAuth authorization required"));
        assert!(ComposioMcpTool::is_oauth_needed_message("Please connect your account"));
        assert!(ComposioMcpTool::is_oauth_needed_message("Authentication required"));
        assert!(ComposioMcpTool::is_oauth_needed_message("Permission denied - not authenticated"));
        assert!(ComposioMcpTool::is_oauth_needed_message("Unauthorized access"));
        assert!(!ComposioMcpTool::is_oauth_needed_message("Invalid parameters"));
        assert!(!ComposioMcpTool::is_oauth_needed_message("Rate limit exceeded"));
        assert!(!ComposioMcpTool::is_oauth_needed_message("Network error"));
    }
}
