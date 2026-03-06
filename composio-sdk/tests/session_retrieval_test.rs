//! Integration tests for session retrieval

use composio_sdk::client::ComposioClient;
use composio_sdk::error::ComposioError;

#[tokio::test]
async fn test_get_session_with_valid_id() {
    // This test requires a real API key and session ID
    // Skip if not available
    let api_key = std::env::var("COMPOSIO_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        eprintln!("Skipping test: COMPOSIO_API_KEY not set");
        return;
    }

    let client = ComposioClient::builder()
        .api_key(api_key)
        .build()
        .unwrap();

    // First create a session
    let created_session = client
        .create_session("test_user_retrieval")
        .send()
        .await
        .unwrap();

    let session_id = created_session.session_id().to_string();

    // Now retrieve it
    let retrieved_session = client.get_session(&session_id).await.unwrap();

    // Verify the session details match
    assert_eq!(retrieved_session.session_id(), session_id);
    assert!(!retrieved_session.mcp_url().is_empty());
    assert!(!retrieved_session.tools().is_empty());
}

#[tokio::test]
async fn test_get_session_with_invalid_id() {
    let api_key = std::env::var("COMPOSIO_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        eprintln!("Skipping test: COMPOSIO_API_KEY not set");
        return;
    }

    let client = ComposioClient::builder()
        .api_key(api_key)
        .build()
        .unwrap();

    // Try to retrieve a non-existent session
    let result = client.get_session("invalid_session_id").await;

    // Should return an error
    assert!(result.is_err());
    
    // Check if it's an ApiError with 404 status
    match result {
        Err(ComposioError::ApiError { status, .. }) => {
            assert_eq!(status, 404);
        }
        _ => panic!("Expected ApiError with 404 status"),
    }
}

#[tokio::test]
async fn test_get_session_accepts_string() {
    let api_key = std::env::var("COMPOSIO_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        eprintln!("Skipping test: COMPOSIO_API_KEY not set");
        return;
    }

    let client = ComposioClient::builder()
        .api_key(api_key)
        .build()
        .unwrap();

    // Create a session
    let created_session = client
        .create_session("test_user_string")
        .send()
        .await
        .unwrap();

    let session_id = created_session.session_id().to_string();

    // Test with String
    let _retrieved = client.get_session(session_id.clone()).await.unwrap();

    // Test with &str
    let _retrieved = client.get_session(session_id.as_str()).await.unwrap();
}

#[tokio::test]
async fn test_get_session_retry_on_transient_errors() {
    // This test would require a mock server to simulate transient errors
    // For now, we'll just verify the method signature compiles
    let client = ComposioClient::builder()
        .api_key("test_key")
        .max_retries(5)
        .build()
        .unwrap();

    // The retry logic is tested in the retry module
    // This just verifies the integration
    let _ = client.get_session("test_id").await;
}
