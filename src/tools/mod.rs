//! Tool subsystem for agent-callable capabilities.
//!
//! This module implements the tool execution surface exposed to the LLM during
//! agentic loops. Each tool implements the [`Tool`] trait defined in [`traits`],
//! which requires a name, description, JSON parameter schema, and an async
//! `execute` method returning a structured [`ToolResult`].
//!
//! Tools are assembled into registries by [`default_tools`] (shell, file read/write)
//! and [`all_tools`] (full set including memory, browser, cron, HTTP, delegation,
//! and optional integrations). Security policy enforcement is injected via
//! [`SecurityPolicy`](crate::security::SecurityPolicy) at construction time.
//!
//! # Extension
//!
//! To add a new tool, implement [`Tool`] in a new submodule and register it in
//! [`all_tools_with_runtime`]. See `AGENTS.md` §7.3 for the full change playbook.

pub mod browser;
pub mod browser_open;
pub mod cli_discovery;
pub mod composio;
pub mod composio_rest;
pub mod composio_meta;
pub mod composio_mcp;
pub mod composio_nl;
pub mod content_search;
pub mod cron_add;
pub mod cron_list;
pub mod cron_remove;
pub mod cron_run;
pub mod cron_runs;
pub mod cron_update;
pub mod delegate;
pub mod file_edit;
pub mod file_read;
pub mod file_write;
pub mod git_operations;
pub mod glob_search;
pub mod hardware_board_info;
pub mod hardware_memory_map;
pub mod hardware_memory_read;
pub mod http_request;
pub mod image_info;
pub mod memory_forget;
pub mod memory_recall;
pub mod memory_store;
pub mod model_routing_config;
pub mod pdf_read;
pub mod proxy_config;
pub mod pushover;
pub mod schedule;
pub mod schema;
pub mod screenshot;
pub mod shell;
pub mod traits;
pub mod web_search_tool;

pub use browser::{BrowserTool, ComputerUseConfig};
pub use browser_open::BrowserOpenTool;
pub use composio_rest::ComposioTool;
pub use composio_mcp::ComposioMcpTool;
pub use composio_nl::ComposioNaturalLanguageTool;
pub use content_search::ContentSearchTool;
pub use cron_add::CronAddTool;
pub use cron_list::CronListTool;
pub use cron_remove::CronRemoveTool;
pub use cron_run::CronRunTool;
pub use cron_runs::CronRunsTool;
pub use cron_update::CronUpdateTool;
pub use delegate::DelegateTool;
pub use file_edit::FileEditTool;
pub use file_read::FileReadTool;
pub use file_write::FileWriteTool;
pub use git_operations::GitOperationsTool;
pub use glob_search::GlobSearchTool;
pub use hardware_board_info::HardwareBoardInfoTool;
pub use hardware_memory_map::HardwareMemoryMapTool;
pub use hardware_memory_read::HardwareMemoryReadTool;
pub use http_request::HttpRequestTool;
pub use image_info::ImageInfoTool;
pub use memory_forget::MemoryForgetTool;
pub use memory_recall::MemoryRecallTool;
pub use memory_store::MemoryStoreTool;
pub use model_routing_config::ModelRoutingConfigTool;
pub use pdf_read::PdfReadTool;
pub use proxy_config::ProxyConfigTool;
pub use pushover::PushoverTool;
pub use schedule::ScheduleTool;
#[allow(unused_imports)]
pub use schema::{CleaningStrategy, SchemaCleanr};
pub use screenshot::ScreenshotTool;
pub use shell::ShellTool;
pub use traits::Tool;
#[allow(unused_imports)]
pub use traits::{ToolResult, ToolSpec};
pub use web_search_tool::WebSearchTool;

use crate::config::{Config, DelegateAgentConfig};
use crate::memory::Memory;
use crate::mcp::{ComposioMcpClient, McpClient, McpTool};
use crate::runtime::{NativeRuntime, RuntimeAdapter};
use crate::security::SecurityPolicy;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
struct ArcDelegatingTool {
    inner: Arc<dyn Tool>,
}

impl ArcDelegatingTool {
    fn boxed(inner: Arc<dyn Tool>) -> Box<dyn Tool> {
        Box::new(Self { inner })
    }
}

#[async_trait]
impl Tool for ArcDelegatingTool {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.inner.parameters_schema()
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        self.inner.execute(args).await
    }
}

pub fn boxed_registry_from_arcs(tools: Vec<Arc<dyn Tool>>) -> Vec<Box<dyn Tool>> {
    tools.into_iter().map(ArcDelegatingTool::boxed).collect()
}

/// Create the default tool registry
pub fn default_tools(security: Arc<SecurityPolicy>) -> Vec<Box<dyn Tool>> {
    default_tools_with_runtime(security, Arc::new(NativeRuntime::new()))
}

