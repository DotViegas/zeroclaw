//! Circuit breaker and retry logic for Composio integration resilience.
//!
//! Provides circuit breaker pattern to prevent cascading failures when
//! external services (MCP server, Composio API, OAuth) are experiencing issues.
//! Also provides retry logic with exponential backoff for transient failures.

use anyhow::{bail, Result};
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Circuit breaker state machine states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed, requests flow normally
    Closed,
    /// Circuit is open, requests are rejected immediately
    Open,
    /// Circuit is half-open, testing if service has recovered
    HalfOpen,
}

/// Circuit breaker configuration for different service types
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening circuit
    pub failure_threshold: u32,
    /// Number of consecutive successes in HalfOpen to close circuit
    pub success_threshold: u32,
    /// Duration to wait before transitioning from Open to HalfOpen
    pub timeout_duration: Duration,
}

impl CircuitBreakerConfig {
    /// Configuration for MCP server circuit breaker
    pub fn mcp() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout_duration: Duration::seconds(60),
        }
    }

    /// Configuration for Composio API circuit breaker
    pub fn composio_api() -> Self {
        Self {
            failure_threshold: 10,
            success_threshold: 3,
            timeout_duration: Duration::seconds(60),
        }
    }

    /// Configuration for OAuth/auth circuit breaker
    pub fn auth() -> Self {
        Self {
            failure_threshold: 3,
            success_threshold: 3,
            timeout_duration: Duration::seconds(60),
        }
    }
}

/// Circuit breaker implementation
#[derive(Debug)]
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<RwLock<CircuitBreakerState>>,
}

#[derive(Debug)]
struct CircuitBreakerState {
    current_state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<DateTime<Utc>>,
    state_changed_at: DateTime<Utc>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given configuration
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(CircuitBreakerState {
                current_state: CircuitState::Closed,
                failure_count: 0,
                success_count: 0,
                last_failure_time: None,
                state_changed_at: Utc::now(),
            })),
        }
    }

    /// Create a circuit breaker for MCP server
    pub fn for_mcp() -> Self {
        Self::new(CircuitBreakerConfig::mcp())
    }

    /// Create a circuit breaker for Composio API
    pub fn for_composio_api() -> Self {
        Self::new(CircuitBreakerConfig::composio_api())
    }

    /// Create a circuit breaker for OAuth/auth
    pub fn for_auth() -> Self {
        Self::new(CircuitBreakerConfig::auth())
    }

    /// Get the current state of the circuit breaker
    pub async fn state(&self) -> CircuitState {
        let state = self.state.read().await;
        state.current_state
    }

    /// Check if a request can proceed through the circuit breaker
    pub async fn can_proceed(&self) -> Result<()> {
        let mut state = self.state.write().await;

        match state.current_state {
            CircuitState::Closed => Ok(()),
            CircuitState::Open => {
                // Check if timeout has elapsed to transition to HalfOpen
                let now = Utc::now();
                let elapsed = now - state.state_changed_at;

                if elapsed >= self.config.timeout_duration {
                    // Transition to HalfOpen
                    state.current_state = CircuitState::HalfOpen;
                    state.success_count = 0;
                    state.state_changed_at = now;
                    Ok(())
                } else {
                    bail!(
                        "Circuit breaker is open. Service unavailable. Retry after {} seconds.",
                        (self.config.timeout_duration - elapsed).num_seconds()
                    )
                }
            }
            CircuitState::HalfOpen => Ok(()),
        }
    }

    /// Record a successful operation
    pub async fn record_success(&self) {
        let mut state = self.state.write().await;

        match state.current_state {
            CircuitState::Closed => {
                // Reset failure count on success
                state.failure_count = 0;
            }
            CircuitState::HalfOpen => {
                state.success_count += 1;

                // Check if we've reached success threshold to close circuit
                if state.success_count >= self.config.success_threshold {
                    state.current_state = CircuitState::Closed;
                    state.failure_count = 0;
                    state.success_count = 0;
                    state.state_changed_at = Utc::now();
                }
            }
            CircuitState::Open => {
                // Should not happen, but reset if it does
                state.current_state = CircuitState::Closed;
                state.failure_count = 0;
                state.success_count = 0;
                state.state_changed_at = Utc::now();
            }
        }
    }

    /// Record a failed operation
    pub async fn record_failure(&self) {
        let mut state = self.state.write().await;
        let now = Utc::now();

        state.last_failure_time = Some(now);

        match state.current_state {
            CircuitState::Closed => {
                state.failure_count += 1;

                // Check if we've reached failure threshold to open circuit
                if state.failure_count >= self.config.failure_threshold {
                    state.current_state = CircuitState::Open;
                    state.success_count = 0;
                    state.state_changed_at = now;
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in HalfOpen immediately opens the circuit
                state.current_state = CircuitState::Open;
                state.failure_count = self.config.failure_threshold;
                state.success_count = 0;
                state.state_changed_at = now;
            }
            CircuitState::Open => {
                // Already open, just update timestamp
                state.state_changed_at = now;
            }
        }
    }

    /// Execute a function with circuit breaker protection
    pub async fn execute<F, T>(&self, f: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        // Check if we can proceed
        self.can_proceed().await?;

        // Execute the function
        match f.await {
            Ok(result) => {
                self.record_success().await;
                Ok(result)
            }
            Err(err) => {
                self.record_failure().await;
                Err(err)
            }
        }
    }

    /// Get circuit breaker statistics
    pub async fn stats(&self) -> CircuitBreakerStats {
        let state = self.state.read().await;
        CircuitBreakerStats {
            current_state: state.current_state,
            failure_count: state.failure_count,
            success_count: state.success_count,
            last_failure_time: state.last_failure_time,
            state_changed_at: state.state_changed_at,
        }
    }

    /// Reset the circuit breaker to closed state
    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        state.current_state = CircuitState::Closed;
        state.failure_count = 0;
        state.success_count = 0;
        state.last_failure_time = None;
        state.state_changed_at = Utc::now();
    }
}

