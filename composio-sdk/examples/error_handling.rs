//! Error Handling Example - Composio Rust SDK
//!
//! This example demonstrates comprehensive error handling patterns for the Composio SDK.
//! It shows how to:
//! - Match on different error types
//! - Handle ApiError with various status codes
//! - Handle NetworkError for connection issues
//! - Handle SerializationError for JSON parsing failures
//! - Understand retry behavior for transient errors
//! - Access error details like request_id and suggested_fix
//! - Implement robust error handling in production code
//!
//! ## Prerequisites
//!
//! 1. Set your Composio API key as an environment variable:
//!    ```bash
//!    export COMPOSIO_API_KEY="your-api-key-here"
//!    ```
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run --example error_handling
//! ```
//!
//! ## What This Example Shows
//!
//! - **Error Type Matching**: Pattern matching on different error variants
//! - **ApiError Handling**: Extracting status codes, messages, and suggested fixes
//! - **NetworkError Handling**: Dealing with connection issues
//! - **SerializationError Handling**: Handling JSON parsing failures
//! - **Retry Logic**: Understanding which errors are automatically retried
//! - **Error Details**: Accessing request_id, suggested_fix, and detailed errors
//! - **Production Patterns**: Best practices for error handling in real applications

use composio_sdk::{ComposioClient, ComposioError};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Composio Rust SDK - Error Handling Example ===\n");

    // ========================================================================
    // Example 1: Error Type Matching
    // ========================================================================
    //
    // The SDK provides five main error types:
    // - ApiError: HTTP errors from the Composio API
    // - NetworkError: Connection issues, timeouts
    // - SerializationError: JSON parsing failures
    // - InvalidInput: Client-side validation errors
    // - ConfigError: Configuration issues
    //
    // Use pattern matching to handle each error type appropriately.

    println!("Example 1: Error Type Matching\n");
    println!("Demonstrating how to match on different error types...\n");

    // Initialize client
    let client = match ComposioClient::builder()
        .api_key(std::env::var("COMPOSIO_API_KEY").unwrap_or_else(|_| {
            eprintln!("Error: COMPOSIO_API_KEY environment variable not set");
            std::process::exit(1);
        }))
        .build()
    {
        Ok(client) => {
            println!("✓ Client initialized successfully\n");
            client
        }
        Err(e) => {
            println!("✗ Failed to initialize client:");
            demonstrate_error_matching(&e);
            return Err(e.into());
        }
    };

    // ========================================================================
    // Example 2: Handling ApiError with Status Codes
    // ========================================================================
    //
    // ApiError includes detailed information about HTTP errors:
    // - status: HTTP status code (400, 401, 404, 500, etc.)
    // - message: Human-readable error message
    // - code: Machine-readable error code (optional)
    // - slug: URL-friendly error identifier (optional)
    // - request_id: Unique request identifier for support (optional)
    // - suggested_fix: Actionable guidance for fixing the error (optional)
    // - errors: Array of detailed field-level errors (optional)
    //
    // Different status codes require different handling strategies.

    println!("\nExample 2: Handling ApiError with Status Codes\n");
    println!("Creating a session and attempting operations that may fail...\n");

    let session = client
        .create_session("error_demo_user")
        .toolkits(vec!["github"])
        .send()
        .await?;

    println!("✓ Session created: {}\n", session.session_id());

    // Demonstrate 404 Not Found error
    println!("2a. Attempting to execute a non-existent tool...");
    match session
        .execute_tool("NONEXISTENT_TOOL_SLUG", json!({}))
        .await
    {
        Ok(_) => println!("  Unexpected success"),
        Err(e) => {
            println!("✓ Error caught as expected:");
            demonstrate_api_error_handling(&e);
        }
    }

    // Demonstrate 400 Bad Request error (missing required arguments)
    println!("\n2b. Attempting to execute a tool with invalid arguments...");
    match session
        .execute_tool(
            "GITHUB_CREATE_ISSUE",
            json!({
                // Missing required fields: owner, repo, title
                "body": "This will fail due to missing required fields"
            }),
        )
        .await
    {
        Ok(_) => println!("  Unexpected success"),
        Err(e) => {
            println!("✓ Error caught as expected:");
            demonstrate_api_error_handling(&e);
        }
    }

    // ========================================================================
    // Example 3: Handling NetworkError
    // ========================================================================
    //
    // NetworkError occurs when there are connection issues:
    // - DNS resolution failures
    // - Connection timeouts
    // - Connection refused
    // - TLS/SSL errors
    //
    // These errors are automatically retried by the SDK.

    println!("\n\nExample 3: Handling NetworkError\n");
    println!("Attempting to connect to an invalid base URL...\n");

    match ComposioClient::builder()
        .api_key("test_key")
        .base_url("https://invalid-domain-that-does-not-exist-12345.com")
        .build()
    {
        Ok(invalid_client) => {
            // Try to create a session with the invalid client
            match invalid_client
                .create_session("test_user")
                .send()
                .await
            {
                Ok(_) => println!("  Unexpected success"),
                Err(e) => {
                    println!("✓ Network error caught as expected:");
                    demonstrate_network_error_handling(&e);
                }
            }
        }
        Err(e) => {
            println!("✗ Failed to create client: {}", e);
        }
    }

    // ========================================================================
    // Example 4: Handling SerializationError
    // ========================================================================
    //
    // SerializationError occurs when:
    // - Invalid JSON is provided as arguments
    // - Response cannot be parsed as expected type
    //
    // These errors indicate a problem with data format.

    println!("\n\nExample 4: Handling SerializationError\n");
    println!("Demonstrating JSON serialization error handling...\n");

    // Note: The SDK handles most serialization internally, but you might
    // encounter these errors when working with custom data structures.
    
    // Example: Attempting to serialize invalid data
    let invalid_json_str = "{invalid json}";
    match serde_json::from_str::<serde_json::Value>(invalid_json_str) {
        Ok(_) => println!("  Unexpected success"),
        Err(e) => {
            println!("✓ Serialization error caught:");
            let composio_error: ComposioError = e.into();
            demonstrate_serialization_error_handling(&composio_error);
        }
    }

    // ========================================================================
    // Example 5: Understanding Retry Behavior
    // ========================================================================
    //
    // The SDK automatically retries certain errors:
    // - 429 (Rate Limited) - Retried with exponential backoff
    // - 500 (Internal Server Error) - Retried
    // - 502 (Bad Gateway) - Retried
    // - 503 (Service Unavailable) - Retried
    // - 504 (Gateway Timeout) - Retried
    // - NetworkError - Retried
    //
    // The SDK does NOT retry:
    // - 400 (Bad Request) - Client error, won't succeed on retry
    // - 401 (Unauthorized) - Authentication issue
    // - 403 (Forbidden) - Permission issue
    // - 404 (Not Found) - Resource doesn't exist
    // - SerializationError - Data format issue
    // - InvalidInput - Validation error
    // - ConfigError - Configuration issue

    println!("\n\nExample 5: Understanding Retry Behavior\n");
    println!("Checking which errors are retryable...\n");

    demonstrate_retry_behavior();

    // ========================================================================
    // Example 6: Accessing Error Details
    // ========================================================================
    //
    // ApiError provides rich error details that help with debugging:
    // - request_id: Include this when contacting support
    // - suggested_fix: Actionable guidance for fixing the error
    // - errors: Array of field-level validation errors
    //
    // Always check these fields for additional context.

    println!("\n\nExample 6: Accessing Error Details\n");
    println!("Demonstrating how to extract detailed error information...\n");

    // Attempt an operation that will return detailed errors
    match session
        .execute_tool(
            "GITHUB_CREATE_ISSUE",
            json!({
                "owner": "",  // Invalid: empty string
                "repo": "",   // Invalid: empty string
                "title": ""   // Invalid: empty string
            }),
        )
        .await
    {
        Ok(_) => println!("  Unexpected success"),
        Err(e) => {
            println!("✓ Error with details caught:");
            demonstrate_error_details_access(&e);
        }
    }

    // ========================================================================
    // Example 7: Production Error Handling Pattern
    // ========================================================================
    //
    // In production code, you should:
    // 1. Log errors with appropriate severity levels
    // 2. Return user-friendly error messages
    // 3. Include request_id for support inquiries
    // 4. Monitor error rates and patterns
    // 5. Implement circuit breakers for repeated failures
    // 6. Provide fallback behavior when appropriate

    println!("\n\nExample 7: Production Error Handling Pattern\n");
    println!("Demonstrating production-ready error handling...\n");

    match execute_tool_with_production_error_handling(
        &session,
        "GITHUB_GET_REPOS",
        json!({
            "owner": "composio",
            "type": "public"
        }),
    )
    .await
    {
        Ok(result) => {
            println!("✓ Tool executed successfully");
            println!("  Log ID: {}", result.log_id);
        }
        Err(user_message) => {
            println!("✗ Operation failed");
            println!("  User message: {}", user_message);
        }
    }

    println!("\n=== Example completed successfully! ===\n");
    
    println!("Key Takeaways:");
    println!("1. Always use pattern matching to handle different error types");
    println!("2. Check status codes to determine appropriate action");
    println!("3. Use suggested_fix for actionable guidance");
    println!("4. Include request_id when contacting support");
    println!("5. Trust the SDK's automatic retry logic for transient errors");
    println!("6. Log errors appropriately in production");
    println!("7. Provide user-friendly error messages");

    Ok(())
}

