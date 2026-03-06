//! HTTP client for Composio API
//!
//! This module provides the main HTTP client for interacting with the Composio API.
//! It uses the builder pattern for flexible configuration and includes automatic
//! retry logic for transient failures.
//!
//! # Example
//!
//! ```no_run
//! use composio_sdk::client::ComposioClient;
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = ComposioClient::builder()
//!     .api_key("your_api_key")
//!     .timeout(Duration::from_secs(60))
//!     .max_retries(5)
//!     .build()?;
//! # Ok(())
//! # }
//! ```

use crate::config::ComposioConfig;
use crate::error::ComposioError;
use crate::retry::RetryPolicy;
use std::time::Duration;

/// Main client for interacting with Composio API
///
/// The client manages HTTP connections and configuration for all API requests.
/// It includes automatic retry logic for transient failures and proper error handling.
///
/// # Example
///
/// ```no_run
/// use composio_sdk::client::ComposioClient;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = ComposioClient::builder()
///     .api_key("your_api_key")
///     .build()?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ComposioClient {
    http_client: reqwest::Client,
    config: ComposioConfig,
}

/// Builder for ComposioClient
///
/// Provides a fluent API for configuring the Composio client with custom settings.
/// All configuration options are optional and will use sensible defaults if not specified.
///
/// # Example
///
/// ```no_run
/// use composio_sdk::client::ComposioClient;
/// use std::time::Duration;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = ComposioClient::builder()
///     .api_key("your_api_key")
///     .base_url("https://custom.api.com")
///     .timeout(Duration::from_secs(60))
///     .max_retries(5)
///     .initial_retry_delay(Duration::from_secs(2))
///     .max_retry_delay(Duration::from_secs(30))
///     .build()?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default)]
pub struct ComposioClientBuilder {
    api_key: Option<String>,
    base_url: Option<String>,
    timeout: Option<Duration>,
    max_retries: Option<u32>,
    initial_retry_delay: Option<Duration>,
    max_retry_delay: Option<Duration>,
}

impl ComposioClient {
    /// Create a new client builder
    ///
    /// Returns a `ComposioClientBuilder` that can be used to configure and build
    /// a `ComposioClient` instance.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::client::ComposioClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ComposioClient::builder()
    ///     .api_key("your_api_key")
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn builder() -> ComposioClientBuilder {
        ComposioClientBuilder::default()
    }

    /// Get a reference to the HTTP client
    ///
    /// This is useful for advanced use cases where you need direct access to the
    /// underlying reqwest client.
    pub fn http_client(&self) -> &reqwest::Client {
        &self.http_client
    }

    /// Get a reference to the configuration
    ///
    /// Returns the configuration used by this client.
    pub fn config(&self) -> &ComposioConfig {
        &self.config
    }

    /// Create a new session for a user
    ///
    /// Returns a `SessionBuilder` that can be used to configure and create
    /// a Tool Router session for the specified user.
    ///
    /// # Arguments
    ///
    /// * `user_id` - User identifier for session isolation
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::client::ComposioClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ComposioClient::builder()
    ///     .api_key("your_api_key")
    ///     .build()?;
    ///
    /// let session = client
    ///     .create_session("user_123")
    ///     .toolkits(vec!["github", "gmail"])
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_session(&self, user_id: impl Into<String>) -> crate::session::SessionBuilder<'_> {
        crate::session::SessionBuilder::new(self, user_id.into())
    }

    /// Get an existing session by ID
    ///
    /// Retrieves session details for a previously created Tool Router session.
    /// This is useful for inspecting session configuration and available tools.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session ID to retrieve
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Session not found (404)
    /// - Network error occurs
    /// - API returns an error response
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::client::ComposioClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ComposioClient::builder()
    ///     .api_key("your_api_key")
    ///     .build()?;
    ///
    /// let session = client.get_session("sess_abc123").await?;
    /// println!("Session ID: {}", session.session_id());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_session(
        &self,
        session_id: impl Into<String>,
    ) -> Result<crate::session::Session, ComposioError> {
        let session_id = session_id.into();
        let url = format!(
            "{}/tool_router/session/{}",
            self.config.base_url, session_id
        );

        // Execute request with retry logic
        let response = crate::retry::with_retry(&self.config.retry_policy, || async {
            let response = self
                .http_client
                .get(&url)
                .send()
                .await
                .map_err(ComposioError::NetworkError)?;

            // Check for errors
            if !response.status().is_success() {
                return Err(ComposioError::from_response(response).await);
            }

            Ok(response)
        })
        .await?;

        // Parse response
        let session_response: crate::models::SessionResponse = response
            .json()
            .await
            .map_err(ComposioError::NetworkError)?;

        // Convert to Session
        Ok(crate::session::Session::from_response(
            self.clone(),
            session_response,
        ))
    }
}

