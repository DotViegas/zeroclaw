//! Composio Remote Workbench Implementation
//!
//! This module implements the COMPOSIO_REMOTE_WORKBENCH handler for large data operations
//! that would overflow the context window. The Workbench provides a persistent Python sandbox
//! environment with:
//! - Session management (max 10 active sessions, 1 hour timeout)
//! - File I/O operations
//! - Tool execution via run_composio_tool() helper
//! - Output summarization for large responses (>4000 tokens)

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

use crate::tools::traits::ToolResult;
use super::meta_tools::{McpClientTrait, JsonRpcResponse};

/// Workbench session state
///
/// Maintains state for a persistent Python sandbox session including:
/// - session_id: Unique identifier for the Workbench session
/// - user_id: User ID for session isolation
/// - files: HashMap of file paths to content (for file I/O operations)
/// - last_output: Last execution output (for context in subsequent calls)
/// - created_at: Session creation timestamp (for timeout management)
/// - last_accessed_at: Last access timestamp (for inactivity timeout)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkbenchState {
    pub session_id: Option<String>,
    pub user_id: String,
    pub files: HashMap<String, String>,
    pub last_output: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub last_accessed_at: Option<DateTime<Utc>>,
}

impl WorkbenchState {
    /// Create a new empty Workbench state for a user
    pub fn new(user_id: String) -> Self {
        Self {
            session_id: None,
            user_id,
            files: HashMap::new(),
            last_output: None,
            created_at: None,
            last_accessed_at: None,
        }
    }

    /// Check if the session is expired (1 hour inactivity timeout)
    pub fn is_expired(&self, timeout_hours: i64) -> bool {
        if let Some(last_accessed) = self.last_accessed_at {
            let now = Utc::now();
            let elapsed = now.signed_duration_since(last_accessed);
            elapsed.num_hours() >= timeout_hours
        } else {
            false
        }
    }

    /// Update last accessed timestamp
    pub fn touch(&mut self) {
        self.last_accessed_at = Some(Utc::now());
    }

    /// Reset the session state
    pub fn reset(&mut self) {
        self.session_id = None;
        self.files.clear();
        self.last_output = None;
        self.created_at = None;
        self.last_accessed_at = None;
    }
}

/// Workbench handler for managing Python sandbox sessions
///
/// Supports:
/// - Multiple concurrent sessions (max 10 per instance)
/// - Per-user session isolation
/// - LRU eviction when max sessions reached
/// - Automatic cleanup of expired sessions (1 hour inactivity timeout)
pub struct WorkbenchHandler {
    sessions: Arc<RwLock<HashMap<String, WorkbenchState>>>,
    max_sessions: usize,
    session_timeout_hours: i64,
    cleanup_task: Option<tokio::task::JoinHandle<()>>,
}

impl WorkbenchHandler {
    /// Create a new Workbench handler
    ///
    /// # Arguments
    /// * `max_sessions` - Maximum number of active sessions (default: 10)
    /// * `session_timeout_hours` - Session inactivity timeout in hours (default: 1)
    pub fn new(max_sessions: usize, session_timeout_hours: i64) -> Self {
        let sessions = Arc::new(RwLock::new(HashMap::new()));
        
        // Start background cleanup task
        let cleanup_sessions = Arc::clone(&sessions);
        let cleanup_timeout = session_timeout_hours;
        let cleanup_task = tokio::spawn(async move {
            Self::cleanup_loop(cleanup_sessions, cleanup_timeout).await;
        });

        Self {
            sessions,
            max_sessions,
            session_timeout_hours,
            cleanup_task: Some(cleanup_task),
        }
    }

    /// Background cleanup loop for expired sessions
    ///
    /// Runs every 5 minutes and removes sessions that have been inactive for more than
    /// the configured timeout period.
    async fn cleanup_loop(sessions: Arc<RwLock<HashMap<String, WorkbenchState>>>, timeout_hours: i64) {
        let mut cleanup_interval = interval(Duration::from_secs(300)); // 5 minutes
        
        loop {
            cleanup_interval.tick().await;
            
            let mut sessions_guard = sessions.write().await;
            let expired_keys: Vec<String> = sessions_guard
                .iter()
                .filter(|(_, state)| state.is_expired(timeout_hours))
                .map(|(key, _)| key.clone())
                .collect();
            
            for key in &expired_keys {
                if let Some(state) = sessions_guard.remove(key) {
                    tracing::info!(
                        session_id = state.session_id.as_deref().unwrap_or("unknown"),
                        user_id = %state.user_id,
                        "Cleaned up expired Workbench session"
                    );
                }
            }
            
            if !expired_keys.is_empty() {
                tracing::debug!(
                    cleaned_count = expired_keys.len(),
                    remaining_count = sessions_guard.len(),
                    "Workbench session cleanup completed"
                );
            }
        }
    }

