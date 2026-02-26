// Composio OAuth Onboarding Module
//
// Provides automatic OAuth connection management for Composio MCP tools.
// Supports multiple UX modes: CLI auto-open, CLI print-only, and server return link.

use crate::composio::ComposioRestClient;
use async_trait::async_trait;
use std::sync::Arc;

/// UX mode for OAuth onboarding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingUx {
    /// CLI mode: auto-open browser + polling
    CliAutoOpen,
    /// CLI mode: print link only + polling
    CliPrintOnly,
    /// Server mode: return link in error (no polling)
    ServerReturnLink,
}

/// Trait for managing Composio OAuth onboarding
#[async_trait]
pub trait ComposioOnboarding: Send + Sync {
    /// Ensure a toolkit is connected for the given entity
    ///
    /// # Arguments
    /// * `toolkit_slug` - The toolkit identifier (e.g., "gmail", "github")
    /// * `entity_id` - The user/entity identifier
    ///
    /// # Returns
    /// Ok(()) if connection is established, Err otherwise
    async fn ensure_connected(
        &self,
        toolkit_slug: &str,
        entity_id: &str,
    ) -> anyhow::Result<()>;
}

/// CLI-based onboarding implementation (modes A and D)
pub struct CliOnboarding {
    rest_client: Arc<ComposioRestClient>,
    ux: OnboardingUx,
}

impl CliOnboarding {
    /// Create a new CLI onboarding handler
    pub fn new(rest_client: Arc<ComposioRestClient>, ux: OnboardingUx) -> Self {
        Self { rest_client, ux }
    }

    /// Get OAuth connection URL for a toolkit
    pub async fn get_connection_url(
        &self,
        toolkit: &str,
        entity_id: &str,
    ) -> anyhow::Result<String> {
        let link = self
            .rest_client
            .get_connection_url(Some(toolkit), None, entity_id)
            .await?;
        Ok(link.redirect_url)
    }

    /// Poll until the toolkit is connected or timeout (public version)
    pub async fn poll_until_connected(
        &self,
        toolkit: &str,
        entity_id: &str,
        timeout_secs: u64,
    ) -> anyhow::Result<()> {
        self.poll_until_connected_internal(toolkit, entity_id, timeout_secs).await
    }

    /// Poll until the toolkit is connected or timeout (internal implementation)
    async fn poll_until_connected_internal(
        &self,
        toolkit: &str,
        entity_id: &str,
        timeout_secs: u64,
    ) -> anyhow::Result<()> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);
        let mut attempts = 0;

        loop {
            attempts += 1;
            
            if start.elapsed() > timeout {
                anyhow::bail!(
                    "Timeout waiting for {} connection after {} attempts ({} seconds). \
                    Please ensure you completed the OAuth authorization in your browser.",
                    toolkit,
                    attempts,
                    timeout_secs
                );
            }

            // Check if connection is active
            match self
                .rest_client
                .list_connected_accounts(Some(toolkit), Some(entity_id))
                .await
            {
                Ok(accounts) => {
                    let has_active = accounts
                        .iter()
                        .any(|a| a.status.eq_ignore_ascii_case("ACTIVE"));
                    if has_active {
                        tracing::info!(
                            toolkit = toolkit,
                            entity_id = entity_id,
                            attempts = attempts,
                            elapsed_secs = start.elapsed().as_secs(),
                            "OAuth connection established successfully"
                        );
                        return Ok(());
                    }
                    
                    // Log progress every 10 attempts (20 seconds)
                    if attempts % 10 == 0 {
                        tracing::debug!(
                            toolkit = toolkit,
                            attempts = attempts,
                            elapsed_secs = start.elapsed().as_secs(),
                            "Still waiting for OAuth authorization..."
                        );
                    }
                }
                Err(e) => {
                    // Log but continue trying
                    tracing::warn!(
                        toolkit = toolkit,
                        error = %e,
                        "Failed to check connection status, will retry"
                    );
                }
            }

            // Wait 2 seconds before next attempt
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }
}

