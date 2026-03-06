//! Session management for Tool Router

use crate::client::ComposioClient;
use crate::error::ComposioError;
use crate::models::request::{
    SessionConfig, TagsConfig, ToolFilter, ToolsConfig, ToolkitFilter, WorkbenchConfig,
};
use crate::models::response::ToolSchema;
use crate::models::enums::TagType;
use std::collections::HashMap;
use std::sync::Arc;

/// Represents a Tool Router session
///
/// A session provides scoped access to tools and toolkits for a specific user.
/// It maintains a reference to the client for making API calls and stores
/// session metadata including available tools.
///
/// # Example
///
/// ```no_run
/// use composio_sdk::ComposioClient;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = ComposioClient::builder()
///     .api_key("your-api-key")
///     .build()?;
///
/// let session = client
///     .create_session("user_123")
///     .toolkits(vec!["github", "gmail"])
///     .send()
///     .await?;
///
/// println!("Session ID: {}", session.session_id());
/// println!("MCP URL: {}", session.mcp_url());
/// println!("Available tools: {}", session.tools().len());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Session {
    /// Shared reference to the Composio client for making API calls
    client: Arc<ComposioClient>,
    /// Unique identifier for this session
    session_id: String,
    /// MCP server URL for this session
    mcp_url: String,
    /// List of available tool slugs in this session
    tools: Vec<String>,
}

impl Session {
    /// Get the session ID
    ///
    /// Returns the unique identifier for this session. This ID is used
    /// for all session-scoped API operations.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::ComposioClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = ComposioClient::builder().api_key("key").build()?;
    /// let session = client.create_session("user_123").send().await?;
    /// println!("Session ID: {}", session.session_id());
    /// # Ok(())
    /// # }
    /// ```
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get the MCP URL
    ///
    /// Returns the Model Context Protocol (MCP) server URL for this session.
    /// This URL can be used to connect MCP-compatible clients to access
    /// the session's tools.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::ComposioClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = ComposioClient::builder().api_key("key").build()?;
    /// let session = client.create_session("user_123").send().await?;
    /// println!("MCP URL: {}", session.mcp_url());
    /// # Ok(())
    /// # }
    /// ```
    pub fn mcp_url(&self) -> &str {
        &self.mcp_url
    }

