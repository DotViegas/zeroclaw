# Changelog

All notable changes to the Composio Rust SDK will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2026-03-06

### Fixed
- Corrected repository URL in Cargo.toml to https://github.com/DotViegas/composio-sdk-rust
- Updated all documentation links to point to correct repository
- Fixed author information in package metadata

### Changed
- Updated README with correct repository and author information
- Improved README with comprehensive architecture explanation
- Added architecture diagram reference

## [0.1.0] - 2026-03-05

### Added

#### Core Infrastructure
- **ComposioClient**: Main SDK entry point with builder pattern for flexible configuration
- **Session Management**: User-scoped sessions for data isolation and tool access control
- **HTTP Client**: Async reqwest-based client with connection pooling and timeout configuration
- **Configuration**: Sensible defaults with customizable base URL, timeout, and retry policies
- **Retry Logic**: Exponential backoff for transient failures (429, 5xx status codes)

#### Data Models
- **Request Models**: Type-safe structs for SessionConfig, ToolExecutionRequest, MetaToolExecutionRequest, LinkRequest
- **Response Models**: Comprehensive response types for all API endpoints
- **Enums**: MetaToolSlug, TagType, AuthScheme with proper serialization

#### Session Operations
- Session creation with toolkit filtering (enable/disable lists)
- Session retrieval by ID
- Custom auth config overrides per toolkit
- Connected account selection per toolkit
- Connection management configuration
- Tool and tag-based filtering
- Workbench configuration support

#### Tool Execution
- Regular tool execution with automatic authentication
- Meta tool execution (all 5 meta tools supported):
  - COMPOSIO_SEARCH_TOOLS - Discover relevant tools
  - COMPOSIO_MULTI_EXECUTE_TOOL - Parallel tool execution
  - COMPOSIO_MANAGE_CONNECTIONS - Connection management
  - COMPOSIO_REMOTE_WORKBENCH - Python sandbox execution
  - COMPOSIO_REMOTE_BASH_TOOL - Bash command execution

#### Additional Features
- Toolkit listing with pagination and filtering
- Meta tools schema retrieval
- Authentication link creation for OAuth flows
- Connection status checking

#### Error Handling
- Comprehensive ComposioError enum with variants:
  - ApiError (with status, message, code, slug, request_id, suggested_fix)
  - NetworkError (reqwest errors)
  - SerializationError (JSON errors)
  - InvalidInput (validation errors)
  - ConfigError (configuration errors)
- Automatic error conversion from HTTP responses
- Retryable error detection

#### Documentation
- Complete rustdoc documentation for all public APIs
- README with quickstart guide and examples
- 7 working examples demonstrating core functionality:
  - basic_usage.rs - Session creation and tool execution
  - meta_tools.rs - All meta tools usage
  - error_handling.rs - Error handling patterns
  - authentication.rs - Auth flows (in-chat and manual)
  - toolkit_listing.rs - Listing and filtering toolkits
  - tool_execution.rs - Tool execution patterns
  - wizard_instructions.rs - Wizard instruction generation

#### Testing
- Comprehensive unit tests for all modules
- Integration tests with mock HTTP server (wiremock)
- Compatibility validation tests against Python SDK
- Error handling integration tests
- Retry behavior tests

#### Performance & Memory
- Memory footprint: ~2 MB (measured with cargo bloat)
- Library size: 2.45 MB (release build)
- Runtime overhead: 112 bytes (client) + 296 bytes (session builder)
- Initialization time: ~200 µs
- Zero-copy deserialization where possible
- Efficient Arc-based sharing for client references

#### Skills Integration
- SkillsExtractor for parsing Composio Skills repository
- WizardInstructionGenerator for creating context-aware instructions
- InstructionValidator for checking against official patterns
- Build-time Skills repository integration
- Support for toolkit-specific and generic instructions

### Security
- Path traversal prevention in file operations
- Secure credential handling
- API key validation
- HTTPS-only connections

### Compatibility
- Full Python SDK Tool Router API compatibility
- JSON serialization/deserialization parity
- Identical request/response formats
- Compatible error handling patterns

### Performance Benchmarks
- Session creation: ~150ms (network dependent)
- Tool execution: ~200ms (network dependent)
- Client initialization: ~200µs
- Memory usage: 112 bytes (client) + 296 bytes (session builder)

### Dependencies
- reqwest 0.11 (HTTP client with JSON support)
- serde 1.0 (serialization framework)
- serde_json 1.0 (JSON support)
- tokio 1.0 (async runtime)
- tokio-retry 0.3 (retry logic)
- thiserror 1.0 (error handling)

### Development Dependencies
- wiremock 0.5 (HTTP mocking for tests)
- tokio-test 0.4 (async test utilities)
- criterion 0.5 (benchmarking)

### Notes
- This is the initial release focusing on Tool Router API
- Designed for minimal memory footprint (~2 MB)
- Built for ZeroClaw integration (lightweight Rust AI assistant)
- Follows Composio Python SDK design patterns
- Requires Rust 1.70 or later
- Requires tokio async runtime

### Known Limitations
- Direct tool execution (non-session) not yet supported
- Triggers support not yet implemented
- Connected accounts management API not yet implemented
- Auth configs management API not yet implemented
- File upload/download not yet implemented
- Workbench mount operations not yet implemented

### Future Roadmap
- Direct tool execution support
- Triggers and webhooks
- Connected accounts management
- Auth configs management
- File operations
- Workbench mount operations
- Additional meta tools as they become available

[0.1.1]: https://github.com/DotViegas/composio-sdk-rust/releases/tag/v0.1.1
[0.1.0]: https://github.com/DotViegas/composio-sdk-rust/releases/tag/v0.1.0
