// Composio REST API Client — shared client for v2/v3 API access
//
// This client is used by both ComposioTool and onboarding flows to avoid
// circular dependencies and code duplication.
//
// Enhanced for Composio Permanent Integration:
// - Direct tool execution via REST API v3
// - Connection pooling (8 idle connections per host)
// - Timeout configuration (180s execution, 30s metadata)
// - Proper error handling with structured error types

use anyhow::Context;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

const COMPOSIO_API_BASE_V2: &str = "https://backend.composio.dev/api/v2";
const COMPOSIO_API_BASE_V3: &str = "https://backend.composio.dev/api/v3";
const COMPOSIO_TOOL_VERSION_LATEST: &str = "latest";

// Timeout constants
const EXECUTION_TIMEOUT_SECS: u64 = 180; // 3 minutes for tool execution
const METADATA_TIMEOUT_SECS: u64 = 30;   // 30 seconds for metadata operations

/// Shared REST client for Composio API (v2/v3)
///
/// Supports both OAuth connection management and direct tool execution.
/// Uses connection pooling and proper timeouts for optimal performance.
pub struct ComposioRestClient {
    api_key: String,
    user_id: String,
    client: Client,
    execution_client: Client, // Separate client with longer timeout for tool execution
}

impl ComposioRestClient {
    /// Create a new Composio REST client with connection pooling and timeouts
    ///
    /// # Arguments
    /// * `api_key` - Composio API key
    /// * `user_id` - User ID for v3 session-based architecture
    ///
    /// # Connection Pooling
    /// - 8 idle connections per host for optimal performance
    /// - Automatic connection reuse across requests
    ///
    /// # Timeouts
    /// - Metadata operations: 30 seconds
    /// - Tool execution: 180 seconds
    pub fn new(api_key: String, user_id: String) -> Self {
        // Metadata client with 30s timeout
        let client = Client::builder()
            .pool_max_idle_per_host(8)
            .timeout(Duration::from_secs(METADATA_TIMEOUT_SECS))
            .build()
            .unwrap_or_else(|_| Client::new());

        // Execution client with 180s timeout
        let execution_client = Client::builder()
            .pool_max_idle_per_host(8)
            .timeout(Duration::from_secs(EXECUTION_TIMEOUT_SECS))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            api_key,
            user_id,
            client,
            execution_client,
        }
    }

    /// Get the user ID for this client
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    /// Execute a tool directly via REST API v3
    ///
    /// # Arguments
    /// * `tool_name` - Name of the tool to execute (e.g., "GMAIL_SEND_EMAIL")
    /// * `params` - Tool parameters as JSON value
    ///
    /// # Returns
    /// Tool execution result with success status and output
    ///
    /// # Errors
    /// Returns error if:
    /// - Tool not found
    /// - Authentication required (OAuth not connected)
    /// - Invalid parameters
    /// - Network error
    /// - Execution timeout (180s)
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        params: Value,
    ) -> anyhow::Result<ToolExecutionResult> {
        let url = format!("{COMPOSIO_API_BASE_V3}/actions/{}/execute", tool_name);

        let body = json!({
            "input": params,
            "userId": self.user_id,
        });

        tracing::debug!(
            tool = tool_name,
            user_id = %self.user_id,
            "Executing tool via REST API v3"
        );

        let resp = self
            .execution_client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .json(&body)
            .send()
            .await
            .context("Failed to send tool execution request")?;

        let status = resp.status();
        tracing::debug!(
            tool = tool_name,
            status = %status,
            "Received tool execution response"
        );

        if !status.is_success() {
            let err = response_error(resp).await;
            
            // Check for authentication errors
            if status.as_u16() == 401 || status.as_u16() == 403 {
                return Err(anyhow::anyhow!(
                    "Authentication required for tool '{}': {}. Please connect your account.",
                    tool_name,
                    err
                ));
            }
            
            return Err(anyhow::anyhow!(
                "Tool execution failed for '{}': {}",
                tool_name,
                err
            ));
        }

        let result: ToolExecutionResult = resp
            .json()
            .await
            .context("Failed to decode tool execution response")?;

        tracing::info!(
            tool = tool_name,
            success = result.success,
            "Tool execution completed"
        );

        Ok(result)
    }

    /// List available tools for the user
    ///
    /// # Arguments
    /// * `toolkit` - Optional toolkit filter (e.g., "gmail", "slack")
    ///
    /// # Returns
    /// List of available tools with their schemas
    pub async fn list_tools(&self, toolkit: Option<&str>) -> anyhow::Result<Vec<ToolInfo>> {
        let url = format!("{COMPOSIO_API_BASE_V3}/actions");
        
        let mut req = self
            .client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .query(&[("userId", &self.user_id)]);

        if let Some(tk) = toolkit {
            req = req.query(&[("apps", tk)]);
        }

        tracing::debug!(
            toolkit = ?toolkit,
            user_id = %self.user_id,
            "Listing tools via REST API v3"
        );

        let resp = req
            .send()
            .await
            .context("Failed to send list tools request")?;

        if !resp.status().is_success() {
            let err = response_error(resp).await;
            return Err(anyhow::anyhow!("Failed to list tools: {}", err));
        }

        let body: ToolsListResponse = resp
            .json()
            .await
            .context("Failed to decode tools list response")?;

        tracing::info!(
            count = body.items.len(),
            "Retrieved tools list"
        );

        Ok(body.items)
    }

    /// Get detailed schema for a specific tool
    ///
    /// # Arguments
    /// * `tool_name` - Name of the tool
    ///
    /// # Returns
    /// Tool schema with parameters and description
    pub async fn get_tool_schema(&self, tool_name: &str) -> anyhow::Result<ToolInfo> {
        let url = format!("{COMPOSIO_API_BASE_V3}/actions/{}", tool_name);

        tracing::debug!(
            tool = tool_name,
            "Fetching tool schema via REST API v3"
        );

        let resp = self
            .client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .send()
            .await
            .context("Failed to send get tool schema request")?;

        if !resp.status().is_success() {
            let err = response_error(resp).await;
            return Err(anyhow::anyhow!(
                "Failed to get tool schema for '{}': {}",
                tool_name,
                err
            ));
        }

        let tool: ToolInfo = resp
            .json()
            .await
            .context("Failed to decode tool schema response")?;

        tracing::debug!(
            tool = tool_name,
            "Retrieved tool schema"
        );

        Ok(tool)
    }

    /// Get the OAuth connection URL for a specific app/toolkit or auth config
    pub async fn get_connection_url(
        &self,
        app_name: Option<&str>,
        auth_config_id: Option<&str>,
        entity_id: &str,
    ) -> anyhow::Result<ComposioConnectionLink> {
        let v3 = self
            .get_connection_url_v3(app_name, auth_config_id, entity_id)
            .await;
        match v3 {
            Ok(url) => Ok(url),
            Err(v3_err) => {
                let app = app_name.ok_or_else(|| {
                    anyhow::anyhow!(
                        "Composio v3 connect failed ({v3_err}) and v2 fallback requires 'app'"
                    )
                })?;
                match self.get_connection_url_v2(app, entity_id).await {
                    Ok(url) => Ok(url),
                    Err(v2_err) => anyhow::bail!(
                        "Composio connect failed on v3 ({v3_err}) and v2 fallback ({v2_err})"
                    ),
                }
            }
        }
    }

    async fn get_connection_url_v3(

        &self,
        app_name: Option<&str>,
        auth_config_id: Option<&str>,
        entity_id: &str,
    ) -> anyhow::Result<ComposioConnectionLink> {
        let auth_config_id = match auth_config_id {
            Some(id) => id.to_string(),
            None => {
                let app = app_name.ok_or_else(|| {
                    anyhow::anyhow!("Missing 'app' or 'auth_config_id' for v3 connect")
                })?;
                self.resolve_auth_config_id(app).await?
            }
        };

        let url = format!("{COMPOSIO_API_BASE_V3}/connected_accounts/link");
        let body = json!({
            "authConfigId": auth_config_id,
            "userId": entity_id,
        });

        tracing::debug!(
            url = %url,
            auth_config_id = %auth_config_id,
            user_id = %entity_id,
            "Sending Composio v3 connect request"
        );

        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        tracing::debug!(
            status = %status,
            "Received Composio v3 connect response"
        );

        if !status.is_success() {
            let err = response_error(resp).await;
            anyhow::bail!("Composio v3 connect failed: {err}");
        }

        let result: serde_json::Value = resp
            .json()
            .await
            .context("Failed to decode Composio v3 connect response")?;
        
        // Debug log to see the actual response
        tracing::debug!(
            response = ?result,
            "Composio v3 connect response received"
        );
        
        let redirect_url = extract_redirect_url(&result)
            .ok_or_else(|| {
                tracing::error!(
                    response = ?result,
                    "Failed to extract redirect URL from Composio v3 response"
                );
                anyhow::anyhow!("No redirect URL in Composio v3 response: {:?}", result)
            })?;
        
        tracing::info!(
            redirect_url = %redirect_url,
            "Successfully extracted redirect URL from Composio v3"
        );
        
        Ok(ComposioConnectionLink {
            redirect_url,
            connected_account_id: extract_connected_account_id(&result),
        })
    }

    async fn get_connection_url_v2(
        &self,
        app_name: &str,
        entity_id: &str,
    ) -> anyhow::Result<ComposioConnectionLink> {
        let url = format!("{COMPOSIO_API_BASE_V2}/connectedAccounts");

        let body = json!({
            "integrationId": app_name,
            "entityId": entity_id,
        });

        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = response_error(resp).await;
            anyhow::bail!("Composio v2 connect failed: {err}");
        }

        let result: serde_json::Value = resp
            .json()
            .await
            .context("Failed to decode Composio v2 connect response")?;
        let redirect_url = extract_redirect_url(&result)
            .ok_or_else(|| anyhow::anyhow!("No redirect URL in Composio v2 response"))?;
        Ok(ComposioConnectionLink {
            redirect_url,
            connected_account_id: extract_connected_account_id(&result),
        })
    }

    /// Resolve auth config ID for a given app/toolkit
    pub async fn resolve_auth_config_id(&self, app_name: &str) -> anyhow::Result<String> {
        let url = format!("{COMPOSIO_API_BASE_V3}/auth_configs");

        let resp = self
            .client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .query(&[
                ("toolkit_slug", app_name),
                ("show_disabled", "true"),
                ("limit", "25"),
            ])
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = response_error(resp).await;
            anyhow::bail!("Composio v3 auth config lookup failed: {err}");
        }

        let body: ComposioAuthConfigsResponse = resp
            .json()
            .await
            .context("Failed to decode Composio v3 auth configs response")?;

        if body.items.is_empty() {
            anyhow::bail!(
                "No authentication configuration found for app '{app_name}'. \
                 \nThis usually means the app needs to be set up in your Composio account first.\
                 \nPlease contact support or check the Composio documentation for setup instructions."
            );
        }

        let preferred = body
            .items
            .iter()
            .find(|cfg| cfg.is_enabled())
            .or_else(|| body.items.first())
            .context("No usable auth config returned by Composio")?;

        Ok(preferred.id.clone())
    }

    /// List connected accounts for a user and optional toolkit/app
    pub async fn list_connected_accounts(
        &self,
        app_name: Option<&str>,
        entity_id: Option<&str>,
    ) -> anyhow::Result<Vec<ComposioConnectedAccount>> {
        let url = format!("{COMPOSIO_API_BASE_V3}/connected_accounts");
        let mut req = self.client.get(&url).header("x-api-key", &self.api_key);

        req = req.query(&[
            ("limit", "50"),
            ("order_by", "updated_at"),
            ("order_direction", "desc"),
            ("statuses", "INITIALIZING"),
            ("statuses", "ACTIVE"),
            ("statuses", "INITIATED"),
        ]);

        if let Some(app) = app_name
            .map(normalize_app_slug)
            .filter(|app| !app.is_empty())
        {
            req = req.query(&[("toolkit_slugs", app.as_str())]);
        }

        if let Some(entity) = entity_id {
            req = req.query(&[("user_ids", entity)]);
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            let err = response_error(resp).await;
            anyhow::bail!("Composio v3 connected accounts lookup failed: {err}");
        }

        let body: ComposioConnectedAccountsResponse = resp
            .json()
            .await
            .context("Failed to decode Composio v3 connected accounts response")?;
        Ok(body.items)
    }

    /// Delete a connected account by ID
    ///
    /// # Arguments
    /// * `connection_id` - The connected account ID to delete
    ///
    /// # Returns
    /// Ok(()) if deletion was successful, Err otherwise
    pub async fn delete_connected_account(&self, connection_id: &str) -> anyhow::Result<()> {
        let url = format!(
            "https://backend.composio.dev/api/v1/connectedAccounts/{}",
            connection_id
        );

        let resp = self
            .client
            .delete(&url)
            .header("X-API-Key", &self.api_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = response_error(resp).await;
            anyhow::bail!("Failed to delete connected account {}: {}", connection_id, err);
        }

        Ok(())
    }
}

