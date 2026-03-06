//! Performance benchmarks for Composio Rust SDK.
//!
//! Benchmarks cover:
//!   - Session creation time
//!   - Tool execution time
//!   - Retry logic overhead
//!   - HTTP client performance
//!   - JSON serialization/deserialization
//!
//! Run: `cargo bench --package composio-sdk`
//!
//! These benchmarks help ensure the SDK maintains minimal overhead
//! and meets the ≤2 MB memory footprint requirement.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use composio_sdk::{
    ComposioClient, SessionConfig, ToolkitFilter, MetaToolSlug,
    ToolExecutionRequest, MetaToolExecutionRequest,
    models::ManageConnectionsConfig,
};
use serde_json::json;
use std::time::Duration;
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, header};

// ─────────────────────────────────────────────────────────────────────────────
// Helper: Create mock server with session creation endpoint
// ─────────────────────────────────────────────────────────────────────────────

async fn setup_mock_server() -> MockServer {
    let mock_server = MockServer::start().await;
    
    // Mock session creation endpoint
    Mock::given(method("POST"))
        .and(path("/tool_router/session"))
        .and(header("x-api-key", "test_key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "session_id": "sess_benchmark_123",
            "mcp": {
                "url": "https://mcp.composio.dev/sess_benchmark_123"
            },
            "tool_router_tools": [
                {
                    "slug": "COMPOSIO_SEARCH_TOOLS",
                    "name": "Search Tools",
                    "description": "Search for tools",
                    "toolkit": {
                        "slug": "composio",
                        "name": "Composio"
                    },
                    "input_parameters": {},
                    "output_parameters": {},
                    "scopes": [],
                    "tags": [],
                    "version": "1.0.0",
                    "available_versions": ["1.0.0"],
                    "is_deprecated": false,
                    "no_auth": false
                }
            ],
            "config": {
                "user_id": "user_benchmark"
            }
        })))
        .mount(&mock_server)
        .await;
    
    // Mock tool execution endpoint
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_benchmark_123/execute"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {"result": "success"},
            "error": null,
            "log_id": "log_123"
        })))
        .mount(&mock_server)
        .await;
    
    // Mock meta tool execution endpoint
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_benchmark_123/execute_meta"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {"tools": []},
            "error": null,
            "log_id": "log_456"
        })))
        .mount(&mock_server)
        .await;
    
    // Mock retry scenarios - 429 then success
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_retry_test/execute"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_json(json!({
                    "message": "Rate limited",
                    "status": 429
                }))
        )
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;
    
    Mock::given(method("POST"))
        .and(path("/tool_router/session/sess_retry_test/execute"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {"result": "success after retry"},
            "error": null,
            "log_id": "log_retry"
        })))
        .mount(&mock_server)
        .await;
    
    mock_server
}

// ─────────────────────────────────────────────────────────────────────────────
// Benchmark 8.5.1: Session creation time
// ─────────────────────────────────────────────────────────────────────────────

fn bench_session_creation(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("session_creation_minimal", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mock_server = setup_mock_server().await;
                let client = ComposioClient::builder()
                    .api_key("test_key")
                    .base_url(mock_server.uri())
                    .build()
                    .unwrap();
                
                let session = client
                    .create_session(black_box("user_benchmark"))
                    .send()
                    .await
                    .unwrap();
                
                black_box(session)
            })
        });
    });
    
    c.bench_function("session_creation_with_toolkits", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mock_server = setup_mock_server().await;
                let client = ComposioClient::builder()
                    .api_key("test_key")
                    .base_url(mock_server.uri())
                    .build()
                    .unwrap();
                
                let session = client
                    .create_session(black_box("user_benchmark"))
                    .toolkits(vec!["github", "gmail", "slack"])
                    .send()
                    .await
                    .unwrap();
                
                black_box(session)
            })
        });
    });
    
    c.bench_function("session_creation_with_config", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mock_server = setup_mock_server().await;
                let client = ComposioClient::builder()
                    .api_key("test_key")
                    .base_url(mock_server.uri())
                    .build()
                    .unwrap();
                
                let session = client
                    .create_session(black_box("user_benchmark"))
                    .toolkits(vec!["github", "gmail"])
                    .manage_connections(true)
                    .send()
                    .await
                    .unwrap();
                
                black_box(session)
            })
        });
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Benchmark 8.5.2: Tool execution time
// ─────────────────────────────────────────────────────────────────────────────

