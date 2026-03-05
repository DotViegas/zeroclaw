//! Cost tracking for Composio API usage
//!
//! This module provides cost estimation and budget tracking for Composio tool executions.
//! It records API calls per (user_id, toolkit), estimates costs based on Composio pricing tiers,
//! and warns when daily budgets are exceeded.

use chrono::{DateTime, Utc, TimeZone};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Composio pricing tier for cost estimation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PricingTier {
    /// Free tier: limited calls, no cost
    Free,
    /// Starter tier: $0.001 per call
    Starter,
    /// Professional tier: $0.0005 per call
    Professional,
    /// Enterprise tier: custom pricing (estimated at $0.0003 per call)
    Enterprise,
}

impl PricingTier {
    /// Get the cost per API call in USD
    pub fn cost_per_call(&self) -> f64 {
        match self {
            PricingTier::Free => 0.0,
            PricingTier::Starter => 0.001,
            PricingTier::Professional => 0.0005,
            PricingTier::Enterprise => 0.0003,
        }
    }
}

impl Default for PricingTier {
    fn default() -> Self {
        PricingTier::Starter
    }
}

/// Cost record for a single API call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostRecord {
    /// User identifier
    pub user_id: String,
    /// Toolkit name (e.g., "gmail", "slack")
    pub toolkit: String,
    /// Timestamp of the call
    pub timestamp: DateTime<Utc>,
    /// Estimated cost in USD
    pub estimated_cost_usd: f64,
    /// Pricing tier used for estimation
    pub pricing_tier: PricingTier,
}

/// Daily cost summary per user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyCostSummary {
    /// User identifier
    pub user_id: String,
    /// Date (YYYY-MM-DD)
    pub date: String,
    /// Total calls made today
    pub total_calls: u64,
    /// Total estimated cost in USD
    pub total_cost_usd: f64,
    /// Cost breakdown by toolkit
    pub cost_by_toolkit: HashMap<String, f64>,
    /// Daily budget limit in USD (if configured)
    pub daily_budget_usd: Option<f64>,
    /// Percentage of budget consumed (0.0 to 1.0)
    pub budget_consumed_pct: f64,
}

impl DailyCostSummary {
    /// Check if budget warning threshold (80%) is exceeded
    pub fn should_warn(&self) -> bool {
        self.budget_consumed_pct >= 0.8
    }

    /// Check if budget is fully exceeded
    pub fn is_budget_exceeded(&self) -> bool {
        self.budget_consumed_pct >= 1.0
    }

    /// Get remaining budget in USD
    pub fn remaining_budget_usd(&self) -> Option<f64> {
        self.daily_budget_usd
            .map(|budget| (budget - self.total_cost_usd).max(0.0))
    }
}

/// Cost tracker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostTrackerConfig {
    /// Enable cost tracking
    pub enabled: bool,
    /// Default pricing tier for cost estimation
    pub default_pricing_tier: PricingTier,
    /// Daily budget per user in USD (None = unlimited)
    pub daily_budget_per_user_usd: Option<f64>,
    /// Per-toolkit cost limits in USD (None = unlimited)
    pub toolkit_cost_limits_usd: HashMap<String, f64>,
    /// Budget warning threshold (0.0 to 1.0, default 0.8 = 80%)
    pub budget_warning_threshold: f64,
}

impl Default for CostTrackerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_pricing_tier: PricingTier::default(),
            daily_budget_per_user_usd: None,
            toolkit_cost_limits_usd: HashMap::new(),
            budget_warning_threshold: 0.8,
        }
    }
}

/// Cost tracker for Composio API usage
pub struct CostTracker {
    config: CostTrackerConfig,
    /// Cost records indexed by user_id
    records: Arc<RwLock<HashMap<String, Vec<CostRecord>>>>,
    /// Daily summaries indexed by (user_id, date)
    daily_summaries: Arc<RwLock<HashMap<(String, String), DailyCostSummary>>>,
}

