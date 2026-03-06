//! Meta Tools Module
//!
//! This module provides native Rust implementations of Composio meta tools,
//! eliminating the need for Python dependencies for most operations.
//!
//! # Meta Tools
//!
//! - **Search**: Discover relevant tools across 1000+ apps
//! - **MultiExecutor**: Execute up to 20 tools in parallel
//! - **Connections**: Manage OAuth and API key authentication
//! - **Bash**: Execute bash commands in isolated environment
//! - **Workbench**: Python sandbox for bulk operations (hybrid: Rust wrapper + remote Python)
//!
//! # Example
//!
//! ```no_run
//! use composio_sdk::{ComposioClient, meta_tools::ToolSearch};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = ComposioClient::builder()
//!         .api_key(std::env::var("COMPOSIO_API_KEY")?)
//!         .build()?;
//!
//!     let session = client.create_session("user_123")
//!         .toolkits(vec!["github", "gmail"])
//!         .send()
//!         .await?;
//!
//!     // Search for tools
//!     let search = ToolSearch::new(client.clone());
//!     let tools = search.search("send email", &session.session_id()).await?;
//!
//!     println!("Found {} tools", tools.len());
//!     Ok(())
//! }
//! ```

pub mod search;
pub mod multi_executor;
pub mod connections;
pub mod bash;
pub mod workbench;

pub use search::ToolSearch;
pub use multi_executor::MultiExecutor;
pub use connections::ConnectionManager;
pub use bash::BashExecutor;
pub use workbench::{WorkbenchExecutor, PandasOperation, WorkbenchResult};