/// Demonstrates error type matching
///
/// This function shows how to use pattern matching to handle different
/// error types appropriately. Each error type requires different handling.
fn demonstrate_error_matching(error: &ComposioError) {
    println!("  Error Type: {}", match error {
        ComposioError::ApiError { .. } => "ApiError",
        ComposioError::NetworkError(_) => "NetworkError",
        ComposioError::SerializationError(_) => "SerializationError",
        ComposioError::InvalidInput(_) => "InvalidInput",
        ComposioError::ConfigError(_) => "ConfigError",
    });
    
    println!("  Message: {}", error);
    
    // Check if error is retryable
    if error.is_retryable() {
        println!("  ℹ️  This error will be automatically retried by the SDK");
    } else {
        println!("  ℹ️  This error will NOT be retried (fix required)");
    }
}

/// Demonstrates ApiError handling with status codes
///
/// ApiError includes rich information about HTTP errors. Different status
/// codes require different handling strategies:
/// - 4xx: Client errors - fix the request
/// - 5xx: Server errors - retry or wait
/// - 429: Rate limit - backoff and retry
fn demonstrate_api_error_handling(error: &ComposioError) {
    match error {
        ComposioError::ApiError {
            status,
            message,
            code,
            slug,
            request_id,
            suggested_fix,
            errors,
        } => {
            println!("  Status Code: {}", status);
            println!("  Message: {}", message);
            
            if let Some(code) = code {
                println!("  Error Code: {}", code);
            }
            
            if let Some(slug) = slug {
                println!("  Error Slug: {}", slug);
            }
            
            if let Some(request_id) = request_id {
                println!("  Request ID: {}", request_id);
                println!("  💡 Include this request ID when contacting support");
            }
            
            if let Some(fix) = suggested_fix {
                println!("  Suggested Fix: {}", fix);
            }
            
            if let Some(errors) = errors {
                if !errors.is_empty() {
                    println!("  Detailed Errors:");
                    for err in errors {
                        if let Some(field) = &err.field {
                            println!("    - Field '{}': {}", field, err.message);
                        } else {
                            println!("    - {}", err.message);
                        }
                    }
                }
            }
            
            // Provide guidance based on status code
            println!("  Guidance:");
            match *status {
                400 => println!("    - Bad Request: Check your request parameters"),
                401 => println!("    - Unauthorized: Verify your API key"),
                403 => println!("    - Forbidden: Check your permissions"),
                404 => println!("    - Not Found: Verify the resource exists"),
                429 => println!("    - Rate Limited: SDK will automatically retry with backoff"),
                500 => println!("    - Internal Server Error: SDK will automatically retry"),
                502 => println!("    - Bad Gateway: SDK will automatically retry"),
                503 => println!("    - Service Unavailable: SDK will automatically retry"),
                504 => println!("    - Gateway Timeout: SDK will automatically retry"),
                _ => println!("    - HTTP {}: See error message for details", status),
            }
        }
        _ => {
            println!("  Not an ApiError");
        }
    }
}