/// Create the default tool registry with explicit runtime adapter.
pub fn default_tools_with_runtime(
    security: Arc<SecurityPolicy>,
    runtime: Arc<dyn RuntimeAdapter>,
) -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(ShellTool::new(security.clone(), runtime)),
        Box::new(FileReadTool::new(security.clone())),
        Box::new(FileWriteTool::new(security.clone())),
        Box::new(FileEditTool::new(security.clone())),
        Box::new(GlobSearchTool::new(security.clone())),
        Box::new(ContentSearchTool::new(security)),
    ]
}

/// Load Composio tools with automatic pattern selection.
///
/// This is the single entry point for loading Composio tools. It automatically
/// selects the appropriate pattern based on configuration:
/// - Pattern 2 (MCP meta-tools): When MCP is enabled
/// - Pattern 1 (direct REST tools): When MCP disabled but API key exists
/// - No tools: When Composio is disabled
///
/// # Arguments
/// * `config` - Full Composio configuration
/// * `security` - Security policy for access control
/// * `provider` - Optional LLM provider for intelligent parameter extraction
/// * `model` - Optional model name for LLM provider
///
/// # Returns
/// Vector of Arc-wrapped tools ready to be added to the tool registry
pub async fn load_composio_tools(
    config: &crate::config::ComposioConfig,
    security: Arc<SecurityPolicy>,
    provider: Option<Arc<dyn crate::providers::Provider>>,
    model: Option<String>,
) -> Vec<Arc<dyn Tool>> {
    // Auto-migrate configuration if needed
    let config = config.auto_migrate();
    
    // Pattern selection logic - PRIORITIZES Pattern 2 (MCP) over Pattern 1 (REST)
    // Pattern 2 (MCP meta-tools) is the recommended approach for dynamic tool discovery
    
    if !config.enabled {
        // Composio disabled - return empty tool list
        tracing::debug!("Composio integration is disabled in configuration");
        return Vec::new();
    }

    if config.mcp.enabled {
        // Pattern 2: MCP-first with meta tools (RECOMMENDED)
        tracing::info!("Loading Composio tools using Pattern 2 (MCP meta-tools) - recommended approach");
        
        let (user_id, is_legacy) = config.effective_user_id();
        if is_legacy {
            tracing::warn!(
                "Using legacy entity_id field. Please migrate to user_id in config.toml"
            );
            eprintln!(
                "⚠ WARNING: Using legacy entity_id field. Please migrate to user_id in config.toml"
            );
        }

        if let Some(api_key) = &config.api_key {
            match load_composio_mcp_tools(
                &config.mcp,
                api_key,
                &user_id,
                security,
                provider,
                model,
            )
            .await
            {
                Ok(tools) => {
                    tracing::info!(
                        tool_count = tools.len(),
                        "Successfully loaded Composio tools using Pattern 2 (MCP)"
                    );
                    tools
                },
                Err(e) => {
                    // Graceful degradation: Log warning and continue without Composio tools
                    tracing::warn!(
                        error = %e,
                        "Failed to load Composio MCP tools. Agent will continue without Composio integration."
                    );
                    eprintln!("⚠ Failed to load Composio MCP tools: {}", e);
                    eprintln!("  Agent will continue operating with reduced functionality.");
                    eprintln!("  Check Composio API connectivity and configuration.");
                    Vec::new()
                }
            }
        } else {
            tracing::warn!("Composio MCP enabled but no API key provided");
            eprintln!("⚠ Composio MCP enabled but no API key provided");
            Vec::new()
        }
    } else if let Some(api_key) = &config.api_key {
        // Pattern 1: REST API fallback (DEPRECATED)
        if !api_key.is_empty() {
            let (user_id, _) = config.effective_user_id();
            tracing::warn!(
                "Using deprecated Composio Pattern 1 (REST API). Consider enabling MCP for Pattern 2 (meta-tools)."
            );
            eprintln!("\n╔════════════════════════════════════════════════════════════════╗");
            eprintln!("║  ⚠️  DEPRECATION WARNING: Composio Pattern 1 (REST API)      ║");
            eprintln!("╠════════════════════════════════════════════════════════════════╣");
            eprintln!("║  Pattern 1 (REST API) is DEPRECATED and will be removed in    ║");
            eprintln!("║  a future version. Please migrate to Pattern 2 (MCP) for:     ║");
            eprintln!("║                                                                ║");
            eprintln!("║  ✓ Dynamic tool discovery (1000+ tools)                       ║");
            eprintln!("║  ✓ Natural language queries                                   ║");
            eprintln!("║  ✓ Simplified OAuth handling                                  ║");
            eprintln!("║  ✓ Better error messages                                      ║");
            eprintln!("║                                                                ║");
            eprintln!("║  Migration steps:                                             ║");
            eprintln!("║  1. Add [composio.mcp] section to config.toml                 ║");
            eprintln!("║  2. Set enabled=true                                          ║");
            eprintln!("║  3. Add server_id or mcp_url from Composio dashboard          ║");
            eprintln!("║                                                                ║");
            eprintln!("║  See: docs/composio-migration.md for detailed guide           ║");
            eprintln!("╚════════════════════════════════════════════════════════════════╝\n");
            vec![Arc::new(ComposioTool::new(
                api_key,
                Some(&user_id),
                security,
            )) as Arc<dyn Tool>]
        } else {
            Vec::new()
        }
    } else {
        // Composio enabled but no configuration provided
        tracing::warn!("Composio enabled but no API key or MCP configuration provided");
        eprintln!("⚠ Composio enabled but no API key or MCP configuration provided");
        Vec::new()
    }
}