    /// Evict least recently used session when max sessions limit is reached
    ///
    /// Uses LRU (Least Recently Used) eviction strategy based on last_accessed_at timestamp.
    async fn evict_lru_session(&self) {
        let mut sessions_guard = self.sessions.write().await;
        
        if sessions_guard.len() < self.max_sessions {
            return; // No eviction needed
        }
        
        // Find LRU session
        let lru_key = sessions_guard
            .iter()
            .min_by_key(|(_, state)| state.last_accessed_at)
            .map(|(key, _)| key.clone());
        
        if let Some(key) = lru_key {
            if let Some(state) = sessions_guard.remove(&key) {
                tracing::warn!(
                    session_id = state.session_id.as_deref().unwrap_or("unknown"),
                    user_id = %state.user_id,
                    "Evicted LRU Workbench session (max sessions limit reached)"
                );
            }
        }
    }

    /// Get or create session state for a user
    ///
    /// Implements per-user session isolation by using user_id as the session key.
    async fn get_or_create_session_state(&self, user_id: &str) -> WorkbenchState {
        let mut sessions_guard = self.sessions.write().await;
        
        sessions_guard
            .entry(user_id.to_string())
            .or_insert_with(|| WorkbenchState::new(user_id.to_string()))
            .clone()
    }

    /// Update session state
    async fn update_session_state(&self, user_id: &str, state: WorkbenchState) {
        let mut sessions_guard = self.sessions.write().await;
        sessions_guard.insert(user_id.to_string(), state);
    }

    /// Ensure a Workbench session exists, creating or reusing as needed
    ///
    /// This method implements the session management logic:
    /// 1. Get or create session state for the user
    /// 2. Check if a valid session exists (not expired)
    /// 3. If expired or missing, create a new session
    /// 4. Evict LRU session if max sessions limit reached
    /// 5. Return the session_id for use in MCP calls
    ///
    /// # Arguments
    /// * `mcp_client` - MCP client for making requests
    /// * `user_id` - User ID for session isolation
    /// * `request_id` - JSON-RPC request ID
    ///
    /// # Returns
    /// * `Ok(String)` - Session ID (existing or newly created)
    /// * `Err(anyhow::Error)` - Session creation errors
    pub async fn ensure_workbench_session(
        &self,
        mcp_client: &dyn McpClientTrait,
        user_id: &str,
        request_id: i64,
    ) -> Result<String> {
        // Get or create session state for this user
        let mut state = self.get_or_create_session_state(user_id).await;

        // Check if we have a valid session
        if let Some(ref session_id) = state.session_id {
            if !state.is_expired(self.session_timeout_hours) {
                let session_id_clone = session_id.clone();
                
                tracing::debug!(
                    session_id = %session_id_clone,
                    user_id = user_id,
                    "Reusing existing Workbench session"
                );
                
                // Update last accessed timestamp
                state.touch();
                self.update_session_state(user_id, state).await;
                
                return Ok(session_id_clone);
            } else {
                tracing::info!(
                    session_id = %session_id,
                    user_id = user_id,
                    "Workbench session expired, creating new session"
                );
                state.reset();
            }
        }

        // Evict LRU session if we're at max capacity
        self.evict_lru_session().await;

        // Create a new session by making an initial Workbench call
        tracing::info!(
            user_id = user_id,
            "Creating new Workbench session"
        );

        // Initialize session with a simple Python command
        let init_code = "print('Workbench session initialized')";
        let params = serde_json::json!({
            "code": init_code,
            "user_id": user_id
        });

        let result = mcp_client
            .tools_call(request_id, "COMPOSIO_REMOTE_WORKBENCH", params)
            .await
            .context("Failed to initialize Workbench session")?;

        // Parse JSON-RPC response
        let rpc_response: JsonRpcResponse = serde_json::from_value(result)
            .context("Failed to parse JSON-RPC response")?;

        if let Some(error) = rpc_response.error {
            anyhow::bail!(
                "Failed to create Workbench session: {} (code: {})",
                error.message,
                error.code
            );
        }

        let result_data = rpc_response
            .result
            .ok_or_else(|| anyhow::anyhow!("No result in JSON-RPC response"))?;

        // Extract session_id from response
        let session_id = Self::extract_session_id(&result_data)
            .context("Failed to extract session_id from Workbench response")?;

        // Update state with new session
        state.session_id = Some(session_id.clone());
        state.created_at = Some(Utc::now());
        state.last_accessed_at = Some(Utc::now());
        state.last_output = Some("Workbench session initialized".to_string());
        
        self.update_session_state(user_id, state).await;

        tracing::info!(
            session_id = %session_id,
            user_id = user_id,
            "Workbench session created successfully"
        );

        Ok(session_id)
    }

