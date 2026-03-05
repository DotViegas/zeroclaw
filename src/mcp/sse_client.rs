// Composio MCP Client with SSE Support
//
// This client implements Server-Sent Events (SSE) parsing for Composio's MCP endpoint.
// Based on implementation provided by Composio team.
//
// Architecture:
// - Sends JSON-RPC 2.0 requests
// - Receives SSE responses (text/event-stream)
// - Parses "data:" lines containing JSON-RPC responses
// - Supports multiple events per response

use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

/// MCP client with SSE support
#[derive(Clone)]
pub struct McpClient {
    http: reqwest::Client,
    mcp_url: String,
    api_key: String,
    timeout: Duration,
}

impl McpClient {
    /// Create a new MCP client
    pub fn new(mcp_url: impl Into<String>, api_key: impl Into<String>) -> Result<Self> {
        let http = reqwest::Client::builder()
            .tcp_keepalive(Some(Duration::from_secs(30)))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(8)
            .build()
            .context("failed to build reqwest client")?;
        
        Ok(Self {
            http,
            mcp_url: mcp_url.into(),
            api_key: api_key.into(),
            timeout: Duration::from_secs(180),
        })
    }

    /// Set custom timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Build headers for MCP requests
    fn headers(&self) -> Result<HeaderMap> {
        let mut h = HeaderMap::new();
        h.insert("x-api-key", HeaderValue::from_str(&self.api_key)?);
        h.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        // Server may ignore and return SSE anyway; still good to declare
        h.insert(
            ACCEPT,
            HeaderValue::from_static("text/event-stream, application/json"),
        );
        Ok(h)
    }

    /// Call MCP initialize method
    pub async fn initialize(&self) -> Result<Value> {
        let req = JsonRpcRequest::new(
            1,
            "initialize",
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "zeroclaw",
                    "version": "0.1.0"
                }
            }),
        );
        self.post_jsonrpc_collect_last(req).await
    }

    /// List available tools
    pub async fn tools_list(&self, id: i64) -> Result<Value> {
        let req = JsonRpcRequest::new(id, "tools/list", serde_json::json!({}));
        self.post_jsonrpc_collect_last(req).await
    }

    /// Call a tool
    pub async fn tools_call(&self, id: i64, name: &str, arguments: Value) -> Result<Value> {
        let req = JsonRpcRequest::new(
            id,
            "tools/call",
            serde_json::json!({
                "name": name,
                "arguments": arguments
            }),
        );
        self.post_jsonrpc_collect_last(req).await
    }

    /// Post JSON-RPC request and collect SSE events; returns the LAST `data:` JSON parseable.
    /// Useful for "single-shot" responses that come inside SSE.
    pub async fn post_jsonrpc_collect_last(&self, req: JsonRpcRequest) -> Result<Value> {
        let headers = self.headers()?;
        
        tracing::debug!(
            method = req.method,
            id = req.id,
            "Sending MCP request"
        );
        
        let resp = self
            .http
            .post(&self.mcp_url)
            .headers(headers)
            .timeout(self.timeout)
            .json(&req)
            .send()
            .await
            .context("mcp request send failed")?;

        let status = resp.status();
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("<missing>")
            .to_string();

        tracing::debug!(
            status = %status,
            content_type = %content_type,
            "Received MCP response headers"
        );

        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!(
                "mcp http error: status={} content-type={} body={}",
                status,
                content_type,
                body
            ));
        }

        // Even if application/json, some servers still stream/chunk.
        // Support both:
        if content_type.contains("text/event-stream") {
            self.read_sse_last_json(resp).await
        } else {
            let v: Value = resp.json().await.context("failed to parse json body")?;
            Ok(v)
        }
    }

    /// Read SSE stream and return the last JSON payload
    async fn read_sse_last_json(&self, resp: reqwest::Response) -> Result<Value> {
        let mut stream = resp.bytes_stream();
        
        // SSE parser "good enough":
        // accumulate text until finding "\n\n" (end of an SSE event)
        let mut buf = Vec::<u8>::new();
        let mut last_json: Option<Value> = None;

        while let Some(item) = stream.next().await {
            let chunk = item.context("error while reading sse chunk")?;
            buf.extend_from_slice(&chunk);

            while let Some(event_end) = find_double_newline(&buf) {
                let event_bytes: Vec<u8> = buf.drain(..event_end).collect();
                // drain the 2 '\n' as well
                let _ = buf.drain(..2);

                let event_str = String::from_utf8_lossy(&event_bytes);
                
                for line in event_str.lines() {
                    // Common MCP format: `data: {...}`
                    if let Some(data) = line.strip_prefix("data:") {
                        let data = data.trim();
                        if data.is_empty() {
                            continue;
                        }
                        
                        // Some servers may send "data: [DONE]" etc.
                        if data == "[DONE]" {
                            // Doesn't necessarily close connection; ignore
                            continue;
                        }
                        
                        // Try to parse as JSON
                        match serde_json::from_str::<Value>(data) {
                            Ok(v) => {
                                tracing::trace!(
                                    json_preview = %format!("{:.100}", v.to_string()),
                                    "Parsed SSE data event"
                                );
                                last_json = Some(v);
                            }
                            Err(e) => {
                                tracing::trace!(
                                    error = %e,
                                    data_preview = %format!("{:.100}", data),
                                    "Failed to parse SSE data as JSON"
                                );
                                // data may be fragmented or non-JSON; ignore or log as needed
                            }
                        }
                    }
                }
            }
        }

        last_json.ok_or_else(|| anyhow!("no JSON payload found in SSE stream"))
    }
}

