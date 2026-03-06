//! Multi-Tool Executor Implementation
//!
//! Native Rust implementation of COMPOSIO_MULTI_EXECUTE_TOOL meta tool.
//! Executes up to 20 tools in parallel using Tokio's async runtime.

use crate::client::ComposioClient;
use crate::error::ComposioError;
use crate::models::ToolExecutionResponse;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::task::JoinHandle;

/// Tool call specification for parallel execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool slug to execute
    pub tool_slug: String,
    
    /// Tool arguments
    pub arguments: serde_json::Value,
    
    /// Optional connected account ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connected_account_id: Option<String>,
}

/// Result of parallel tool execution
#[derive(Debug, Clone)]
pub struct MultiExecutionResult {
    /// Individual tool results (in same order as input)
    pub results: Vec<Result<ToolExecutionResponse, ComposioError>>,
    
    /// Number of successful executions
    pub successful: usize,
    
    /// Number of failed executions
    pub failed: usize,
    
    /// Total execution time in milliseconds
    pub total_time_ms: u128,
}

/// Multi-tool executor
pub struct MultiExecutor {
    client: Arc<ComposioClient>,
}

impl MultiExecutor {
    /// Create a new multi-executor instance
    ///
    /// # Arguments
    ///
    /// * `client` - Composio client instance
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::{ComposioClient, meta_tools::MultiExecutor};
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ComposioClient::builder()
    ///     .api_key("your-api-key")
    ///     .build()?;
    ///
    /// let executor = MultiExecutor::new(Arc::new(client));
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(client: Arc<ComposioClient>) -> Self {
        Self { client }
    }

