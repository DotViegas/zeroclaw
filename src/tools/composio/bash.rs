//! Composio Remote Bash Tool Implementation
//!
//! This module implements the COMPOSIO_REMOTE_BASH_TOOL handler for executing
//! bash commands in Composio's remote sandbox environment.
//!
//! The bash tool provides:
//! - Command execution in a secure sandbox
//! - Exit code handling
//! - Stdout and stderr capture
//! - Working directory support

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::tools::traits::ToolResult;
use super::meta_tools::{McpClientTrait, JsonRpcResponse};

/// Bash execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Bash handler for executing commands in Composio's remote sandbox
pub struct BashHandler;

impl BashHandler {
    /// Create a new bash handler
    pub fn new() -> Self {
        Self
    }

    /// Execute a bash command via COMPOSIO_REMOTE_BASH_TOOL
    ///
    /// This method implements the bash execution flow:
    /// 1. Make MCP call to COMPOSIO_REMOTE_BASH_TOOL with command
    /// 2. Parse response and extract exit code, stdout, stderr
    /// 3. Return structured result with success based on exit code
    ///
    /// # Arguments
    /// * `mcp_client` - MCP client for making requests
    /// * `command` - Bash command to execute
    /// * `user_id` - User ID for session isolation
    /// * `request_id` - JSON-RPC request ID
    /// * `working_directory` - Optional working directory (defaults to /workspace)
    ///
    /// # Returns
    /// * `Ok(ToolResult)` - Execution result with stdout/stderr and metadata
    /// * `Err(anyhow::Error)` - Execution errors (network, parsing, etc.)
    pub async fn execute_bash(
        &self,
        mcp_client: &dyn McpClientTrait,
        command: &str,
        user_id: &str,
        request_id: i64,
        working_directory: Option<&str>,
    ) -> Result<ToolResult> {
        tracing::debug!(
            command = command,
            user_id = user_id,
            working_directory = working_directory,
            "Executing bash command via COMPOSIO_REMOTE_BASH_TOOL"
        );

        // Step 1: Make MCP call to COMPOSIO_REMOTE_BASH_TOOL
        let params = serde_json::json!({
            "command": command,
            "user_id": user_id,
            "working_directory": working_directory.unwrap_or("/workspace")
        });

        let result = mcp_client
            .tools_call(request_id, "COMPOSIO_REMOTE_BASH_TOOL", params)
            .await
            .context("Failed to call COMPOSIO_REMOTE_BASH_TOOL")?;

        // Step 2: Parse JSON-RPC response
        let rpc_response: JsonRpcResponse = serde_json::from_value(result)
            .context("Failed to parse JSON-RPC response")?;

        if let Some(error) = rpc_response.error {
            tracing::error!(
                command = command,
                user_id = user_id,
                error_code = error.code,
                error_message = %error.message,
                "Bash execution failed"
            );

            return Ok(ToolResult {
                success: false,
                output: format!("Bash execution failed: {}", error.message),
                error: Some(error.message),
            });
        }

        let result_data = rpc_response
            .result
            .ok_or_else(|| anyhow::anyhow!("No result in JSON-RPC response"))?;

        // Step 3: Extract bash result from response
        let bash_result = Self::extract_bash_result(&result_data)?;

        // Step 4: Determine success based on exit code
        let success = bash_result.exit_code == 0;

        // Step 5: Format output with stdout and stderr
        let output = if bash_result.stderr.is_empty() {
            bash_result.stdout.clone()
        } else {
            format!(
                "STDOUT:\n{}\n\nSTDERR:\n{}",
                bash_result.stdout, bash_result.stderr
            )
        };

        tracing::info!(
            command = command,
            user_id = user_id,
            exit_code = bash_result.exit_code,
            success = success,
            stdout_size = bash_result.stdout.len(),
            stderr_size = bash_result.stderr.len(),
            "Bash execution completed"
        );

        Ok(ToolResult {
            success,
            output,
            error: if success {
                None
            } else {
                Some(format!("Command exited with code {}", bash_result.exit_code))
            },
        })
    }

