# Local testing script for Composio SDK development (PowerShell)

Write-Host "🧪 Composio SDK Local Testing Suite" -ForegroundColor Cyan
Write-Host "====================================" -ForegroundColor Cyan
Write-Host ""

# Check if API key is set
if (-not $env:COMPOSIO_API_KEY) {
    Write-Host "❌ Error: COMPOSIO_API_KEY environment variable not set" -ForegroundColor Red
    Write-Host "   Please set it with: `$env:COMPOSIO_API_KEY='your_key_here'" -ForegroundColor Yellow
    exit 1
}

Write-Host "✅ API key found" -ForegroundColor Green
Write-Host ""

# 1. Run unit tests
Write-Host "📝 Running unit tests..." -ForegroundColor Yellow
cargo test --lib
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host ""

# 2. Run integration tests
Write-Host "🔗 Running integration tests..." -ForegroundColor Yellow
cargo test --test '*'
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host ""

# 3. Run examples with debug output
Write-Host "🎯 Running debug example..." -ForegroundColor Yellow
cargo run --example local_debug --features local-debug
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host ""

# 4. Check code formatting
Write-Host "🎨 Checking code formatting..." -ForegroundColor Yellow
cargo fmt --check
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host ""

# 5. Run clippy lints
Write-Host "📎 Running clippy..." -ForegroundColor Yellow
cargo clippy -- -D warnings
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host ""

# 6. Build documentation
Write-Host "📚 Building documentation..." -ForegroundColor Yellow
cargo doc --no-deps --features local-debug
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host ""

# 7. Check binary size
Write-Host "📦 Checking binary size..." -ForegroundColor Yellow
cargo build --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host ""

Write-Host "✨ All local tests passed!" -ForegroundColor Green
Write-Host ""
Write-Host "💡 Next steps:" -ForegroundColor Cyan
Write-Host "   - Run specific example: cargo run --example basic_usage"
Write-Host "   - Open docs: cargo doc --open"
Write-Host "   - Profile memory: cargo run --example memory_profile"
