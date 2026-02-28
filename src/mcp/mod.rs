// MCP (Model Context Protocol) integration module
//
// This module provides MCP client implementations for various services.
// Currently supports:
// - Composio MCP (1000+ OAuth apps)

pub mod composio_client;
pub mod sse_client;

pub use composio_client::{ComposioMcpClient, McpTool};
pub use sse_client::{McpClient, JsonRpcRequest, JsonRpcResponse, JsonRpcError};
