# Composio Rust SDK Tests

This directory contains comprehensive integration tests for the Composio Rust SDK.

## Test Files

### Error Handling Tests

- **error_handling_integration_test.rs**: Tests for HTTP error code handling (400, 401, 403, 404, 500)
  - Validates proper conversion of HTTP errors to ComposioError
  - Verifies error messages include request_id and suggested_fix
  - Tests both tool execution and meta tool execution error paths
  - Tests error responses with minimal fields and malformed JSON

- **retry_behavior_test.rs**: Tests for retry logic on transient errors
  - Validates retry behavior for 429, 500, 502, 503, 504 status codes
  - Verifies no retry for client errors (400, 401, 403, 404)
  - Tests exponential backoff timing
  - Tests retry exhaustion on persistent errors
  - Tests both tool execution and meta tool execution retry paths

### Session Management Tests

- **session_creation_test.rs**: Tests for session creation with various configurations
- **session_retrieval_test.rs**: Tests for retrieving existing sessions

### Tool Execution Tests

- **tool_execution_test.rs**: Tests for regular tool execution
- **meta_tool_execution_test.rs**: Tests for meta tool execution (COMPOSIO_SEARCH_TOOLS, etc.)

### Data Model Tests

- **request_models_test.rs**: Tests for request model serialization
- **response_models_test.rs**: Tests for response model deserialization

## Running Tests

Run all tests:
```bash
cargo test --package composio-sdk
```

Run specific test file:
```bash
cargo test --package composio-sdk --test error_handling_integration_test
cargo test --package composio-sdk --test retry_behavior_test
```

Run tests with output:
```bash
cargo test --package composio-sdk -- --nocapture
```

## Python SDK Reference

The error handling tests are designed to be compatible with the Python SDK's error handling behavior. Key compatibility points:

1. **Error Structure**: Matches Python SDK's error response format with message, status, code, slug, request_id, suggested_fix, and errors array
2. **Retry Logic**: Implements same retry behavior as Python SDK (retry on 429, 5xx; no retry on 4xx except 429)
3. **Error Context**: Preserves all error context fields for debugging and user feedback

### Python SDK Error Fixtures

When the Python SDK is available at `vendor/composio-python/`, you can reference error fixtures from:
- `vendor/composio-python/tests/fixtures/` - Test fixtures with sample error responses
- `vendor/composio-python/composio/core/models/errors.py` - Error type definitions

To download the Python SDK for reference:
```bash
git clone --depth 1 https://github.com/ComposioHQ/composio.git vendor/composio-python
```

## Test Coverage

Current test coverage includes:

- ✅ HTTP error code handling (400, 401, 403, 404, 500)
- ✅ Error message structure validation
- ✅ request_id and suggested_fix field verification
- ✅ Retry behavior for transient errors (429, 500, 502, 503, 504)
- ✅ No retry for client errors (400, 401, 403, 404)
- ✅ Exponential backoff timing
- ✅ Retry exhaustion handling
- ✅ Error handling for both tool and meta tool execution
- ✅ Malformed JSON error response handling
- ✅ Minimal error response handling (missing optional fields)

## Notes

- All tests use wiremock for HTTP mocking to avoid external dependencies
- Tests are designed to run quickly with minimal retry delays (10-50ms)
- Network error retry behavior is tested indirectly through HTTP error simulation
- Tests validate both successful retry scenarios and retry exhaustion scenarios