/// Demonstrates NetworkError handling
///
/// NetworkError occurs when there are connection issues. These errors
/// are automatically retried by the SDK with exponential backoff.
fn demonstrate_network_error_handling(error: &ComposioError) {
    match error {
        ComposioError::NetworkError(e) => {
            println!("  Network Error Details: {}", e);
            println!("  Possible Causes:");
            println!("    - DNS resolution failure");
            println!("    - Connection timeout");
            println!("    - Connection refused");
            println!("    - TLS/SSL error");
            println!("    - Network unreachable");
            println!("  ℹ️  The SDK will automatically retry this error");
            println!("  ℹ️  Check your internet connection and firewall settings");
        }
        _ => {
            println!("  Not a NetworkError");
        }
    }
}

/// Demonstrates SerializationError handling
///
/// SerializationError occurs when JSON parsing fails. These errors
/// indicate a problem with data format and require fixing the input.
fn demonstrate_serialization_error_handling(error: &ComposioError) {
    match error {
        ComposioError::SerializationError(e) => {
            println!("  Serialization Error Details: {}", e);
            println!("  Possible Causes:");
            println!("    - Invalid JSON syntax");
            println!("    - Type mismatch (expected string, got number)");
            println!("    - Missing required fields");
            println!("    - Extra fields not allowed by schema");
            println!("  ℹ️  This error will NOT be retried");
            println!("  ℹ️  Fix the JSON format and try again");
        }
        _ => {
            println!("  Not a SerializationError");
        }
    }
}