/// Find the position of the first "\n\n" in the buffer.
/// Returns the index of the first '\n' of the separator.
fn find_double_newline(buf: &[u8]) -> Option<usize> {
    buf.windows(2).position(|w| w == b"\n\n")
}

/// JSON-RPC 2.0 request
#[derive(Debug, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: &'static str,
    pub id: i64,
    pub method: String,
    pub params: Value,
}

impl JsonRpcRequest {
    pub fn new(id: i64, method: impl Into<String>, params: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            method: method.into(),
            params,
        }
    }
}

/// JSON-RPC 2.0 response
#[derive(Debug, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: Option<String>,
    pub id: Option<i64>,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error
#[derive(Debug, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Feature: composio-permanent-integration
    // Task 30.2: Unit tests for MCP client (call, list_tools, health_check)

    #[test]
    fn test_find_double_newline() {
        let buf = b"event: message\ndata: {}\n\n";
        assert_eq!(find_double_newline(buf), Some(26));

        let buf = b"no double newline";
        assert_eq!(find_double_newline(buf), None);

        let buf = b"\n\n";
        assert_eq!(find_double_newline(buf), Some(0));
    }

    #[test]
    fn test_find_double_newline_multiple_occurrences() {
        let buf = b"data: first\n\ndata: second\n\n";
        assert_eq!(find_double_newline(buf), Some(11)); // First occurrence
    }

    #[test]
    fn test_find_double_newline_at_end() {
        let buf = b"data: test\n\n";
        assert_eq!(find_double_newline(buf), Some(10));
    }

    #[test]
    fn test_jsonrpc_request_serialization() {
        let req = JsonRpcRequest::new(1, "test/method", serde_json::json!({"key": "value"}));
        let json = serde_json::to_string(&req).unwrap();
        
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"method\":\"test/method\""));
    }

    #[test]
    fn test_jsonrpc_request_with_empty_params() {
        let req = JsonRpcRequest::new(42, "empty/method", serde_json::json!({}));
        let json = serde_json::to_string(&req).unwrap();
        
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":42"));
        assert!(json.contains("\"method\":\"empty/method\""));
        assert!(json.contains("\"params\":{}"));
    }

    #[test]
    fn test_jsonrpc_request_with_array_params() {
        let req = JsonRpcRequest::new(3, "array/method", serde_json::json!([1, 2, 3]));
        let json = serde_json::to_string(&req).unwrap();
        
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":3"));
        assert!(json.contains("\"params\":[1,2,3]"));
    }

    #[test]
    fn test_jsonrpc_response_deserialization_success() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"status":"ok"}}"#;
        let response: JsonRpcResponse = serde_json::from_str(json).unwrap();
        
        assert_eq!(response.jsonrpc, Some("2.0".to_string()));
        assert_eq!(response.id, Some(1));
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_jsonrpc_response_deserialization_error() {
        let json = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid Request"}}"#;
        let response: JsonRpcResponse = serde_json::from_str(json).unwrap();
        
        assert_eq!(response.jsonrpc, Some("2.0".to_string()));
        assert_eq!(response.id, Some(1));
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        
        let error = response.error.unwrap();
        assert_eq!(error.code, -32600);
        assert_eq!(error.message, "Invalid Request");
    }

    #[test]
    fn test_jsonrpc_response_deserialization_with_error_data() {
        let json = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32602,"message":"Invalid params","data":{"field":"missing"}}}"#;
        let response: JsonRpcResponse = serde_json::from_str(json).unwrap();
        
        let error = response.error.unwrap();
        assert_eq!(error.code, -32602);
        assert_eq!(error.message, "Invalid params");
        assert!(error.data.is_some());
    }

    #[test]
    fn test_jsonrpc_response_deserialization_minimal() {
        // Minimal valid response (optional fields missing)
        let json = r#"{"id":1,"result":null}"#;
        let response: JsonRpcResponse = serde_json::from_str(json).unwrap();
        
        assert_eq!(response.id, Some(1));
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_mcp_client_creation() {
        let client = McpClient::new("https://mcp.composio.dev", "test_api_key");
        assert!(client.is_ok());
        
        let client = client.unwrap();
        assert_eq!(client.mcp_url, "https://mcp.composio.dev");
        assert_eq!(client.api_key, "test_api_key");
        assert_eq!(client.timeout, Duration::from_secs(180));
    }

    #[test]
    fn test_mcp_client_with_custom_timeout() {
        let client = McpClient::new("https://mcp.composio.dev", "test_api_key")
            .unwrap()
            .with_timeout(Duration::from_secs(60));
        
        assert_eq!(client.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_mcp_client_headers() {
        let client = McpClient::new("https://mcp.composio.dev", "test_api_key").unwrap();
        let headers = client.headers().unwrap();
        
        assert_eq!(headers.get("x-api-key").unwrap(), "test_api_key");
        assert_eq!(headers.get("content-type").unwrap(), "application/json");
        assert!(headers.get("accept").is_some());
    }

    #[test]
    fn test_mcp_client_headers_with_special_characters() {
        let client = McpClient::new("https://mcp.composio.dev", "key_with_special!@#").unwrap();
        let headers = client.headers();
        
        // Should handle special characters in API key
        assert!(headers.is_ok());
    }

    #[test]
    fn test_jsonrpc_request_id_uniqueness() {
        let req1 = JsonRpcRequest::new(1, "method1", serde_json::json!({}));
        let req2 = JsonRpcRequest::new(2, "method2", serde_json::json!({}));
        
        assert_ne!(req1.id, req2.id);
    }

    #[test]
    fn test_jsonrpc_request_method_names() {
        let methods = vec!["initialize", "tools/list", "tools/call", "tools/get"];
        
        for (idx, method) in methods.iter().enumerate() {
            let req = JsonRpcRequest::new(idx as i64, *method, serde_json::json!({}));
            assert_eq!(req.method, *method);
        }
    }

    #[test]
    fn test_jsonrpc_error_deserialization_standard_codes() {
        // Test standard JSON-RPC error codes
        let error_codes = vec![
            (-32700, "Parse error"),
            (-32600, "Invalid Request"),
            (-32601, "Method not found"),
            (-32602, "Invalid params"),
            (-32603, "Internal error"),
        ];
        
        for (code, message) in error_codes {
            let json = format!(
                r#"{{"jsonrpc":"2.0","id":1,"error":{{"code":{},"message":"{}"}}}}"#,
                code, message
            );
            let response: JsonRpcResponse = serde_json::from_str(&json).unwrap();
            
            let error = response.error.unwrap();
            assert_eq!(error.code, code);
            assert_eq!(error.message, message);
        }
    }

    #[test]
    fn test_mcp_client_timeout_configuration() {
        let timeouts = vec![
            Duration::from_secs(30),
            Duration::from_secs(60),
            Duration::from_secs(180),
            Duration::from_secs(300),
        ];
        
        for timeout in timeouts {
            let client = McpClient::new("https://mcp.composio.dev", "test_key")
                .unwrap()
                .with_timeout(timeout);
            assert_eq!(client.timeout, timeout);
        }
    }
}
