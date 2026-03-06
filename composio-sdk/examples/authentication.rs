//! Comprehensive Authentication Example
//!
//! This example demonstrates the complete authentication workflow with Composio:
//! 1. Creating a session for a user
//! 2. Listing available toolkits and checking connection status
//! 3. Identifying which toolkits need authentication
//! 4. Creating authentication links (Connect Links) for disconnected toolkits
//! 5. Handling the authentication callback flow
//!
//! This example combines toolkit listing, connection checking, and auth link creation
//! to show a complete authentication management workflow.
//!
//! Run with: cargo run --example authentication

use composio_sdk::ComposioClient;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = env::var("COMPOSIO_API_KEY")
        .expect("COMPOSIO_API_KEY environment variable must be set");

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║     Composio Authentication Workflow Example              ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Initialize the Composio client
    println!("🔧 Initializing Composio client...");
    let client = ComposioClient::builder().api_key(api_key).build()?;
    println!("✓ Client initialized\n");

    // Step 1: Create a session for a user
    // Sessions scope toolkits, authentication, and tool execution to a specific user
    let user_id = "user_demo_auth_123";
    println!("👤 Creating session for user: {}", user_id);
    println!("   Enabling toolkits: github, gmail, slack, notion");

    let session = client
        .create_session(user_id)
        .toolkits(vec!["github", "gmail", "slack", "notion"])
        .manage_connections(true) // Enable in-chat authentication
        .send()
        .await?;

    println!("✓ Session created successfully!");
    println!("  Session ID: {}", session.session_id());
    println!("  MCP URL: {}\n", session.mcp_url());

    // Step 2: List all available toolkits and check their connection status
    // This shows which toolkits are connected and which need authentication
    println!("📋 Listing available toolkits and checking connection status...\n");

    let toolkits = session.list_toolkits().limit(20).send().await?;

    println!("Found {} toolkits (showing top {})", toolkits.total_items, toolkits.items.len());
    println!("─────────────────────────────────────────────────────────────");

    let mut connected_toolkits = Vec::new();
    let mut disconnected_toolkits = Vec::new();

    for toolkit in &toolkits.items {
        let is_connected = toolkit.connected_account.is_some();
        
        if is_connected {
            connected_toolkits.push(toolkit);
            println!("✓ {} ({})", toolkit.name, toolkit.slug);
            println!("  Status: CONNECTED");
            if let Some(account) = &toolkit.connected_account {
                println!("  Account ID: {}", account.id);
                println!("  Account Status: {}", account.status);
                println!("  Created: {}", account.created_at);
            }
        } else {
            disconnected_toolkits.push(toolkit);
            println!("✗ {} ({})", toolkit.name, toolkit.slug);
            println!("  Status: NOT CONNECTED");
            println!("  Auth Required: {}", !toolkit.is_no_auth);
        }
        
        println!("  Tools: {} | Triggers: {}", 
                 toolkit.meta.tools_count, 
                 toolkit.meta.triggers_count);
        println!();
    }

    // Step 3: Summary of connection status
    println!("─────────────────────────────────────────────────────────────");
    println!("📊 Connection Summary:");
    println!("  Connected: {} toolkits", connected_toolkits.len());
    println!("  Disconnected: {} toolkits", disconnected_toolkits.len());
    println!();

    // Step 4: Check if specific toolkits are connected
    // This is useful when you need to verify authentication before executing tools
    println!("🔍 Checking specific toolkit connection status...\n");

    let toolkits_to_check = vec!["github", "gmail", "slack"];
    
    for toolkit_slug in &toolkits_to_check {
        // Filter to get specific toolkit
        let result = session
            .list_toolkits()
            .toolkits(vec![*toolkit_slug])
            .send()
            .await?;

        if let Some(toolkit) = result.items.first() {
            let is_connected = toolkit.connected_account.is_some();
            
            if is_connected {
                println!("✓ {} is CONNECTED", toolkit.name);
                if let Some(account) = &toolkit.connected_account {
                    println!("  Account: {} ({})", account.id, account.status);
                }
            } else {
                println!("✗ {} is NOT CONNECTED", toolkit.name);
                println!("  Authentication required before using tools");
            }
        } else {
            println!("⚠ {} not found in session", toolkit_slug);
        }
        println!();
    }

    // Step 5: Create authentication links for disconnected toolkits
    // These links allow users to authenticate with external services
    println!("🔗 Creating authentication links for disconnected toolkits...\n");

    // Example callback URL (in production, this would be your application's callback endpoint)
    let callback_url = "https://your-app.com/auth/callback";

    for toolkit in disconnected_toolkits.iter().take(3) {
        println!("Creating auth link for {}...", toolkit.name);
        
        match session
            .create_auth_link(&toolkit.slug, Some(callback_url.to_string()))
            .await
        {
            Ok(link) => {
                println!("✓ Auth link created successfully!");
                println!("  Link Token: {}", link.link_token);
                println!("  Redirect URL: {}", link.redirect_url);
                println!("  Callback URL: {}", callback_url);
                
                if let Some(account_id) = link.connected_account_id {
                    println!("  Existing Account: {}", account_id);
                } else {
                    println!("  New Connection: Yes");
                }
                
                println!("\n  📱 Send this URL to the user:");
                println!("  {}", link.redirect_url);
                println!();
            }
            Err(e) => {
                println!("✗ Failed to create auth link: {}", e);
                println!();
            }
        }
    }

    // Step 6: Demonstrate authentication flow explanation
    println!("─────────────────────────────────────────────────────────────");
    println!("📖 Authentication Flow Explanation:\n");
    
    println!("1️⃣  CHECK CONNECTION STATUS");
    println!("   Use session.list_toolkits() to see which toolkits are connected");
    println!("   Filter by is_connected(true/false) to get specific status\n");
    
    println!("2️⃣  CREATE AUTH LINK");
    println!("   For disconnected toolkits, call session.create_auth_link()");
    println!("   Provide optional callback_url for redirect after auth\n");
    
    println!("3️⃣  USER AUTHENTICATES");
    println!("   Send redirect_url to user (via chat, email, or UI)");
    println!("   User visits URL and completes OAuth flow with the service\n");
    
    println!("4️⃣  HANDLE CALLBACK");
    println!("   After authentication, user is redirected to callback_url");
    println!("   Callback includes query parameters:");
    println!("   - status: 'success' or 'failed'");
    println!("   - connected_account_id: ID of the connected account\n");
    
    println!("5️⃣  VERIFY CONNECTION");
    println!("   Check connection status again using session.list_toolkits()");
    println!("   Verify connected_account is present and status is 'ACTIVE'\n");
    
    println!("6️⃣  EXECUTE TOOLS");
    println!("   Once connected, you can execute tools for that toolkit");
    println!("   Tools automatically use the user's authenticated credentials\n");

    // Step 7: Best practices and tips
    println!("─────────────────────────────────────────────────────────────");
    println!("💡 Best Practices:\n");
    
    println!("✓ Always check connection status before executing tools");
    println!("✓ Use callback URLs in production for better user experience");
    println!("✓ Store link_token for tracking authentication sessions");
    println!("✓ Handle authentication errors gracefully");
    println!("✓ Use in-chat auth (manage_connections=true) for conversational flows");
    println!("✓ Use manual auth for onboarding or settings pages");
    println!("✓ Check account status (ACTIVE, EXPIRED, FAILED) before tool execution");
    println!("✓ Implement re-authentication flow for expired connections\n");

    // Step 8: Example of filtering connected vs disconnected toolkits
    println!("─────────────────────────────────────────────────────────────");
    println!("🔎 Advanced Filtering Examples:\n");

    // Get only connected toolkits
    println!("Fetching only CONNECTED toolkits...");
    let connected = session
        .list_toolkits()
        .is_connected(true)
        .limit(5)
        .send()
        .await?;
    
    println!("✓ Found {} connected toolkits", connected.items.len());
    for toolkit in &connected.items {
        println!("  - {} ({})", toolkit.name, toolkit.slug);
    }
    println!();

    // Get only disconnected toolkits
    println!("Fetching only DISCONNECTED toolkits...");
    let disconnected = session
        .list_toolkits()
        .is_connected(false)
        .limit(5)
        .send()
        .await?;
    
    println!("✓ Found {} disconnected toolkits", disconnected.items.len());
    for toolkit in &disconnected.items {
        println!("  - {} ({})", toolkit.name, toolkit.slug);
    }
    println!();

    // Search for specific toolkits
    println!("Searching for 'mail' toolkits...");
    let search_results = session
        .list_toolkits()
        .search("mail")
        .limit(5)
        .send()
        .await?;
    
    println!("✓ Found {} toolkits matching 'mail'", search_results.items.len());
    for toolkit in &search_results.items {
        let status = if toolkit.connected_account.is_some() {
            "CONNECTED"
        } else {
            "NOT CONNECTED"
        };
        println!("  - {} ({}) - {}", toolkit.name, toolkit.slug, status);
    }
    println!();

    println!("─────────────────────────────────────────────────────────────");
    println!("✅ Authentication workflow example completed successfully!");
    println!("─────────────────────────────────────────────────────────────\n");

    Ok(())
}
