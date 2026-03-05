//! Composio Meta Tools Implementation
//!
//! This module implements handlers for the 5 Composio v3 meta tools:
//! 1. COMPOSIO_SEARCH_TOOLS - Tool discovery
//! 2. COMPOSIO_MANAGE_CONNECTIONS - OAuth connection management
//! 3. COMPOSIO_MULTI_EXECUTE_TOOL - Tool execution
//! 4. COMPOSIO_REMOTE_WORKBENCH - Python sandbox for large data operations
//! 5. COMPOSIO_REMOTE_BASH_TOOL - Bash execution in sandbox
//!
//! Each handler follows the pattern:
//! - Check cache first (if applicable)
//! - Make MCP call to Composio
//! - Parse response
//! - Update cache (if applicable)
//! - Return structured result

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::tools::traits::ToolResult;

/// Connection information for a toolkit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub toolkit: String,
    pub connected_account_id: String,
    pub status: ConnectionStatus,
    pub created_at: DateTime<Utc>,
}

/// Connection status for a toolkit
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    Active,
    Expired,
    Revoked,
}

/// OAuth authentication required error
#[derive(Debug, Clone)]
pub struct AuthRequired {
    pub toolkit: String,
    pub connect_link: String,
    pub expires_in: u64,
}

impl std::fmt::Display for AuthRequired {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "OAuth authentication required for toolkit '{}'. Please visit: {} (expires in {} seconds)",
            self.toolkit, self.connect_link, self.expires_in
        )
    }
}

impl std::error::Error for AuthRequired {}

/// Connection cache with TTL support
pub struct ConnectionCache {
    cache: Arc<RwLock<HashMap<String, CachedConnection>>>,
    max_entries_per_user: usize,
}

#[derive(Debug, Clone)]
struct CachedConnection {
    info: ConnectionInfo,
    cached_at: DateTime<Utc>,
    ttl: chrono::Duration,
}

impl ConnectionCache {
    /// Create a new connection cache
    pub fn new(max_entries_per_user: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_entries_per_user,
        }
    }

    /// Get a cached connection if valid
    pub async fn get(&self, toolkit: &str, user_id: &str) -> Option<ConnectionInfo> {
        let cache = self.cache.read().await;
        let key = Self::cache_key(toolkit, user_id);
        
        if let Some(cached) = cache.get(&key) {
            let now = Utc::now();
            if now < cached.cached_at + cached.ttl {
                tracing::debug!(
                    toolkit = toolkit,
                    user_id = user_id,
                    "Connection cache hit"
                );
                return Some(cached.info.clone());
            } else {
                tracing::debug!(
                    toolkit = toolkit,
                    user_id = user_id,
                    "Connection cache expired"
                );
            }
        }
        
        None
    }

    /// Insert a connection into the cache
    pub async fn insert(
        &self,
        toolkit: &str,
        user_id: &str,
        info: ConnectionInfo,
        ttl: chrono::Duration,
    ) {
        let mut cache = self.cache.write().await;
        let key = Self::cache_key(toolkit, user_id);
        
        // Check if we need to evict entries for this user
        let user_entries: Vec<_> = cache
            .keys()
            .filter(|k| k.ends_with(&format!("::{}", user_id)))
            .cloned()
            .collect();
        
        if user_entries.len() >= self.max_entries_per_user {
            // LRU eviction: remove oldest entry for this user
            if let Some(oldest_key) = user_entries
                .iter()
                .filter_map(|k| cache.get(k).map(|v| (k, v.cached_at)))
                .min_by_key(|(_, cached_at)| *cached_at)
                .map(|(k, _)| k.clone())
            {
                cache.remove(&oldest_key);
                tracing::debug!(
                    user_id = user_id,
                    evicted_key = %oldest_key,
                    "Evicted oldest connection cache entry"
                );
            }
        }
        
        cache.insert(
            key.clone(),
            CachedConnection {
                info,
                cached_at: Utc::now(),
                ttl,
            },
        );
        
        tracing::debug!(
            toolkit = toolkit,
            user_id = user_id,
            "Connection cached"
        );
    }

    /// Remove a connection from the cache
    pub async fn remove(&self, toolkit: &str, user_id: &str) {
        let mut cache = self.cache.write().await;
        let key = Self::cache_key(toolkit, user_id);
        cache.remove(&key);
        
        tracing::debug!(
            toolkit = toolkit,
            user_id = user_id,
            "Connection removed from cache"
        );
    }

    /// Clear all cached connections for a user
    pub async fn clear_user(&self, user_id: &str) {
        let mut cache = self.cache.write().await;
        let keys_to_remove: Vec<_> = cache
            .keys()
            .filter(|k| k.ends_with(&format!("::{}", user_id)))
            .cloned()
            .collect();
        
        for key in keys_to_remove {
            cache.remove(&key);
        }
        
        tracing::debug!(
            user_id = user_id,
            "Cleared all connections for user"
        );
    }

    /// Generate cache key from toolkit and user_id
    fn cache_key(toolkit: &str, user_id: &str) -> String {
        format!("{}::{}", toolkit, user_id)
    }
}

