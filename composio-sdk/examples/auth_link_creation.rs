// Example: Creating authentication links for toolkits
//
// This example demonstrates how to create authentication links (Connect Links)
// for users to authenticate with external services like GitHub, Gmail, Slack, etc.
//
// Authentication links are used in two scenarios:
// 1. In-chat authentication: Agent automatically prompts users during conversation
// 2. Manual authentication: Pre-authenticate users during onboarding or from settings
//
// Run with: cargo run --example auth_link_creation

use composio_sdk::ComposioClient;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = env::var("COMPOSIO_API_KEY")
        .expect("COMPOSIO_API_KEY environment variable must be set");

    // Initialize client
    let client = ComposioClient::builder().api_key(api_key).build()?;

    println!("=== Authentication Link Creation Example ===\n");

    // Create a session for a user
    let user_id = "user_demo_123";
    println!("Creating session for user: {}", user_id);

    let session = client
        .create_session(user_id)
        .toolkits(vec!["github", "gmail", "slack"])
        .send()
        .await?;

    println!("✓ Session created: {}\n", session.session_id());

    // Example 1: Create auth link without callback URL
    println!("--- Example 1: Basic Auth Link ---");
    println!("Creating auth link for GitHub...");

    match session.create_auth_link("github", None).await {
        Ok(link) => {
            println!("✓ Auth link created successfully!");
            println!("  Link Token: {}", link.link_token);
            println!("  Redirect URL: {}", link.redirect_url);
            if let Some(account_id) = link.connected_account_id {
                println!("  Connected Account: {}", account_id);
            } else {
                println!("  Connected Account: None (new connection)");
            }
            println!("\n  👉 User should visit: {}\n", link.redirect_url);
        }
        Err(e) => {
            println!("✗ Failed to create auth link: {}", e);
        }
    }

    // Example 2: Create auth link with callback URL
    println!("--- Example 2: Auth Link with Callback ---");
    println!("Creating auth link for Gmail with callback...");

    let callback_url = "https://example.com/auth/callback";
    match session
        .create_auth_link("gmail", Some(callback_url.to_string()))
        .await
    {
        Ok(link) => {
            println!("✓ Auth link created successfully!");
            println!("  Link Token: {}", link.link_token);
            println!("  Redirect URL: {}", link.redirect_url);
            println!("  Callback URL: {}", callback_url);
            println!(
                "\n  After authentication, user will be redirected to:"
            );
            println!(
                "  {}?status=<success|failed>&connected_account_id=<id>\n",
                callback_url
            );
        }
        Err(e) => {
            println!("✗ Failed to create auth link: {}", e);
        }
    }

    // Example 3: Handle existing connection
    println!("--- Example 3: Handling Existing Connection ---");
    println!("Attempting to create auth link for already connected toolkit...");

    match session.create_auth_link("github", None).await {
        Ok(link) => {
            if link.connected_account_id.is_some() {
                println!("✓ User already has a connected account!");
                println!("  Connected Account: {:?}", link.connected_account_id);
                println!("  Link can be used to re-authenticate if needed.");
            } else {
                println!("✓ New auth link created");
            }
        }
        Err(e) => {
            println!("✗ Error: {}", e);
            println!("  This might happen if:");
            println!("  - Toolkit is invalid");
            println!("  - Connection already exists and cannot be recreated");
            println!("  - Network or API error occurred");
        }
    }

    // Example 4: Multiple toolkits
    println!("\n--- Example 4: Creating Links for Multiple Toolkits ---");
    let toolkits = vec!["slack", "notion", "linear"];

    for toolkit in toolkits {
        println!("Creating auth link for {}...", toolkit);
        match session.create_auth_link(toolkit, None).await {
            Ok(link) => {
                println!("  ✓ {}: {}", toolkit, link.redirect_url);
            }
            Err(e) => {
                println!("  ✗ {}: {}", toolkit, e);
            }
        }
    }

    println!("\n=== Authentication Flow ===");
    println!("1. Generate auth link using create_auth_link()");
    println!("2. Send redirect_url to user (via chat, email, or UI)");
    println!("3. User visits URL and completes OAuth flow");
    println!("4. User is redirected to callback_url (if provided)");
    println!("5. Check connection status using session.list_toolkits()");
    println!("\n=== Best Practices ===");
    println!("• Use callback URLs in production for better UX");
    println!("• Store link_token for tracking auth sessions");
    println!("• Handle errors gracefully (invalid toolkit, existing connection)");
    println!("• Check connection status before creating new links");
    println!("• Use in-chat auth (manage_connections=true) for conversational flows");
    println!("• Use manual auth for onboarding or settings pages");

    Ok(())
}
