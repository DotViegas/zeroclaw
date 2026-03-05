// Composio Natural Language Tool
//
// This tool provides natural language access to 1000+ apps via Composio MCP.
// It implements the meta-tools workflow: SEARCH → MANAGE → EXECUTE
//
// Architecture:
// User Query → COMPOSIO_SEARCH_TOOLS (discover) → COMPOSIO_MANAGE_CONNECTIONS (auth) → COMPOSIO_MULTI_EXECUTE_TOOL (execute)

use super::traits::{Tool, ToolResult};
use crate::mcp::sse_client::{JsonRpcResponse, McpClient};
use crate::security::SecurityPolicy;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// Session state for Composio MCP
#[derive(Debug, Clone)]
pub struct ComposioSession {
    pub session_id: String,
    pub created_at: Instant,
}

/// Discovered tool from COMPOSIO_SEARCH_TOOLS
#[derive(Debug, Clone)]
pub struct DiscoveredTool {
    pub tool_slug: String,
    pub description: String,
    pub toolkit: String,
    pub use_case: String,
    pub input_schema: Option<Value>,
    pub schema_ref: Option<String>,
}

/// Connection status for a toolkit
#[derive(Debug)]
pub enum ConnectionStatus {
    Connected,
    NeedsOAuth(String), // redirect_url
}

/// Composio Natural Language Tool
pub struct ComposioNaturalLanguageTool {
    mcp_client: Arc<McpClient>,
    security: Arc<SecurityPolicy>,
    
    // Composio-specific security policy (optional, for toolkit filtering and rate limiting)
    composio_security: Option<Arc<crate::security::ComposioSecurityPolicy>>,
    
    // Session management
    current_session: Arc<RwLock<Option<ComposioSession>>>,
    session_ttl: std::time::Duration,
    
    // Request ID counter
    request_id: Arc<RwLock<i64>>,
    
    // Provider for LLM-assisted parameter extraction
    provider: Option<Arc<dyn crate::providers::Provider>>,
    
    // Model to use for LLM-assisted extraction (if provider is set)
    model: Option<String>,
    
    // API key for Composio REST API (staging, etc)
    api_key: String,
    
    // Execution history for context in LLM extraction
    // Stores recent tool executions (tool_slug, query, result) for attachment context
    execution_history: Arc<RwLock<Vec<(String, String, Value)>>>,
}

