# Release Guide for Composio Rust SDK v0.1.0

This guide walks you through the process of releasing the Composio Rust SDK as a separate repository and publishing it to crates.io.

## Prerequisites

- [x] Version set to 0.1.0 in Cargo.toml
- [x] CHANGELOG.md created with release notes
- [x] All tests passing
- [x] Documentation complete
- [x] Examples working
- [ ] GitHub repository created: https://github.com/DotViegas/composio-sdk-rust
- [ ] crates.io account with API token

## Step 1: Extract SDK to Separate Directory

Since the SDK is currently in the `composio-sdk/` subdirectory of the ZeroClaw project, you need to extract it to a separate location:

```bash
# From the root of the ZeroClaw project
cd ..
cp -r zeroclaw/composio-sdk composio-sdk-rust
cd composio-sdk-rust
```

Or on Windows:
```powershell
cd ..
Copy-Item -Recurse zeroclaw/composio-sdk composio-sdk-rust
cd composio-sdk-rust
```

## Step 2: Initialize Git Repository

Run the setup script to initialize the repository:

**On Linux/macOS:**
```bash
chmod +x setup-repo.sh
./setup-repo.sh
```

**On Windows:**
```powershell
.\setup-repo.ps1
```

This script will:
- Initialize a new git repository
- Create .gitignore
- Add all files
- Create initial commit
- Set default branch to main
- Add remote origin
- Create v0.1.0 tag

## Step 3: Push to GitHub

After the script completes, push to GitHub:

```bash
# Push main branch
git push -u origin main

# Push tags
git push --tags
```

## Step 4: Create GitHub Release

1. Go to: https://github.com/DotViegas/composio-sdk-rust/releases/new
2. Select tag: `v0.1.0`
3. Release title: `v0.1.0 - Initial Release`
4. Copy the release notes from CHANGELOG.md
5. Add highlights:

```markdown
# Composio Rust SDK v0.1.0

Initial release of the Composio Rust SDK - a minimal, type-safe SDK for the Composio Tool Router REST API.

## 🎉 Highlights

- **Type-Safe**: Compile-time validation with Rust's type system
- **Async/Await**: Built on tokio for high-performance async operations
- **Minimal Footprint**: ~2 MB memory overhead
- **Comprehensive**: Full Tool Router API coverage
- **Well-Documented**: Complete rustdoc + 7 working examples

## 📦 Installation

Add to your `Cargo.toml`:
```toml
[dependencies]
composio-sdk = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
```

## 🚀 Quick Start

```rust
use composio_sdk::{ComposioClient, ComposioError};

#[tokio::main]
async fn main() -> Result<(), ComposioError> {
    let client = ComposioClient::builder()
        .api_key(std::env::var("COMPOSIO_API_KEY")?)
        .build()?;

    let session = client
        .create_session("user_123")
        .toolkits(vec!["github", "gmail"])
        .send()
        .await?;

    println!("Session ID: {}", session.session_id());
    Ok(())
}
```

## 📚 Documentation

- [API Documentation](https://docs.rs/composio-sdk)
- [Examples](https://github.com/DotViegas/composio-sdk-rust/tree/main/examples)
- [Composio Platform Docs](https://docs.composio.dev)

## 🔗 Links

- [Crates.io](https://crates.io/crates/composio-sdk)
- [Documentation](https://docs.rs/composio-sdk)
- [Repository](https://github.com/DotViegas/composio-sdk-rust)

## 🙏 Acknowledgments

Built for [ZeroClaw](https://github.com/zeroclaw) and follows design patterns from the official [Composio Python SDK](https://github.com/ComposioHQ/composio).
```

6. Click "Publish release"

## Step 5: Publish to crates.io

### 5.1 Get crates.io API Token

1. Go to https://crates.io/settings/tokens
2. Create a new token with name "composio-sdk-publish"
3. Copy the token

### 5.2 Login to crates.io

```bash
cargo login <your-token>
```

### 5.3 Verify Package

Before publishing, verify the package:

```bash
# Check what will be published
cargo package --list

# Build the package
cargo package

# Test the package locally
cargo package --no-verify
cd target/package/composio-sdk-0.1.0
cargo test
cd ../../..
```

### 5.4 Publish

```bash
cargo publish
```

If you get an error about missing dependencies, you may need to publish with `--allow-dirty`:

```bash
cargo publish --allow-dirty
```

### 5.5 Verify Publication

After publishing, verify at:
- https://crates.io/crates/composio-sdk
- https://docs.rs/composio-sdk

Note: docs.rs may take a few minutes to build the documentation.

## Step 6: Update README with Installation Instructions

The README already includes installation instructions for the published crate:

```toml
[dependencies]
composio-sdk = "0.1.0"
```

No changes needed!

## Step 7: Announce Release

Consider announcing the release:

1. **Composio Discord**: Share in the community channel
2. **Reddit**: r/rust
3. **Twitter/X**: Tag @composiohq
4. **LinkedIn**: Share with your network

Example announcement:

```
🎉 Excited to announce the first release of composio-sdk - a minimal, type-safe Rust SDK for @composiohq Tool Router API!

✨ Features:
- Type-safe with compile-time validation
- Async/await with tokio
- ~2 MB memory footprint
- Full Tool Router API coverage
- Comprehensive docs + examples

Built for ZeroClaw, a lightweight Rust AI assistant.

📦 crates.io/crates/composio-sdk
📚 docs.rs/composio-sdk
🔗 github.com/DotViegas/composio-sdk-rust

#rustlang #ai #opensource
```

## Troubleshooting

### Issue: "repository not found"

Make sure you've created the GitHub repository first:
1. Go to https://github.com/new
2. Repository name: `composio-sdk-rust`
3. Description: "Minimal, type-safe Rust SDK for Composio Tool Router REST API"
4. Public repository
5. Don't initialize with README (we already have one)
6. Create repository

### Issue: "failed to publish"

Common reasons:
- Crate name already taken (check crates.io)
- Missing required fields in Cargo.toml (already complete)
- Network issues (retry)
- Need to verify email on crates.io

### Issue: "docs.rs build failed"

- Check the build logs at docs.rs
- Usually resolves automatically after a few minutes
- May need to add `[package.metadata.docs.rs]` section to Cargo.toml

## Post-Release Checklist

- [ ] GitHub repository created and pushed
- [ ] GitHub release created with notes
- [ ] Published to crates.io
- [ ] Verified on crates.io
- [ ] Verified docs.rs build
- [ ] Updated README (already done)
- [ ] Announced release
- [ ] Added badges to README (already done)

## Next Steps

After the initial release:

1. **Monitor Issues**: Watch for bug reports and feature requests
2. **Community Engagement**: Respond to questions and feedback
3. **Future Releases**: Plan v0.2.0 with additional features
4. **Integration**: Test with real-world applications
5. **Performance**: Continue optimizing memory and speed

## Version Numbering

Following Semantic Versioning (semver):
- v0.1.0 - Initial release
- v0.1.x - Bug fixes
- v0.2.0 - New features (backward compatible)
- v1.0.0 - Stable API (production ready)

## Support

For questions or issues:
- GitHub Issues: https://github.com/DotViegas/composio-sdk-rust/issues
- Composio Discord: https://discord.gg/composio
- Email: support@composio.dev

---

**Congratulations on releasing v0.1.0! 🎉**
