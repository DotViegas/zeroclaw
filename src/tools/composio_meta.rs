// Composio Meta-Tool Wrapper (Pattern 2)
//
// This module implements dynamic tool discovery and execution using Composio's
// meta-tools: COMPOSIO_SEARCH_TOOLS, COMPOSIO_MULTI_EXECUTE_TOOL, etc.
//
// Architecture:
// 1. User query → COMPOSIO_SEARCH_TOOLS (discover relevant tools)
// 2. If schemaRef → COMPOSIO_GET_TOOL_SCHEMAS (fetch full schema)
// 3. If not connected → COMPOSIO_MANAGE_CONNECTIONS (OAuth flow)
// 4. Execute → COMPOSIO_MULTI_EXECUTE_TOOL (run discovered tool)

use super::traits::{Tool, ToolResult};
use crate::composio::{ComposioRestClient, ComposioOnboarding};
use crate::mcp::ComposioMcpClient;
use crate::security::SecurityPolicy;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Session state for Tool Router pattern
#[derive(Debug, Clone)]
pub struct ComposioSession {
    pub session_id: String,
    pub created_at: std::time::Instant,
}

/// Cache entry for discovered tools
#[derive(Debug, Clone)]
struct ToolCacheEntry {
    tool_slug: String,
    description: String,
    input_schema: Option<Value>,
    schema_ref: Option<String>,
    toolkit: String,
    cached_at: std::time::Instant,
}

/// Cache entry for tool schemas
#[derive(Debug, Clone)]
struct SchemaCacheEntry {
    schema: Value,
    cached_at: std::time::Instant,
}

/// Composio Meta-Tool - wraps Pattern 2 discovery and execution
pub struct ComposioMetaTool {
    mcp_client: Arc<ComposioMcpClient>,
    rest_client: Arc<ComposioRestClient>,
    security: Arc<SecurityPolicy>,
    onboarding: Option<Arc<dyn ComposioOnboarding>>,
    
    // Session management
    current_session: Arc<RwLock<Option<ComposioSession>>>,
    
    // Caching
    tool_cache: Arc<RwLock<HashMap<String, ToolCacheEntry>>>, // intent -> tool
    schema_cache: Arc<RwLock<HashMap<String, SchemaCacheEntry>>>, // tool_slug -> schema
    
    // Configuration
    tool_cache_ttl: std::time::Duration,
    schema_cache_ttl: std::time::Duration,
    session_ttl: std::time::Duration,
}

impl ComposioMetaTool {
    /// Create a new meta-tool wrapper
    pub fn new(
        mcp_client: Arc<ComposioMcpClient>,
        rest_client: Arc<ComposioRestClient>,
        security: Arc<SecurityPolicy>,
        onboarding: Option<Arc<dyn ComposioOnboarding>>,
    ) -> Self {
        Self {
            mcp_client,
            rest_client,
            security,
            onboarding,
            current_session: Arc::new(RwLock::new(None)),
            tool_cache: Arc::new(RwLock::new(HashMap::new())),
            schema_cache: Arc::new(RwLock::new(HashMap::new())),
            tool_cache_ttl: std::time::Duration::from_secs(300), // 5 minutes
            schema_cache_ttl: std::time::Duration::from_secs(3600), // 1 hour
            session_ttl: std::time::Duration::from_secs(1800), // 30 minutes
        }
    }
    
    /// Get or create a session for the current conversation
    async fn ensure_session(&self) -> Result<String> {
        let mut session_lock = self.current_session.write().await;
        
        // Check if we have a valid session
        if let Some(session) = session_lock.as_ref() {
            if session.created_at.elapsed() < self.session_ttl {
                return Ok(session.session_id.clone());
            }
            
            tracing::debug!(
                session_id = session.session_id,
                age_secs = session.created_at.elapsed().as_secs(),
                "Session expired, creating new one"
            );
        }
        
        // Create new session by calling COMPOSIO_SEARCH_TOOLS with generate_id: true
        tracing::info!("Creating new Composio Tool Router session");
        
        let search_params = serde_json::json!({
            "queries": [{
                "use_case": "initialize session"
            }],
            "session": {
                "generate_id": true
            }
        });
        
        let result = self.mcp_client
            .execute_tool("COMPOSIO_SEARCH_TOOLS", search_params)
            .await
            .context("Failed to create new session")?;
        
        // Extract session_id from response
        let session_id = result
            .content
            .first()
            .and_then(|c| c.data.as_ref())
            .and_then(|d| d.get("session_id"))
            .and_then(|s| s.as_str())
            .ok_or_else(|| anyhow::anyhow!("No session_id in COMPOSIO_SEARCH_TOOLS response"))?
            .to_string();
        
        tracing::info!(session_id = session_id, "Created new session");
        
        let new_session = ComposioSession {
            session_id: session_id.clone(),
            created_at: std::time::Instant::now(),
        };
        
        *session_lock = Some(new_session);
        Ok(session_id)
    }
    