/// Demonstrates retry behavior for different error types
///
/// The SDK automatically retries certain errors with exponential backoff.
/// This function shows which errors are retryable and which are not.
fn demonstrate_retry_behavior() {
    println!("Retryable Errors (SDK will automatically retry):");
    
    // Rate limit error
    let rate_limit_error = ComposioError::ApiError {
        status: 429,
        message: "Rate limited".to_string(),
        code: None,
        slug: None,
        request_id: None,
        suggested_fix: None,
        errors: None,
    };
    println!("  - 429 Rate Limited: {}", rate_limit_error.is_retryable());
    
    // Server errors
    for status in [500, 502, 503, 504] {
        let error = ComposioError::ApiError {
            status,
            message: format!("Server error {}", status),
            code: None,
            slug: None,
            request_id: None,
            suggested_fix: None,
            errors: None,
        };
        println!("  - {} Server Error: {}", status, error.is_retryable());
    }
    
    println!("\nNon-Retryable Errors (fix required):");
    
    // Client errors
    for status in [400, 401, 403, 404] {
        let error = ComposioError::ApiError {
            status,
            message: format!("Client error {}", status),
            code: None,
            slug: None,
            request_id: None,
            suggested_fix: None,
            errors: None,
        };
        println!("  - {} Client Error: {}", status, error.is_retryable());
    }
    
    // Other error types
    let invalid_input = ComposioError::InvalidInput("Invalid API key".to_string());
    println!("  - InvalidInput: {}", invalid_input.is_retryable());
    
    let config_error = ComposioError::ConfigError("Invalid base URL".to_string());
    println!("  - ConfigError: {}", config_error.is_retryable());
    
    println!("\nRetry Configuration:");
    println!("  - Max retries: 3 (configurable)");
    println!("  - Initial delay: 1 second (configurable)");
    println!("  - Max delay: 10 seconds (configurable)");
    println!("  - Strategy: Exponential backoff");
}