/// Load Composio MCP tools dynamically from the MCP server
///
/// This function creates a Composio MCP client and fetches all available tools
/// from the configured MCP server, wrapping them as native ZeroClaw tools.
///
/// # Arguments
/// * `mcp_config` - MCP configuration from config.toml
/// * `api_key` - Composio API key for authentication
/// * `entity_id_fallback` - Fallback entity ID if user_id not set in MCP config
/// * `security` - Security policy for access control
/// * `provider` - Optional LLM provider for intelligent parameter extraction
///
/// # Returns
/// Vector of Arc-wrapped tools ready to be added to the tool registry
pub async fn load_composio_mcp_tools(
    mcp_config: &crate::config::ComposioMcpConfig,
    api_key: &str,
    entity_id_fallback: &str,
    security: Arc<SecurityPolicy>,
    provider: Option<Arc<dyn crate::providers::Provider>>,
    model: Option<String>,
) -> anyhow::Result<Vec<Arc<dyn Tool>>> {
    if !mcp_config.enabled {
        return Ok(Vec::new());
    }

    // Validate configuration before proceeding
    use anyhow::Context;
    crate::composio::validate_mcp_config(
        mcp_config.enabled,
        &mcp_config.mcp_url,
        &mcp_config.server_id,
        &mcp_config.toolkits,
    )
    .context("Invalid Composio MCP configuration")?;

    // Use MCP-specific API key if provided, otherwise fall back to main Composio API key
    let mcp_api_key = mcp_config
        .api_key
        .as_deref()
        .unwrap_or(api_key);

    // Use user_id from MCP config, or fall back to entity_id
    let user_id = mcp_config
        .user_id
        .as_deref()
        .unwrap_or(entity_id_fallback);

    let ttl = Duration::from_secs(mcp_config.tools_cache_ttl_secs);

    tracing::info!(
        mcp_url = mcp_config.mcp_url.as_deref(),
        server_id = mcp_config.server_id.as_deref(),
        user_id = user_id,
        toolkits = ?mcp_config.toolkits,
        "Loading Composio MCP tools"
    );

    // Create MCP client - prefer mcp_url over server_id
    let client = if let Some(mcp_url) = &mcp_config.mcp_url {
        // Recommended: use generated MCP URL
        Arc::new(ComposioMcpClient::new_with_mcp_url(
            mcp_api_key.to_string(),
            mcp_url.clone(),
            mcp_config.server_id.clone(),
            Some(user_id.to_string()),
            ttl,
        ))
    } else if let Some(server_id) = &mcp_config.server_id {
        // Legacy: use server_id
        Arc::new(ComposioMcpClient::new_with_ttl(
            mcp_api_key.to_string(),
            server_id.clone(),
            user_id.to_string(),
            ttl,
        ))
    } else {
        anyhow::bail!("MCP configuration requires either mcp_url or server_id");
    };

    // Fetch available tools from MCP server with graceful degradation
    let mcp_tools: Vec<McpTool> = match client.list_tools().await {
        Ok(tools) => tools,
        Err(e) => {
            // Graceful degradation: MCP server unreachable
            tracing::warn!(
                error = %e,
                mcp_url = mcp_config.mcp_url.as_deref(),
                server_id = mcp_config.server_id.as_deref(),
                "MCP server unreachable. Agent will continue without Composio integration."
            );
            eprintln!("⚠ MCP server unreachable: {}", e);
            eprintln!("  Agent will continue without Composio integration.");
            eprintln!("  Check Composio API connectivity and configuration.");
            return Ok(Vec::new());
        }
    };

    if mcp_tools.is_empty() {
        tracing::warn!("Composio MCP server returned no tools");
        eprintln!("Warning: Composio MCP server returned no tools. Check your MCP configuration.");
        return Ok(Vec::new());
    }

    // Detect Pattern 2 (meta-tools) vs Pattern 1 (direct app tools)
    let has_search_tools = mcp_tools.iter().any(|t| t.name == "COMPOSIO_SEARCH_TOOLS");
    let has_multi_execute = mcp_tools.iter().any(|t| t.name == "COMPOSIO_MULTI_EXECUTE_TOOL");
    let is_pattern_2 = has_search_tools && has_multi_execute;

    if is_pattern_2 {
        eprintln!(
            "Info: Detected Composio Pattern 2 (meta-tools) - Dynamic discovery enabled for 1000+ tools"
        );
        tracing::info!(
            "Composio Pattern 2 detected: using dynamic tool discovery via meta-tools"
        );

        // Create SSE-based MCP client for meta-tools
        let mcp_url = if let Some(url) = &mcp_config.mcp_url {
            url.clone()
        } else if let Some(server_id) = &mcp_config.server_id {
            // Construct URL from server_id
            if server_id.starts_with("trs_") {
                // Tool Router Session format
                format!(
                    "https://backend.composio.dev/tool_router/{}/mcp?include_composio_helper_actions=true&user_id={}",
                    server_id, user_id
                )
            } else {
                // Dedicated MCP Server format
                format!(
                    "https://backend.composio.dev/tool_router/{}/mcp?include_composio_helper_actions=true&user_id={}",
                    server_id, user_id
                )
            }
        } else {
            anyhow::bail!("MCP configuration requires either mcp_url or server_id");
        };

        let sse_client = Arc::new(
            McpClient::new(mcp_url, mcp_api_key.to_string())
                .context("Failed to create SSE MCP client")?
        );

        // Create natural language tool that wraps meta-tools workflow
        let nl_tool: Arc<dyn Tool> = if let Some(provider) = provider {
            Arc::new(ComposioNaturalLanguageTool::new_with_provider(
                sse_client,
                security,
                provider,
                model,
                api_key.to_string(),
            ))
        } else {
            Arc::new(ComposioNaturalLanguageTool::new(
                sse_client,
                security,
                api_key.to_string(),
            ))
        };

        return Ok(vec![nl_tool]);
    }

    // Pattern 1: Direct app tools
    tracing::warn!(
        tool_count = mcp_tools.len(),
        "Composio Pattern 1 detected via MCP: using direct app tools (not recommended)"
    );
    eprintln!("\n⚠️  WARNING: Composio Pattern 1 detected via MCP server");
    eprintln!("   Your MCP server is configured to return direct app tools instead of meta-tools.");
    eprintln!("   For better functionality, reconfigure your MCP server to use Pattern 2 (meta-tools).");
    eprintln!("   Pattern 2 provides dynamic tool discovery and natural language queries.");
    eprintln!("   See: docs/composio-migration.md for migration guide\n");
    eprintln!(
        "Info: Loaded {} tools from Composio MCP server (cache TTL: {}s)",
        mcp_tools.len(),
        mcp_config.tools_cache_ttl_secs
    );

    // Create onboarding handler based on UI mode
    let onboarding: Option<Arc<dyn crate::composio::ComposioOnboarding>> = {
        use crate::composio::{CliOnboarding, ComposioRestClient, OnboardingUx, ServerOnboarding};

        // Create REST client for onboarding
        let rest_client = Arc::new(ComposioRestClient::new(api_key.to_string(), user_id.to_string()));

        // Determine UI mode from environment
        let ui_mode = std::env::var("ZEROCLAW_UI_MODE")
            .unwrap_or_else(|_| "cli".to_string());

        match ui_mode.as_str() {
            "server" => Some(Arc::new(ServerOnboarding::new(rest_client))),
            "cli" => {
                // Check if browser auto-open is disabled
                let no_browser = std::env::var("ZEROCLAW_NO_BROWSER")
                    .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                    .unwrap_or(false);

                let ux = if no_browser {
                    OnboardingUx::CliPrintOnly
                } else {
                    OnboardingUx::CliAutoOpen
                };

                Some(Arc::new(CliOnboarding::new(rest_client, ux)))
            }
            _ => None,
        }
    };

    // Convert to ZeroClaw tools with onboarding support
    let tools: Vec<Arc<dyn Tool>> = mcp_tools
        .into_iter()
        .map(|mcp_tool| {
            Arc::new(ComposioMcpTool::new_with_onboarding(
                client.clone(),
                mcp_tool,
                security.clone(),
                onboarding.clone(),
            )) as Arc<dyn Tool>
        })
        .collect();

    Ok(tools)
}
/// Refresh Composio tools without agent restart
///
/// This function reloads Composio tools from the MCP server or REST API,
/// invalidating caches and fetching fresh tool lists. It enables hot reload
/// of the tool registry without requiring agent restart.
///
/// # Arguments
/// * `config` - Composio configuration
/// * `security` - Security policy for tool execution
/// * `provider` - Optional LLM provider for natural language tool
/// * `model` - Optional model name for natural language tool
///
/// # Returns
/// Vector of refreshed Arc-wrapped tools
///
/// # Behavior
/// - Invalidates all tool caches (tool list, schemas, connections)
/// - Fetches fresh tool list from MCP server or REST API
/// - Recreates tool instances with clean state
/// - Maintains same pattern selection logic as load_composio_tools
/// - Gracefully degrades on failure (returns empty vec, logs warning)
///
/// # Example
/// ```no_run
/// use std::sync::Arc;
/// use zeroclaw::tools::refresh_composio_tools;
/// use zeroclaw::security::SecurityPolicy;
///
/// # async fn example() -> anyhow::Result<()> {
/// let config = /* load config */;
/// let security = Arc::new(SecurityPolicy::default());
/// let refreshed_tools = refresh_composio_tools(&config, security, None, None).await;
/// # Ok(())
/// # }
/// ```
pub async fn refresh_composio_tools(
    config: &crate::config::ComposioConfig,
    security: Arc<SecurityPolicy>,
    provider: Option<Arc<dyn crate::providers::Provider>>,
    model: Option<String>,
) -> Vec<Arc<dyn Tool>> {
    tracing::info!("Refreshing Composio tools (hot reload)");
    eprintln!("🔄 Refreshing Composio tools...");

    // Invalidate caches before reloading
    // Note: Cache invalidation happens implicitly by creating new client instances
    // with fresh cache state. The old client instances will be dropped when tools
    // are replaced in the registry.

    // Use the same pattern selection logic as load_composio_tools
    if config.mcp.enabled {
        // Pattern 2: MCP-first with meta tools
        let (user_id, is_legacy) = config.effective_user_id();
        if is_legacy {
            eprintln!(
                "WARNING: Using legacy entity_id field. Please migrate to user_id in config.toml"
            );
        }

        if let Some(api_key) = &config.api_key {
            // Force cache refresh by creating new MCP client with zero TTL temporarily
            let mut mcp_config_refresh = config.mcp.clone();
            // Set TTL to 0 to force immediate refresh, then restore original TTL
            let original_ttl = mcp_config_refresh.tools_cache_ttl_secs;
            mcp_config_refresh.tools_cache_ttl_secs = 0;

            match load_composio_mcp_tools(
                &mcp_config_refresh,
                api_key,
                &user_id,
                security.clone(),
                provider.clone(),
                model.clone(),
            )
            .await
            {
                Ok(tools) => {
                    eprintln!("✓ Successfully refreshed {} Composio tool(s)", tools.len());
                    tracing::info!(
                        tool_count = tools.len(),
                        "Composio tools refreshed successfully"
                    );

                    // Restore original TTL for subsequent operations
                    // Note: This is informational only; the actual TTL is set in the client
                    tracing::debug!(
                        original_ttl = original_ttl,
                        "Tool cache TTL will be restored on next load"
                    );

                    tools
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Failed to refresh Composio MCP tools. Keeping existing tools."
                    );
                    eprintln!("⚠ Failed to refresh Composio MCP tools: {}", e);
                    eprintln!("  Existing tools will remain active.");
                    Vec::new()
                }
            }
        } else {
            tracing::warn!("Composio MCP enabled but no API key provided");
            eprintln!("⚠ Composio MCP enabled but no API key provided");
            Vec::new()
        }
    } else if let Some(api_key) = &config.api_key {
        // Pattern 1: REST API fallback (deprecated)
        if !api_key.is_empty() {
            let (user_id, _) = config.effective_user_id();
            eprintln!(
                "Info: Refreshing Composio Pattern 1 (REST API). Consider enabling MCP for Pattern 2."
            );
            tracing::info!("Refreshing Composio REST API tool");
            vec![Arc::new(ComposioTool::new(
                api_key,
                Some(&user_id),
                security,
            )) as Arc<dyn Tool>]
        } else {
            Vec::new()
        }
    } else {
        // Composio disabled
        tracing::debug!("Composio disabled, no tools to refresh");
        Vec::new()
    }
}