// ── Helper functions ──────────────────────────────────────────

fn normalize_app_slug(app_name: &str) -> String {
    app_name
        .trim()
        .replace('_', "-")
        .to_ascii_lowercase()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn extract_redirect_url(result: &serde_json::Value) -> Option<String> {
    // Try all possible field names and locations
    let url = result
        .get("redirectUrl")
        .and_then(|v| v.as_str())
        .or_else(|| result.get("redirect_url").and_then(|v| v.as_str()))
        .or_else(|| result.get("url").and_then(|v| v.as_str()))
        .or_else(|| result.get("link").and_then(|v| v.as_str()))
        .or_else(|| {
            result
                .get("data")
                .and_then(|v| v.get("redirectUrl"))
                .and_then(|v| v.as_str())
        })
        .or_else(|| {
            result
                .get("data")
                .and_then(|v| v.get("redirect_url"))
                .and_then(|v| v.as_str())
        })
        .or_else(|| {
            result
                .get("data")
                .and_then(|v| v.get("url"))
                .and_then(|v| v.as_str())
        })
        .map(ToString::to_string);
    
    if url.is_none() {
        tracing::warn!(
            response_keys = ?result.as_object().map(|o| o.keys().collect::<Vec<_>>()),
            "Could not find redirect URL in response. Available keys logged."
        );
    }
    
    url
}

fn extract_connected_account_id(result: &serde_json::Value) -> Option<String> {
    result
        .get("connected_account_id")
        .and_then(|v| v.as_str())
        .or_else(|| result.get("connectedAccountId").and_then(|v| v.as_str()))
        .or_else(|| {
            result
                .get("data")
                .and_then(|v| v.get("connected_account_id"))
                .and_then(|v| v.as_str())
        })
        .or_else(|| {
            result
                .get("data")
                .and_then(|v| v.get("connectedAccountId"))
                .and_then(|v| v.as_str())
        })
        .map(ToString::to_string)
}

async fn response_error(resp: reqwest::Response) -> String {
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if body.trim().is_empty() {
        return format!("HTTP {}", status.as_u16());
    }

    if let Some(api_error) = extract_api_error_message(&body) {
        return format!("HTTP {}: {}", status.as_u16(), api_error);
    }

    format!("HTTP {}", status.as_u16())
}

fn extract_api_error_message(body: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(body).ok()?;
    parsed
        .get("error")
        .and_then(|v| v.get("message"))
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
        .or_else(|| {
            parsed
                .get("message")
                .and_then(|v| v.as_str())
                .map(ToString::to_string)
        })
}

// ── API response types ──────────────────────────────────────────

/// Tool execution result from REST API v3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    /// Whether the execution was successful
    #[serde(default)]
    pub success: bool,
    
    /// Execution output data
    #[serde(default)]
    pub data: Option<Value>,
    
    /// Error message if execution failed
    #[serde(default)]
    pub error: Option<String>,
    
    /// Execution metadata
    #[serde(default)]
    pub metadata: Option<Value>,
}

