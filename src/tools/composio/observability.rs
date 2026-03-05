//! Structured logging and observability for Composio tool executions
//!
//! This module provides JSON-structured logging for all Composio tool operations,
//! ensuring no sensitive data (API keys, OAuth tokens) is logged while maintaining
//! comprehensive audit trails for debugging and monitoring.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tracing::{debug, error, info, trace, warn};

/// Log level for structured logging
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    /// Check if this log level should be emitted based on configured filter
    pub fn should_log(&self, filter: LogLevel) -> bool {
        use LogLevel::*;
        let self_priority = match self {
            Error => 0,
            Warn => 1,
            Info => 2,
            Debug => 3,
            Trace => 4,
        };
        let filter_priority = match filter {
            Error => 0,
            Warn => 1,
            Info => 2,
            Debug => 3,
            Trace => 4,
        };
        self_priority <= filter_priority
    }
}

/// Structured log entry for Composio tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLog {
    /// ISO 8601 timestamp
    pub timestamp: DateTime<Utc>,
    /// Log level
    pub level: LogLevel,
    /// Component identifier (e.g., "composio.meta_tools", "composio.workbench")
    pub component: String,
    /// Event type (e.g., "tool_execution", "oauth_connection", "cache_hit")
    pub event: String,
    /// User identifier (v3 terminology)
    pub user_id: String,
    /// Tool name (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    /// Execution duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Success status
    pub success: bool,
    /// Additional context (sanitized, no secrets)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Value>,
}

impl ExecutionLog {
    /// Create a new execution log entry
    pub fn new(
        level: LogLevel,
        component: impl Into<String>,
        event: impl Into<String>,
        user_id: impl Into<String>,
        success: bool,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            level,
            component: component.into(),
            event: event.into(),
            user_id: user_id.into(),
            tool_name: None,
            duration_ms: None,
            success,
            context: None,
        }
    }

    /// Set tool name
    pub fn with_tool_name(mut self, tool_name: impl Into<String>) -> Self {
        self.tool_name = Some(tool_name.into());
        self
    }

    /// Set execution duration
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration_ms = Some(duration.as_millis() as u64);
        self
    }

    /// Set additional context (will be sanitized)
    pub fn with_context(mut self, context: Value) -> Self {
        self.context = Some(sanitize_context(context));
        self
    }

    /// Emit the log entry using tracing
    pub fn emit(&self, filter: LogLevel) {
        if !self.level.should_log(filter) {
            return;
        }

        let json = serde_json::to_string(self).unwrap_or_else(|e| {
            format!(r#"{{"error":"failed to serialize log","reason":"{}"}}"#, e)
        });

        match self.level {
            LogLevel::Error => error!("{}", json),
            LogLevel::Warn => warn!("{}", json),
            LogLevel::Info => info!("{}", json),
            LogLevel::Debug => debug!("{}", json),
            LogLevel::Trace => trace!("{}", json),
        }
    }
}

/// Sanitize context to remove sensitive data
///
/// This function removes or redacts:
/// - API keys (any field containing "api_key", "apikey", "key")
/// - OAuth tokens (any field containing "token", "access_token", "refresh_token")
/// - Credentials (any field containing "password", "secret", "credential")
/// - Authorization headers
fn sanitize_context(mut context: Value) -> Value {
    if let Some(obj) = context.as_object_mut() {
        let sensitive_keys = [
            "api_key",
            "apikey",
            "key",
            "token",
            "access_token",
            "refresh_token",
            "password",
            "secret",
            "credential",
            "authorization",
            "auth",
            "bearer",
            "oauth",
        ];

        for (key, value) in obj.iter_mut() {
            let key_lower = key.to_lowercase();
            if sensitive_keys
                .iter()
                .any(|sensitive| key_lower.contains(sensitive))
            {
                *value = Value::String("[REDACTED]".to_string());
            } else if value.is_object() || value.is_array() {
                *value = sanitize_context(value.clone());
            }
        }
    } else if let Some(arr) = context.as_array_mut() {
        for item in arr.iter_mut() {
            *item = sanitize_context(item.clone());
        }
    }

    context
}