/// Demonstrates accessing detailed error information
///
/// ApiError provides rich error details that help with debugging and
/// providing user-friendly error messages.
fn demonstrate_error_details_access(error: &ComposioError) {
    match error {
        ComposioError::ApiError {
            status,
            message,
            code,
            slug,
            request_id,
            suggested_fix,
            errors,
        } => {
            println!("  Accessing Error Details:");
            println!("    status: {}", status);
            println!("    message: {}", message);
            println!("    code: {:?}", code);
            println!("    slug: {:?}", slug);
            println!("    request_id: {:?}", request_id);
            println!("    suggested_fix: {:?}", suggested_fix);
            
            if let Some(errors) = errors {
                println!("    errors: {} field-level error(s)", errors.len());
                for (i, err) in errors.iter().enumerate() {
                    println!("      Error {}:", i + 1);
                    println!("        field: {:?}", err.field);
                    println!("        message: {}", err.message);
                }
            } else {
                println!("    errors: None");
            }
            
            println!("\n  How to Use These Details:");
            println!("    - Log 'message' for debugging");
            println!("    - Show 'suggested_fix' to users");
            println!("    - Include 'request_id' in support tickets");
            println!("    - Use 'errors' array for field-level validation feedback");
            println!("    - Check 'status' to determine retry strategy");
        }
        _ => {
            println!("  Not an ApiError - limited details available");
            println!("  Error: {}", error);
        }
    }
}

/// Production-ready error handling pattern
///
/// This function demonstrates how to handle errors in production code:
/// 1. Log errors with appropriate context
/// 2. Return user-friendly error messages
/// 3. Include request_id for support
/// 4. Handle different error types appropriately
async fn execute_tool_with_production_error_handling(
    session: &composio_sdk::Session,
    tool_slug: &str,
    arguments: serde_json::Value,
) -> Result<composio_sdk::ToolExecutionResponse, String> {
    match session.execute_tool(tool_slug, arguments).await {
        Ok(response) => {
            // Check if the tool itself returned an error
            if let Some(error) = &response.error {
                // Log the error (in production, use a proper logging framework)
                eprintln!("[ERROR] Tool execution failed: {}", error);
                eprintln!("[ERROR] Log ID: {}", response.log_id);
                
                // Return user-friendly message
                return Err(format!(
                    "The operation failed. Please try again or contact support with log ID: {}",
                    response.log_id
                ));
            }
            
            Ok(response)
        }
        Err(e) => {
            // Log the error with full details
            eprintln!("[ERROR] SDK error: {}", e);
            
            // Handle different error types
            match &e {
                ComposioError::ApiError {
                    status,
                    message,
                    request_id,
                    suggested_fix,
                    ..
                } => {
                    eprintln!("[ERROR] Status: {}", status);
                    eprintln!("[ERROR] Message: {}", message);
                    if let Some(req_id) = request_id {
                        eprintln!("[ERROR] Request ID: {}", req_id);
                    }
                    
                    // Return user-friendly message based on status
                    let user_message = match *status {
                        400 => "Invalid request. Please check your input and try again.".to_string(),
                        401 => "Authentication failed. Please check your credentials.".to_string(),
                        403 => "Access denied. You don't have permission for this operation.".to_string(),
                        404 => "Resource not found. Please verify the tool name.".to_string(),
                        429 => "Too many requests. Please wait a moment and try again.".to_string(),
                        500..=599 => "Service temporarily unavailable. Please try again in a few moments.".to_string(),
                        _ => suggested_fix.clone().unwrap_or_else(|| message.clone()),
                    };
                    
                    // Include request_id if available
                    if let Some(req_id) = request_id {
                        Err(format!("{} (Request ID: {})", user_message, req_id))
                    } else {
                        Err(user_message)
                    }
                }
                
                ComposioError::NetworkError(_) => {
                    eprintln!("[ERROR] Network error - will retry automatically");
                    Err("Connection issue. Please check your internet connection.".to_string())
                }
                
                ComposioError::SerializationError(_) => {
                    eprintln!("[ERROR] Serialization error - invalid data format");
                    Err("Invalid data format. Please contact support.".to_string())
                }
                
                ComposioError::InvalidInput(msg) => {
                    eprintln!("[ERROR] Invalid input: {}", msg);
                    Err(format!("Invalid input: {}", msg))
                }
                
                ComposioError::ConfigError(msg) => {
                    eprintln!("[ERROR] Configuration error: {}", msg);
                    Err("Configuration error. Please contact support.".to_string())
                }
            }
        }
    }
}