    /// Extract session_id from Workbench response
    ///
    /// Handles multiple response formats:
    /// - content[0].text format (JSON string with session_id field)
    /// - Direct result format with session_id field
    /// - Generate session_id if not provided
    fn extract_session_id(result_data: &Value) -> Result<String> {
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
                if let Some(session_id) = parsed.get("session_id").and_then(|s| s.as_str()) {
                    return Ok(session_id.to_string());
                }
            }
        }

        // Try direct result format
        if let Some(session_id) = result_data.get("session_id").and_then(|s| s.as_str()) {
            return Ok(session_id.to_string());
        }

        // Generate a session_id if not provided
        let session_id = format!("wb_{}", uuid::Uuid::new_v4());
        tracing::debug!(
            session_id = %session_id,
            "Generated session_id (not provided by Workbench)"
        );
        Ok(session_id)
    }

    /// Execute code via Workbench (COMPOSIO_REMOTE_WORKBENCH handler)
    ///
    /// This method implements the full Workbench execution flow:
    /// 1. Ensure a valid Workbench session exists
    /// 2. Make MCP call to COMPOSIO_REMOTE_WORKBENCH with Python code
    /// 3. Parse response and extract output
    /// 4. Update Workbench state (last_output, last_accessed_at, files if applicable)
    /// 5. Summarize output if too large (>4000 tokens)
    /// 6. Return structured result with metadata
    ///
    /// # Arguments
    /// * `mcp_client` - MCP client for making requests
    /// * `python_code` - Python code to execute in the sandbox
    /// * `user_id` - User ID for session isolation
    /// * `request_id` - JSON-RPC request ID
    ///
    /// # Returns
    /// * `Ok(ToolResult)` - Execution result with output and metadata
    /// * `Err(anyhow::Error)` - Execution errors (network, parsing, etc.)
    pub async fn execute_via_workbench(
        &self,
        mcp_client: &dyn McpClientTrait,
        python_code: &str,
        user_id: &str,
        request_id: i64,
    ) -> Result<ToolResult> {
        // Step 1: Ensure we have a valid session
        let session_id = self
            .ensure_workbench_session(mcp_client, user_id, request_id)
            .await?;

        tracing::debug!(
            session_id = %session_id,
            user_id = user_id,
            code_length = python_code.len(),
            "Executing code via Workbench"
        );

        // Step 2: Make MCP call to COMPOSIO_REMOTE_WORKBENCH
        let params = serde_json::json!({
            "session_id": session_id,
            "code": python_code,
            "user_id": user_id
        });

        let result = mcp_client
            .tools_call(request_id, "COMPOSIO_REMOTE_WORKBENCH", params)
            .await
            .context("Failed to call COMPOSIO_REMOTE_WORKBENCH")?;

        // Step 3: Parse JSON-RPC response
        let rpc_response: JsonRpcResponse = serde_json::from_value(result)
            .context("Failed to parse JSON-RPC response")?;

        if let Some(error) = rpc_response.error {
            tracing::error!(
                session_id = %session_id,
                user_id = user_id,
                error_code = error.code,
                error_message = %error.message,
                "Workbench execution failed"
            );

            return Ok(ToolResult {
                success: false,
                output: format!("Workbench execution failed: {}", error.message),
                error: Some(error.message),
            });
        }

        let result_data = rpc_response
            .result
            .ok_or_else(|| anyhow::anyhow!("No result in JSON-RPC response"))?;

        // Step 4: Extract output from response
        let output = Self::extract_workbench_output(&result_data)?;
        let output_size = output.len();

        // Step 5: Update Workbench state
        {
            let mut state = self.get_or_create_session_state(user_id).await;
            state.last_output = Some(output.clone());
            state.touch(); // Update last accessed timestamp
            self.update_session_state(user_id, state).await;
        }

        // Step 6: Summarize output if too large (>4000 tokens, ~16000 chars)
        let (final_output, was_summarized) = if output_size > 16000 {
            let summary = Self::summarize_workbench_output(&output);
            // Include metadata in the summarized output
            let output_with_metadata = format!(
                "{}\n\n[Workbench Metadata: session_id={}, output_size={} bytes, full_output_available=true]",
                summary,
                session_id,
                output_size
            );
            (output_with_metadata, true)
        } else {
            (output, false)
        };

        tracing::info!(
            session_id = %session_id,
            user_id = user_id,
            output_size = output_size,
            was_summarized = was_summarized,
            "Workbench execution completed"
        );

        Ok(ToolResult {
            success: true,
            output: final_output,
            error: None,
        })
    }

    /// Extract output from Workbench response
    ///
    /// Handles multiple response formats:
    /// - content[0].text format (JSON string or plain text)
    /// - Direct result format with output field
    fn extract_workbench_output(result_data: &Value) -> Result<String> {
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
                if let Some(output) = parsed.get("output").and_then(|o| o.as_str()) {
                    return Ok(output.to_string());
                }
                // If no output field, return the entire parsed JSON
                return Ok(serde_json::to_string_pretty(&parsed)
                    .unwrap_or_else(|_| content_text.to_string()));
            }

            // If not JSON, treat as plain text output
            return Ok(content_text.to_string());
        }

        // Try direct result format
        if let Some(output) = result_data.get("output").and_then(|o| o.as_str()) {
            return Ok(output.to_string());
        }

        // Fallback: serialize entire response
        Ok(serde_json::to_string_pretty(result_data)
            .unwrap_or_else(|_| result_data.to_string()))
    }

    /// Summarize large Workbench output to prevent context window overflow
    ///
    /// Strategy:
    /// - Keep first 8000 chars (beginning of output)
    /// - Keep last 4000 chars (end of output)
    /// - Add truncation notice in the middle
    ///
    /// This method is called when output exceeds 4000 tokens (~16000 chars)
    pub fn summarize_workbench_output(output: &str) -> String {
        const HEAD_SIZE: usize = 8000;
        const TAIL_SIZE: usize = 4000;

        if output.len() <= HEAD_SIZE + TAIL_SIZE {
            return output.to_string();
        }

        let head = &output[..HEAD_SIZE];
        let tail = &output[output.len() - TAIL_SIZE..];

        format!(
            "{}\n\n... [Output truncated: {} chars omitted] ...\n\n{}",
            head,
            output.len() - HEAD_SIZE - TAIL_SIZE,
            tail
        )
    }

    /// Get the current Workbench state for a user (for testing)
    pub async fn get_state(&self, user_id: &str) -> Option<WorkbenchState> {
        let sessions_guard = self.sessions.read().await;
        sessions_guard.get(user_id).cloned()
    }

    /// Get all active sessions (for testing)
    pub async fn get_all_sessions(&self) -> HashMap<String, WorkbenchState> {
        self.sessions.read().await.clone()
    }

    /// Clear all Workbench sessions (for testing)
    pub async fn clear_all_sessions(&self) {
        self.sessions.write().await.clear();
    }

    /// Clear a specific user's session (for testing)
    pub async fn clear_session(&self, user_id: &str) {
        self.sessions.write().await.remove(user_id);
    }
}

