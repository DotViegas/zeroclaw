// Composio REST API Client — shared client for v2/v3 API access
//
// This client is used by both ComposioTool and onboarding flows to avoid
// circular dependencies and code duplication.

use anyhow::Context;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

const COMPOSIO_API_BASE_V2: &str = "https://backend.composio.dev/api/v2";
const COMPOSIO_API_BASE_V3: &str = "https://backend.composio.dev/api/v3";
const COMPOSIO_TOOL_VERSION_LATEST: &str = "latest";

/// Shared REST client for Composio API (v2/v3)
pub struct ComposioRestClient {
    api_key: String,
    client: Client,
}

impl ComposioRestClient {
    /// Create a new Composio REST client
    pub fn new(api_key: String) -> Self {
        let client = crate::config::build_runtime_proxy_client_with_timeouts("tool.composio", 60, 10);
        Self { api_key, client }
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
            "auth_config_id": auth_config_id,
            "user_id": entity_id,
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
            anyhow::bail!("Composio v3 connect failed: {err}");
        }

        let result: serde_json::Value = resp
            .json()
            .await
            .context("Failed to decode Composio v3 connect response")?;
        let redirect_url = extract_redirect_url(&result)
            .ok_or_else(|| anyhow::anyhow!("No redirect URL in Composio v3 response"))?;
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
                 \nTo fix this:\
                 \n1. Visit https://app.composio.dev/apps and search for '{app_name}'\
                 \n2. Click 'Add Integration' or 'Configure' for {app_name}\
                 \n3. Follow the setup wizard to create an auth config\
                 \n4. Once created, retry action='connect' with app='{app_name}'"
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
    result
        .get("redirect_url")
        .and_then(|v| v.as_str())
        .or_else(|| result.get("redirectUrl").and_then(|v| v.as_str()))
        .or_else(|| {
            result
                .get("data")
                .and_then(|v| v.get("redirect_url"))
                .and_then(|v| v.as_str())
        })
        .map(ToString::to_string)
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
}