impl CostTracker {
    /// Create a new cost tracker with the given configuration
    pub fn new(config: CostTrackerConfig) -> Self {
        Self {
            config,
            records: Arc::new(RwLock::new(HashMap::new())),
            daily_summaries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record a Composio API call and estimate its cost
    pub fn record_call(&self, user_id: impl Into<String>, toolkit: impl Into<String>) -> f64 {
        if !self.config.enabled {
            return 0.0;
        }

        let user_id = user_id.into();
        let toolkit = toolkit.into();
        let timestamp = Utc::now();
        let estimated_cost = self.config.default_pricing_tier.cost_per_call();

        let record = CostRecord {
            user_id: user_id.clone(),
            toolkit: toolkit.clone(),
            timestamp,
            estimated_cost_usd: estimated_cost,
            pricing_tier: self.config.default_pricing_tier,
        };

        // Store the record
        let mut records = self.records.write();
        records
            .entry(user_id.clone())
            .or_insert_with(Vec::new)
            .push(record);

        // Update daily summary
        self.update_daily_summary(&user_id, &toolkit, estimated_cost);

        estimated_cost
    }

    /// Update the daily cost summary for a user
    fn update_daily_summary(&self, user_id: &str, toolkit: &str, cost: f64) {
        let date = Utc::now().format("%Y-%m-%d").to_string();
        let key = (user_id.to_string(), date.clone());

        let mut summaries = self.daily_summaries.write();
        let summary = summaries.entry(key).or_insert_with(|| DailyCostSummary {
            user_id: user_id.to_string(),
            date: date.clone(),
            total_calls: 0,
            total_cost_usd: 0.0,
            cost_by_toolkit: HashMap::new(),
            daily_budget_usd: self.config.daily_budget_per_user_usd,
            budget_consumed_pct: 0.0,
        });

        summary.total_calls += 1;
        summary.total_cost_usd += cost;
        *summary
            .cost_by_toolkit
            .entry(toolkit.to_string())
            .or_insert(0.0) += cost;

        // Update budget consumption percentage
        if let Some(budget) = summary.daily_budget_usd {
            summary.budget_consumed_pct = (summary.total_cost_usd / budget).min(1.0);
        }
    }

    /// Get the daily cost summary for a user
    pub fn get_daily_summary(&self, user_id: impl Into<String>) -> Option<DailyCostSummary> {
        let user_id = user_id.into();
        let date = Utc::now().format("%Y-%m-%d").to_string();
        let key = (user_id, date);

        self.daily_summaries.read().get(&key).cloned()
    }

    /// Check if a user should receive a budget warning
    pub fn should_warn_budget(&self, user_id: impl Into<String>) -> bool {
        if !self.config.enabled {
            return false;
        }

        self.get_daily_summary(user_id)
            .map(|s| s.should_warn())
            .unwrap_or(false)
    }

    /// Check if a user has exceeded their daily budget
    pub fn is_budget_exceeded(&self, user_id: impl Into<String>) -> bool {
        if !self.config.enabled {
            return false;
        }

        self.get_daily_summary(user_id)
            .map(|s| s.is_budget_exceeded())
            .unwrap_or(false)
    }

    /// Check if a toolkit cost limit is exceeded for a user
    pub fn is_toolkit_limit_exceeded(
        &self,
        user_id: impl Into<String>,
        toolkit: impl Into<String>,
    ) -> bool {
        if !self.config.enabled {
            return false;
        }

        let toolkit = toolkit.into();
        let limit = match self.config.toolkit_cost_limits_usd.get(&toolkit) {
            Some(&limit) => limit,
            None => return false, // No limit configured
        };

        let summary = match self.get_daily_summary(user_id) {
            Some(s) => s,
            None => return false,
        };

        let toolkit_cost = summary.cost_by_toolkit.get(&toolkit).copied().unwrap_or(0.0);
        toolkit_cost >= limit
    }

    /// Estimate the cost of a future API call
    pub fn estimate_call_cost(&self) -> f64 {
        if !self.config.enabled {
            return 0.0;
        }
        self.config.default_pricing_tier.cost_per_call()
    }

    /// Get all cost records for a user
    pub fn get_user_records(&self, user_id: impl Into<String>) -> Vec<CostRecord> {
        let user_id = user_id.into();
        self.records
            .read()
            .get(&user_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Get total cost for a user across all time
    pub fn get_total_cost(&self, user_id: impl Into<String>) -> f64 {
        self.get_user_records(user_id)
            .iter()
            .map(|r| r.estimated_cost_usd)
            .sum()
    }

    /// Clear old records (older than 30 days) to prevent memory bloat
    pub fn cleanup_old_records(&self) {
        let cutoff = Utc::now() - chrono::Duration::days(30);

        let mut records = self.records.write();
        for user_records in records.values_mut() {
            user_records.retain(|r| r.timestamp > cutoff);
        }

        let mut summaries = self.daily_summaries.write();
        summaries.retain(|(_, date), _| {
            // Keep summaries from the last 30 days
            if let Ok(summary_date) = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
                if let Some(summary_datetime) = summary_date.and_hms_opt(0, 0, 0) {
                    let summary_datetime_utc = Utc.from_utc_datetime(&summary_datetime);
                    summary_datetime_utc > cutoff
                } else {
                    false
                }
            } else {
                false
            }
        });
    }

    /// Export cost metrics for observability backend
    pub fn export_metrics(&self) -> CostMetrics {
        let summaries = self.daily_summaries.read();
        let today = Utc::now().format("%Y-%m-%d").to_string();

        let mut total_calls_today = 0u64;
        let mut total_cost_today = 0.0f64;
        let mut users_over_budget = 0u64;
        let mut users_near_budget = 0u64;

        for ((_, date), summary) in summaries.iter() {
            if date == &today {
                total_calls_today += summary.total_calls;
                total_cost_today += summary.total_cost_usd;

                if summary.is_budget_exceeded() {
                    users_over_budget += 1;
                } else if summary.should_warn() {
                    users_near_budget += 1;
                }
            }
        }

        CostMetrics {
            total_calls_today,
            total_cost_today_usd: total_cost_today,
            users_over_budget,
            users_near_budget,
            cost_per_call_usd: self.config.default_pricing_tier.cost_per_call(),
        }
    }
}

/// Cost metrics for observability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostMetrics {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pricing_tier_costs() {
        assert_eq!(PricingTier::Free.cost_per_call(), 0.0);
        assert_eq!(PricingTier::Starter.cost_per_call(), 0.001);
        assert_eq!(PricingTier::Professional.cost_per_call(), 0.0005);
        assert_eq!(PricingTier::Enterprise.cost_per_call(), 0.0003);
    }

    #[test]
    fn test_cost_tracker_disabled() {
        let config = CostTrackerConfig {
            enabled: false,
            ..Default::default()
        };
        let tracker = CostTracker::new(config);

        let cost = tracker.record_call("user1", "gmail");
        assert_eq!(cost, 0.0);

        assert!(!tracker.should_warn_budget("user1"));
        assert!(!tracker.is_budget_exceeded("user1"));
    }

    #[test]
    fn test_cost_tracker_record_call() {
        let config = CostTrackerConfig {
            enabled: true,
            default_pricing_tier: PricingTier::Starter,
            daily_budget_per_user_usd: Some(1.0),
            ..Default::default()
        };
        let tracker = CostTracker::new(config);

        let cost = tracker.record_call("user1", "gmail");
        assert_eq!(cost, 0.001);

        let summary = tracker.get_daily_summary("user1").unwrap();
        assert_eq!(summary.total_calls, 1);
        assert_eq!(summary.total_cost_usd, 0.001);
        assert_eq!(summary.cost_by_toolkit.get("gmail"), Some(&0.001));
    }

    #[test]
    fn test_budget_warning_threshold() {
        let config = CostTrackerConfig {
            enabled: true,
            default_pricing_tier: PricingTier::Starter,
            daily_budget_per_user_usd: Some(1.0),
            budget_warning_threshold: 0.8,
            ..Default::default()
        };
        let tracker = CostTracker::new(config);

        // Record 800 calls = $0.80 (80% of $1.00 budget)
        for _ in 0..800 {
            tracker.record_call("user1", "gmail");
        }

        assert!(tracker.should_warn_budget("user1"));
        assert!(!tracker.is_budget_exceeded("user1"));

        let summary = tracker.get_daily_summary("user1").unwrap();
        assert_eq!(summary.budget_consumed_pct, 0.8);
    }

    #[test]
    fn test_budget_exceeded() {
        let config = CostTrackerConfig {
            enabled: true,
            default_pricing_tier: PricingTier::Starter,
            daily_budget_per_user_usd: Some(1.0),
            ..Default::default()
        };
        let tracker = CostTracker::new(config);

        // Record 1000 calls = $1.00 (100% of budget)
        for _ in 0..1000 {
            tracker.record_call("user1", "gmail");
        }

        assert!(tracker.is_budget_exceeded("user1"));

        let summary = tracker.get_daily_summary("user1").unwrap();
        assert_eq!(summary.budget_consumed_pct, 1.0);
        assert_eq!(summary.remaining_budget_usd(), Some(0.0));
    }

    #[test]
    fn test_toolkit_cost_limit() {
        let mut toolkit_limits = HashMap::new();
        toolkit_limits.insert("gmail".to_string(), 0.5);

        let config = CostTrackerConfig {
            enabled: true,
            default_pricing_tier: PricingTier::Starter,
            toolkit_cost_limits_usd: toolkit_limits,
            ..Default::default()
        };
        let tracker = CostTracker::new(config);

        // Record 500 calls to gmail = $0.50
        for _ in 0..500 {
            tracker.record_call("user1", "gmail");
        }

        assert!(tracker.is_toolkit_limit_exceeded("user1", "gmail"));
        assert!(!tracker.is_toolkit_limit_exceeded("user1", "slack"));
    }

    #[test]
    fn test_multiple_toolkits() {
        let config = CostTrackerConfig {
            enabled: true,
            default_pricing_tier: PricingTier::Starter,
            daily_budget_per_user_usd: Some(1.0),
            ..Default::default()
        };
        let tracker = CostTracker::new(config);

        tracker.record_call("user1", "gmail");
        tracker.record_call("user1", "slack");
        tracker.record_call("user1", "github");

        let summary = tracker.get_daily_summary("user1").unwrap();
        assert_eq!(summary.total_calls, 3);
        assert_eq!(summary.total_cost_usd, 0.003);
        assert_eq!(summary.cost_by_toolkit.len(), 3);
    }

    #[test]
    fn test_multiple_users() {
        let config = CostTrackerConfig {
            enabled: true,
            default_pricing_tier: PricingTier::Starter,
            ..Default::default()
        };
        let tracker = CostTracker::new(config);

        tracker.record_call("user1", "gmail");
        tracker.record_call("user2", "slack");

        let summary1 = tracker.get_daily_summary("user1").unwrap();
        let summary2 = tracker.get_daily_summary("user2").unwrap();

        assert_eq!(summary1.total_calls, 1);
        assert_eq!(summary2.total_calls, 1);
        assert_ne!(summary1.user_id, summary2.user_id);
    }

    #[test]
    fn test_estimate_call_cost() {
        let config = CostTrackerConfig {
            enabled: true,
            default_pricing_tier: PricingTier::Professional,
            ..Default::default()
        };
        let tracker = CostTracker::new(config);

        assert_eq!(tracker.estimate_call_cost(), 0.0005);
    }

    #[test]
    fn test_export_metrics() {
        let config = CostTrackerConfig {
            enabled: true,
            default_pricing_tier: PricingTier::Starter,
            daily_budget_per_user_usd: Some(1.0),
            ..Default::default()
        };
        let tracker = CostTracker::new(config);

        // User 1: 900 calls (90% of budget - over warning threshold)
        for _ in 0..900 {
            tracker.record_call("user1", "gmail");
        }

        // User 2: 1100 calls (110% of budget - exceeded)
        for _ in 0..1100 {
            tracker.record_call("user2", "slack");
        }

        let metrics = tracker.export_metrics();
        assert_eq!(metrics.total_calls_today, 2000);
        assert_eq!(metrics.total_cost_today_usd, 2.0);
        assert_eq!(metrics.users_over_budget, 1); // user2
        assert_eq!(metrics.users_near_budget, 1); // user1
    }
}
