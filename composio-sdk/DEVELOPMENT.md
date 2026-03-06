# Development Guide

This guide helps you set up and work with the Composio Rust SDK locally.

## Prerequisites

- Rust 1.70 or later
- Composio API key ([get one here](https://app.composio.dev))
- Git (for cloning dependencies)

## Setup

1. **Clone the repository:**
   ```bash
   git clone https://github.com/composio/composio-rust-sdk.git
   cd composio-rust-sdk/composio-sdk
   ```

2. **Set your API key:**
   ```bash
   # Linux/macOS
   export COMPOSIO_API_KEY="your_api_key_here"
   
   # Windows PowerShell
   $env:COMPOSIO_API_KEY="your_api_key_here"
   ```

3. **Install dependencies:**
   ```bash
   cargo build
   ```

## Local Testing

### Quick Test
Run the comprehensive test suite:

```bash
# Linux/macOS
chmod +x test-local.sh
./test-local.sh

# Windows
.\test-local.ps1
```

### Individual Tests

**Unit tests:**
```bash
cargo test --lib
```

**Integration tests:**
```bash
cargo test --test '*'
```

**Specific test:**
```bash
cargo test test_session_creation
```

### Running Examples

**Basic usage:**
```bash
cargo run --example basic_usage
```

**With debug logging:**
```bash
cargo run --example local_debug --features local-debug
```

**All examples:**
```bash
# List available examples
ls examples/

# Run any example
cargo run --example <example_name>
```

## Debugging

### Enable Debug Logging

Use the `local-debug` feature for detailed logging:

```bash
cargo run --example local_debug --features local-debug
```

This enables:
- Request/response logging
- Timing information
- Detailed error inspection
- Network debugging

### Debug Specific Components

**Client initialization:**
```bash
cargo run --example test_api_key
```

**Session management:**
```bash
cargo run --example test_simple_session
```

**Tool execution:**
```bash
cargo run --example tool_execution
```

**Error handling:**
```bash
cargo run --example error_handling
```

### Memory Profiling

Check memory usage:
```bash
cargo run --example memory_profile
```

### Performance Benchmarking

Run benchmarks:
```bash
cargo bench
```

## Development Workflow

### 1. Make Changes

Edit source files in `src/`:
- `client.rs` - HTTP client and main API
- `session.rs` - Session management
- `models/` - Request/response types
- `error.rs` - Error handling
- `config.rs` - Configuration

### 2. Run Tests

```bash
# Quick check
cargo test

# With coverage (requires cargo-tarpaulin)
cargo tarpaulin --out Html
```

### 3. Check Code Quality

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Run linter
cargo clippy

# Fix clippy warnings
cargo clippy --fix
```

### 4. Update Documentation

```bash
# Build docs
cargo doc --no-deps

# Open docs in browser
cargo doc --open

# Check for broken links
cargo doc --no-deps 2>&1 | grep warning
```

### 5. Test Against Real API

Create a test script:

```rust
// test_real_api.rs
use composio_sdk::ComposioClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ComposioClient::builder()
        .api_key(std::env::var("COMPOSIO_API_KEY")?)
        .build()?;

    let session = client
        .create_session("test_user")
        .toolkits(vec!["github"])
        .send()
        .await?;

    println!("Session created: {}", session.session_id());
    Ok(())
}
```

Run it:
```bash
cargo run --bin test_real_api
```

## Troubleshooting

### API Key Issues

**Error: "Invalid API key"**
- Verify your API key is correct
- Check it's properly set in environment
- Try regenerating the key in Composio dashboard

**Test:**
```bash
cargo run --example test_api_key
```

### Network Issues

**Error: "Connection refused"**
- Check internet connection
- Verify Composio API is accessible
- Check firewall settings

**Debug:**
```bash
cargo run --example debug_request --features local-debug
```

### Serialization Issues

**Error: "Failed to deserialize response"**
- Check API response format hasn't changed
- Verify request payload structure
- Run serialization tests

**Test:**
```bash
cargo run --example test_serialization
```

### Build Issues

**Error: "Failed to compile"**
- Update Rust: `rustup update`
- Clean build: `cargo clean && cargo build`
- Check Cargo.toml dependencies

## Testing Against Python SDK

To ensure compatibility with the Python SDK:

1. **Compare request formats:**
   ```bash
   cargo run --example test_serialization
   ```

2. **Compare response handling:**
   ```bash
   cargo test compatibility_validation
   ```

3. **Check API coverage:**
   Review `COMPATIBILITY.md` for feature parity

## Contributing

Before submitting a PR:

1. ✅ Run full test suite: `./test-local.sh`
2. ✅ Format code: `cargo fmt`
3. ✅ Pass clippy: `cargo clippy`
4. ✅ Update documentation
5. ✅ Add tests for new features
6. ✅ Update CHANGELOG.md

## Useful Commands

```bash
# Watch for changes and run tests
cargo watch -x test

# Check dependencies
cargo tree

# Update dependencies
cargo update

# Check for outdated dependencies
cargo outdated

# Security audit
cargo audit

# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage

# Profile binary size
cargo bloat --release

# Check compile times
cargo build --timings
```

## IDE Setup

### VS Code

Install extensions:
- rust-analyzer
- CodeLLDB (for debugging)
- crates (dependency management)

### IntelliJ IDEA / CLion

Install Rust plugin for full IDE support.

## Resources

- [Composio Documentation](https://docs.composio.dev)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Tokio Documentation](https://tokio.rs)
- [Reqwest Documentation](https://docs.rs/reqwest)

## Getting Help

- 💬 [Discord Community](https://discord.gg/composio)
- 🐛 [GitHub Issues](https://github.com/composio/composio-rust-sdk/issues)
- 📧 Email: support@composio.dev