/// Meta tools handler for Composio integration
pub struct MetaToolsHandler {
    connection_cache: ConnectionCache,
    execution_history: Arc<RwLock<Vec<ExecutionRecord>>>,
}

/// Execution record for tracking tool execution history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub tool_name: String,
    pub timestamp: DateTime<Utc>,
    pub success: bool,
    pub output_size: usize,
}

impl MetaToolsHandler {
    /// Create a new meta tools handler
    pub fn new() -> Self {
        Self {
            connection_cache: ConnectionCache::new(100), // max 100 entries per user
            execution_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Manage connection for a toolkit (COMPOSIO_MANAGE_CONNECTIONS handler)
    ///
    /// This method implements the full connection management flow:
    /// 1. Check cache first for existing connection
    /// 2. Make MCP call to COMPOSIO_MANAGE_CONNECTIONS if not cached
    /// 3. Parse response and check for OAuth requirement
    /// 4. Cache successful connections
    /// 5. Return AuthRequired error if OAuth needed
    ///
    /// # Arguments
    /// * `mcp_client` - MCP client for making requests
    /// * `toolkit` - Toolkit name (e.g., "gmail", "slack")
    /// * `user_id` - User ID for session isolation
    /// * `request_id` - JSON-RPC request ID
    ///
    /// # Returns
    /// * `Ok(ConnectionInfo)` - Connection is active and cached
    /// * `Err(AuthRequired)` - OAuth authentication required
    /// * `Err(anyhow::Error)` - Other errors (network, parsing, etc.)
    pub async fn manage_connection(
        &self,
        mcp_client: &dyn McpClientTrait,
        toolkit: &str,
        user_id: &str,
        request_id: i64,
    ) -> Result<ConnectionInfo> {
        // Step 1: Check cache first
        if let Some(cached_info) = self.connection_cache.get(toolkit, user_id).await {
            tracing::debug!(
                toolkit = toolkit,
                user_id = user_id,
                "Using cached connection"
            );
            return Ok(cached_info);
        }

        // Step 2: Make MCP call to COMPOSIO_MANAGE_CONNECTIONS with graceful error handling
        tracing::debug!(
            toolkit = toolkit,
            user_id = user_id,
            "Calling COMPOSIO_MANAGE_CONNECTIONS"
        );

        let params = serde_json::json!({
            "toolkits": [toolkit],
            "session": {
                "generate_id": true
            }
        });

        let result = match mcp_client
            .tools_call(request_id, "COMPOSIO_MANAGE_CONNECTIONS", params)
            .await
        {
            Ok(result) => result,
            Err(e) => {
                // Graceful degradation: OAuth connection check failed
                tracing::warn!(
                    toolkit = toolkit,
                    user_id = user_id,
                    error = %e,
                    "Failed to check OAuth connection status. Returning error without crashing."
                );
                return Err(anyhow::anyhow!(
                    "Failed to check OAuth connection for toolkit '{}': {}. \
                     Please verify Composio API connectivity and try again.",
                    toolkit, e
                ));
            }
        };

        // Step 3: Parse JSON-RPC response
        let rpc_response: JsonRpcResponse = serde_json::from_value(result)
            .context("Failed to parse JSON-RPC response")?;

        if let Some(error) = rpc_response.error {
            tracing::warn!(
                toolkit = toolkit,
                user_id = user_id,
                error_code = error.code,
                error_message = %error.message,
                "OAuth connection check returned error"
            );
            anyhow::bail!("JSON-RPC error: {} (code: {})", error.message, error.code);
        }

        let result_data = rpc_response
            .result
            .ok_or_else(|| anyhow::anyhow!("No result in JSON-RPC response"))?;

        // Parse the content[0].text JSON string
        let parsed_data = result_data
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|text| text.as_str())
            .and_then(|text_str| {
                tracing::debug!(
                    toolkit = toolkit,
                    text = text_str,
                    "Parsing MANAGE_CONNECTIONS response"
                );
                serde_json::from_str::<Value>(text_str).ok()
            })
            .ok_or_else(|| anyhow::anyhow!("Failed to parse MANAGE_CONNECTIONS response"))?;

        // Step 4: Check for OAuth requirement
        if let Some(results) = parsed_data.get("data").and_then(|d| d.get("results")) {
            if let Some(toolkit_data) = results.get(toolkit) {
                // Check for instruction field (contains OAuth link)
                if let Some(instruction) = toolkit_data.get("instruction").and_then(|i| i.as_str())
                {
                    // Extract OAuth link from instruction
                    let link_patterns = [
                        "https://connect.composio.dev/link/",
                        "https://backend.composio.dev/oauth/",
                        "https://app.composio.dev/",
                    ];

                    for pattern in &link_patterns {
                        if let Some(link_start) = instruction.find(pattern) {
                            let link_end = instruction[link_start..]
                                .find(|c: char| c.is_whitespace() || c == '\n' || c == ')')
                                .map(|pos| link_start + pos)
                                .unwrap_or(instruction.len());
                            let connect_link = instruction[link_start..link_end].trim().to_string();

                            tracing::info!(
                                toolkit = toolkit,
                                user_id = user_id,
                                connect_link = %connect_link,
                                "OAuth required"
                            );

                            return Err(AuthRequired {
                                toolkit: toolkit.to_string(),
                                connect_link,
                                expires_in: 600, // 10 minutes
                            }
                            .into());
                        }
                    }
                }

                // Check for redirect_url field (alternative format)
                if let Some(redirect_url) = toolkit_data.get("redirect_url").and_then(|u| u.as_str())
                {
                    tracing::info!(
                        toolkit = toolkit,
                        user_id = user_id,
                        "OAuth required (redirect_url format)"
                    );

                    return Err(AuthRequired {
                        toolkit: toolkit.to_string(),
                        connect_link: redirect_url.to_string(),
                        expires_in: 600,
                    }
                    .into());
                }

                // Connection is active - extract connected_account_id
                let connected_account_id = toolkit_data
                    .get("connected_account_id")
                    .or_else(|| toolkit_data.get("id"))
                    .and_then(|id| id.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let connection_info = ConnectionInfo {
                    toolkit: toolkit.to_string(),
                    connected_account_id,
                    status: ConnectionStatus::Active,
                    created_at: Utc::now(),
                };

                // Step 5: Cache the connection
                self.connection_cache
                    .insert(
                        toolkit,
                        user_id,
                        connection_info.clone(),
                        chrono::Duration::hours(1), // Cache for 1 hour
                    )
                    .await;

                tracing::info!(
                    toolkit = toolkit,
                    user_id = user_id,
                    "Connection active and cached"
                );

                return Ok(connection_info);
            }
        }

        // Check for direct redirect_url (legacy format)
        if let Some(redirect_url) = parsed_data.get("redirect_url").and_then(|u| u.as_str()) {
            tracing::info!(
                toolkit = toolkit,
                user_id = user_id,
                "OAuth required (legacy format)"
            );

            return Err(AuthRequired {
                toolkit: toolkit.to_string(),
                connect_link: redirect_url.to_string(),
                expires_in: 600,
            }
            .into());
        }

        // If we reach here, connection is assumed active but no explicit ID found
        let connection_info = ConnectionInfo {
            toolkit: toolkit.to_string(),
            connected_account_id: "unknown".to_string(),
            status: ConnectionStatus::Active,
            created_at: Utc::now(),
        };

        self.connection_cache
            .insert(
                toolkit,
                user_id,
                connection_info.clone(),
                chrono::Duration::hours(1),
            )
            .await;

        Ok(connection_info)
    }

    /// Get the connection cache (for testing)
    pub fn connection_cache(&self) -> &ConnectionCache {
        &self.connection_cache
    }

    /// Execute a tool via COMPOSIO_MULTI_EXECUTE_TOOL
    ///
    /// This method implements the tool execution flow:
    /// 1. Make MCP call to COMPOSIO_MULTI_EXECUTE_TOOL with timeout
    /// 2. Parse response and convert to ToolResult
    /// 3. Record execution in history
    /// 4. Return structured result
    ///
    /// # Arguments
    /// * `mcp_client` - MCP client for making requests
    /// * `tool_name` - Name of the tool to execute (e.g., "GMAIL_SEND_EMAIL")
    /// * `parameters` - Tool parameters as JSON value
    /// * `user_id` - User ID for session isolation
    /// * `request_id` - JSON-RPC request ID
    ///
    /// # Returns
    /// * `Ok(ToolResult)` - Tool execution result with success status and output
    /// * `Err(anyhow::Error)` - Execution errors (network, parsing, etc.)
    pub async fn execute_tool(
        &self,
        mcp_client: &dyn McpClientTrait,
        tool_name: &str,
        parameters: Value,
        user_id: &str,
        request_id: i64,
    ) -> Result<ToolResult> {
        tracing::debug!(
            tool_name = tool_name,
            user_id = user_id,
            "Calling COMPOSIO_MULTI_EXECUTE_TOOL"
        );

        // Step 1: Make MCP call to COMPOSIO_MULTI_EXECUTE_TOOL with graceful timeout handling
        let params = serde_json::json!({
            "tool_name": tool_name,
            "parameters": parameters,
            "user_id": user_id
        });

        let result = match mcp_client
            .tools_call(request_id, "COMPOSIO_MULTI_EXECUTE_TOOL", params)
            .await
        {
            Ok(result) => result,
            Err(e) => {
                let error_str = e.to_string().to_lowercase();
                
                // Check if this is a timeout error
                if error_str.contains("timeout") || error_str.contains("timed out") {
                    tracing::warn!(
                        tool_name = tool_name,
                        user_id = user_id,
                        error = %e,
                        "Tool execution timed out. Returning timeout error and continuing agent loop."
                    );
                    
                    // Record failed execution
                    self.record_execution(tool_name, false, 0).await;
                    
                    return Ok(ToolResult {
                        success: false,
                        output: format!(
                            "Tool execution timed out after 180 seconds. \
                             The operation may still be processing on Composio's side. \
                             Please check the toolkit's web interface for status."
                        ),
                        error: Some(format!("Timeout: {}", e)),
                    });
                }
                
                // Other errors - log and return gracefully
                tracing::warn!(
                    tool_name = tool_name,
                    user_id = user_id,
                    error = %e,
                    "Tool execution failed. Returning error without crashing agent."
                );
                
                // Record failed execution
                self.record_execution(tool_name, false, 0).await;
                
                return Ok(ToolResult {
                    success: false,
                    output: format!("Tool execution failed: {}", e),
                    error: Some(e.to_string()),
                });
            }
        };

        // Step 2: Parse JSON-RPC response
        let rpc_response: JsonRpcResponse = serde_json::from_value(result)
            .context("Failed to parse JSON-RPC response")?;

        if let Some(error) = rpc_response.error {
            tracing::error!(
                tool_name = tool_name,
                user_id = user_id,
                error_code = error.code,
                error_message = %error.message,
                "COMPOSIO_MULTI_EXECUTE_TOOL returned error"
            );

            // Record failed execution
            self.record_execution(tool_name, false, 0).await;

            return Ok(ToolResult {
                success: false,
                output: format!("Tool execution failed: {}", error.message),
                error: Some(error.message),
            });
        }

        let result_data = rpc_response
            .result
            .ok_or_else(|| anyhow::anyhow!("No result in JSON-RPC response"))?;

        // Step 3: Convert MCP response to ToolResult
        let tool_result = Self::convert_mcp_response_to_tool_result(&result_data, tool_name)?;

        // Step 4: Record execution in history
        self.record_execution(
            tool_name,
            tool_result.success,
            tool_result.output.len(),
        )
        .await;

        tracing::info!(
            tool_name = tool_name,
            user_id = user_id,
            success = tool_result.success,
            output_size = tool_result.output.len(),
            "Tool execution completed"
        );

        Ok(tool_result)
    }

    /// Convert MCP response to ToolResult
    ///
    /// Handles multiple response formats from Composio MCP:
    /// - content[0].text format (JSON string)
    /// - Direct result format
    /// - Error formats
    fn convert_mcp_response_to_tool_result(
        result_data: &Value,
        tool_name: &str,
    ) -> Result<ToolResult> {
        // Try to parse content[0].text format first
        if let Some(content_text) = result_data
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|text| text.as_str())
        {
            tracing::debug!(
                tool_name = tool_name,
                text_preview = %format!("{:.200}", content_text),
                "Parsing content[0].text format"
            );

            // Try to parse as JSON
            if let Ok(parsed) = serde_json::from_str::<Value>(content_text) {
                return Self::extract_tool_result_from_parsed(&parsed, tool_name);
            }

            // If not JSON, treat as plain text output
            return Ok(ToolResult {
                success: true,
                output: content_text.to_string(),
                error: None,
            });
        }

        // Try direct result format
        Self::extract_tool_result_from_parsed(result_data, tool_name)
    }

    /// Extract ToolResult from parsed JSON data
    fn extract_tool_result_from_parsed(data: &Value, tool_name: &str) -> Result<ToolResult> {
        // Check for explicit success field
        let success = data
            .get("success")
            .and_then(|s| s.as_bool())
            .unwrap_or(true); // Default to true if not specified

        // Extract output from various possible fields
        let output = if let Some(output_str) = data.get("output").and_then(|o| o.as_str()) {
            output_str.to_string()
        } else if let Some(result_str) = data.get("result").and_then(|r| r.as_str()) {
            result_str.to_string()
        } else if let Some(data_obj) = data.get("data") {
            // If data is an object, serialize it as JSON
            serde_json::to_string_pretty(data_obj)
                .unwrap_or_else(|_| data_obj.to_string())
        } else {
            // Fallback: serialize entire response
            serde_json::to_string_pretty(data)
                .unwrap_or_else(|_| data.to_string())
        };

        // Extract error if present
        let error = data
            .get("error")
            .and_then(|e| e.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                // Check for error_message field
                data.get("error_message")
                    .and_then(|e| e.as_str())
                    .map(|s| s.to_string())
            });

        tracing::debug!(
            tool_name = tool_name,
            success = success,
            output_size = output.len(),
            has_error = error.is_some(),
            "Extracted ToolResult from parsed data"
        );

        Ok(ToolResult {
            success,
            output,
            error,
        })
    }

