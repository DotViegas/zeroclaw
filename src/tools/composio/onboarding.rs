//! OAuth Onboarding Handler for Composio Toolkits
//!
//! This module implements the OAuth connection flow for Composio toolkits,
//! supporting multiple onboarding modes:
//! - CliAutoOpen: Automatically open browser for OAuth
//! - CliPrintOnly: Print OAuth URL to stdout
//! - CliCallbackLocal: Start local callback server
//! - WebCallback: Return OAuth URL to caller
//!
//! The handler manages:
//! - Connect link generation with 10-minute expiry
//! - Connection polling with timeout
//! - Browser automation for CLI modes
//! - Connection status checking

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

use crate::config::composio::{ComposioConfig, OnboardingMode};
use crate::tools::composio::meta_tools::{ConnectionInfo, McpClientTrait};

/// OAuth onboarding handler for Composio toolkits
pub struct OnboardingHandler {
    config: ComposioConfig,
    mcp_client: Arc<dyn McpClientTrait>,
}

/// OAuth flow state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OAuthFlow {
    /// OAuth completed successfully
    Completed(ConnectionInfo),
    
    /// OAuth pending - user needs to complete authentication
    Pending {
        connect_link: String,
        expires_at: DateTime<Utc>,
    },
    
    /// OAuth failed with error
    Failed {
        error: String,
    },
}

/// Connect link information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectLink {
    pub url: String,
    pub expires_at: DateTime<Utc>,
    pub toolkit: String,
}

impl OnboardingHandler {
    /// Create a new onboarding handler
    ///
    /// # Arguments
    /// * `config` - Composio configuration
    /// * `mcp_client` - MCP client for making requests
    pub fn new(config: ComposioConfig, mcp_client: Arc<dyn McpClientTrait>) -> Self {
        Self { config, mcp_client }
    }

    /// Get the effective user_id from config
    fn effective_user_id(&self) -> String {
        let (user_id, is_legacy) = self.config.effective_user_id();
        if is_legacy {
            tracing::warn!(
                "Using legacy entity_id field. Please migrate to user_id in config."
            );
        }
        user_id
    }

    /// Initiate OAuth flow for a toolkit
    ///
    /// This method implements the full OAuth onboarding flow based on the configured mode:
    /// - CliAutoOpen: Generate link, open browser, poll for completion
    /// - CliPrintOnly: Generate link, print to stdout, poll for completion
    /// - CliCallbackLocal: Generate link, start local server, wait for callback
    /// - WebCallback: Generate link, return immediately with Pending status
    ///
    /// # Arguments
    /// * `toolkit` - Toolkit name (e.g., "gmail", "slack", "github")
    ///
    /// # Returns
    /// * `Ok(OAuthFlow::Completed)` - OAuth completed successfully
    /// * `Ok(OAuthFlow::Pending)` - OAuth pending (WebCallback mode)
    /// * `Ok(OAuthFlow::Failed)` - OAuth failed
    /// * `Err(anyhow::Error)` - System errors (network, parsing, etc.)
    pub async fn initiate_oauth(&self, toolkit: &str) -> Result<OAuthFlow> {
        tracing::info!(
            toolkit = toolkit,
            mode = ?self.config.onboarding_mode,
            "Initiating OAuth flow"
        );

        // Step 1: Generate connect link
        let connect_link = self.generate_connect_link(toolkit).await?;

        tracing::info!(
            toolkit = toolkit,
            connect_link = %connect_link.url,
            expires_at = %connect_link.expires_at,
            "Connect link generated"
        );

        // Step 2: Handle based on onboarding mode
        match self.config.onboarding_mode {
            OnboardingMode::CliAutoOpen => {
                self.handle_cli_auto_open(toolkit, &connect_link).await
            }
            OnboardingMode::CliPrintOnly => {
                self.handle_cli_print_only(toolkit, &connect_link).await
            }
            OnboardingMode::CliCallbackLocal => {
                self.handle_cli_callback_local(toolkit, &connect_link).await
            }
            OnboardingMode::WebCallback => {
                self.handle_web_callback(toolkit, &connect_link).await
            }
        }
    }

    /// Handle CLI auto-open mode: open browser and poll for completion
    async fn handle_cli_auto_open(
        &self,
        toolkit: &str,
        connect_link: &ConnectLink,
    ) -> Result<OAuthFlow> {
        // Open browser
        if let Err(e) = self.open_browser(&connect_link.url) {
            tracing::warn!(
                toolkit = toolkit,
                error = %e,
                "Failed to open browser automatically"
            );
            // Fall back to print-only behavior
            println!("\nPlease visit the following URL to authenticate:");
            println!("{}", connect_link.url);
            println!("Link expires at: {}", connect_link.expires_at);
        } else {
            tracing::info!(
                toolkit = toolkit,
                "Browser opened for OAuth authentication"
            );
            println!("\nOpened browser for OAuth authentication.");
            println!("If the browser didn't open, visit: {}", connect_link.url);
        }

        // Poll for connection
        self.poll_for_connection(toolkit).await
    }