impl ToolExecutionResult {
    /// Convert to output string for display
    pub fn to_output_string(&self) -> String {
        if let Some(ref error) = self.error {
            return error.clone();
        }
        
        if let Some(ref data) = self.data {
            return serde_json::to_string_pretty(data)
                .unwrap_or_else(|_| format!("{:?}", data));
        }
        
        "No output".to_string()
    }
    
    /// Check if this is an error result
    pub fn is_error(&self) -> bool {
        !self.success || self.error.is_some()
    }
}

/// Tool information from REST API v3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    /// Tool name (e.g., "GMAIL_SEND_EMAIL")
    pub name: String,
    
    /// Tool description
    #[serde(default)]
    pub description: Option<String>,
    
    /// Tool parameters schema (JSON Schema)
    #[serde(default)]
    pub parameters: Option<Value>,
    
    /// Toolkit/app name (e.g., "gmail")
    #[serde(default)]
    pub app_name: Option<String>,
    
    /// Whether authentication is required
    #[serde(default)]
    pub requires_auth: bool,
}

/// Response from tools list endpoint
#[derive(Debug, Deserialize)]
struct ToolsListResponse {
    #[serde(default)]
    items: Vec<ToolInfo>,
}

#[derive(Debug, Deserialize)]
struct ComposioConnectedAccountsResponse {
    #[serde(default)]
    items: Vec<ComposioConnectedAccount>,
}

