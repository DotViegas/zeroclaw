# Test argument extraction from natural language queries

Write-Host "=== Testing Argument Extraction ===" -ForegroundColor Cyan
Write-Host ""

# Test 1: Dropbox list folder with path
Write-Host "Test 1: 'list my Dropbox folder /Documents'" -ForegroundColor Yellow
cargo run -- test-tool COMPOSIO_SEARCH_TOOLS '{
  "queries": [{
    "use_case": "list dropbox folder /Documents"
  }],
  "session": {
    "id": "trs_Ij9jR5rIS4_7"
  }
}' 2>&1 | Select-String -Pattern "DROPBOX_LIST","tool_slug","path","description" -Context 0,2

Write-Host ""
Write-Host "---" -ForegroundColor Gray
Write-Host ""

# Test 2: Dropbox list folder without path (root)
Write-Host "Test 2: 'list my Dropbox folder'" -ForegroundColor Yellow
cargo run -- test-tool COMPOSIO_SEARCH_TOOLS '{
  "queries": [{
    "use_case": "list dropbox folder"
  }],
  "session": {
    "id": "trs_Ij9jR5rIS4_7"
  }
}' 2>&1 | Select-String -Pattern "DROPBOX_LIST","tool_slug","path","description" -Context 0,2

Write-Host ""
Write-Host "---" -ForegroundColor Gray
Write-Host ""

# Test 3: Check connection status
Write-Host "Test 3: Check Dropbox connection status" -ForegroundColor Yellow
cargo run -- test-tool COMPOSIO_MANAGE_CONNECTIONS '{
  "toolkits": ["dropbox"],
  "session_id": "trs_Ij9jR5rIS4_7"
}' 2>&1 | Select-String -Pattern "has_active_connection","redirect_url","status" -Context 0,2

Write-Host ""
Write-Host "=== Tests Complete ===" -ForegroundColor Cyan