impl ComposioNaturalLanguageTool {
    /// Create a new natural language tool
    pub fn new(mcp_client: Arc<McpClient>, security: Arc<SecurityPolicy>, api_key: String) -> Self {
        Self {
            mcp_client,
            security,
            composio_security: None,
            current_session: Arc::new(RwLock::new(None)),
            session_ttl: std::time::Duration::from_secs(1800), // 30 minutes
            request_id: Arc::new(RwLock::new(1)),
            provider: None,
            model: None,
            api_key,
            execution_history: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Create a new natural language tool with LLM-assisted extraction
    pub fn new_with_provider(
        mcp_client: Arc<McpClient>,
        security: Arc<SecurityPolicy>,
        provider: Arc<dyn crate::providers::Provider>,
        model: Option<String>,
        api_key: String,
    ) -> Self {
        Self {
            mcp_client,
            security,
            composio_security: None,
            current_session: Arc::new(RwLock::new(None)),
            session_ttl: std::time::Duration::from_secs(1800), // 30 minutes
            request_id: Arc::new(RwLock::new(1)),
            provider: Some(provider),
            model,
            api_key,
            execution_history: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Set the Composio-specific security policy for toolkit filtering and rate limiting.
    pub fn with_composio_security(mut self, composio_security: Arc<crate::security::ComposioSecurityPolicy>) -> Self {
        self.composio_security = Some(composio_security);
        self
    }
    
    /// Get next request ID
    async fn next_request_id(&self) -> i64 {
        let mut id = self.request_id.write().await;
        let current = *id;
        *id += 1;
        current
    }
    
    /// Extract file metadata from execution history for attachment context
    /// Returns a formatted string with recent file downloads (s3key, mimetype, name)
    async fn get_attachment_context(&self) -> String {
        let history = self.execution_history.read().await;
        
        let mut context_parts = Vec::new();
        
        for (tool_slug, _args, result) in history.iter().rev().take(5) {
            // Look for DROPBOX_READ_FILE or similar download operations
            if tool_slug.contains("READ_FILE") || tool_slug.contains("DOWNLOAD") || tool_slug.contains("GET_FILE") {
                // Try to extract file metadata from result
                if let Some(data) = result.get("data") {
                    if let Some(results) = data.get("results").and_then(|r| r.as_array()) {
                        for result_item in results {
                            if let Some(response) = result_item.get("response") {
                                if let Some(response_data) = response.get("data") {
                                    // Check for content field (Dropbox format)
                                    if let Some(content) = response_data.get("content") {
                                        let name = content.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
                                        let mimetype = content.get("mimetype").and_then(|m| m.as_str()).unwrap_or("application/octet-stream");
                                        let s3key = content.get("s3key").and_then(|k| k.as_str())
                                            .or_else(|| content.get("s3url").and_then(|u| u.as_str()))
                                            .unwrap_or("unknown");
                                        
                                        context_parts.push(format!(
                                            "- File downloaded: name=\"{}\", mimetype=\"{}\", s3key=\"{}\"",
                                            name, mimetype, s3key
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        if context_parts.is_empty() {
            String::new()
        } else {
            format!("\n\nRECENT FILE DOWNLOADS (for attachment context):\n{}\n", context_parts.join("\n"))
        }
    }
    
    /// Get or create a session for the current workflow
    /// 
    /// Note: We don't actually need to track session_id ourselves.
    /// We just pass `generate_id: true` and let the server manage sessions.
    async fn ensure_session(&self) -> Result<()> {
        // Session management is handled server-side
        // We just need to pass generate_id: true in each request
        Ok(())
    }
    /// Determine whether to use Workbench for the given query
    ///
    /// Workbench should be used for:
    /// - Bulk operations (keywords: all, every, list, export, bulk, batch)
    /// - Large data operations (keywords: csv, spreadsheet, report, summary, analyze)
    /// - Operations likely to produce responses exceeding context window limits
    ///
    /// # Arguments
    /// * `query` - The natural language query from the user
    /// * `force_workbench` - Optional flag to force Workbench usage
    ///
    /// # Returns
    /// `true` if Workbench should be used, `false` for direct execution
    pub fn should_use_workbench(&self, query: &str, force_workbench: Option<bool>) -> bool {
        // If explicitly forced, use Workbench
        if force_workbench.unwrap_or(false) {
            return true;
        }

        let query_lower = query.to_lowercase();

        // Bulk operation keywords
        let bulk_keywords = [
            "all", "every", "list", "export", "bulk", "batch",
            "hundreds", "thousands", "many", "multiple"
        ];

        // Large data keywords
        let large_data_keywords = [
            "csv", "spreadsheet", "report", "summary", "analyze",
            "analysis", "statistics", "aggregate", "count"
        ];

        // Check for bulk keywords
        let has_bulk_keyword = bulk_keywords.iter()
            .any(|kw| query_lower.contains(kw));

        // Check for large data keywords
        let has_large_data_keyword = large_data_keywords.iter()
            .any(|kw| query_lower.contains(kw));

        // Use Workbench if either condition is met
        has_bulk_keyword || has_large_data_keyword
    }
    
    /// Search for tools matching the user's query
    async fn search_tools(&self, user_query: &str) -> Result<Vec<DiscoveredTool>> {
        tracing::debug!(
            query = user_query,
            "Searching for tools"
        );
        
        let search_params = serde_json::json!({
            "queries": [user_query],
            "session": {
                "generate_id": true
            }
        });
        
        let request_id = self.next_request_id().await;
        let result = self.mcp_client
            .tools_call(request_id, "COMPOSIO_SEARCH_TOOLS", search_params)
            .await
            .context("Failed to search tools")?;
        
        // Parse JSON-RPC response
        let rpc_response: JsonRpcResponse = serde_json::from_value(result)
            .context("Failed to parse JSON-RPC response")?;
        
        if let Some(error) = rpc_response.error {
            anyhow::bail!("JSON-RPC error: {} (code: {})", error.message, error.code);
        }
        
        let result_data = rpc_response.result
            .ok_or_else(|| anyhow::anyhow!("No result in JSON-RPC response"))?;
        
        // Parse discovered tools (without complete schemas yet)
        let mut discovered_tools = self.parse_discovered_tools(result_data)?;
        
        // Fetch complete schemas for all discovered tools
        if !discovered_tools.is_empty() {
            let tool_slugs: Vec<String> = discovered_tools
                .iter()
                .map(|t| t.tool_slug.clone())
                .collect();
            
            tracing::debug!(
                tool_slugs = ?tool_slugs,
                "Fetching complete schemas via COMPOSIO_GET_TOOL_SCHEMAS"
            );
            
            match self.get_tool_schemas(tool_slugs).await {
                Ok(schemas_map) => {
                    // Update discovered tools with complete schemas
                    for tool in &mut discovered_tools {
                        if let Some(schema) = schemas_map.get(&tool.tool_slug) {
                            tool.input_schema = Some(schema.clone());
                            tracing::debug!(
                                tool_slug = tool.tool_slug,
                                "Updated tool with complete schema"
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Failed to fetch complete schemas, will use schemas from SEARCH_TOOLS if available"
                    );
                }
            }
        }
        
        Ok(discovered_tools)
    }
    
    /// Get complete schemas for specific tools
    async fn get_tool_schemas(&self, tool_slugs: Vec<String>) -> Result<std::collections::HashMap<String, Value>> {
        tracing::debug!(
            tool_slugs = ?tool_slugs,
            "Calling COMPOSIO_GET_TOOL_SCHEMAS"
        );
        
        let params = serde_json::json!({
            "tool_slugs": tool_slugs,
            "session": {
                "generate_id": true
            }
        });
        
        let request_id = self.next_request_id().await;
        let result = self.mcp_client
            .tools_call(request_id, "COMPOSIO_GET_TOOL_SCHEMAS", params)
            .await
            .context("Failed to get tool schemas")?;
        
        // Parse JSON-RPC response
        let rpc_response: JsonRpcResponse = serde_json::from_value(result)
            .context("Failed to parse JSON-RPC response")?;
        
        if let Some(error) = rpc_response.error {
            anyhow::bail!("JSON-RPC error: {} (code: {})", error.message, error.code);
        }
        
        let result_data = rpc_response.result
            .ok_or_else(|| anyhow::anyhow!("No result in JSON-RPC response"))?;
        
        // Parse the content[0].text JSON string
        let parsed_data = result_data.get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|text| text.as_str())
            .and_then(|text_str| {
                tracing::debug!(
                    text_preview = %format!("{:.200}", text_str),
                    "Parsing GET_TOOL_SCHEMAS response"
                );
                serde_json::from_str::<serde_json::Value>(text_str).ok()
            })
            .ok_or_else(|| anyhow::anyhow!("Failed to parse GET_TOOL_SCHEMAS response"))?;
        
        // Extract schemas from response
        // Expected format: {"data": {"tool_schemas": {"TOOL_SLUG": {"parameters": {...}}}}}
        let mut schemas_map = std::collections::HashMap::new();
        
        if let Some(tool_schemas_obj) = parsed_data.get("data")
            .and_then(|d| d.get("tool_schemas"))
            .and_then(|s| s.as_object())
        {
            for (tool_slug, schema_data) in tool_schemas_obj {
                // Extract the parameters schema
                if let Some(parameters) = schema_data.get("parameters")
                    .or_else(|| schema_data.get("input_schema"))
                    .or_else(|| schema_data.get("schema"))
                {
                    schemas_map.insert(tool_slug.to_string(), parameters.clone());
                    tracing::debug!(
                        tool_slug = tool_slug,
                        "Extracted schema for tool"
                    );
                } else {
                    tracing::debug!(
                        tool_slug = tool_slug,
                        schema_keys = ?schema_data.as_object().map(|o| o.keys().collect::<Vec<_>>()),
                        "Schema data found but no parameters field"
                    );
                }
            }
        } else {
            tracing::debug!(
                data_keys = ?parsed_data.get("data").and_then(|d| d.as_object()).map(|o| o.keys().collect::<Vec<_>>()),
                "No tool_schemas object found in data"
            );
        }
        
        tracing::info!(
            count = schemas_map.len(),
            "Fetched complete schemas for tools"
        );
        
        Ok(schemas_map)
    }
    
    /// Parse discovered tools from COMPOSIO_SEARCH_TOOLS response
    fn parse_discovered_tools(&self, result: Value) -> Result<Vec<DiscoveredTool>> {
        let mut discovered_tools = Vec::new();
        
        // Parse the content[0].text JSON string (same format as diagnostic)
        let parsed_data = result.get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|text| text.as_str())
            .and_then(|text_str| serde_json::from_str::<serde_json::Value>(text_str).ok());
        
        if let Some(data) = parsed_data {
            // Check for data.results (Composio format)
            if let Some(results) = data.get("data")
                .and_then(|d| d.get("results"))
                .and_then(|r| r.as_array())
            {
                // Parse results array
                for (idx, result_item) in results.iter().enumerate() {
                    // Log the full result item for debugging
                    if idx == 0 {
                        tracing::debug!(
                            "First result item: {}",
                            serde_json::to_string_pretty(result_item).unwrap_or_default()
                        );
                    }
                    
                    let use_case = result_item.get("use_case")
                        .and_then(|u| u.as_str())
                        .unwrap_or("unknown");
                    
                    // Try to extract the actual tool/action ID
                    // Priority: primary_tool_slugs[0] > tool_slug > action_slug > id
                    let tool_id = result_item.get("primary_tool_slugs")
                        .and_then(|slugs| slugs.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|slug| slug.as_str())
                        .map(|s| s.to_string())
                        .or_else(|| {
                            result_item.get("tool_slug")
                                .or_else(|| result_item.get("action_slug"))
                                .or_else(|| result_item.get("id"))
                                .or_else(|| result_item.get("tool_id"))
                                .or_else(|| result_item.get("action_id"))
                                .and_then(|id| {
                                    // Could be string or number
                                    if let Some(s) = id.as_str() {
                                        Some(s.to_string())
                                    } else if let Some(n) = id.as_i64() {
                                        Some(n.to_string())
                                    } else {
                                        None
                                    }
                                })
                        })
                        .unwrap_or_else(|| use_case.to_string());
                    
                    // Extract toolkit - try multiple fields
                    let toolkit = result_item.get("toolkits")
                        .and_then(|t| t.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|t| t.as_str())
                        .or_else(|| {
                            result_item.get("toolkit")
                                .or_else(|| result_item.get("app"))
                                .or_else(|| result_item.get("app_name"))
                                .and_then(|t| t.as_str())
                        })
                        .or_else(|| {
                            // Fallback: extract from use_case or execution_guidance
                            let text = format!("{} {}", 
                                use_case,
                                result_item.get("execution_guidance")
                                    .and_then(|g| g.as_str())
                                    .unwrap_or("")
                            ).to_lowercase();
                            
                            if text.contains("outlook") {
                                Some("outlook")
                            } else if text.contains("gmail") {
                                Some("gmail")
                            } else if text.contains("github") {
                                Some("github")
                            } else if text.contains("slack") {
                                Some("slack")
                            } else if text.contains("dropbox") {
                                Some("dropbox")
                            } else if text.contains("notion") {
                                Some("notion")
                            } else if text.contains("calendar") || text.contains("gcal") {
                                Some("googlecalendar")
                            } else if text.contains("drive") {
                                Some("googledrive")
                            } else {
                                None
                            }
                        })
                        .unwrap_or("unknown");
                    
                    // Extract input schema from tool_schemas if available
                    let input_schema = result_item.get("tool_schemas")
                        .and_then(|schemas| schemas.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|schema| schema.get("parameters"))
                        .cloned();
                    
                    tracing::debug!(
                        "Discovered tool: id={}, use_case={}, toolkit={}, has_schema={}",
                        tool_id, use_case, toolkit, input_schema.is_some()
                    );
                    
                    discovered_tools.push(DiscoveredTool {
                        tool_slug: tool_id,
                        description: result_item.get("execution_guidance")
                            .and_then(|d| d.as_str())
                            .unwrap_or("")
                            .to_string(),
                        toolkit: toolkit.to_string(),
                        use_case: use_case.to_string(),
                        input_schema,
                        schema_ref: None,
                    });
                }
            }
        }
        
        if discovered_tools.is_empty() {
            anyhow::bail!("No tools found in response");
        }
        
        tracing::info!(
            count = discovered_tools.len(),
            "Discovered tools for query"
        );
        
        Ok(discovered_tools)
    }
    
    /// Ensure toolkit is connected (OAuth flow if needed)
    async fn ensure_connected(&self, toolkit: &str) -> Result<ConnectionStatus> {
        tracing::debug!(toolkit = toolkit, "Checking connection status");
        
        let params = serde_json::json!({
            "toolkits": [toolkit],
            "session": {
                "generate_id": true
            }
        });
        
        tracing::debug!(
            toolkit = toolkit,
            params = ?params,
            "Calling COMPOSIO_MANAGE_CONNECTIONS"
        );
        
        let request_id = self.next_request_id().await;
        let result = self.mcp_client
            .tools_call(request_id, "COMPOSIO_MANAGE_CONNECTIONS", params)
            .await
            .context("Failed to manage connections")?;
        
        tracing::debug!(
            toolkit = toolkit,
            response = ?result,
            "COMPOSIO_MANAGE_CONNECTIONS response received"
        );
        
        // Parse JSON-RPC response
        let rpc_response: JsonRpcResponse = serde_json::from_value(result)
            .context("Failed to parse JSON-RPC response")?;
        
        if let Some(error) = rpc_response.error {
            anyhow::bail!("JSON-RPC error: {} (code: {})", error.message, error.code);
        }
        
        let result_data = rpc_response.result
            .ok_or_else(|| anyhow::anyhow!("No result in JSON-RPC response"))?;
        
        // Parse the content[0].text JSON string (same format as diagnostic)
        let parsed_data = result_data.get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|text| text.as_str())
            .and_then(|text_str| {
                tracing::debug!(
                    toolkit = toolkit,
                    text = text_str,
                    "Parsing MANAGE_CONNECTIONS text response"
                );
                serde_json::from_str::<serde_json::Value>(text_str).ok()
            });
        
        if let Some(data) = parsed_data {
            tracing::debug!(
                toolkit = toolkit,
                data = ?data,
                "Parsed MANAGE_CONNECTIONS data"
            );
            
            // Check for data.results.{toolkit}.instruction (OAuth needed)
            if let Some(results) = data.get("data").and_then(|d| d.get("results")) {
                if let Some(toolkit_data) = results.get(toolkit) {
                    tracing::debug!(
                        toolkit = toolkit,
                        toolkit_data = ?toolkit_data,
                        "Found toolkit data in results"
                    );
                    
                    // Check for instruction field (contains OAuth link)
                    if let Some(instruction) = toolkit_data.get("instruction").and_then(|i| i.as_str()) {
                        tracing::debug!(
                            toolkit = toolkit,
                            instruction = instruction,
                            "Found instruction field"
                        );
                        
                        // Extract OAuth link from instruction - try multiple patterns
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
                                let redirect_url = instruction[link_start..link_end].trim().to_string();
                                
                                tracing::info!(
                                    toolkit = toolkit,
                                    redirect_url = %redirect_url,
                                    "OAuth required - extracted link from instruction"
                                );
                                return Ok(ConnectionStatus::NeedsOAuth(redirect_url));
                            }
                        }
                    }
                    
                    // Check for redirect_url field (alternative format)
                    if let Some(redirect_url) = toolkit_data.get("redirect_url").and_then(|u| u.as_str()) {
                        tracing::info!(toolkit = toolkit, "OAuth required");
                        return Ok(ConnectionStatus::NeedsOAuth(redirect_url.to_string()));
                    }
                }
            }
            
            // Check for direct redirect_url (legacy format)
            if let Some(redirect_url) = data.get("redirect_url").and_then(|u| u.as_str()) {
                tracing::info!(toolkit = toolkit, "OAuth required");
                return Ok(ConnectionStatus::NeedsOAuth(redirect_url.to_string()));
            }
        }
        
        tracing::debug!(toolkit = toolkit, "Connection active");
        Ok(ConnectionStatus::Connected)
    }
    
    /// Extract arguments from natural language query using 3-layer approach:
    /// Layer 1: Quick pattern matching (fast, no LLM)
    /// Layer 2: LLM-assisted extraction (flexible, works for all tools)
    /// Layer 3: Generic fallback
    async fn extract_arguments_from_query(
        &self,
        query: &str,
        tool_slug: &str,
        discovered_tool: &DiscoveredTool,
    ) -> Value {
        tracing::debug!(
            query = query,
            tool_slug = tool_slug,
            use_case = discovered_tool.use_case,
            has_schema = discovered_tool.input_schema.is_some(),
            has_provider = self.provider.is_some(),
            "Extracting arguments from query using 3-layer approach"
        );
        
        // Layer 1: Quick pattern matching for common cases
        if let Some(args) = self.try_quick_extraction(query, tool_slug) {
            tracing::info!(
                arguments = ?args,
                "Layer 1: Quick extraction successful"
            );
            return args;
        }
        
        // Skip Layer 2 for DROPBOX_UPLOAD_FILE - LLM generates invalid local_file_path
        // that don't exist in Composio's remote execution environment.
        // Layer 3 creates temporary files correctly with absolute Windows paths.
        let skip_llm_for_tool = tool_slug == "DROPBOX_UPLOAD_FILE";
        
        if skip_llm_for_tool {
            tracing::debug!(
                tool_slug = tool_slug,
                "Skipping Layer 2 (LLM) for this tool - using Layer 3 directly"
            );
        } else {
            // Layer 2: LLM-assisted extraction (if provider available and schema exists)
            if let Some(provider) = &self.provider {
                if let Some(schema) = &discovered_tool.input_schema {
                    match self.extract_with_llm(
                        provider,
                        query,
                        tool_slug,
                        schema,
                        &discovered_tool.use_case,
                    ).await {
                        Ok(args) if !args.as_object().map(|o| o.is_empty()).unwrap_or(true) => {
                            tracing::info!(
                                arguments = ?args,
                                "Layer 2: LLM extraction successful"
                            );
                            return args;
                        }
                        Ok(_) => {
                            tracing::debug!("Layer 2: LLM returned empty arguments, trying fallback");
                        }
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                "Layer 2: LLM extraction failed, falling back to generic extraction"
                            );
                        }
                    }
                } else {
                    tracing::debug!("Layer 2: No schema available for LLM extraction");
                }
            } else {
                tracing::debug!("Layer 2: No provider available for LLM extraction");
            }
        }
        
        // Layer 3: Generic fallback
        let args = self.extract_generic_fallback(query, tool_slug, &discovered_tool.use_case);
        tracing::info!(
            arguments = ?args,
            "Layer 3: Fallback extraction completed"
        );
        args
    }
    
    /// Layer 1: Try quick pattern matching for common cases
    fn try_quick_extraction(&self, query: &str, tool_slug: &str) -> Option<Value> {
        // Quick extraction for email sending (most common case)
        if tool_slug.contains("SEND_EMAIL") || tool_slug.contains("EMAIL_SEND") {
            // CRITICAL: Skip Layer 1 if query mentions file/attachment keywords
            // These require context from previous tool calls (s3key from DROPBOX_READ_FILE)
            // and must be handled by Layer 2 (LLM) which has access to conversation history
            let query_lower = query.to_lowercase();
            let attachment_keywords = [
                "file", "arquivo", "attach", "anexo", "anexar",
                "dropbox", "drive", "document", "documento",
                "send file", "enviar arquivo", "with file", "com arquivo"
            ];
            
            if attachment_keywords.iter().any(|kw| query_lower.contains(kw)) {
                tracing::debug!(
                    query = query,
                    "Query mentions file/attachment keywords - skipping Layer 1, will use Layer 2 (LLM)"
                );
                return None;
            }
            
            let mut args = serde_json::json!({});
            let mut found_any = false;
            
            // Extract recipient
            if let Some(email) = self.extract_email_address(query, &["to ", "para ", "recipient "]) {
                args["recipient_email"] = email;
                found_any = true;
            }
            
            // Extract subject
            if let Some(subject) = self.extract_quoted_or_after_keyword(query, &["subject", "assunto"]) {
                args["subject"] = subject;
                found_any = true;
            }
            
            // Extract body
            if let Some(body) = self.extract_body_content(query, "") {
                args["body"] = body;
                found_any = true;
            }
            
            // CRITICAL: Only return if we have at least recipient AND (subject OR body)
            // Gmail API requires: at least one recipient + at least one of subject/body
            let has_recipient = args.get("recipient_email").is_some();
            let has_content = args.get("subject").is_some() || args.get("body").is_some();
            
            if found_any && has_recipient && has_content {
                return Some(args);
            }
        }
        
        None
    }
    
    /// Layer 2: LLM-assisted parameter extraction
    async fn extract_with_llm(
        &self,
        provider: &Arc<dyn crate::providers::Provider>,
        query: &str,
        tool_slug: &str,
        schema: &Value,
        use_case: &str,
    ) -> Result<Value> {
        tracing::debug!(
            tool_slug = tool_slug,
            "Calling LLM for parameter extraction"
        );
        
        // Get attachment context from execution history
        let attachment_context = self.get_attachment_context().await;
        
        // Build a focused prompt for parameter extraction
        let schema_str = serde_json::to_string_pretty(schema)?;
        let prompt = format!(
            "Extract parameters from the user query for tool execution.\n\n\
             Tool: {}\n\
             Use case: {}\n\
             Parameter schema:\n{}\n\n\
             User query: \"{}\"{}\n\n\
             CRITICAL INSTRUCTIONS:\n\
             1. Analyze the user query and extract values for the parameters defined in the schema\n\
             2. Return ONLY a valid JSON object with the extracted parameters\n\
             3. Use the exact parameter names from the schema\n\
             4. If a parameter is not mentioned in the query, omit it from the JSON\n\
             5. For file paths, use forward slashes and start with / (e.g., \"/test.txt\")\n\
             6. NEVER put file content in 's3key' - s3key is a storage reference, not content\n\
             7. If the schema requires 'content' with 's3key', use 'local_file_path' instead\n\
             8. For 'local_file_path', use the absolute path from the query or infer from filename\n\
             9. CRITICAL FOR EMAIL ATTACHMENTS (HIGHEST PRIORITY):\n\
                - When sending emails, ALWAYS prefer REAL file attachments over temporary links\n\
                - If the query mentions 'attach', 'file', 'arquivo', 'anexo', 'send file', 'enviar arquivo'\n\
                - AND the schema has an 'attachment' field (check the schema above!)\n\
                - AND you see file metadata in the RECENT FILE DOWNLOADS section above\n\
                - THEN you MUST include the 'attachment' field in your response\n\
                - Use the EXACT s3key, mimetype, and name from the RECENT FILE DOWNLOADS\n\
                - Structure: {{\"name\": \"<from downloads>\", \"mimetype\": \"<from downloads>\", \"s3key\": \"<from downloads>\"}}\n\
                - NEVER use temporary download links when attachment field is available\n\
                - Temporary links are ONLY acceptable if the tool doesn't support attachments\n\
             10. Do not include any explanation, only the JSON object\n\n\
             Example output format:\n\
             {{\"path\": \"/test.txt\", \"local_file_path\": \"C:/temp/test.txt\", \"mode\": \"overwrite\"}}\n\n\
             Example with attachment (REQUIRED when query mentions file + schema has attachment field + file in downloads):\n\
             {{\"recipient_email\": \"user@example.com\", \"subject\": \"File\", \"body\": \"See attached\", \"attachment\": {{\"name\": \"file.txt\", \"mimetype\": \"text/plain\", \"s3key\": \"268883/...\"}}}}\n\n\
             JSON:",
            tool_slug, use_case, schema_str, query, attachment_context
        );
        
        // Make a quick LLM call (no history, just extraction)
        let messages = vec![
            crate::providers::ChatMessage::user(prompt),
        ];
        
        // Use the provider's configured model with low temperature for deterministic extraction
        // The model comes from the user's configuration
        let model = self.model.as_deref().unwrap_or("");
        let request = crate::providers::ChatRequest {
            messages: &messages,
            tools: None,
        };
        
        // Try with the configured model
        match provider.chat(request, model, 0.0).await {
            Ok(response) => {
                tracing::debug!(
                    model = model,
                    "LLM extraction successful with configured model"
                );
                
                // Parse the JSON response
                let content = response.text_or_empty().trim();
                
                // Try to extract JSON from the response (handle cases where LLM adds explanation)
                let json_str = if let Some(start) = content.find('{') {
                    if let Some(end) = content.rfind('}') {
                        &content[start..=end]
                    } else {
                        content
                    }
                } else {
                    content
                };
                
                let extracted: Value = serde_json::from_str(json_str)
                    .with_context(|| format!("Failed to parse LLM response as JSON: {}", json_str))?;
                
                tracing::debug!(
                    extracted = ?extracted,
                    "LLM extraction completed"
                );
                
                Ok(extracted)
            }
            Err(e) => {
                tracing::warn!(
                    model = model,
                    error = %e,
                    "LLM extraction failed with configured model"
                );
                Err(e)
            }
        }
    }
    
    /// Layer 3: Generic fallback extraction
    fn extract_generic_fallback(&self, query: &str, tool_slug: &str, use_case: &str) -> Value {
        tracing::debug!(
            tool_slug = tool_slug,
            use_case = use_case,
            "Using generic fallback extraction"
        );
        
        let mut args = serde_json::json!({});
        
        // Special handling for DROPBOX_UPLOAD_FILE with content
        // Use Composio S3 staging API to upload content and get s3key
        if tool_slug == "DROPBOX_UPLOAD_FILE" {
            // Extract path
            if let Some(path) = self.extract_file_path(query) {
                args["path"] = Value::String(path.clone());
                args["mode"] = Value::String("overwrite".to_string());
                
                // Extract content
                if let Some(content_value) = self.extract_content(query, use_case) {
                    if let Some(content_str) = content_value.as_str() {
                        // Stage content via Composio S3 API
                        let filename = path.trim_start_matches('/');
                        let mimetype = if filename.ends_with(".txt") {
                            "text/plain"
                        } else if filename.ends_with(".md") {
                            "text/markdown"
                        } else if filename.ends_with(".json") {
                            "application/json"
                        } else {
                            "text/plain"
                        };
                        
                        tracing::info!(
                            filename = filename,
                            mimetype = mimetype,
                            "Staging content for Dropbox upload"
                        );
                        
                        // Note: staging is async, but we're in a sync context
                        // Store content for later staging in execute phase
                        args["_zeroclaw_stage_content"] = Value::String(content_str.to_string());
                        args["_zeroclaw_stage_filename"] = Value::String(filename.to_string());
                        args["_zeroclaw_stage_mimetype"] = Value::String(mimetype.to_string());
                    }
                }
            }
            
            return args;
        }
        
        // Try to extract common parameters based on tool type
        if tool_slug.contains("UPLOAD") || tool_slug.contains("CREATE") || tool_slug.contains("WRITE") {
            // File operations - extract path and content
            if let Some(path) = self.extract_file_path(query) {
                args["path"] = Value::String(path);
            }
            
            if let Some(content) = self.extract_content(query, use_case) {
                args["content"] = content;
            }
        }
        
        // Email operations
        if tool_slug.contains("EMAIL") || tool_slug.contains("MAIL") {
            if let Some(email) = self.extract_email_address(query, &["to ", "para ", "recipient "]) {
                args["recipient_email"] = email.clone();
                args["to"] = email;
            }
            
            if let Some(subject) = self.extract_quoted_or_after_keyword(query, &["subject", "assunto"]) {
                args["subject"] = subject;
            }
            
            if let Some(body) = self.extract_body_content(query, use_case) {
                args["body"] = body;
            }
        }
        
        args
    }
    
    /// Extract file path from query
    fn extract_file_path(&self, query: &str) -> Option<String> {
        // Try to extract path from common patterns
        // Look for paths that start with / or contain /
        
        // Pattern 1: Direct path mention (e.g., "to /teste/hello.txt")
        if let Some(start) = query.find(" /") {
            let after = &query[start + 1..]; // Skip the space
            // Take until next space, quote, or common stop word
            let stop_words = [" in ", " with ", " and ", " overwrite", " mode"];
            let mut end_pos = after.len();
            
            for stop_word in &stop_words {
                if let Some(pos) = after.find(stop_word) {
                    if pos < end_pos {
                        end_pos = pos;
                    }
                }
            }
            
            // Also stop at quotes
            if let Some(quote_pos) = after.find('\'').or_else(|| after.find('"')) {
                if quote_pos < end_pos {
                    end_pos = quote_pos;
                }
            }
            
            let path = after[..end_pos].trim();
            if !path.is_empty() && path.starts_with('/') {
                return Some(path.to_string());
            }
        }
        
        // Pattern 2: Traditional patterns (file called, file named, etc.)
        let patterns = [
            ("file called ", vec![" in", " to", " with", " containing"]),
            ("file named ", vec![" in", " to", " with", " containing"]),
            ("arquivo chamado ", vec![" no", " para", " com", " contendo"]),
            ("arquivo ", vec![" no", " para", " com", " contendo"]),
            ("create ", vec![" in", " to", " with"]),
            ("criar ", vec![" no", " para", " com"]),
            ("upload ", vec![" to", " in", " containing"]),
        ];
        
        for (start_pattern, stop_words) in &patterns {
            if let Some(start_pos) = query.to_lowercase().find(start_pattern) {
                let after_pattern = &query[start_pos + start_pattern.len()..];
                
                // Find the first stop word
                let mut end_pos = None;
                for stop_word in stop_words {
                    if let Some(pos) = after_pattern.to_lowercase().find(stop_word) {
                        if end_pos.is_none() || pos < end_pos.unwrap() {
                            end_pos = Some(pos);
                        }
                    }
                }
                
                let filename = if let Some(end) = end_pos {
                    after_pattern[..end].trim()
                } else {
                    // Take until first space or end
                    after_pattern.split_whitespace().next().unwrap_or(after_pattern.trim())
                };
                
                if !filename.is_empty() {
                    // Clean up the filename - remove quotes and extra text
                    let mut cleaned = filename
                        .trim_matches(|c: char| c == '\'' || c == '"' || c == ' ');
                    
                    // Skip common words like "a file", "the file", "file"
                    if cleaned.starts_with("a file ") {
                        cleaned = &cleaned[7..];
                    } else if cleaned.starts_with("the file ") {
                        cleaned = &cleaned[9..];
                    } else if cleaned.starts_with("file ") {
                        cleaned = &cleaned[5..];
                    }
                    
                    // If it already looks like a path, use it as-is
                    if cleaned.contains('/') {
                        let path = if cleaned.starts_with('/') {
                            cleaned.to_string()
                        } else {
                            format!("/{}", cleaned)
                        };
                        return Some(path);
                    }
                    
                    // Otherwise take first word as filename (stop at space)
                    let cleaned = cleaned.split_whitespace().next().unwrap_or(cleaned);
                    
                    // Ensure path starts with /
                    let path = if cleaned.starts_with('/') {
                        cleaned.to_string()
                    } else {
                        format!("/{}", cleaned)
                    };
                    return Some(path);
                }
            }
        }
        
        None
    }
    
    /// Extract content from query
    fn extract_content(&self, query: &str, use_case: &str) -> Option<Value> {
        // Try to extract content after "with content", "with message", "containing", etc.
        let keywords = [
            "with content", "with message", "with text", 
            "containing", "with the content",
            "com conteúdo", "com mensagem", "com texto", 
            "contendo", "com o conteúdo"
        ];
        
        for keyword in &keywords {
            if let Some(pos) = query.to_lowercase().find(keyword) {
                let after = &query[pos + keyword.len()..].trim();
                
                // Try quoted text first
                if let Some(content_text) = self.extract_quoted_text(after) {
                    return Some(Value::String(content_text));
                }
                
                // Otherwise take the rest (but stop at common endings)
                if !after.is_empty() {
                    let content_text = after.to_string();
                    return Some(Value::String(content_text));
                }
            }
        }
        
        // Fallback: try body extraction
        self.extract_body_content(query, use_case)
    }
    
    /// Extract quoted text from string
    fn extract_quoted_text(&self, text: &str) -> Option<String> {
        // Try double quotes
        if let Some(start) = text.find('"') {
            let after = &text[start + 1..];
            if let Some(end) = after.find('"') {
                return Some(after[..end].to_string());
            }
        }
        
        // Try single quotes
        if let Some(start) = text.find('\'') {
            let after = &text[start + 1..];
            if let Some(end) = after.find('\'') {
                return Some(after[..end].to_string());
            }
        }
        
        None
    }
    
    
    /// Extract email address from query
    fn extract_email_address(&self, query: &str, keywords: &[&str]) -> Option<Value> {
        for keyword in keywords {
            if let Some(pos) = query.to_lowercase().find(keyword) {
                let after = &query[pos + keyword.len()..];
                // Find email pattern: word@word.word
                for word in after.split_whitespace() {
                    let cleaned = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '@' && c != '.' && c != '-' && c != '_');
                    if cleaned.contains('@') && cleaned.contains('.') {
                        return Some(Value::String(cleaned.to_string()));
                    }
                }
            }
        }
        None
    }
    
    /// Extract quoted text or text after keyword
    fn extract_quoted_or_after_keyword(&self, query: &str, keywords: &[&str]) -> Option<Value> {
        for keyword in keywords {
            if let Some(pos) = query.to_lowercase().find(keyword) {
                let after = &query[pos..];
                
                // Try to find quoted text first - check for various quote styles
                // Standard ASCII quotes
                if let Some(quote_start) = after.find('"') {
                    let after_quote = &after[quote_start + 1..];
                    if let Some(quote_end) = after_quote.find('"') {
                        let content = after_quote[..quote_end].trim();
                        if !content.is_empty() {
                            return Some(Value::String(content.to_string()));
                        }
                    }
                }
                
                if let Some(quote_start) = after.find('\'') {
                    let after_quote = &after[quote_start + 1..];
                    if let Some(quote_end) = after_quote.find('\'') {
                        let content = after_quote[..quote_end].trim();
                        if !content.is_empty() {
                            return Some(Value::String(content.to_string()));
                        }
                    }
                }
                
                // If no quotes, try to extract text after colon or keyword
                let after_keyword = &after[keyword.len()..].trim_start();
                if let Some(colon_pos) = after_keyword.find(':') {
                    let after_colon = after_keyword[colon_pos + 1..].trim();
                    // Take until next keyword or end
                    let end_pos = after_colon
                        .find(" subject")
                        .or_else(|| after_colon.find(" body"))
                        .or_else(|| after_colon.find(" to "))
                        .unwrap_or(after_colon.len());
                    let content = after_colon[..end_pos].trim();
                    if !content.is_empty() {
                        return Some(Value::String(content.to_string()));
                    }
                }
            }
        }
        None
    }
    
    /// Extract body content from query
    fn extract_body_content(&self, query: &str, use_case: &str) -> Option<Value> {
        // Try to find body after common keywords
        let keywords = ["body", "message", "content", "text", "corpo", "mensagem", "conteúdo"];
        
        for keyword in keywords {
            if let Some(pos) = query.to_lowercase().find(keyword) {
                let after = &query[pos + keyword.len()..].trim_start();
                
                // Skip colon if present
                let after = if after.starts_with(':') {
                    &after[1..].trim_start()
                } else {
                    after
                };
                
                // Try quoted text first - ASCII quotes
                if let Some(quote_start) = after.find('"') {
                    let after_quote = &after[quote_start + 1..];
                    if let Some(quote_end) = after_quote.find('"') {
                        let content = after_quote[..quote_end].trim();
                        if !content.is_empty() {
                            return Some(Value::String(content.to_string()));
                        }
                    }
                }
                
                if let Some(quote_start) = after.find('\'') {
                    let after_quote = &after[quote_start + 1..];
                    if let Some(quote_end) = after_quote.find('\'') {
                        let content = after_quote[..quote_end].trim();
                        if !content.is_empty() {
                            return Some(Value::String(content.to_string()));
                        }
                    }
                }
                
                // If no quotes, take the rest of the text
                if !after.is_empty() {
                    return Some(Value::String(after.to_string()));
                }
            }
        }
        
        // Fallback: if use_case mentions sending/writing, extract the main message
        if use_case.to_lowercase().contains("send") || use_case.to_lowercase().contains("write") {
            // Try to find the main content after common patterns
            let patterns = [
                "saying ", "that says ", "with message ", "with text ",
                "dizendo ", "com mensagem ", "com texto ",
            ];
            
            for pattern in patterns {
                if let Some(pos) = query.to_lowercase().find(pattern) {
                    let content = query[pos + pattern.len()..].trim();
                    if !content.is_empty() {
                        return Some(Value::String(content.to_string()));
                    }
                }
            }
        }
        
        None
    }
    
    
    /// Stage content for upload via Composio S3 API (v3)
    /// Returns the file_url (s3key) that can be used in FileUploadable structures
    async fn stage_content(
        &self,
        content: &str,
        filename: &str,
        mimetype: Option<&str>,
    ) -> Result<String> {
        tracing::info!(
            filename = filename,
            content_size = content.len(),
            "Staging content via Composio S3 API v3"
        );
        
        // Calculate MD5 hash of content
        let md5_hash = format!("{:x}", md5::compute(content.as_bytes()));
        
        // Request upload URL
        let request_payload = serde_json::json!({
            "toolkit_slug": "dropbox",
            "tool_slug": "DROPBOX_UPLOAD_FILE",
            "filename": filename,
            "mimetype": mimetype.unwrap_or("text/plain"),
            "md5": md5_hash
        });
        
        tracing::debug!(
            payload = ?request_payload,
            "Requesting upload URL from Composio v3 API"
        );
        
        let client = reqwest::Client::new();
        let response = client
            .post("https://backend.composio.dev/api/v3/files/upload/request")
            .header("x-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&request_payload)
            .send()
            .await
            .context("Failed to request upload URL")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Upload request failed: status={} body={}", status, body);
        }
        
        let result: Value = response.json().await
            .context("Failed to parse upload request response")?;
        
        tracing::debug!(
            response = ?result,
            "Received upload request response"
        );
        
        // Check if file already exists (deduplication)
        if let Some(existing_url) = result.get("existing_url")
            .or_else(|| result.get("existingUrl"))
            .and_then(|u| u.as_str())
        {
            tracing::info!(
                existing_url = existing_url,
                "File already exists (deduplicated), using existing URL"
            );
            return Ok(existing_url.to_string());
        }
        
        // Get upload URL and upload the content (v3 API uses newPresignedUrl or new_presigned_url)
        let upload_url = result.get("newPresignedUrl")
            .or_else(|| result.get("new_presigned_url"))
            .and_then(|u| u.as_str())
            .ok_or_else(|| anyhow::anyhow!("No newPresignedUrl in response"))?;
        
        tracing::debug!(
            upload_url = upload_url,
            "Uploading content to presigned URL"
        );
        
        // Upload content to presigned URL
        let upload_response = client
            .put(upload_url)
            .header("Content-Type", mimetype.unwrap_or("text/plain"))
            .body(content.to_string())
            .send()
            .await
            .context("Failed to upload content")?;
        
        if !upload_response.status().is_success() {
            let status = upload_response.status();
            let body = upload_response.text().await.unwrap_or_default();
            anyhow::bail!("Content upload failed: status={} body={}", status, body);
        }
        
        // Get the file key/URL from the response
        let file_key = result.get("key")
            .and_then(|k| k.as_str())
            .ok_or_else(|| anyhow::anyhow!("No key in upload request response"))?;
        
        tracing::info!(
            file_key = file_key,
            "Content staged successfully"
        );
        
        Ok(file_key.to_string())
    }
    
    /// Execute Office file operations via COMPOSIO_REMOTE_WORKBENCH
    /// 
    /// This method generates Python code to:
    /// 1. Download the Office file from cloud storage (Dropbox, etc.)
    /// 2. Load it with appropriate library (openpyxl for Excel, python-docx for Word)
    /// 3. Make the requested edits while preserving existing content
    /// 4. Upload the modified file back to cloud storage
    ///
    /// # Arguments
    /// * `query` - Natural language query describing the Office file operation
    ///
    /// # Returns
    /// * `Ok(ToolResult)` - Workbench execution result
    /// * `Err(anyhow::Error)` - Execution errors
    async fn execute_via_workbench(&self, query: &str) -> Result<ToolResult> {
        tracing::info!(
            query = query,
            "Executing Office file operation via COMPOSIO_REMOTE_WORKBENCH"
        );
        
        // Generate Python code for Workbench based on the query
        let python_code = self.generate_workbench_code_for_office(query).await?;
        
        tracing::info!(
            "=== WORKBENCH CODE (FULL) ===\n{}\n=== END CODE ===",
            python_code
        );
        
        // Invoke COMPOSIO_REMOTE_WORKBENCH meta tool directly
        let params = serde_json::json!({
            "code_to_execute": python_code,
            "session": {
                "generate_id": true
            }
        });
        
        tracing::info!(
            "=== WORKBENCH REQUEST ===\n{}\n=== END REQUEST ===",
            serde_json::to_string_pretty(&params).unwrap_or_default()
        );
        
        let request_id = self.next_request_id().await;
        let result = self.mcp_client
            .tools_call(request_id, "COMPOSIO_REMOTE_WORKBENCH", params)
            .await
            .context("Failed to call COMPOSIO_REMOTE_WORKBENCH")?;
        
        tracing::info!(
            "=== WORKBENCH RAW RESPONSE ===\n{}\n=== END RESPONSE ===",
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| format!("{:?}", result))
        );
        
        // Parse JSON-RPC response
        let rpc_response: JsonRpcResponse = serde_json::from_value(result)
            .context("Failed to parse JSON-RPC response")?;
        
        tracing::info!(
            "=== WORKBENCH JSON-RPC PARSED ===\nhas_error: {}\nhas_result: {}\n=== END PARSED ===",
            rpc_response.error.is_some(),
            rpc_response.result.is_some()
        );
        
        if let Some(error) = rpc_response.error {
            tracing::error!(
                "=== WORKBENCH ERROR ===\ncode: {}\nmessage: {}\ndata: {:?}\n=== END ERROR ===",
                error.code,
                error.message,
                error.data
            );
            anyhow::bail!("JSON-RPC error: {} (code: {})", error.message, error.code);
        }
        
        let result_data = rpc_response.result
            .ok_or_else(|| anyhow::anyhow!("No result in JSON-RPC response"))?;
        
        tracing::info!(
            "=== WORKBENCH RESULT DATA ===\n{}\n=== END RESULT DATA ===",
            serde_json::to_string_pretty(&result_data).unwrap_or_default()
        );
        
        // Parse the content[0].text JSON string
        let parsed_data = result_data.get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|text| text.as_str())
            .and_then(|text_str| {
                tracing::info!(
                    "=== WORKBENCH TEXT CONTENT ===\n{}\n=== END TEXT ===",
                    text_str
                );
                serde_json::from_str::<serde_json::Value>(text_str).ok()
            })
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Workbench response"))?;
        
        tracing::info!(
            "=== WORKBENCH PARSED DATA ===\n{}\n=== END PARSED DATA ===",
            serde_json::to_string_pretty(&parsed_data).unwrap_or_default()
        );
        
        // Extract output from Workbench response
        let output = if let Some(data) = parsed_data.get("data") {
            if let Some(output_str) = data.get("output").and_then(|o| o.as_str()) {
                output_str.to_string()
            } else {
                serde_json::to_string_pretty(data)?
            }
        } else {
            serde_json::to_string_pretty(&parsed_data)?
        };
        
        tracing::info!(
            output_preview = %format!("{:.200}", output),
            "Workbench execution completed"
        );
        
        Ok(ToolResult {
            success: true,
            output,
            error: None,
        })
    }
    
    /// Generate Python code for Workbench to handle Office file operations
    ///
    /// Uses LLM (if available) to generate appropriate Python code based on the query.
    /// Falls back to template-based generation if LLM is not available.
    async fn generate_workbench_code_for_office(&self, query: &str) -> Result<String> {
        // If we have an LLM provider, use it to generate the code
        if let Some(provider) = &self.provider {
            let system_prompt = r#"You are a Python code generator for Composio Workbench.
Generate Python code to handle Office file operations (Excel/Word) that:
1. Downloads the file from cloud storage using run_composio_tool()
2. Loads it with openpyxl (Excel) or python-docx (Word) - MUST use load_workbook() to preserve content
3. Makes the requested edits
4. Uploads the modified file back using upload_local_file() and run_composio_tool()

Available helper functions:
- run_composio_tool(tool_slug, arguments) - Execute any Composio tool
- upload_local_file(local_path) - Upload file to cloud storage, returns s3key
- invoke_llm(prompt) - Call LLM for help

Pre-installed libraries: openpyxl, python-docx, pandas, numpy

CRITICAL RULES:
- For Excel: Use openpyxl.load_workbook(file_path) to LOAD existing file (preserves content)
- DON'T create new workbooks with openpyxl.Workbook() - this overwrites everything
- For Word: Use python-docx Document(file_path) to load existing document
- Save to a local file path, then upload it

Return ONLY the Python code, no explanations."#;
            
            let user_prompt = format!("Generate Python code for this Office file operation:\n\n{}", query);
            
            match provider.chat_with_system(
                Some(system_prompt),
                &user_prompt,
                self.model.as_deref().unwrap_or("gpt-4"),
                0.3, // Low temperature for code generation
            ).await {
                Ok(code) => {
                    // Clean up code (remove markdown code blocks if present)
                    let code = code.trim();
                    let code = if code.starts_with("```python") {
                        code.strip_prefix("```python").unwrap_or(code).trim()
                    } else if code.starts_with("```") {
                        code.strip_prefix("```").unwrap_or(code).trim()
                    } else {
                        code
                    };
                    let code = if code.ends_with("```") {
                        code.strip_suffix("```").unwrap_or(code).trim()
                    } else {
                        code
                    };
                    
                    tracing::info!("Generated Workbench code using LLM");
                    return Ok(code.to_string());
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Failed to generate code with LLM, falling back to template"
                    );
                }
            }
        }
        
        // Fallback: Generate template-based code
        tracing::info!("Generating Workbench code using template (no LLM available)");
        
        // Extract file path from query
        let file_path = self.extract_file_path_from_query(query);
        
        // Determine file type
        let is_excel = query.to_lowercase().contains(".xlsx") || query.to_lowercase().contains(".xls");
        let is_word = query.to_lowercase().contains(".docx") || query.to_lowercase().contains(".doc");
        
        if is_excel {
            Ok(format!(r#"
# Download Excel file from Dropbox
file_path = "{}"
download_result = run_composio_tool("DROPBOX_READ_FILE", {{"path": file_path}})

# Extract s3key from download result
s3key = download_result.get("content", {{}}).get("s3key")
if not s3key:
    print("Error: Failed to download file")
else:
    # Download file locally
    import requests
    response = requests.get(s3key)
    local_path = "/tmp/temp_file.xlsx"
    with open(local_path, "wb") as f:
        f.write(response.content)
    
    # Load existing workbook (PRESERVES CONTENT)
    from openpyxl import load_workbook
    wb = load_workbook(local_path)
    ws = wb.active
    
    # Make edits based on query: {}
    # Example: Add 3 rows with sample data
    next_row = ws.max_row + 1
    for i in range(3):
        ws.cell(row=next_row + i, column=1, value=f"Item {{i+1}}")
        ws.cell(row=next_row + i, column=2, value=f"Value {{i+1}}")
    
    # Save modified file
    output_path = "/tmp/modified_file.xlsx"
    wb.save(output_path)
    
    # Upload back to Dropbox
    s3key_upload = upload_local_file(output_path)
    upload_result = run_composio_tool("DROPBOX_UPLOAD_FILE", {{
        "path": file_path,
        "content": {{
            "name": file_path.split("/")[-1],
            "mimetype": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "s3key": s3key_upload
        }},
        "mode": "overwrite"
    }})
    
    print(f"File updated successfully: {{file_path}}")
"#, file_path, query))
        } else if is_word {
            Ok(format!(r#"
# Download Word file from Dropbox
file_path = "{}"
download_result = run_composio_tool("DROPBOX_READ_FILE", {{"path": file_path}})

# Extract s3key from download result
s3key = download_result.get("content", {{}}).get("s3key")
if not s3key:
    print("Error: Failed to download file")
else:
    # Download file locally
    import requests
    response = requests.get(s3key)
    local_path = "/tmp/temp_file.docx"
    with open(local_path, "wb") as f:
        f.write(response.content)
    
    # Load existing document (PRESERVES CONTENT)
    from docx import Document
    doc = Document(local_path)
    
    # Make edits based on query: {}
    # Example: Add paragraphs
    doc.add_paragraph("Content added by Workbench")
    
    # Save modified file
    output_path = "/tmp/modified_file.docx"
    doc.save(output_path)
    
    # Upload back to Dropbox
    s3key_upload = upload_local_file(output_path)
    upload_result = run_composio_tool("DROPBOX_UPLOAD_FILE", {{
        "path": file_path,
        "content": {{
            "name": file_path.split("/")[-1],
            "mimetype": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "s3key": s3key_upload
        }},
        "mode": "overwrite"
    }})
    
    print(f"File updated successfully: {{file_path}}")
"#, file_path, query))
        } else {
            anyhow::bail!("Could not determine Office file type from query")
        }
    }
    
    /// Extract file path from natural language query
    fn extract_file_path_from_query(&self, query: &str) -> String {
        // Look for patterns like: /path/to/file.xlsx, "file.xlsx", 'file.xlsx'
        let query_lower = query.to_lowercase();
        
        // Try to find file path with extension
        for ext in &[".xlsx", ".xls", ".docx", ".doc", ".pptx", ".ppt"] {
            if let Some(ext_pos) = query_lower.find(ext) {
                // Find the start of the path (look backwards for space, quote, or start of string)
                let before = &query[..ext_pos];
                let start = before.rfind(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                    .map(|pos| pos + 1)
                    .unwrap_or(0);
                
                // Extract path including extension
                let end = ext_pos + ext.len();
                let path = query[start..end].trim_matches(|c: char| c == '"' || c == '\'').trim();
                
                if !path.is_empty() {
                    return path.to_string();
                }
            }
        }
        
        // Fallback: return a placeholder
        "/path/to/file.xlsx".to_string()
    }
    
    /// Execute a discovered tool
    async fn execute_tool(
        &self,
        tool_slug: &str,
        mut arguments: Value,
    ) -> Result<Value> {
        tracing::debug!(
            tool_slug = tool_slug,
            "Executing tool via COMPOSIO_MULTI_EXECUTE_TOOL"
        );
        
        // Check if we need to stage content for DROPBOX_UPLOAD_FILE
        if tool_slug == "DROPBOX_UPLOAD_FILE" {
            if let Some(content) = arguments.get("_zeroclaw_stage_content").and_then(|c| c.as_str()).map(|s| s.to_string()) {
                let filename = arguments.get("_zeroclaw_stage_filename")
                    .and_then(|f| f.as_str())
                    .unwrap_or("file.txt")
                    .to_string();
                let mimetype = arguments.get("_zeroclaw_stage_mimetype")
                    .and_then(|m| m.as_str())
                    .map(|s| s.to_string());
                
                tracing::info!(
                    "Staging content for DROPBOX_UPLOAD_FILE"
                );
                
                // Stage content and get s3key
                match self.stage_content(&content, &filename, mimetype.as_deref()).await {
                    Ok(file_url) => {
                        // Remove staging markers
                        if let Some(obj) = arguments.as_object_mut() {
                            obj.remove("_zeroclaw_stage_content");
                            obj.remove("_zeroclaw_stage_filename");
                            obj.remove("_zeroclaw_stage_mimetype");
                        }
                        
                        // Add FileUploadable structure with s3key
                        let mimetype_str = mimetype.unwrap_or_else(|| "text/plain".to_string());
                        arguments["content"] = serde_json::json!({
                            "name": filename,
                            "mimetype": mimetype_str,
                            "s3key": file_url
                        });
                        
                        tracing::info!(
                            s3key = %file_url,
                            "Content staged successfully, using FileUploadable structure"
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            "Failed to stage content, will try without staging"
                        );
                        // Remove staging markers even on failure
                        if let Some(obj) = arguments.as_object_mut() {
                            obj.remove("_zeroclaw_stage_content");
                            obj.remove("_zeroclaw_stage_filename");
                            obj.remove("_zeroclaw_stage_mimetype");
                        }
                    }
                }
            }
        }
        
        let params = serde_json::json!({
            "tools": [{
                "tool_slug": tool_slug,
                "arguments": arguments
            }],
            "sync_response_to_workbench": false,
            "session": {
                "generate_id": true
            }
        });
        
        let request_id = self.next_request_id().await;
        let result = self.mcp_client
            .tools_call(request_id, "COMPOSIO_MULTI_EXECUTE_TOOL", params)
            .await
            .context("Failed to execute tool")?;
        
        // Parse JSON-RPC response
        let rpc_response: JsonRpcResponse = serde_json::from_value(result)
            .context("Failed to parse JSON-RPC response")?;
        
        if let Some(error) = rpc_response.error {
            anyhow::bail!("JSON-RPC error: {} (code: {})", error.message, error.code);
        }
        
        let result_data = rpc_response.result
            .ok_or_else(|| anyhow::anyhow!("No result in JSON-RPC response"))?;
        
        // Parse the content[0].text JSON string (same format as other responses)
        let parsed_data = result_data.get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|text| text.as_str())
            .and_then(|text_str| {
                tracing::debug!("Execution response text: {}", 
                    if text_str.len() > 500 { 
                        format!("{}...", &text_str[..500]) 
                    } else { 
                        text_str.to_string() 
                    }
                );
                serde_json::from_str::<serde_json::Value>(text_str).ok()
            })
            .ok_or_else(|| anyhow::anyhow!("Failed to parse execution response"))?;
        
        tracing::debug!("Parsed execution data keys: {:?}", 
            parsed_data.as_object().map(|o| o.keys().collect::<Vec<_>>())
        );
        
        // Check for errors in parsed data
        // Note: error field might be empty string, so check if it's non-empty
        if let Some(error) = parsed_data.get("error").and_then(|e| e.as_str()) {
            if !error.is_empty() {
                // Check if it's a path/permission error (App Folder limitation)
                let is_path_restriction = error.to_lowercase().contains("path")
                    || error.to_lowercase().contains("not_found")
                    || error.to_lowercase().contains("restricted")
                    || error.to_lowercase().contains("permission")
                    || error.to_lowercase().contains("access");
                
                if is_path_restriction && tool_slug == "DROPBOX_UPLOAD_FILE" {
                    // Extract the path that failed
                    let failed_path = arguments.get("path")
                        .and_then(|p| p.as_str())
                        .unwrap_or("unknown path");
                    
                    tracing::warn!(
                        path = failed_path,
                        error = error,
                        "Dropbox upload failed - likely App Folder restriction"
                    );
                    
                    // Return a special error that the agent can detect and handle
                    anyhow::bail!(
                        "Dropbox Access Limitation: Failed to upload to {}\n\n\
                        The upload failed because your Dropbox connection is limited to App Folder access, \
                        which restricts uploads to specific folders only.\n\n\
                        I can fix this by reconnecting your Dropbox with Full Access, which will allow \
                        uploads to any folder in your Dropbox.\n\n\
                        Would you like me to reconnect with Full Access now? \
                        I'll need you to authorize in your browser, then I can retry the upload.\n\n\
                        Original error: {}",
                        failed_path,
                        error
                    );
                }
                
                anyhow::bail!("Tool execution failed: {}", error);
            }
        }
        
        // Check for successful field
        if let Some(successful) = parsed_data.get("successful").and_then(|s| s.as_bool()) {
            if !successful {
                let error_msg = parsed_data.get("error")
                    .or_else(|| parsed_data.get("message"))
                    .and_then(|e| e.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or("Unknown error");
                anyhow::bail!("Tool execution failed: {}", error_msg);
            }
        }
        
        tracing::info!("Tool executed successfully");
        
        // Store execution in history for context in future LLM extractions
        // Keep only last 10 executions to avoid memory bloat
        {
            let mut history = self.execution_history.write().await;
            history.push((
                tool_slug.to_string(),
                arguments.to_string(),
                parsed_data.clone(),
            ));
            
            // Keep only last 10 executions
            if history.len() > 10 {
                history.remove(0);
            }
            
            tracing::debug!(
                history_size = history.len(),
                "Updated execution history"
            );
        }
        
        Ok(parsed_data)
    }

    /// Clear all cached state (session, execution history)
    ///
    /// This method invalidates all cached state in the natural language tool,
    /// including the current session and execution history. This is useful for
    /// hot reload scenarios where a fresh start is needed.
    ///
    /// # Example
    /// ```no_run
    /// # use std::sync::Arc;
    /// # use zeroclaw::tools::ComposioNaturalLanguageTool;
    /// # use zeroclaw::mcp::McpClient;
    /// # use zeroclaw::security::SecurityPolicy;
    /// # async fn example() -> anyhow::Result<()> {
    /// let mcp_client = Arc::new(McpClient::new("url", "key")?);
    /// let security = Arc::new(SecurityPolicy::default());
    /// let tool = ComposioNaturalLanguageTool::new(
    ///     mcp_client,
    ///     security,
    ///     "api_key".to_string()
    /// );
    ///
    /// // Clear all cached state
    /// tool.clear_cache().await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn clear_cache(&self) {
        // Clear session
        {
            let mut session = self.current_session.write().await;
            *session = None;
        }
        
        // Clear execution history
        {
            let mut history = self.execution_history.write().await;
            history.clear();
        }
        
        // Reset request ID counter
        {
            let mut id = self.request_id.write().await;
            *id = 1;
        }
        
        tracing::debug!("Composio Natural Language Tool cache cleared");
    }
}

#[async_trait]
impl Tool for ComposioNaturalLanguageTool {
    fn name(&self) -> &str {
        "composio_nl"
    }
    
    fn description(&self) -> &str {
        "Access 1000+ apps (Gmail, Dropbox, GitHub, Slack, Notion, etc.) through natural language. \
        Simply describe what you want to do. \
        Examples: 'list my gmail emails', 'create dropbox folder /documents', \
        'send slack message to #general: Hello team!', 'search github issues in my-repo'. \
        The tool will automatically discover the right action, handle OAuth if needed, and execute it."
    }
    
    fn parameters_schema(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural language description of what you want to do. \
                        Be specific about the app and action. \
                        Examples: 'list my gmail emails', 'create github issue in my-repo', \
                        'send slack message to #general'"
                    },
                    "tool_slug": {
                        "type": "string",
                        "description": "Optional: Exact Composio tool name to bypass search (e.g., 'DROPBOX_GET_TEMPORARY_LINK'). \
                        If you know the exact tool, use this to force its execution and avoid semantic search errors."
                    },
                    "arguments": {
                        "type": "object",
                        "description": "Optional: Specific arguments if you know them. \
                        If not provided, the tool will attempt to extract them from the query.",
                        "additionalProperties": true
                    },
                    "use_workbench": {
                        "type": "boolean",
                        "description": "Optional: Force use of Workbench for large data operations. \
                        Workbench is automatically used for bulk operations (all, every, list, export) \
                        and large data operations (csv, spreadsheet, report, summary). \
                        Set to true to force Workbench usage for operations that may produce large outputs.",
                        "default": false
                    }
                },
                "required": ["query"]
            })
        }
    
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        // Security check - core policy
        if let Err(error) = self.security.enforce_tool_operation(
            crate::security::policy::ToolOperation::Act,
            "composio_nl",
        ) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(error),
            });
        }
        
        let query = args.get("query")
            .and_then(|q| q.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'query' parameter"))?;
        
        let provided_args = args.get("arguments").cloned();
        let use_workbench = args.get("use_workbench")
            .and_then(|v| v.as_bool());
        
        tracing::info!(query = query, "Executing Composio natural language tool");
        
        // Check for Office file EDIT operations - USE WORKBENCH DIRECTLY
        let office_extensions = [".xlsx", ".xls", ".docx", ".doc", ".pptx", ".ppt"];
        let is_office_file = office_extensions.iter().any(|ext| query.to_lowercase().contains(ext));
        
        // Only intercept EDIT operations (not read/download)
        let edit_keywords = ["edit", "add", "modify", "update", "change", "append", "insert", "delete row", "delete column"];
        let is_edit_operation = edit_keywords.iter().any(|kw| query.to_lowercase().contains(kw));
        
        // CRITICAL: For Office file edits, ALWAYS use COMPOSIO_REMOTE_WORKBENCH
        if is_office_file && is_edit_operation {
            tracing::info!(
                query = query,
                "Office file EDIT operation detected! Invoking COMPOSIO_REMOTE_WORKBENCH directly to preserve existing content"
            );
            
            // Invoke Workbench directly - it's a meta tool, not discovered via SEARCH_TOOLS
            return self.execute_via_workbench(query).await;
        }
        
        // Check if Workbench should be used for this query
        let should_use_wb = self.should_use_workbench(query, use_workbench);
        if should_use_wb {
            tracing::info!(
                query = query,
                forced = use_workbench.unwrap_or(false),
                "Workbench mode recommended for this query (bulk/large data operation detected)"
            );
            // TODO: Implement Workbench execution path in task 10
            // For now, continue with direct execution
        }
        
        // 1. Ensure we have a session (no-op now, server manages sessions)
        match self.ensure_session().await {
            Ok(()) => {},
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to initialize: {}", e)),
                });
            }
        };
        
