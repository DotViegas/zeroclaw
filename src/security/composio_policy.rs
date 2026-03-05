//! Composio-specific security policy enforcement.
//!
//! This module extends the core [`SecurityPolicy`] with Composio-specific
//! access control, including toolkit allowlists/denylists, rate limiting,
//! and security event logging for Composio tool executions.
//!
//! # Integration
//!
//! The [`ComposioSecurityPolicy`] wraps the core security policy and adds
//! Composio-specific validation before tool execution. It enforces:
//!
//! - ToolOperation::Act permission checks (via core SecurityPolicy)
//! - Toolkit allowlist/denylist filtering
//! - Per-user rate limiting
//! - Security event logging for denied requests
//!
//! # Usage
//!
//! ```rust,ignore
//! use zeroclaw::security::{SecurityPolicy, ComposioSecurityPolicy};
//! use std::sync::Arc;
//!
//! let core_policy = Arc::new(SecurityPolicy::default());
//! let composio_policy = ComposioSecurityPolicy::new(
//!     core_policy,
//!     Some(vec!["gmail".to_string(), "slack".to_string()]),
//!     None,
//!     60,
//! );
//!
//! // Check if toolkit execution is allowed
//! match composio_policy.check_toolkit_execution("gmail", "user123") {
//!     Ok(()) => println!("Execution allowed"),
//!     Err(e) => println!("Execution denied: {}", e),
//! }
//! ```

use crate::security::{AuditEvent, AuditEventType, AuditLogger, SecurityPolicy};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Rate limiter using sliding window algorithm.
#[derive(Debug)]
struct RateLimiter {
    /// Timestamps of recent requests per user_id
    requests: RwLock<HashMap<String, Vec<Instant>>>,
    /// Maximum requests per minute
    max_requests_per_minute: u32,
    /// Window duration (1 minute)
    window: Duration,
}

impl RateLimiter {
    fn new(max_requests_per_minute: u32) -> Self {
        Self {
            requests: RwLock::new(HashMap::new()),
            max_requests_per_minute,
            window: Duration::from_secs(60),
        }
    }

    /// Check if a request is allowed for the given user_id.
    /// Returns Ok(()) if allowed, Err with retry-after seconds if rate limited.
    fn check_and_record(&self, user_id: &str) -> Result<(), u64> {
        let mut requests = self.requests.write();
        let now = Instant::now();
        let cutoff = now.checked_sub(self.window).unwrap_or(now);

        // Get or create user's request history
        let user_requests = requests.entry(user_id.to_string()).or_insert_with(Vec::new);

        // Remove expired requests
        user_requests.retain(|&timestamp| timestamp > cutoff);

        // Check rate limit
        if user_requests.len() >= self.max_requests_per_minute as usize {
            // Calculate retry-after based on oldest request in window
            let oldest = user_requests.first().copied().unwrap_or(now);
            let retry_after = oldest
                .checked_add(self.window)
                .and_then(|t| t.checked_duration_since(now))
                .map(|d| d.as_secs())
                .unwrap_or(60);

            return Err(retry_after);
        }

        // Record this request
        user_requests.push(now);
        Ok(())
    }

    /// Get current request count for a user (for metrics/debugging)
    fn count(&self, user_id: &str) -> usize {
        let mut requests = self.requests.write();
        let now = Instant::now();
        let cutoff = now.checked_sub(self.window).unwrap_or(now);

        if let Some(user_requests) = requests.get_mut(user_id) {
            user_requests.retain(|&timestamp| timestamp > cutoff);
            user_requests.len()
        } else {
            0
        }
    }
}

/// Composio-specific security policy configuration.
#[derive(Debug, Clone)]
pub struct ComposioSecurityConfig {
    /// Allowed toolkits (if Some, only these toolkits are permitted)
    pub allowed_toolkits: Option<Vec<String>>,
    /// Denied toolkits (these toolkits are always blocked)
    pub denied_toolkits: Option<Vec<String>>,
    /// Maximum Composio API calls per minute per user
    pub rate_limit_per_minute: u32,
}