    /// Get the available tool slugs
    ///
    /// Returns a slice of tool slugs available in this session.
    /// These are the meta tools (COMPOSIO_SEARCH_TOOLS,
    /// COMPOSIO_MULTI_EXECUTE_TOOL, etc.) that can be used with this session.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::ComposioClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = ComposioClient::builder().api_key("key").build()?;
    /// let session = client.create_session("user_123").send().await?;
    /// 
    /// for tool_slug in session.tools() {
    ///     println!("Tool: {}", tool_slug);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn tools(&self) -> &[String] {
        &self.tools
    }

    /// Create a Session from a SessionResponse
    ///
    /// Internal method used to construct a Session from an API response.
    /// This is used by both session creation and retrieval.
    ///
    /// # Arguments
    ///
    /// * `client` - The ComposioClient to use for API calls
    /// * `response` - The SessionResponse from the API
    pub(crate) fn from_response(
        client: ComposioClient,
        response: crate::models::response::SessionResponse,
    ) -> Self {
        Self {
            client: Arc::new(client),
            session_id: response.session_id,
            mcp_url: response.mcp.url,
            tools: response.tool_router_tools,
        }
    }

    /// Execute a tool within this session
    ///
    /// Executes a specific tool with the provided arguments. The tool must be
    /// available in this session (either through enabled toolkits or via
    /// COMPOSIO_SEARCH_TOOLS).
    ///
    /// # Arguments
    ///
    /// * `tool_slug` - The tool identifier (e.g., "GITHUB_CREATE_ISSUE")
    /// * `arguments` - JSON value containing the tool's input parameters
    ///
    /// # Returns
    ///
    /// Returns a `ToolExecutionResponse` containing:
    /// - `data`: The tool's output data
    /// - `error`: Optional error message if execution failed
    /// - `log_id`: Unique identifier for this execution (for debugging)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The tool is not found or not available in this session
    /// - The user doesn't have a connected account for the toolkit
    /// - The arguments are invalid or missing required fields
    /// - Network error occurs
    /// - API returns an error response
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::ComposioClient;
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ComposioClient::builder()
    ///     .api_key("your-api-key")
    ///     .build()?;
    ///
    /// let session = client
    ///     .create_session("user_123")
    ///     .toolkits(vec!["github"])
    ///     .send()
    ///     .await?;
    ///
    /// let result = session
    ///     .execute_tool(
    ///         "GITHUB_CREATE_ISSUE",
    ///         json!({
    ///             "owner": "composio",
    ///             "repo": "composio",
    ///             "title": "Test issue",
    ///             "body": "Created via Rust SDK"
    ///         })
    ///     )
    ///     .await?;
    ///
    /// println!("Result: {:?}", result.data);
    /// println!("Log ID: {}", result.log_id);
    ///
    /// if let Some(error) = result.error {
    ///     eprintln!("Tool execution error: {}", error);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_tool(
        &self,
        tool_slug: impl Into<String>,
        arguments: serde_json::Value,
    ) -> Result<crate::models::response::ToolExecutionResponse, ComposioError> {
        use crate::models::request::ToolExecutionRequest;
        use crate::models::response::ToolExecutionResponse;
        use crate::retry::with_retry;

        let tool_slug = tool_slug.into();
        let url = format!(
            "{}/tool_router/session/{}/execute",
            self.client.config().base_url,
            self.session_id
        );

        // Create request body
        let request_body = ToolExecutionRequest {
            tool_slug: tool_slug.clone(),
            arguments: Some(arguments),
        };

        let policy = &self.client.config().retry_policy;

        // Execute request with retry logic
        let response = with_retry(policy, || {
            let url = url.clone();
            let request_body = request_body.clone();
            let client = self.client.http_client().clone();

            async move {
                let response = client
                    .post(&url)
                    .json(&request_body)
                    .send()
                    .await
                    .map_err(ComposioError::NetworkError)?;

                // Check for HTTP errors
                if !response.status().is_success() {
                    return Err(ComposioError::from_response(response).await);
                }

                Ok(response)
            }
        })
        .await?;

        // Parse successful response
        let execution_response: ToolExecutionResponse = response
            .json()
            .await
            .map_err(ComposioError::NetworkError)?;

        Ok(execution_response)
    }

    /// Execute a meta tool within this session
    ///
    /// Meta tools are special tools provided by Composio for runtime tool discovery,
    /// connection management, and advanced operations:
    /// - `COMPOSIO_SEARCH_TOOLS`: Discover relevant tools across 1000+ apps
    /// - `COMPOSIO_MULTI_EXECUTE_TOOL`: Execute up to 20 tools in parallel
    /// - `COMPOSIO_MANAGE_CONNECTIONS`: Handle OAuth and API key authentication
    /// - `COMPOSIO_REMOTE_WORKBENCH`: Run Python code in persistent sandbox
    /// - `COMPOSIO_REMOTE_BASH_TOOL`: Execute bash commands for file/data processing
    ///
    /// # Arguments
    ///
    /// * `slug` - The meta tool identifier (MetaToolSlug enum)
    /// * `arguments` - JSON value containing the meta tool's input parameters
    ///
    /// # Returns
    ///
    /// Returns a `MetaToolExecutionResponse` containing:
    /// - `data`: The meta tool's output data
    /// - `error`: Optional error message if execution failed
    /// - `log_id`: Unique identifier for this execution (for debugging)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The arguments are invalid or missing required fields
    /// - Network error occurs
    /// - API returns an error response
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::{ComposioClient, MetaToolSlug};
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ComposioClient::builder()
    ///     .api_key("your-api-key")
    ///     .build()?;
    ///
    /// let session = client
    ///     .create_session("user_123")
    ///     .toolkits(vec!["github"])
    ///     .send()
    ///     .await?;
    ///
    /// // Search for tools
    /// let search_result = session
    ///     .execute_meta_tool(
    ///         MetaToolSlug::ComposioSearchTools,
    ///         json!({
    ///             "query": "create a GitHub issue"
    ///         })
    ///     )
    ///     .await?;
    ///
    /// println!("Search result: {:?}", search_result.data);
    ///
    /// // Multi-execute tools
    /// let multi_result = session
    ///     .execute_meta_tool(
    ///         MetaToolSlug::ComposioMultiExecuteTool,
    ///         json!({
    ///             "tools": [
    ///                 {
    ///                     "tool_slug": "GITHUB_GET_REPOS",
    ///                     "arguments": {"owner": "composio"}
    ///                 },
    ///                 {
    ///                     "tool_slug": "GITHUB_GET_ISSUES",
    ///                     "arguments": {"owner": "composio", "repo": "composio"}
    ///                 }
    ///             ]
    ///         })
    ///     )
    ///     .await?;
    ///
    /// println!("Multi-execute result: {:?}", multi_result.data);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_meta_tool(
        &self,
        slug: crate::models::enums::MetaToolSlug,
        arguments: serde_json::Value,
    ) -> Result<crate::models::response::MetaToolExecutionResponse, ComposioError> {
        use crate::models::request::MetaToolExecutionRequest;
        use crate::models::response::MetaToolExecutionResponse;
        use crate::retry::with_retry;

        let url = format!(
            "{}/tool_router/session/{}/execute_meta",
            self.client.config().base_url,
            self.session_id
        );

        // Create request body
        let request_body = MetaToolExecutionRequest {
            slug,
            arguments: Some(arguments),
        };

        let policy = &self.client.config().retry_policy;

        // Execute request with retry logic
        let response = with_retry(policy, || {
            let url = url.clone();
            let request_body = request_body.clone();
            let client = self.client.http_client().clone();

            async move {
                let response = client
                    .post(&url)
                    .json(&request_body)
                    .send()
                    .await
                    .map_err(ComposioError::NetworkError)?;

                // Check for HTTP errors
                if !response.status().is_success() {
                    return Err(ComposioError::from_response(response).await);
                }

                Ok(response)
            }
        })
        .await?;

        // Parse successful response
        let execution_response: MetaToolExecutionResponse = response
            .json()
            .await
            .map_err(ComposioError::NetworkError)?;

        Ok(execution_response)
    }

    /// List available toolkits in this session
    ///
    /// Returns a builder for listing toolkits with optional filtering.
    /// The builder allows you to configure pagination, search, and filtering
    /// options before executing the request.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::ComposioClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = ComposioClient::builder().api_key("key").build()?;
    /// let session = client.create_session("user_123").send().await?;
    ///
    /// // List all toolkits
    /// let toolkits = session.list_toolkits().send().await?;
    ///
    /// // List only connected toolkits
    /// let connected = session.list_toolkits()
    ///     .is_connected(true)
    ///     .send()
    ///     .await?;
    ///
    /// // Search for specific toolkits
    /// let github_toolkits = session.list_toolkits()
    ///     .search("github")
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn list_toolkits(&self) -> ToolkitListBuilder<'_> {
        ToolkitListBuilder::new(self)
    }

    /// Get meta tools schemas for this session
    ///
    /// Retrieves the complete schemas for all meta tools available in this session.
    /// Meta tools include:
    /// - `COMPOSIO_SEARCH_TOOLS`: Discover relevant tools across 1000+ apps
    /// - `COMPOSIO_MULTI_EXECUTE_TOOL`: Execute up to 20 tools in parallel
    /// - `COMPOSIO_MANAGE_CONNECTIONS`: Handle OAuth and API key authentication
    /// - `COMPOSIO_REMOTE_WORKBENCH`: Run Python code in persistent sandbox
    /// - `COMPOSIO_REMOTE_BASH_TOOL`: Execute bash commands for file/data processing
    ///
    /// The returned schemas include detailed information about input parameters,
    /// output parameters, descriptions, and other metadata needed to use the tools.
    ///
    /// # Returns
    ///
    /// Returns a vector of `ToolSchema` objects, each containing:
    /// - `slug`: Tool identifier (e.g., "COMPOSIO_SEARCH_TOOLS")
    /// - `name`: Human-readable name
    /// - `description`: Detailed functionality explanation
    /// - `input_parameters`: JSON schema of required inputs
    /// - `output_parameters`: JSON schema of return values
    /// - `version`: Current version
    /// - Other metadata fields
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Network error occurs
    /// - API returns an error response
    /// - Response cannot be parsed
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::ComposioClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ComposioClient::builder()
    ///     .api_key("your-api-key")
    ///     .build()?;
    ///
    /// let session = client
    ///     .create_session("user_123")
    ///     .send()
    ///     .await?;
    ///
    /// // Get all meta tool schemas
    /// let meta_tools = session.get_meta_tools().await?;
    ///
    /// for tool in meta_tools {
    ///     println!("Tool: {}", tool.slug);
    ///     println!("  Name: {}", tool.name);
    ///     println!("  Description: {}", tool.description);
    ///     println!("  Version: {}", tool.version);
    ///     println!("  Input schema: {}", tool.input_parameters);
    ///     println!("  Output schema: {}", tool.output_parameters);
    ///     println!();
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_meta_tools(&self) -> Result<Vec<ToolSchema>, ComposioError> {
        use crate::retry::with_retry;

        let url = format!(
            "{}/tool_router/session/{}/tools",
            self.client.config().base_url,
            self.session_id
        );

        let policy = &self.client.config().retry_policy;

        // Execute request with retry logic
        let response = with_retry(policy, || {
            let url = url.clone();
            let client = self.client.http_client().clone();

            async move {
                let response = client
                    .get(&url)
                    .send()
                    .await
                    .map_err(ComposioError::NetworkError)?;

                // Check for HTTP errors
                if !response.status().is_success() {
                    return Err(ComposioError::from_response(response).await);
                }

                Ok(response)
            }
        })
        .await?;

        // Parse successful response - API returns array of ToolSchema
        let tools: Vec<ToolSchema> = response
            .json()
            .await
            .map_err(ComposioError::NetworkError)?;

        Ok(tools)
    }

    /// Create an authentication link for a toolkit
    ///
    /// Generates a Connect Link URL that users can visit to authenticate with
    /// a specific toolkit (e.g., GitHub, Gmail, Slack). This is used for both
    /// in-chat authentication and manual authentication flows.
    ///
    /// # Arguments
    ///
    /// * `toolkit` - The toolkit slug to create an auth link for (e.g., "github", "gmail")
    /// * `callback_url` - Optional URL to redirect to after authentication completes.
    ///                    Query parameters `status` and `connected_account_id` will be appended.
    ///
    /// # Returns
    ///
    /// Returns a `LinkResponse` containing:
    /// - `link_token`: Token identifying this auth link session
    /// - `redirect_url`: URL for the user to visit to complete authentication
    /// - `connected_account_id`: Optional ID if account already exists
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Toolkit slug is invalid or not found (400 Bad Request)
    /// - Connected account already exists for this toolkit (400 Bad Request)
    /// - Network error occurs
    /// - API returns an error response
    /// - Response cannot be parsed
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::ComposioClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ComposioClient::builder()
    ///     .api_key("your-api-key")
    ///     .build()?;
    ///
    /// let session = client
    ///     .create_session("user_123")
    ///     .send()
    ///     .await?;
    ///
    /// // Create auth link without callback
    /// let link = session.create_auth_link("github", None).await?;
    /// println!("Visit: {}", link.redirect_url);
    ///
    /// // Create auth link with callback
    /// let link = session.create_auth_link(
    ///     "gmail",
    ///     Some("https://example.com/callback".to_string())
    /// ).await?;
    /// println!("Link token: {}", link.link_token);
    /// println!("Redirect URL: {}", link.redirect_url);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # See Also
    ///
    /// - [Manual Authentication Guide](https://docs.composio.dev/guides/authentication)
    /// - [In-Chat Authentication](https://docs.composio.dev/guides/in-chat-auth)
    pub async fn create_auth_link(
        &self,
        toolkit: impl Into<String>,
        callback_url: Option<String>,
    ) -> Result<crate::models::response::LinkResponse, ComposioError> {
        use crate::models::request::LinkRequest;
        use crate::retry::with_retry;

        let toolkit = toolkit.into();
        let url = format!(
            "{}/tool_router/session/{}/link",
            self.client.config().base_url,
            self.session_id
        );

        let request_body = LinkRequest {
            toolkit: toolkit.clone(),
            callback_url,
        };

        let policy = &self.client.config().retry_policy;

        // Execute request with retry logic
        let response = with_retry(policy, || {
            let url = url.clone();
            let client = self.client.http_client().clone();
            let body = request_body.clone();

            async move {
                let response = client
                    .post(&url)
                    .json(&body)
                    .send()
                    .await
                    .map_err(ComposioError::NetworkError)?;

                // Check for HTTP errors
                if !response.status().is_success() {
                    return Err(ComposioError::from_response(response).await);
                }

                Ok(response)
            }
        })
        .await?;

        // Parse successful response
        let link_response: crate::models::response::LinkResponse = response
            .json()
            .await
            .map_err(ComposioError::NetworkError)?;

        Ok(link_response)
    }
}