impl Drop for WorkbenchHandler {
    fn drop(&mut self) {
        // Abort cleanup task when handler is dropped
        if let Some(task) = self.cleanup_task.take() {
            task.abort();
        }
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
    fn test_workbench_state_new() {
        let state = WorkbenchState::new("user1".to_string());
        assert!(state.session_id.is_none());
        assert_eq!(state.user_id, "user1");
        assert!(state.files.is_empty());
        assert!(state.last_output.is_none());
        assert!(state.created_at.is_none());
        assert!(state.last_accessed_at.is_none());
    }

    #[test]
    fn test_workbench_state_is_expired() {
        let mut state = WorkbenchState::new("user1".to_string());
        
        // No last_accessed_at - not expired
        assert!(!state.is_expired(1));
        
        // Recent session - not expired
        state.last_accessed_at = Some(Utc::now());
        assert!(!state.is_expired(1));
        
        // Old session - expired
        state.last_accessed_at = Some(Utc::now() - chrono::Duration::hours(2));
        assert!(state.is_expired(1));
    }

    #[test]
    fn test_workbench_state_touch() {
        let mut state = WorkbenchState::new("user1".to_string());
        assert!(state.last_accessed_at.is_none());
        
        state.touch();
        assert!(state.last_accessed_at.is_some());
        
        let first_access = state.last_accessed_at.unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        state.touch();
        let second_access = state.last_accessed_at.unwrap();
        
        assert!(second_access > first_access);
    }

    #[test]
    fn test_workbench_state_reset() {
        let mut state = WorkbenchState::new("user1".to_string());
        state.session_id = Some("session_123".to_string());
        state.files.insert("file1.txt".to_string(), "content".to_string());
        state.last_output = Some("output".to_string());
        state.created_at = Some(Utc::now());
        state.last_accessed_at = Some(Utc::now());
        
        state.reset();
        
        assert!(state.session_id.is_none());
        assert!(state.files.is_empty());
        assert!(state.last_output.is_none());
        assert!(state.created_at.is_none());
        assert!(state.last_accessed_at.is_none());
    }

    #[tokio::test]
    async fn test_workbench_handler_new() {
        let handler = WorkbenchHandler::new(10, 1);
        let sessions = handler.get_all_sessions().await;
        
        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn test_workbench_handler_clear_sessions() {
        let handler = WorkbenchHandler::new(10, 1);
        
        // Create sessions for multiple users
        let state1 = WorkbenchState::new("user1".to_string());
        let state2 = WorkbenchState::new("user2".to_string());
        
        handler.update_session_state("user1", state1).await;
        handler.update_session_state("user2", state2).await;
        
        let sessions = handler.get_all_sessions().await;
        assert_eq!(sessions.len(), 2);
        
        // Clear all sessions
        handler.clear_all_sessions().await;
        
        let sessions = handler.get_all_sessions().await;
        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn test_ensure_workbench_session_creates_new() {
        let handler = WorkbenchHandler::new(10, 1);
        
        // Mock response with session_id
        let mock_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{
                    "text": r#"{"session_id": "wb_test_123", "output": "Workbench session initialized"}"#
                }]
            }
        });
        
