#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::assigning_clones,
    clippy::bool_to_int_with_if,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::cast_possible_wrap,
    clippy::doc_markdown,
    clippy::field_reassign_with_default,
    clippy::float_cmp,
    clippy::implicit_clone,
    clippy::items_after_statements,
    clippy::map_unwrap_or,
    clippy::manual_let_else,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::needless_pass_by_value,
    clippy::needless_raw_string_hashes,
    clippy::redundant_closure_for_method_calls,
    clippy::similar_names,
    clippy::single_match_else,
    clippy::struct_field_names,
    clippy::too_many_lines,
    clippy::uninlined_format_args,
    clippy::unused_self,
    clippy::cast_precision_loss,
    clippy::unnecessary_cast,
    clippy::unnecessary_lazy_evaluations,
    clippy::unnecessary_literal_bound,
    clippy::unnecessary_map_or,
    clippy::unnecessary_wraps,
    dead_code
)]

use anyhow::{bail, Context, Result};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use dialoguer::{Input, Password};
use serde::{Deserialize, Serialize};
use std::io::Write;
use tracing::{info, warn};
use tracing_subscriber::{fmt, EnvFilter};

fn parse_temperature(s: &str) -> std::result::Result<f64, String> {
    let t: f64 = s.parse().map_err(|e| format!("{e}"))?;
    if !(0.0..=2.0).contains(&t) {
        return Err("temperature must be between 0.0 and 2.0".to_string());
    }
    Ok(t)
}

mod agent;
mod approval;
mod auth;
mod channels;
mod composio {
    pub use zeroclaw::composio::*;
}
mod rag {
    pub use zeroclaw::rag::*;
}
mod config;
mod cost;
mod cron;
mod daemon;
mod doctor;
mod gateway;
mod hardware;
mod health;
mod heartbeat;
mod hooks;
mod identity;
mod integrations;
mod mcp {
    pub use zeroclaw::mcp::*;
}
mod memory;
mod migration;
mod multimodal;
mod observability;
mod onboard;
mod peripherals;
mod providers;
mod runtime;
mod security;
mod service;
mod skillforge;
mod skills;
mod tools;
mod tunnel;
mod util;

use config::Config;