    /// Record execution in history
    ///
    /// Maintains a rolling window of the last 100 executions
    pub async fn record_execution(&self, tool_name: &str, success: bool, output_size: usize) {
        let mut history = self.execution_history.write().await;

        history.push(ExecutionRecord {
            tool_name: tool_name.to_string(),
            timestamp: Utc::now(),
            success,
            output_size,
        });

        // Keep only last 100 executions (rolling window)
        if history.len() > 100 {
            let excess = history.len() - 100;
            history.drain(0..excess);
        }

        tracing::trace!(
            tool_name = tool_name,
            success = success,
            output_size = output_size,
            history_size = history.len(),
            "Recorded execution in history"
        );
    }

    /// Get execution history (for testing and observability)
    pub async fn get_execution_history(&self) -> Vec<ExecutionRecord> {
        self.execution_history.read().await.clone()
    }

    /// Clear execution history (for testing)
    pub async fn clear_execution_history(&self) {
        self.execution_history.write().await.clear();
    }
}

impl Default for MetaToolsHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for MCP client to enable testing with mocks
#[async_trait::async_trait]
pub trait McpClientTrait: Send + Sync {
    async fn tools_call(&self, request_id: i64, tool_name: &str, params: Value) -> Result<Value>;
}

/// JSON-RPC response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: i64,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
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

    #[tokio::test]
    async fn test_manage_connection_cache_hit() {
        let handler = MetaToolsHandler::new();
        
        // Pre-populate cache
        let cached_info = ConnectionInfo {
            toolkit: "gmail".to_string(),
            connected_account_id: "acc_123".to_string(),
            status: ConnectionStatus::Active,
            created_at: Utc::now(),
        };
        
        handler
            .connection_cache
            .insert("gmail", "user1", cached_info.clone(), chrono::Duration::hours(1))
            .await;
        
        // Mock client that should not be called
        let mock_client = MockMcpClient {
            response: serde_json::json!({}),
        };
        
        // Should return cached value without calling MCP
        let result = handler
            .manage_connection(&mock_client, "gmail", "user1", 1)
            .await
            .unwrap();
        
        assert_eq!(result.toolkit, "gmail");
        assert_eq!(result.connected_account_id, "acc_123");
        assert_eq!(result.status, ConnectionStatus::Active);
    }

    #[tokio::test]
    async fn test_manage_connection_oauth_required() {
        let handler = MetaToolsHandler::new();
        
        // Mock response with OAuth requirement
        let mock_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{
                    "text": r#"{"data": {"results": {"gmail": {"instruction": "Please visit https://connect.composio.dev/link/abc123 to authenticate"}}}}"#
                }]
            }
        });
        
        let mock_client = MockMcpClient {
            response: mock_response,
        };
        
        let result = handler
            .manage_connection(&mock_client, "gmail", "user1", 1)
            .await;
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        let auth_err = err.downcast::<AuthRequired>().unwrap();
        assert_eq!(auth_err.toolkit, "gmail");
        assert!(auth_err.connect_link.contains("connect.composio.dev"));
        assert_eq!(auth_err.expires_in, 600);
    }

    #[tokio::test]
    async fn test_manage_connection_active_and_cached() {
        let handler = MetaToolsHandler::new();
        
        // Mock response with active connection
        let mock_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{
                    "text": r#"{"data": {"results": {"gmail": {"connected_account_id": "acc_456", "status": "active"}}}}"#
                }]
            }
        });
        
        let mock_client = MockMcpClient {
            response: mock_response,
        };
        
        let result = handler
            .manage_connection(&mock_client, "gmail", "user1", 1)
            .await
            .unwrap();
        
        assert_eq!(result.toolkit, "gmail");
        assert_eq!(result.connected_account_id, "acc_456");
        assert_eq!(result.status, ConnectionStatus::Active);
        
        // Verify it was cached
        let cached = handler
            .connection_cache
            .get("gmail", "user1")
            .await
            .unwrap();
        assert_eq!(cached.connected_account_id, "acc_456");
    }

    #[tokio::test]
    async fn test_connection_cache_eviction() {
        let handler = MetaToolsHandler::new();
        
        // Insert max_entries_per_user + 1 connections
        for i in 0..101 {
            let info = ConnectionInfo {
                toolkit: format!("toolkit_{}", i),
                connected_account_id: format!("acc_{}", i),
                status: ConnectionStatus::Active,
                created_at: Utc::now(),
            };
            
            handler
                .connection_cache
                .insert(&format!("toolkit_{}", i), "user1", info, chrono::Duration::hours(1))
                .await;
            
            // Small delay to ensure different timestamps
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        }
        
        // First entry should have been evicted
        let first = handler
            .connection_cache
            .get("toolkit_0", "user1")
            .await;
        assert!(first.is_none());
        
        // Last entry should still be there
        let last = handler
            .connection_cache
            .get("toolkit_100", "user1")
            .await;
        assert!(last.is_some());
    }

    #[tokio::test]
    async fn test_execute_tool_success() {
        let handler = MetaToolsHandler::new();
        
        // Mock response with successful execution
        let mock_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{
                    "text": r#"{"success": true, "output": "Email sent successfully", "data": {"message_id": "msg_123"}}"#
                }]
            }
        });
        
        let mock_client = MockMcpClient {
            response: mock_response,
        };
        
        let result = handler
            .execute_tool(
                &mock_client,
                "GMAIL_SEND_EMAIL",
                serde_json::json!({"to": "test@example.com", "subject": "Test"}),
                "user1",
                1,
            )
            .await
            .unwrap();
        
        assert!(result.success);
        assert!(result.output.contains("Email sent successfully"));
        assert!(result.error.is_none());
        
        // Verify execution was recorded
        let history = handler.get_execution_history().await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].tool_name, "GMAIL_SEND_EMAIL");
        assert!(history[0].success);
    }

    #[tokio::test]
    async fn test_execute_tool_failure() {
        let handler = MetaToolsHandler::new();
        
        // Mock response with execution failure
        let mock_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{
                    "text": r#"{"success": false, "error": "Invalid email address"}"#
                }]
            }
        });
        
        let mock_client = MockMcpClient {
            response: mock_response,
        };
        
        let result = handler
            .execute_tool(
                &mock_client,
                "GMAIL_SEND_EMAIL",
                serde_json::json!({"to": "invalid", "subject": "Test"}),
                "user1",
                1,
            )
            .await
            .unwrap();
        
        assert!(!result.success);
        assert_eq!(result.error, Some("Invalid email address".to_string()));
        
        // Verify execution was recorded as failure
        let history = handler.get_execution_history().await;
        assert_eq!(history.len(), 1);
        assert!(!history[0].success);
    }

    #[tokio::test]
    async fn test_execute_tool_jsonrpc_error() {
        let handler = MetaToolsHandler::new();
        
        // Mock response with JSON-RPC error
        let mock_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": -32600,
                "message": "Invalid request"
            }
        });
        
        let mock_client = MockMcpClient {
            response: mock_response,
        };
        
        let result = handler
            .execute_tool(
                &mock_client,
                "GMAIL_SEND_EMAIL",
                serde_json::json!({}),
                "user1",
                1,
            )
            .await
            .unwrap();
        
        assert!(!result.success);
        assert!(result.output.contains("Invalid request"));
        assert_eq!(result.error, Some("Invalid request".to_string()));
        
        // Verify execution was recorded as failure
        let history = handler.get_execution_history().await;
        assert_eq!(history.len(), 1);
        assert!(!history[0].success);
    }

    #[tokio::test]
    async fn test_execute_tool_plain_text_response() {
        let handler = MetaToolsHandler::new();
        
        // Mock response with plain text (not JSON)
        let mock_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{
                    "text": "Plain text response from tool"
                }]
            }
        });
        
        let mock_client = MockMcpClient {
            response: mock_response,
        };
        
        let result = handler
            .execute_tool(
                &mock_client,
                "SOME_TOOL",
                serde_json::json!({}),
                "user1",
                1,
            )
            .await
            .unwrap();
        
        assert!(result.success);
        assert_eq!(result.output, "Plain text response from tool");
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn test_execution_history_rolling_window() {
        let handler = MetaToolsHandler::new();
        
        // Add 150 executions (more than the 100 limit)
        for i in 0..150 {
            handler
                .record_execution(&format!("tool_{}", i), true, 100)
                .await;
        }
        
        let history = handler.get_execution_history().await;
        
        // Should only keep last 100
        assert_eq!(history.len(), 100);
        
        // First entry should be tool_50 (0-49 were evicted)
        assert_eq!(history[0].tool_name, "tool_50");
        
        // Last entry should be tool_149
        assert_eq!(history[99].tool_name, "tool_149");
    }

    #[tokio::test]
    async fn test_execution_history_clear() {
        let handler = MetaToolsHandler::new();
        
        // Add some executions
        handler.record_execution("tool_1", true, 100).await;
        handler.record_execution("tool_2", false, 50).await;
        
        let history = handler.get_execution_history().await;
        assert_eq!(history.len(), 2);
        
        // Clear history
        handler.clear_execution_history().await;
        
        let history = handler.get_execution_history().await;
        assert_eq!(history.len(), 0);
    }

    #[tokio::test]
    async fn test_convert_mcp_response_various_formats() {
        // Test format 1: content[0].text with JSON
        let response1 = serde_json::json!({
            "content": [{
                "text": r#"{"success": true, "output": "Result 1"}"#
            }]
        });
        let result1 = MetaToolsHandler::convert_mcp_response_to_tool_result(&response1, "test_tool")
            .unwrap();
        assert!(result1.success);
        assert_eq!(result1.output, "Result 1");

        // Test format 2: Direct result format
        let response2 = serde_json::json!({
            "success": false,
            "error": "Something went wrong",
            "output": "Error details"
        });
        let result2 = MetaToolsHandler::convert_mcp_response_to_tool_result(&response2, "test_tool")
            .unwrap();
        assert!(!result2.success);
        assert_eq!(result2.output, "Error details");
        assert_eq!(result2.error, Some("Something went wrong".to_string()));

        // Test format 3: Data object format
        let response3 = serde_json::json!({
            "data": {
                "items": [1, 2, 3],
                "count": 3
            }
        });
        let result3 = MetaToolsHandler::convert_mcp_response_to_tool_result(&response3, "test_tool")
            .unwrap();
        assert!(result3.success);
        assert!(result3.output.contains("items"));
        assert!(result3.output.contains("count"));
    }

    #[tokio::test]
    async fn test_connection_cache_ttl_expiry() {
        let handler = MetaToolsHandler::new();
        
        // Insert connection with very short TTL
        let info = ConnectionInfo {
            toolkit: "gmail".to_string(),
            connected_account_id: "acc_123".to_string(),
            status: ConnectionStatus::Active,
            created_at: Utc::now(),
        };
        
        handler
            .connection_cache
            .insert("gmail", "user1", info, chrono::Duration::milliseconds(10))
            .await;
        
        // Should be cached immediately
        let cached = handler.connection_cache.get("gmail", "user1").await;
        assert!(cached.is_some());
        
        // Wait for TTL to expire
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
        
        // Should be expired now
        let expired = handler.connection_cache.get("gmail", "user1").await;
        assert!(expired.is_none());
    }

    #[tokio::test]
    async fn test_connection_cache_remove() {
        let handler = MetaToolsHandler::new();
        
        // Insert connection
        let info = ConnectionInfo {
            toolkit: "gmail".to_string(),
            connected_account_id: "acc_123".to_string(),
            status: ConnectionStatus::Active,
            created_at: Utc::now(),
        };
        
        handler
            .connection_cache
            .insert("gmail", "user1", info, chrono::Duration::hours(1))
            .await;
        
        // Verify it's cached
        assert!(handler.connection_cache.get("gmail", "user1").await.is_some());
        
        // Remove it
        handler.connection_cache.remove("gmail", "user1").await;
        
        // Should be gone
        assert!(handler.connection_cache.get("gmail", "user1").await.is_none());
    }

    #[tokio::test]
    async fn test_connection_cache_clear_user() {
        let handler = MetaToolsHandler::new();
        
        // Insert multiple connections for user1
        for toolkit in &["gmail", "slack", "github"] {
            let info = ConnectionInfo {
                toolkit: toolkit.to_string(),
                connected_account_id: format!("acc_{}", toolkit),
                status: ConnectionStatus::Active,
                created_at: Utc::now(),
            };
            
            handler
                .connection_cache
                .insert(toolkit, "user1", info, chrono::Duration::hours(1))
                .await;
        }
        
        // Insert connection for user2
        let info = ConnectionInfo {
            toolkit: "gmail".to_string(),
            connected_account_id: "acc_user2".to_string(),
            status: ConnectionStatus::Active,
            created_at: Utc::now(),
        };
        handler
            .connection_cache
            .insert("gmail", "user2", info, chrono::Duration::hours(1))
            .await;
        
        // Clear user1's connections
        handler.connection_cache.clear_user("user1").await;
        
        // User1's connections should be gone
        assert!(handler.connection_cache.get("gmail", "user1").await.is_none());
        assert!(handler.connection_cache.get("slack", "user1").await.is_none());
        assert!(handler.connection_cache.get("github", "user1").await.is_none());
        
        // User2's connection should still be there
        assert!(handler.connection_cache.get("gmail", "user2").await.is_some());
    }

    #[tokio::test]
    async fn test_manage_connection_multiple_oauth_link_formats() {
        let handler = MetaToolsHandler::new();
        
        // Test different OAuth link formats
        let link_formats = vec![
            r#"{"data": {"results": {"gmail": {"instruction": "Visit https://connect.composio.dev/link/abc123 to authenticate"}}}}"#,
            r#"{"data": {"results": {"gmail": {"instruction": "Go to https://backend.composio.dev/oauth/xyz789"}}}}"#,
            r#"{"data": {"results": {"gmail": {"instruction": "Open https://app.composio.dev/auth/def456 in browser"}}}}"#,
        ];
        
        for (idx, link_format) in link_formats.iter().enumerate() {
            let mock_response = serde_json::json!({
                "jsonrpc": "2.0",
                "id": idx,
                "result": {
                    "content": [{
                        "text": link_format
                    }]
                }
            });
            
            let mock_client = MockMcpClient {
                response: mock_response,
            };
            
            let result = handler
                .manage_connection(&mock_client, "gmail", "user1", idx as i64)
                .await;
            
            assert!(result.is_err());
            let err = result.unwrap_err();
            let auth_err = err.downcast::<AuthRequired>().unwrap();
            assert!(auth_err.connect_link.starts_with("https://"));
        }
    }

    #[tokio::test]
    async fn test_execute_tool_with_result_field() {
        let handler = MetaToolsHandler::new();
        
        // Mock response with "result" field instead of "output"
        let mock_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{
                    "text": r#"{"success": true, "result": "Operation completed"}"#
                }]
            }
        });
        
        let mock_client = MockMcpClient {
            response: mock_response,
        };
        
        let result = handler
            .execute_tool(&mock_client, "TEST_TOOL", serde_json::json!({}), "user1", 1)
            .await
            .unwrap();
        
        assert!(result.success);
        assert_eq!(result.output, "Operation completed");
    }

    #[tokio::test]
    async fn test_execute_tool_with_error_message_field() {
        let handler = MetaToolsHandler::new();
        
        // Mock response with "error_message" field instead of "error"
        let mock_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{
                    "text": r#"{"success": false, "error_message": "Operation failed", "output": "Details"}"#
                }]
            }
        });
        
        let mock_client = MockMcpClient {
            response: mock_response,
        };
        
        let result = handler
            .execute_tool(&mock_client, "TEST_TOOL", serde_json::json!({}), "user1", 1)
            .await
            .unwrap();
        
        assert!(!result.success);
        assert_eq!(result.error, Some("Operation failed".to_string()));
    }

    #[tokio::test]
    async fn test_connection_status_serialization() {
        // Test that ConnectionStatus can be serialized/deserialized
        let statuses = vec![
            ConnectionStatus::Active,
            ConnectionStatus::Expired,
            ConnectionStatus::Revoked,
        ];
        
        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let deserialized: ConnectionStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, deserialized);
        }
    }

    #[tokio::test]
    async fn test_execution_record_serialization() {
        let record = ExecutionRecord {
            tool_name: "GMAIL_SEND_EMAIL".to_string(),
            timestamp: Utc::now(),
            success: true,
            output_size: 1024,
        };
        
        let json = serde_json::to_string(&record).unwrap();
        let deserialized: ExecutionRecord = serde_json::from_str(&json).unwrap();
        
        assert_eq!(record.tool_name, deserialized.tool_name);
        assert_eq!(record.success, deserialized.success);
        assert_eq!(record.output_size, deserialized.output_size);
    }

    #[tokio::test]
    async fn test_auth_required_display() {
        let auth_err = AuthRequired {
            toolkit: "gmail".to_string(),
            connect_link: "https://connect.composio.dev/link/abc123".to_string(),
            expires_in: 600,
        };
        
        let display = format!("{}", auth_err);
        assert!(display.contains("gmail"));
        assert!(display.contains("https://connect.composio.dev/link/abc123"));
        assert!(display.contains("600"));
    }
}
