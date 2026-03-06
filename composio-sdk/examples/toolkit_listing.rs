//! Toolkit Listing Example - Composio Rust SDK
//!
//! This example demonstrates comprehensive toolkit listing and filtering capabilities:
//! 1. Basic toolkit listing with pagination
//! 2. Filtering by connection status (connected vs disconnected)
//! 3. Pagination with cursor for large result sets
//! 4. Searching toolkits by name or description
//! 5. Filtering by specific toolkit slugs
//! 6. Understanding toolkit metadata and capabilities
//!
//! ## What are Toolkits?
//!
//! Toolkits are collections of related tools for external services:
//! - **github**: 150+ tools for GitHub operations (repos, issues, PRs, etc.)
//! - **gmail**: 45+ tools for Gmail operations (send, read, search emails)
//! - **slack**: 80+ tools for Slack operations (messages, channels, users)
//! - **notion**: 60+ tools for Notion operations (pages, databases, blocks)
//!
//! ## Filtering Options
//!
//! The SDK provides several filtering options for toolkit listing:
//! - **limit(n)**: Limit results to n toolkits (default: 20)
//! - **cursor(c)**: Pagination cursor for fetching next page
//! - **is_connected(bool)**: Filter by connection status (true = connected, false = disconnected)
//! - **search(query)**: Search toolkits by name or description
//! - **toolkits(slugs)**: Filter by specific toolkit slugs
//!
//! ## Use Cases
//!
//! - Building a connections dashboard showing all available integrations
//! - Checking which toolkits a user has already connected
//! - Discovering toolkits by searching for keywords
//! - Paginating through large lists of toolkits
//! - Displaying toolkit metadata (logo, description, tool counts)
//!
//! Run with: cargo run --example toolkit_listing