    /// Execute multiple tools in parallel
    ///
    /// # Arguments
    ///
    /// * `session_id` - Session ID for execution context
    /// * `tools` - Vector of tool calls to execute (max 20)
    ///
    /// # Returns
    ///
    /// Multi-execution result with individual results and statistics
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::{ComposioClient, meta_tools::{MultiExecutor, ToolCall}};
    /// # use std::sync::Arc;
    /// # use serde_json::json;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = Arc::new(ComposioClient::builder().api_key("key").build()?);
    /// let executor = MultiExecutor::new(client);
    ///
    /// let tools = vec![
    ///     ToolCall {
    ///         tool_slug: "GITHUB_GET_REPOS".to_string(),
    ///         arguments: json!({ "owner": "composio" }),
    ///         connected_account_id: None,
    ///     },
    ///     ToolCall {
    ///         tool_slug: "GITHUB_GET_ISSUES".to_string(),
    ///         arguments: json!({ "owner": "composio", "repo": "composio" }),
    ///         connected_account_id: None,
    ///     },
    /// ];
    ///
    /// let result = executor.execute_parallel("session_123", tools).await?;
    /// println!("Successful: {}, Failed: {}", result.successful, result.failed);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_parallel(
        &self,
        session_id: &str,
        tools: Vec<ToolCall>,
    ) -> Result<MultiExecutionResult, ComposioError> {
        // Validate tool count
        if tools.is_empty() {
            return Err(ComposioError::ValidationError(
                "At least one tool must be provided".to_string(),
            ));
        }

        if tools.len() > 20 {
            return Err(ComposioError::ValidationError(
                "Maximum 20 tools can be executed in parallel".to_string(),
            ));
        }

        let start_time = std::time::Instant::now();

        // Spawn parallel execution tasks
        let mut handles: Vec<JoinHandle<Result<ToolExecutionResponse, ComposioError>>> = Vec::new();

        for tool in tools {
            let client = self.client.clone();
            let session_id = session_id.to_string();

            let handle = tokio::spawn(async move {
                let url = format!(
                    "{}/tool_router/session/{}/execute",
                    client.base_url(),
                    session_id
                );

                let response = client
                    .http_client()
                    .post(&url)
                    .json(&serde_json::json!({
                        "tool_slug": tool.tool_slug,
                        "arguments": tool.arguments,
                        "connected_account_id": tool.connected_account_id,
                    }))
                    .send()
                    .await
                    .map_err(|e| ComposioError::NetworkError(e.to_string()))?;

                if !response.status().is_success() {
                    let status = response.status();
                    let error_text = response
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unknown error".to_string());
                    return Err(ComposioError::ApiError {
                        status: status.as_u16(),
                        message: error_text,
                        request_id: None,
                        suggested_fix: None,
                    });
                }

                let result: ToolExecutionResponse = response
                    .json()
                    .await
                    .map_err(|e| ComposioError::SerializationError(e.to_string()))?;

                Ok(result)
            });

            handles.push(handle);
        }

        // Wait for all tasks to complete
        let mut results = Vec::new();
        let mut successful = 0;
        let mut failed = 0;

        for handle in handles {
            match handle.await {
                Ok(result) => {
                    if result.is_ok() {
                        successful += 1;
                    } else {
                        failed += 1;
                    }
                    results.push(result);
                }
                Err(e) => {
                    failed += 1;
                    results.push(Err(ComposioError::ExecutionError(format!(
                        "Task panicked: {}",
                        e
                    ))));
                }
            }
        }

        let total_time_ms = start_time.elapsed().as_millis();

        Ok(MultiExecutionResult {
            results,
            successful,
            failed,
            total_time_ms,
        })
    }

    /// Execute tools sequentially (fallback for when parallel execution is not desired)
    ///
    /// # Arguments
    ///
    /// * `session_id` - Session ID
    /// * `tools` - Vector of tool calls
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::{ComposioClient, meta_tools::{MultiExecutor, ToolCall}};
    /// # use std::sync::Arc;
    /// # use serde_json::json;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = Arc::new(ComposioClient::builder().api_key("key").build()?);
    /// let executor = MultiExecutor::new(client);
    ///
    /// let tools = vec![
    ///     ToolCall {
    ///         tool_slug: "GITHUB_CREATE_ISSUE".to_string(),
    ///         arguments: json!({ "title": "Bug", "body": "Description" }),
    ///         connected_account_id: None,
    ///     },
    /// ];
    ///
    /// let result = executor.execute_sequential("session_123", tools).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_sequential(
        &self,
        session_id: &str,
        tools: Vec<ToolCall>,
    ) -> Result<MultiExecutionResult, ComposioError> {
        let start_time = std::time::Instant::now();
        let mut results = Vec::new();
        let mut successful = 0;
        let mut failed = 0;

        for tool in tools {
            let url = format!(
                "{}/tool_router/session/{}/execute",
                self.client.base_url(),
                session_id
            );

            let result = async {
                let response = self
                    .client
                    .http_client()
                    .post(&url)
                    .json(&serde_json::json!({
                        "tool_slug": tool.tool_slug,
                        "arguments": tool.arguments,
                        "connected_account_id": tool.connected_account_id,
                    }))
                    .send()
                    .await
                    .map_err(|e| ComposioError::NetworkError(e.to_string()))?;

                if !response.status().is_success() {
                    let status = response.status();
                    let error_text = response
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unknown error".to_string());
                    return Err(ComposioError::ApiError {
                        status: status.as_u16(),
                        message: error_text,
                        request_id: None,
                        suggested_fix: None,
                    });
                }

                let result: ToolExecutionResponse = response
                    .json()
                    .await
                    .map_err(|e| ComposioError::SerializationError(e.to_string()))?;

                Ok(result)
            }
            .await;

            if result.is_ok() {
                successful += 1;
            } else {
                failed += 1;
            }

            results.push(result);
        }

        let total_time_ms = start_time.elapsed().as_millis();

        Ok(MultiExecutionResult {
            results,
            successful,
            failed,
            total_time_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_call_serialization() {
        let call = ToolCall {
            tool_slug: "GITHUB_CREATE_ISSUE".to_string(),
            arguments: serde_json::json!({
                "title": "Test Issue",
                "body": "Test body"
            }),
            connected_account_id: Some("ca_123".to_string()),
        };

        let json = serde_json::to_string(&call).unwrap();
        assert!(json.contains("GITHUB_CREATE_ISSUE"));
        assert!(json.contains("Test Issue"));
        assert!(json.contains("ca_123"));

        let deserialized: ToolCall = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.tool_slug, "GITHUB_CREATE_ISSUE");
    }

    #[test]
    fn test_tool_call_without_account_id() {
        let call = ToolCall {
            tool_slug: "GMAIL_SEND_EMAIL".to_string(),
            arguments: serde_json::json!({ "to": "user@example.com" }),
            connected_account_id: None,
        };

        let json = serde_json::to_string(&call).unwrap();
        assert!(!json.contains("connected_account_id"));
    }
}
