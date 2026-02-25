// Composio MCP Tool Wrapper — converts MCP tools to ZeroClaw tools
//
// This module wraps Composio MCP tools as native ZeroClaw tools, allowing
// seamless integration with the existing tool system.

use super::traits::{Tool, ToolResult};
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
        }
    }

    /// Get the underlying MCP tool name
    pub fn mcp_tool_name(&self) -> &str {
        &self.tool_name
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
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(error),
            });
        }

        // Execute via MCP
        match self.client.execute_tool(&self.tool_name, args).await {
            Ok(result) => {
                if result.is_error() {
                    Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(result.to_output_string()),
                    })
                } else {
                    Ok(ToolResult {
                        success: true,
                        output: result.to_output_string(),
                        error: None,
                    })
                }
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("MCP execution failed: {}", e)),
            }),
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
}