/// Builder for creating sessions with fluent API
///
/// # Example
///
/// ```no_run
/// use composio_sdk::ComposioClient;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = ComposioClient::builder()
///     .api_key("your-api-key")
///     .build()?;
///
/// let session = client
///     .create_session("user_123")
///     .toolkits(vec!["github", "gmail"])
///     .manage_connections(true)
///     .send()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct SessionBuilder<'a> {
    client: &'a ComposioClient,
    #[allow(dead_code)]
    user_id: String,
    config: SessionConfig,
}

impl<'a> SessionBuilder<'a> {
    /// Create a new session builder
    ///
    /// # Arguments
    ///
    /// * `client` - Reference to the ComposioClient
    /// * `user_id` - User identifier for session isolation
    pub fn new(client: &'a ComposioClient, user_id: String) -> Self {
        Self {
            client,
            user_id: user_id.clone(),
            config: SessionConfig {
                user_id,
                toolkits: None,
                auth_configs: None,
                connected_accounts: None,
                manage_connections: None,
                tools: None,
                tags: None,
                workbench: None,
            },
        }
    }

    /// Enable specific toolkits for this session
    ///
    /// By default, all toolkits are accessible via COMPOSIO_SEARCH_TOOLS.
    /// Use this method to restrict the session to specific toolkits.
    ///
    /// # Arguments
    ///
    /// * `toolkits` - Vector of toolkit slugs to enable (e.g., "github", "gmail")
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::ComposioClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = ComposioClient::builder().api_key("key").build()?;
    /// let session = client
    ///     .create_session("user_123")
    ///     .toolkits(vec!["github", "gmail", "slack"])
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn toolkits(mut self, toolkits: Vec<impl Into<String>>) -> Self {
        self.config.toolkits = Some(ToolkitFilter::Enable(
            toolkits.into_iter().map(|t| t.into()).collect(),
        ));
        self
    }