fn bench_tool_execution(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    // Setup: Create session once for all tool execution benchmarks
    let (_client, session) = rt.block_on(async {
        let mock_server = setup_mock_server().await;
        let client = ComposioClient::builder()
            .api_key("test_key")
            .base_url(mock_server.uri())
            .build()
            .unwrap();
        
        let session = client
            .create_session("user_benchmark")
            .send()
            .await
            .unwrap();
        
        (client, session)
    });
    
    c.bench_function("tool_execution_simple", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = session
                    .execute_tool(
                        black_box("GITHUB_GET_REPOS"),
                        black_box(json!({"owner": "composio"}))
                    )
                    .await
                    .unwrap();
                
                black_box(result)
            })
        });
    });
    
    c.bench_function("tool_execution_complex_args", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = session
                    .execute_tool(
                        black_box("GITHUB_CREATE_ISSUE"),
                        black_box(json!({
                            "owner": "composio",
                            "repo": "composio",
                            "title": "Benchmark test issue",
                            "body": "This is a benchmark test with complex arguments",
                            "labels": ["benchmark", "test", "performance"],
                            "assignees": ["user1", "user2"]
                        }))
                    )
                    .await
                    .unwrap();
                
                black_box(result)
            })
        });
    });
    
    c.bench_function("meta_tool_execution_search", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = session
                    .execute_meta_tool(
                        black_box(MetaToolSlug::ComposioSearchTools),
                        black_box(json!({"query": "create github issue"}))
                    )
                    .await
                    .unwrap();
                
                black_box(result)
            })
        });
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Benchmark 8.5.3: Retry logic overhead
// ─────────────────────────────────────────────────────────────────────────────

fn bench_retry_logic(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("retry_no_failure", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mock_server = setup_mock_server().await;
                let client = ComposioClient::builder()
                    .api_key("test_key")
                    .base_url(mock_server.uri())
                    .max_retries(3)
                    .initial_retry_delay(Duration::from_millis(10))
                    .build()
                    .unwrap();
                
                let session = client
                    .create_session("user_benchmark")
                    .send()
                    .await
                    .unwrap();
                
                let result = session
                    .execute_tool("GITHUB_GET_REPOS", json!({"owner": "composio"}))
                    .await
                    .unwrap();
                
                black_box(result)
            })
        });
    });
    
    c.bench_function("retry_with_backoff", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mock_server = setup_mock_server().await;
                
                // Create session that will trigger retries
                Mock::given(method("POST"))
                    .and(path("/tool_router/session"))
                    .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                        "session_id": "sess_retry_test",
                        "mcp": {"url": "https://mcp.composio.dev/sess_retry_test"},
                        "tool_router_tools": [],
                        "config": {"user_id": "user_benchmark"}
                    })))
                    .mount(&mock_server)
                    .await;
                
                let client = ComposioClient::builder()
                    .api_key("test_key")
                    .base_url(mock_server.uri())
                    .max_retries(3)
                    .initial_retry_delay(Duration::from_millis(10))
                    .max_retry_delay(Duration::from_millis(100))
                    .build()
                    .unwrap();
                
                let session = client
                    .create_session("user_benchmark")
                    .send()
                    .await
                    .unwrap();
                
                // This will retry 2 times before succeeding
                let result = session
                    .execute_tool("GITHUB_GET_REPOS", json!({"owner": "composio"}))
                    .await
                    .unwrap();
                
                black_box(result)
            })
        });
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Additional benchmarks: JSON serialization/deserialization
// ─────────────────────────────────────────────────────────────────────────────

fn bench_json_operations(c: &mut Criterion) {
    let session_config = SessionConfig {
        user_id: "user_benchmark".to_string(),
        toolkits: Some(ToolkitFilter::Enable(vec![
            "github".to_string(),
            "gmail".to_string(),
            "slack".to_string(),
        ])),
        auth_configs: None,
        connected_accounts: None,
        manage_connections: Some(ManageConnectionsConfig::Bool(true)),
        tools: None,
        tags: None,
        workbench: None,
    };
    
    c.bench_function("json_serialize_session_config", |b| {
        b.iter(|| {
            let json = serde_json::to_string(black_box(&session_config)).unwrap();
            black_box(json)
        });
    });
    
    let tool_request = ToolExecutionRequest {
        tool_slug: "GITHUB_CREATE_ISSUE".to_string(),
        arguments: Some(json!({
            "owner": "composio",
            "repo": "composio",
            "title": "Test issue",
            "body": "Test body"
        })),
    };
    
    c.bench_function("json_serialize_tool_request", |b| {
        b.iter(|| {
            let json = serde_json::to_string(black_box(&tool_request)).unwrap();
            black_box(json)
        });
    });
    
    let meta_tool_request = MetaToolExecutionRequest {
        slug: MetaToolSlug::ComposioSearchTools,
        arguments: Some(json!({"query": "create github issue"})),
    };
    
    c.bench_function("json_serialize_meta_tool_request", |b| {
        b.iter(|| {
            let json = serde_json::to_string(black_box(&meta_tool_request)).unwrap();
            black_box(json)
        });
    });
    
    let response_json = r#"{
        "data": {"result": "success", "issue_number": 123},
        "error": null,
        "log_id": "log_abc123"
    }"#;
    
    c.bench_function("json_deserialize_tool_response", |b| {
        b.iter(|| {
            let response: composio_sdk::ToolExecutionResponse = 
                serde_json::from_str(black_box(response_json)).unwrap();
            black_box(response)
        });
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Benchmark groups
// ─────────────────────────────────────────────────────────────────────────────

criterion_group!(
    benches,
    bench_session_creation,
    bench_tool_execution,
    bench_retry_logic,
    bench_json_operations,
);

criterion_main!(benches);