impl Default for ComposioSecurityConfig {
    fn default() -> Self {
        Self {
            allowed_toolkits: None,
            denied_toolkits: None,
            rate_limit_per_minute: 60,
        }
    }
}

/// Composio security policy enforcer.
///
/// Wraps the core [`SecurityPolicy`] and adds Composio-specific validation:
/// - Toolkit allowlist/denylist filtering
/// - Per-user rate limiting
/// - Security event logging
pub struct ComposioSecurityPolicy {
    /// Core security policy (for ToolOperation::Act checks)
    core_policy: Arc<SecurityPolicy>,
    /// Composio-specific configuration
    config: ComposioSecurityConfig,
    /// Rate limiter per user_id
    rate_limiter: RateLimiter,
    /// Audit logger for security events (optional, requires config and path)
    audit_logger: Option<Arc<AuditLogger>>,
}

impl ComposioSecurityPolicy {
    /// Create a new Composio security policy.
    pub fn new(
        core_policy: Arc<SecurityPolicy>,
        allowed_toolkits: Option<Vec<String>>,
        denied_toolkits: Option<Vec<String>>,
        rate_limit_per_minute: u32,
    ) -> Self {
        Self {
            core_policy,
            config: ComposioSecurityConfig {
                allowed_toolkits,
                denied_toolkits,
                rate_limit_per_minute,
            },
            rate_limiter: RateLimiter::new(rate_limit_per_minute),
            audit_logger: None,
        }
    }

    /// Create from a ComposioSecurityConfig.
    pub fn from_config(core_policy: Arc<SecurityPolicy>, config: ComposioSecurityConfig) -> Self {
        Self {
            core_policy,
            rate_limiter: RateLimiter::new(config.rate_limit_per_minute),
            config,
            audit_logger: None,
        }
    }
    
    /// Set the audit logger for security event logging.
    pub fn with_audit_logger(mut self, audit_logger: Arc<AuditLogger>) -> Self {
        self.audit_logger = Some(audit_logger);
        self
    }

    /// Check if a toolkit execution is allowed.
    ///
    /// This performs the following checks in order:
    /// 1. Core security policy ToolOperation::Act permission
    /// 2. Toolkit denylist check
    /// 3. Toolkit allowlist check (if configured)
    /// 4. Rate limiting check
    ///
    /// Returns Ok(()) if all checks pass, Err with reason if denied.
    pub fn check_toolkit_execution(
        &self,
        toolkit: &str,
        user_id: &str,
    ) -> Result<(), ComposioSecurityError> {
        // 1. Check core security policy for ToolOperation::Act permission
        // Note: The core policy doesn't have a direct "check_tool_operation" method,
        // but we can use the autonomy level as a proxy. ReadOnly autonomy blocks all actions.
        if self.core_policy.autonomy == crate::security::AutonomyLevel::ReadOnly {
            self.log_security_event(
                user_id,
                toolkit,
                "permission_denied",
                "Autonomy level is ReadOnly",
            );
            return Err(ComposioSecurityError::PermissionDenied {
                toolkit: toolkit.to_string(),
                reason: "Autonomy level is ReadOnly - all tool executions are blocked".to_string(),
            });
        }

        // 2. Check denylist
        if let Some(ref denied) = self.config.denied_toolkits {
            let normalized_toolkit = normalize_toolkit_slug(toolkit);
            if denied
                .iter()
                .any(|d| normalize_toolkit_slug(d) == normalized_toolkit)
            {
                self.log_security_event(
                    user_id,
                    toolkit,
                    "toolkit_denied",
                    "Toolkit is in denylist",
                );
                return Err(ComposioSecurityError::ToolkitDenied {
                    toolkit: toolkit.to_string(),
                    reason: "Toolkit is in the security policy denylist".to_string(),
                });
            }
        }

        // 3. Check allowlist (if configured)
        if let Some(ref allowed) = self.config.allowed_toolkits {
            let normalized_toolkit = normalize_toolkit_slug(toolkit);
            if !allowed
                .iter()
                .any(|a| normalize_toolkit_slug(a) == normalized_toolkit)
            {
                self.log_security_event(
                    user_id,
                    toolkit,
                    "toolkit_not_allowed",
                    "Toolkit is not in allowlist",
                );
                return Err(ComposioSecurityError::ToolkitNotAllowed {
                    toolkit: toolkit.to_string(),
                    reason: "Toolkit is not in the security policy allowlist".to_string(),
                });
            }
        }

        // 4. Check rate limit
        if let Err(retry_after) = self.rate_limiter.check_and_record(user_id) {
            self.log_security_event(
                user_id,
                toolkit,
                "rate_limit_exceeded",
                &format!("Rate limit exceeded, retry after {}s", retry_after),
            );
            return Err(ComposioSecurityError::RateLimitExceeded {
                user_id: user_id.to_string(),
                retry_after,
            });
        }

        // All checks passed
        self.log_security_event(user_id, toolkit, "execution_allowed", "All checks passed");
        Ok(())
    }

