#!/bin/bash
# Local testing script for Composio SDK development

set -e

echo "🧪 Composio SDK Local Testing Suite"
echo "===================================="
echo ""

# Check if API key is set
if [ -z "$COMPOSIO_API_KEY" ]; then
    echo "❌ Error: COMPOSIO_API_KEY environment variable not set"
    echo "   Please set it with: export COMPOSIO_API_KEY=your_key_here"
    exit 1
fi

echo "✅ API key found"
echo ""

# 1. Run unit tests
echo "📝 Running unit tests..."
cargo test --lib
echo ""

# 2. Run integration tests
echo "🔗 Running integration tests..."
cargo test --test '*'
echo ""

# 3. Run examples with debug output
echo "🎯 Running debug example..."
cargo run --example local_debug --features local-debug
echo ""

# 4. Check code formatting
echo "🎨 Checking code formatting..."
cargo fmt --check
echo ""

# 5. Run clippy lints
echo "📎 Running clippy..."
cargo clippy -- -D warnings
echo ""

# 6. Build documentation
echo "📚 Building documentation..."
cargo doc --no-deps --features local-debug
echo ""

# 7. Check binary size
echo "📦 Checking binary size..."
cargo build --release
ls -lh target/release/libcomposio_sdk.rlib 2>/dev/null || echo "   (Library built)"
echo ""

echo "✨ All local tests passed!"
echo ""
echo "💡 Next steps:"
echo "   - Run specific example: cargo run --example basic_usage"
echo "   - Open docs: cargo doc --open"
echo "   - Profile memory: cargo run --example memory_profile"