/// Create full tool registry including memory tools and optional Composio
#[allow(clippy::implicit_hasher, clippy::too_many_arguments)]
pub async fn all_tools(
    config: Arc<Config>,
    security: &Arc<SecurityPolicy>,
    memory: Arc<dyn Memory>,
    composio_key: Option<&str>,
    composio_entity_id: Option<&str>,
    browser_config: &crate::config::BrowserConfig,
    http_config: &crate::config::HttpRequestConfig,
    workspace_dir: &std::path::Path,
    agents: &HashMap<String, DelegateAgentConfig>,
    fallback_api_key: Option<&str>,
    root_config: &crate::config::Config,
    provider: Option<Arc<dyn crate::providers::Provider>>,
    model: Option<String>,
) -> Vec<Box<dyn Tool>> {
    all_tools_with_runtime(
        config,
        security,
        Arc::new(NativeRuntime::new()),
        memory,
        composio_key,
        composio_entity_id,
        browser_config,
        http_config,
        workspace_dir,
        agents,
        fallback_api_key,
        root_config,
        provider,
        model,
    )
    .await
}

/// Create full tool registry including memory tools and optional Composio.
#[allow(clippy::implicit_hasher, clippy::too_many_arguments)]
pub async fn all_tools_with_runtime(
    config: Arc<Config>,
    security: &Arc<SecurityPolicy>,
    runtime: Arc<dyn RuntimeAdapter>,
    memory: Arc<dyn Memory>,
    composio_key: Option<&str>,
    _composio_entity_id: Option<&str>,
    browser_config: &crate::config::BrowserConfig,
    http_config: &crate::config::HttpRequestConfig,
    workspace_dir: &std::path::Path,
    agents: &HashMap<String, DelegateAgentConfig>,
    fallback_api_key: Option<&str>,
    root_config: &crate::config::Config,
    provider: Option<Arc<dyn crate::providers::Provider>>,
    model: Option<String>,
) -> Vec<Box<dyn Tool>> {
    let mut tool_arcs: Vec<Arc<dyn Tool>> = vec![
        Arc::new(ShellTool::new(security.clone(), runtime)),
        Arc::new(FileReadTool::new(security.clone())),
        Arc::new(FileWriteTool::new(security.clone())),
        Arc::new(FileEditTool::new(security.clone())),
        Arc::new(GlobSearchTool::new(security.clone())),
        Arc::new(ContentSearchTool::new(security.clone())),
        Arc::new(CronAddTool::new(config.clone(), security.clone())),
        Arc::new(CronListTool::new(config.clone())),
        Arc::new(CronRemoveTool::new(config.clone(), security.clone())),
        Arc::new(CronUpdateTool::new(config.clone(), security.clone())),
        Arc::new(CronRunTool::new(config.clone(), security.clone())),
        Arc::new(CronRunsTool::new(config.clone())),
        Arc::new(MemoryStoreTool::new(memory.clone(), security.clone())),
        Arc::new(MemoryRecallTool::new(memory.clone())),
        Arc::new(MemoryForgetTool::new(memory, security.clone())),
        Arc::new(ScheduleTool::new(security.clone(), root_config.clone())),
        Arc::new(ModelRoutingConfigTool::new(
            config.clone(),
            security.clone(),
        )),
        Arc::new(ProxyConfigTool::new(config.clone(), security.clone())),
        Arc::new(GitOperationsTool::new(
            security.clone(),
            workspace_dir.to_path_buf(),
        )),
        Arc::new(PushoverTool::new(
            security.clone(),
            workspace_dir.to_path_buf(),
        )),
    ];

    if browser_config.enabled {
        // Add legacy browser_open tool for simple URL opening
        tool_arcs.push(Arc::new(BrowserOpenTool::new(
            security.clone(),
            browser_config.allowed_domains.clone(),
        )));
        // Add full browser automation tool (pluggable backend)
        tool_arcs.push(Arc::new(BrowserTool::new_with_backend(
            security.clone(),
            browser_config.allowed_domains.clone(),
            browser_config.session_name.clone(),
            browser_config.backend.clone(),
            browser_config.native_headless,
            browser_config.native_webdriver_url.clone(),
            browser_config.native_chrome_path.clone(),
            ComputerUseConfig {
                endpoint: browser_config.computer_use.endpoint.clone(),
                api_key: browser_config.computer_use.api_key.clone(),
                timeout_ms: browser_config.computer_use.timeout_ms,
                allow_remote_endpoint: browser_config.computer_use.allow_remote_endpoint,
                window_allowlist: browser_config.computer_use.window_allowlist.clone(),
                max_coordinate_x: browser_config.computer_use.max_coordinate_x,
                max_coordinate_y: browser_config.computer_use.max_coordinate_y,
            },
        )));
    }

    if http_config.enabled {
        tool_arcs.push(Arc::new(HttpRequestTool::new(
            security.clone(),
            http_config.allowed_domains.clone(),
            http_config.max_response_size,
            http_config.timeout_secs,
        )));
    }

    // Web search tool (enabled by default for GLM and other models)
    if root_config.web_search.enabled {
        tool_arcs.push(Arc::new(WebSearchTool::new(
            root_config.web_search.provider.clone(),
            root_config.web_search.brave_api_key.clone(),
            root_config.web_search.max_results,
            root_config.web_search.timeout_secs,
        )));
    }

    // PDF extraction (feature-gated at compile time via rag-pdf)
    tool_arcs.push(Arc::new(PdfReadTool::new(security.clone())));

    // Vision tools are always available
    tool_arcs.push(Arc::new(ScreenshotTool::new(security.clone())));
    tool_arcs.push(Arc::new(ImageInfoTool::new(security.clone())));

    // Add delegation tool when agents are configured
    if !agents.is_empty() {
        let delegate_agents: HashMap<String, DelegateAgentConfig> = agents
            .iter()
            .map(|(name, cfg)| (name.clone(), cfg.clone()))
            .collect();
        let delegate_fallback_credential = fallback_api_key.and_then(|value| {
            let trimmed_value = value.trim();
            (!trimmed_value.is_empty()).then(|| trimmed_value.to_owned())
        });
        let parent_tools = Arc::new(tool_arcs.clone());
        let delegate_tool = DelegateTool::new_with_options(
            delegate_agents,
            delegate_fallback_credential,
            security.clone(),
            crate::providers::ProviderRuntimeOptions {
                auth_profile_override: None,
                zeroclaw_dir: root_config
                    .config_path
                    .parent()
                    .map(std::path::PathBuf::from),
                secrets_encrypt: root_config.secrets.encrypt,
                reasoning_enabled: root_config.runtime.reasoning_enabled,
            },
        )
        .with_parent_tools(parent_tools)
        .with_multimodal_config(root_config.multimodal.clone());
        tool_arcs.push(Arc::new(delegate_tool));
    }

    // Load Composio tools with automatic pattern selection
    if let Some(key) = composio_key {
        if !key.is_empty() && root_config.composio.enabled {
            match load_composio_tools(
                &root_config.composio,
                security.clone(),
                provider.clone(),
                model.clone(),
            )
            .await
            {
                tools if !tools.is_empty() => {
                    eprintln!("✓ Loaded {} Composio tool(s)", tools.len());
                    tool_arcs.extend(tools);
                }
                _ => {
                    eprintln!("⚠ No Composio tools loaded");
                }
            }
        }
    }

    boxed_registry_from_arcs(tool_arcs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BrowserConfig, Config, MemoryConfig};
    use tempfile::TempDir;

    fn test_config(tmp: &TempDir) -> Config {
        Config {
            workspace_dir: tmp.path().join("workspace"),
            config_path: tmp.path().join("config.toml"),
            ..Config::default()
        }
    }

    #[test]
    fn default_tools_has_expected_count() {
        let security = Arc::new(SecurityPolicy::default());
        let tools = default_tools(security);
        assert_eq!(tools.len(), 6);
    }

    #[tokio::test]
    async fn all_tools_excludes_browser_when_disabled() {
        let tmp = TempDir::new().unwrap();
        let security = Arc::new(SecurityPolicy::default());
        let mem_cfg = MemoryConfig {
            backend: "markdown".into(),
            ..MemoryConfig::default()
        };
        let mem: Arc<dyn Memory> =
            Arc::from(crate::memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());

        let browser = BrowserConfig {
            enabled: false,
            allowed_domains: vec!["example.com".into()],
            session_name: None,
            ..BrowserConfig::default()
        };
        let http = crate::config::HttpRequestConfig::default();
        let cfg = test_config(&tmp);

        let tools = all_tools(
            Arc::new(Config::default()),
            &security,
            mem,
            None,
            None,
            &browser,
            &http,
            tmp.path(),
            &HashMap::new(),
            None,
            &cfg,
            None, // provider
            None, // model
        )
        .await;
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(!names.contains(&"browser_open"));
        assert!(names.contains(&"schedule"));
        assert!(names.contains(&"model_routing_config"));
        assert!(names.contains(&"pushover"));
        assert!(names.contains(&"proxy_config"));
    }

    #[tokio::test]
    async fn all_tools_includes_browser_when_enabled() {
        let tmp = TempDir::new().unwrap();
        let security = Arc::new(SecurityPolicy::default());
        let mem_cfg = MemoryConfig {
            backend: "markdown".into(),
            ..MemoryConfig::default()
        };
        let mem: Arc<dyn Memory> =
            Arc::from(crate::memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());

        let browser = BrowserConfig {
            enabled: true,
            allowed_domains: vec!["example.com".into()],
            session_name: None,
            ..BrowserConfig::default()
        };
        let http = crate::config::HttpRequestConfig::default();
        let cfg = test_config(&tmp);

        let tools = all_tools(
            Arc::new(Config::default()),
            &security,
            mem,
            None,
            None,
            &browser,
            &http,
            tmp.path(),
            &HashMap::new(),
            None,
            &cfg,
            None, // provider
            None, // model
        )
        .await;
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"browser_open"));
        assert!(names.contains(&"content_search"));
        assert!(names.contains(&"model_routing_config"));
        assert!(names.contains(&"pushover"));
        assert!(names.contains(&"proxy_config"));
    }

    #[test]
    fn default_tools_names() {
        let security = Arc::new(SecurityPolicy::default());
        let tools = default_tools(security);
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"shell"));
        assert!(names.contains(&"file_read"));
        assert!(names.contains(&"file_write"));
        assert!(names.contains(&"file_edit"));
        assert!(names.contains(&"glob_search"));
        assert!(names.contains(&"content_search"));
    }

    #[test]
    fn default_tools_all_have_descriptions() {
        let security = Arc::new(SecurityPolicy::default());
        let tools = default_tools(security);
        for tool in &tools {
            assert!(
                !tool.description().is_empty(),
                "Tool {} has empty description",
                tool.name()
            );
        }
    }

    #[test]
    fn default_tools_all_have_schemas() {
        let security = Arc::new(SecurityPolicy::default());
        let tools = default_tools(security);
        for tool in &tools {
            let schema = tool.parameters_schema();
            assert!(
                schema.is_object(),
                "Tool {} schema is not an object",
                tool.name()
            );
            assert!(
                schema["properties"].is_object(),
                "Tool {} schema has no properties",
                tool.name()
            );
        }
    }

    #[test]
    fn tool_spec_generation() {
        let security = Arc::new(SecurityPolicy::default());
        let tools = default_tools(security);
        for tool in &tools {
            let spec = tool.spec();
            assert_eq!(spec.name, tool.name());
            assert_eq!(spec.description, tool.description());
            assert!(spec.parameters.is_object());
        }
    }

    #[test]
    fn tool_result_serde() {
        let result = ToolResult {
            success: true,
            output: "hello".into(),
            error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: ToolResult = serde_json::from_str(&json).unwrap();
        assert!(parsed.success);
        assert_eq!(parsed.output, "hello");
        assert!(parsed.error.is_none());
    }

    #[test]
    fn tool_result_with_error_serde() {
        let result = ToolResult {
            success: false,
            output: String::new(),
            error: Some("boom".into()),
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: ToolResult = serde_json::from_str(&json).unwrap();
        assert!(!parsed.success);
        assert_eq!(parsed.error.as_deref(), Some("boom"));
    }

    #[test]
    fn tool_spec_serde() {
        let spec = ToolSpec {
            name: "test".into(),
            description: "A test tool".into(),
            parameters: serde_json::json!({"type": "object"}),
        };
        let json = serde_json::to_string(&spec).unwrap();
        let parsed: ToolSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "test");
        assert_eq!(parsed.description, "A test tool");
    }

    #[tokio::test]
    async fn all_tools_includes_delegate_when_agents_configured() {
        let tmp = TempDir::new().unwrap();
        let security = Arc::new(SecurityPolicy::default());
        let mem_cfg = MemoryConfig {
            backend: "markdown".into(),
            ..MemoryConfig::default()
        };
        let mem: Arc<dyn Memory> =
            Arc::from(crate::memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());

        let browser = BrowserConfig::default();
        let http = crate::config::HttpRequestConfig::default();
        let cfg = test_config(&tmp);

        let mut agents = HashMap::new();
        agents.insert(
            "researcher".to_string(),
            DelegateAgentConfig {
                provider: "ollama".to_string(),
                model: "llama3".to_string(),
                system_prompt: None,
                api_key: None,
                temperature: None,
                max_depth: 3,
                agentic: false,
                allowed_tools: Vec::new(),
                max_iterations: 10,
            },
        );

        let tools = all_tools(
            Arc::new(Config::default()),
            &security,
            mem,
            None,
            None,
            &browser,
            &http,
            tmp.path(),
            &agents,
            Some("delegate-test-credential"),
            &cfg,
            None, // provider
            None, // model
        )
        .await;
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"delegate"));
    }

    #[tokio::test]
    async fn all_tools_excludes_delegate_when_no_agents() {
        let tmp = TempDir::new().unwrap();
        let security = Arc::new(SecurityPolicy::default());
        let mem_cfg = MemoryConfig {
            backend: "markdown".into(),
            ..MemoryConfig::default()
        };
        let mem: Arc<dyn Memory> =
            Arc::from(crate::memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());

        let browser = BrowserConfig::default();
        let http = crate::config::HttpRequestConfig::default();
        let cfg = test_config(&tmp);

        let tools = all_tools(
            Arc::new(Config::default()),
            &security,
            mem,
            None,
            None,
            &browser,
            &http,
            tmp.path(),
            &HashMap::new(),
            None,
            &cfg,
            None, // provider
            None, // model
        )
        .await;
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(!names.contains(&"delegate"));
    }
}