/// Metrics for Composio tool operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposioMetrics {
    /// Availability metrics
    pub composio_api_availability: f64,
    pub mcp_server_availability: f64,
    
    /// Performance metrics
    pub request_latency_p50_ms: f64,
    pub request_latency_p95_ms: f64,
    pub request_latency_p99_ms: f64,
    pub execution_duration_p50_ms: f64,
    pub execution_duration_p95_ms: f64,
    pub execution_duration_p99_ms: f64,
    pub cache_hit_rate: f64,
    
    /// Usage metrics
    pub tool_executions_total: u64,
    pub oauth_connections_total: u64,
    pub workbench_executions_total: u64,
    
    /// Error metrics
    pub composio_errors_total: u64,
    pub auth_failures_total: u64,
    pub rate_limit_hits_total: u64,
    
    /// Cost metrics (optional, only if cost tracking is enabled)
    pub cost_metrics: Option<CostMetricsSnapshot>,
}

/// Cost metrics snapshot for observability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostMetricsSnapshot {
    /// Total API calls today across all users
    pub total_calls_today: u64,
    /// Total estimated cost today in USD
    pub total_cost_today_usd: f64,
    /// Number of users who exceeded their daily budget
    pub users_over_budget: u64,
    /// Number of users near budget warning threshold (>80%)
    pub users_near_budget: u64,
    /// Cost per API call in USD
    pub cost_per_call_usd: f64,
}

impl Default for ComposioMetrics {
    fn default() -> Self {
        Self {
            composio_api_availability: 1.0,
            mcp_server_availability: 1.0,
            request_latency_p50_ms: 0.0,
            request_latency_p95_ms: 0.0,
            request_latency_p99_ms: 0.0,
            execution_duration_p50_ms: 0.0,
            execution_duration_p95_ms: 0.0,
            execution_duration_p99_ms: 0.0,
            cache_hit_rate: 0.0,
            tool_executions_total: 0,
            oauth_connections_total: 0,
            workbench_executions_total: 0,
            composio_errors_total: 0,
            auth_failures_total: 0,
            rate_limit_hits_total: 0,
            cost_metrics: None,
        }
    }
}

/// Metric collector for tracking latencies and computing percentiles
#[derive(Debug)]
struct LatencyCollector {
    samples: parking_lot::Mutex<Vec<u64>>,
    max_samples: usize,
}

impl LatencyCollector {
    fn new(max_samples: usize) -> Self {
        Self {
            samples: parking_lot::Mutex::new(Vec::with_capacity(max_samples)),
            max_samples,
        }
    }
    
    fn record(&self, latency_ms: u64) {
        let mut samples = self.samples.lock();
        samples.push(latency_ms);
        
        // Keep only the most recent samples
        if samples.len() > self.max_samples {
            let excess = samples.len() - self.max_samples;
            samples.drain(0..excess);
        }
    }
    
    fn percentile(&self, p: f64) -> f64 {
        let mut samples = self.samples.lock();
        if samples.is_empty() {
            return 0.0;
        }
        
        samples.sort_unstable();
        let index = ((samples.len() as f64 * p).ceil() as usize).saturating_sub(1);
        samples.get(index).copied().unwrap_or(0) as f64
    }
}

/// Logger for Composio tool operations
pub struct ComposioLogger {
    filter: LogLevel,
    
    // Availability tracking
    composio_api_requests: parking_lot::Mutex<u64>,
    composio_api_failures: parking_lot::Mutex<u64>,
    mcp_server_requests: parking_lot::Mutex<u64>,
    mcp_server_failures: parking_lot::Mutex<u64>,
    
    // Latency tracking
    request_latencies: LatencyCollector,
    execution_durations: LatencyCollector,
    
    // Cache tracking
    cache_hits: parking_lot::Mutex<u64>,
    cache_misses: parking_lot::Mutex<u64>,
    
    // Usage tracking
    tool_executions: parking_lot::Mutex<u64>,
    oauth_connections: parking_lot::Mutex<u64>,
    workbench_executions: parking_lot::Mutex<u64>,
    
    // Error tracking
    composio_errors: parking_lot::Mutex<u64>,
    auth_failures: parking_lot::Mutex<u64>,
    rate_limit_hits: parking_lot::Mutex<u64>,
}

impl ComposioLogger {
    /// Create a new logger with the specified filter level
    pub fn new(filter: LogLevel) -> Self {
        Self {
            filter,
            composio_api_requests: parking_lot::Mutex::new(0),
            composio_api_failures: parking_lot::Mutex::new(0),
            mcp_server_requests: parking_lot::Mutex::new(0),
            mcp_server_failures: parking_lot::Mutex::new(0),
            request_latencies: LatencyCollector::new(1000),
            execution_durations: LatencyCollector::new(1000),
            cache_hits: parking_lot::Mutex::new(0),
            cache_misses: parking_lot::Mutex::new(0),
            tool_executions: parking_lot::Mutex::new(0),
            oauth_connections: parking_lot::Mutex::new(0),
            workbench_executions: parking_lot::Mutex::new(0),
            composio_errors: parking_lot::Mutex::new(0),
            auth_failures: parking_lot::Mutex::new(0),
            rate_limit_hits: parking_lot::Mutex::new(0),
        }
    }
    