// Re-export so binary modules can use crate::<CommandEnum> while keeping a single source of truth.
pub use zeroclaw::{
    ChannelCommands, CronCommands, HardwareCommands, IntegrationCommands, MigrateCommands,
    PeripheralCommands, ServiceCommands, SkillCommands,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum CompletionShell {
    #[value(name = "bash")]
    Bash,
    #[value(name = "fish")]
    Fish,
    #[value(name = "zsh")]
    Zsh,
    #[value(name = "powershell")]
    PowerShell,
    #[value(name = "elvish")]
    Elvish,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum EstopLevelArg {
    #[value(name = "kill-all")]
    KillAll,
    #[value(name = "network-kill")]
    NetworkKill,
    #[value(name = "domain-block")]
    DomainBlock,
    #[value(name = "tool-freeze")]
    ToolFreeze,
}

/// `ZeroClaw` - Zero overhead. Zero compromise. 100% Rust.
#[derive(Parser, Debug)]
#[command(name = "zeroclaw")]
#[command(author = "theonlyhennygod")]
#[command(version)]
#[command(about = "The fastest, smallest AI assistant.", long_about = None)]
struct Cli {
    #[arg(long, global = true)]
    config_dir: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize your workspace and configuration
    Onboard {
        /// Run the full interactive wizard (default is quick setup)
        #[arg(long)]
        interactive: bool,

        /// Overwrite existing config without confirmation
        #[arg(long)]
        force: bool,

        /// Reconfigure channels only (fast repair flow)
        #[arg(long)]
        channels_only: bool,

        /// API key (used in quick mode, ignored with --interactive)
        #[arg(long)]
        api_key: Option<String>,

        /// Provider name (used in quick mode, default: openrouter)
        #[arg(long)]
        provider: Option<String>,
        /// Model ID override (used in quick mode)
        #[arg(long)]
        model: Option<String>,
        /// Memory backend (sqlite, lucid, markdown, none) - used in quick mode, default: sqlite
        #[arg(long)]
        memory: Option<String>,
    },

    /// Start the AI agent loop
    #[command(long_about = "\
Start the AI agent loop.

Launches an interactive chat session with the configured AI provider. \
Use --message for single-shot queries without entering interactive mode.

Examples:
  zeroclaw agent                              # interactive session
  zeroclaw agent -m \"Summarize today's logs\"  # single message
  zeroclaw agent -p anthropic --model claude-sonnet-4-20250514
  zeroclaw agent --peripheral nucleo-f401re:/dev/ttyACM0")]
    Agent {
        /// Single message mode (don't enter interactive mode)
        #[arg(short, long)]
        message: Option<String>,

        /// Provider to use (openrouter, anthropic, openai, openai-codex)
        #[arg(short, long)]
        provider: Option<String>,

        /// Model to use
        #[arg(long)]
        model: Option<String>,

        /// Temperature (0.0 - 2.0)
        #[arg(short, long, default_value = "0.7", value_parser = parse_temperature)]
        temperature: f64,

        /// Attach a peripheral (board:path, e.g. nucleo-f401re:/dev/ttyACM0)
        #[arg(long)]
        peripheral: Vec<String>,
    },

    /// Start the gateway server (webhooks, websockets)
    #[command(long_about = "\
Start the gateway server (webhooks, websockets).

Runs the HTTP/WebSocket gateway that accepts incoming webhook events \
and WebSocket connections. Bind address defaults to the values in \
your config file (gateway.host / gateway.port).

Examples:
  zeroclaw gateway                  # use config defaults
  zeroclaw gateway -p 8080          # listen on port 8080
  zeroclaw gateway --host 0.0.0.0   # bind to all interfaces
  zeroclaw gateway -p 0             # random available port")]
    Gateway {
        /// Port to listen on (use 0 for random available port); defaults to config gateway.port
        #[arg(short, long)]
        port: Option<u16>,

        /// Host to bind to; defaults to config gateway.host
        #[arg(long)]
        host: Option<String>,
    },

    /// Start long-running autonomous runtime (gateway + channels + heartbeat + scheduler)
    #[command(long_about = "\
Start the long-running autonomous daemon.

Launches the full ZeroClaw runtime: gateway server, all configured \
channels (Telegram, Discord, Slack, etc.), heartbeat monitor, and \
the cron scheduler. This is the recommended way to run ZeroClaw in \
production or as an always-on assistant.

Use 'zeroclaw service install' to register the daemon as an OS \
service (systemd/launchd) for auto-start on boot.

Examples:
  zeroclaw daemon                   # use config defaults
  zeroclaw daemon -p 9090           # gateway on port 9090
  zeroclaw daemon --host 127.0.0.1  # localhost only")]
    Daemon {
        /// Port to listen on (use 0 for random available port); defaults to config gateway.port
        #[arg(short, long)]
        port: Option<u16>,

        /// Host to bind to; defaults to config gateway.host
        #[arg(long)]
        host: Option<String>,
    },

    /// Manage OS service lifecycle (launchd/systemd user service)
    Service {
        /// Init system to use: auto (detect), systemd, or openrc
        #[arg(long, default_value = "auto", value_parser = ["auto", "systemd", "openrc"])]
        service_init: String,

        #[command(subcommand)]
        service_command: ServiceCommands,
    },

    /// Run diagnostics for daemon/scheduler/channel freshness
    Doctor {
        #[command(subcommand)]
        doctor_command: Option<DoctorCommands>,
    },

    /// Show system status (full details)
    Status,

    /// Engage, inspect, and resume emergency-stop states.
    ///
    /// Examples:
    /// - `zeroclaw estop`
    /// - `zeroclaw estop --level network-kill`
    /// - `zeroclaw estop --level domain-block --domain "*.chase.com"`
    /// - `zeroclaw estop --level tool-freeze --tool shell --tool browser`
    /// - `zeroclaw estop status`
    /// - `zeroclaw estop resume --network`
    /// - `zeroclaw estop resume --domain "*.chase.com"`
    /// - `zeroclaw estop resume --tool shell`
    Estop {
        #[command(subcommand)]
        estop_command: Option<EstopSubcommands>,

        /// Level used when engaging estop from `zeroclaw estop`.
        #[arg(long, value_enum)]
        level: Option<EstopLevelArg>,

        /// Domain pattern(s) for `domain-block` (repeatable).
        #[arg(long = "domain")]
        domains: Vec<String>,

        /// Tool name(s) for `tool-freeze` (repeatable).
        #[arg(long = "tool")]
        tools: Vec<String>,
    },

    /// Configure and manage scheduled tasks
    #[command(long_about = "\
Configure and manage scheduled tasks.

Schedule recurring, one-shot, or interval-based tasks using cron \
expressions, RFC 3339 timestamps, durations, or fixed intervals.

Cron expressions use the standard 5-field format: \
'min hour day month weekday'. Timezones default to UTC; \
override with --tz and an IANA timezone name.

Examples:
  zeroclaw cron list
  zeroclaw cron add '0 9 * * 1-5' 'Good morning' --tz America/New_York
  zeroclaw cron add '*/30 * * * *' 'Check system health'
  zeroclaw cron add-at 2025-01-15T14:00:00Z 'Send reminder'
  zeroclaw cron add-every 60000 'Ping heartbeat'
  zeroclaw cron once 30m 'Run backup in 30 minutes'
  zeroclaw cron pause <task-id>
  zeroclaw cron update <task-id> --expression '0 8 * * *' --tz Europe/London")]
    Cron {
        #[command(subcommand)]
        cron_command: CronCommands,
    },

    /// Manage provider model catalogs
    Models {
        #[command(subcommand)]
        model_command: ModelCommands,
    },

    /// List supported AI providers
    Providers,

    /// Manage channels (telegram, discord, slack)
    #[command(long_about = "\
Manage communication channels.

Add, remove, list, and health-check channels that connect ZeroClaw \
to messaging platforms. Supported channel types: telegram, discord, \
slack, whatsapp, matrix, imessage, email.

Examples:
  zeroclaw channel list
  zeroclaw channel doctor
  zeroclaw channel add telegram '{\"bot_token\":\"...\",\"name\":\"my-bot\"}'
  zeroclaw channel remove my-bot
  zeroclaw channel bind-telegram zeroclaw_user")]
    Channel {
        #[command(subcommand)]
        channel_command: ChannelCommands,
    },

    /// Browse 50+ integrations
    Integrations {
        #[command(subcommand)]
        integration_command: IntegrationCommands,
    },

    /// Manage skills (user-defined capabilities)
    Skills {
        #[command(subcommand)]
        skill_command: SkillCommands,
    },

    /// Migrate data from other agent runtimes
    Migrate {
        #[command(subcommand)]
        migrate_command: MigrateCommands,
    },

    /// Manage provider subscription authentication profiles
    Auth {
        #[command(subcommand)]
        auth_command: AuthCommands,
    },

    /// Discover and introspect USB hardware
    #[command(long_about = "\
Discover and introspect USB hardware.

Enumerate connected USB devices, identify known development boards \
(STM32 Nucleo, Arduino, ESP32), and retrieve chip information via \
probe-rs / ST-Link.

Examples:
  zeroclaw hardware discover
  zeroclaw hardware introspect /dev/ttyACM0
  zeroclaw hardware info --chip STM32F401RETx")]
    Hardware {
        #[command(subcommand)]
        hardware_command: zeroclaw::HardwareCommands,
    },

    /// Manage hardware peripherals (STM32, RPi GPIO, etc.)
    #[command(long_about = "\
Manage hardware peripherals.

Add, list, flash, and configure hardware boards that expose tools \
to the agent (GPIO, sensors, actuators). Supported boards: \
nucleo-f401re, rpi-gpio, esp32, arduino-uno.

Examples:
  zeroclaw peripheral list
  zeroclaw peripheral add nucleo-f401re /dev/ttyACM0
  zeroclaw peripheral add rpi-gpio native
  zeroclaw peripheral flash --port /dev/cu.usbmodem12345
  zeroclaw peripheral flash-nucleo")]
    Peripheral {
        #[command(subcommand)]
        peripheral_command: zeroclaw::PeripheralCommands,
    },

    /// Manage agent memory (list, get, stats, clear)
    #[command(long_about = "\
Manage agent memory entries.

List, inspect, and clear memory entries stored by the agent. \
Supports filtering by category and session, pagination, and \
batch clearing with confirmation.

Examples:
  zeroclaw memory stats
  zeroclaw memory list
  zeroclaw memory list --category core --limit 10
  zeroclaw memory get <key>
  zeroclaw memory clear --category conversation --yes")]
    Memory {
        #[command(subcommand)]
        memory_command: MemoryCommands,
    },

    /// Manage configuration
    #[command(long_about = "\
Manage ZeroClaw configuration.

Inspect and export configuration settings. Use 'schema' to dump \
the full JSON Schema for the config file, which documents every \
available key, type, and default value.

Examples:
  zeroclaw config schema              # print JSON Schema to stdout
  zeroclaw config schema > schema.json")]
    Config {
        #[command(subcommand)]
        config_command: ConfigCommands,
    },

    /// Generate shell completion script to stdout
    #[command(long_about = "\
Generate shell completion scripts for `zeroclaw`.

The script is printed to stdout so it can be sourced directly:

Examples:
  source <(zeroclaw completions bash)
  zeroclaw completions zsh > ~/.zfunc/_zeroclaw
  zeroclaw completions fish > ~/.config/fish/completions/zeroclaw.fish")]
    Completions {
        /// Target shell
        #[arg(value_enum)]
        shell: CompletionShell,
    },

    /// Manage Composio integration (health checks, connections)
    #[command(long_about = "\
Manage Composio integration.

Check health status, verify connections, and troubleshoot Composio \
MCP integration issues. Useful for diagnosing OAuth connection \
problems and toolkit availability.

Examples:
  zeroclaw composio health
  zeroclaw composio health --verbose")]
    Composio {
        #[command(subcommand)]
        composio_command: ComposioCommands,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigCommands {
    /// Dump the full configuration JSON Schema to stdout
    Schema,
}

#[derive(Subcommand, Debug)]
enum EstopSubcommands {
    /// Print current estop status.
    Status,
    /// Resume from an engaged estop level.
    Resume {
        /// Resume only network kill.
        #[arg(long)]
        network: bool,
        /// Resume one or more blocked domain patterns.
        #[arg(long = "domain")]
        domains: Vec<String>,
        /// Resume one or more frozen tools.
        #[arg(long = "tool")]
        tools: Vec<String>,
        /// OTP code. If omitted and OTP is required, a prompt is shown.
        #[arg(long)]
        otp: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum AuthCommands {
    /// Login with OAuth (OpenAI Codex or Gemini)
    Login {
        /// Provider (`openai-codex` or `gemini`)
        #[arg(long)]
        provider: String,
        /// Profile name (default: default)
        #[arg(long, default_value = "default")]
        profile: String,
        /// Use OAuth device-code flow
        #[arg(long)]
        device_code: bool,
    },
    /// Complete OAuth by pasting redirect URL or auth code
    PasteRedirect {
        /// Provider (`openai-codex`)
        #[arg(long)]
        provider: String,
        /// Profile name (default: default)
        #[arg(long, default_value = "default")]
        profile: String,
        /// Full redirect URL or raw OAuth code
        #[arg(long)]
        input: Option<String>,
    },
    /// Paste setup token / auth token (for Anthropic subscription auth)
    PasteToken {
        /// Provider (`anthropic`)
        #[arg(long)]
        provider: String,
        /// Profile name (default: default)
        #[arg(long, default_value = "default")]
        profile: String,
        /// Token value (if omitted, read interactively)
        #[arg(long)]
        token: Option<String>,
        /// Auth kind override (`authorization` or `api-key`)
        #[arg(long)]
        auth_kind: Option<String>,
    },
    /// Alias for `paste-token` (interactive by default)
    SetupToken {
        /// Provider (`anthropic`)
        #[arg(long)]
        provider: String,
        /// Profile name (default: default)
        #[arg(long, default_value = "default")]
        profile: String,
    },
    /// Refresh OpenAI Codex access token using refresh token
    Refresh {
        /// Provider (`openai-codex`)
        #[arg(long)]
        provider: String,
        /// Profile name or profile id
        #[arg(long)]
        profile: Option<String>,
    },
    /// Remove auth profile
    Logout {
        /// Provider
        #[arg(long)]
        provider: String,
        /// Profile name (default: default)
        #[arg(long, default_value = "default")]
        profile: String,
    },
    /// Set active profile for a provider
    Use {
        /// Provider
        #[arg(long)]
        provider: String,
        /// Profile name or full profile id
        #[arg(long)]
        profile: String,
    },
    /// List auth profiles
    List,
    /// Show auth status with active profile and token expiry info
    Status,
}

#[derive(Subcommand, Debug)]
enum ModelCommands {
    /// Refresh and cache provider models
    Refresh {
        /// Provider name (defaults to configured default provider)
        #[arg(long)]
        provider: Option<String>,

        /// Force live refresh and ignore fresh cache
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand, Debug)]
enum DoctorCommands {
    /// Probe model catalogs across providers and report availability
    Models {
        /// Probe a specific provider only (default: all known providers)
        #[arg(long)]
        provider: Option<String>,

        /// Prefer cached catalogs when available (skip forced live refresh)
        #[arg(long)]
        use_cache: bool,
    },
    /// Query runtime trace events (tool diagnostics and model replies)
    Traces {
        /// Show a specific trace event by id
        #[arg(long)]
        id: Option<String>,
        /// Filter list output by event type
        #[arg(long)]
        event: Option<String>,
        /// Case-insensitive text match across message/payload
        #[arg(long)]
        contains: Option<String>,
        /// Maximum number of events to display
        #[arg(long, default_value = "20")]
        limit: usize,
    },
}

#[derive(Subcommand, Debug)]
enum MemoryCommands {
    /// List memory entries with optional filters
    List {
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        session: Option<String>,
        #[arg(long, default_value = "50")]
        limit: usize,
        #[arg(long, default_value = "0")]
        offset: usize,
    },
    /// Get a specific memory entry by key
    Get { key: String },
    /// Show memory backend statistics and health
    Stats,
    /// Clear memories by category, by key, or clear all
    Clear {
        /// Delete a single entry by key (supports prefix match)
        #[arg(long)]
        key: Option<String>,
        #[arg(long)]
        category: Option<String>,
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand, Debug)]
enum ComposioCommands {
    /// Check Composio MCP health and connection status
    Health {
        /// Show detailed information including all toolkits
        #[arg(long)]
        verbose: bool,
    },
    
    /// Connect a toolkit interactively via OAuth
    Connect {
        /// Toolkit to connect (e.g., gmail, github, slack)
        toolkit: String,
        
        /// User/entity ID (defaults to config value)
        #[arg(long)]
        user_id: Option<String>,
        
        /// Timeout in seconds (default: 120)
        #[arg(long, default_value = "120")]
        timeout: u64,
    },
    
    /// Disconnect a toolkit (revoke OAuth connection)
    Disconnect {
        /// Toolkit to disconnect (e.g., gmail, github, slack)
        toolkit: String,
        
        /// User/entity ID (defaults to config value)
        #[arg(long)]
        user_id: Option<String>,
        
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },
    
    /// List available toolkits from Composio
    ListToolkits {
        /// Show only connected toolkits
        #[arg(long)]
        connected: bool,
        
        /// Show only disconnected toolkits
        #[arg(long)]
        disconnected: bool,
    },
    
    /// List connected accounts
    ListConnections {
        /// Filter by toolkit (e.g., gmail, github)
        #[arg(long)]
        toolkit: Option<String>,
        
        /// Show detailed information
        #[arg(long)]
        verbose: bool,
    },
    
    /// Show overall Composio integration status
    Status {
        /// Show detailed information
        #[arg(long)]
        verbose: bool,
    },
    
    /// List available tools from MCP server
    ListTools {
        /// Filter by toolkit (e.g., gmail, github)
        #[arg(long)]
        toolkit: Option<String>,
        
        /// Show tool schemas
        #[arg(long)]
        schema: bool,
    },
    
    /// Refresh MCP URL with current toolkit configuration
    Refresh {
        /// Force refresh even if MCP URL already exists
        #[arg(long)]
        force: bool,
        
        /// Update config file with new MCP URL
        #[arg(long)]
        save: bool,
    },
    
    /// Test a Composio MCP tool
    Test {
        /// Tool name to test (e.g., GMAIL_SEND_EMAIL)
        tool: String,
        
        /// Tool arguments as JSON (e.g., '{"to":"test@example.com"}')
        #[arg(long)]
        args: Option<String>,
        
        /// Show detailed execution information
        #[arg(long)]
        verbose: bool,
    },
}


#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> Result<()> {
    // Install default crypto provider for Rustls TLS.
    // This prevents the error: "could not automatically determine the process-level CryptoProvider"
    // when both aws-lc-rs and ring features are available (or neither is explicitly selected).
    if let Err(e) = rustls::crypto::ring::default_provider().install_default() {
        eprintln!("Warning: Failed to install default crypto provider: {e:?}");
    }

    let cli = Cli::parse();

    if let Some(config_dir) = &cli.config_dir {
        if config_dir.trim().is_empty() {
            bail!("--config-dir cannot be empty");
        }
        std::env::set_var("ZEROCLAW_CONFIG_DIR", config_dir);
    }

    // Completions must remain stdout-only and should not load config or initialize logging.
    // This avoids warnings/log lines corrupting sourced completion scripts.
    if let Commands::Completions { shell } = &cli.command {
        let mut stdout = std::io::stdout().lock();
        write_shell_completion(*shell, &mut stdout)?;
        return Ok(());
    }

    // Initialize logging - respects RUST_LOG env var, defaults to INFO
    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Onboard runs quick setup by default, or the interactive wizard with --interactive.
    // The onboard wizard uses reqwest::blocking internally, which creates its own
    // Tokio runtime. To avoid "Cannot drop a runtime in a context where blocking is
    // not allowed", we run the wizard on a blocking thread via spawn_blocking.
    if let Commands::Onboard {
        interactive,
        force,
        channels_only,
        api_key,
        provider,
        model,
        memory,
    } = &cli.command
    {
        let interactive = *interactive;
        let force = *force;
        let channels_only = *channels_only;
        let api_key = api_key.clone();
        let provider = provider.clone();
        let model = model.clone();
        let memory = memory.clone();

        if interactive && channels_only {
            bail!("Use either --interactive or --channels-only, not both");
        }
        if channels_only
            && (api_key.is_some() || provider.is_some() || model.is_some() || memory.is_some())
        {
            bail!("--channels-only does not accept --api-key, --provider, --model, or --memory");
        }
        if channels_only && force {
            bail!("--channels-only does not accept --force");
        }
        let config = if channels_only {
            onboard::run_channels_repair_wizard().await
        } else if interactive {
            onboard::run_wizard(force).await
        } else {
            onboard::run_quick_setup(
                api_key.as_deref(),
                provider.as_deref(),
                model.as_deref(),
                memory.as_deref(),
                force,
            )
            .await
        }?;
        // Auto-start channels if user said yes during wizard
        if std::env::var("ZEROCLAW_AUTOSTART_CHANNELS").as_deref() == Ok("1") {
            channels::start_channels(config).await?;
        }
        return Ok(());
    }

    // All other commands need config loaded first
    let mut config = Config::load_or_init().await?;
    config.apply_env_overrides();
    observability::runtime_trace::init_from_config(&config.observability, &config.workspace_dir);
    if config.security.otp.enabled {
        let config_dir = config
            .config_path
            .parent()
            .context("Config path must have a parent directory")?;
        let store = security::SecretStore::new(config_dir, config.secrets.encrypt);
        let (_validator, enrollment_uri) =
            security::OtpValidator::from_config(&config.security.otp, config_dir, &store)?;
        if let Some(uri) = enrollment_uri {
            println!("Initialized OTP secret for ZeroClaw.");
            println!("Enrollment URI: {uri}");
        }
    }

    match cli.command {
        Commands::Onboard { .. } => unreachable!(),
        Commands::Completions { .. } => unreachable!(),

        Commands::Agent {
            message,
            provider,
            model,
            temperature,
            peripheral,
        } => {
            // Set UI mode for Composio MCP onboarding
            std::env::set_var("ZEROCLAW_UI_MODE", "cli");
            
            agent::run(
                config,
                message,
                provider,
                model,
                temperature,
                peripheral,
                true,
            )
            .await
            .map(|_| ())
        }

        Commands::Gateway { port, host } => {
            // Set UI mode for Composio MCP onboarding
            std::env::set_var("ZEROCLAW_UI_MODE", "server");
            
            let port = port.unwrap_or(config.gateway.port);
            let host = host.unwrap_or_else(|| config.gateway.host.clone());
            if port == 0 {
                info!("🚀 Starting ZeroClaw Gateway on {host} (random port)");
            } else {
                info!("🚀 Starting ZeroClaw Gateway on {host}:{port}");
            }
            gateway::run_gateway(&host, port, config).await
        }

        Commands::Daemon { port, host } => {
            // Set UI mode for Composio MCP onboarding
            std::env::set_var("ZEROCLAW_UI_MODE", "server");
            
            let port = port.unwrap_or(config.gateway.port);
            let host = host.unwrap_or_else(|| config.gateway.host.clone());
            if port == 0 {
                info!("🧠 Starting ZeroClaw Daemon on {host} (random port)");
            } else {
                info!("🧠 Starting ZeroClaw Daemon on {host}:{port}");
            }
            daemon::run(config, host, port).await
        }

        Commands::Status => {
            println!("🦀 ZeroClaw Status");
            println!();
            println!("Version:     {}", env!("CARGO_PKG_VERSION"));
            println!("Workspace:   {}", config.workspace_dir.display());
            println!("Config:      {}", config.config_path.display());
            println!();
            println!(
                "🤖 Provider:      {}",
                config.default_provider.as_deref().unwrap_or("openrouter")
            );
            println!(
                "   Model:         {}",
                config.default_model.as_deref().unwrap_or("(default)")
            );
            println!("📊 Observability:  {}", config.observability.backend);
            println!(
                "🧾 Trace storage:  {} ({})",
                config.observability.runtime_trace_mode, config.observability.runtime_trace_path
            );
            println!("🛡️  Autonomy:      {:?}", config.autonomy.level);
            println!("⚙️  Runtime:       {}", config.runtime.kind);
            let effective_memory_backend = memory::effective_memory_backend_name(
                &config.memory.backend,
                Some(&config.storage.provider.config),
            );
            println!(
                "💓 Heartbeat:      {}",
                if config.heartbeat.enabled {
                    format!("every {}min", config.heartbeat.interval_minutes)
                } else {
                    "disabled".into()
                }
            );
            println!(
                "🧠 Memory:         {} (auto-save: {})",
                effective_memory_backend,
                if config.memory.auto_save { "on" } else { "off" }
            );

            println!();
            println!("Security:");
            println!("  Workspace only:    {}", config.autonomy.workspace_only);
            println!(
                "  Allowed roots:     {}",
                if config.autonomy.allowed_roots.is_empty() {
                    "(none)".to_string()
                } else {
                    config.autonomy.allowed_roots.join(", ")
                }
            );
            println!(
                "  Allowed commands:  {}",
                config.autonomy.allowed_commands.join(", ")
            );
            println!(
                "  Max actions/hour:  {}",
                config.autonomy.max_actions_per_hour
            );
            println!(
                "  Max cost/day:      ${:.2}",
                f64::from(config.autonomy.max_cost_per_day_cents) / 100.0
            );
            println!("  OTP enabled:       {}", config.security.otp.enabled);
            println!("  E-stop enabled:    {}", config.security.estop.enabled);
            println!();
            println!("Channels:");
            println!("  CLI:      ✅ always");
            for (channel, configured) in config.channels_config.channels() {
                println!(
                    "  {:9} {}",
                    channel.name(),
                    if configured {
                        "✅ configured"
                    } else {
                        "❌ not configured"
                    }
                );
            }
            println!();
            println!("Peripherals:");
            println!(
                "  Enabled:   {}",
                if config.peripherals.enabled {
                    "yes"
                } else {
                    "no"
                }
            );
            println!("  Boards:    {}", config.peripherals.boards.len());

            Ok(())
        }

        Commands::Estop {
            estop_command,
            level,
            domains,
            tools,
        } => handle_estop_command(&config, estop_command, level, domains, tools),

        Commands::Cron { cron_command } => cron::handle_command(cron_command, &config),

        Commands::Models { model_command } => match model_command {
            ModelCommands::Refresh { provider, force } => {
                onboard::run_models_refresh(&config, provider.as_deref(), force).await
            }
        },

        Commands::Providers => {
            let providers = providers::list_providers();
            let current = config
                .default_provider
                .as_deref()
                .unwrap_or("openrouter")
                .trim()
                .to_ascii_lowercase();
            println!("Supported providers ({} total):\n", providers.len());
            println!("  ID (use in config)  DESCRIPTION");
            println!("  ─────────────────── ───────────");
            for p in &providers {
                let is_active = p.name.eq_ignore_ascii_case(&current)
                    || p.aliases
                        .iter()
                        .any(|alias| alias.eq_ignore_ascii_case(&current));
                let marker = if is_active { " (active)" } else { "" };
                let local_tag = if p.local { " [local]" } else { "" };
                let aliases = if p.aliases.is_empty() {
                    String::new()
                } else {
                    format!("  (aliases: {})", p.aliases.join(", "))
                };
                println!(
                    "  {:<19} {}{}{}{}",
                    p.name, p.display_name, local_tag, marker, aliases
                );
            }
            println!("\n  custom:<URL>   Any OpenAI-compatible endpoint");
            println!("  anthropic-custom:<URL>  Any Anthropic-compatible endpoint");
            Ok(())
        }

        Commands::Service {
            service_command,
            service_init,
        } => {
            let init_system = service_init.parse()?;
            service::handle_command(&service_command, &config, init_system)
        }

        Commands::Doctor { doctor_command } => match doctor_command {
            Some(DoctorCommands::Models {
                provider,
                use_cache,
            }) => doctor::run_models(&config, provider.as_deref(), use_cache).await,
            Some(DoctorCommands::Traces {
                id,
                event,
                contains,
                limit,
            }) => doctor::run_traces(
                &config,
                id.as_deref(),
                event.as_deref(),
                contains.as_deref(),
                limit,
            ),
            None => doctor::run(&config),
        },

        Commands::Channel { channel_command } => match channel_command {
            ChannelCommands::Start => channels::start_channels(config).await,
            ChannelCommands::Doctor => channels::doctor_channels(config).await,
            other => channels::handle_command(other, &config).await,
        },

        Commands::Integrations {
            integration_command,
        } => integrations::handle_command(integration_command, &config),

        Commands::Skills { skill_command } => skills::handle_command(skill_command, &config),

        Commands::Migrate { migrate_command } => {
            migration::handle_command(migrate_command, &config).await
        }

        Commands::Memory { memory_command } => {
            memory::cli::handle_command(memory_command, &config).await
        }

        Commands::Auth { auth_command } => handle_auth_command(auth_command, &config).await,

        Commands::Hardware { hardware_command } => {
            hardware::handle_command(hardware_command.clone(), &config)
        }

        Commands::Peripheral { peripheral_command } => {
            peripherals::handle_command(peripheral_command.clone(), &config).await
        }

        Commands::Composio { composio_command } => {
            handle_composio_command(composio_command, &config).await
        }

        Commands::Config { config_command } => match config_command {
            ConfigCommands::Schema => {
                let schema = schemars::schema_for!(config::Config);
                println!(
                    "{}",
                    serde_json::to_string_pretty(&schema).expect("failed to serialize JSON Schema")
                );
                Ok(())
            }
        },
    }
}

fn handle_estop_command(
    config: &Config,
    estop_command: Option<EstopSubcommands>,
    level: Option<EstopLevelArg>,
    domains: Vec<String>,
    tools: Vec<String>,
) -> Result<()> {
    if !config.security.estop.enabled {
        bail!("Emergency stop is disabled. Enable [security.estop].enabled = true in config.toml");
    }

    let config_dir = config
        .config_path
        .parent()
        .context("Config path must have a parent directory")?;
    let mut manager = security::EstopManager::load(&config.security.estop, config_dir)?;

    match estop_command {
        Some(EstopSubcommands::Status) => {
            print_estop_status(&manager.status());
            Ok(())
        }
        Some(EstopSubcommands::Resume {
            network,
            domains,
            tools,
            otp,
        }) => {
            let selector = build_resume_selector(network, domains, tools)?;
            let mut otp_code = otp;
            let otp_validator = if config.security.estop.require_otp_to_resume {
                if !config.security.otp.enabled {
                    bail!(
                        "security.estop.require_otp_to_resume=true but security.otp.enabled=false"
                    );
                }
                if otp_code.is_none() {
                    let entered = Password::new()
                        .with_prompt("Enter OTP code")
                        .allow_empty_password(false)
                        .interact()?;
                    otp_code = Some(entered);
                }

                let store = security::SecretStore::new(config_dir, config.secrets.encrypt);
                let (validator, enrollment_uri) =
                    security::OtpValidator::from_config(&config.security.otp, config_dir, &store)?;
                if let Some(uri) = enrollment_uri {
                    println!("Initialized OTP secret for ZeroClaw.");
                    println!("Enrollment URI: {uri}");
                }
                Some(validator)
            } else {
                None
            };

            manager.resume(selector, otp_code.as_deref(), otp_validator.as_ref())?;
            println!("Estop resume completed.");
            print_estop_status(&manager.status());
            Ok(())
        }
        None => {
            let engage_level = build_engage_level(level, domains, tools)?;
            manager.engage(engage_level)?;
            println!("Estop engaged.");
            print_estop_status(&manager.status());
            Ok(())
        }
    }
}

fn build_engage_level(
    level: Option<EstopLevelArg>,
    domains: Vec<String>,
    tools: Vec<String>,
) -> Result<security::EstopLevel> {
    let requested = level.unwrap_or(EstopLevelArg::KillAll);
    match requested {
        EstopLevelArg::KillAll => {
            if !domains.is_empty() || !tools.is_empty() {
                bail!("--domain/--tool are only valid with --level domain-block/tool-freeze");
            }
            Ok(security::EstopLevel::KillAll)
        }
        EstopLevelArg::NetworkKill => {
            if !domains.is_empty() || !tools.is_empty() {
                bail!("--domain/--tool are not valid with --level network-kill");
            }
            Ok(security::EstopLevel::NetworkKill)
        }
        EstopLevelArg::DomainBlock => {
            if domains.is_empty() {
                bail!("--level domain-block requires at least one --domain");
            }
            if !tools.is_empty() {
                bail!("--tool is not valid with --level domain-block");
            }
            Ok(security::EstopLevel::DomainBlock(domains))
        }
        EstopLevelArg::ToolFreeze => {
            if tools.is_empty() {
                bail!("--level tool-freeze requires at least one --tool");
            }
            if !domains.is_empty() {
                bail!("--domain is not valid with --level tool-freeze");
            }
            Ok(security::EstopLevel::ToolFreeze(tools))
        }
    }
}

fn build_resume_selector(
    network: bool,
    domains: Vec<String>,
    tools: Vec<String>,
) -> Result<security::ResumeSelector> {
    let selected =
        usize::from(network) + usize::from(!domains.is_empty()) + usize::from(!tools.is_empty());
    if selected > 1 {
        bail!("Use only one of --network, --domain, or --tool for estop resume");
    }
    if network {
        return Ok(security::ResumeSelector::Network);
    }
    if !domains.is_empty() {
        return Ok(security::ResumeSelector::Domains(domains));
    }
    if !tools.is_empty() {
        return Ok(security::ResumeSelector::Tools(tools));
    }
    Ok(security::ResumeSelector::KillAll)
}

fn print_estop_status(state: &security::EstopState) {
    println!("Estop status:");
    println!(
        "  engaged:        {}",
        if state.is_engaged() { "yes" } else { "no" }
    );
    println!(
        "  kill_all:       {}",
        if state.kill_all { "active" } else { "inactive" }
    );
    println!(
        "  network_kill:   {}",
        if state.network_kill {
            "active"
        } else {
            "inactive"
        }
    );
    if state.blocked_domains.is_empty() {
        println!("  domain_blocks:  (none)");
    } else {
        println!("  domain_blocks:  {}", state.blocked_domains.join(", "));
    }
    if state.frozen_tools.is_empty() {
        println!("  tool_freeze:    (none)");
    } else {
        println!("  tool_freeze:    {}", state.frozen_tools.join(", "));
    }
    if let Some(updated_at) = &state.updated_at {
        println!("  updated_at:     {updated_at}");
    }
}

async fn handle_composio_command(command: ComposioCommands, config: &Config) -> Result<()> {
    match command {
        ComposioCommands::Health { verbose } => {
            // Check if Composio is enabled
            if !config.composio.enabled {
                println!("❌ Composio is disabled in configuration");
                println!("\nTo enable Composio:");
                println!("  1. Run: zeroclaw onboard");
                println!("  2. Select 'Composio (managed OAuth)' when prompted");
                return Ok(());
            }

            // Check if MCP is configured
            if !config.composio.mcp.enabled {
                println!("⚠️  Composio is enabled but MCP integration is disabled");
                println!("\nTo enable MCP:");
                println!("  1. Run: zeroclaw onboard");
                println!("  2. Enable MCP integration when prompted");
                return Ok(());
            }

            // Validate configuration
            use zeroclaw::composio::validation::validate_mcp_config;
            
            if let Err(e) = validate_mcp_config(
                config.composio.mcp.enabled,
                &config.composio.mcp.mcp_url,
                &config.composio.mcp.server_id,
                &config.composio.mcp.toolkits,
            ) {
                println!("❌ Configuration validation failed:");
                println!("   {}", e);
                return Ok(());
            }

            println!("✓ Configuration is valid");

            // Get API key
            let api_key = match &config.composio.api_key {
                Some(key) => key.clone(),
                None => {
                    println!("❌ Composio API key not configured");
                    return Ok(());
                }
            };

            // Create clients
            use zeroclaw::composio::{ComposioRestClient, check_mcp_health};
            use zeroclaw::mcp::ComposioMcpClient;
            use std::sync::Arc;

            let rest_client = Arc::new(ComposioRestClient::new(api_key.clone()));
            
            let user_id = config.composio.mcp.user_id.clone().unwrap_or_else(|| "default".to_string());
            
            let mcp_client = if let Some(mcp_url) = &config.composio.mcp.mcp_url {
                Arc::new(ComposioMcpClient::new_with_mcp_url(
                    api_key,
                    mcp_url.clone(),
                    config.composio.mcp.server_id.clone(),
                    Some(user_id.clone()),
                    std::time::Duration::from_secs(config.composio.mcp.tools_cache_ttl_secs),
                ))
            } else if let Some(server_id) = &config.composio.mcp.server_id {
                Arc::new(ComposioMcpClient::new(
                    api_key,
                    server_id.clone(),
                    user_id.clone(),
                ))
            } else {
                println!("❌ Neither mcp_url nor server_id is configured");
                return Ok(());
            };

            // Perform health check
            println!("\n🔍 Checking Composio MCP health...\n");
            
            let health = check_mcp_health(
                &mcp_client,
                rest_client,
                &user_id,
                &config.composio.mcp.toolkits,
            ).await;

            // Display results
            println!("{}", health.status_message());

            if verbose {
                println!("\nConfiguration:");
                println!("  User ID: {}", user_id);
                println!("  Toolkits: {}", config.composio.mcp.toolkits.join(", "));
                if let Some(url) = &config.composio.mcp.mcp_url {
                    println!("  MCP URL: {}", url);
                }
                if let Some(server_id) = &config.composio.mcp.server_id {
                    println!("  Server ID: {}", server_id);
                }
                
                if !health.connected_toolkits.is_empty() {
                    println!("\nConnected Toolkits:");
                    for toolkit in &health.connected_toolkits {
                        println!("  ✓ {}", toolkit);
                    }
                }
                
                if !health.disconnected_toolkits.is_empty() {
                    println!("\nDisconnected Toolkits:");
                    for toolkit in &health.disconnected_toolkits {
                        println!("  ✗ {}", toolkit);
                        println!("    Run: zeroclaw agent");
                        println!("    Then use a {} tool to trigger OAuth", toolkit);
                    }
                }
            }

            if !health.is_healthy() {
                std::process::exit(1);
            }

            Ok(())
        }
        
        ComposioCommands::Connect { toolkit, user_id, timeout } => {
            handle_composio_connect(config, &toolkit, user_id.as_deref(), timeout).await
        }
        
        ComposioCommands::Disconnect { toolkit, user_id, yes } => {
            handle_composio_disconnect(config, &toolkit, user_id.as_deref(), yes).await
        }
        
        ComposioCommands::ListToolkits { connected, disconnected } => {
            handle_composio_list_toolkits(config, connected, disconnected).await
        }
        
        ComposioCommands::ListConnections { toolkit, verbose } => {
            handle_composio_list_connections(config, toolkit.as_deref(), verbose).await
        }
        
        ComposioCommands::Status { verbose } => {
            handle_composio_status(config, verbose).await
        }
        
        ComposioCommands::ListTools { toolkit, schema } => {
            handle_composio_list_tools(config, toolkit.as_deref(), schema).await
        }
        
        ComposioCommands::Refresh { force, save } => {
            handle_composio_refresh(config, force, save).await
        }
        
        ComposioCommands::Test { tool, args, verbose } => {
            handle_composio_test(config, &tool, args.as_deref(), verbose).await
        }
    }
}

async fn handle_composio_connect(
    config: &Config,
    toolkit: &str,
    user_id_override: Option<&str>,
    timeout_secs: u64,
) -> Result<()> {
    use zeroclaw::composio::{ComposioRestClient, normalize_toolkit_slug, CliOnboarding, OnboardingUx};
    use std::sync::Arc;
    
    // Check if Composio is enabled
    if !config.composio.enabled {
        bail!("Composio is not enabled. Run 'zeroclaw onboard' to configure.");
    }
    
    // Get API key
    let api_key = config.composio.api_key.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Composio API key not configured"))?;
    
    // Normalize toolkit slug
    let toolkit_slug = normalize_toolkit_slug(toolkit);
    
    // Get user ID
    let user_id = user_id_override
        .map(String::from)
        .or_else(|| config.composio.mcp.user_id.clone())
        .unwrap_or_else(|| "default".to_string());
    
    println!("🔗 Connecting {} for user '{}'...\n", toolkit_slug, user_id);
    
    // Create REST client
    let rest_client = Arc::new(ComposioRestClient::new(api_key.clone()));
    
    // Check if already connected
    match rest_client.list_connected_accounts(Some(&toolkit_slug), Some(&user_id)).await {
        Ok(accounts) => {
            let has_active = accounts.iter().any(|a| a.status.eq_ignore_ascii_case("ACTIVE"));
            if has_active {
                println!("✓ {} is already connected!", toolkit_slug);
                return Ok(());
            }
        }
        Err(_) => {
            // Continue with connection
        }
    }
    
    // Determine UX mode
    let auto_open = !std::env::var("ZEROCLAW_NO_BROWSER")
        .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    let ux = if auto_open {
        OnboardingUx::CliAutoOpen
    } else {
        OnboardingUx::CliPrintOnly
    };
    
    // Create onboarding handler
    let onboarding = CliOnboarding::new(rest_client, ux);
    
    // Perform connection with custom timeout
    println!("Opening OAuth authorization page...");
    
    // Get connection URL
    let link = onboarding.get_connection_url(&toolkit_slug, &user_id).await?;
    
    println!("\n🔗 {} OAuth Required", toolkit_slug.to_uppercase());
    println!("Open this URL in your browser:");
    println!("  {}", link);
    
    // Try to open browser
    if auto_open {
        if let Err(e) = open::that(&link) {
            eprintln!("⚠ Could not auto-open browser: {}", e);
            println!("Please open the URL manually.");
        } else {
            println!("✓ Browser opened automatically");
        }
    }
    
    // Poll for connection
    println!("\n⏳ Waiting for authorization (timeout: {}s)...", timeout_secs);
    println!("Complete the OAuth flow in your browser to continue.\n");
    
    match onboarding.poll_until_connected(&toolkit_slug, &user_id, timeout_secs).await {
        Ok(()) => {
            println!("✓ {} connected successfully!", toolkit_slug.to_uppercase());
            println!("\nYou can now use {} tools in your agent.", toolkit_slug);
            Ok(())
        }
        Err(e) => {
            eprintln!("\n❌ Connection failed: {}", e);
            eprintln!("\nTroubleshooting:");
            eprintln!("  1. Make sure you completed the OAuth flow in your browser");
            eprintln!("  2. Check that you authorized the correct account");
            eprintln!("  3. Try again with a longer timeout: --timeout 300");
            eprintln!("  4. Check your Composio dashboard: https://app.composio.dev");
            std::process::exit(1);
        }
    }
}

async fn handle_composio_list_toolkits(
    config: &Config,
    connected_only: bool,
    disconnected_only: bool,
) -> Result<()> {
    use zeroclaw::composio::ComposioRestClient;
    use std::sync::Arc;
    
    // Check if Composio is enabled
    if !config.composio.enabled {
        bail!("Composio is not enabled. Run 'zeroclaw onboard' to configure.");
    }
    
    // Get API key
    let api_key = config.composio.api_key.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Composio API key not configured"))?;
    
    // Get user ID
    let user_id = config.composio.mcp.user_id.clone().unwrap_or_else(|| "default".to_string());
    
    // Create REST client
    let rest_client = Arc::new(ComposioRestClient::new(api_key.clone()));
    
    // Get configured toolkits
    let configured_toolkits = &config.composio.mcp.toolkits;
    
    if configured_toolkits.is_empty() {
        println!("No toolkits configured.");
        println!("\nTo add toolkits:");
        println!("  1. Run: zeroclaw onboard");
        println!("  2. Enable MCP and select toolkits");
        return Ok(());
    }
    
    println!("Configured Toolkits:\n");
    
    let mut connected = Vec::new();
    let mut disconnected = Vec::new();
    
    // Check connection status for each toolkit
    for toolkit in configured_toolkits {
        match rest_client.list_connected_accounts(Some(toolkit), Some(&user_id)).await {
            Ok(accounts) => {
                let has_active = accounts.iter().any(|a| a.status.eq_ignore_ascii_case("ACTIVE"));
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
    
    // Display based on filters
    if !connected_only && !disconnected.is_empty() {
        println!("Disconnected:");
        for toolkit in &disconnected {
            println!("  ✗ {}", toolkit);
        }
        if !disconnected_only {
            println!();
        }
    }
    
    if !disconnected_only && !connected.is_empty() {
        println!("Connected:");
        for toolkit in &connected {
            println!("  ✓ {}", toolkit);
        }
    }
    
    // Show connection command for disconnected
    if !disconnected.is_empty() && !connected_only {
        println!("\nTo connect a toolkit:");
        println!("  zeroclaw composio connect <toolkit>");
        println!("\nExample:");
        println!("  zeroclaw composio connect {}", disconnected[0]);
    }
    
    Ok(())
}

async fn handle_composio_list_connections(
    config: &Config,
    toolkit_filter: Option<&str>,
    verbose: bool,
) -> Result<()> {
    use zeroclaw::composio::ComposioRestClient;
    use std::sync::Arc;
    
    // Check if Composio is enabled
    if !config.composio.enabled {
        bail!("Composio is not enabled. Run 'zeroclaw onboard' to configure.");
    }
    
    // Get API key
    let api_key = config.composio.api_key.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Composio API key not configured"))?;
    
    // Get user ID
    let user_id = config.composio.mcp.user_id.clone().unwrap_or_else(|| "default".to_string());
    
    // Create REST client
    let rest_client = Arc::new(ComposioRestClient::new(api_key.clone()));
    
    // List connections
    let accounts = rest_client.list_connected_accounts(toolkit_filter, Some(&user_id)).await?;
    
    if accounts.is_empty() {
        if let Some(toolkit) = toolkit_filter {
            println!("No connections found for toolkit: {}", toolkit);
        } else {
            println!("No connections found.");
        }
        println!("\nTo connect a toolkit:");
        println!("  zeroclaw composio connect <toolkit>");
        return Ok(());
    }
    
    println!("Connected Accounts:\n");
    
    for account in &accounts {
        let status_icon = if account.status.eq_ignore_ascii_case("ACTIVE") {
            "✓"
        } else {
            "✗"
        };
        
        let toolkit_name = account.toolkit_slug().unwrap_or("unknown");
        
        println!("{} {} ({})", status_icon, toolkit_name, account.status);
        
        if verbose {
            println!("  ID: {}", account.id);
            if let Some(toolkit) = &account.toolkit {
                if let Some(name) = &toolkit.name {
                    println!("  Name: {}", name);
                }
            }
            println!();
        }
    }
    
    if !verbose {
        println!("\nUse --verbose for more details");
    }
    
    Ok(())
}

async fn handle_composio_disconnect(
    config: &Config,
    toolkit: &str,
    user_id_override: Option<&str>,
    skip_confirm: bool,
) -> Result<()> {
    use zeroclaw::composio::{ComposioRestClient, normalize_toolkit_slug};
    use std::sync::Arc;
    use std::io::{self, Write};
    
    // Check if Composio is enabled
    if !config.composio.enabled {
        bail!("Composio is not enabled. Run 'zeroclaw onboard' to configure.");
    }
    
    // Get API key
    let api_key = config.composio.api_key.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Composio API key not configured"))?;
    
    // Normalize toolkit slug
    let toolkit_slug = normalize_toolkit_slug(toolkit);
    
    // Get user ID
    let user_id = user_id_override
        .map(String::from)
        .or_else(|| config.composio.mcp.user_id.clone())
        .unwrap_or_else(|| "default".to_string());
    
    println!("🔌 Disconnecting {} for user '{}'...\n", toolkit_slug, user_id);
    
    // Create REST client
    let rest_client = Arc::new(ComposioRestClient::new(api_key.clone()));
    
    // Check if connected
    let accounts = rest_client.list_connected_accounts(Some(&toolkit_slug), Some(&user_id)).await?;
    
    if accounts.is_empty() {
        println!("⚠️  {} is not connected", toolkit_slug);
        return Ok(());
    }
    
    let active_accounts: Vec<_> = accounts.iter()
        .filter(|a| a.status.eq_ignore_ascii_case("ACTIVE"))
        .collect();
    
    if active_accounts.is_empty() {
        println!("⚠️  {} has no active connections", toolkit_slug);
        return Ok(());
    }
    
    // Confirm disconnection
    if !skip_confirm {
        println!("This will disconnect {} connection(s) for {}:", active_accounts.len(), toolkit_slug);
        for account in &active_accounts {
            println!("  • Connection ID: {}", account.id);
        }
        println!();
        
        print!("Are you sure you want to disconnect? (y/N): ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if !input.trim().eq_ignore_ascii_case("y") && !input.trim().eq_ignore_ascii_case("yes") {
            println!("Cancelled.");
            return Ok(());
        }
    }
    
    // Disconnect each account
    println!("\nDisconnecting...");
    
    let mut success_count = 0;
    let mut error_count = 0;
    
    for account in &active_accounts {
        match rest_client.delete_connected_account(&account.id).await {
            Ok(()) => {
                println!("  ✓ Disconnected connection {}", account.id);
                success_count += 1;
            }
            Err(e) => {
                eprintln!("  ✗ Failed to disconnect {}: {}", account.id, e);
                error_count += 1;
            }
        }
    }
    
    println!();
    if error_count == 0 {
        println!("✓ Successfully disconnected {} from {}", toolkit_slug, user_id);
        println!("\nTo reconnect:");
        println!("  zeroclaw composio connect {}", toolkit_slug);
    } else {
        eprintln!("⚠️  Disconnected {} connection(s), {} failed", success_count, error_count);
        std::process::exit(1);
    }
    
    Ok(())
}

async fn handle_composio_refresh(
    config: &Config,
    force: bool,
    save: bool,
) -> Result<()> {
    use zeroclaw::composio::ComposioRestBlockingClient;
    
    // Check if Composio is enabled
    if !config.composio.enabled {
        bail!("Composio is not enabled. Run 'zeroclaw onboard' to configure.");
    }
    
    // Check if MCP is enabled
    if !config.composio.mcp.enabled {
        bail!("Composio MCP is not enabled. Run 'zeroclaw onboard' to configure.");
    }
    
    // Get API key
    let api_key = config.composio.api_key.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Composio API key not configured"))?;
    
    // Check if MCP URL already exists
    if config.composio.mcp.mcp_url.is_some() && !force {
        println!("⚠️  MCP URL already exists in configuration");
        println!("\nCurrent URL: {}", config.composio.mcp.mcp_url.as_ref().unwrap());
        println!("\nUse --force to regenerate anyway");
        return Ok(());
    }
    
    // Get toolkits
    let toolkits = &config.composio.mcp.toolkits;
    if toolkits.is_empty() {
        bail!("No toolkits configured. Run 'zeroclaw onboard' to add toolkits.");
    }
    
    // Get user ID
    let user_id = config.composio.mcp.user_id.clone().unwrap_or_else(|| "default".to_string());
    
    println!("🔄 Generating new MCP URL...");
    println!("  User ID: {}", user_id);
    println!("  Toolkits: {}", toolkits.join(", "));
    println!();
    
    // Create blocking client
    let client = ComposioRestBlockingClient::new(api_key.clone());
    
    // Generate MCP URL
    let mcp_url = if user_id.starts_with("trs_") {
        // Tool Router Session - construct URL directly
        let toolkits_param = toolkits.join(",");
        format!(
            "https://backend.composio.dev/tool_router/{}/mcp?include_composio_helper_actions=true&user_id={}&toolkits={}",
            user_id, user_id, toolkits_param
        )
    } else {
        // Regular user ID - use API
        client.generate_mcp_url(toolkits.clone(), &user_id)?
    };
    
    println!("✓ MCP URL generated successfully\n");
    println!("New MCP URL:");
    println!("  {}\n", mcp_url);
    
    if save {
        println!("⚠️  Automatic config saving is not yet implemented");
        println!("\nTo update your configuration manually:");
        println!("  1. Edit your config.toml file");
        println!("  2. Update [composio.mcp].mcp_url with the URL above");
        println!("  3. Restart your agent or gateway");
    } else {
        println!("To save this URL to your configuration:");
        println!("  1. Edit your config.toml file");
        println!("  2. Update [composio.mcp].mcp_url with the URL above");
        println!("  3. Or run: zeroclaw composio refresh --save (when implemented)");
    }
    
    Ok(())
}

async fn handle_composio_test(
    config: &Config,
    tool_name: &str,
    args_json: Option<&str>,
    verbose: bool,
) -> Result<()> {
    use zeroclaw::mcp::ComposioMcpClient;
    use serde_json::Value;
    
    // Check if Composio is enabled
    if !config.composio.enabled {
        bail!("Composio is not enabled. Run 'zeroclaw onboard' to configure.");
    }
    
    // Check if MCP is enabled
    if !config.composio.mcp.enabled {
        bail!("Composio MCP is not enabled. Run 'zeroclaw onboard' to configure.");
    }
    
    // Get API key
    let api_key = config.composio.api_key.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Composio API key not configured"))?;
    
    // Get user ID
    let user_id = config.composio.mcp.user_id.clone().unwrap_or_else(|| "default".to_string());
    
    // Create MCP client
    let mcp_client = if let Some(mcp_url) = &config.composio.mcp.mcp_url {
        ComposioMcpClient::new_with_mcp_url(
            api_key.clone(),
            mcp_url.clone(),
            config.composio.mcp.server_id.clone(),
            Some(user_id.clone()),
            std::time::Duration::from_secs(config.composio.mcp.tools_cache_ttl_secs),
        )
    } else if let Some(server_id) = &config.composio.mcp.server_id {
        ComposioMcpClient::new(
            api_key.clone(),
            server_id.clone(),
            user_id.clone(),
        )
    } else {
        bail!("Neither mcp_url nor server_id is configured");
    };
    
    println!("🧪 Testing tool: {}\n", tool_name);
    
    // Parse arguments
    let args: Value = if let Some(json) = args_json {
        serde_json::from_str(json)
            .map_err(|e| anyhow::anyhow!("Invalid JSON arguments: {}", e))?
    } else {
        serde_json::json!({})
    };
    
    if verbose {
        println!("Arguments:");
        println!("{}\n", serde_json::to_string_pretty(&args)?);
    }
    
    // Execute tool
    println!("Executing...");
    
    let start = std::time::Instant::now();
    let result = mcp_client.execute_tool(tool_name, args).await;
    let elapsed = start.elapsed();
    
    println!();
    
    match result {
        Ok(response) => {
            if response.is_error() {
                println!("❌ Tool execution failed");
                println!("\nError:");
                println!("{}", response.to_output_string());
                std::process::exit(1);
            } else {
                println!("✓ Tool executed successfully");
                println!("\nResult:");
                println!("{}", response.to_output_string());
                
                if verbose {
                    println!("\nExecution time: {:?}", elapsed);
                }
            }
        }
        Err(e) => {
            println!("❌ Tool execution failed");
            println!("\nError: {}", e);
            
            if verbose {
                println!("\nExecution time: {:?}", elapsed);
            }
            
            std::process::exit(1);
        }
    }
    
    Ok(())
}

async fn handle_composio_status(config: &Config, verbose: bool) -> Result<()> {
    use zeroclaw::composio::{ComposioRestClient, check_mcp_health};
    use zeroclaw::mcp::ComposioMcpClient;
    use std::sync::Arc;
    
    println!("📊 Composio Integration Status\n");
    
    // Check if Composio is enabled
    if !config.composio.enabled {
        println!("Status: ❌ Disabled");
        println!("\nTo enable Composio:");
        println!("  zeroclaw onboard");
        return Ok(());
    }
    
    println!("Status: ✓ Enabled");
    
    // Check API key
    let api_key = match &config.composio.api_key {
        Some(key) => {
            println!("API Key: ✓ Configured");
            key.clone()
        }
        None => {
            println!("API Key: ❌ Not configured");
            return Ok(());
        }
    };
    
    // Check MCP configuration
    println!("\nMCP Integration:");
    if !config.composio.mcp.enabled {
        println!("  Status: ❌ Disabled");
        return Ok(());
    }
    
    println!("  Status: ✓ Enabled");
    
    let user_id = config.composio.mcp.user_id.clone().unwrap_or_else(|| "default".to_string());
    println!("  User ID: {}", user_id);
    
    if let Some(url) = &config.composio.mcp.mcp_url {
        println!("  MCP URL: ✓ Configured");
        if verbose {
            println!("    {}", url);
        }
    } else if let Some(server_id) = &config.composio.mcp.server_id {
        println!("  Server ID: ✓ Configured ({})", server_id);
    } else {
        println!("  Configuration: ❌ Neither mcp_url nor server_id configured");
        return Ok(());
    }
    
    // Toolkits
    println!("\nToolkits:");
    if config.composio.mcp.toolkits.is_empty() {
        println!("  ⚠️  No toolkits configured");
    } else {
        println!("  Count: {}", config.composio.mcp.toolkits.len());
        if verbose {
            for toolkit in &config.composio.mcp.toolkits {
                println!("    • {}", toolkit);
            }
        } else {
            println!("  {}", config.composio.mcp.toolkits.join(", "));
        }
    }
    
    // Create clients for health check
    let rest_client = Arc::new(ComposioRestClient::new(api_key.clone()));
    
    let mcp_client = if let Some(mcp_url) = &config.composio.mcp.mcp_url {
        Arc::new(ComposioMcpClient::new_with_mcp_url(
            api_key,
            mcp_url.clone(),
            config.composio.mcp.server_id.clone(),
            Some(user_id.clone()),
            std::time::Duration::from_secs(config.composio.mcp.tools_cache_ttl_secs),
        ))
    } else if let Some(server_id) = &config.composio.mcp.server_id {
        Arc::new(ComposioMcpClient::new(
            api_key,
            server_id.clone(),
            user_id.clone(),
        ))
    } else {
        return Ok(());
    };
    
    // Health check
    println!("\nHealth Check:");
    let health = check_mcp_health(
        &mcp_client,
        rest_client.clone(),
        &user_id,
        &config.composio.mcp.toolkits,
    ).await;
    
    if health.is_healthy() {
        println!("  ✓ MCP server is healthy");
        println!("  Tools available: {}", health.tools_count);
    } else {
        println!("  ❌ MCP server is unhealthy");
        if let Some(error) = &health.error {
            println!("  Error: {}", error);
        }
    }
    
    // Connection status
    if !config.composio.mcp.toolkits.is_empty() {
        println!("\nConnection Status:");
        
        if !health.connected_toolkits.is_empty() {
            println!("  Connected ({}):", health.connected_toolkits.len());
            for toolkit in &health.connected_toolkits {
                println!("    ✓ {}", toolkit);
            }
        }
        
        if !health.disconnected_toolkits.is_empty() {
            println!("  Disconnected ({}):", health.disconnected_toolkits.len());
            for toolkit in &health.disconnected_toolkits {
                println!("    ✗ {}", toolkit);
            }
        }
    }
    
    // Summary
    println!("\nQuick Actions:");
    if !health.disconnected_toolkits.is_empty() {
        println!("  • Connect toolkit: zeroclaw composio connect <toolkit>");
    }
    println!("  • Health check: zeroclaw composio health --verbose");
    println!("  • List tools: zeroclaw composio list-tools");
    
    Ok(())
}

async fn handle_composio_list_tools(
    config: &Config,
    toolkit_filter: Option<&str>,
    show_schema: bool,
) -> Result<()> {
    use zeroclaw::mcp::ComposioMcpClient;
    use zeroclaw::composio::normalize_toolkit_slug;
    
    // Check if Composio is enabled
    if !config.composio.enabled {
        bail!("Composio is not enabled. Run 'zeroclaw onboard' to configure.");
    }
    
    // Check if MCP is enabled
    if !config.composio.mcp.enabled {
        bail!("Composio MCP is not enabled. Run 'zeroclaw onboard' to configure.");
    }
    
    // Get API key
    let api_key = config.composio.api_key.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Composio API key not configured"))?;
    
    // Get user ID
    let user_id = config.composio.mcp.user_id.clone().unwrap_or_else(|| "default".to_string());
    
    // Create MCP client
    let mcp_client = if let Some(mcp_url) = &config.composio.mcp.mcp_url {
        ComposioMcpClient::new_with_mcp_url(
            api_key.clone(),
            mcp_url.clone(),
            config.composio.mcp.server_id.clone(),
            Some(user_id.clone()),
            std::time::Duration::from_secs(config.composio.mcp.tools_cache_ttl_secs),
        )
    } else if let Some(server_id) = &config.composio.mcp.server_id {
        ComposioMcpClient::new(
            api_key.clone(),
            server_id.clone(),
            user_id.clone(),
        )
    } else {
        bail!("Neither mcp_url nor server_id is configured");
    };
    
    println!("🔧 Listing Composio MCP Tools\n");
    
    // List tools
    let tools = mcp_client.list_tools().await?;
    
    if tools.is_empty() {
        println!("No tools available.");
        println!("\nPossible reasons:");
        println!("  • No toolkits configured");
        println!("  • No toolkits connected");
        println!("  • MCP server issue");
        return Ok(());
    }
    
    // Filter by toolkit if specified
    let filtered_tools: Vec<_> = if let Some(filter) = toolkit_filter {
        let normalized_filter = normalize_toolkit_slug(filter);
        tools.into_iter()
            .filter(|tool| {
                tool.name.to_lowercase().starts_with(&normalized_filter)
                    || tool.name.to_lowercase().contains(&format!("_{}_", normalized_filter))
            })
            .collect()
    } else {
        tools
    };
    
    if filtered_tools.is_empty() {
        if let Some(filter) = toolkit_filter {
            println!("No tools found for toolkit: {}", filter);
        } else {
            println!("No tools available.");
        }
        return Ok(());
    }
    
    println!("Found {} tool(s):\n", filtered_tools.len());
    
    for tool in &filtered_tools {
        println!("• {}", tool.name);
        
        if let Some(desc) = &tool.description {
            println!("  {}", desc);
        }
        
        if show_schema {
            println!("  Schema:");
            println!("  {}", serde_json::to_string_pretty(&tool.input_schema)?);
            println!();
        }
    }
    
    if !show_schema {
        println!("\nUse --schema to see tool input schemas");
    }
    
    println!("\nTo test a tool:");
    println!("  zeroclaw composio test <TOOL_NAME>");
    
    Ok(())
}

fn write_shell_completion<W: Write>(shell: CompletionShell, writer: &mut W) -> Result<()> {
    use clap_complete::generate;
    use clap_complete::shells;

    let mut cmd = Cli::command();
    let bin_name = cmd.get_name().to_string();

    match shell {
        CompletionShell::Bash => generate(shells::Bash, &mut cmd, bin_name.clone(), writer),
        CompletionShell::Fish => generate(shells::Fish, &mut cmd, bin_name.clone(), writer),
        CompletionShell::Zsh => generate(shells::Zsh, &mut cmd, bin_name.clone(), writer),
        CompletionShell::PowerShell => {
            generate(shells::PowerShell, &mut cmd, bin_name.clone(), writer);
        }
        CompletionShell::Elvish => generate(shells::Elvish, &mut cmd, bin_name, writer),
    }

    writer.flush()?;
    Ok(())
}

// ─── Generic Pending OAuth Login ────────────────────────────────────────────

/// Generic pending OAuth login state, shared across providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingOAuthLogin {
    provider: String,
    profile: String,
    code_verifier: String,
    state: String,
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingOAuthLoginFile {
    #[serde(default)]
    provider: Option<String>,
    profile: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    code_verifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    encrypted_code_verifier: Option<String>,
    state: String,
    created_at: String,
}

fn pending_oauth_login_path(config: &Config, provider: &str) -> std::path::PathBuf {
    let filename = format!("auth-{}-pending.json", provider);
    auth::state_dir_from_config(config).join(filename)
}

fn pending_oauth_secret_store(config: &Config) -> security::secrets::SecretStore {
    security::secrets::SecretStore::new(
        &auth::state_dir_from_config(config),
        config.secrets.encrypt,
    )
}

#[cfg(unix)]
fn set_owner_only_permissions(path: &std::path::Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_owner_only_permissions(_path: &std::path::Path) -> Result<()> {
    Ok(())
}

fn save_pending_oauth_login(config: &Config, pending: &PendingOAuthLogin) -> Result<()> {
    let path = pending_oauth_login_path(config, &pending.provider);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let secret_store = pending_oauth_secret_store(config);
    let encrypted_code_verifier = secret_store.encrypt(&pending.code_verifier)?;
    let persisted = PendingOAuthLoginFile {
        provider: Some(pending.provider.clone()),
        profile: pending.profile.clone(),
        code_verifier: None,
        encrypted_code_verifier: Some(encrypted_code_verifier),
        state: pending.state.clone(),
        created_at: pending.created_at.clone(),
    };
    let tmp = path.with_extension(format!(
        "tmp.{}.{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let json = serde_json::to_vec_pretty(&persisted)?;
    std::fs::write(&tmp, json)?;
    set_owner_only_permissions(&tmp)?;
    std::fs::rename(tmp, &path)?;
    set_owner_only_permissions(&path)?;
    Ok(())
}

fn load_pending_oauth_login(config: &Config, provider: &str) -> Result<Option<PendingOAuthLogin>> {
    let path = pending_oauth_login_path(config, provider);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(&path)?;
    if bytes.is_empty() {
        return Ok(None);
    }
    let persisted: PendingOAuthLoginFile = serde_json::from_slice(&bytes)?;
    let secret_store = pending_oauth_secret_store(config);
    let code_verifier = if let Some(encrypted) = persisted.encrypted_code_verifier {
        secret_store.decrypt(&encrypted)?
    } else if let Some(plaintext) = persisted.code_verifier {
        plaintext
    } else {
        bail!("Pending {} login is missing code verifier", provider);
    };
    Ok(Some(PendingOAuthLogin {
        provider: persisted.provider.unwrap_or_else(|| provider.to_string()),
        profile: persisted.profile,
        code_verifier,
        state: persisted.state,
        created_at: persisted.created_at,
    }))
}

fn clear_pending_oauth_login(config: &Config, provider: &str) {
    let path = pending_oauth_login_path(config, provider);
    if let Ok(file) = std::fs::OpenOptions::new().write(true).open(&path) {
        let _ = file.set_len(0);
        let _ = file.sync_all();
    }
    let _ = std::fs::remove_file(path);
}

fn read_auth_input(prompt: &str) -> Result<String> {
    let input = Password::new()
        .with_prompt(prompt)
        .allow_empty_password(false)
        .interact()?;
    Ok(input.trim().to_string())
}

fn read_plain_input(prompt: &str) -> Result<String> {
    let input: String = Input::new().with_prompt(prompt).interact_text()?;
    Ok(input.trim().to_string())
}

fn extract_openai_account_id_for_profile(access_token: &str) -> Option<String> {
    let account_id = auth::openai_oauth::extract_account_id_from_jwt(access_token);
    if account_id.is_none() {
        warn!(
            "Could not extract OpenAI account id from OAuth access token; \
             requests may fail until re-authentication."
        );
    }
    account_id
}

fn format_expiry(profile: &auth::profiles::AuthProfile) -> String {
    match profile
        .token_set
        .as_ref()
        .and_then(|token_set| token_set.expires_at)
    {
        Some(ts) => {
            let now = chrono::Utc::now();
            if ts <= now {
                format!("expired at {}", ts.to_rfc3339())
            } else {
                let mins = (ts - now).num_minutes();
                format!("expires in {mins}m ({})", ts.to_rfc3339())
            }
        }
        None => "n/a".to_string(),
    }
}

#[allow(clippy::too_many_lines)]
async fn handle_auth_command(auth_command: AuthCommands, config: &Config) -> Result<()> {
    let auth_service = auth::AuthService::from_config(config);

    match auth_command {
        AuthCommands::Login {
            provider,
            profile,
            device_code,
        } => {
            let provider = auth::normalize_provider(&provider)?;
            let client = reqwest::Client::new();

            match provider.as_str() {
                "gemini" => {
                    // Gemini OAuth flow
                    if device_code {
                        match auth::gemini_oauth::start_device_code_flow(&client).await {
                            Ok(device) => {
                                println!("Google/Gemini device-code login started.");
                                println!("Visit: {}", device.verification_uri);
                                println!("Code:  {}", device.user_code);
                                if let Some(uri_complete) = &device.verification_uri_complete {
                                    println!("Fast link: {uri_complete}");
                                }

                                let token_set =
                                    auth::gemini_oauth::poll_device_code_tokens(&client, &device)
                                        .await?;
                                let account_id = token_set.id_token.as_deref().and_then(
                                    auth::gemini_oauth::extract_account_email_from_id_token,
                                );

                                auth_service
                                    .store_gemini_tokens(&profile, token_set, account_id, true)
                                    .await?;

                                println!("Saved profile {profile}");
                                println!("Active profile for gemini: {profile}");
                                return Ok(());
                            }
                            Err(e) => {
                                println!(
                                    "Device-code flow unavailable: {e}. Falling back to browser flow."
                                );
                            }
                        }
                    }

                    let pkce = auth::gemini_oauth::generate_pkce_state();
                    let authorize_url = auth::gemini_oauth::build_authorize_url(&pkce)?;

                    // Save pending login for paste-redirect fallback
                    let pending = PendingOAuthLogin {
                        provider: "gemini".to_string(),
                        profile: profile.clone(),
                        code_verifier: pkce.code_verifier.clone(),
                        state: pkce.state.clone(),
                        created_at: chrono::Utc::now().to_rfc3339(),
                    };
                    save_pending_oauth_login(config, &pending)?;

                    println!("Open this URL in your browser and authorize access:");
                    println!("{authorize_url}");
                    println!();

                    let code = match auth::gemini_oauth::receive_loopback_code(
                        &pkce.state,
                        std::time::Duration::from_secs(180),
                    )
                    .await
                    {
                        Ok(code) => {
                            clear_pending_oauth_login(config, "gemini");
                            code
                        }
                        Err(e) => {
                            println!("Callback capture failed: {e}");
                            println!(
                                "Run `zeroclaw auth paste-redirect --provider gemini --profile {profile}`"
                            );
                            return Ok(());
                        }
                    };

                    let token_set =
                        auth::gemini_oauth::exchange_code_for_tokens(&client, &code, &pkce).await?;
                    let account_id = token_set
                        .id_token
                        .as_deref()
                        .and_then(auth::gemini_oauth::extract_account_email_from_id_token);

                    auth_service
                        .store_gemini_tokens(&profile, token_set, account_id, true)
                        .await?;

                    println!("Saved profile {profile}");
                    println!("Active profile for gemini: {profile}");
                    Ok(())
                }
                "openai-codex" => {
                    // OpenAI Codex OAuth flow
                    if device_code {
                        match auth::openai_oauth::start_device_code_flow(&client).await {
                            Ok(device) => {
                                println!("OpenAI device-code login started.");
                                println!("Visit: {}", device.verification_uri);
                                println!("Code:  {}", device.user_code);
                                if let Some(uri_complete) = &device.verification_uri_complete {
                                    println!("Fast link: {uri_complete}");
                                }
                                if let Some(message) = &device.message {
                                    println!("{message}");
                                }

                                let token_set =
                                    auth::openai_oauth::poll_device_code_tokens(&client, &device)
                                        .await?;
                                let account_id =
                                    extract_openai_account_id_for_profile(&token_set.access_token);

                                auth_service
                                    .store_openai_tokens(&profile, token_set, account_id, true)
                                    .await?;
                                clear_pending_oauth_login(config, "openai");

                                println!("Saved profile {profile}");
                                println!("Active profile for openai-codex: {profile}");
                                return Ok(());
                            }
                            Err(e) => {
                                println!(
                                    "Device-code flow unavailable: {e}. Falling back to browser/paste flow."
                                );
                            }
                        }
                    }

                    let pkce = auth::openai_oauth::generate_pkce_state();
                    let pending = PendingOAuthLogin {
                        provider: "openai".to_string(),
                        profile: profile.clone(),
                        code_verifier: pkce.code_verifier.clone(),
                        state: pkce.state.clone(),
                        created_at: chrono::Utc::now().to_rfc3339(),
                    };
                    save_pending_oauth_login(config, &pending)?;

                    let authorize_url = auth::openai_oauth::build_authorize_url(&pkce);
                    println!("Open this URL in your browser and authorize access:");
                    println!("{authorize_url}");
                    println!();
                    println!("Waiting for callback at http://localhost:1455/auth/callback ...");

                    let code = match auth::openai_oauth::receive_loopback_code(
                        &pkce.state,
                        std::time::Duration::from_secs(180),
                    )
                    .await
                    {
                        Ok(code) => code,
                        Err(e) => {
                            println!("Callback capture failed: {e}");
                            println!(
                                "Run `zeroclaw auth paste-redirect --provider openai-codex --profile {profile}`"
                            );
                            return Ok(());
                        }
                    };

                    let token_set =
                        auth::openai_oauth::exchange_code_for_tokens(&client, &code, &pkce).await?;
                    let account_id = extract_openai_account_id_for_profile(&token_set.access_token);

                    auth_service
                        .store_openai_tokens(&profile, token_set, account_id, true)
                        .await?;
                    clear_pending_oauth_login(config, "openai");

                    println!("Saved profile {profile}");
                    println!("Active profile for openai-codex: {profile}");
                    Ok(())
                }
                _ => {
                    bail!(
                        "`auth login` supports --provider openai-codex or gemini, got: {provider}"
                    );
                }
            }
        }

        AuthCommands::PasteRedirect {
            provider,
            profile,
            input,
        } => {
            let provider = auth::normalize_provider(&provider)?;

            match provider.as_str() {
                "openai-codex" => {
                    let pending = load_pending_oauth_login(config, "openai")?.ok_or_else(|| {
                        anyhow::anyhow!(
                            "No pending OpenAI login found. Run `zeroclaw auth login --provider openai-codex` first."
                        )
                    })?;

                    if pending.profile != profile {
                        bail!(
                            "Pending login profile mismatch: pending={}, requested={}",
                            pending.profile,
                            profile
                        );
                    }

                    let redirect_input = match input {
                        Some(value) => value,
                        None => read_plain_input("Paste redirect URL or OAuth code")?,
                    };

                    let code = auth::openai_oauth::parse_code_from_redirect(
                        &redirect_input,
                        Some(&pending.state),
                    )?;

                    let pkce = auth::openai_oauth::PkceState {
                        code_verifier: pending.code_verifier.clone(),
                        code_challenge: String::new(),
                        state: pending.state.clone(),
                    };

                    let client = reqwest::Client::new();
                    let token_set =
                        auth::openai_oauth::exchange_code_for_tokens(&client, &code, &pkce).await?;
                    let account_id = extract_openai_account_id_for_profile(&token_set.access_token);

                    auth_service
                        .store_openai_tokens(&profile, token_set, account_id, true)
                        .await?;
                    clear_pending_oauth_login(config, "openai");

                    println!("Saved profile {profile}");
                    println!("Active profile for openai-codex: {profile}");
                }
                "gemini" => {
                    let pending = load_pending_oauth_login(config, "gemini")?.ok_or_else(|| {
                        anyhow::anyhow!(
                            "No pending Gemini login found. Run `zeroclaw auth login --provider gemini` first."
                        )
                    })?;

                    if pending.profile != profile {
                        bail!(
                            "Pending login profile mismatch: pending={}, requested={}",
                            pending.profile,
                            profile
                        );
                    }

                    let redirect_input = match input {
                        Some(value) => value,
                        None => read_plain_input("Paste redirect URL or OAuth code")?,
                    };

                    let code = auth::gemini_oauth::parse_code_from_redirect(
                        &redirect_input,
                        Some(&pending.state),
                    )?;

                    let pkce = auth::gemini_oauth::PkceState {
                        code_verifier: pending.code_verifier.clone(),
                        code_challenge: String::new(),
                        state: pending.state.clone(),
                    };

                    let client = reqwest::Client::new();
                    let token_set =
                        auth::gemini_oauth::exchange_code_for_tokens(&client, &code, &pkce).await?;
                    let account_id = token_set
                        .id_token
                        .as_deref()
                        .and_then(auth::gemini_oauth::extract_account_email_from_id_token);

                    auth_service
                        .store_gemini_tokens(&profile, token_set, account_id, true)
                        .await?;
                    clear_pending_oauth_login(config, "gemini");

                    println!("Saved profile {profile}");
                    println!("Active profile for gemini: {profile}");
                }
                _ => {
                    bail!("`auth paste-redirect` supports --provider openai-codex or gemini");
                }
            }
            Ok(())
        }

        AuthCommands::PasteToken {
            provider,
            profile,
            token,
            auth_kind,
        } => {
            let provider = auth::normalize_provider(&provider)?;
            let token = match token {
                Some(token) => token.trim().to_string(),
                None => read_auth_input("Paste token")?,
            };
            if token.is_empty() {
                bail!("Token cannot be empty");
            }

            let kind = auth::anthropic_token::detect_auth_kind(&token, auth_kind.as_deref());
            let mut metadata = std::collections::HashMap::new();
            metadata.insert(
                "auth_kind".to_string(),
                kind.as_metadata_value().to_string(),
            );

            auth_service
                .store_provider_token(&provider, &profile, &token, metadata, true)
                .await?;
            println!("Saved profile {profile}");
            println!("Active profile for {provider}: {profile}");
            Ok(())
        }

        AuthCommands::SetupToken { provider, profile } => {
            let provider = auth::normalize_provider(&provider)?;
            let token = read_auth_input("Paste token")?;
            if token.is_empty() {
                bail!("Token cannot be empty");
            }

            let kind = auth::anthropic_token::detect_auth_kind(&token, Some("authorization"));
            let mut metadata = std::collections::HashMap::new();
            metadata.insert(
                "auth_kind".to_string(),
                kind.as_metadata_value().to_string(),
            );

            auth_service
                .store_provider_token(&provider, &profile, &token, metadata, true)
                .await?;
            println!("Saved profile {profile}");
            println!("Active profile for {provider}: {profile}");
            Ok(())
        }

        AuthCommands::Refresh { provider, profile } => {
            let provider = auth::normalize_provider(&provider)?;

            match provider.as_str() {
                "openai-codex" => {
                    match auth_service
                        .get_valid_openai_access_token(profile.as_deref())
                        .await?
                    {
                        Some(_) => {
                            println!("OpenAI Codex token is valid (refresh completed if needed).");
                            Ok(())
                        }
                        None => {
                            bail!(
                                "No OpenAI Codex auth profile found. Run `zeroclaw auth login --provider openai-codex`."
                            )
                        }
                    }
                }
                "gemini" => {
                    match auth_service
                        .get_valid_gemini_access_token(profile.as_deref())
                        .await?
                    {
                        Some(_) => {
                            let profile_name = profile.as_deref().unwrap_or("default");
                            println!("✓ Gemini token refreshed successfully");
                            println!("  Profile: gemini:{}", profile_name);
                            Ok(())
                        }
                        None => {
                            bail!(
                                "No Gemini auth profile found. Run `zeroclaw auth login --provider gemini`."
                            )
                        }
                    }
                }
                _ => bail!("`auth refresh` supports --provider openai-codex or gemini"),
            }
        }

        AuthCommands::Logout { provider, profile } => {
            let provider = auth::normalize_provider(&provider)?;
            let removed = auth_service.remove_profile(&provider, &profile).await?;
            if removed {
                println!("Removed auth profile {provider}:{profile}");
            } else {
                println!("Auth profile not found: {provider}:{profile}");
            }
            Ok(())
        }

        AuthCommands::Use { provider, profile } => {
            let provider = auth::normalize_provider(&provider)?;
            auth_service.set_active_profile(&provider, &profile).await?;
            println!("Active profile for {provider}: {profile}");
            Ok(())
        }

        AuthCommands::List => {
            let data = auth_service.load_profiles().await?;
            if data.profiles.is_empty() {
                println!("No auth profiles configured.");
                return Ok(());
            }

            for (id, profile) in &data.profiles {
                let active = data
                    .active_profiles
                    .get(&profile.provider)
                    .is_some_and(|active_id| active_id == id);
                let marker = if active { "*" } else { " " };
                println!("{marker} {id}");
            }

            Ok(())
        }

        AuthCommands::Status => {
            let data = auth_service.load_profiles().await?;
            if data.profiles.is_empty() {
                println!("No auth profiles configured.");
                return Ok(());
            }

            for (id, profile) in &data.profiles {
                let active = data
                    .active_profiles
                    .get(&profile.provider)
                    .is_some_and(|active_id| active_id == id);
                let marker = if active { "*" } else { " " };
                println!(
                    "{} {} kind={:?} account={} expires={}",
                    marker,
                    id,
                    profile.kind,
                    crate::security::redact(profile.account_id.as_deref().unwrap_or("unknown")),
                    format_expiry(profile)
                );
            }

            println!();
            println!("Active profiles:");
            for (provider, profile_id) in &data.active_profiles {
                println!("  {provider}: {profile_id}");
            }

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{CommandFactory, Parser};

    #[test]
    fn cli_definition_has_no_flag_conflicts() {
        Cli::command().debug_assert();
    }

    #[test]
    fn onboard_help_includes_model_flag() {
        let cmd = Cli::command();
        let onboard = cmd
            .get_subcommands()
            .find(|subcommand| subcommand.get_name() == "onboard")
            .expect("onboard subcommand must exist");

        let has_model_flag = onboard
            .get_arguments()
            .any(|arg| arg.get_id().as_str() == "model" && arg.get_long() == Some("model"));

        assert!(
            has_model_flag,
            "onboard help should include --model for quick setup overrides"
        );
    }

    #[test]
    fn onboard_cli_accepts_model_provider_and_api_key_in_quick_mode() {
        let cli = Cli::try_parse_from([
            "zeroclaw",
            "onboard",
            "--provider",
            "openrouter",
            "--model",
            "custom-model-946",
            "--api-key",
            "sk-issue946",
        ])
        .expect("quick onboard invocation should parse");

        match cli.command {
            Commands::Onboard {
                interactive,
                force,
                channels_only,
                api_key,
                provider,
                model,
                ..
            } => {
                assert!(!interactive);
                assert!(!force);
                assert!(!channels_only);
                assert_eq!(provider.as_deref(), Some("openrouter"));
                assert_eq!(model.as_deref(), Some("custom-model-946"));
                assert_eq!(api_key.as_deref(), Some("sk-issue946"));
            }
            other => panic!("expected onboard command, got {other:?}"),
        }
    }

    #[test]
    fn completions_cli_parses_supported_shells() {
        for shell in ["bash", "fish", "zsh", "powershell", "elvish"] {
            let cli = Cli::try_parse_from(["zeroclaw", "completions", shell])
                .expect("completions invocation should parse");
            match cli.command {
                Commands::Completions { .. } => {}
                other => panic!("expected completions command, got {other:?}"),
            }
        }
    }

    #[test]
    fn completion_generation_mentions_binary_name() {
        let mut output = Vec::new();
        write_shell_completion(CompletionShell::Bash, &mut output)
            .expect("completion generation should succeed");
        let script = String::from_utf8(output).expect("completion output should be valid utf-8");
        assert!(
            script.contains("zeroclaw"),
            "completion script should reference binary name"
        );
    }

    #[test]
    fn onboard_cli_accepts_force_flag() {
        let cli = Cli::try_parse_from(["zeroclaw", "onboard", "--force"])
            .expect("onboard --force should parse");

        match cli.command {
            Commands::Onboard { force, .. } => assert!(force),
            other => panic!("expected onboard command, got {other:?}"),
        }
    }

    #[test]
    fn cli_parses_estop_default_engage() {
        let cli = Cli::try_parse_from(["zeroclaw", "estop"]).expect("estop command should parse");

        match cli.command {
            Commands::Estop {
                estop_command,
                level,
                domains,
                tools,
            } => {
                assert!(estop_command.is_none());
                assert!(level.is_none());
                assert!(domains.is_empty());
                assert!(tools.is_empty());
            }
            other => panic!("expected estop command, got {other:?}"),
        }
    }

    #[test]
    fn cli_parses_estop_resume_domain() {
        let cli = Cli::try_parse_from(["zeroclaw", "estop", "resume", "--domain", "*.chase.com"])
            .expect("estop resume command should parse");

        match cli.command {
            Commands::Estop {
                estop_command: Some(EstopSubcommands::Resume { domains, .. }),
                ..
            } => assert_eq!(domains, vec!["*.chase.com".to_string()]),
            other => panic!("expected estop resume command, got {other:?}"),
        }
    }
}