        let mock_client = MockMcpClient {
            response: mock_response,
        };
        
        let session_id = handler
            .ensure_workbench_session(&mock_client, "user1", 1)
            .await
            .unwrap();
        
        assert_eq!(session_id, "wb_test_123");
        
        // Verify state was updated
        let state = handler.get_state("user1").await.unwrap();
        assert_eq!(state.session_id, Some("wb_test_123".to_string()));
        assert_eq!(state.user_id, "user1");
        assert!(state.created_at.is_some());
        assert!(state.last_accessed_at.is_some());
        assert!(state.last_output.is_some());
    }

    #[tokio::test]
    async fn test_ensure_workbench_session_reuses_existing() {
        let handler = WorkbenchHandler::new(10, 1);
        
        // Pre-populate state with valid session
        let mut state = WorkbenchState::new("user1".to_string());
        state.session_id = Some("wb_existing_456".to_string());
        state.created_at = Some(Utc::now());
        state.last_accessed_at = Some(Utc::now());
        handler.update_session_state("user1", state).await;
        
        // Mock client that should not be called
        let mock_client = MockMcpClient {
            response: serde_json::json!({}),
        };
        
        let session_id = handler
            .ensure_workbench_session(&mock_client, "user1", 1)
            .await
            .unwrap();
        
        // Should return existing session_id
        assert_eq!(session_id, "wb_existing_456");
    }

    #[tokio::test]
    async fn test_ensure_workbench_session_recreates_expired() {
        let handler = WorkbenchHandler::new(10, 1);
        
        // Pre-populate state with expired session
        let mut state = WorkbenchState::new("user1".to_string());
        state.session_id = Some("wb_expired_789".to_string());
        state.created_at = Some(Utc::now() - chrono::Duration::hours(2));
        state.last_accessed_at = Some(Utc::now() - chrono::Duration::hours(2));
        handler.update_session_state("user1", state).await;
        
        // Mock response with new session_id
        let mock_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{
                    "text": r#"{"session_id": "wb_new_999", "output": "Workbench session initialized"}"#
                }]
            }
        });
        
        let mock_client = MockMcpClient {
            response: mock_response,
        };
        
        let session_id = handler
            .ensure_workbench_session(&mock_client, "user1", 1)
            .await
            .unwrap();
        
        // Should return new session_id
        assert_eq!(session_id, "wb_new_999");
        
        // Verify old session was reset
        let state = handler.get_state("user1").await.unwrap();
        assert_eq!(state.session_id, Some("wb_new_999".to_string()));
    }

    #[tokio::test]
    async fn test_extract_session_id_from_content_text() {
        let result_data = serde_json::json!({
            "content": [{
                "text": r#"{"session_id": "wb_content_123"}"#
            }]
        });
        
        let session_id = WorkbenchHandler::extract_session_id(&result_data).unwrap();
        assert_eq!(session_id, "wb_content_123");
    }

    #[tokio::test]
    async fn test_extract_session_id_from_direct_result() {
        let result_data = serde_json::json!({
            "session_id": "wb_direct_456"
        });
        
        let session_id = WorkbenchHandler::extract_session_id(&result_data).unwrap();
        assert_eq!(session_id, "wb_direct_456");
    }

    #[tokio::test]
    async fn test_extract_session_id_generates_if_missing() {
        let result_data = serde_json::json!({
            "output": "Some output without session_id"
        });
        
        let session_id = WorkbenchHandler::extract_session_id(&result_data).unwrap();
        assert!(session_id.starts_with("wb_"));
    }

    #[tokio::test]
    async fn test_execute_via_workbench_success() {
        let handler = WorkbenchHandler::new(10, 1);
        
        // Mock response for session creation
        let session_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{
                    "text": r#"{"session_id": "wb_exec_123", "output": "Workbench session initialized"}"#
                }]
            }
        });
        
        // Mock response for code execution
        let exec_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "result": {
                "content": [{
                    "text": r#"{"output": "Execution result: 42"}"#
                }]
            }
        });
        
        // Create a mock client that returns different responses
        struct MultiResponseMockClient {
            call_count: Arc<RwLock<usize>>,
            responses: Vec<Value>,
        }
        
        #[async_trait::async_trait]
        impl McpClientTrait for MultiResponseMockClient {
            async fn tools_call(
                &self,
                _request_id: i64,
                _tool_name: &str,
                _params: Value,
            ) -> Result<Value> {
                let mut count = self.call_count.write().await;
                let response = self.responses[*count].clone();
                *count += 1;
                Ok(response)
            }
        }
        
        let mock_client = MultiResponseMockClient {
            call_count: Arc::new(RwLock::new(0)),
            responses: vec![session_response, exec_response],
        };
        
        let result = handler
            .execute_via_workbench(&mock_client, "print(42)", "user1", 1)
            .await
            .unwrap();
        
        assert!(result.success);
        assert!(result.output.contains("42"));
        assert!(result.error.is_none());
        
        // Verify state was updated
        let state = handler.get_state("user1").await.unwrap();
        assert!(state.last_output.is_some());
    }

    #[tokio::test]
    async fn test_execute_via_workbench_with_existing_session() {
        let handler = WorkbenchHandler::new(10, 1);
        
        // Pre-populate state with valid session
        let mut state = WorkbenchState::new("user1".to_string());
        state.session_id = Some("wb_existing_789".to_string());
        state.created_at = Some(Utc::now());
        state.last_accessed_at = Some(Utc::now());
        handler.update_session_state("user1", state).await;
        
        // Mock response for code execution (no session creation needed)
        let exec_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{
                    "text": r#"{"output": "Using existing session"}"#
                }]
            }
        });
        
        let mock_client = MockMcpClient {
            response: exec_response,
        };
        
        let result = handler
            .execute_via_workbench(&mock_client, "print('test')", "user1", 1)
            .await
            .unwrap();
        
        assert!(result.success);
        assert!(result.output.contains("existing session"));
    }

    #[tokio::test]
    async fn test_execute_via_workbench_error() {
        let handler = WorkbenchHandler::new(10, 1);
        
        // Mock error response
        let error_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": -32000,
                "message": "Python execution error: NameError"
            }
        });
        
        struct ErrorMockClient {
            response: Value,
        }
        
        #[async_trait::async_trait]
        impl McpClientTrait for ErrorMockClient {
            async fn tools_call(
                &self,
                _request_id: i64,
                _tool_name: &str,
                _params: Value,
            ) -> Result<Value> {
                Ok(self.response.clone())
            }
        }
        
        let mock_client = ErrorMockClient {
            response: error_response,
        };
        
        let result = handler
            .execute_via_workbench(&mock_client, "invalid_code", "user1", 1)
            .await
            .unwrap();
        
        assert!(!result.success);
        assert!(result.output.contains("Python execution error"));
        assert!(result.error.is_some());
    }

    #[test]
    fn test_extract_workbench_output_from_content_text() {
        let result_data = serde_json::json!({
            "content": [{
                "text": r#"{"output": "Test output from Workbench"}"#
            }]
        });
        
        let output = WorkbenchHandler::extract_workbench_output(&result_data).unwrap();
        assert_eq!(output, "Test output from Workbench");
    }

    #[test]
    fn test_extract_workbench_output_plain_text() {
        let result_data = serde_json::json!({
            "content": [{
                "text": "Plain text output"
            }]
        });
        
        let output = WorkbenchHandler::extract_workbench_output(&result_data).unwrap();
        assert_eq!(output, "Plain text output");
    }

    #[test]
    fn test_extract_workbench_output_direct_result() {
        let result_data = serde_json::json!({
            "output": "Direct result output"
        });
        
        let output = WorkbenchHandler::extract_workbench_output(&result_data).unwrap();
        assert_eq!(output, "Direct result output");
    }

    #[test]
    fn test_summarize_output_small() {
        let small_output = "Short output";
        let summary = WorkbenchHandler::summarize_workbench_output(small_output);
        assert_eq!(summary, small_output);
    }

    #[test]
    fn test_summarize_output_large() {
        let large_output = "x".repeat(20000);
        let summary = WorkbenchHandler::summarize_workbench_output(&large_output);
        
        // Should be shorter than original
        assert!(summary.len() < large_output.len());
        
        // Should contain truncation notice
        assert!(summary.contains("Output truncated"));
        
        // Should contain head and tail
        assert!(summary.starts_with("xxx"));
        assert!(summary.ends_with("xxx"));
    }

    // Feature: composio-permanent-integration, Property 28: Workbench Session Persistence
    #[tokio::test]
    async fn test_session_isolation_per_user() {
        let handler = WorkbenchHandler::new(10, 1);
        
        // Mock response for session creation
        let mock_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{
                    "text": r#"{"session_id": "wb_session", "output": "Initialized"}"#
                }]
            }
        });
        
        let mock_client = MockMcpClient {
            response: mock_response,
        };
        
        // Create sessions for different users
        let session1 = handler
            .ensure_workbench_session(&mock_client, "user1", 1)
            .await
            .unwrap();
        
        let session2 = handler
            .ensure_workbench_session(&mock_client, "user2", 2)
            .await
            .unwrap();
        
        // Sessions should be isolated
        let state1 = handler.get_state("user1").await.unwrap();
        let state2 = handler.get_state("user2").await.unwrap();
        
        assert_eq!(state1.user_id, "user1");
        assert_eq!(state2.user_id, "user2");
        assert_ne!(session1, session2);
    }

    // Feature: composio-permanent-integration, Property 28: Workbench Session Persistence
    #[tokio::test]
    async fn test_lru_eviction_when_max_sessions_reached() {
        let handler = WorkbenchHandler::new(3, 1); // Max 3 sessions
        
        // Mock response
        let mock_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{
                    "text": r#"{"session_id": "wb_session", "output": "Initialized"}"#
                }]
            }
        });
        
        let mock_client = MockMcpClient {
            response: mock_response,
        };
        
        // Create 3 sessions
        handler.ensure_workbench_session(&mock_client, "user1", 1).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        handler.ensure_workbench_session(&mock_client, "user2", 2).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        handler.ensure_workbench_session(&mock_client, "user3", 3).await.unwrap();
        
        // Verify 3 sessions exist
        let sessions = handler.get_all_sessions().await;
        assert_eq!(sessions.len(), 3);
        
        // Create 4th session - should evict LRU (user1)
        handler.ensure_workbench_session(&mock_client, "user4", 4).await.unwrap();
        
        let sessions = handler.get_all_sessions().await;
        assert_eq!(sessions.len(), 3); // Still max 3
        assert!(sessions.contains_key("user2"));
        assert!(sessions.contains_key("user3"));
        assert!(sessions.contains_key("user4"));
        assert!(!sessions.contains_key("user1")); // user1 was evicted
    }

    // Feature: composio-permanent-integration, Property 28: Workbench Session Persistence
    #[tokio::test]
    async fn test_session_cleanup_expired_sessions() {
        let handler = WorkbenchHandler::new(10, 1);
        
        // Create expired session
        let mut expired_state = WorkbenchState::new("user_expired".to_string());
        expired_state.session_id = Some("wb_expired".to_string());
        expired_state.created_at = Some(Utc::now() - chrono::Duration::hours(2));
        expired_state.last_accessed_at = Some(Utc::now() - chrono::Duration::hours(2));
        handler.update_session_state("user_expired", expired_state).await;
        
        // Create active session
        let mut active_state = WorkbenchState::new("user_active".to_string());
        active_state.session_id = Some("wb_active".to_string());
        active_state.created_at = Some(Utc::now());
        active_state.last_accessed_at = Some(Utc::now());
        handler.update_session_state("user_active", active_state).await;
        
        // Verify both sessions exist
        let sessions = handler.get_all_sessions().await;
        assert_eq!(sessions.len(), 2);
        
        // Manually trigger cleanup (simulating background task)
        {
            let mut sessions_guard = handler.sessions.write().await;
            let expired_keys: Vec<String> = sessions_guard
                .iter()
                .filter(|(_, state)| state.is_expired(1))
                .map(|(key, _)| key.clone())
                .collect();
            
            for key in expired_keys {
                sessions_guard.remove(&key);
            }
        }
        
        // Verify only active session remains
        let sessions = handler.get_all_sessions().await;
        assert_eq!(sessions.len(), 1);
        assert!(sessions.contains_key("user_active"));
        assert!(!sessions.contains_key("user_expired"));
    }

    // Feature: composio-permanent-integration, Property 28: Workbench Session Persistence
    #[tokio::test]
    async fn test_multi_step_workbench_operations() {
        let handler = WorkbenchHandler::new(10, 1);
        
        // Mock responses for multi-step operations
        struct MultiStepMockClient {
            call_count: Arc<RwLock<usize>>,
        }
        
        #[async_trait::async_trait]
        impl McpClientTrait for MultiStepMockClient {
            async fn tools_call(
                &self,
                _request_id: i64,
                _tool_name: &str,
                _params: Value,
            ) -> Result<Value> {
                let mut count = self.call_count.write().await;
                *count += 1;
                
                let response = if *count == 1 {
                    // Session initialization
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": 1,
                        "result": {
                            "content": [{
                                "text": r#"{"session_id": "wb_multi_step", "output": "Initialized"}"#
                            }]
                        }
                    })
                } else {
                    // Subsequent operations
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": *count as i64,
                        "result": {
                            "content": [{
                                "text": format!(r#"{{"output": "Step {} completed"}}"#, *count - 1)
                            }]
                        }
                    })
                };
                
                Ok(response)
            }
        }
        
        let mock_client = MultiStepMockClient {
            call_count: Arc::new(RwLock::new(0)),
        };
        
        // Step 1: Execute first operation
        let result1 = handler
            .execute_via_workbench(&mock_client, "step1_code", "user1", 1)
            .await
            .unwrap();
        assert!(result1.success);
        assert!(result1.output.contains("Step 1 completed"));
        
        // Step 2: Execute second operation (should reuse session)
        let result2 = handler
            .execute_via_workbench(&mock_client, "step2_code", "user1", 2)
            .await
            .unwrap();
        assert!(result2.success);
        assert!(result2.output.contains("Step 2 completed"));
        
        // Verify session was reused (state persisted)
        let state = handler.get_state("user1").await.unwrap();
        assert_eq!(state.session_id, Some("wb_multi_step".to_string()));
        assert!(state.last_output.is_some());
    }

    // Feature: composio-permanent-integration, Property 28: Workbench Session Persistence
    #[tokio::test]
    async fn test_session_touch_updates_last_accessed() {
        let handler = WorkbenchHandler::new(10, 1);
        
        // Create initial session
        let mut state = WorkbenchState::new("user1".to_string());
        state.session_id = Some("wb_test".to_string());
        state.created_at = Some(Utc::now());
        state.last_accessed_at = Some(Utc::now() - chrono::Duration::minutes(30));
        handler.update_session_state("user1", state).await;
        
        let initial_access = handler.get_state("user1").await.unwrap().last_accessed_at.unwrap();
        
        // Mock response
        let mock_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{
                    "text": r#"{"output": "Test output"}"#
                }]
            }
        });
        
        let mock_client = MockMcpClient {
            response: mock_response,
        };
        
        // Execute operation (should touch session)
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        handler
            .execute_via_workbench(&mock_client, "test_code", "user1", 1)
            .await
            .unwrap();
        
        // Verify last_accessed_at was updated
        let updated_access = handler.get_state("user1").await.unwrap().last_accessed_at.unwrap();
        assert!(updated_access > initial_access);
    }
}
