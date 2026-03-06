//! Configuration for Composio SDK

use crate::error::ComposioError;
use crate::retry::RetryPolicy;
use std::time::Duration;

/// Configuration for Composio client
#[derive(Debug, Clone)]
pub struct ComposioConfig {
    /// API key for authenticating with the Composio API
    pub api_key: String,
    /// Base URL for the Composio API (default: https://backend.composio.dev/api/v3)
    pub base_url: String,
    /// Timeout duration for HTTP requests (default: 30 seconds)
    pub timeout: Duration,
    /// Retry policy for handling transient failures
    pub retry_policy: RetryPolicy,
}

impl ComposioConfig {
    /// Create a new configuration with the given API key
    /// 
    /// # Arguments
    /// 
    /// * `api_key` - The Composio API key
    /// 
    /// # Defaults
    /// 
    /// * `base_url`: <https://backend.composio.dev/api/v3>
    /// * `timeout`: 30 seconds
    /// * `retry_policy`: 3 retries, 1s initial delay, 10s max delay
    /// 
    /// # Example
    /// 
    /// ```
    /// use composio_sdk::config::ComposioConfig;
    /// 
    /// let config = ComposioConfig::new("my_api_key");
    /// assert_eq!(config.api_key, "my_api_key");
    /// assert_eq!(config.base_url, "https://backend.composio.dev/api/v3");
    /// ```
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://backend.composio.dev/api/v3".to_string(),
            timeout: Duration::from_secs(30),
            retry_policy: RetryPolicy::default(),
        }
    }

    /// Validate the configuration
    /// 
    /// # Errors
    /// 
    /// Returns an error if:
    /// * API key is empty
    /// * Base URL doesn't start with http:// or https://
    /// 
    /// # Example
    /// 
    /// ```
    /// use composio_sdk::config::ComposioConfig;
    /// 
    /// let config = ComposioConfig::new("my_api_key");
    /// assert!(config.validate().is_ok());
    /// 
    /// let invalid_config = ComposioConfig::new("");
    /// assert!(invalid_config.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), ComposioError> {
        if self.api_key.is_empty() {
            return Err(ComposioError::InvalidInput(
                "API key cannot be empty".to_string(),
            ));
        }

        if !self.base_url.starts_with("http") {
            return Err(ComposioError::ConfigError(
                "Base URL must start with http:// or https://".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_config_with_defaults() {
        let config = ComposioConfig::new("test_api_key");
        
        assert_eq!(config.api_key, "test_api_key");
        assert_eq!(config.base_url, "https://backend.composio.dev/api/v3");
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.retry_policy.max_retries, 3);
        assert_eq!(config.retry_policy.initial_delay, Duration::from_secs(1));
        assert_eq!(config.retry_policy.max_delay, Duration::from_secs(10));
    }

    #[test]
    fn test_new_config_accepts_string() {
        let config = ComposioConfig::new("test_key".to_string());
        assert_eq!(config.api_key, "test_key");
    }

    #[test]
    fn test_new_config_accepts_str() {
        let config = ComposioConfig::new("test_key");
        assert_eq!(config.api_key, "test_key");
    }

    #[test]
    fn test_validate_valid_config() {
        let config = ComposioConfig::new("valid_api_key");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_api_key() {
        let config = ComposioConfig::new("");
        let result = config.validate();
        
        assert!(result.is_err());
        match result {
            Err(ComposioError::InvalidInput(msg)) => {
                assert_eq!(msg, "API key cannot be empty");
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_validate_invalid_base_url() {
        let mut config = ComposioConfig::new("test_key");
        config.base_url = "invalid-url".to_string();
        
        let result = config.validate();
        assert!(result.is_err());
        match result {
            Err(ComposioError::ConfigError(msg)) => {
                assert_eq!(msg, "Base URL must start with http:// or https://");
            }
            _ => panic!("Expected ConfigError"),
        }
    }

    #[test]
    fn test_validate_http_base_url() {
        let mut config = ComposioConfig::new("test_key");
        config.base_url = "http://localhost:8080".to_string();
        
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_https_base_url() {
        let mut config = ComposioConfig::new("test_key");
        config.base_url = "https://api.example.com".to_string();
        
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_is_cloneable() {
        let config = ComposioConfig::new("test_key");
        let cloned = config.clone();
        
        assert_eq!(config.api_key, cloned.api_key);
        assert_eq!(config.base_url, cloned.base_url);
        assert_eq!(config.timeout, cloned.timeout);
    }

    #[test]
    fn test_config_is_debuggable() {
        let config = ComposioConfig::new("test_key");
        let debug_str = format!("{:?}", config);
        
        assert!(debug_str.contains("ComposioConfig"));
        assert!(debug_str.contains("test_key"));
    }

    #[test]
    fn test_default_base_url() {
        let config = ComposioConfig::new("key");
        assert_eq!(config.base_url, "https://backend.composio.dev/api/v3");
    }

    #[test]
    fn test_default_timeout() {
        let config = ComposioConfig::new("key");
        assert_eq!(config.timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_default_retry_policy() {
        let config = ComposioConfig::new("key");
        assert_eq!(config.retry_policy.max_retries, 3);
        assert_eq!(config.retry_policy.initial_delay, Duration::from_secs(1));
        assert_eq!(config.retry_policy.max_delay, Duration::from_secs(10));
    }

    #[test]
    fn test_custom_base_url() {
        let mut config = ComposioConfig::new("key");
        config.base_url = "https://custom.api.com".to_string();
        
        assert!(config.validate().is_ok());
        assert_eq!(config.base_url, "https://custom.api.com");
    }

    #[test]
    fn test_custom_timeout() {
        let mut config = ComposioConfig::new("key");
        config.timeout = Duration::from_secs(60);
        
        assert_eq!(config.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_custom_retry_policy() {
        let mut config = ComposioConfig::new("key");
        config.retry_policy = RetryPolicy {
            max_retries: 5,
            initial_delay: Duration::from_secs(2),
            max_delay: Duration::from_secs(20),
        };
        
        assert_eq!(config.retry_policy.max_retries, 5);
        assert_eq!(config.retry_policy.initial_delay, Duration::from_secs(2));
        assert_eq!(config.retry_policy.max_delay, Duration::from_secs(20));
    }
}