    /// Extract bash result from MCP response
    ///
    /// Handles multiple response formats:
    /// - content[0].text format (JSON string with exit_code, stdout, stderr)
    /// - Direct result format with exit_code, stdout, stderr fields
    fn extract_bash_result(result_data: &Value) -> Result<BashResult> {
        // Try to parse content[0].text format first
        if let Some(content_text) = result_data
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|text| text.as_str())
        {
            // Try to parse as JSON
            if let Ok(parsed) = serde_json::from_str::<Value>(content_text) {
                return Self::parse_bash_result_from_value(&parsed);
            }

            // If not JSON, treat as plain text stdout with exit code 0
            return Ok(BashResult {
                exit_code: 0,
                stdout: content_text.to_string(),
                stderr: String::new(),
            });
        }

        // Try direct result format
        Self::parse_bash_result_from_value(result_data)
    }

    /// Parse BashResult from a JSON value
    ///
    /// Extracts exit_code, stdout, and stderr from various field names
    fn parse_bash_result_from_value(data: &Value) -> Result<BashResult> {
        // Extract exit code (try multiple field names)
        let exit_code = data
            .get("exit_code")
            .or_else(|| data.get("exitCode"))
            .or_else(|| data.get("code"))
            .or_else(|| data.get("return_code"))
            .and_then(|c| c.as_i64())
            .unwrap_or(0) as i32;

        // Extract stdout (try multiple field names)
        let stdout = data
            .get("stdout")
            .or_else(|| data.get("output"))
            .or_else(|| data.get("result"))
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();

        // Extract stderr (try multiple field names)
        let stderr = data
            .get("stderr")
            .or_else(|| data.get("error_output"))
            .or_else(|| data.get("errors"))
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();

        tracing::debug!(
            exit_code = exit_code,
            stdout_size = stdout.len(),
            stderr_size = stderr.len(),
            "Extracted bash result from parsed data"
        );

        Ok(BashResult {
            exit_code,
            stdout,
            stderr,
        })
    }
}