    /// Get the current request count for a user (for metrics/debugging).
    pub fn get_request_count(&self, user_id: &str) -> usize {
        self.rate_limiter.count(user_id)
    }

    /// Get the rate limit configuration.
    pub fn get_rate_limit(&self) -> u32 {
        self.config.rate_limit_per_minute
    }

    /// Get the core security policy.
    pub fn core_policy(&self) -> &Arc<SecurityPolicy> {
        &self.core_policy
    }

    /// Log a security event.
    fn log_security_event(&self, user_id: &str, toolkit: &str, event_type: &str, details: &str) {
        // Only log if audit logger is configured
        if let Some(ref logger) = self.audit_logger {
            let event = AuditEvent::new(AuditEventType::SecurityEvent)
                .with_actor("composio".to_string(), Some(user_id.to_string()), None)
                .with_action(
                    format!("toolkit:{} event:{}", toolkit, event_type),
                    "medium".to_string(),
                    false,
                    event_type == "execution_allowed",
                )
                .with_result(
                    event_type == "execution_allowed",
                    None,
                    0,
                    if event_type != "execution_allowed" {
                        Some(details.to_string())
                    } else {
                        None
                    },
                );

            // Log the event (ignore errors since logging shouldn't fail the operation)
            let _ = logger.log(&event);
        }
    }
}

/// Normalize toolkit slug to lowercase alphanumeric for consistent matching.
///
/// This implements Property 7: Toolkit Slug Normalization from the design spec.
fn normalize_toolkit_slug(slug: &str) -> String {
    slug.to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect()
}

