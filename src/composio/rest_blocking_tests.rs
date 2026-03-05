// Integration tests for ComposioRestBlockingClient
//
// These tests verify the blocking client works correctly for wizard use.
// Note: These are unit tests that don't require a real API key.

#[cfg(test)]
mod tests {
    use super::super::rest_blocking::ComposioRestBlockingClient;

    #[test]
    fn client_constructs_without_panicking() {
        let client = ComposioRestBlockingClient::new("test_key".to_string());
        // Just verify it constructs without panicking
        drop(client);
    }

    // Note: Real API tests would go here with #[ignore] attribute
    // and would require COMPOSIO_API_KEY environment variable
}
