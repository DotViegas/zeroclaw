// Composio MCP Client — HTTP-based client for Composio's Model Context Protocol server
//
// This client communicates with Composio's cloud MCP server to access 1000+ OAuth apps
// (Gmail, Dropbox, GitHub, Slack, etc.) through a unified interface.
//
// Architecture:
// ZeroClaw (Rust) → HTTP → Composio MCP Server (Cloud) → Gmail/Dropbox/GitHub/etc.

use anyhow::Context;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const COMPOSIO_MCP_BASE_URL: &str = "https://backend.composio.dev/tool_router";
const DEFAULT_TIMEOUT_SECS: u64 = 60;
const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 10;

/// Parse Server-Sent Events (SSE) response format
///
/// MCP responses come in SSE format:
/// ```
/// event: message
/// data: {"jsonrpc":"2.0","result":{...},"id":1}
/// ```
fn parse_sse_response(text: &str) -> anyhow::Result<String> {
    // Find the "data: " line
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(json) = trimmed.strip_prefix("data:").or_else(|| trimmed.strip_prefix("data: ")) {
            return Ok(json.trim().to_string());
        }
    }
    
    // If no SSE format found, try to parse as direct JSON
    if text.trim().starts_with('{') {
        return Ok(text.trim().to_string());
    }
    
    anyhow::bail!("Invalid SSE response format: no 'data:' line found")
}

/// Client for communicating with Composio MCP server
pub struct ComposioMcpClient {
    api_key: String,
    server_id: String,
    user_id: String,
    mcp_url: String,
    client: Client,
}

impl ComposioMcpClient {
    /// Create a new Composio MCP client
    ///
    /// # Arguments
    /// * `api_key` - Composio API key for authentication
    /// * `server_id` - MCP server ID (created via Composio dashboard or API)
    /// * `user_id` - User/entity ID for this MCP instance
    pub fn new(api_key: String, server_id: String, user_id: String) -> Self {
        // Detect if this is a Tool Router Session (starts with "trs_") or Dedicated MCP Server
        let mcp_url = if server_id.starts_with("trs_") {
            // Tool Router Session format
            format!(
                "https://backend.composio.dev/tool_router/{}/mcp?include_composio_helper_actions=true&user_id={}",
                server_id, user_id
            )
        } else {
            // Dedicated MCP Server format
            format!(
                "{}/{}/mcp?include_composio_helper_actions=true&user_id={}",
                COMPOSIO_MCP_BASE_URL, server_id, user_id
            )
        };

        let client = crate::config::build_runtime_proxy_client_with_timeouts(
            "mcp.composio",
            DEFAULT_TIMEOUT_SECS,
            DEFAULT_CONNECT_TIMEOUT_SECS,
        );

        Self {
            api_key,
            server_id,
            user_id,
            mcp_url,
            client,
        }
    }

    /// List available tools from the MCP server
    ///
    /// This fetches all tools that are available based on the server's
    /// configured toolkits and the user's connected accounts.
    pub async fn list_tools(&self) -> anyhow::Result<Vec<McpTool>> {
        // MCP uses JSON-RPC 2.0 protocol
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        });

        let response = self
            .client
            .post(&self.mcp_url)
            .header("x-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send MCP tools list request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("MCP tools list failed ({}): {}", status, error_text);
        }

        let response_text = response
            .text()
            .await
            .context("Failed to read MCP response")?;

        // Parse SSE response (format: "event: message\ndata: {...}")
        let json_data = parse_sse_response(&response_text)?;

        // Parse JSON-RPC response
        let rpc_response: serde_json::Value = serde_json::from_str(&json_data)
            .context("Failed to parse JSON-RPC response")?;

        // Check for JSON-RPC error
        if let Some(error) = rpc_response.get("error") {
            anyhow::bail!("MCP JSON-RPC error: {}", error);
        }

        // Extract tools from result
        let tools = rpc_response
            .get("result")
            .and_then(|r| r.get("tools"))
            .and_then(|t| t.as_array())
            .context("Invalid MCP response: missing result.tools")?;

        let parsed_tools: Vec<McpTool> = tools
            .iter()
            .filter_map(|tool| serde_json::from_value(tool.clone()).ok())
            .collect();

        Ok(parsed_tools)
    }

    /// Execute a tool via the MCP server
    ///
    /// # Arguments
    /// * `tool_name` - Name of the tool to execute (e.g., "GMAIL_SEND_EMAIL")
    /// * `arguments` - Tool arguments as JSON value
    ///
    /// # Returns
    /// The tool execution result as a JSON value
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> anyhow::Result<McpToolResult> {
        // MCP uses JSON-RPC 2.0 protocol
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            }
        });

        let response = self
            .client
            .post(&self.mcp_url)
            .header("x-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send MCP tool execution request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("MCP tool execution failed ({}): {}", status, error_text);
        }

        let response_text = response
            .text()
            .await
            .context("Failed to read MCP response")?;

        // Parse SSE response
        let json_data = parse_sse_response(&response_text)?;

        // Parse JSON-RPC response
        let rpc_response: serde_json::Value = serde_json::from_str(&json_data)
            .context("Failed to parse JSON-RPC response")?;

        // Check for JSON-RPC error
        if let Some(error) = rpc_response.get("error") {
            anyhow::bail!("MCP JSON-RPC error: {}", error);
        }

        // Extract result
        let result = rpc_response
            .get("result")
            .context("Invalid MCP response: missing result")?;

        let tool_result: McpToolResult = serde_json::from_value(result.clone())
            .context("Failed to parse tool result")?;

        Ok(tool_result)
    }

    /// Get the MCP server URL
    pub fn mcp_url(&self) -> &str {
        &self.mcp_url
    }

    /// Get the server ID
    pub fn server_id(&self) -> &str {
        &self.server_id
    }

    /// Get the user ID
    pub fn user_id(&self) -> &str {
        &self.user_id
    }
}

