// Health check utilities for Composio MCP
//
// Provides functions to verify MCP connection and toolkit status.

use crate::composio::ComposioRestClient;
use std::sync::Arc;

/// Health check result for MCP connection
#[derive(Debug, Clone)]
pub struct McpHealthCheck {
    /// Whether MCP server is reachable
    pub server_reachable: bool,
    /// Number of tools available
    pub tools_count: usize,
    /// Connected toolkits
    pub connected_toolkits: Vec<String>,
    /// Disconnected toolkits
    pub disconnected_toolkits: Vec<String>,
    /// Error message if health check failed
    pub error: Option<String>,
}

impl McpHealthCheck {
    /// Check if MCP is healthy (server reachable and has tools)
    pub fn is_healthy(&self) -> bool {
        self.server_reachable && self.tools_count > 0 && self.error.is_none()
    }

    /// Get a human-readable status message
    pub fn status_message(&self) -> String {
        if let Some(error) = &self.error {
            return format!("❌ MCP Health Check Failed: {}", error);
        }

        if !self.server_reachable {
            return "❌ MCP server is not reachable".to_string();
        }

        if self.tools_count == 0 {
            return "⚠️  MCP server is reachable but no tools are available".to_string();
        }

        let mut parts = vec![format!("✅ MCP is healthy ({} tools available)", self.tools_count)];

        if !self.connected_toolkits.is_empty() {
            parts.push(format!(
                "Connected: {}",
                self.connected_toolkits.join(", ")
            ));
        }

        if !self.disconnected_toolkits.is_empty() {
            parts.push(format!(
                "⚠️  Disconnected: {}",
                self.disconnected_toolkits.join(", ")
            ));
        }

        parts.join("\n")
    }
}

/// Perform a health check on MCP connection
///
/// # Arguments
/// * `mcp_client` - The MCP client to check
/// * `rest_client` - REST client for checking toolkit connections
/// * `entity_id` - User/entity identifier
/// * `expected_toolkits` - List of toolkits that should be connected
pub async fn check_mcp_health(
    mcp_client: &crate::mcp::ComposioMcpClient,
    rest_client: Arc<ComposioRestClient>,
    entity_id: &str,
    expected_toolkits: &[String],
) -> McpHealthCheck {
    // Try to list tools from MCP server
    let tools_result = mcp_client.list_tools().await;

    match tools_result {
        Ok(tools) => {
            let tools_count = tools.len();

            // Check toolkit connections if we have expected toolkits
            let (connected, disconnected) = if !expected_toolkits.is_empty() {
                check_toolkit_connections(&rest_client, entity_id, expected_toolkits).await
            } else {
                (Vec::new(), Vec::new())
            };

            McpHealthCheck {
                server_reachable: true,
                tools_count,
                connected_toolkits: connected,
                disconnected_toolkits: disconnected,
                error: None,
            }
        }
        Err(e) => McpHealthCheck {
            server_reachable: false,
            tools_count: 0,
            connected_toolkits: Vec::new(),
            disconnected_toolkits: expected_toolkits.to_vec(),
            error: Some(e.to_string()),
        },
    }
}

/// Check which toolkits are connected
async fn check_toolkit_connections(
    rest_client: &ComposioRestClient,
    entity_id: &str,
    toolkits: &[String],
) -> (Vec<String>, Vec<String>) {
    let mut connected = Vec::new();
    let mut disconnected = Vec::new();

    for toolkit in toolkits {
        match rest_client
            .list_connected_accounts(Some(toolkit), Some(entity_id))
            .await
        {
            Ok(accounts) => {
                let has_active = accounts
                    .iter()
                    .any(|a| a.status.eq_ignore_ascii_case("ACTIVE"));

                if has_active {
                    connected.push(toolkit.clone());
                } else {
                    disconnected.push(toolkit.clone());
                }
            }
            Err(_) => {
                disconnected.push(toolkit.clone());
            }
        }
    }

    (connected, disconnected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_check_is_healthy_when_all_good() {
        let check = McpHealthCheck {
            server_reachable: true,
            tools_count: 10,
            connected_toolkits: vec!["gmail".to_string()],
            disconnected_toolkits: Vec::new(),
            error: None,
        };

        assert!(check.is_healthy());
    }

    #[test]
    fn health_check_is_unhealthy_when_server_unreachable() {
        let check = McpHealthCheck {
            server_reachable: false,
            tools_count: 0,
            connected_toolkits: Vec::new(),
            disconnected_toolkits: Vec::new(),
            error: Some("Connection refused".to_string()),
        };

        assert!(!check.is_healthy());
    }

    #[test]
    fn health_check_is_unhealthy_when_no_tools() {
        let check = McpHealthCheck {
            server_reachable: true,
            tools_count: 0,
            connected_toolkits: Vec::new(),
            disconnected_toolkits: Vec::new(),
            error: None,
        };

        assert!(!check.is_healthy());
    }

    #[test]
    fn health_check_status_message_shows_success() {
        let check = McpHealthCheck {
            server_reachable: true,
            tools_count: 10,
            connected_toolkits: vec!["gmail".to_string(), "github".to_string()],
            disconnected_toolkits: Vec::new(),
            error: None,
        };

        let message = check.status_message();
        assert!(message.contains("✅"));
        assert!(message.contains("10 tools"));
        assert!(message.contains("gmail"));
        assert!(message.contains("github"));
    }

    #[test]
    fn health_check_status_message_shows_disconnected() {
        let check = McpHealthCheck {
            server_reachable: true,
            tools_count: 5,
            connected_toolkits: vec!["gmail".to_string()],
            disconnected_toolkits: vec!["slack".to_string()],
            error: None,
        };

        let message = check.status_message();
        assert!(message.contains("⚠️"));
        assert!(message.contains("slack"));
    }

    #[test]
    fn health_check_status_message_shows_error() {
        let check = McpHealthCheck {
            server_reachable: false,
            tools_count: 0,
            connected_toolkits: Vec::new(),
            disconnected_toolkits: Vec::new(),
            error: Some("Connection timeout".to_string()),
        };

        let message = check.status_message();
        assert!(message.contains("❌"));
        assert!(message.contains("Connection timeout"));
    }
}