#[derive(Debug, Deserialize)]
struct ComposioAuthConfigsResponse {
    #[serde(default)]
    items: Vec<ComposioAuthConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComposioConnectedAccount {
    pub id: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub toolkit: Option<ComposioToolkitRef>,
}

impl ComposioConnectedAccount {
    pub fn is_usable(&self) -> bool {
        self.status.eq_ignore_ascii_case("INITIALIZING")
            || self.status.eq_ignore_ascii_case("ACTIVE")
            || self.status.eq_ignore_ascii_case("INITIATED")
    }

    pub fn toolkit_slug(&self) -> Option<&str> {
        self.toolkit
            .as_ref()
            .and_then(|toolkit| toolkit.slug.as_deref())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComposioToolkitRef {
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ComposioConnectionLink {
    pub redirect_url: String,
    pub connected_account_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComposioAuthConfig {
    pub id: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

impl ComposioAuthConfig {
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(false)
            || self
                .status
                .as_deref()
                .is_some_and(|v| v.eq_ignore_ascii_case("enabled"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_app_slug_removes_spaces_and_normalizes_case() {
        assert_eq!(normalize_app_slug(" Gmail "), "gmail");
        assert_eq!(normalize_app_slug("GITHUB_APP"), "github-app");
    }

    #[test]
    fn extract_redirect_url_supports_v2_and_v3_shapes() {
        let v2 = serde_json::json!({"redirectUrl": "https://app.composio.dev/connect-v2"});
        let v3 = serde_json::json!({"redirect_url": "https://app.composio.dev/connect-v3"});
        let nested = serde_json::json!({"data": {"redirect_url": "https://app.composio.dev/connect-nested"}});

        assert_eq!(
            extract_redirect_url(&v2).as_deref(),
            Some("https://app.composio.dev/connect-v2")
        );
        assert_eq!(
            extract_redirect_url(&v3).as_deref(),
            Some("https://app.composio.dev/connect-v3")
        );
        assert_eq!(
            extract_redirect_url(&nested).as_deref(),
            Some("https://app.composio.dev/connect-nested")
        );
    }

    #[test]
    fn extract_connected_account_id_supports_common_shapes() {
        let root = serde_json::json!({"connected_account_id": "ca_root"});
        let camel = serde_json::json!({"connectedAccountId": "ca_camel"});
        let nested = serde_json::json!({"data": {"connected_account_id": "ca_nested"}});

        assert_eq!(
            extract_connected_account_id(&root).as_deref(),
            Some("ca_root")
        );
        assert_eq!(
            extract_connected_account_id(&camel).as_deref(),
            Some("ca_camel")
        );
        assert_eq!(
            extract_connected_account_id(&nested).as_deref(),
            Some("ca_nested")
        );
    }

    #[test]
    fn connected_account_is_usable_for_initializing_active_and_initiated() {
        for status in ["INITIALIZING", "ACTIVE", "INITIATED"] {
            let account = ComposioConnectedAccount {
                id: "ca_1".to_string(),
                status: status.to_string(),
                toolkit: None,
            };
            assert!(account.is_usable(), "status {status} should be usable");
        }
    }

    #[test]
    fn auth_config_prefers_enabled_status() {
        let enabled = ComposioAuthConfig {
            id: "cfg_1".into(),
            status: Some("ENABLED".into()),
            enabled: None,
        };
        let disabled = ComposioAuthConfig {
            id: "cfg_2".into(),
            status: Some("DISABLED".into()),
            enabled: Some(false),
        };

        assert!(enabled.is_enabled());
        assert!(!disabled.is_enabled());
    }

    #[test]
    fn rest_client_stores_user_id() {
        let client = ComposioRestClient::new(
            "test_api_key".to_string(),
            "test_user".to_string(),
        );
        assert_eq!(client.user_id(), "test_user");
    }

    #[test]
    fn tool_execution_result_to_output_string_returns_error_first() {
        let result = ToolExecutionResult {
            success: false,
            data: Some(json!({"result": "data"})),
            error: Some("Error occurred".to_string()),
            metadata: None,
        };
        assert_eq!(result.to_output_string(), "Error occurred");
    }

    #[test]
    fn tool_execution_result_to_output_string_returns_data_when_no_error() {
        let result = ToolExecutionResult {
            success: true,
            data: Some(json!({"result": "success"})),
            error: None,
            metadata: None,
        };
        let output = result.to_output_string();
        assert!(output.contains("result"));
        assert!(output.contains("success"));
    }

    #[test]
    fn tool_execution_result_to_output_string_returns_no_output_when_empty() {
        let result = ToolExecutionResult {
            success: true,
            data: None,
            error: None,
            metadata: None,
        };
        assert_eq!(result.to_output_string(), "No output");
    }

    #[test]
    fn tool_execution_result_is_error_when_success_false() {
        let result = ToolExecutionResult {
            success: false,
            data: None,
            error: None,
            metadata: None,
        };
        assert!(result.is_error());
    }

    #[test]
    fn tool_execution_result_is_error_when_error_present() {
        let result = ToolExecutionResult {
            success: true,
            data: None,
            error: Some("Error".to_string()),
            metadata: None,
        };
        assert!(result.is_error());
    }

    #[test]
    fn tool_execution_result_is_not_error_when_successful() {
        let result = ToolExecutionResult {
            success: true,
            data: Some(json!({"result": "ok"})),
            error: None,
            metadata: None,
        };
        assert!(!result.is_error());
    }
}