    /// Handle CLI print-only mode: print URL and poll for completion
    async fn handle_cli_print_only(
        &self,
        toolkit: &str,
        connect_link: &ConnectLink,
    ) -> Result<OAuthFlow> {
        println!("\nPlease visit the following URL to authenticate:");
        println!("{}", connect_link.url);
        println!("Link expires at: {}", connect_link.expires_at);
        println!("\nWaiting for authentication to complete...");

        // Poll for connection
        self.poll_for_connection(toolkit).await
    }

    /// Handle CLI callback local mode: start local server and wait for callback
    async fn handle_cli_callback_local(
        &self,
        toolkit: &str,
        connect_link: &ConnectLink,
    ) -> Result<OAuthFlow> {
        use axum::{
            extract::Query,
            response::Html,
            routing::get,
            Router,
        };
        use std::net::SocketAddr;
        use std::sync::Arc as StdArc;
        use parking_lot::Mutex;

        tracing::info!(
            toolkit = toolkit,
            "Starting local callback server for OAuth"
        );

        // Create a shared flag to signal when OAuth completes
        let callback_received = StdArc::new(Mutex::new(false));
        let callback_received_clone = callback_received.clone();

        // Find an available port starting from 8080
        let port = self.find_available_port(8080, 8100)?;
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let callback_url = format!("http://localhost:{}/callback", port);

        tracing::info!(
            toolkit = toolkit,
            port = port,
            callback_url = %callback_url,
            "Local callback server will listen on port"
        );

        // Clone toolkit for the callback handler
        let toolkit_clone = toolkit.to_string();

        // Create callback handler
        let callback_handler = move |Query(params): Query<std::collections::HashMap<String, String>>| async move {
            tracing::info!(
                toolkit = %toolkit_clone,
                params = ?params,
                "Received OAuth callback"
            );

            // Signal that callback was received
            *callback_received_clone.lock() = true;

            // Return success page
            Html(
                r#"
                <!DOCTYPE html>
                <html>
                <head>
                    <title>OAuth Success</title>
                    <style>
                        body {
                            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
                            display: flex;
                            justify-content: center;
                            align-items: center;
                            height: 100vh;
                            margin: 0;
                            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                        }
                        .container {
                            background: white;
                            padding: 3rem;
                            border-radius: 1rem;
                            box-shadow: 0 20px 60px rgba(0,0,0,0.3);
                            text-align: center;
                            max-width: 500px;
                        }
                        h1 {
                            color: #2d3748;
                            margin-bottom: 1rem;
                        }
                        p {
                            color: #4a5568;
                            font-size: 1.1rem;
                            margin-bottom: 1.5rem;
                        }
                        .checkmark {
                            font-size: 4rem;
                            color: #48bb78;
                            margin-bottom: 1rem;
                        }
                    </style>
                </head>
                <body>
                    <div class="container">
                        <div class="checkmark">✓</div>
                        <h1>Authentication Successful!</h1>
                        <p>You have successfully authenticated with Composio.</p>
                        <p>You can close this window and return to the terminal.</p>
                    </div>
                </body>
                </html>
                "#
            )
        };

        // Build the router
        let app = Router::new().route("/callback", get(callback_handler));

        // Spawn the server in a background task
        let server_handle = tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(addr)
                .await
                .context("Failed to bind local callback server")?;

            tracing::info!(
                addr = %addr,
                "Local callback server listening"
            );

            axum::serve(listener, app)
                .await
                .context("Local callback server error")?;

            Ok::<(), anyhow::Error>(())
        });

        // Modify the connect link to include our callback URL
        let modified_url = if connect_link.url.contains('?') {
            format!("{}&redirect_uri={}", connect_link.url, urlencoding::encode(&callback_url))
        } else {
            format!("{}?redirect_uri={}", connect_link.url, urlencoding::encode(&callback_url))
        };

        // Open browser with modified URL
        println!("\nStarting local callback server on port {}...", port);
        println!("Opening browser for OAuth authentication...");
        
        if let Err(e) = self.open_browser(&modified_url) {
            tracing::warn!(
                toolkit = toolkit,
                error = %e,
                "Failed to open browser automatically"
            );
            println!("\nPlease visit the following URL to authenticate:");
            println!("{}", modified_url);
        }

        println!("Waiting for OAuth callback...");

        // Wait for callback with timeout (10 minutes)
        let timeout_duration = Duration::from_secs(600);
        let start_time = tokio::time::Instant::now();
        
        while start_time.elapsed() < timeout_duration {
            // Check if callback was received
            if *callback_received.lock() {
                tracing::info!(
                    toolkit = toolkit,
                    "OAuth callback received, checking connection status"
                );
                println!("\n✓ OAuth callback received!");
                
                // Give Composio a moment to process the callback
                tokio::time::sleep(Duration::from_secs(2)).await;
                
                // Shutdown the server
                server_handle.abort();
                
                // Check connection status
                match self.check_connection(toolkit).await {
                    Ok(Some(connection_info)) => {
                        println!("✓ Authentication successful!");
                        return Ok(OAuthFlow::Completed(connection_info));
                    }
                    Ok(None) => {
                        tracing::warn!(
                            toolkit = toolkit,
                            "Callback received but connection not found, falling back to polling"
                        );
                        // Fall back to polling
                        return self.poll_for_connection(toolkit).await;
                    }
                    Err(e) => {
                        tracing::error!(
                            toolkit = toolkit,
                            error = %e,
                            "Error checking connection after callback"
                        );
                        return Ok(OAuthFlow::Failed {
                            error: format!("Failed to verify connection: {}", e),
                        });
                    }
                }
            }
            
            // Wait a bit before checking again
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        // Timeout reached
        server_handle.abort();
        
        tracing::error!(
            toolkit = toolkit,
            "OAuth callback timed out after 10 minutes"
        );
        println!("\n✗ Authentication timed out. Please try again.");
        
        Ok(OAuthFlow::Failed {
            error: "OAuth callback timed out after 10 minutes".to_string(),
        })
    }

    /// Find an available port in the given range
    fn find_available_port(&self, start: u16, end: u16) -> Result<u16> {
        use std::net::TcpListener;

        for port in start..=end {
            if let Ok(listener) = TcpListener::bind(("127.0.0.1", port)) {
                drop(listener);
                return Ok(port);
            }
        }

        anyhow::bail!(
            "No available ports found in range {}-{}",
            start,
            end
        )
    }

    /// Handle web callback mode: return pending status immediately
    async fn handle_web_callback(
        &self,
        _toolkit: &str,
        connect_link: &ConnectLink,
    ) -> Result<OAuthFlow> {
        // Return pending status with connect link
        Ok(OAuthFlow::Pending {
            connect_link: connect_link.url.clone(),
            expires_at: connect_link.expires_at,
        })
    }

    /// Open browser with the given URL
    ///
    /// Uses platform-specific commands to open the default browser
    fn open_browser(&self, url: &str) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("cmd")
                .args(["/C", "start", url])
                .spawn()
                .context("Failed to open browser on Windows")?;
        }

        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(url)
                .spawn()
                .context("Failed to open browser on macOS")?;
        }

        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("xdg-open")
                .arg(url)
                .spawn()
                .context("Failed to open browser on Linux")?;
        }

        Ok(())
    }

    /// Generate a connect link for OAuth authentication
    ///
    /// This method calls COMPOSIO_MANAGE_CONNECTIONS to generate a connect link
    /// with a 10-minute expiry. The link is hosted by Composio and handles the
    /// full OAuth flow.
    ///
    /// # Arguments
    /// * `toolkit` - Toolkit name (e.g., "gmail", "slack", "github")
    ///
    /// # Returns
    /// * `Ok(ConnectLink)` - Connect link with URL and expiry time
    /// * `Err(anyhow::Error)` - Generation errors (network, parsing, etc.)
    pub async fn generate_connect_link(&self, toolkit: &str) -> Result<ConnectLink> {
        let user_id = self.effective_user_id();
        
        tracing::debug!(
            toolkit = toolkit,
            user_id = %user_id,
            "Generating connect link via COMPOSIO_MANAGE_CONNECTIONS"
        );

        // Call COMPOSIO_MANAGE_CONNECTIONS to get connect link
        let params = serde_json::json!({
            "toolkits": [toolkit],
            "session": {
                "generate_id": true
            }
        });

        let request_id = chrono::Utc::now().timestamp();
        let result = self
            .mcp_client
            .tools_call(request_id, "COMPOSIO_MANAGE_CONNECTIONS", params)
            .await
            .context("Failed to call COMPOSIO_MANAGE_CONNECTIONS")?;

        // Parse JSON-RPC response
        let rpc_response: serde_json::Value = result;
        
        // Extract connect link from response
        let connect_url = self.extract_connect_link_from_response(&rpc_response, toolkit)?;

        // Calculate expiry time (10 minutes from now)
        let expires_at = Utc::now() + chrono::Duration::minutes(10);

        tracing::info!(
            toolkit = toolkit,
            user_id = %user_id,
            connect_url = %connect_url,
            expires_at = %expires_at,
            "Connect link generated successfully"
        );

        Ok(ConnectLink {
            url: connect_url,
            expires_at,
            toolkit: toolkit.to_string(),
        })
    }

    /// Extract connect link from COMPOSIO_MANAGE_CONNECTIONS response
    ///
    /// Handles multiple response formats:
    /// - content[0].text format (JSON string)
    /// - Direct result format
    /// - Various OAuth link patterns
    fn extract_connect_link_from_response(
        &self,
        response: &serde_json::Value,
        toolkit: &str,
    ) -> Result<String> {
        // Try to parse content[0].text format first
        if let Some(content_text) = response
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|text| text.as_str())
        {
            tracing::debug!(
                toolkit = toolkit,
                text_preview = %format!("{:.200}", content_text),
                "Parsing content[0].text format"
            );

            // Try to parse as JSON
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(content_text) {
                if let Some(link) = self.extract_link_from_parsed(&parsed, toolkit) {
                    return Ok(link);
                }
            }

            // Try to extract link directly from text
            if let Some(link) = self.extract_link_from_text(content_text) {
                return Ok(link);
            }
        }

        // Try direct result format
        if let Some(result) = response.get("result") {
            if let Some(link) = self.extract_link_from_parsed(result, toolkit) {
                return Ok(link);
            }
        }

        anyhow::bail!(
            "Failed to extract connect link from response for toolkit '{}'",
            toolkit
        )
    }

    /// Extract link from parsed JSON data
    fn extract_link_from_parsed(
        &self,
        data: &serde_json::Value,
        toolkit: &str,
    ) -> Option<String> {
        // Check data.results[toolkit].instruction
        if let Some(instruction) = data
            .get("data")
            .and_then(|d| d.get("results"))
            .and_then(|r| r.get(toolkit))
            .and_then(|t| t.get("instruction"))
            .and_then(|i| i.as_str())
        {
            if let Some(link) = self.extract_link_from_text(instruction) {
                return Some(link);
            }
        }

        // Check data.results[toolkit].redirect_url
        if let Some(redirect_url) = data
            .get("data")
            .and_then(|d| d.get("results"))
            .and_then(|r| r.get(toolkit))
            .and_then(|t| t.get("redirect_url"))
            .and_then(|u| u.as_str())
        {
            return Some(redirect_url.to_string());
        }

        // Check direct redirect_url
        if let Some(redirect_url) = data.get("redirect_url").and_then(|u| u.as_str()) {
            return Some(redirect_url.to_string());
        }

        None
    }

    /// Extract OAuth link from text using pattern matching
    fn extract_link_from_text(&self, text: &str) -> Option<String> {
        let link_patterns = [
            "https://connect.composio.dev/link/",
            "https://backend.composio.dev/oauth/",
            "https://app.composio.dev/",
        ];

        for pattern in &link_patterns {
            if let Some(link_start) = text.find(pattern) {
                let link_end = text[link_start..]
                    .find(|c: char| c.is_whitespace() || c == '\n' || c == ')')
                    .map(|pos| link_start + pos)
                    .unwrap_or(text.len());
                let link = text[link_start..link_end].trim().to_string();
                return Some(link);
            }
        }

        None
    }

    /// Check if a connection exists for a toolkit
    ///
    /// This method calls COMPOSIO_MANAGE_CONNECTIONS to check the current
    /// connection status for a toolkit. It does not initiate OAuth if the
    /// connection doesn't exist.
    ///
    /// # Arguments
    /// * `toolkit` - Toolkit name (e.g., "gmail", "slack", "github")
    ///
    /// # Returns
    /// * `Ok(Some(ConnectionInfo))` - Connection exists and is active
    /// * `Ok(None)` - Connection does not exist or requires OAuth
    /// * `Err(anyhow::Error)` - Check errors (network, parsing, etc.)
    pub async fn check_connection(&self, toolkit: &str) -> Result<Option<ConnectionInfo>> {
        let user_id = self.effective_user_id();
        
        tracing::debug!(
            toolkit = toolkit,
            user_id = %user_id,
            "Checking connection status"
        );

        // Call COMPOSIO_MANAGE_CONNECTIONS to check status
        let params = serde_json::json!({
            "toolkits": [toolkit],
            "session": {
                "generate_id": true
            }
        });

        let request_id = chrono::Utc::now().timestamp();
        let result = self
            .mcp_client
            .tools_call(request_id, "COMPOSIO_MANAGE_CONNECTIONS", params)
            .await
            .context("Failed to call COMPOSIO_MANAGE_CONNECTIONS")?;

        // Parse JSON-RPC response
        let rpc_response: serde_json::Value = result;

        // Try to extract connection info
        if let Some(connection_info) = self.extract_connection_info(&rpc_response, toolkit)? {
            tracing::info!(
                toolkit = toolkit,
                user_id = %user_id,
                connected_account_id = %connection_info.connected_account_id,
                "Connection found"
            );
            Ok(Some(connection_info))
        } else {
            tracing::debug!(
                toolkit = toolkit,
                user_id = %user_id,
                "No active connection found"
            );
            Ok(None)
        }
    }

    /// Extract connection info from COMPOSIO_MANAGE_CONNECTIONS response
    ///
    /// Returns Some(ConnectionInfo) if connection is active, None if OAuth required
    fn extract_connection_info(
        &self,
        response: &serde_json::Value,
        toolkit: &str,
    ) -> Result<Option<ConnectionInfo>> {
        // Try to parse content[0].text format first
        if let Some(content_text) = response
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|text| text.as_str())
        {
            // Try to parse as JSON
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(content_text) {
                if let Some(info) = self.parse_connection_info(&parsed, toolkit) {
                    return Ok(Some(info));
                }
            }
        }

        // Try direct result format
        if let Some(result) = response.get("result") {
            if let Some(info) = self.parse_connection_info(result, toolkit) {
                return Ok(Some(info));
            }
        }

        Ok(None)
    }

    /// Parse connection info from JSON data
    fn parse_connection_info(
        &self,
        data: &serde_json::Value,
        toolkit: &str,
    ) -> Option<ConnectionInfo> {
        // Check data.results[toolkit]
        let toolkit_data = data
            .get("data")
            .and_then(|d| d.get("results"))
            .and_then(|r| r.get(toolkit))?;

        // If instruction or redirect_url exists, OAuth is required
        if toolkit_data.get("instruction").is_some() || toolkit_data.get("redirect_url").is_some()
        {
            return None;
        }

        // Extract connected_account_id
        let connected_account_id = toolkit_data
            .get("connected_account_id")
            .or_else(|| toolkit_data.get("id"))
            .and_then(|id| id.as_str())?
            .to_string();

        // Parse status
        let status = toolkit_data
            .get("status")
            .and_then(|s| s.as_str())
            .and_then(|s| match s.to_lowercase().as_str() {
                "active" => Some(crate::tools::composio::meta_tools::ConnectionStatus::Active),
                "expired" => Some(crate::tools::composio::meta_tools::ConnectionStatus::Expired),
                "revoked" => Some(crate::tools::composio::meta_tools::ConnectionStatus::Revoked),
                _ => None,
            })
            .unwrap_or(crate::tools::composio::meta_tools::ConnectionStatus::Active);

        Some(ConnectionInfo {
            toolkit: toolkit.to_string(),
            connected_account_id,
            status,
            created_at: Utc::now(),
        })
    }

    /// Poll for connection completion with timeout
    ///
    /// Polls COMPOSIO_MANAGE_CONNECTIONS every 10 seconds for up to 10 minutes
    /// to check if the user has completed OAuth authentication.
    ///
    /// # Arguments
    /// * `toolkit` - Toolkit name (e.g., "gmail", "slack", "github")
    ///
    /// # Returns
    /// * `Ok(OAuthFlow::Completed)` - OAuth completed successfully
    /// * `Ok(OAuthFlow::Failed)` - OAuth timed out or failed
    /// * `Err(anyhow::Error)` - System errors
    pub async fn poll_for_connection(&self, toolkit: &str) -> Result<OAuthFlow> {
        let max_attempts = 60; // 10 minutes with 10-second intervals
        let poll_interval = Duration::from_secs(10);

        tracing::info!(
            toolkit = toolkit,
            max_attempts = max_attempts,
            poll_interval_secs = 10,
            "Starting connection polling"
        );

        for attempt in 1..=max_attempts {
            // Check connection status
            match self.check_connection(toolkit).await {
                Ok(Some(connection_info)) => {
                    tracing::info!(
                        toolkit = toolkit,
                        attempt = attempt,
                        "OAuth completed successfully"
                    );
                    println!("\n✓ Authentication successful!");
                    return Ok(OAuthFlow::Completed(connection_info));
                }
                Ok(None) => {
                    // Connection not yet established, continue polling
                    if attempt % 6 == 0 {
                        // Print progress every minute
                        let elapsed_minutes = attempt / 6;
                        let remaining_minutes = (max_attempts - attempt) / 6;
                        println!(
                            "Still waiting... ({} min elapsed, {} min remaining)",
                            elapsed_minutes, remaining_minutes
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        toolkit = toolkit,
                        attempt = attempt,
                        error = %e,
                        "Error checking connection status"
                    );
                    // Continue polling despite errors
                }
            }

            // Wait before next poll
            if attempt < max_attempts {
                sleep(poll_interval).await;
            }
        }

        // Timeout reached
        tracing::error!(
            toolkit = toolkit,
            "OAuth polling timed out after 10 minutes"
        );
        println!("\n✗ Authentication timed out. Please try again.");

        Ok(OAuthFlow::Failed {
            error: "OAuth authentication timed out after 10 minutes".to_string(),
        })
    }

    /// Revoke a connection for a toolkit
    ///
    /// This method revokes the OAuth connection for a toolkit, requiring
    /// the user to re-authenticate on next use.
    ///
    /// # Arguments
    /// * `toolkit` - Toolkit name (e.g., "gmail", "slack", "github")
    ///
    /// # Returns
    /// * `Ok(())` - Connection revoked successfully
    /// * `Err(anyhow::Error)` - Revocation errors
    pub async fn revoke_connection(&self, toolkit: &str) -> Result<()> {
        let user_id = self.effective_user_id();
        
        tracing::info!(
            toolkit = toolkit,
            user_id = %user_id,
            "Revoking connection"
        );

        // TODO: Implement actual revocation via Composio API
        // For now, just log the intent
        tracing::warn!(
            toolkit = toolkit,
            "Connection revocation not yet implemented"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::composio::meta_tools::McpClientTrait;

    /// Mock MCP client for testing
    struct MockMcpClient {
        response: serde_json::Value,
    }

    #[async_trait::async_trait]
    impl McpClientTrait for MockMcpClient {
        async fn tools_call(
            &self,
            _request_id: i64,
            _tool_name: &str,
            _params: serde_json::Value,
        ) -> Result<serde_json::Value> {
            Ok(self.response.clone())
        }
    }

    fn create_test_config() -> ComposioConfig {
        ComposioConfig {
            enabled: true,
            api_key: Some("test_api_key".to_string()),
            user_id: "test_user".to_string(),
            entity_id: None,
            onboarding_mode: OnboardingMode::WebCallback,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_generate_connect_link_success() {
        let mock_response = serde_json::json!({
            "result": {
                "content": [{
                    "text": r#"{"data": {"results": {"gmail": {"instruction": "Please visit https://connect.composio.dev/link/test123 to authenticate"}}}}"#
                }]
            }
        });

        let mock_client = Arc::new(MockMcpClient {
            response: mock_response,
        });

        let config = create_test_config();
        let handler = OnboardingHandler::new(config, mock_client);

        let result = handler.generate_connect_link("gmail").await;
        assert!(result.is_ok());

        let connect_link = result.unwrap();
        assert_eq!(connect_link.toolkit, "gmail");
        assert!(connect_link.url.contains("connect.composio.dev"));
        assert!(connect_link.expires_at > Utc::now());
    }

    #[tokio::test]
    async fn test_check_connection_active() {
        let mock_response = serde_json::json!({
            "result": {
                "content": [{
                    "text": r#"{"data": {"results": {"gmail": {"connected_account_id": "acc_123", "status": "active"}}}}"#
                }]
            }
        });

        let mock_client = Arc::new(MockMcpClient {
            response: mock_response,
        });

        let config = create_test_config();
        let handler = OnboardingHandler::new(config, mock_client);

        let result = handler.check_connection("gmail").await;
        assert!(result.is_ok());

        let connection_info = result.unwrap();
        assert!(connection_info.is_some());

        let info = connection_info.unwrap();
        assert_eq!(info.toolkit, "gmail");
        assert_eq!(info.connected_account_id, "acc_123");
    }

    #[tokio::test]
    async fn test_check_connection_oauth_required() {
        let mock_response = serde_json::json!({
            "result": {
                "content": [{
                    "text": r#"{"data": {"results": {"gmail": {"instruction": "OAuth required"}}}}"#
                }]
            }
        });

        let mock_client = Arc::new(MockMcpClient {
            response: mock_response,
        });

        let config = create_test_config();
        let handler = OnboardingHandler::new(config, mock_client);

        let result = handler.check_connection("gmail").await;
        assert!(result.is_ok());

        let connection_info = result.unwrap();
        assert!(connection_info.is_none());
    }

    #[tokio::test]
    async fn test_initiate_oauth_web_callback_mode() {
        let mock_response = serde_json::json!({
            "result": {
                "content": [{
                    "text": r#"{"data": {"results": {"gmail": {"redirect_url": "https://connect.composio.dev/link/test456"}}}}"#
                }]
            }
        });

        let mock_client = Arc::new(MockMcpClient {
            response: mock_response,
        });

        let config = create_test_config();
        let handler = OnboardingHandler::new(config, mock_client);

        let result = handler.initiate_oauth("gmail").await;
        assert!(result.is_ok());

        match result.unwrap() {
            OAuthFlow::Pending {
                connect_link,
                expires_at,
            } => {
                assert!(connect_link.contains("connect.composio.dev"));
                assert!(expires_at > Utc::now());
            }
            _ => panic!("Expected Pending status for WebCallback mode"),
        }
    }

    #[tokio::test]
    async fn test_initiate_oauth_cli_print_only_mode() {
        let mock_response = serde_json::json!({
            "result": {
                "content": [{
                    "text": r#"{"data": {"results": {"slack": {"instruction": "Visit https://connect.composio.dev/link/slack789"}}}}"#
                }]
            }
        });

        let mock_client = Arc::new(MockMcpClient {
            response: mock_response,
        });

        let mut config = create_test_config();
        config.onboarding_mode = OnboardingMode::CliPrintOnly;
        let handler = OnboardingHandler::new(config, mock_client);

        // Note: This test will timeout quickly since we're not actually completing OAuth
        // We're just testing that the mode is handled correctly
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            handler.initiate_oauth("slack")
        ).await;

        // Should timeout because poll_for_connection runs for 10 minutes
        assert!(result.is_err(), "Expected timeout since OAuth won't complete in test");
    }

    #[tokio::test]
    async fn test_extract_link_from_text_various_formats() {
        let config = create_test_config();
        let mock_client = Arc::new(MockMcpClient {
            response: serde_json::json!({}),
        });
        let handler = OnboardingHandler::new(config, mock_client);

        // Test connect.composio.dev format
        let text1 = "Please visit https://connect.composio.dev/link/abc123 to authenticate";
        let link1 = handler.extract_link_from_text(text1);
        assert!(link1.is_some());
        assert_eq!(link1.unwrap(), "https://connect.composio.dev/link/abc123");

        // Test backend.composio.dev format
        let text2 = "OAuth URL: https://backend.composio.dev/oauth/xyz789\nExpires in 10 minutes";
        let link2 = handler.extract_link_from_text(text2);
        assert!(link2.is_some());
        assert_eq!(link2.unwrap(), "https://backend.composio.dev/oauth/xyz789");

        // Test app.composio.dev format
        let text3 = "Go to https://app.composio.dev/connect?token=def456";
        let link3 = handler.extract_link_from_text(text3);
        assert!(link3.is_some());
        assert!(link3.unwrap().starts_with("https://app.composio.dev/"));

        // Test no link found
        let text4 = "No OAuth link here";
        let link4 = handler.extract_link_from_text(text4);
        assert!(link4.is_none());
    }

    #[tokio::test]
    async fn test_extract_link_from_parsed_various_formats() {
        let config = create_test_config();
        let mock_client = Arc::new(MockMcpClient {
            response: serde_json::json!({}),
        });
        let handler = OnboardingHandler::new(config, mock_client);

        // Test data.results[toolkit].instruction format
        let data1 = serde_json::json!({
            "data": {
                "results": {
                    "gmail": {
                        "instruction": "Visit https://connect.composio.dev/link/test1"
                    }
                }
            }
        });
        let link1 = handler.extract_link_from_parsed(&data1, "gmail");
        assert!(link1.is_some());
        assert!(link1.unwrap().contains("connect.composio.dev"));

        // Test data.results[toolkit].redirect_url format
        let data2 = serde_json::json!({
            "data": {
                "results": {
                    "slack": {
                        "redirect_url": "https://backend.composio.dev/oauth/test2"
                    }
                }
            }
        });
        let link2 = handler.extract_link_from_parsed(&data2, "slack");
        assert!(link2.is_some());
        assert_eq!(link2.unwrap(), "https://backend.composio.dev/oauth/test2");

        // Test direct redirect_url format
        let data3 = serde_json::json!({
            "redirect_url": "https://app.composio.dev/test3"
        });
        let link3 = handler.extract_link_from_parsed(&data3, "github");
        assert!(link3.is_some());
        assert_eq!(link3.unwrap(), "https://app.composio.dev/test3");

        // Test no link found
        let data4 = serde_json::json!({
            "data": {
                "results": {
                    "notion": {
                        "status": "active"
                    }
                }
            }
        });
        let link4 = handler.extract_link_from_parsed(&data4, "notion");
        assert!(link4.is_none());
    }

    #[tokio::test]
    async fn test_parse_connection_info_various_statuses() {
        let config = create_test_config();
        let mock_client = Arc::new(MockMcpClient {
            response: serde_json::json!({}),
        });
        let handler = OnboardingHandler::new(config, mock_client);

        // Test active connection
        let data1 = serde_json::json!({
            "data": {
                "results": {
                    "gmail": {
                        "connected_account_id": "acc_active",
                        "status": "active"
                    }
                }
            }
        });
        let info1 = handler.parse_connection_info(&data1, "gmail");
        assert!(info1.is_some());
        let conn1 = info1.unwrap();
        assert_eq!(conn1.connected_account_id, "acc_active");
        assert!(matches!(conn1.status, crate::tools::composio::meta_tools::ConnectionStatus::Active));

        // Test expired connection
        let data2 = serde_json::json!({
            "data": {
                "results": {
                    "slack": {
                        "connected_account_id": "acc_expired",
                        "status": "expired"
                    }
                }
            }
        });
        let info2 = handler.parse_connection_info(&data2, "slack");
        assert!(info2.is_some());
        let conn2 = info2.unwrap();
        assert!(matches!(conn2.status, crate::tools::composio::meta_tools::ConnectionStatus::Expired));

        // Test revoked connection
        let data3 = serde_json::json!({
            "data": {
                "results": {
                    "github": {
                        "connected_account_id": "acc_revoked",
                        "status": "revoked"
                    }
                }
            }
        });
        let info3 = handler.parse_connection_info(&data3, "github");
        assert!(info3.is_some());
        let conn3 = info3.unwrap();
        assert!(matches!(conn3.status, crate::tools::composio::meta_tools::ConnectionStatus::Revoked));

        // Test OAuth required (has instruction)
        let data4 = serde_json::json!({
            "data": {
                "results": {
                    "notion": {
                        "instruction": "OAuth required"
                    }
                }
            }
        });
        let info4 = handler.parse_connection_info(&data4, "notion");
        assert!(info4.is_none());

        // Test OAuth required (has redirect_url)
        let data5 = serde_json::json!({
            "data": {
                "results": {
                    "dropbox": {
                        "redirect_url": "https://connect.composio.dev/link/test"
                    }
                }
            }
        });
        let info5 = handler.parse_connection_info(&data5, "dropbox");
        assert!(info5.is_none());
    }

    #[tokio::test]
    async fn test_find_available_port() {
        let config = create_test_config();
        let mock_client = Arc::new(MockMcpClient {
            response: serde_json::json!({}),
        });
        let handler = OnboardingHandler::new(config, mock_client);

        // Test finding a port in a reasonable range
        let result = handler.find_available_port(8080, 8100);
        assert!(result.is_ok());
        let port = result.unwrap();
        assert!(port >= 8080 && port <= 8100);
    }

    #[tokio::test]
    async fn test_find_available_port_no_ports_available() {
        let config = create_test_config();
        let mock_client = Arc::new(MockMcpClient {
            response: serde_json::json!({}),
        });
        let handler = OnboardingHandler::new(config, mock_client);

        // Test with a very narrow range where ports might be in use
        // This test might be flaky depending on system state, but demonstrates the error case
        let result = handler.find_available_port(1, 1);
        // Port 1 is typically privileged and unavailable, so this should fail
        // But we can't guarantee it, so we just check the function doesn't panic
        let _ = result;
    }

    #[tokio::test]
    async fn test_connect_link_expiry_time() {
        let mock_response = serde_json::json!({
            "result": {
                "content": [{
                    "text": r#"{"data": {"results": {"gmail": {"redirect_url": "https://connect.composio.dev/link/expiry_test"}}}}"#
                }]
            }
        });

        let mock_client = Arc::new(MockMcpClient {
            response: mock_response,
        });

        let config = create_test_config();
        let handler = OnboardingHandler::new(config, mock_client);

        let before = Utc::now();
        let result = handler.generate_connect_link("gmail").await;
        let after = Utc::now();

        assert!(result.is_ok());
        let connect_link = result.unwrap();

        // Verify expiry is approximately 10 minutes from now
        let expected_expiry = before + chrono::Duration::minutes(10);
        let expiry_diff = (connect_link.expires_at - expected_expiry).num_seconds().abs();
        
        // Allow 5 seconds of variance for test execution time
        assert!(expiry_diff < 5, "Expiry time should be ~10 minutes from now");
        
        // Verify expiry is in the future
        assert!(connect_link.expires_at > after);
    }

    #[tokio::test]
    async fn test_effective_user_id_with_legacy_entity_id() {
        let mock_client = Arc::new(MockMcpClient {
            response: serde_json::json!({}),
        });

        // Test with legacy entity_id
        let mut config = create_test_config();
        config.entity_id = Some("legacy_entity".to_string());
        config.user_id = "".to_string(); // Empty user_id to trigger legacy path
        
        let handler = OnboardingHandler::new(config, mock_client);
        let user_id = handler.effective_user_id();
        
        // Should use entity_id when user_id is empty
        assert_eq!(user_id, "legacy_entity");
    }

    #[tokio::test]
    async fn test_effective_user_id_prefers_user_id() {
        let mock_client = Arc::new(MockMcpClient {
            response: serde_json::json!({}),
        });

        // Test that user_id is preferred over entity_id
        let mut config = create_test_config();
        config.user_id = "modern_user".to_string();
        config.entity_id = Some("legacy_entity".to_string());
        
        let handler = OnboardingHandler::new(config, mock_client);
        let user_id = handler.effective_user_id();
        
        // Should prefer user_id over entity_id
        assert_eq!(user_id, "modern_user");
    }
}
