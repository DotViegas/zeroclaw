// Composio REST API Blocking Client for Wizard
//
// This module provides a blocking HTTP client for Composio's REST API,
// specifically designed for use in the onboarding wizard which runs in
// a blocking context.

use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Blocking REST client for Composio API (wizard use only)
pub struct ComposioRestBlockingClient {
    api_key: String,
    client: Client,
}

#[derive(Debug, Serialize)]
struct GenerateMcpUrlRequest {
    #[serde(rename = "toolkits")]
    toolkits: Vec<String>,
    #[serde(rename = "userId")]
    user_id: String,
}

#[derive(Debug, Deserialize)]
struct GenerateMcpUrlResponse {
    #[serde(rename = "url")]
    url: String,
}

#[derive(Debug, Deserialize)]
struct ConnectionLinkResponse {
    #[serde(rename = "redirectUrl")]
    redirect_url: Option<String>,
    #[serde(rename = "redirect_url")]
    redirect_url_alt: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ConnectedAccount {
    status: String,
}

#[derive(Debug, Deserialize)]
struct ConnectedAccountsResponse {
    items: Vec<ConnectedAccount>,
}

#[derive(Debug, Deserialize)]
struct AuthConfigsResponse {
    items: Vec<AuthConfig>,
}

#[derive(Debug, Deserialize)]
struct AuthConfig {
    id: String,
    #[serde(default)]
    enabled: Option<bool>,
}

impl ComposioRestBlockingClient {
    /// Create a new blocking REST client
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { api_key, client }
    }

    /// Generate an MCP URL for the given toolkits
    ///
    /// # Arguments
    /// * `toolkits` - List of toolkit slugs (e.g., ["gmail", "github", "slack"])
    /// * `user_id` - User/entity identifier
    ///
    /// # Returns
    /// The generated MCP URL
    pub fn generate_mcp_url(&self, toolkits: Vec<String>, user_id: &str) -> Result<String> {
        // Use v3 API endpoint (v1 is deprecated as of 410 Gone error)
        let url = "https://backend.composio.dev/api/v3/mcp/generate";

        let request_body = GenerateMcpUrlRequest {
            toolkits,
            user_id: user_id.to_string(),
        };

        let response = self
            .client
            .post(url)
            .header("X-API-Key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .context("Failed to send MCP URL generation request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!(
                "Failed to generate MCP URL (status {}): {}",
                status,
                body
            );
        }

        let result: GenerateMcpUrlResponse = response
            .json()
            .context("Failed to parse MCP URL generation response")?;

        Ok(result.url)
    }

    /// Get a connection URL for OAuth onboarding
    ///
    /// # Arguments
    /// * `app_name` - The app/toolkit slug (e.g., "gmail", "github")
    /// * `entity_id` - User/entity identifier
    ///
    /// # Returns
    /// The OAuth redirect URL
    pub fn get_connection_url(&self, app_name: &str, entity_id: &str) -> Result<String> {
        // First, resolve the auth_config_id for the app (v3 API requirement)
        let auth_config_id = self.resolve_auth_config_id(app_name)?;
        
        // Now create the connection link with auth_config_id and user_id
        let url = "https://backend.composio.dev/api/v3/connected_accounts/link";

        let request_body = serde_json::json!({
            "auth_config_id": auth_config_id,
            "user_id": entity_id,
        });

        let response = self
            .client
            .post(url)
            .header("X-API-Key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .context("Failed to send connection URL request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!(
                "Failed to get connection URL (status {}): {}",
                status,
                body
            );
        }

        let result: ConnectionLinkResponse = response
            .json()
            .context("Failed to parse connection URL response")?;

        // Try both field names (API inconsistency)
        result
            .redirect_url
            .or(result.redirect_url_alt)
            .context("No redirect URL in response")
    }

    /// Resolve auth config ID for a given app/toolkit (blocking version)
    ///
    /// # Arguments
    /// * `app_name` - The app/toolkit slug (e.g., "gmail", "github")
    ///
    /// # Returns
    /// The auth_config_id for the app
    fn resolve_auth_config_id(&self, app_name: &str) -> Result<String> {
        let url = "https://backend.composio.dev/api/v3/auth_configs";

        let response = self
            .client
            .get(url)
            .header("X-API-Key", &self.api_key)
            .query(&[
                ("toolkit_slug", app_name),
                ("show_disabled", "true"),
                ("limit", "25"),
            ])
            .send()
            .context("Failed to fetch auth configs")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!(
                "Failed to resolve auth config (status {}): {}",
                status,
                body
            );
        }

        let result: AuthConfigsResponse = response
            .json()
            .context("Failed to parse auth configs response")?;

        if result.items.is_empty() {
            anyhow::bail!(
                "No authentication configuration found for app '{}'.\n\
                To fix this:\n\
                1. Visit https://app.composio.dev/apps and search for '{}'\n\
                2. Click 'Add Integration' or 'Configure' for {}\n\
                3. Follow the setup wizard to create an auth config\n\
                4. Once created, retry connecting",
                app_name, app_name, app_name
            );
        }

        // Use the first auth config (or find enabled one)
        let auth_config = result
            .items
            .iter()
            .find(|c| c.enabled.unwrap_or(true))
            .or_else(|| result.items.first())
            .context("No auth config available")?;

        Ok(auth_config.id.clone())
    }

    /// Check if an active connection exists for the given app
    ///
    /// # Arguments
    /// * `app_name` - The app/toolkit slug (e.g., "gmail", "github")
    /// * `entity_id` - User/entity identifier
    ///
    /// # Returns
    /// true if an ACTIVE connection exists, false otherwise
    pub fn has_active_connection(&self, app_name: &str, entity_id: &str) -> Result<bool> {
        let url = format!(
            "https://backend.composio.dev/api/v3/connected_accounts?integrationId={}&entityId={}",
            urlencoding::encode(app_name),
            urlencoding::encode(entity_id)
        );

        let response = self
            .client
            .get(&url)
            .header("X-API-Key", &self.api_key)
            .send()
            .context("Failed to check connected accounts")?;

        if !response.status().is_success() {
            // Don't fail on error, just return false
            return Ok(false);
        }

        let result: ConnectedAccountsResponse = response
            .json()
            .context("Failed to parse connected accounts response")?;

        Ok(result
            .items
            .iter()
            .any(|a| a.status.eq_ignore_ascii_case("ACTIVE")))
    }

    /// Poll until connection is active or timeout
    ///
    /// # Arguments
    /// * `app_name` - The app/toolkit slug
    /// * `entity_id` - User/entity identifier
    /// * `timeout_secs` - Maximum time to wait
    ///
    /// # Returns
    /// Ok(()) if connection becomes active, Err on timeout or error
    pub fn poll_until_connected(
        &self,
        app_name: &str,
        entity_id: &str,
        timeout_secs: u64,
    ) -> Result<()> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        loop {
            if start.elapsed() > timeout {
                anyhow::bail!("Timeout waiting for {} connection", app_name);
            }

            if self.has_active_connection(app_name, entity_id)? {
                return Ok(());
            }

            // Wait 2 seconds before next check
            std::thread::sleep(Duration::from_secs(2));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_constructs_with_api_key() {
        let client = ComposioRestBlockingClient::new("test_key".to_string());
        assert_eq!(client.api_key, "test_key");
    }
}
