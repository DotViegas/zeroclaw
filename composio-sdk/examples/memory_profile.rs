use composio_sdk::{ComposioClient, ComposioError};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), ComposioError> {
    println!("=== Composio SDK Memory Profiling ===\n");
    
    // Get API key from environment
    let api_key = std::env::var("COMPOSIO_API_KEY")
        .unwrap_or_else(|_| "test_key".to_string());
    
    println!("1. Creating Composio client...");
    let start = Instant::now();
    let client = ComposioClient::builder()
        .api_key(api_key)
        .build()?;
    println!("   Client created in {:?}", start.elapsed());
    
    println!("\n2. Creating session...");
    let start = Instant::now();
    let session_builder = client
        .create_session("memory_test_user")
        .toolkits(vec!["github"]);
    println!("   Session builder created in {:?}", start.elapsed());
    
    println!("\n3. Memory footprint analysis:");
    println!("   - Client size: {} bytes", std::mem::size_of_val(&client));
    println!("   - Session builder size: {} bytes", std::mem::size_of_val(&session_builder));
    
    println!("\n=== Memory Profile Complete ===");
    println!("\nNote: For detailed runtime memory profiling, use tools like:");
    println!("  - valgrind (Linux): valgrind --tool=massif ./target/release/examples/memory_profile");
    println!("  - heaptrack (Linux): heaptrack ./target/release/examples/memory_profile");
    println!("  - Windows Performance Analyzer (Windows)");
    println!("  - Instruments (macOS)");
    
    Ok(())
}