    /// Search for tools matching the user's intent
    async fn search_tools(&self, user_query: &str, session_id: &str) -> Result<Vec<ToolCacheEntry>> {
        tracing::debug!(
            query = user_query,
            session_id = session_id,
            "Searching for tools"
        );
        
        let search_params = serde_json::json!({
            "queries": [{
                "use_case": user_query
            }],
            "session": {
                "id": session_id
            }
        });
        
        let result = self.mcp_client
            .execute_tool("COMPOSIO_SEARCH_TOOLS", search_params)
            .await
            .context("Failed to search tools")?;
        
        // Parse response to extract tools
        let tools_data = result
            .content
            .first()
            .and_then(|c| c.data.as_ref())
            .ok_or_else(|| anyhow::anyhow!("No data in COMPOSIO_SEARCH_TOOLS response"))?;
        
        let mut discovered_tools = Vec::new();
        
        // Extract tools from response (structure varies, handle both formats)
        if let Some(toolkits) = tools_data.get("toolkits").and_then(|t| t.as_array()) {
            for toolkit_obj in toolkits {
                let toolkit_name = toolkit_obj
                    .get("toolkit")
                    .and_then(|t| t.as_str())
                    .unwrap_or("unknown");
                
                if let Some(tools) = toolkit_obj.get("tools").and_then(|t| t.as_array()) {
                    for tool in tools {
                        let tool_slug = match tool.get("tool_slug").and_then(|s| s.as_str()) {
                            Some(slug) => slug,
                            None => continue,
                        };
                        
                        let description = tool.get("description")
                            .and_then(|d| d.as_str())
                            .unwrap_or("")
                            .to_string();
                        
                        let input_schema = tool.get("input_schema").cloned();
                        let schema_ref = tool.get("schemaRef")
                            .and_then(|s| s.as_str())
                            .map(|s| s.to_string());
                        
                        discovered_tools.push(ToolCacheEntry {
                            tool_slug: tool_slug.to_string(),
                            description,
                            input_schema,
                            schema_ref,
                            toolkit: toolkit_name.to_string(),
                            cached_at: std::time::Instant::now(),
                        });
                    }
                }
            }
        }
        
        tracing::info!(
            count = discovered_tools.len(),
            "Discovered tools for query"
        );
        
        Ok(discovered_tools)
    }
    
    /// Get full schema for a tool (if it has schemaRef)
    async fn get_tool_schema(&self, tool_slug: &str, session_id: &str) -> Result<Value> {
        // Check cache first
        {
            let cache = self.schema_cache.read().await;
            if let Some(entry) = cache.get(tool_slug) {
                if entry.cached_at.elapsed() < self.schema_cache_ttl {
                    tracing::debug!(tool_slug = tool_slug, "Schema cache hit");
                    return Ok(entry.schema.clone());
                }
            }
        }
        
        tracing::debug!(tool_slug = tool_slug, "Fetching schema");
        
        let params = serde_json::json!({
            "tool_slugs": [tool_slug],
            "session_id": session_id
        });
        
        let result = self.mcp_client
            .execute_tool("COMPOSIO_GET_TOOL_SCHEMAS", params)
            .await
            .context("Failed to get tool schema")?;
        
        let schema = result
            .content
            .first()
            .and_then(|c| c.data.as_ref())
            .and_then(|d| d.get("schemas"))
            .and_then(|s| s.as_array())
            .and_then(|arr| arr.first())
            .and_then(|s| s.get("input_schema"))
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No schema in response"))?;
        
        // Cache it
        {
            let mut cache = self.schema_cache.write().await;
            cache.insert(
                tool_slug.to_string(),
                SchemaCacheEntry {
                    schema: schema.clone(),
                    cached_at: std::time::Instant::now(),
                },
            );
        }
        
        Ok(schema)
    }
    