    /// Disable specific toolkits for this session
    ///
    /// Use this to exclude certain toolkits while keeping all others accessible.
    ///
    /// # Arguments
    ///
    /// * `toolkits` - Vector of toolkit slugs to disable
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::ComposioClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = ComposioClient::builder().api_key("key").build()?;
    /// let session = client
    ///     .create_session("user_123")
    ///     .disable_toolkits(vec!["exa", "firecrawl"])
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn disable_toolkits(mut self, toolkits: Vec<impl Into<String>>) -> Self {
        self.config.toolkits = Some(ToolkitFilter::Disable {
            disable: toolkits.into_iter().map(|t| t.into()).collect(),
        });
        self
    }

    /// Override the default auth config for a specific toolkit
    ///
    /// Use this to specify a custom auth configuration (e.g., your own OAuth app)
    /// instead of Composio's managed authentication.
    ///
    /// # Arguments
    ///
    /// * `toolkit` - Toolkit slug (e.g., "github")
    /// * `auth_config_id` - Auth config ID (e.g., "ac_your_config")
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::ComposioClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = ComposioClient::builder().api_key("key").build()?;
    /// let session = client
    ///     .create_session("user_123")
    ///     .auth_config("github", "ac_custom_github_oauth")
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn auth_config(
        mut self,
        toolkit: impl Into<String>,
        auth_config_id: impl Into<String>,
    ) -> Self {
        self.config
            .auth_configs
            .get_or_insert_with(HashMap::new)
            .insert(toolkit.into(), auth_config_id.into());
        self
    }

    /// Select a specific connected account for a toolkit
    ///
    /// Use this when a user has multiple connected accounts for the same toolkit
    /// (e.g., work and personal email) and you want to specify which one to use.
    ///
    /// # Arguments
    ///
    /// * `toolkit` - Toolkit slug (e.g., "gmail")
    /// * `connected_account_id` - Connected account ID (e.g., "ca_work_gmail")
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::ComposioClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = ComposioClient::builder().api_key("key").build()?;
    /// let session = client
    ///     .create_session("user_123")
    ///     .connected_account("gmail", "ca_work_gmail")
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn connected_account(
        mut self,
        toolkit: impl Into<String>,
        connected_account_id: impl Into<String>,
    ) -> Self {
        self.config
            .connected_accounts
            .get_or_insert_with(HashMap::new)
            .insert(toolkit.into(), connected_account_id.into());
        self
    }

    /// Enable or disable automatic connection management
    ///
    /// When enabled (default), the agent will automatically prompt users with
    /// Connect Links during chat when authentication is needed.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable automatic connection management
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::ComposioClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = ComposioClient::builder().api_key("key").build()?;
    /// // Disable in-chat authentication (use manual auth flow instead)
    /// let session = client
    ///     .create_session("user_123")
    ///     .manage_connections(false)
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn manage_connections(mut self, enabled: bool) -> Self {
        self.config.manage_connections = Some(crate::models::ManageConnectionsConfig::Bool(enabled));
        self
    }

    /// Configure per-toolkit tool filtering
    ///
    /// Use this to enable or disable specific tools within a toolkit.
    ///
    /// # Arguments
    ///
    /// * `toolkit` - Toolkit slug (e.g., "github")
    /// * `tools` - Vector of tool slugs to enable
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::ComposioClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = ComposioClient::builder().api_key("key").build()?;
    /// let session = client
    ///     .create_session("user_123")
    ///     .tools("github", vec!["GITHUB_CREATE_ISSUE", "GITHUB_GET_REPOS"])
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn tools(mut self, toolkit: impl Into<String>, tools: Vec<impl Into<String>>) -> Self {
        let tool_filter = ToolFilter::EnableList(tools.into_iter().map(|t| t.into()).collect());

        self.config
            .tools
            .get_or_insert_with(|| ToolsConfig(HashMap::new()))
            .0
            .insert(toolkit.into(), tool_filter);
        self
    }

    /// Configure tag-based tool filtering
    ///
    /// Tags are MCP annotation hints that categorize tools by behavior:
    /// - `readOnlyHint`: Read-only tools (safe, no modifications)
    /// - `destructiveHint`: Tools that modify or delete data
    /// - `idempotentHint`: Tools that can be safely retried
    /// - `openWorldHint`: Tools that interact with external world
    ///
    /// # Arguments
    ///
    /// * `enabled` - Tags that tools must have (at least one)
    /// * `disabled` - Tags that tools must NOT have (any)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::ComposioClient;
    /// # use composio_sdk::models::enums::TagType;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = ComposioClient::builder().api_key("key").build()?;
    /// // Only allow read-only tools, exclude destructive ones
    /// let session = client
    ///     .create_session("user_123")
    ///     .tags(
    ///         Some(vec![TagType::ReadOnlyHint]),
    ///         Some(vec![TagType::DestructiveHint])
    ///     )
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn tags(
        mut self,
        enabled: Option<Vec<TagType>>,
        disabled: Option<Vec<TagType>>,
    ) -> Self {
        self.config.tags = Some(TagsConfig { enabled, disabled });
        self
    }

