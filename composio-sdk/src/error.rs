use serde::Deserialize;
use thiserror::Error;

/// Main error type for the Composio SDK
#[derive(Debug, Error)]
pub enum ComposioError {
    /// API error returned from Composio backend
    #[error("API error: {message} (status: {status})")]
    ApiError {
        /// HTTP status code
        status: u16,
        /// Error message
        message: String,
        /// Error code (optional)
        code: Option<String>,
        /// Error slug identifier (optional)
        slug: Option<String>,
        /// Request ID for debugging (optional)
        request_id: Option<String>,
        /// Suggested fix for the error (optional)
        suggested_fix: Option<String>,
        /// Detailed field-level errors (optional)
        errors: Option<Vec<ErrorDetail>>,
    },

    /// Network error from HTTP client
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    /// JSON serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Validation error
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Execution error
    #[error("Execution error: {0}")]
    ExecutionError(String),

    /// Invalid input provided by user
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Detailed error information for individual field errors
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ErrorDetail {
    /// Field name that caused the error (optional)
    pub field: Option<String>,
    /// Error message for this field
    pub message: String,
}

/// Error response structure from Composio API
#[derive(Debug, Clone, Deserialize)]
pub struct ErrorResponse {
    /// Error message
    pub message: String,
    /// Error code (optional)
    pub code: Option<String>,
    /// Error slug identifier (optional)
    pub slug: Option<String>,
    /// HTTP status code
    pub status: u16,
    /// Request ID for debugging (optional)
    pub request_id: Option<String>,
    /// Suggested fix for the error (optional)
    pub suggested_fix: Option<String>,
    /// Detailed field-level errors (optional)
    pub errors: Option<Vec<ErrorDetail>>,
}

impl ComposioError {
    /// Create an ApiError from an HTTP response
    ///
    /// This method attempts to parse the response body as an ErrorResponse.
    /// If parsing fails, it creates a generic ApiError with the status code.
    pub async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status().as_u16();

        match response.json::<ErrorResponse>().await {
            Ok(err_resp) => ComposioError::ApiError {
                status,
                message: err_resp.message,
                code: err_resp.code,
                slug: err_resp.slug,
                request_id: err_resp.request_id,
                suggested_fix: err_resp.suggested_fix,
                errors: err_resp.errors,
            },
            Err(_) => ComposioError::ApiError {
                status,
                message: format!("HTTP error {}", status),
                code: None,
                slug: None,
                request_id: None,
                suggested_fix: None,
                errors: None,
            },
        }
    }

    /// Check if this error should be retried
    ///
    /// Returns true for transient errors that may succeed on retry:
    /// - 429 (Rate Limited)
    /// - 500 (Internal Server Error)
    /// - 502 (Bad Gateway)
    /// - 503 (Service Unavailable)
    /// - 504 (Gateway Timeout)
    /// - Network errors
    ///
    /// Returns false for client errors (4xx except 429) that won't succeed on retry.
    pub fn is_retryable(&self) -> bool {
        match self {
            ComposioError::ApiError { status, .. } => {
                matches!(status, 429 | 500 | 502 | 503 | 504)
            }
            ComposioError::NetworkError(_) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_error_display() {
        let error = ComposioError::ApiError {
            status: 404,
            message: "Resource not found".to_string(),
            code: Some("NOT_FOUND".to_string()),
            slug: Some("resource-not-found".to_string()),
            request_id: Some("req_123".to_string()),
            suggested_fix: Some("Check the resource ID".to_string()),
            errors: None,
        };

        let display = format!("{}", error);
        assert!(display.contains("API error"));
        assert!(display.contains("Resource not found"));
        assert!(display.contains("404"));
    }

    #[test]
    fn test_invalid_input_error() {
        let error = ComposioError::InvalidInput("Invalid API key".to_string());
        let display = format!("{}", error);
        assert!(display.contains("Invalid input"));
        assert!(display.contains("Invalid API key"));
    }

    #[test]
    fn test_config_error() {
        let error = ComposioError::ConfigError("Invalid base URL".to_string());
        let display = format!("{}", error);
        assert!(display.contains("Configuration error"));
        assert!(display.contains("Invalid base URL"));
    }

    #[test]
    fn test_serialization_error_conversion() {
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json")
            .unwrap_err();
        let error: ComposioError = json_error.into();

        match error {
            ComposioError::SerializationError(_) => (),
            _ => panic!("Expected SerializationError"),
        }
    }

    #[test]
    fn test_is_retryable_for_rate_limit() {
        let error = ComposioError::ApiError {
            status: 429,
            message: "Rate limited".to_string(),
            code: None,
            slug: None,
            request_id: None,
            suggested_fix: None,
            errors: None,
        };

        assert!(error.is_retryable());
    }

    #[test]
    fn test_is_retryable_for_server_errors() {
        for status in [500, 502, 503, 504] {
            let error = ComposioError::ApiError {
                status,
                message: "Server error".to_string(),
                code: None,
                slug: None,
                request_id: None,
                suggested_fix: None,
                errors: None,
            };

            assert!(
                error.is_retryable(),
                "Status {} should be retryable",
                status
            );
        }
    }

    #[test]
    fn test_is_not_retryable_for_client_errors() {
        for status in [400, 401, 403, 404] {
            let error = ComposioError::ApiError {
                status,
                message: "Client error".to_string(),
                code: None,
                slug: None,
                request_id: None,
                suggested_fix: None,
                errors: None,
            };

            assert!(
                !error.is_retryable(),
                "Status {} should not be retryable",
                status
            );
        }
    }

    #[test]
    fn test_serialization_error_is_not_retryable() {
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json")
            .unwrap_err();
        let error: ComposioError = json_error.into();

        assert!(!error.is_retryable());
    }

    #[test]
    fn test_invalid_input_not_retryable() {
        let error = ComposioError::InvalidInput("Invalid API key".to_string());
        assert!(!error.is_retryable());
    }

    #[test]
    fn test_config_error_not_retryable() {
        let error = ComposioError::ConfigError("Invalid base URL".to_string());
        assert!(!error.is_retryable());
    }

    #[test]
    fn test_error_detail_deserialization() {
        let json = r#"{
            "field": "email",
            "message": "Invalid email format"
        }"#;

        let detail: ErrorDetail = serde_json::from_str(json).unwrap();
        assert_eq!(detail.field, Some("email".to_string()));
        assert_eq!(detail.message, "Invalid email format");
    }

    #[test]
    fn test_error_response_deserialization() {
        let json = r#"{
            "message": "Validation failed",
            "code": "VALIDATION_ERROR",
            "slug": "validation-failed",
            "status": 400,
            "request_id": "req_abc123",
            "suggested_fix": "Check your input parameters",
            "errors": [
                {
                    "field": "user_id",
                    "message": "User ID is required"
                }
            ]
        }"#;

        let response: ErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.message, "Validation failed");
        assert_eq!(response.code, Some("VALIDATION_ERROR".to_string()));
        assert_eq!(response.status, 400);
        assert!(response.errors.is_some());
        assert_eq!(response.errors.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_error_response_minimal_deserialization() {
        let json = r#"{
            "message": "Internal server error",
            "status": 500
        }"#;

        let response: ErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.message, "Internal server error");
        assert_eq!(response.status, 500);
        assert!(response.code.is_none());
        assert!(response.errors.is_none());
    }
}
