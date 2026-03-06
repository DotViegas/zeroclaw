//! Tool Search Implementation
//!
//! Native Rust implementation of COMPOSIO_SEARCH_TOOLS meta tool.
//! Discovers relevant tools across 1000+ apps using natural language queries.

use crate::client::ComposioClient;
use crate::error::ComposioError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Tool search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Tool slug (e.g., "GITHUB_CREATE_ISSUE")
    pub slug: String,
    
    /// Human-readable tool name
    pub name: String,
    
    /// Tool description
    pub description: String,
    
    /// Toolkit this tool belongs to
    pub toolkit: String,
    
    /// Whether user has connected this toolkit
    pub is_connected: bool,
    
    /// Relevance score (0.0 - 1.0)
    pub score: f64,
    
    /// Tool input schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,
    
    /// Recommended execution plan
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_plan: Option<Vec<String>>,
    
    /// Known pitfalls and warnings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub known_pitfalls: Option<Vec<String>>,
}

/// Tool search implementation
pub struct ToolSearch {
    client: Arc<ComposioClient>,
}

impl ToolSearch {
    /// Create a new tool search instance
    ///
    /// # Arguments
    ///
    /// * `client` - Composio client instance
    ///
    /// # Example
    ///
    /// ```no_run
    /// use composio_sdk::{ComposioClient, meta_tools::ToolSearch};
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ComposioClient::builder()
    ///     .api_key("your-api-key")
    ///     .build()?;
    ///
    /// let search = ToolSearch::new(Arc::new(client));
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(client: Arc<ComposioClient>) -> Self {
        Self { client }
    }

    /// Search for tools using natural language query
    ///
    /// # Arguments
    ///
    /// * `query` - Natural language description of what you want to do
    /// * `session_id` - Session ID for context-aware search
    ///
    /// # Returns
    ///
    /// Vector of search results ordered by relevance
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::{ComposioClient, meta_tools::ToolSearch};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = Arc::new(ComposioClient::builder().api_key("key").build()?);
    /// let search = ToolSearch::new(client);
    /// let results = search.search("send email to user", "session_123").await?;
    ///
    /// for result in results {
    ///     println!("{}: {} ({})", result.slug, result.name, result.score);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search(
        &self,
        query: &str,
        session_id: &str,
    ) -> Result<Vec<SearchResult>, ComposioError> {
        let url = format!(
            "{}/tool_router/session/{}/search",
            self.client.base_url(),
            session_id
        );

        let response = self
            .client
            .http_client()
            .post(&url)
            .json(&serde_json::json!({
                "query": query,
                "include_schema": true,
                "include_plan": true,
            }))
            .send()
            .await
            .map_err(|e| ComposioError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ComposioError::ApiError {
                status: status.as_u16(),
                message: error_text,
                request_id: None,
                suggested_fix: None,
            });
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ComposioError::SerializationError(e.to_string()))?;

        // Parse search results
        let results = data["data"]["tools"]
            .as_array()
            .ok_or_else(|| {
                ComposioError::SerializationError("Invalid search response format".to_string())
            })?
            .iter()
            .filter_map(|tool| serde_json::from_value(tool.clone()).ok())
            .collect();

        Ok(results)
    }

    /// Search for tools with additional filters
    ///
    /// # Arguments
    ///
    /// * `query` - Natural language query
    /// * `session_id` - Session ID
    /// * `toolkits` - Optional list of toolkits to search within
    /// * `limit` - Maximum number of results (default: 10)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use composio_sdk::{ComposioClient, meta_tools::ToolSearch};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = Arc::new(ComposioClient::builder().api_key("key").build()?);
    /// let search = ToolSearch::new(client);
    /// let results = search.search_filtered(
    ///     "create issue",
    ///     "session_123",
    ///     Some(vec!["github", "linear"]),
    ///     Some(5),
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search_filtered(
        &self,
        query: &str,
        session_id: &str,
        toolkits: Option<Vec<&str>>,
        limit: Option<usize>,
    ) -> Result<Vec<SearchResult>, ComposioError> {
        let url = format!(
            "{}/tool_router/session/{}/search",
            self.client.base_url(),
            session_id
        );

        let mut body = serde_json::json!({
            "query": query,
            "include_schema": true,
            "include_plan": true,
        });

        if let Some(tk) = toolkits {
            body["toolkits"] = serde_json::json!(tk);
        }

        if let Some(lim) = limit {
            body["limit"] = serde_json::json!(lim);
        }

        let response = self
            .client
            .http_client()
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ComposioError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ComposioError::ApiError {
                status: status.as_u16(),
                message: error_text,
                request_id: None,
                suggested_fix: None,
            });
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ComposioError::SerializationError(e.to_string()))?;

        let results = data["data"]["tools"]
            .as_array()
            .ok_or_else(|| {
                ComposioError::SerializationError("Invalid search response format".to_string())
            })?
            .iter()
            .filter_map(|tool| serde_json::from_value(tool.clone()).ok())
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_result_serialization() {
        let result = SearchResult {
            slug: "GITHUB_CREATE_ISSUE".to_string(),
            name: "Create Issue".to_string(),
            description: "Create a new issue in a repository".to_string(),
            toolkit: "github".to_string(),
            is_connected: true,
            score: 0.95,
            input_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string" },
                    "body": { "type": "string" }
                }
            })),
            execution_plan: Some(vec![
                "Ensure GitHub is connected".to_string(),
                "Provide repository owner and name".to_string(),
            ]),
            known_pitfalls: Some(vec![
                "Title is required".to_string(),
            ]),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("GITHUB_CREATE_ISSUE"));
        assert!(json.contains("0.95"));

        let deserialized: SearchResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.slug, "GITHUB_CREATE_ISSUE");
        assert_eq!(deserialized.score, 0.95);
    }

    #[test]
    fn test_search_result_without_optional_fields() {
        let result = SearchResult {
            slug: "GMAIL_SEND_EMAIL".to_string(),
            name: "Send Email".to_string(),
            description: "Send an email".to_string(),
            toolkit: "gmail".to_string(),
            is_connected: false,
            score: 0.88,
            input_schema: None,
            execution_plan: None,
            known_pitfalls: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(!json.contains("input_schema"));
        assert!(!json.contains("execution_plan"));
        assert!(!json.contains("known_pitfalls"));
    }
}
