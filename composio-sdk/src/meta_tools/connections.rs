//! Connection Manager Implementation
//!
//! Native Rust implementation of COMPOSIO_MANAGE_CONNECTIONS meta tool.
//! Handles OAuth and API key authentication management.

use crate::client::ComposioClient;
use crate::error::ComposioError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Connection status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConnectionStatus {
    Active,
    Initiated,
    Expired,
    Failed,
    Inactive,
}

/// Connected account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedAccount {
    /// Account ID
    pub id: String,
    
    /// Toolkit slug
    pub toolkit: String,
    
    /// Connection status
    pub status: ConnectionStatus,
    
    /// User ID
    pub user_id: String,
    
    /// Created timestamp
    pub created_at: String,
    
    /// Updated timestamp
    pub updated_at: String,
}

/// Authorization link response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthLink {
    /// Redirect URL for OAuth flow
    pub redirect_url: String,
    
    /// Link token
    pub link_token: String,
    
    /// Expiration timestamp
    pub expires_at: String,
    
    /// Connected account ID (if already exists)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connected_account_id: Option<String>,
}

/// Connection manager
pub struct ConnectionManager {
    client: Arc<ComposioClient>,
}

impl ConnectionManager {
    /// Create a new connection manager instance
    ///
    /// # Arguments
    ///
    /// * `client` - Composio client instance
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::{ComposioClient, meta_tools::ConnectionManager};
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ComposioClient::builder()
    ///     .api_key("your-api-key")
    ///     .build()?;
    ///
    /// let manager = ConnectionManager::new(Arc::new(client));
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(client: Arc<ComposioClient>) -> Self {
        Self { client }
    }