use composio_sdk::ComposioClient;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = env::var("COMPOSIO_API_KEY")
        .expect("COMPOSIO_API_KEY environment variable must be set");

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║        Composio Toolkit Listing Example                   ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Initialize the Composio client
    println!("🔧 Initializing Composio client...");
    let client = ComposioClient::builder().api_key(api_key).build()?;
    println!("✓ Client initialized\n");

    // Create a session for a user
    // Sessions scope toolkit access and authentication to a specific user
    let user_id = "user_toolkit_demo_123";
    println!("👤 Creating session for user: {}", user_id);
    println!("   Note: Not specifying toolkits - all toolkits will be available via COMPOSIO_SEARCH_TOOLS");

    let session = client
        .create_session(user_id)
        .send()
        .await?;

    println!("✓ Session created successfully!");
    println!("  Session ID: {}", session.session_id());
    println!();

    // ═══════════════════════════════════════════════════════════════════
    // Example 1: List all toolkits (basic usage)
    // ═══════════════════════════════════════════════════════════════════
    println!("═══════════════════════════════════════════════════════════════");
    println!("Example 1: List all toolkits (basic usage)");
    println!("═══════════════════════════════════════════════════════════════\n");

    // By default, list_toolkits() returns the first 20 toolkits
    let all_toolkits = session.list_toolkits().send().await?;

    println!("📊 Pagination Info:");
    println!("  Found: {} toolkits on this page", all_toolkits.items.len());
    println!("  Total: {} toolkits available", all_toolkits.total_items);
    println!("  Page: {} of {}", all_toolkits.current_page, all_toolkits.total_pages);
    println!("  Next cursor: {}", all_toolkits.next_cursor.as_deref().unwrap_or("None"));
    println!();

    println!("📋 Toolkits:");
    for toolkit in &all_toolkits.items {
        println!("─────────────────────────────────────────────────────────────");
        println!("🔧 {} ({})", toolkit.name, toolkit.slug);
        println!("   Enabled: {}", toolkit.enabled);
        println!("   Tools: {} | Triggers: {}", 
                 toolkit.meta.tools_count, 
                 toolkit.meta.triggers_count);
        
        // Check connection status
        if let Some(account) = &toolkit.connected_account {
            println!("   ✓ Connected: {} ({})", account.id, account.status);
            println!("   Created: {}", account.created_at);
        } else {
            println!("   ✗ Not Connected");
        }
    }
    println!();

    // ═══════════════════════════════════════════════════════════════════
    // Example 2: Filter by connection status (connected toolkits only)
    // ═══════════════════════════════════════════════════════════════════
    println!("═══════════════════════════════════════════════════════════════");
    println!("Example 2: Filter by connection status (connected only)");
    println!("═══════════════════════════════════════════════════════════════\n");

    // Use is_connected(true) to get only toolkits with active connections
    // This is useful for showing users which integrations they've already set up
    let connected_toolkits = session
        .list_toolkits()
        .is_connected(true)  // Filter: only connected toolkits
        .send()
        .await?;

    println!("✓ Found {} connected toolkits", connected_toolkits.items.len());
    println!();

    for toolkit in &connected_toolkits.items {
        println!("🔗 {} ({})", toolkit.name, toolkit.slug);
        if let Some(account) = &toolkit.connected_account {
            println!("   Account ID: {}", account.id);
            println!("   Status: {}", account.status);
            println!("   Created: {}", account.created_at);
        }
        println!();
    }

    // ═══════════════════════════════════════════════════════════════════
    // Example 3: Filter by connection status (disconnected toolkits only)
    // ═══════════════════════════════════════════════════════════════════
    println!("═══════════════════════════════════════════════════════════════");
    println!("Example 3: Filter by connection status (disconnected only)");
    println!("═══════════════════════════════════════════════════════════════\n");

    // Use is_connected(false) to get only toolkits without connections
    // This is useful for showing users which integrations are available to connect
    let disconnected_toolkits = session
        .list_toolkits()
        .is_connected(false)  // Filter: only disconnected toolkits
        .limit(10)            // Limit to 10 results
        .send()
        .await?;

    println!("✗ Found {} disconnected toolkits (showing first 10)", disconnected_toolkits.items.len());
    println!();

    for toolkit in &disconnected_toolkits.items {
        println!("📦 {} ({})", toolkit.name, toolkit.slug);
        println!("   Description: {}", toolkit.meta.description);
        println!("   No auth required: {}", toolkit.is_no_auth);
        println!("   Auth schemes: {:?}", toolkit.composio_managed_auth_schemes);
        println!();
    }

    // ═══════════════════════════════════════════════════════════════════
    // Example 4: Search toolkits by keyword
    // ═══════════════════════════════════════════════════════════════════
    println!("═══════════════════════════════════════════════════════════════");
    println!("Example 4: Search toolkits by keyword");
    println!("═══════════════════════════════════════════════════════════════\n");

    // Use search() to find toolkits by name or description
    // This is useful for implementing a search bar in your UI
    let search_query = "git";
    println!("🔍 Searching for toolkits matching '{}'...", search_query);
    
    let search_results = session
        .list_toolkits()
        .search(search_query)  // Search by name or description
        .send()
        .await?;

    println!("✓ Found {} toolkits matching '{}'", search_results.items.len(), search_query);
    println!();

    for toolkit in &search_results.items {
        println!("🔎 {} ({})", toolkit.name, toolkit.slug);
        println!("   Description: {}", toolkit.meta.description);
        println!("   Categories: {:?}", toolkit.meta.categories);
        println!();
    }

    // Try another search
    let search_query2 = "mail";
    println!("🔍 Searching for toolkits matching '{}'...", search_query2);
    
    let search_results2 = session
        .list_toolkits()
        .search(search_query2)
        .send()
        .await?;

    println!("✓ Found {} toolkits matching '{}'", search_results2.items.len(), search_query2);
    println!();

    for toolkit in &search_results2.items {
        println!("📧 {} ({})", toolkit.name, toolkit.slug);
        println!("   Description: {}", toolkit.meta.description);
        println!();
    }

    // ═══════════════════════════════════════════════════════════════════
    // Example 5: Filter by specific toolkit slugs
    // ═══════════════════════════════════════════════════════════════════
    println!("═══════════════════════════════════════════════════════════════");
    println!("Example 5: Filter by specific toolkit slugs");
    println!("═══════════════════════════════════════════════════════════════\n");

    // Use toolkits() to get information about specific toolkits
    // This is useful when you want to check the status of particular integrations
    let specific_slugs = vec!["github", "gmail", "slack"];
    println!("📌 Fetching specific toolkits: {:?}", specific_slugs);
    
    let specific_toolkits = session
        .list_toolkits()
        .toolkits(specific_slugs.clone())  // Filter by specific slugs
        .send()
        .await?;

    println!("✓ Found {} specific toolkits", specific_toolkits.items.len());
    println!();

    for toolkit in &specific_toolkits.items {
        println!("🎯 {} ({})", toolkit.name, toolkit.slug);
        println!("   Categories: {:?}", toolkit.meta.categories);
        println!("   Version: {}", toolkit.meta.version);
        println!("   Logo: {}", toolkit.meta.logo);
        println!("   Auth schemes: {:?}", toolkit.composio_managed_auth_schemes);
        
        if let Some(account) = &toolkit.connected_account {
            println!("   ✓ Connected ({})", account.status);
        } else {
            println!("   ✗ Not connected");
        }
        println!();
    }

    // ═══════════════════════════════════════════════════════════════════
    // Example 6: Pagination with cursor
    // ═══════════════════════════════════════════════════════════════════
    println!("═══════════════════════════════════════════════════════════════");
    println!("Example 6: Pagination with cursor");
    println!("═══════════════════════════════════════════════════════════════\n");

    // When dealing with large lists, use pagination to fetch results in chunks
    // The cursor allows you to fetch the next page of results
    println!("📄 Paginating through toolkits (5 per page)...");
    println!();

    let mut cursor = None;
    let mut page = 1;
    let limit = 5;
    let max_pages = 3;  // Limit to 3 pages for demo

    loop {
        println!("─── Page {} ───", page);
        
        // Build the request with optional cursor
        let mut builder = session.list_toolkits().limit(limit);
        
        if let Some(c) = cursor {
            builder = builder.cursor(c);  // Use cursor for next page
        }

        let response = builder.send().await?;

        println!("Toolkits on this page: {}", response.items.len());
        for (i, toolkit) in response.items.iter().enumerate() {
            println!("  {}. {} ({})", i + 1, toolkit.name, toolkit.slug);
        }

        // Check if there are more pages
        cursor = response.next_cursor;
        if cursor.is_none() {
            println!("\n✓ No more pages available");
            break;
        }

        page += 1;
        
        // Stop after max_pages for demo purposes
        if page > max_pages {
            println!("\n⚠ Stopping after {} pages (demo limit)", max_pages);
            println!("  Next cursor available: {}", cursor.as_deref().unwrap_or("None"));
            break;
        }
        
        println!();
    }
    println!();

    // ═══════════════════════════════════════════════════════════════════
    // Example 7: Combined filters
    // ═══════════════════════════════════════════════════════════════════
    println!("═══════════════════════════════════════════════════════════════");
    println!("Example 7: Combined filters");
    println!("═══════════════════════════════════════════════════════════════\n");

    // You can combine multiple filters for more specific queries
    // This example finds disconnected toolkits matching a search term
    println!("🔍 Finding disconnected toolkits matching 'mail'...");
    
    let filtered = session
        .list_toolkits()
        .limit(10)              // Limit results
        .is_connected(false)    // Only disconnected
        .search("mail")         // Search for 'mail'
        .send()
        .await?;

    println!("✓ Found {} disconnected toolkits matching 'mail'", filtered.items.len());
    println!();

    for toolkit in &filtered.items {
        println!("📬 {} ({})", toolkit.name, toolkit.slug);
        println!("   Description: {}", toolkit.meta.description);
        println!("   No auth required: {}", toolkit.is_no_auth);
        println!("   Tools: {} | Triggers: {}", 
                 toolkit.meta.tools_count, 
                 toolkit.meta.triggers_count);
        println!();
    }

    // Another combined filter example: connected toolkits with specific slugs
    println!("🔍 Finding connected toolkits from specific list...");
    
    let connected_specific = session
        .list_toolkits()
        .toolkits(vec!["github", "gmail", "slack", "notion"])
        .is_connected(true)
        .send()
        .await?;

    println!("✓ Found {} connected toolkits from the specified list", connected_specific.items.len());
    println!();

    for toolkit in &connected_specific.items {
        println!("✅ {} ({})", toolkit.name, toolkit.slug);
        if let Some(account) = &toolkit.connected_account {
            println!("   Account: {} ({})", account.id, account.status);
        }
        println!();
    }

    // ═══════════════════════════════════════════════════════════════════
    // Example 8: Toolkit metadata deep dive
    // ═══════════════════════════════════════════════════════════════════
    println!("═══════════════════════════════════════════════════════════════");
    println!("Example 8: Toolkit metadata deep dive");
    println!("═══════════════════════════════════════════════════════════════\n");

    // Toolkit metadata provides rich information about each integration
    // This is useful for displaying detailed information in your UI
    if let Some(toolkit) = all_toolkits.items.first() {
        println!("📊 Detailed metadata for: {}", toolkit.name);
        println!("─────────────────────────────────────────────────────────────");
        println!("Basic Info:");
        println!("  Slug: {}", toolkit.slug);
        println!("  Name: {}", toolkit.name);
        println!("  Enabled: {}", toolkit.enabled);
        println!();
        
        println!("Metadata:");
        println!("  Logo URL: {}", toolkit.meta.logo);
        println!("  Description: {}", toolkit.meta.description);
        println!("  Categories: {:?}", toolkit.meta.categories);
        println!("  Version: {}", toolkit.meta.version);
        println!();
        
        println!("Capabilities:");
        println!("  Tools count: {}", toolkit.meta.tools_count);
        println!("  Triggers count: {}", toolkit.meta.triggers_count);
        println!();
        
        println!("Authentication:");
        println!("  No auth required: {}", toolkit.is_no_auth);
        println!("  Composio managed auth schemes: {:?}", toolkit.composio_managed_auth_schemes);
        println!();
        
        println!("Connection Status:");
        if let Some(account) = &toolkit.connected_account {
            println!("  ✓ Connected");
            println!("  Account ID: {}", account.id);
            println!("  Status: {}", account.status);
            println!("  Created: {}", account.created_at);
        } else {
            println!("  ✗ Not connected");
        }
        println!();
    }

    // ═══════════════════════════════════════════════════════════════════
    // Summary and Best Practices
    // ═══════════════════════════════════════════════════════════════════
    println!("═══════════════════════════════════════════════════════════════");
    println!("📖 Summary and Best Practices");
    println!("═══════════════════════════════════════════════════════════════\n");

    println!("✅ Filtering Options Summary:");
    println!("  • limit(n) - Limit results to n toolkits");
    println!("  • cursor(c) - Fetch next page using cursor");
    println!("  • is_connected(bool) - Filter by connection status");
    println!("  • search(query) - Search by name or description");
    println!("  • toolkits(slugs) - Filter by specific toolkit slugs");
    println!();

    println!("💡 Best Practices:");
    println!("  • Use pagination for large lists (default limit: 20)");
    println!("  • Filter by is_connected to show relevant toolkits");
    println!("  • Use search for user-friendly toolkit discovery");
    println!("  • Check toolkit.enabled before showing to users");
    println!("  • Display toolkit.meta for rich UI information");
    println!("  • Verify connection status before tool execution");
    println!("  • Use specific toolkit slugs for targeted queries");
    println!();

    println!("🎯 Common Use Cases:");
    println!("  • Connections dashboard: List all with connection status");
    println!("  • Available integrations: Filter is_connected(false)");
    println!("  • Active integrations: Filter is_connected(true)");
    println!("  • Search bar: Use search() with user input");
    println!("  • Specific toolkit check: Use toolkits() with slugs");
    println!("  • Large lists: Use pagination with cursor");
    println!();

    println!("─────────────────────────────────────────────────────────────");
    println!("✅ Toolkit listing example completed successfully!");
    println!("─────────────────────────────────────────────────────────────\n");

    Ok(())
}
