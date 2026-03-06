//! Meta Tools Usage Example
//!
//! This example demonstrates how to use the native Rust meta tools
//! for search, multi-execution, connections, bash, and workbench operations.

use composio_sdk::meta_tools::{
    BashExecutor, ConnectionManager, ExcelOperation, MultiExecutor, PandasOperation, ToolCall,
    ToolSearch, WorkbenchExecutor,
};
use composio_sdk::ComposioClient;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client
    let api_key = std::env::var("COMPOSIO_API_KEY")
        .expect("COMPOSIO_API_KEY environment variable not set");

    let client = Arc::new(ComposioClient::builder().api_key(api_key).build()?);

    // Create a session
    let session = client
        .create_session("user_123")
        .toolkits(vec!["github", "gmail", "slack"])
        .manage_connections(true)
        .send()
        .await?;

    println!("✓ Session created: {}", session.session_id());
    println!();

    // ========================================================================
    // 1. Tool Search
    // ========================================================================
    println!("=== Tool Search ===");
    let search = ToolSearch::new(client.clone());
    let results = search
        .search("send email to user", session.session_id())
        .await?;

    println!("Found {} tools:", results.len());
    for result in results.iter().take(3) {
        println!(
            "  - {} ({}): {} [score: {:.2}]",
            result.slug, result.toolkit, result.name, result.score
        );
    }
    println!();

    // ========================================================================
    // 2. Multi-Tool Execution
    // ========================================================================
    println!("=== Multi-Tool Execution ===");
    let multi_exec = MultiExecutor::new(client.clone());

    let tools = vec![
        ToolCall {
            tool_slug: "GITHUB_GET_REPOS".to_string(),
            arguments: serde_json::json!({ "owner": "composio" }),
            connected_account_id: None,
        },
        ToolCall {
            tool_slug: "GITHUB_GET_ISSUES".to_string(),
            arguments: serde_json::json!({
                "owner": "composio",
                "repo": "composio"
            }),
            connected_account_id: None,
        },
    ];

    let result = multi_exec
        .execute_parallel(session.session_id(), tools)
        .await?;

    println!(
        "Executed {} tools in {}ms",
        result.successful + result.failed,
        result.total_time_ms
    );
    println!("  Successful: {}", result.successful);
    println!("  Failed: {}", result.failed);
    println!();

    // ========================================================================
    // 3. Connection Management
    // ========================================================================
    println!("=== Connection Management ===");
    let conn_manager = ConnectionManager::new(client.clone());

    // List connections
    let connections = conn_manager.list_connections(session.session_id()).await?;
    println!("Connected accounts: {}", connections.len());
    for conn in connections.iter().take(3) {
        println!("  - {}: {:?}", conn.toolkit, conn.status);
    }

    // Check if GitHub is connected
    let is_github_connected = conn_manager
        .is_connected(session.session_id(), "github")
        .await?;
    println!("GitHub connected: {}", is_github_connected);

    // Create auth link if not connected
    if !is_github_connected {
        let auth_link = conn_manager
            .create_auth_link(
                session.session_id(),
                "github",
                Some("https://myapp.com/callback"),
            )
            .await?;
        println!("Auth link: {}", auth_link.redirect_url);
    }
    println!();

    // ========================================================================
    // 4. Bash Executor
    // ========================================================================
    println!("=== Bash Executor ===");
    let bash = BashExecutor::new()
        .timeout(10)
        .env("MY_VAR", "test_value");

    let bash_result = bash.execute("echo 'Hello from Rust!' && echo $MY_VAR").await?;
    println!("Bash output:");
    println!("{}", bash_result.stdout);
    println!("Exit code: {}", bash_result.exit_code);
    println!("Execution time: {}ms", bash_result.execution_time_ms);
    println!();

    // ========================================================================
    // 5. Workbench - Pandas Operations
    // ========================================================================
    println!("=== Workbench - Pandas ===");
    let workbench = WorkbenchExecutor::new(client.clone(), session.session_id());

    // Generate pandas code
    let pandas_code = workbench.generate_pandas_code(PandasOperation::ReadCsv {
        url: "https://raw.githubusercontent.com/datasciencedojo/datasets/master/titanic.csv"
            .to_string(),
    });

    println!("Generated pandas code:");
    println!("{}", pandas_code);

    // Execute in workbench
    let pandas_result = workbench.execute_python(&pandas_code).await?;
    println!("Pandas result:");
    println!("{}", pandas_result.output);
    println!();

    // ========================================================================
    // 6. Workbench - Excel Operations
    // ========================================================================
    println!("=== Workbench - Excel ===");

    // Generate Excel read code
    let excel_code = workbench.generate_excel_code(ExcelOperation::Read {
        s3_url: "https://example.com/sample.xlsx".to_string(),
    });

    println!("Generated Excel code:");
    println!("{}", excel_code);
    println!();

    // ========================================================================
    // 7. Custom Python Code
    // ========================================================================
    println!("=== Workbench - Custom Python ===");

    let custom_code = r#"
import pandas as pd
import numpy as np

# Create sample data
data = {
    'name': ['Alice', 'Bob', 'Charlie'],
    'age': [25, 30, 35],
    'city': ['New York', 'London', 'Paris']
}

df = pd.DataFrame(data)
print("Sample DataFrame:")
print(df)
print(f"\nAverage age: {df['age'].mean()}")
"#;

    let custom_result = workbench.execute_python(custom_code).await?;
    println!("Custom Python result:");
    println!("{}", custom_result.output);
    println!();

    // ========================================================================
    // 8. Batch Bash Operations
    // ========================================================================
    println!("=== Batch Bash Operations ===");

    let bash_commands = vec![
        "echo 'Step 1: Creating directory'",
        "mkdir -p test_dir",
        "echo 'Step 2: Creating file'",
        "echo 'Hello, World!' > test_dir/hello.txt",
        "echo 'Step 3: Reading file'",
        "cat test_dir/hello.txt",
    ];

    let batch_results = bash.execute_batch(bash_commands).await?;
    println!("Executed {} bash commands:", batch_results.len());
    for (i, result) in batch_results.iter().enumerate() {
        if !result.stdout.is_empty() {
            println!("  Command {}: {}", i + 1, result.stdout.trim());
        }
    }
    println!();

    println!("✓ All meta tools demonstrated successfully!");

    Ok(())
}
