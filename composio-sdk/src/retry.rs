use crate::error::ComposioError;
use std::time::Duration;
use tokio_retry::strategy::ExponentialBackoff;

/// Retry policy configuration for handling transient failures
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial delay before first retry
    pub initial_delay: Duration,
    /// Maximum delay between retries (caps exponential backoff)
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(10),
        }
    }
}

impl RetryPolicy {
    /// Creates an exponential backoff iterator for this policy
    pub fn strategy(&self) -> impl Iterator<Item = Duration> {
        ExponentialBackoff::from_millis(self.initial_delay.as_millis() as u64)
            .max_delay(self.max_delay)
            .take(self.max_retries as usize)
    }
}

/// Execute an async operation with retry logic
pub async fn with_retry<F, Fut, T>(
    policy: &RetryPolicy,
    operation: F,
) -> Result<T, ComposioError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, ComposioError>>,
{
    let mut last_error = None;
    
    for delay in std::iter::once(Duration::ZERO).chain(policy.strategy()) {
        if delay > Duration::ZERO {
            tokio::time::sleep(delay).await;
        }
        
        match operation().await {
            Ok(value) => return Ok(value),
            Err(e) if should_retry(&e) => {
                last_error = Some(e);
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    
    Err(last_error.unwrap())
}

/// Check if an error should be retried
pub fn should_retry(error: &ComposioError) -> bool {
    error.is_retryable()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_retry_policy() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_retries, 3);
        assert_eq!(policy.initial_delay, Duration::from_secs(1));
        assert_eq!(policy.max_delay, Duration::from_secs(10));
    }

    #[test]
    fn test_custom_retry_policy() {
        let policy = RetryPolicy {
            max_retries: 5,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
        };

        assert_eq!(policy.max_retries, 5);
        assert_eq!(policy.initial_delay, Duration::from_millis(500));
        assert_eq!(policy.max_delay, Duration::from_secs(30));
    }

    #[test]
    fn test_strategy_yields_correct_number_of_delays() {
        let policy = RetryPolicy {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
        };

        let delays: Vec<_> = policy.strategy().collect();
        assert_eq!(delays.len(), 3);
    }

    #[test]
    fn test_strategy_respects_max_delay() {
        let policy = RetryPolicy {
            max_retries: 10,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(5),
        };

        let delays: Vec<_> = policy.strategy().collect();
        
        for delay in delays {
            assert!(delay <= policy.max_delay);
        }
    }

    #[test]
    fn test_should_retry_for_rate_limit() {
        let error = ComposioError::ApiError {
            status: 429,
            message: "Rate limited".to_string(),
            code: None,
            slug: None,
            request_id: None,
            suggested_fix: None,
            errors: None,
        };

        assert!(should_retry(&error));
    }

    #[test]
    fn test_should_retry_for_server_errors() {
        for status in [500, 502, 503, 504] {
            let error = ComposioError::ApiError {
                status,
                message: "Server error".to_string(),
                code: None,
                slug: None,
                request_id: None,
                suggested_fix: None,
                errors: None,
            };

            assert!(
                should_retry(&error),
                "Status {} should be retryable",
                status
            );
        }
    }

    #[test]
    fn test_should_not_retry_for_client_errors() {
        for status in [400, 401, 403, 404] {
            let error = ComposioError::ApiError {
                status,
                message: "Client error".to_string(),
                code: None,
                slug: None,
                request_id: None,
                suggested_fix: None,
                errors: None,
            };

            assert!(
                !should_retry(&error),
                "Status {} should not be retryable",
                status
            );
        }
    }

    #[test]
    fn test_should_not_retry_for_serialization_error() {
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json")
            .unwrap_err();
        let error: ComposioError = json_error.into();

        assert!(!should_retry(&error));
    }

    #[test]
    fn test_should_not_retry_for_invalid_input() {
        let error = ComposioError::InvalidInput("Invalid API key".to_string());
        assert!(!should_retry(&error));
    }

    #[test]
    fn test_should_not_retry_for_config_error() {
        let error = ComposioError::ConfigError("Invalid base URL".to_string());
        assert!(!should_retry(&error));
    }

    #[tokio::test]
    async fn test_with_retry_succeeds_on_first_attempt() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};
        
        let policy = RetryPolicy::default();
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = with_retry(&policy, move || {
            let count = call_count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Ok::<_, ComposioError>("success")
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_with_retry_succeeds_after_retries() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};
        
        let policy = RetryPolicy {
            max_retries: 3,
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(50),
        };
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = with_retry(&policy, move || {
            let count = call_count_clone.clone();
            async move {
                let current = count.fetch_add(1, Ordering::SeqCst) + 1;
                if current < 3 {
                    Err(ComposioError::ApiError {
                        status: 503,
                        message: "Service unavailable".to_string(),
                        code: None,
                        slug: None,
                        request_id: None,
                        suggested_fix: None,
                        errors: None,
                    })
                } else {
                    Ok::<_, ComposioError>("success")
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_with_retry_fails_after_max_retries() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};
        
        let policy = RetryPolicy {
            max_retries: 2,
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(50),
        };
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = with_retry(&policy, move || {
            let count = call_count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Err::<String, _>(ComposioError::ApiError {
                    status: 503,
                    message: "Service unavailable".to_string(),
                    code: None,
                    slug: None,
                    request_id: None,
                    suggested_fix: None,
                    errors: None,
                })
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_with_retry_does_not_retry_non_retryable_errors() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};
        
        let policy = RetryPolicy::default();
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = with_retry(&policy, move || {
            let count = call_count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Err::<String, _>(ComposioError::ApiError {
                    status: 404,
                    message: "Not found".to_string(),
                    code: None,
                    slug: None,
                    request_id: None,
                    suggested_fix: None,
                    errors: None,
                })
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }
}