#[async_trait]
impl ComposioOnboarding for CliOnboarding {
    async fn ensure_connected(
        &self,
        toolkit_slug: &str,
        entity_id: &str,
    ) -> anyhow::Result<()> {
        tracing::info!(
            toolkit = toolkit_slug,
            entity_id = entity_id,
            mode = ?self.ux,
            "Starting OAuth onboarding flow"
        );

        // 1. Get connection URL
        let link = self
            .rest_client
            .get_connection_url(Some(toolkit_slug), None, entity_id)
            .await
            .map_err(|e| {
                tracing::error!(
                    toolkit = toolkit_slug,
                    error = %e,
                    "Failed to get OAuth connection URL"
                );
                anyhow::anyhow!(
                    "Failed to get OAuth URL for {}: {}. \
                    Please check your Composio API key and network connection.",
                    toolkit_slug,
                    e
                )
            })?;

        // 2. Present URL to user
        println!("\n🔗 {} OAuth Required", toolkit_slug.to_uppercase());
        println!("Open this URL in your browser:");
        println!("  {}", link.redirect_url);

        // 3. Try to auto-open browser (mode A only)
        if matches!(self.ux, OnboardingUx::CliAutoOpen) {
            match open::that(&link.redirect_url) {
                Ok(()) => {
                    tracing::debug!("Successfully opened browser for OAuth");
                    println!("  ✓ Browser opened automatically");
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to auto-open browser");
                    eprintln!("⚠ Could not auto-open browser: {}", e);
                    eprintln!("  Please open the URL manually in your browser.");
                }
            }
        } else {
            println!("  → Please open this URL manually in your browser");
        }

        // 4. Poll for connection
        println!("\n⏳ Waiting for authorization (timeout: 120s)...");
        println!("  Complete the OAuth flow in your browser to continue.");
        
        self.poll_until_connected_internal(toolkit_slug, entity_id, 120)
            .await?;

        println!("✓ {} connected successfully!", toolkit_slug);
        Ok(())
    }
}

/// Server-based onboarding implementation (mode C)
pub struct ServerOnboarding {
    rest_client: Arc<ComposioRestClient>,
}

impl ServerOnboarding {
    /// Create a new server onboarding handler
    pub fn new(rest_client: Arc<ComposioRestClient>) -> Self {
        Self { rest_client }
    }
}

#[async_trait]
impl ComposioOnboarding for ServerOnboarding {
    async fn ensure_connected(
        &self,
        toolkit_slug: &str,
        entity_id: &str,
    ) -> anyhow::Result<()> {
        tracing::info!(
            toolkit = toolkit_slug,
            entity_id = entity_id,
            mode = "server",
            "OAuth required, returning URL to client"
        );

        // Get connection URL
        let link = self
            .rest_client
            .get_connection_url(Some(toolkit_slug), None, entity_id)
            .await
            .map_err(|e| {
                tracing::error!(
                    toolkit = toolkit_slug,
                    error = %e,
                    "Failed to get OAuth connection URL"
                );
                e
            })?;

        // Return structured error message with clear OAuth URL
        // Format designed to be easily parsed and presented by the agent
        let error_message = format!(
            "OAuth authorization required for {}.\n\n\
            Please click this link to authorize:\n{}\n\n\
            After authorizing, please retry your request.\n\
            The authorization link expires in 10 minutes.",
            toolkit_slug.to_uppercase(),
            link.redirect_url
        );

        anyhow::bail!("{}", error_message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn onboarding_ux_modes_are_distinct() {
        assert_ne!(OnboardingUx::CliAutoOpen, OnboardingUx::CliPrintOnly);
        assert_ne!(OnboardingUx::CliAutoOpen, OnboardingUx::ServerReturnLink);
        assert_ne!(OnboardingUx::CliPrintOnly, OnboardingUx::ServerReturnLink);
    }

    #[test]
    fn onboarding_ux_debug_format() {
        let mode = OnboardingUx::CliAutoOpen;
        let debug_str = format!("{:?}", mode);
        assert!(debug_str.contains("CliAutoOpen"));
    }
}