impl Default for BashHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock MCP client for testing
    struct MockMcpClient {
        response: Value,
    }

    #[async_trait::async_trait]
    impl McpClientTrait for MockMcpClient {
        async fn tools_call(
            &self,
            _request_id: i64,
            _tool_name: &str,
            _params: Value,
        ) -> Result<Value> {
            Ok(self.response.clone())
        }
    }

    #[test]
    fn test_bash_handler_new() {
        let handler = BashHandler::new();
        // Just verify it can be created
        assert!(std::mem::size_of_val(&handler) >= 0);
    }

    #[tokio::test]
    async fn test_execute_bash_success() {
        let handler = BashHandler::new();

        // Mock response with successful execution
        let mock_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{
                    "text": r#"{"exit_code": 0, "stdout": "Hello, World!\n", "stderr": ""}"#
                }]
            }
        });

        let mock_client = MockMcpClient {
            response: mock_response,
        };

        let result = handler
            .execute_bash(&mock_client, "echo 'Hello, World!'", "user1", 1, None)
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output, "Hello, World!\n");
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn test_execute_bash_with_stderr() {
        let handler = BashHandler::new();

        // Mock response with stderr output
        let mock_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{
                    "text": r#"{"exit_code": 0, "stdout": "Output\n", "stderr": "Warning: something\n"}"#
                }]
            }
        });

        let mock_client = MockMcpClient {
            response: mock_response,
        };

        let result = handler
            .execute_bash(&mock_client, "some_command", "user1", 1, None)
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("STDOUT:"));
        assert!(result.output.contains("STDERR:"));
        assert!(result.output.contains("Output"));
        assert!(result.output.contains("Warning: something"));
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn test_execute_bash_failure() {
        let handler = BashHandler::new();

        // Mock response with non-zero exit code
        let mock_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{
                    "text": r#"{"exit_code": 1, "stdout": "", "stderr": "command not found\n"}"#
                }]
            }
        });

        let mock_client = MockMcpClient {
            response: mock_response,
        };

        let result = handler
            .execute_bash(&mock_client, "invalid_command", "user1", 1, None)
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.output.contains("command not found"));
        assert_eq!(result.error, Some("Command exited with code 1".to_string()));
    }

    #[tokio::test]
    async fn test_execute_bash_with_working_directory() {
        let handler = BashHandler::new();

        // Mock response
        let mock_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{
                    "text": r#"{"exit_code": 0, "stdout": "/custom/path\n", "stderr": ""}"#
                }]
            }
        });

        let mock_client = MockMcpClient {
            response: mock_response,
        };

        let result = handler
            .execute_bash(
                &mock_client,
                "pwd",
                "user1",
                1,
                Some("/custom/path"),
            )
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("/custom/path"));
    }

    #[tokio::test]
    async fn test_execute_bash_jsonrpc_error() {
        let handler = BashHandler::new();

        // Mock response with JSON-RPC error
        let mock_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": -32000,
                "message": "Sandbox unavailable"
            }
        });

        let mock_client = MockMcpClient {
            response: mock_response,
        };

        let result = handler
            .execute_bash(&mock_client, "echo test", "user1", 1, None)
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.output.contains("Sandbox unavailable"));
        assert_eq!(result.error, Some("Sandbox unavailable".to_string()));
    }

    #[tokio::test]
    async fn test_execute_bash_plain_text_response() {
        let handler = BashHandler::new();

        // Mock response with plain text (not JSON)
        let mock_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{
                    "text": "Plain text output from command"
                }]
            }
        });

        let mock_client = MockMcpClient {
            response: mock_response,
        };

        let result = handler
            .execute_bash(&mock_client, "echo test", "user1", 1, None)
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output, "Plain text output from command");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_extract_bash_result_from_content_text() {
        let result_data = serde_json::json!({
            "content": [{
                "text": r#"{"exit_code": 0, "stdout": "Test output", "stderr": ""}"#
            }]
        });

        let bash_result = BashHandler::extract_bash_result(&result_data).unwrap();
        assert_eq!(bash_result.exit_code, 0);
        assert_eq!(bash_result.stdout, "Test output");
        assert_eq!(bash_result.stderr, "");
    }

    #[test]
    fn test_extract_bash_result_direct_format() {
        let result_data = serde_json::json!({
            "exit_code": 127,
            "stdout": "",
            "stderr": "command not found"
        });

        let bash_result = BashHandler::extract_bash_result(&result_data).unwrap();
        assert_eq!(bash_result.exit_code, 127);
        assert_eq!(bash_result.stdout, "");
        assert_eq!(bash_result.stderr, "command not found");
    }

    #[test]
    fn test_extract_bash_result_alternative_field_names() {
        let result_data = serde_json::json!({
            "exitCode": 2,
            "output": "Some output",
            "error_output": "Some error"
        });

        let bash_result = BashHandler::extract_bash_result(&result_data).unwrap();
        assert_eq!(bash_result.exit_code, 2);
        assert_eq!(bash_result.stdout, "Some output");
        assert_eq!(bash_result.stderr, "Some error");
    }

    #[test]
    fn test_extract_bash_result_plain_text() {
        let result_data = serde_json::json!({
            "content": [{
                "text": "Plain text output"
            }]
        });

        let bash_result = BashHandler::extract_bash_result(&result_data).unwrap();
        assert_eq!(bash_result.exit_code, 0);
        assert_eq!(bash_result.stdout, "Plain text output");
        assert_eq!(bash_result.stderr, "");
    }

    #[test]
    fn test_parse_bash_result_from_value_minimal() {
        let data = serde_json::json!({});

        let bash_result = BashHandler::parse_bash_result_from_value(&data).unwrap();
        assert_eq!(bash_result.exit_code, 0);
        assert_eq!(bash_result.stdout, "");
        assert_eq!(bash_result.stderr, "");
    }

    #[test]
    fn test_parse_bash_result_from_value_complete() {
        let data = serde_json::json!({
            "exit_code": 42,
            "stdout": "Complete stdout",
            "stderr": "Complete stderr"
        });

        let bash_result = BashHandler::parse_bash_result_from_value(&data).unwrap();
        assert_eq!(bash_result.exit_code, 42);
        assert_eq!(bash_result.stdout, "Complete stdout");
        assert_eq!(bash_result.stderr, "Complete stderr");
    }
}