    /// Configure workbench settings
    ///
    /// The workbench is a persistent Python sandbox for complex operations.
    ///
    /// # Arguments
    ///
    /// * `proxy_execution` - Whether to enable proxy execution
    /// * `auto_offload_threshold` - Threshold for automatic offloading
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::ComposioClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = ComposioClient::builder().api_key("key").build()?;
    /// let session = client
    ///     .create_session("user_123")
    ///     .workbench(Some(true), Some(1000))
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn workbench(
        mut self,
        proxy_execution: Option<bool>,
        auto_offload_threshold: Option<u32>,
    ) -> Self {
        self.config.workbench = Some(WorkbenchConfig {
            proxy_execution,
            auto_offload_threshold,
        });
        self
    }

    /// Send the session creation request
    ///
    /// This consumes the builder and creates the session on the Composio API.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The API request fails
    /// - The response cannot be parsed
    /// - Authentication is invalid
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::ComposioClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = ComposioClient::builder().api_key("key").build()?;
    /// let session = client
    ///     .create_session("user_123")
    ///     .toolkits(vec!["github"])
    ///     .send()
    ///     .await?;
    ///
    /// println!("Session ID: {}", session.session_id());
    /// println!("MCP URL: {}", session.mcp_url());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send(self) -> Result<Session, ComposioError> {
        use crate::models::response::SessionResponse;
        use crate::retry::with_retry;

        let url = format!("{}/tool_router/session", self.client.config().base_url);
        let policy = &self.client.config().retry_policy;

        // Execute request with retry logic
        let response = with_retry(policy, || {
            let url = url.clone();
            let config = self.config.clone();
            let client = self.client.http_client().clone();

            async move {
                let response = client
                    .post(&url)
                    .json(&config)
                    .send()
                    .await
                    .map_err(ComposioError::NetworkError)?;

                // Check for HTTP errors
                if !response.status().is_success() {
                    return Err(ComposioError::from_response(response).await);
                }

                Ok(response)
            }
        })
        .await?;

        // Parse successful response
        let session_response: SessionResponse = response
            .json()
            .await
            .map_err(ComposioError::NetworkError)?;

        // Create Session struct with Arc-wrapped client
        Ok(Session {
            client: Arc::new(self.client.clone()),
            session_id: session_response.session_id,
            mcp_url: session_response.mcp.url,
            tools: session_response.tool_router_tools,
        })
    }
}

/// Builder for listing toolkits with filtering options
///
/// This builder provides a fluent API for configuring toolkit listing requests.
/// It supports pagination, filtering by connection status, searching, and
/// filtering by specific toolkit slugs.
///
/// # Example
///
/// ```no_run
/// # use composio_sdk::ComposioClient;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let client = ComposioClient::builder().api_key("key").build()?;
/// let session = client.create_session("user_123").send().await?;
///
/// // List first 10 connected toolkits
/// let toolkits = session.list_toolkits()
///     .limit(10)
///     .is_connected(true)
///     .send()
///     .await?;
///
/// // Paginate through results
/// let mut cursor = None;
/// loop {
///     let mut builder = session.list_toolkits().limit(20);
///     if let Some(c) = cursor {
///         builder = builder.cursor(c);
///     }
///     
///     let response = builder.send().await?;
///     // Process response.items...
///     
///     cursor = response.next_cursor;
///     if cursor.is_none() {
///         break;
///     }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct ToolkitListBuilder<'a> {
    session: &'a Session,
    limit: Option<u32>,
    cursor: Option<String>,
    toolkits: Option<Vec<String>>,
    is_connected: Option<bool>,
    search: Option<String>,
}

impl<'a> ToolkitListBuilder<'a> {
    /// Create a new toolkit list builder
    fn new(session: &'a Session) -> Self {
        Self {
            session,
            limit: None,
            cursor: None,
            toolkits: None,
            is_connected: None,
            search: None,
        }
    }