    /// Record a Composio API request
    pub fn record_composio_api_request(&self, success: bool, latency_ms: u64) {
        *self.composio_api_requests.lock() += 1;
        if !success {
            *self.composio_api_failures.lock() += 1;
        }
        self.request_latencies.record(latency_ms);
    }
    
    /// Record an MCP server request
    pub fn record_mcp_server_request(&self, success: bool, latency_ms: u64) {
        *self.mcp_server_requests.lock() += 1;
        if !success {
            *self.mcp_server_failures.lock() += 1;
        }
        self.request_latencies.record(latency_ms);
    }
    
    /// Get current metrics snapshot
    pub fn get_metrics(&self) -> ComposioMetrics {
        let composio_requests = *self.composio_api_requests.lock();
        let composio_failures = *self.composio_api_failures.lock();
        let mcp_requests = *self.mcp_server_requests.lock();
        let mcp_failures = *self.mcp_server_failures.lock();
        
        let composio_api_availability = if composio_requests > 0 {
            1.0 - (composio_failures as f64 / composio_requests as f64)
        } else {
            1.0
        };
        
        let mcp_server_availability = if mcp_requests > 0 {
            1.0 - (mcp_failures as f64 / mcp_requests as f64)
        } else {
            1.0
        };
        
        let cache_hits = *self.cache_hits.lock();
        let cache_misses = *self.cache_misses.lock();
        let cache_total = cache_hits + cache_misses;
        let cache_hit_rate = if cache_total > 0 {
            cache_hits as f64 / cache_total as f64
        } else {
            0.0
        };
        
        ComposioMetrics {
            composio_api_availability,
            mcp_server_availability,
            request_latency_p50_ms: self.request_latencies.percentile(0.50),
            request_latency_p95_ms: self.request_latencies.percentile(0.95),
            request_latency_p99_ms: self.request_latencies.percentile(0.99),
            execution_duration_p50_ms: self.execution_durations.percentile(0.50),
            execution_duration_p95_ms: self.execution_durations.percentile(0.95),
            execution_duration_p99_ms: self.execution_durations.percentile(0.99),
            cache_hit_rate,
            tool_executions_total: *self.tool_executions.lock(),
            oauth_connections_total: *self.oauth_connections.lock(),
            workbench_executions_total: *self.workbench_executions.lock(),
            composio_errors_total: *self.composio_errors.lock(),
            auth_failures_total: *self.auth_failures.lock(),
            rate_limit_hits_total: *self.rate_limit_hits.lock(),
            cost_metrics: None, // Cost metrics not tracked in this collector
        }
    }

    /// Log a tool execution
    pub fn log_tool_execution(
        &self,
        user_id: impl Into<String>,
        tool_name: impl Into<String>,
        duration: Duration,
        success: bool,
        context: Option<Value>,
    ) {
        // Record metrics
        *self.tool_executions.lock() += 1;
        self.execution_durations.record(duration.as_millis() as u64);
        if !success {
            *self.composio_errors.lock() += 1;
        }
        
        let mut log = ExecutionLog::new(
            if success {
                LogLevel::Info
            } else {
                LogLevel::Error
            },
            "composio.tool_execution",
            "tool_executed",
            user_id,
            success,
        )
        .with_tool_name(tool_name)
        .with_duration(duration);

        if let Some(ctx) = context {
            log = log.with_context(ctx);
        }

        log.emit(self.filter);
    }

    /// Log an OAuth connection attempt
    pub fn log_oauth_connection(
        &self,
        user_id: impl Into<String>,
        toolkit: impl Into<String>,
        success: bool,
        context: Option<Value>,
    ) {
        // Record metrics
        *self.oauth_connections.lock() += 1;
        if !success {
            *self.auth_failures.lock() += 1;
        }
        
        let mut log = ExecutionLog::new(
            if success {
                LogLevel::Info
            } else {
                LogLevel::Warn
            },
            "composio.oauth",
            "oauth_connection",
            user_id,
            success,
        )
        .with_tool_name(toolkit);

        if let Some(ctx) = context {
            log = log.with_context(ctx);
        }

        log.emit(self.filter);
    }