impl ComposioClientBuilder {
    /// Set the API key
    ///
    /// The API key is required for authenticating with the Composio API.
    /// You can obtain your API key from the Composio dashboard.
    ///
    /// # Arguments
    ///
    /// * `key` - The Composio API key (can be `String` or `&str`)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::client::ComposioClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ComposioClient::builder()
    ///     .api_key("your_api_key")
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Set the base URL
    ///
    /// Override the default Composio API base URL. This is useful for testing
    /// or when using a custom Composio deployment.
    ///
    /// # Arguments
    ///
    /// * `url` - The base URL (must start with http:// or https://)
    ///
    /// # Default
    ///
    /// `https://backend.composio.dev/api/v3`
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::client::ComposioClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ComposioClient::builder()
    ///     .api_key("your_api_key")
    ///     .base_url("https://custom.api.com")
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Set the request timeout
    ///
    /// Configure how long to wait for API requests to complete before timing out.
    ///
    /// # Arguments
    ///
    /// * `timeout` - The timeout duration
    ///
    /// # Default
    ///
    /// 30 seconds
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::client::ComposioClient;
    /// use std::time::Duration;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ComposioClient::builder()
    ///     .api_key("your_api_key")
    ///     .timeout(Duration::from_secs(60))
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set the maximum number of retries
    ///
    /// Configure how many times to retry failed requests for transient errors
    /// (rate limits, server errors, network issues).
    ///
    /// # Arguments
    ///
    /// * `retries` - Maximum number of retry attempts
    ///
    /// # Default
    ///
    /// 3 retries
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::client::ComposioClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ComposioClient::builder()
    ///     .api_key("your_api_key")
    ///     .max_retries(5)
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = Some(retries);
        self
    }

    /// Set the initial retry delay
    ///
    /// Configure the delay before the first retry attempt. Subsequent retries
    /// use exponential backoff based on this initial delay.
    ///
    /// # Arguments
    ///
    /// * `delay` - Initial delay duration
    ///
    /// # Default
    ///
    /// 1 second
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::client::ComposioClient;
    /// use std::time::Duration;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ComposioClient::builder()
    ///     .api_key("your_api_key")
    ///     .initial_retry_delay(Duration::from_secs(2))
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn initial_retry_delay(mut self, delay: Duration) -> Self {
        self.initial_retry_delay = Some(delay);
        self
    }

    /// Set the maximum retry delay
    ///
    /// Configure the maximum delay between retry attempts. This caps the
    /// exponential backoff to prevent excessively long waits.
    ///
    /// # Arguments
    ///
    /// * `delay` - Maximum delay duration
    ///
    /// # Default
    ///
    /// 10 seconds
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::client::ComposioClient;
    /// use std::time::Duration;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ComposioClient::builder()
    ///     .api_key("your_api_key")
    ///     .max_retry_delay(Duration::from_secs(30))
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn max_retry_delay(mut self, delay: Duration) -> Self {
        self.max_retry_delay = Some(delay);
        self
    }

    /// Build the client
    ///
    /// Validates the configuration and constructs a `ComposioClient` instance.
    /// The reqwest HTTP client is configured with the specified timeout and
    /// default headers (including the API key).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - API key is not provided or is empty
    /// - Base URL is invalid (doesn't start with http:// or https://)
    /// - HTTP client construction fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::client::ComposioClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ComposioClient::builder()
    ///     .api_key("your_api_key")
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(self) -> Result<ComposioClient, ComposioError> {
        // Require API key
        let api_key = self.api_key.ok_or_else(|| {
            ComposioError::InvalidInput("API key is required".to_string())
        })?;

        // Build configuration with defaults
        let mut config = ComposioConfig::new(api_key);

        if let Some(base_url) = self.base_url {
            config.base_url = base_url;
        }

        if let Some(timeout) = self.timeout {
            config.timeout = timeout;
        }

        // Build retry policy
        let mut retry_policy = RetryPolicy::default();
        if let Some(max_retries) = self.max_retries {
            retry_policy.max_retries = max_retries;
        }
        if let Some(initial_delay) = self.initial_retry_delay {
            retry_policy.initial_delay = initial_delay;
        }
        if let Some(max_delay) = self.max_retry_delay {
            retry_policy.max_delay = max_delay;
        }
        config.retry_policy = retry_policy;

        // Validate configuration
        config.validate()?;

        // Build HTTP client with timeout and default headers
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "x-api-key",
            reqwest::header::HeaderValue::from_str(&config.api_key)
                .map_err(|_| ComposioError::InvalidInput("Invalid API key format".to_string()))?,
        );

        let http_client = reqwest::Client::builder()
            .timeout(config.timeout)
            .default_headers(headers)
            .build()
            .map_err(|e| ComposioError::NetworkError(e))?;

        Ok(ComposioClient {
            http_client,
            config,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_with_api_key_only() {
        let client = ComposioClient::builder()
            .api_key("test_key")
            .build()
            .unwrap();

        assert_eq!(client.config().api_key, "test_key");
        assert_eq!(
            client.config().base_url,
            "https://backend.composio.dev/api/v3"
        );
        assert_eq!(client.config().timeout, Duration::from_secs(30));
        assert_eq!(client.config().retry_policy.max_retries, 3);
    }

    #[test]
    fn test_builder_with_all_options() {
        let client = ComposioClient::builder()
            .api_key("test_key")
            .base_url("https://custom.api.com")
            .timeout(Duration::from_secs(60))
            .max_retries(5)
            .initial_retry_delay(Duration::from_secs(2))
            .max_retry_delay(Duration::from_secs(30))
            .build()
            .unwrap();

        assert_eq!(client.config().api_key, "test_key");
        assert_eq!(client.config().base_url, "https://custom.api.com");
        assert_eq!(client.config().timeout, Duration::from_secs(60));
        assert_eq!(client.config().retry_policy.max_retries, 5);
        assert_eq!(
            client.config().retry_policy.initial_delay,
            Duration::from_secs(2)
        );
        assert_eq!(
            client.config().retry_policy.max_delay,
            Duration::from_secs(30)
        );
    }

    #[test]
    fn test_builder_without_api_key_fails() {
        let result = ComposioClient::builder().build();

        assert!(result.is_err());
        match result {
            Err(ComposioError::InvalidInput(msg)) => {
                assert_eq!(msg, "API key is required");
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_builder_with_empty_api_key_fails() {
        let result = ComposioClient::builder().api_key("").build();

        assert!(result.is_err());
        match result {
            Err(ComposioError::InvalidInput(msg)) => {
                assert_eq!(msg, "API key cannot be empty");
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_builder_with_invalid_base_url_fails() {
        let result = ComposioClient::builder()
            .api_key("test_key")
            .base_url("invalid-url")
            .build();

        assert!(result.is_err());
        match result {
            Err(ComposioError::ConfigError(msg)) => {
                assert_eq!(msg, "Base URL must start with http:// or https://");
            }
            _ => panic!("Expected ConfigError"),
        }
    }

    #[test]
    fn test_builder_accepts_string_api_key() {
        let client = ComposioClient::builder()
            .api_key("test_key".to_string())
            .build()
            .unwrap();

        assert_eq!(client.config().api_key, "test_key");
    }

    #[test]
    fn test_builder_accepts_str_api_key() {
        let client = ComposioClient::builder()
            .api_key("test_key")
            .build()
            .unwrap();

        assert_eq!(client.config().api_key, "test_key");
    }

    #[test]
    fn test_client_is_cloneable() {
        let client = ComposioClient::builder()
            .api_key("test_key")
            .build()
            .unwrap();

        let cloned = client.clone();
        assert_eq!(client.config().api_key, cloned.config().api_key);
    }

    #[test]
    fn test_client_is_debuggable() {
        let client = ComposioClient::builder()
            .api_key("test_key")
            .build()
            .unwrap();

        let debug_str = format!("{:?}", client);
        assert!(debug_str.contains("ComposioClient"));
    }

    #[test]
    fn test_builder_is_debuggable() {
        let builder = ComposioClient::builder().api_key("test_key");

        let debug_str = format!("{:?}", builder);
        assert!(debug_str.contains("ComposioClientBuilder"));
    }

    #[test]
    fn test_http_client_has_correct_timeout() {
        let client = ComposioClient::builder()
            .api_key("test_key")
            .timeout(Duration::from_secs(45))
            .build()
            .unwrap();

        assert_eq!(client.config().timeout, Duration::from_secs(45));
    }

    #[test]
    fn test_config_accessor() {
        let client = ComposioClient::builder()
            .api_key("test_key")
            .build()
            .unwrap();

        let config = client.config();
        assert_eq!(config.api_key, "test_key");
    }

    #[test]
    fn test_http_client_accessor() {
        let client = ComposioClient::builder()
            .api_key("test_key")
            .build()
            .unwrap();

        let _http_client = client.http_client();
        // Just verify we can access it without panic
    }

    #[test]
    fn test_builder_method_chaining() {
        let client = ComposioClient::builder()
            .api_key("test_key")
            .base_url("https://test.com")
            .timeout(Duration::from_secs(60))
            .max_retries(5)
            .initial_retry_delay(Duration::from_secs(2))
            .max_retry_delay(Duration::from_secs(30))
            .build()
            .unwrap();

        assert_eq!(client.config().api_key, "test_key");
        assert_eq!(client.config().base_url, "https://test.com");
    }

    #[test]
    fn test_default_retry_policy() {
        let client = ComposioClient::builder()
            .api_key("test_key")
            .build()
            .unwrap();

        assert_eq!(client.config().retry_policy.max_retries, 3);
        assert_eq!(
            client.config().retry_policy.initial_delay,
            Duration::from_secs(1)
        );
        assert_eq!(
            client.config().retry_policy.max_delay,
            Duration::from_secs(10)
        );
    }

    #[test]
    fn test_custom_retry_policy() {
        let client = ComposioClient::builder()
            .api_key("test_key")
            .max_retries(7)
            .initial_retry_delay(Duration::from_millis(500))
            .max_retry_delay(Duration::from_secs(20))
            .build()
            .unwrap();

        assert_eq!(client.config().retry_policy.max_retries, 7);
        assert_eq!(
            client.config().retry_policy.initial_delay,
            Duration::from_millis(500)
        );
        assert_eq!(
            client.config().retry_policy.max_delay,
            Duration::from_secs(20)
        );
    }

    #[test]
    fn test_partial_retry_policy_customization() {
        let client = ComposioClient::builder()
            .api_key("test_key")
            .max_retries(5)
            .build()
            .unwrap();

        assert_eq!(client.config().retry_policy.max_retries, 5);
        assert_eq!(
            client.config().retry_policy.initial_delay,
            Duration::from_secs(1)
        );
        assert_eq!(
            client.config().retry_policy.max_delay,
            Duration::from_secs(10)
        );
    }
}