    /// Set the maximum number of toolkits to return
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of toolkits (default: 20)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::ComposioClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = ComposioClient::builder().api_key("key").build()?;
    /// # let session = client.create_session("user_123").send().await?;
    /// let toolkits = session.list_toolkits()
    ///     .limit(50)
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set the pagination cursor
    ///
    /// Use the `next_cursor` value from a previous response to fetch
    /// the next page of results.
    ///
    /// # Arguments
    ///
    /// * `cursor` - Pagination cursor from previous response
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::ComposioClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = ComposioClient::builder().api_key("key").build()?;
    /// # let session = client.create_session("user_123").send().await?;
    /// let first_page = session.list_toolkits().limit(20).send().await?;
    ///
    /// if let Some(cursor) = first_page.next_cursor {
    ///     let second_page = session.list_toolkits()
    ///         .limit(20)
    ///         .cursor(cursor)
    ///         .send()
    ///         .await?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn cursor(mut self, cursor: impl Into<String>) -> Self {
        self.cursor = Some(cursor.into());
        self
    }

    /// Filter by specific toolkit slugs
    ///
    /// Only return toolkits matching the provided slugs.
    ///
    /// # Arguments
    ///
    /// * `toolkits` - List of toolkit slugs to filter by
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::ComposioClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = ComposioClient::builder().api_key("key").build()?;
    /// # let session = client.create_session("user_123").send().await?;
    /// let toolkits = session.list_toolkits()
    ///     .toolkits(vec!["github", "gmail", "slack"])
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn toolkits(mut self, toolkits: Vec<impl Into<String>>) -> Self {
        self.toolkits = Some(toolkits.into_iter().map(|t| t.into()).collect());
        self
    }

    /// Filter by connection status
    ///
    /// When set to `true`, only returns toolkits that have an active
    /// connected account. When set to `false`, only returns toolkits
    /// without a connected account.
    ///
    /// # Arguments
    ///
    /// * `is_connected` - Whether to filter by connection status
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::ComposioClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = ComposioClient::builder().api_key("key").build()?;
    /// # let session = client.create_session("user_123").send().await?;
    /// // Get only connected toolkits
    /// let connected = session.list_toolkits()
    ///     .is_connected(true)
    ///     .send()
    ///     .await?;
    ///
    /// // Get only disconnected toolkits
    /// let disconnected = session.list_toolkits()
    ///     .is_connected(false)
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn is_connected(mut self, is_connected: bool) -> Self {
        self.is_connected = Some(is_connected);
        self
    }

    /// Search toolkits by name or slug
    ///
    /// Returns toolkits whose name or slug contains the search query
    /// (case-insensitive).
    ///
    /// # Arguments
    ///
    /// * `search` - Search query string
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::ComposioClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = ComposioClient::builder().api_key("key").build()?;
    /// # let session = client.create_session("user_123").send().await?;
    /// let github_toolkits = session.list_toolkits()
    ///     .search("github")
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn search(mut self, search: impl Into<String>) -> Self {
        self.search = Some(search.into());
        self
    }

    /// Execute the toolkit listing request
    ///
    /// Sends the request to the Composio API and returns the list of toolkits
    /// matching the configured filters.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The API request fails
    /// - The response cannot be parsed
    /// - Authentication is invalid
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::ComposioClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = ComposioClient::builder().api_key("key").build()?;
    /// # let session = client.create_session("user_123").send().await?;
    /// let response = session.list_toolkits()
    ///     .limit(10)
    ///     .is_connected(true)
    ///     .send()
    ///     .await?;
    ///
    /// println!("Found {} toolkits", response.items.len());
    /// println!("Total: {}", response.total_items);
    /// println!("Page {} of {}", response.current_page, response.total_pages);
    ///
    /// for toolkit in response.items {
    ///     println!("- {} ({})", toolkit.name, toolkit.slug);
    ///     if let Some(account) = toolkit.connected_account {
    ///         println!("  Connected: {} ({})", account.id, account.status);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send(self) -> Result<crate::models::response::ToolkitListResponse, ComposioError> {
        use crate::models::response::ToolkitListResponse;
        use crate::retry::with_retry;

        let url = format!(
            "{}/tool_router/session/{}/toolkits",
            self.session.client.config().base_url,
            self.session.session_id
        );

        // Build query parameters
        let mut query_params = Vec::new();

        if let Some(limit) = self.limit {
            query_params.push(("limit", limit.to_string()));
        }

        if let Some(cursor) = &self.cursor {
            query_params.push(("cursor", cursor.clone()));
        }

        if let Some(toolkits) = &self.toolkits {
            query_params.push(("toolkits", toolkits.join(",")));
        }

        if let Some(is_connected) = self.is_connected {
            query_params.push(("is_connected", is_connected.to_string()));
        }

        if let Some(search) = &self.search {
            query_params.push(("search", search.clone()));
        }

        let policy = &self.session.client.config().retry_policy;

        // Execute request with retry logic
        let response = with_retry(policy, || {
            let url = url.clone();
            let query_params = query_params.clone();
            let client = self.session.client.http_client().clone();

            async move {
                let response = client
                    .get(&url)
                    .query(&query_params)
                    .send()
                    .await
                    .map_err(ComposioError::NetworkError)?;

                // Check for HTTP errors
                if !response.status().is_success() {
                    return Err(ComposioError::from_response(response).await);
                }

                Ok(response)
            }
        })
        .await?;

        // Parse successful response
        let toolkit_response: ToolkitListResponse = response
            .json()
            .await
            .map_err(ComposioError::NetworkError)?;

        Ok(toolkit_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::ComposioClient;
    use crate::models::enums::TagType;
    use crate::models::request::{ManageConnectionsConfig, ToolFilter, ToolkitFilter};

    fn create_test_client() -> ComposioClient {
        ComposioClient::builder()
            .api_key("test_api_key")
            .build()
            .unwrap()
    }

    #[test]
    fn test_session_builder_new() {
        let client = create_test_client();
        let builder = SessionBuilder::new(&client, "user_123".to_string());

        assert_eq!(builder.user_id, "user_123");
        assert!(builder.config.toolkits.is_none());
        assert!(builder.config.auth_configs.is_none());
        assert!(builder.config.connected_accounts.is_none());
        assert!(builder.config.manage_connections.is_none());
        assert!(builder.config.tools.is_none());
        assert!(builder.config.tags.is_none());
        assert!(builder.config.workbench.is_none());
    }

    #[test]
    fn test_session_builder_toolkits_enable() {
        let client = create_test_client();
        let builder = SessionBuilder::new(&client, "user_123".to_string())
            .toolkits(vec!["github", "gmail"]);

        match builder.config.toolkits {
            Some(ToolkitFilter::Enable(toolkits)) => {
                assert_eq!(toolkits.len(), 2);
                assert!(toolkits.contains(&"github".to_string()));
                assert!(toolkits.contains(&"gmail".to_string()));
            }
            _ => panic!("Expected Enable variant"),
        }
    }

    #[test]
    fn test_session_builder_disable_toolkits() {
        let client = create_test_client();
        let builder = SessionBuilder::new(&client, "user_123".to_string())
            .disable_toolkits(vec!["exa", "firecrawl"]);

        match builder.config.toolkits {
            Some(ToolkitFilter::Disable { disable }) => {
                assert_eq!(disable.len(), 2);
                assert!(disable.contains(&"exa".to_string()));
                assert!(disable.contains(&"firecrawl".to_string()));
            }
            _ => panic!("Expected Disable variant"),
        }
    }

    #[test]
    fn test_session_builder_auth_config() {
        let client = create_test_client();
        let builder = SessionBuilder::new(&client, "user_123".to_string())
            .auth_config("github", "ac_custom_config");

        let auth_configs = builder.config.auth_configs.unwrap();
        assert_eq!(auth_configs.len(), 1);
        assert_eq!(auth_configs.get("github"), Some(&"ac_custom_config".to_string()));
    }

    #[test]
    fn test_session_builder_multiple_auth_configs() {
        let client = create_test_client();
        let builder = SessionBuilder::new(&client, "user_123".to_string())
            .auth_config("github", "ac_github_config")
            .auth_config("gmail", "ac_gmail_config");

        let auth_configs = builder.config.auth_configs.unwrap();
        assert_eq!(auth_configs.len(), 2);
        assert_eq!(auth_configs.get("github"), Some(&"ac_github_config".to_string()));
        assert_eq!(auth_configs.get("gmail"), Some(&"ac_gmail_config".to_string()));
    }

    #[test]
    fn test_session_builder_connected_account() {
        let client = create_test_client();
        let builder = SessionBuilder::new(&client, "user_123".to_string())
            .connected_account("gmail", "ca_work_gmail");

        let connected_accounts = builder.config.connected_accounts.unwrap();
        assert_eq!(connected_accounts.len(), 1);
        assert_eq!(connected_accounts.get("gmail"), Some(&"ca_work_gmail".to_string()));
    }

    #[test]
    fn test_session_builder_multiple_connected_accounts() {
        let client = create_test_client();
        let builder = SessionBuilder::new(&client, "user_123".to_string())
            .connected_account("gmail", "ca_work_gmail")
            .connected_account("github", "ca_personal_github");

        let connected_accounts = builder.config.connected_accounts.unwrap();
        assert_eq!(connected_accounts.len(), 2);
        assert_eq!(connected_accounts.get("gmail"), Some(&"ca_work_gmail".to_string()));
        assert_eq!(connected_accounts.get("github"), Some(&"ca_personal_github".to_string()));
    }

    #[test]
    fn test_session_builder_manage_connections_true() {
        let client = create_test_client();
        let builder = SessionBuilder::new(&client, "user_123".to_string())
            .manage_connections(true);

        match builder.config.manage_connections {
            Some(ManageConnectionsConfig::Bool(enabled)) => {
                assert!(enabled);
            }
            _ => panic!("Expected Bool variant with true"),
        }
    }

    #[test]
    fn test_session_builder_manage_connections_false() {
        let client = create_test_client();
        let builder = SessionBuilder::new(&client, "user_123".to_string())
            .manage_connections(false);

        match builder.config.manage_connections {
            Some(ManageConnectionsConfig::Bool(enabled)) => {
                assert!(!enabled);
            }
            _ => panic!("Expected Bool variant with false"),
        }
    }

    #[test]
    fn test_session_builder_tools_enable() {
        let client = create_test_client();
        let builder = SessionBuilder::new(&client, "user_123".to_string())
            .tools("github", vec!["GITHUB_CREATE_ISSUE", "GITHUB_GET_REPOS"]);

        let tools_config = builder.config.tools.unwrap();
        let github_filter = tools_config.0.get("github").unwrap();

        match github_filter {
            ToolFilter::EnableList(tools) => {
                assert_eq!(tools.len(), 2);
                assert!(tools.contains(&"GITHUB_CREATE_ISSUE".to_string()));
                assert!(tools.contains(&"GITHUB_GET_REPOS".to_string()));
            }
            _ => panic!("Expected EnableList variant"),
        }
    }

    #[test]
    fn test_session_builder_multiple_toolkit_tools() {
        let client = create_test_client();
        let builder = SessionBuilder::new(&client, "user_123".to_string())
            .tools("github", vec!["GITHUB_CREATE_ISSUE"])
            .tools("gmail", vec!["GMAIL_SEND_EMAIL"]);

        let tools_config = builder.config.tools.unwrap();
        assert_eq!(tools_config.0.len(), 2);
        assert!(tools_config.0.contains_key("github"));
        assert!(tools_config.0.contains_key("gmail"));
    }

    #[test]
    fn test_session_builder_tags_enabled() {
        let client = create_test_client();
        let builder = SessionBuilder::new(&client, "user_123".to_string())
            .tags(Some(vec![TagType::ReadOnlyHint, TagType::IdempotentHint]), None);

        let tags_config = builder.config.tags.unwrap();
        let enabled = tags_config.enabled.unwrap();
        assert_eq!(enabled.len(), 2);
        assert!(enabled.contains(&TagType::ReadOnlyHint));
        assert!(enabled.contains(&TagType::IdempotentHint));
        assert!(tags_config.disabled.is_none());
    }

    #[test]
    fn test_session_builder_tags_disabled() {
        let client = create_test_client();
        let builder = SessionBuilder::new(&client, "user_123".to_string())
            .tags(None, Some(vec![TagType::DestructiveHint]));

        let tags_config = builder.config.tags.unwrap();
        let disabled = tags_config.disabled.unwrap();
        assert_eq!(disabled.len(), 1);
        assert!(disabled.contains(&TagType::DestructiveHint));
        assert!(tags_config.enabled.is_none());
    }

    #[test]
    fn test_session_builder_tags_both() {
        let client = create_test_client();
        let builder = SessionBuilder::new(&client, "user_123".to_string())
            .tags(
                Some(vec![TagType::ReadOnlyHint]),
                Some(vec![TagType::DestructiveHint])
            );

        let tags_config = builder.config.tags.unwrap();
        assert!(tags_config.enabled.is_some());
        assert!(tags_config.disabled.is_some());
    }

    #[test]
    fn test_session_builder_workbench() {
        let client = create_test_client();
        let builder = SessionBuilder::new(&client, "user_123".to_string())
            .workbench(Some(true), Some(1000));

        let workbench_config = builder.config.workbench.unwrap();
        assert_eq!(workbench_config.proxy_execution, Some(true));
        assert_eq!(workbench_config.auto_offload_threshold, Some(1000));
    }

    #[test]
    fn test_session_builder_workbench_no_threshold() {
        let client = create_test_client();
        let builder = SessionBuilder::new(&client, "user_123".to_string())
            .workbench(Some(false), None);

        let workbench_config = builder.config.workbench.unwrap();
        assert_eq!(workbench_config.proxy_execution, Some(false));
        assert_eq!(workbench_config.auto_offload_threshold, None);
    }

    #[test]
    fn test_session_builder_method_chaining() {
        let client = create_test_client();
        let builder = SessionBuilder::new(&client, "user_123".to_string())
            .toolkits(vec!["github", "gmail"])
            .auth_config("github", "ac_custom")
            .connected_account("gmail", "ca_work")
            .manage_connections(true)
            .tools("github", vec!["GITHUB_CREATE_ISSUE"])
            .tags(Some(vec![TagType::ReadOnlyHint]), None)
            .workbench(Some(true), Some(500));

        // Verify all configurations are set
        assert!(builder.config.toolkits.is_some());
        assert!(builder.config.auth_configs.is_some());
        assert!(builder.config.connected_accounts.is_some());
        assert!(builder.config.manage_connections.is_some());
        assert!(builder.config.tools.is_some());
        assert!(builder.config.tags.is_some());
        assert!(builder.config.workbench.is_some());
    }

    #[test]
    fn test_session_session_id_accessor() {
        let client = Arc::new(create_test_client());
        let session = Session {
            client,
            session_id: "sess_123".to_string(),
            mcp_url: "https://mcp.composio.dev".to_string(),
            tools: vec!["COMPOSIO_SEARCH_TOOLS".to_string()],
        };

        assert_eq!(session.session_id(), "sess_123");
    }

    #[test]
    fn test_session_mcp_url_accessor() {
        let client = Arc::new(create_test_client());
        let session = Session {
            client,
            session_id: "sess_123".to_string(),
            mcp_url: "https://mcp.composio.dev".to_string(),
            tools: vec!["COMPOSIO_SEARCH_TOOLS".to_string()],
        };

        assert_eq!(session.mcp_url(), "https://mcp.composio.dev");
    }

    #[test]
    fn test_session_tools_accessor() {
        let client = Arc::new(create_test_client());
        let tools = vec![
            "COMPOSIO_SEARCH_TOOLS".to_string(),
            "COMPOSIO_MULTI_EXECUTE_TOOL".to_string(),
        ];
        let session = Session {
            client,
            session_id: "sess_123".to_string(),
            mcp_url: "https://mcp.composio.dev".to_string(),
            tools: tools.clone(),
        };

        assert_eq!(session.tools(), &tools);
        assert_eq!(session.tools().len(), 2);
    }

    #[test]
    fn test_toolkit_list_builder_new() {
        let client = Arc::new(create_test_client());
        let session = Session {
            client,
            session_id: "sess_123".to_string(),
            mcp_url: "https://mcp.composio.dev".to_string(),
            tools: vec![],
        };

        let builder = ToolkitListBuilder::new(&session);
        assert!(builder.limit.is_none());
        assert!(builder.cursor.is_none());
        assert!(builder.toolkits.is_none());
        assert!(builder.is_connected.is_none());
        assert!(builder.search.is_none());
    }

    #[test]
    fn test_toolkit_list_builder_limit() {
        let client = Arc::new(create_test_client());
        let session = Session {
            client,
            session_id: "sess_123".to_string(),
            mcp_url: "https://mcp.composio.dev".to_string(),
            tools: vec![],
        };

        let builder = session.list_toolkits().limit(50);
        assert_eq!(builder.limit, Some(50));
    }

    #[test]
    fn test_toolkit_list_builder_cursor() {
        let client = Arc::new(create_test_client());
        let session = Session {
            client,
            session_id: "sess_123".to_string(),
            mcp_url: "https://mcp.composio.dev".to_string(),
            tools: vec![],
        };

        let builder = session.list_toolkits().cursor("cursor_abc");
        assert_eq!(builder.cursor, Some("cursor_abc".to_string()));
    }

    #[test]
    fn test_toolkit_list_builder_toolkits() {
        let client = Arc::new(create_test_client());
        let session = Session {
            client,
            session_id: "sess_123".to_string(),
            mcp_url: "https://mcp.composio.dev".to_string(),
            tools: vec![],
        };

        let builder = session.list_toolkits().toolkits(vec!["github", "gmail"]);
        let toolkits = builder.toolkits.unwrap();
        assert_eq!(toolkits.len(), 2);
        assert!(toolkits.contains(&"github".to_string()));
        assert!(toolkits.contains(&"gmail".to_string()));
    }

    #[test]
    fn test_toolkit_list_builder_is_connected() {
        let client = Arc::new(create_test_client());
        let session = Session {
            client,
            session_id: "sess_123".to_string(),
            mcp_url: "https://mcp.composio.dev".to_string(),
            tools: vec![],
        };

        let builder = session.list_toolkits().is_connected(true);
        assert_eq!(builder.is_connected, Some(true));
    }

    #[test]
    fn test_toolkit_list_builder_search() {
        let client = Arc::new(create_test_client());
        let session = Session {
            client,
            session_id: "sess_123".to_string(),
            mcp_url: "https://mcp.composio.dev".to_string(),
            tools: vec![],
        };

        let builder = session.list_toolkits().search("communication");
        assert_eq!(builder.search, Some("communication".to_string()));
    }

    #[test]
    fn test_toolkit_list_builder_method_chaining() {
        let client = Arc::new(create_test_client());
        let session = Session {
            client,
            session_id: "sess_123".to_string(),
            mcp_url: "https://mcp.composio.dev".to_string(),
            tools: vec![],
        };

        let builder = session.list_toolkits()
            .limit(25)
            .cursor("cursor_xyz")
            .toolkits(vec!["github"])
            .is_connected(true)
            .search("git");

        assert_eq!(builder.limit, Some(25));
        assert_eq!(builder.cursor, Some("cursor_xyz".to_string()));
        assert!(builder.toolkits.is_some());
        assert_eq!(builder.is_connected, Some(true));
        assert_eq!(builder.search, Some("git".to_string()));
    }
}