    /// Log a cache operation
    pub fn log_cache_operation(
        &self,
        user_id: impl Into<String>,
        cache_type: impl Into<String>,
        hit: bool,
    ) {
        // Record metrics
        if hit {
            *self.cache_hits.lock() += 1;
        } else {
            *self.cache_misses.lock() += 1;
        }
        
        let log = ExecutionLog::new(
            LogLevel::Debug,
            "composio.cache",
            if hit { "cache_hit" } else { "cache_miss" },
            user_id,
            true,
        )
        .with_context(serde_json::json!({
            "cache_type": cache_type.into(),
        }));

        log.emit(self.filter);
    }

    /// Log a Workbench operation
    pub fn log_workbench_operation(
        &self,
        user_id: impl Into<String>,
        session_id: impl Into<String>,
        duration: Duration,
        success: bool,
        context: Option<Value>,
    ) {
        // Record metrics
        *self.workbench_executions.lock() += 1;
        self.execution_durations.record(duration.as_millis() as u64);
        if !success {
            *self.composio_errors.lock() += 1;
        }
        
        let mut log = ExecutionLog::new(
            if success {
                LogLevel::Info
            } else {
                LogLevel::Error
            },
            "composio.workbench",
            "workbench_execution",
            user_id,
            success,
        )
        .with_duration(duration)
        .with_context(serde_json::json!({
            "session_id": session_id.into(),
        }));

        if let Some(ctx) = context {
            if let Some(obj) = log.context.as_mut().and_then(|v| v.as_object_mut()) {
                if let Some(ctx_obj) = ctx.as_object() {
                    for (k, v) in ctx_obj {
                        obj.insert(k.clone(), sanitize_context(v.clone()));
                    }
                }
            }
        }

        log.emit(self.filter);
    }

    /// Log an MCP client operation
    pub fn log_mcp_operation(
        &self,
        user_id: impl Into<String>,
        operation: impl Into<String>,
        duration: Duration,
        success: bool,
        context: Option<Value>,
    ) {
        let mut log = ExecutionLog::new(
            if success {
                LogLevel::Debug
            } else {
                LogLevel::Error
            },
            "composio.mcp_client",
            operation,
            user_id,
            success,
        )
        .with_duration(duration);

        if let Some(ctx) = context {
            log = log.with_context(ctx);
        }

        log.emit(self.filter);
    }

    /// Log a security policy check
    pub fn log_security_check(
        &self,
        user_id: impl Into<String>,
        tool_name: impl Into<String>,
        allowed: bool,
        reason: Option<String>,
    ) {
        let mut context = serde_json::json!({
            "tool_name": tool_name.into(),
        });

        if let Some(r) = reason {
            context["reason"] = Value::String(r);
        }

        let log = ExecutionLog::new(
            if allowed {
                LogLevel::Debug
            } else {
                LogLevel::Warn
            },
            "composio.security",
            if allowed {
                "access_granted"
            } else {
                "access_denied"
            },
            user_id,
            allowed,
        )
        .with_context(context);

        log.emit(self.filter);
    }

    /// Log an error
    pub fn log_error(
        &self,
        user_id: impl Into<String>,
        component: impl Into<String>,
        error: impl Into<String>,
        context: Option<Value>,
    ) {
        // Record metrics
        *self.composio_errors.lock() += 1;
        
        let mut log = ExecutionLog::new(
            LogLevel::Error,
            component,
            "error",
            user_id,
            false,
        )
        .with_context(serde_json::json!({
            "error": error.into(),
        }));

        if let Some(ctx) = context {
            if let Some(obj) = log.context.as_mut().and_then(|v| v.as_object_mut()) {
                if let Some(ctx_obj) = ctx.as_object() {
                    for (k, v) in ctx_obj {
                        obj.insert(k.clone(), sanitize_context(v.clone()));
                    }
                }
            }
        }

        log.emit(self.filter);
    }
    
