// Configuration validation for Composio MCP
//
// Validates configuration before attempting to use MCP tools.

use anyhow::{Context, Result};

/// Validate Composio MCP configuration
pub fn validate_mcp_config(
    enabled: bool,
    mcp_url: &Option<String>,
    server_id: &Option<String>,
    toolkits: &[String],
) -> Result<()> {
    if !enabled {
        return Ok(());
    }

    // Must have either mcp_url or server_id
    if mcp_url.is_none() && server_id.is_none() {
        anyhow::bail!(
            "Composio MCP is enabled but neither mcp_url nor server_id is configured. \
            Run 'zeroclaw onboard' to configure MCP integration."
        );
    }

    // Validate mcp_url format if present
    if let Some(url) = mcp_url {
        validate_mcp_url(url).context("Invalid mcp_url")?;
    }

    // Validate toolkits if present
    if !toolkits.is_empty() {
        for toolkit in toolkits {
            validate_toolkit_slug(toolkit)
                .with_context(|| format!("Invalid toolkit slug: {}", toolkit))?;
        }
    }

    Ok(())
}

/// Validate MCP URL format
fn validate_mcp_url(url: &str) -> Result<()> {
    if url.is_empty() {
        anyhow::bail!("MCP URL cannot be empty");
    }

    if !url.starts_with("http://") && !url.starts_with("https://") {
        anyhow::bail!("MCP URL must start with http:// or https://");
    }

    if !url.contains("composio.dev") && !url.contains("localhost") {
        tracing::warn!(
            url = url,
            "MCP URL does not contain 'composio.dev' - this may be incorrect"
        );
    }

    Ok(())
}

/// Validate toolkit slug format
fn validate_toolkit_slug(slug: &str) -> Result<()> {
    if slug.is_empty() {
        anyhow::bail!("Toolkit slug cannot be empty");
    }

    // Toolkit slugs should be lowercase alphanumeric with hyphens
    if !slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        anyhow::bail!(
            "Toolkit slug '{}' contains invalid characters. \
            Use lowercase letters, numbers, and hyphens only.",
            slug
        );
    }

    if slug.starts_with('-') || slug.ends_with('-') {
        anyhow::bail!("Toolkit slug '{}' cannot start or end with a hyphen", slug);
    }

    if slug.contains("--") {
        anyhow::bail!("Toolkit slug '{}' cannot contain consecutive hyphens", slug);
    }

    Ok(())
}

/// Normalize toolkit slug to standard format
pub fn normalize_toolkit_slug(slug: &str) -> String {
    slug.to_lowercase()
        .trim()
        .replace('_', "-")
        .chars()
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_mcp_config_requires_url_or_server_id() {
        let result = validate_mcp_config(true, &None, &None, &[]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("neither mcp_url nor server_id"));
    }

    #[test]
    fn validate_mcp_config_accepts_mcp_url() {
        let result = validate_mcp_config(
            true,
            &Some("https://backend.composio.dev/mcp".to_string()),
            &None,
            &[],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn validate_mcp_config_accepts_server_id() {
        let result = validate_mcp_config(true, &None, &Some("server_123".to_string()), &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_mcp_config_skips_when_disabled() {
        let result = validate_mcp_config(false, &None, &None, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_mcp_url_rejects_empty() {
        let result = validate_mcp_url("");
        assert!(result.is_err());
    }

    #[test]
    fn validate_mcp_url_requires_http_scheme() {
        let result = validate_mcp_url("composio.dev/mcp");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("http://"));
    }

    #[test]
    fn validate_mcp_url_accepts_https() {
        let result = validate_mcp_url("https://backend.composio.dev/mcp");
        assert!(result.is_ok());
    }

    #[test]
    fn validate_toolkit_slug_rejects_empty() {
        let result = validate_toolkit_slug("");
        assert!(result.is_err());
    }

    #[test]
    fn validate_toolkit_slug_rejects_uppercase() {
        let result = validate_toolkit_slug("Gmail");
        assert!(result.is_err());
    }

    #[test]
    fn validate_toolkit_slug_rejects_spaces() {
        let result = validate_toolkit_slug("google drive");
        assert!(result.is_err());
    }

    #[test]
    fn validate_toolkit_slug_accepts_valid() {
        assert!(validate_toolkit_slug("gmail").is_ok());
        assert!(validate_toolkit_slug("google-drive").is_ok());
        assert!(validate_toolkit_slug("github").is_ok());
        assert!(validate_toolkit_slug("slack-v2").is_ok());
    }

    #[test]
    fn validate_toolkit_slug_rejects_leading_hyphen() {
        let result = validate_toolkit_slug("-gmail");
        assert!(result.is_err());
    }

    #[test]
    fn validate_toolkit_slug_rejects_trailing_hyphen() {
        let result = validate_toolkit_slug("gmail-");
        assert!(result.is_err());
    }

    #[test]
    fn validate_toolkit_slug_rejects_consecutive_hyphens() {
        let result = validate_toolkit_slug("google--drive");
        assert!(result.is_err());
    }

    #[test]
    fn normalize_toolkit_slug_converts_to_lowercase() {
        assert_eq!(normalize_toolkit_slug("Gmail"), "gmail");
        assert_eq!(normalize_toolkit_slug("GITHUB"), "github");
    }

    #[test]
    fn normalize_toolkit_slug_replaces_underscores() {
        assert_eq!(normalize_toolkit_slug("google_drive"), "google-drive");
        assert_eq!(normalize_toolkit_slug("slack_v2"), "slack-v2");
    }

    #[test]
    fn normalize_toolkit_slug_removes_invalid_chars() {
        assert_eq!(normalize_toolkit_slug("gmail!@#"), "gmail");
        assert_eq!(normalize_toolkit_slug("git hub"), "github");
    }

    #[test]
    fn normalize_toolkit_slug_trims_whitespace() {
        assert_eq!(normalize_toolkit_slug("  gmail  "), "gmail");
    }
}