/// MCP tool definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpTool {
    /// Tool name (e.g., "GMAIL_SEND_EMAIL")
    pub name: String,

    /// Human-readable description
    #[serde(default)]
    pub description: Option<String>,

    /// JSON schema for tool input parameters
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Result from MCP tool execution
#[derive(Debug, Deserialize, Serialize)]
pub struct McpToolResult {
    /// Tool execution result content
    pub content: Vec<McpToolContent>,

    /// Whether the tool execution was successful
    #[serde(default)]
    pub is_error: Option<bool>,
}

/// Content item from MCP tool result
#[derive(Debug, Deserialize, Serialize)]
pub struct McpToolContent {
    /// Content type (e.g., "text", "json")
    #[serde(rename = "type")]
    pub content_type: String,

    /// Content text (for type="text")
    #[serde(default)]
    pub text: Option<String>,

    /// Content data (for type="json" or other structured types)
    #[serde(default)]
    pub data: Option<Value>,
}

impl McpToolResult {
    /// Get the result as a formatted string
    pub fn to_output_string(&self) -> String {
        let mut output = String::new();

        for (i, content) in self.content.iter().enumerate() {
            if i > 0 {
                output.push_str("\n\n");
            }

            match content.content_type.as_str() {
                "text" => {
                    if let Some(text) = &content.text {
                        output.push_str(text);
                    }
                }
                "json" => {
                    if let Some(data) = &content.data {
                        if let Ok(pretty) = serde_json::to_string_pretty(data) {
                            output.push_str(&pretty);
                        } else {
                            output.push_str(&data.to_string());
                        }
                    }
                }
                _ => {
                    // For unknown types, try to serialize the whole content
                    if let Ok(serialized) = serde_json::to_string_pretty(content) {
                        output.push_str(&serialized);
                    }
                }
            }
        }

        output
    }

    /// Check if the result indicates an error
    pub fn is_error(&self) -> bool {
        self.is_error.unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_client_constructs_correct_url() {
        let client = ComposioMcpClient::new(
            "test_key".to_string(),
            "server_123".to_string(),
            "user_456".to_string(),
        );

        assert_eq!(
            client.mcp_url(),
            "https://backend.composio.dev/tool_router/server_123/mcp?include_composio_helper_actions=true&user_id=user_456"
        );
        assert_eq!(client.server_id(), "server_123");
        assert_eq!(client.user_id(), "user_456");
    }

    #[test]
    fn mcp_client_constructs_tool_router_url() {
        let client = ComposioMcpClient::new(
            "test_key".to_string(),
            "trs_Ij9jR5rIS4_7".to_string(),
            "user_456".to_string(),
        );

        assert_eq!(
            client.mcp_url(),
            "https://backend.composio.dev/tool_router/trs_Ij9jR5rIS4_7/mcp?include_composio_helper_actions=true&user_id=user_456"
        );
        assert_eq!(client.server_id(), "trs_Ij9jR5rIS4_7");
        assert_eq!(client.user_id(), "user_456");
    }

    #[test]
    fn mcp_tool_deserializes_correctly() {
        let json = r#"{
            "name": "GMAIL_SEND_EMAIL",
            "description": "Send an email via Gmail",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "to": {"type": "string"},
                    "subject": {"type": "string"},
                    "body": {"type": "string"}
                },
                "required": ["to", "subject", "body"]
            }
        }"#;

        let tool: McpTool = serde_json::from_str(json).unwrap();
        assert_eq!(tool.name, "GMAIL_SEND_EMAIL");
        assert_eq!(
            tool.description,
            Some("Send an email via Gmail".to_string())
        );
        assert!(tool.input_schema.is_object());
    }

    #[test]
    fn mcp_tool_result_formats_text_content() {
        let result = McpToolResult {
            content: vec![McpToolContent {
                content_type: "text".to_string(),
                text: Some("Email sent successfully!".to_string()),
                data: None,
            }],
            is_error: Some(false),
        };

        assert_eq!(result.to_output_string(), "Email sent successfully!");
        assert!(!result.is_error());
    }

    #[test]
    fn mcp_tool_result_formats_json_content() {
        let result = McpToolResult {
            content: vec![McpToolContent {
                content_type: "json".to_string(),
                text: None,
                data: Some(serde_json::json!({
                    "status": "success",
                    "message_id": "msg_123"
                })),
            }],
            is_error: Some(false),
        };

        let output = result.to_output_string();
        assert!(output.contains("\"status\": \"success\""));
        assert!(output.contains("\"message_id\": \"msg_123\""));
    }

    #[test]
    fn mcp_tool_result_handles_multiple_content_items() {
        let result = McpToolResult {
            content: vec![
                McpToolContent {
                    content_type: "text".to_string(),
                    text: Some("First part".to_string()),
                    data: None,
                },
                McpToolContent {
                    content_type: "text".to_string(),
                    text: Some("Second part".to_string()),
                    data: None,
                },
            ],
            is_error: Some(false),
        };

        assert_eq!(result.to_output_string(), "First part\n\nSecond part");
    }

    #[test]
    fn mcp_tool_result_detects_errors() {
        let result = McpToolResult {
            content: vec![McpToolContent {
                content_type: "text".to_string(),
                text: Some("Error occurred".to_string()),
                data: None,
            }],
            is_error: Some(true),
        };

        assert!(result.is_error());
    }
}