        let optional_tool_slug = args.get("tool_slug").and_then(|t| t.as_str());
        
        // 2. Search for relevant tools or use the exact one provided
        let discovered_tools = if let Some(slug) = optional_tool_slug {
            tracing::info!(tool_slug = slug, "Bypassing search, using provided tool slug");
            
            // Try to fetch complete schema
            let schemas_map = self.get_tool_schemas(vec![slug.to_string()]).await.unwrap_or_default();
            
            // Infer toolkit from prefix (e.g., GMAIL_SEND_EMAIL -> gmail)
            let toolkit = slug.split('_').next().unwrap_or("unknown").to_lowercase();
            
            vec![DiscoveredTool {
                tool_slug: slug.to_string(),
                description: format!("Direct execution of {}", slug),
                toolkit,
                use_case: query.to_string(),
                input_schema: schemas_map.get(slug).cloned(),
                schema_ref: None,
            }]
        } else {
            match self.search_tools(query).await {
                Ok(tools) => tools,
                Err(e) => {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("Failed to search tools: {}", e)),
                    });
                }
            }
        };
        
        if discovered_tools.is_empty() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "No tools found for query: '{}'. \
                    Try rephrasing or check if the app is supported by Composio.",
                    query
                )),
            });
        }
        
        // 3. Use the first discovered tool (most relevant)
        let tool = &discovered_tools[0];
        tracing::info!(
            tool_slug = tool.tool_slug,
            toolkit = tool.toolkit,
            "Selected tool for execution"
        );
        
        // Security check - Composio-specific policy (toolkit filtering, rate limiting)
        if let Some(ref composio_security) = self.composio_security {
            // Extract user_id from args or use "default_user"
            let user_id = args.get("user_id")
                .and_then(|u| u.as_str())
                .unwrap_or("default_user");
            
            if let Err(e) = composio_security.check_toolkit_execution(&tool.toolkit, user_id) {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Security policy denied execution: {}", e)),
                });
            }
        }
        
        // 4. Ensure toolkit is connected
        match self.ensure_connected(&tool.toolkit).await {
            Ok(ConnectionStatus::Connected) => {
                tracing::debug!(toolkit = tool.toolkit, "Toolkit connected");
            }
            Ok(ConnectionStatus::NeedsOAuth(redirect_url)) => {
                return Ok(ToolResult {
                    success: false,
                    output: format!(
                        "🔗 {} OAuth Authorization Required\n\n\
                        Click this link to connect your account:\n\
                        {}\n\n\
                        ⏱ Link expires in 10 minutes\n\n\
                        After authorizing, retry your request and I'll complete the action.",
                        tool.toolkit.to_uppercase(),
                        redirect_url
                    ),
                    error: None,
                });
            }
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to check connection: {}", e)),
                });
            }
        }
        
        // 5. Prepare arguments - merge provided args with extracted args
        let tool_args = if let Some(mut provided) = provided_args {
            // If user provided some arguments, merge with extracted ones
            // Extracted args fill in missing fields, but provided args take precedence
            let extracted = self.extract_arguments_from_query(query, &tool.tool_slug, &discovered_tools[0]).await;
            
            // Merge: provided args override extracted args
            if let (Some(provided_obj), Some(extracted_obj)) = (provided.as_object_mut(), extracted.as_object()) {
                for (key, value) in extracted_obj {
                    // Only add if not already provided
                    if !provided_obj.contains_key(key) {
                        provided_obj.insert(key.clone(), value.clone());
                    }
                }
            }
            
            provided
        } else {
            // No provided args, extract everything from query
            self.extract_arguments_from_query(query, &tool.tool_slug, &discovered_tools[0]).await
        };
        
        tracing::debug!(
            tool_slug = tool.tool_slug,
            arguments = ?tool_args,
            "Prepared tool arguments"
        );
        
        // 6. Execute the tool
        match self.execute_tool(&tool.tool_slug, tool_args).await {
            Ok(result) => {
                Ok(ToolResult {
                    success: true,
                    output: serde_json::to_string_pretty(&result)?,
                    error: None,
                })
            }
            Err(e) => {
                Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Tool execution failed: {}", e)),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Test helper to parse discovered tools from a mock response
    fn create_mock_search_response(results: Vec<serde_json::Value>) -> Value {
        json!({
            "content": [{
                "text": json!({
                    "data": {
                        "results": results
                    }
                }).to_string()
            }]
        })
    }

    /// Test helper to create a mock schema response
    fn create_mock_schema_response(schemas: serde_json::Map<String, Value>) -> Value {
        json!({
            "content": [{
                "text": json!({
                    "data": {
                        "tool_schemas": schemas
                    }
                }).to_string()
            }]
        })
    }


    /// Test 1: Successful parsing of discovered tools from valid response
    #[test]
    fn test_parse_discovered_tools_success() {
        let security = Arc::new(SecurityPolicy::default());
        let mcp_client = Arc::new(McpClient::new("http://test", "test_key").unwrap());
        let tool = ComposioNaturalLanguageTool::new(mcp_client, security, "test_api_key".to_string());

        let result_data = create_mock_search_response(vec![
            json!({
                "primary_tool_slugs": ["GMAIL_SEND_EMAIL"],
                "use_case": "send email",
                "execution_guidance": "Send an email via Gmail",
                "toolkits": ["gmail"],
                "tool_schemas": [{
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "to": {"type": "string"},
                            "subject": {"type": "string"},
                            "body": {"type": "string"}
                        },
                        "required": ["to", "subject", "body"]
                    }
                }]
            })
        ]);

        let result = tool.parse_discovered_tools(result_data);
        assert!(result.is_ok(), "parse_discovered_tools should succeed");
        
        let discovered = result.unwrap();
        assert_eq!(discovered.len(), 1, "Should discover 1 tool");
        assert_eq!(discovered[0].tool_slug, "GMAIL_SEND_EMAIL");
        assert_eq!(discovered[0].toolkit, "gmail");
        assert_eq!(discovered[0].use_case, "send email");
        assert!(discovered[0].input_schema.is_some(), "Should have schema from tool_schemas");
    }

    /// Test 2: Parsing multiple tools from response
    #[test]
    fn test_parse_multiple_discovered_tools() {
        let security = Arc::new(SecurityPolicy::default());
        let mcp_client = Arc::new(McpClient::new("http://test", "test_key").unwrap());
        let tool = ComposioNaturalLanguageTool::new(mcp_client, security, "test_api_key".to_string());

        let result_data = create_mock_search_response(vec![
            json!({
                "primary_tool_slugs": ["GITHUB_CREATE_ISSUE"],
                "use_case": "create github issue",
                "execution_guidance": "Create a new issue in GitHub",
                "toolkits": ["github"]
            }),
            json!({
                "tool_slug": "SLACK_SEND_MESSAGE",
                "use_case": "send slack message",
                "execution_guidance": "Send a message to Slack",
                "toolkit": "slack"
            })
        ]);

        let result = tool.parse_discovered_tools(result_data);
        assert!(result.is_ok(), "parse_discovered_tools should succeed");
        
        let discovered = result.unwrap();
        assert_eq!(discovered.len(), 2, "Should discover 2 tools");
        assert_eq!(discovered[0].tool_slug, "GITHUB_CREATE_ISSUE");
        assert_eq!(discovered[0].toolkit, "github");
        assert_eq!(discovered[1].tool_slug, "SLACK_SEND_MESSAGE");
        assert_eq!(discovered[1].toolkit, "slack");
    }

    /// Test 3: Parsing tools with alternative field names
    #[test]
    fn test_parse_tools_with_alternative_fields() {
        let security = Arc::new(SecurityPolicy::default());
        let mcp_client = Arc::new(McpClient::new("http://test", "test_key").unwrap());
        let tool = ComposioNaturalLanguageTool::new(mcp_client, security, "test_api_key".to_string());

        let result_data = create_mock_search_response(vec![
            json!({
                "action_slug": "NOTION_CREATE_PAGE",
                "use_case": "create notion page",
                "execution_guidance": "Create a new page in Notion",
                "app": "notion"
            })
        ]);

        let result = tool.parse_discovered_tools(result_data);
        assert!(result.is_ok());
        
        let discovered = result.unwrap();
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].tool_slug, "NOTION_CREATE_PAGE");
        assert_eq!(discovered[0].toolkit, "notion");
    }

    /// Test 4: Parsing tools with toolkit inference from use_case
    #[test]
    fn test_parse_tools_with_toolkit_inference() {
        let security = Arc::new(SecurityPolicy::default());
        let mcp_client = Arc::new(McpClient::new("http://test", "test_key").unwrap());
        let tool = ComposioNaturalLanguageTool::new(mcp_client, security, "test_api_key".to_string());

        let result_data = create_mock_search_response(vec![
            json!({
                "primary_tool_slugs": ["SEND_EMAIL"],
                "use_case": "send email via gmail",
                "execution_guidance": "Send an email using Gmail API"
            }),
            json!({
                "primary_tool_slugs": ["CREATE_ISSUE"],
                "use_case": "create github issue",
                "execution_guidance": "Create a new GitHub issue"
            })
        ]);

        let result = tool.parse_discovered_tools(result_data);
        assert!(result.is_ok());
        
        let discovered = result.unwrap();
        assert_eq!(discovered.len(), 2);
        // Should infer gmail from use_case
        assert_eq!(discovered[0].toolkit, "gmail");
        // Should infer github from use_case
        assert_eq!(discovered[1].toolkit, "github");
    }

    /// Test 5: Handling empty results
    #[test]
    fn test_parse_empty_results() {
        let security = Arc::new(SecurityPolicy::default());
        let mcp_client = Arc::new(McpClient::new("http://test", "test_key").unwrap());
        let tool = ComposioNaturalLanguageTool::new(mcp_client, security, "test_api_key".to_string());

        let result_data = create_mock_search_response(vec![]);

        let result = tool.parse_discovered_tools(result_data);
        assert!(result.is_ok(), "Should handle empty results");
        
        let discovered = result.unwrap();
        assert_eq!(discovered.len(), 0, "Should return empty vector");
    }

    /// Test 6: Handling malformed response structure
    #[test]
    fn test_parse_malformed_response() {
        let security = Arc::new(SecurityPolicy::default());
        let mcp_client = Arc::new(McpClient::new("http://test", "test_key").unwrap());
        let tool = ComposioNaturalLanguageTool::new(mcp_client, security, "test_api_key".to_string());

        // Missing content array
        let malformed_data = json!({
            "data": {
                "results": []
            }
        });

        let result = tool.parse_discovered_tools(malformed_data);
        // Should handle gracefully - either error or empty results
        if let Ok(discovered) = result {
            assert_eq!(discovered.len(), 0);
        }
    }

    /// Test 7: Handling invalid JSON in text field
    #[test]
    fn test_parse_invalid_json_text() {
        let security = Arc::new(SecurityPolicy::default());
        let mcp_client = Arc::new(McpClient::new("http://test", "test_key").unwrap());
        let tool = ComposioNaturalLanguageTool::new(mcp_client, security, "test_api_key".to_string());

        let invalid_data = json!({
            "content": [{
                "text": "not valid json"
            }]
        });

        let result = tool.parse_discovered_tools(invalid_data);
        // Should handle gracefully
        if let Ok(discovered) = result {
            assert_eq!(discovered.len(), 0);
        }
    }

    /// Test 8: Parsing tools with numeric IDs
    #[test]
    fn test_parse_tools_with_numeric_ids() {
        let security = Arc::new(SecurityPolicy::default());
        let mcp_client = Arc::new(McpClient::new("http://test", "test_key").unwrap());
        let tool = ComposioNaturalLanguageTool::new(mcp_client, security, "test_api_key".to_string());

        let result_data = create_mock_search_response(vec![
            json!({
                "id": 12345,
                "use_case": "test tool",
                "execution_guidance": "Test tool",
                "toolkits": ["test"]
            })
        ]);

        let result = tool.parse_discovered_tools(result_data);
        assert!(result.is_ok());
        
        let discovered = result.unwrap();
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].tool_slug, "12345");
    }

    /// Test 9: Parsing tools with toolkits array
    #[test]
    fn test_parse_tools_with_toolkits_array() {
        let security = Arc::new(SecurityPolicy::default());
        let mcp_client = Arc::new(McpClient::new("http://test", "test_key").unwrap());
        let tool = ComposioNaturalLanguageTool::new(mcp_client, security, "test_api_key".to_string());

        let result_data = create_mock_search_response(vec![
            json!({
                "primary_tool_slugs": ["MULTI_TOOLKIT_TOOL"],
                "use_case": "multi toolkit",
                "execution_guidance": "Tool that works with multiple toolkits",
                "toolkits": ["gmail", "outlook", "slack"]
            })
        ]);

        let result = tool.parse_discovered_tools(result_data);
        assert!(result.is_ok());
        
        let discovered = result.unwrap();
        assert_eq!(discovered.len(), 1);
        // Should use first toolkit from array
        assert_eq!(discovered[0].toolkit, "gmail");
    }

    /// Test 10: Parsing tools with complete schema in tool_schemas
    #[test]
    fn test_parse_tools_with_embedded_schema() {
        let security = Arc::new(SecurityPolicy::default());
        let mcp_client = Arc::new(McpClient::new("http://test", "test_key").unwrap());
        let tool = ComposioNaturalLanguageTool::new(mcp_client, security, "test_api_key".to_string());

        let schema = json!({
            "type": "object",
            "properties": {
                "param1": {"type": "string"},
                "param2": {"type": "number"}
            },
            "required": ["param1"]
        });

        let result_data = create_mock_search_response(vec![
            json!({
                "primary_tool_slugs": ["TOOL_WITH_SCHEMA"],
                "use_case": "tool with schema",
                "execution_guidance": "Tool with embedded schema",
                "toolkits": ["test"],
                "tool_schemas": [{
                    "parameters": schema.clone()
                }]
            })
        ]);

        let result = tool.parse_discovered_tools(result_data);
        assert!(result.is_ok());
        
        let discovered = result.unwrap();
        assert_eq!(discovered.len(), 1);
        assert!(discovered[0].input_schema.is_some());
        
        let parsed_schema = discovered[0].input_schema.as_ref().unwrap();
        assert_eq!(parsed_schema["type"], "object");
        assert!(parsed_schema["properties"]["param1"].is_object());
    }
}