    /// Ensure toolkit is connected (OAuth flow if needed)
    async fn ensure_connected(&self, toolkit: &str, session_id: &str) -> Result<()> {
        tracing::debug!(toolkit = toolkit, "Checking connection status");
        
        let params = serde_json::json!({
            "toolkits": [toolkit],
            "session_id": session_id
        });
        
        let result = self.mcp_client
            .execute_tool("COMPOSIO_MANAGE_CONNECTIONS", params)
            .await
            .context("Failed to manage connections")?;
        
        // Check if we got a redirect_url (means not connected)
        let data = result
            .content
            .first()
            .and_then(|c| c.data.as_ref());
        
        if let Some(redirect_url) = data.and_then(|d| d.get("redirect_url")).and_then(|u| u.as_str()) {
            // Not connected - need OAuth
            tracing::info!(toolkit = toolkit, "OAuth required");
            
            if let Some(onboarding) = &self.onboarding {
                // Use onboarding to handle OAuth flow
                onboarding.ensure_connected(toolkit, &self.mcp_client.user_id()).await?;
            } else {
                // No onboarding handler - return error with URL
                anyhow::bail!(
                    "OAuth authorization required for {}.\n\n\
                    Please click this link to authorize:\n{}\n\n\
                    After authorizing, please retry your request.\n\
                    The authorization link expires in 10 minutes.",
                    toolkit.to_uppercase(),
                    redirect_url
                );
            }
        } else {
            tracing::debug!(toolkit = toolkit, "Connection active");
        }
        
        Ok(())
    }
    
    /// Execute a discovered tool
    async fn execute_tool(
        &self,
        tool_slug: &str,
        arguments: Value,
        session_id: &str,
    ) -> Result<Value> {
        tracing::debug!(
            tool_slug = tool_slug,
            "Executing tool via COMPOSIO_MULTI_EXECUTE_TOOL"
        );
        
        let params = serde_json::json!({
            "tools": [{
                "tool_slug": tool_slug,
                "arguments": arguments
            }],
            "sync_response_to_workbench": false,
            "session_id": session_id
        });
        
        let result = self.mcp_client
            .execute_tool("COMPOSIO_MULTI_EXECUTE_TOOL", params)
            .await
            .context("Failed to execute tool")?;
        
        // Extract result
        let tool_result = result
            .content
            .first()
            .and_then(|c| c.data.as_ref())
            .and_then(|d| d.get("results"))
            .and_then(|r| r.as_array())
            .and_then(|arr| arr.first())
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No result in COMPOSIO_MULTI_EXECUTE_TOOL response"))?;
        
        // Check for errors
        if let Some(error) = tool_result.get("error") {
            let error_msg = error.as_str().unwrap_or("Unknown error");
            anyhow::bail!("Tool execution failed: {}", error_msg);
        }
        
        Ok(tool_result)
    }
}

#[async_trait]
impl Tool for ComposioMetaTool {
    fn name(&self) -> &str {
        "composio_dynamic"
    }
    
    fn description(&self) -> &str {
        "Dynamic tool discovery and execution for 1000+ apps via Composio. \
        Describe what you want to do (e.g., 'list my Dropbox folder', 'send Gmail email') \
        and this tool will discover and execute the appropriate action."
    }
    
    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Natural language description of what you want to do. \
                    Examples: 'list my Dropbox folder /Documents', 'send email to john@example.com', \
                    'create GitHub issue in my-repo'"
                },
                "arguments": {
                    "type": "object",
                    "description": "Optional: Specific arguments for the tool if you know them. \
                    If not provided, the tool will be discovered and you'll be prompted for required parameters.",
                    "additionalProperties": true
                }
            },
            "required": ["query"]
        })
    }
    
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        // Security check
        if let Err(error) = self.security.enforce_tool_operation(
            crate::security::policy::ToolOperation::Act,
            "composio_dynamic",
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
        
        tracing::info!(query = query, "Executing dynamic Composio tool");
        
        // 1. Ensure we have a session
        let session_id = self.ensure_session().await?;
        
        // 2. Search for relevant tools
        let discovered_tools = self.search_tools(query, &session_id).await?;
        
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
        
        // 4. Get full schema if needed
        let schema = if tool.input_schema.is_some() {
            tool.input_schema.clone().unwrap()
        } else if tool.schema_ref.is_some() {
            self.get_tool_schema(&tool.tool_slug, &session_id).await?
        } else {
            serde_json::json!({})
        };
        
        // 5. Prepare arguments
        let tool_args = if let Some(args) = provided_args {
            args
        } else {
            // TODO: Could implement argument extraction from query here
            // For now, return schema and ask user to provide arguments
            return Ok(ToolResult {
                success: false,
                output: format!(
                    "Tool found: {}\nDescription: {}\n\nRequired parameters:\n{}",
                    tool.tool_slug,
                    tool.description,
                    serde_json::to_string_pretty(&schema)?
                ),
                error: Some(format!(
                    "Please provide arguments for {}. Use the 'arguments' parameter.",
                    tool.tool_slug
                )),
            });
        };
        
        // 6. Ensure toolkit is connected
        if let Err(e) = self.ensure_connected(&tool.toolkit, &session_id).await {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            });
        }
        
        // 7. Execute the tool
        match self.execute_tool(&tool.tool_slug, tool_args, &session_id).await {
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
                    error: Some(e.to_string()),
                })
            }
        }
    }
}