    /// Record a rate limit hit
    pub fn record_rate_limit_hit(&self) {
        *self.rate_limit_hits.lock() += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_filtering() {
        assert!(LogLevel::Error.should_log(LogLevel::Error));
        assert!(LogLevel::Error.should_log(LogLevel::Info));
        assert!(LogLevel::Error.should_log(LogLevel::Trace));

        assert!(!LogLevel::Trace.should_log(LogLevel::Error));
        assert!(!LogLevel::Debug.should_log(LogLevel::Info));
        assert!(LogLevel::Info.should_log(LogLevel::Info));
    }

    #[test]
    fn test_sanitize_context_removes_api_keys() {
        let context = serde_json::json!({
            "api_key": "secret123",
            "user_id": "user_1",
            "data": "safe_data"
        });

        let sanitized = sanitize_context(context);

        assert_eq!(sanitized["api_key"], "[REDACTED]");
        assert_eq!(sanitized["user_id"], "user_1");
        assert_eq!(sanitized["data"], "safe_data");
    }

    #[test]
    fn test_sanitize_context_removes_tokens() {
        let context = serde_json::json!({
            "access_token": "token123",
            "refresh_token": "refresh456",
            "bearer": "bearer789",
            "safe_field": "safe_value"
        });

        let sanitized = sanitize_context(context);

        assert_eq!(sanitized["access_token"], "[REDACTED]");
        assert_eq!(sanitized["refresh_token"], "[REDACTED]");
        assert_eq!(sanitized["bearer"], "[REDACTED]");
        assert_eq!(sanitized["safe_field"], "safe_value");
    }

    #[test]
    fn test_sanitize_context_nested_objects() {
        let context = serde_json::json!({
            "outer": {
                "api_key": "secret",
                "inner": {
                    "token": "token123",
                    "safe": "value"
                }
            },
            "safe_top": "safe"
        });

        let sanitized = sanitize_context(context);

        assert_eq!(sanitized["outer"]["api_key"], "[REDACTED]");
        assert_eq!(sanitized["outer"]["inner"]["token"], "[REDACTED]");
        assert_eq!(sanitized["outer"]["inner"]["safe"], "value");
        assert_eq!(sanitized["safe_top"], "safe");
    }

    #[test]
    fn test_sanitize_context_arrays() {
        let context = serde_json::json!([
            {"api_key": "secret1", "data": "safe1"},
            {"token": "secret2", "data": "safe2"}
        ]);

        let sanitized = sanitize_context(context);

        assert_eq!(sanitized[0]["api_key"], "[REDACTED]");
        assert_eq!(sanitized[0]["data"], "safe1");
        assert_eq!(sanitized[1]["token"], "[REDACTED]");
        assert_eq!(sanitized[1]["data"], "safe2");
    }

    #[test]
    fn test_execution_log_builder() {
        let log = ExecutionLog::new(
            LogLevel::Info,
            "composio.test",
            "test_event",
            "user_123",
            true,
        )
        .with_tool_name("test_tool")
        .with_duration(Duration::from_millis(150))
        .with_context(serde_json::json!({"key": "value"}));

        assert_eq!(log.level, LogLevel::Info);
        assert_eq!(log.component, "composio.test");
        assert_eq!(log.event, "test_event");
        assert_eq!(log.user_id, "user_123");
        assert_eq!(log.tool_name, Some("test_tool".to_string()));
        assert_eq!(log.duration_ms, Some(150));
        assert!(log.success);
        assert!(log.context.is_some());
    }

    #[test]
    fn test_composio_logger_tool_execution() {
        let logger = ComposioLogger::new(LogLevel::Info);

        // This should not panic
        logger.log_tool_execution(
            "user_1",
            "gmail_send_email",
            Duration::from_millis(200),
            true,
            Some(serde_json::json!({"recipient": "test@example.com"})),
        );
    }

    #[test]
    fn test_composio_logger_oauth_connection() {
        let logger = ComposioLogger::new(LogLevel::Info);

        logger.log_oauth_connection(
            "user_1",
            "gmail",
            true,
            Some(serde_json::json!({"mode": "cli_auto_open"})),
        );
    }

    #[test]
    fn test_composio_logger_cache_operation() {
        let logger = ComposioLogger::new(LogLevel::Debug);

        logger.log_cache_operation("user_1", "tool_schema", true);
        logger.log_cache_operation("user_1", "connection", false);
    }

    #[test]
    fn test_composio_logger_security_check() {
        let logger = ComposioLogger::new(LogLevel::Debug);

        logger.log_security_check("user_1", "gmail_send_email", true, None);
        logger.log_security_check(
            "user_1",
            "blocked_tool",
            false,
            Some("toolkit not in allowlist".to_string()),
        );
    }

    #[test]
    fn test_composio_logger_error() {
        let logger = ComposioLogger::new(LogLevel::Error);

        logger.log_error(
            "user_1",
            "composio.mcp_client",
            "connection timeout",
            Some(serde_json::json!({"url": "https://mcp.example.com"})),
        );
    }
}