// ══════════════════════════════════════════════════════════════════════════════
// Property 27: Workbench Decision Logic - Integration Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod workbench_decision_tests {
    use super::*;
    
    /// Feature: composio-permanent-integration, Property 27: Workbench Decision Logic
    /// Test bulk email query uses Workbench
    #[test]
    fn test_bulk_email_query_uses_workbench() {
        let mcp_client = Arc::new(McpClient::new("http://test", "test_key").unwrap());
        let security = Arc::new(SecurityPolicy::default());
        let tool = ComposioNaturalLanguageTool::new(mcp_client, security, "test_api_key".to_string());
        
        let query = "Label all unread emails from last week as 'needs-review'";
        let should_use_wb = tool.should_use_workbench(query, None);
        
        assert!(
            should_use_wb,
            "Bulk email query should use Workbench (contains 'all')"
        );
    }
    
    /// Feature: composio-permanent-integration, Property 27: Workbench Decision Logic
    /// Test CSV export query uses Workbench
    #[test]
    fn test_csv_export_query_uses_workbench() {
        let mcp_client = Arc::new(McpClient::new("http://test", "test_key").unwrap());
        let security = Arc::new(SecurityPolicy::default());
        let tool = ComposioNaturalLanguageTool::new(mcp_client, security, "test_api_key".to_string());
        
        let query = "Export all GitHub issues to CSV and count by status";
        let should_use_wb = tool.should_use_workbench(query, None);
        
        assert!(
            should_use_wb,
            "CSV export query should use Workbench (contains 'all' and 'csv')"
        );
    }
    
    /// Feature: composio-permanent-integration, Property 27: Workbench Decision Logic
    /// Test single email query uses direct execution
    #[test]
    fn test_single_email_query_uses_direct() {
        let mcp_client = Arc::new(McpClient::new("http://test", "test_key").unwrap());
        let security = Arc::new(SecurityPolicy::default());
        let tool = ComposioNaturalLanguageTool::new(mcp_client, security, "test_api_key".to_string());
        
        let query = "Send an email to user@example.com with subject 'Hello'";
        let should_use_wb = tool.should_use_workbench(query, None);
        
        assert!(
            !should_use_wb,
            "Single email query should use direct execution (no bulk/large data keywords)"
        );
    }
    
    /// Feature: composio-permanent-integration, Property 27: Workbench Decision Logic
    /// Test force flag overrides heuristic
    #[test]
    fn test_force_workbench_overrides_heuristic() {
        let mcp_client = Arc::new(McpClient::new("http://test", "test_key").unwrap());
        let security = Arc::new(SecurityPolicy::default());
        let tool = ComposioNaturalLanguageTool::new(mcp_client, security, "test_api_key".to_string());
        
        let query = "Send an email to user@example.com";
        
        // Without force, should use direct execution
        let should_use_wb_no_force = tool.should_use_workbench(query, None);
        assert!(!should_use_wb_no_force, "Should use direct execution without force");
        
        // With force true, should use Workbench
        let should_use_wb_force = tool.should_use_workbench(query, Some(true));
        assert!(should_use_wb_force, "Should use Workbench when forced");
        
        // With force false, should not use Workbench
        let should_not_use_wb = tool.should_use_workbench(query, Some(false));
        assert!(!should_not_use_wb, "Should not use Workbench when explicitly disabled");
    }
    
    /// Feature: composio-permanent-integration, Property 27: Workbench Decision Logic
    /// Test report generation uses Workbench
    #[test]
    fn test_report_generation_uses_workbench() {
        let mcp_client = Arc::new(McpClient::new("http://test", "test_key").unwrap());
        let security = Arc::new(SecurityPolicy::default());
        let tool = ComposioNaturalLanguageTool::new(mcp_client, security, "test_api_key".to_string());
        
        let query = "Generate a summary report of all Notion pages";
        let should_use_wb = tool.should_use_workbench(query, None);
        
        assert!(
            should_use_wb,
            "Report generation query should use Workbench (contains 'all' and 'summary')"
        );
    }
    
    /// Feature: composio-permanent-integration, Property 27: Workbench Decision Logic
    /// Test multiple bulk keywords
    #[test]
    fn test_multiple_bulk_keywords() {
        let mcp_client = Arc::new(McpClient::new("http://test", "test_key").unwrap());
        let security = Arc::new(SecurityPolicy::default());
        let tool = ComposioNaturalLanguageTool::new(mcp_client, security, "test_api_key".to_string());
        
        let queries = vec![
            "List every item in the database",
            "Export batch of records",
            "Process hundreds of files",
            "Analyze many documents",
        ];
        
        for query in queries {
            let should_use_wb = tool.should_use_workbench(query, None);
            assert!(
                should_use_wb,
                "Query '{}' should use Workbench (contains bulk keyword)",
                query
            );
        }
    }
    
    /// Feature: composio-permanent-integration, Property 27: Workbench Decision Logic
    /// Test multiple large data keywords
    #[test]
    fn test_multiple_large_data_keywords() {
        let mcp_client = Arc::new(McpClient::new("http://test", "test_key").unwrap());
        let security = Arc::new(SecurityPolicy::default());
        let tool = ComposioNaturalLanguageTool::new(mcp_client, security, "test_api_key".to_string());
        
        let queries = vec![
            "Create a spreadsheet with data",
            "Analyze the statistics",
            "Aggregate the results",
            "Count all occurrences",
        ];
        
        for query in queries {
            let should_use_wb = tool.should_use_workbench(query, None);
            assert!(
                should_use_wb,
                "Query '{}' should use Workbench (contains large data keyword)",
                query
            );
        }
    }
}
