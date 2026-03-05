//! Composio integration modules
//!
//! This module contains the implementation of Composio v3 permanent integration,
//! including meta tools handlers, caching, OAuth management, and parameter extraction.

pub mod bash;
pub mod cache;
pub mod code_generation;
pub mod cost_tracker;
pub mod meta_tools;
pub mod observability;
pub mod onboarding;
pub mod resilience;
pub mod workbench;

// Re-export commonly used types
// Note: Some types are exported for use in tests even if not used in main codebase
#[allow(unused_imports)]
pub use observability::{ComposioLogger, ExecutionLog, LogLevel};
#[allow(unused_imports)]
pub use workbench::{WorkbenchHandler, WorkbenchState};