    /// List all connected accounts for a session
    ///
    /// # Arguments
    ///
    /// * `session_id` - Session ID
    ///
    /// # Returns
    ///
    /// Vector of connected accounts
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::{ComposioClient, meta_tools::ConnectionManager};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = Arc::new(ComposioClient::builder().api_key("key").build()?);
    /// let manager = ConnectionManager::new(client);
    /// let accounts = manager.list_connections("session_123").await?;
    ///
    /// for account in accounts {
    ///     println!("{}: {:?}", account.toolkit, account.status);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_connections(
        &self,
        session_id: &str,
    ) -> Result<Vec<ConnectedAccount>, ComposioError> {
        let url = format!(
            "{}/tool_router/session/{}/toolkits",
            self.client.base_url(),
            session_id
        );

        let response = self
            .client
            .http_client()
            .get(&url)
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

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ComposioError::SerializationError(e.to_string()))?;

        let accounts = data["data"]["items"]
            .as_array()
            .ok_or_else(|| {
                ComposioError::SerializationError("Invalid response format".to_string())
            })?
            .iter()
            .filter_map(|item| {
                item["connected_account"]
                    .as_object()
                    .and_then(|acc| serde_json::from_value(serde_json::Value::Object(acc.clone())).ok())
            })
            .collect();

        Ok(accounts)
    }

    /// Create authorization link for a toolkit
    ///
    /// # Arguments
    ///
    /// * `session_id` - Session ID
    /// * `toolkit` - Toolkit slug (e.g., "github", "gmail")
    /// * `callback_url` - Optional callback URL after OAuth
    ///
    /// # Returns
    ///
    /// Authorization link with redirect URL
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::{ComposioClient, meta_tools::ConnectionManager};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = Arc::new(ComposioClient::builder().api_key("key").build()?);
    /// let manager = ConnectionManager::new(client);
    /// let link = manager.create_auth_link(
    ///     "session_123",
    ///     "github",
    ///     Some("https://myapp.com/callback"),
    /// ).await?;
    ///
    /// println!("Redirect user to: {}", link.redirect_url);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_auth_link(
        &self,
        session_id: &str,
        toolkit: &str,
        callback_url: Option<&str>,
    ) -> Result<AuthLink, ComposioError> {
        let url = format!(
            "{}/tool_router/session/{}/link",
            self.client.base_url(),
            session_id
        );

        let mut body = serde_json::json!({
            "toolkit": toolkit,
        });

        if let Some(callback) = callback_url {
            body["callback_url"] = serde_json::json!(callback);
        }

        let response = self
            .client
            .http_client()
            .post(&url)
            .json(&body)
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

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ComposioError::SerializationError(e.to_string()))?;

        let link: AuthLink = serde_json::from_value(data["data"].clone())
            .map_err(|e| ComposioError::SerializationError(e.to_string()))?;

        Ok(link)
    }

    /// Check if a toolkit is connected
    ///
    /// # Arguments
    ///
    /// * `session_id` - Session ID
    /// * `toolkit` - Toolkit slug
    ///
    /// # Returns
    ///
    /// True if toolkit is connected and active
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::{ComposioClient, meta_tools::ConnectionManager};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = Arc::new(ComposioClient::builder().api_key("key").build()?);
    /// let manager = ConnectionManager::new(client);
    /// let is_connected = manager.is_connected("session_123", "github").await?;
    ///
    /// if !is_connected {
    ///     println!("GitHub is not connected. Please authenticate.");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn is_connected(&self, session_id: &str, toolkit: &str) -> Result<bool, ComposioError> {
        let accounts = self.list_connections(session_id).await?;
        
        Ok(accounts
            .iter()
            .any(|acc| acc.toolkit == toolkit && acc.status == ConnectionStatus::Active))
    }

    /// Get connection status for a specific toolkit
    ///
    /// # Arguments
    ///
    /// * `session_id` - Session ID
    /// * `toolkit` - Toolkit slug
    ///
    /// # Returns
    ///
    /// Connection status or None if not found
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::{ComposioClient, meta_tools::ConnectionManager};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = Arc::new(ComposioClient::builder().api_key("key").build()?);
    /// let manager = ConnectionManager::new(client);
    /// let status = manager.get_connection_status("session_123", "github").await?;
    ///
    /// match status {
    ///     Some(status) => println!("GitHub status: {:?}", status),
    ///     None => println!("GitHub not connected"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_connection_status(
        &self,
        session_id: &str,
        toolkit: &str,
    ) -> Result<Option<ConnectionStatus>, ComposioError> {
        let accounts = self.list_connections(session_id).await?;
        
        Ok(accounts
            .iter()
            .find(|acc| acc.toolkit == toolkit)
            .map(|acc| acc.status.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_status_serialization() {
        let status = ConnectionStatus::Active;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"ACTIVE\"");

        let deserialized: ConnectionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ConnectionStatus::Active);
    }

    #[test]
    fn test_connected_account_serialization() {
        let account = ConnectedAccount {
            id: "ca_123".to_string(),
            toolkit: "github".to_string(),
            status: ConnectionStatus::Active,
            user_id: "user_123".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&account).unwrap();
        assert!(json.contains("ca_123"));
        assert!(json.contains("github"));
        assert!(json.contains("ACTIVE"));

        let deserialized: ConnectedAccount = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "ca_123");
        assert_eq!(deserialized.status, ConnectionStatus::Active);
    }

    #[test]
    fn test_auth_link_serialization() {
        let link = AuthLink {
            redirect_url: "https://auth.composio.dev/...".to_string(),
            link_token: "lt_abc123".to_string(),
            expires_at: "2024-01-01T01:00:00Z".to_string(),
            connected_account_id: Some("ca_123".to_string()),
        };

        let json = serde_json::to_string(&link).unwrap();
        assert!(json.contains("redirect_url"));
        assert!(json.contains("lt_abc123"));
        assert!(json.contains("ca_123"));
    }

    #[test]
    fn test_auth_link_without_account_id() {
        let link = AuthLink {
            redirect_url: "https://auth.composio.dev/...".to_string(),
            link_token: "lt_abc123".to_string(),
            expires_at: "2024-01-01T01:00:00Z".to_string(),
            connected_account_id: None,
        };

        let json = serde_json::to_string(&link).unwrap();
        assert!(!json.contains("connected_account_id"));
    }
}