/// Circuit breaker statistics
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    pub current_state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_failure_time: Option<DateTime<Utc>>,
    pub state_changed_at: DateTime<Utc>,
}

/// Retry configuration with exponential backoff
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay in milliseconds before first retry
    pub initial_delay_ms: u64,
    /// Maximum delay in milliseconds between retries
    pub max_delay_ms: u64,
    /// Backoff multiplier for exponential backoff
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Create a new retry configuration
    pub fn new(
        max_attempts: u32,
        initial_delay_ms: u64,
        max_delay_ms: u64,
        backoff_multiplier: f64,
    ) -> Self {
        Self {
            max_attempts,
            initial_delay_ms,
            max_delay_ms,
            backoff_multiplier,
        }
    }

    /// Calculate delay for a given attempt with exponential backoff and jitter
    fn calculate_delay(&self, attempt: u32) -> std::time::Duration {
        let base_delay = self.initial_delay_ms as f64
            * self.backoff_multiplier.powi(attempt as i32);
        let capped_delay = base_delay.min(self.max_delay_ms as f64);
        
        // Add jitter (±25% randomization) to avoid thundering herd
        // Use a simple hash-based jitter instead of random for determinism in tests
        let jitter_seed = (attempt as u64).wrapping_mul(0x9e3779b97f4a7c15);
        let jitter_factor = 0.75 + (jitter_seed % 50) as f64 / 100.0;
        let final_delay = (capped_delay * jitter_factor) as u64;
        
        std::time::Duration::from_millis(final_delay)
    }

    /// Execute a function with retry logic
    pub async fn execute<F, Fut, T, E>(&self, mut f: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display + std::fmt::Debug,
    {
        let mut last_error = None;

        for attempt in 0..self.max_attempts {
            match f().await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    last_error = Some(format!("{}", err));
                    
                    // Don't sleep after the last attempt
                    if attempt < self.max_attempts - 1 {
                        let delay = self.calculate_delay(attempt);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        bail!(
            "Operation failed after {} attempts. Last error: {}",
            self.max_attempts,
            last_error.unwrap_or_else(|| "unknown error".to_string())
        )
    }

    /// Check if an error is retryable
    pub fn is_retryable_error(error: &anyhow::Error) -> bool {
        let error_str = error.to_string().to_lowercase();
        
        // Retry on connection errors
        if error_str.contains("connection") 
            || error_str.contains("timeout")
            || error_str.contains("timed out") {
            return true;
        }

        // Retry on transient 5xx errors
        if error_str.contains("500")
            || error_str.contains("502")
            || error_str.contains("503")
            || error_str.contains("504") {
            return true;
        }

        // Retry on rate limit (429)
        if error_str.contains("429") || error_str.contains("rate limit") {
            return true;
        }

        // Don't retry on 4xx client errors (except 429)
        if error_str.contains("400")
            || error_str.contains("401")
            || error_str.contains("403")
            || error_str.contains("404") {
            return false;
        }

        // Default to not retrying for unknown errors
        false
    }

    /// Execute with retry only for retryable errors
    pub async fn execute_with_filter<F, Fut, T>(&self, mut f: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut last_error = None;

        for attempt in 0..self.max_attempts {
            match f().await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    // Check if error is retryable
                    if !Self::is_retryable_error(&err) {
                        return Err(err);
                    }

                    last_error = Some(err);
                    
                    // Don't sleep after the last attempt
                    if attempt < self.max_attempts - 1 {
                        let delay = self.calculate_delay(attempt);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        // Graceful error handling: last_error should always be Some here, but handle None case
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Operation failed after {} attempts with no error recorded", self.max_attempts)))
    }
}

/// Helper functions for retry logic integration

/// Retry wrapper for MCP connection failures
pub async fn retry_mcp_connection<F, Fut, T>(f: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let config = RetryConfig::new(3, 1000, 10000, 2.0);
    config.execute_with_filter(f).await
}

/// Retry wrapper for Composio API calls
pub async fn retry_composio_api<F, Fut, T>(f: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let config = RetryConfig::new(3, 1000, 30000, 2.0);
    config.execute_with_filter(f).await
}

/// Retry wrapper for transient errors (5xx, timeouts, connection issues)
pub async fn retry_transient<F, Fut, T>(f: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let config = RetryConfig::default();
    config.execute_with_filter(f).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_initial_state() {
        let cb = CircuitBreaker::for_mcp();
        assert_eq!(cb.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_closed_to_open() {
        let cb = CircuitBreaker::for_mcp();

        // Record failures up to threshold (5 for MCP)
        for _ in 0..4 {
            cb.record_failure().await;
            assert_eq!(cb.state().await, CircuitState::Closed);
        }

        // Fifth failure should open the circuit
        cb.record_failure().await;
        assert_eq!(cb.state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_open_rejects_requests() {
        let cb = CircuitBreaker::for_mcp();

        // Open the circuit
        for _ in 0..5 {
            cb.record_failure().await;
        }

        // Requests should be rejected
        assert!(cb.can_proceed().await.is_err());
    }

    #[tokio::test]
    async fn test_circuit_breaker_open_to_halfopen() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout_duration: Duration::milliseconds(100),
        };
        let cb = CircuitBreaker::new(config);

        // Open the circuit
        for _ in 0..3 {
            cb.record_failure().await;
        }
        assert_eq!(cb.state().await, CircuitState::Open);

        // Wait for timeout
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        // Should transition to HalfOpen
        assert!(cb.can_proceed().await.is_ok());
        assert_eq!(cb.state().await, CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn test_circuit_breaker_halfopen_to_closed() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 3,
            timeout_duration: Duration::milliseconds(100),
        };
        let cb = CircuitBreaker::new(config);

        // Open the circuit
        for _ in 0..3 {
            cb.record_failure().await;
        }

        // Wait for timeout to transition to HalfOpen
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        let _ = cb.can_proceed().await;

        // Record successes up to threshold
        for _ in 0..2 {
            cb.record_success().await;
            assert_eq!(cb.state().await, CircuitState::HalfOpen);
        }

        // Third success should close the circuit
        cb.record_success().await;
        assert_eq!(cb.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_halfopen_to_open_on_failure() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 3,
            timeout_duration: Duration::milliseconds(100),
        };
        let cb = CircuitBreaker::new(config);

        // Open the circuit
        for _ in 0..3 {
            cb.record_failure().await;
        }

        // Wait for timeout to transition to HalfOpen
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        let _ = cb.can_proceed().await;
        assert_eq!(cb.state().await, CircuitState::HalfOpen);

        // Any failure in HalfOpen should immediately open the circuit
        cb.record_failure().await;
        assert_eq!(cb.state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_success_resets_failure_count() {
        let cb = CircuitBreaker::for_mcp();

        // Record some failures
        for _ in 0..3 {
            cb.record_failure().await;
        }

        // Record a success
        cb.record_success().await;

        // Should still be closed and failure count reset
        assert_eq!(cb.state().await, CircuitState::Closed);

        // Should take 5 more failures to open
        for _ in 0..4 {
            cb.record_failure().await;
            assert_eq!(cb.state().await, CircuitState::Closed);
        }

        cb.record_failure().await;
        assert_eq!(cb.state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_execute_success() {
        let cb = CircuitBreaker::for_mcp();

        let result = cb
            .execute(async { Ok::<i32, anyhow::Error>(42) })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(cb.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_execute_failure() {
        let cb = CircuitBreaker::for_mcp();

        for _ in 0..5 {
            let _ = cb
                .execute(async { Err::<i32, anyhow::Error>(anyhow::anyhow!("test error")) })
                .await;
        }

        assert_eq!(cb.state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_reset() {
        let cb = CircuitBreaker::for_mcp();

        // Open the circuit
        for _ in 0..5 {
            cb.record_failure().await;
        }
        assert_eq!(cb.state().await, CircuitState::Open);

        // Reset
        cb.reset().await;
        assert_eq!(cb.state().await, CircuitState::Closed);

        let stats = cb.stats().await;
        assert_eq!(stats.failure_count, 0);
        assert_eq!(stats.success_count, 0);
    }

    #[tokio::test]
    async fn test_circuit_breaker_different_thresholds() {
        // Test MCP config (5 failures)
        let mcp_cb = CircuitBreaker::for_mcp();
        for _ in 0..5 {
            mcp_cb.record_failure().await;
        }
        assert_eq!(mcp_cb.state().await, CircuitState::Open);

        // Test Composio API config (10 failures)
        let api_cb = CircuitBreaker::for_composio_api();
        for _ in 0..9 {
            api_cb.record_failure().await;
            assert_eq!(api_cb.state().await, CircuitState::Closed);
        }
        api_cb.record_failure().await;
        assert_eq!(api_cb.state().await, CircuitState::Open);

        // Test Auth config (3 failures)
        let auth_cb = CircuitBreaker::for_auth();
        for _ in 0..3 {
            auth_cb.record_failure().await;
        }
        assert_eq!(auth_cb.state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_stats() {
        let cb = CircuitBreaker::for_mcp();

        // Record some failures
        cb.record_failure().await;
        cb.record_failure().await;

        let stats = cb.stats().await;
        assert_eq!(stats.current_state, CircuitState::Closed);
        assert_eq!(stats.failure_count, 2);
        assert_eq!(stats.success_count, 0);
        assert!(stats.last_failure_time.is_some());
    }

    // Retry logic tests

    #[tokio::test]
    async fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_delay_ms, 1000);
        assert_eq!(config.max_delay_ms, 30000);
        assert_eq!(config.backoff_multiplier, 2.0);
    }

    #[tokio::test]
    async fn test_retry_config_custom() {
        let config = RetryConfig::new(5, 500, 10000, 1.5);
        assert_eq!(config.max_attempts, 5);
        assert_eq!(config.initial_delay_ms, 500);
        assert_eq!(config.max_delay_ms, 10000);
        assert_eq!(config.backoff_multiplier, 1.5);
    }

    #[tokio::test]
    async fn test_retry_success_on_first_attempt() {
        let config = RetryConfig::default();
        let call_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = config
            .execute(move || {
                let count = call_count_clone.clone();
                async move {
                    count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok::<i32, anyhow::Error>(42)
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let config = RetryConfig::new(3, 10, 100, 2.0);
        let call_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = config
            .execute(move || {
                let count = call_count_clone.clone();
                async move {
                    let current = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    if current < 3 {
                        Err(anyhow::anyhow!("temporary failure"))
                    } else {
                        Ok::<i32, anyhow::Error>(42)
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_exhausts_attempts() {
        let config = RetryConfig::new(3, 10, 100, 2.0);
        let call_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = config
            .execute(move || {
                let count = call_count_clone.clone();
                async move {
                    count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Err::<i32, anyhow::Error>(anyhow::anyhow!("persistent failure"))
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 3);
        assert!(result.unwrap_err().to_string().contains("after 3 attempts"));
    }

    #[tokio::test]
    async fn test_retry_exponential_backoff() {
        let config = RetryConfig::new(3, 100, 10000, 2.0);

        // Test delay calculation (without jitter for predictability)
        // Attempt 0: 100ms * 2^0 = 100ms
        // Attempt 1: 100ms * 2^1 = 200ms
        // Attempt 2: 100ms * 2^2 = 400ms

        let delay0 = config.calculate_delay(0);
        let delay1 = config.calculate_delay(1);
        let delay2 = config.calculate_delay(2);

        // With jitter (±25%), delays should be in expected ranges
        assert!(delay0.as_millis() >= 75 && delay0.as_millis() <= 125);
        assert!(delay1.as_millis() >= 150 && delay1.as_millis() <= 250);
        assert!(delay2.as_millis() >= 300 && delay2.as_millis() <= 500);
    }

    #[tokio::test]
    async fn test_retry_max_delay_cap() {
        let config = RetryConfig::new(10, 1000, 5000, 2.0);

        // After several attempts, delay should be capped at max_delay_ms
        let delay_high = config.calculate_delay(10);

        // With jitter (±25%), should not exceed max_delay * 1.25
        assert!(delay_high.as_millis() <= 6250);
    }

    #[tokio::test]
    async fn test_is_retryable_error_connection() {
        let err = anyhow::anyhow!("Connection refused");
        assert!(RetryConfig::is_retryable_error(&err));

        let err = anyhow::anyhow!("Connection timeout");
        assert!(RetryConfig::is_retryable_error(&err));

        let err = anyhow::anyhow!("Request timed out");
        assert!(RetryConfig::is_retryable_error(&err));
    }

    #[tokio::test]
    async fn test_is_retryable_error_5xx() {
        let err = anyhow::anyhow!("HTTP 500 Internal Server Error");
        assert!(RetryConfig::is_retryable_error(&err));

        let err = anyhow::anyhow!("HTTP 502 Bad Gateway");
        assert!(RetryConfig::is_retryable_error(&err));

        let err = anyhow::anyhow!("HTTP 503 Service Unavailable");
        assert!(RetryConfig::is_retryable_error(&err));

        let err = anyhow::anyhow!("HTTP 504 Gateway Timeout");
        assert!(RetryConfig::is_retryable_error(&err));
    }

    #[tokio::test]
    async fn test_is_retryable_error_rate_limit() {
        let err = anyhow::anyhow!("HTTP 429 Too Many Requests");
        assert!(RetryConfig::is_retryable_error(&err));

        let err = anyhow::anyhow!("Rate limit exceeded");
        assert!(RetryConfig::is_retryable_error(&err));
    }

    #[tokio::test]
    async fn test_is_not_retryable_error_4xx() {
        let err = anyhow::anyhow!("HTTP 400 Bad Request");
        assert!(!RetryConfig::is_retryable_error(&err));

        let err = anyhow::anyhow!("HTTP 401 Unauthorized");
        assert!(!RetryConfig::is_retryable_error(&err));

        let err = anyhow::anyhow!("HTTP 403 Forbidden");
        assert!(!RetryConfig::is_retryable_error(&err));

        let err = anyhow::anyhow!("HTTP 404 Not Found");
        assert!(!RetryConfig::is_retryable_error(&err));
    }

    #[tokio::test]
    async fn test_execute_with_filter_retries_retryable() {
        let config = RetryConfig::new(3, 10, 100, 2.0);
        let call_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = config
            .execute_with_filter(move || {
                let count = call_count_clone.clone();
                async move {
                    let current = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    if current < 3 {
                        Err(anyhow::anyhow!("Connection timeout"))
                    } else {
                        Ok::<i32, anyhow::Error>(42)
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_execute_with_filter_skips_non_retryable() {
        let config = RetryConfig::new(3, 10, 100, 2.0);
        let call_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = config
            .execute_with_filter(move || {
                let count = call_count_clone.clone();
                async move {
                    count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Err::<i32, anyhow::Error>(anyhow::anyhow!("HTTP 401 Unauthorized"))
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1); // Should not retry
        assert!(result.unwrap_err().to_string().contains("401"));
    }
}