/// Errors that can occur during Composio security policy enforcement.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ComposioSecurityError {
    #[error("Permission denied for toolkit '{toolkit}': {reason}")]
    PermissionDenied { toolkit: String, reason: String },

    #[error("Toolkit '{toolkit}' is denied by security policy: {reason}")]
    ToolkitDenied { toolkit: String, reason: String },

    #[error("Toolkit '{toolkit}' is not in the allowlist: {reason}")]
    ToolkitNotAllowed { toolkit: String, reason: String },

    #[error("Rate limit exceeded for user '{user_id}'. Retry after {retry_after} seconds")]
    RateLimitExceeded { user_id: String, retry_after: u64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_policy() -> ComposioSecurityPolicy {
        let core_policy = Arc::new(SecurityPolicy::default());
        ComposioSecurityPolicy::new(core_policy, None, None, 60)
    }

    #[test]
    fn test_normalize_toolkit_slug() {
        assert_eq!(normalize_toolkit_slug("Gmail"), "gmail");
        assert_eq!(normalize_toolkit_slug("SLACK"), "slack");
        assert_eq!(normalize_toolkit_slug("GitHub"), "github");
        assert_eq!(normalize_toolkit_slug("google-drive"), "google-drive");
        assert_eq!(normalize_toolkit_slug("my_toolkit"), "my_toolkit");
        assert_eq!(normalize_toolkit_slug("Tool@123!"), "tool123");
    }

    #[test]
    fn test_toolkit_denylist() {
        let core_policy = Arc::new(SecurityPolicy::default());
        let policy = ComposioSecurityPolicy::new(
            core_policy,
            None,
            Some(vec!["gmail".to_string(), "slack".to_string()]),
            60,
        );

        // Denied toolkit
        assert!(policy
            .check_toolkit_execution("gmail", "user1")
            .is_err());
        assert!(policy
            .check_toolkit_execution("GMAIL", "user1")
            .is_err()); // Case insensitive

        // Allowed toolkit
        assert!(policy
            .check_toolkit_execution("github", "user1")
            .is_ok());
    }

    #[test]
    fn test_toolkit_allowlist() {
        let core_policy = Arc::new(SecurityPolicy::default());
        let policy = ComposioSecurityPolicy::new(
            core_policy,
            Some(vec!["gmail".to_string(), "slack".to_string()]),
            None,
            60,
        );

        // Allowed toolkit
        assert!(policy
            .check_toolkit_execution("gmail", "user1")
            .is_ok());
        assert!(policy
            .check_toolkit_execution("SLACK", "user1")
            .is_ok()); // Case insensitive

        // Not in allowlist
        assert!(policy
            .check_toolkit_execution("github", "user1")
            .is_err());
    }

    #[test]
    fn test_rate_limiting() {
        let core_policy = Arc::new(SecurityPolicy::default());
        let policy = ComposioSecurityPolicy::new(core_policy, None, None, 3); // 3 requests per minute

        // First 3 requests should succeed
        assert!(policy
            .check_toolkit_execution("gmail", "user1")
            .is_ok());
        assert!(policy
            .check_toolkit_execution("gmail", "user1")
            .is_ok());
        assert!(policy
            .check_toolkit_execution("gmail", "user1")
            .is_ok());

        // 4th request should fail
        let result = policy.check_toolkit_execution("gmail", "user1");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ComposioSecurityError::RateLimitExceeded { .. }
        ));
    }

    #[test]
    fn test_rate_limiting_per_user() {
        let core_policy = Arc::new(SecurityPolicy::default());
        let policy = ComposioSecurityPolicy::new(core_policy, None, None, 2);

        // User1 uses their quota
        assert!(policy
            .check_toolkit_execution("gmail", "user1")
            .is_ok());
        assert!(policy
            .check_toolkit_execution("gmail", "user1")
            .is_ok());
        assert!(policy
            .check_toolkit_execution("gmail", "user1")
            .is_err());

        // User2 should still have their quota
        assert!(policy
            .check_toolkit_execution("gmail", "user2")
            .is_ok());
        assert!(policy
            .check_toolkit_execution("gmail", "user2")
            .is_ok());
    }

    #[test]
    fn test_readonly_autonomy_blocks_execution() {
        let mut core_policy = SecurityPolicy::default();
        core_policy.autonomy = crate::security::AutonomyLevel::ReadOnly;
        let policy = ComposioSecurityPolicy::new(Arc::new(core_policy), None, None, 60);

        let result = policy.check_toolkit_execution("gmail", "user1");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ComposioSecurityError::PermissionDenied { .. }
        ));
    }

    #[test]
    fn test_get_request_count() {
        let core_policy = Arc::new(SecurityPolicy::default());
        let policy = ComposioSecurityPolicy::new(core_policy, None, None, 10);

        assert_eq!(policy.get_request_count("user1"), 0);

        policy.check_toolkit_execution("gmail", "user1").ok();
        assert_eq!(policy.get_request_count("user1"), 1);

        policy.check_toolkit_execution("slack", "user1").ok();
        assert_eq!(policy.get_request_count("user1"), 2);
    }
}
