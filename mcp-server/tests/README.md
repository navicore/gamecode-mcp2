# MCP Server Tests

This test suite validates the MCP server implementation using real YAML configurations.

## Test Structure

### Unit Tests
- **tool_loading_tests.rs** - Validates YAML parsing and tool configuration loading
- **tool_execution_tests.rs** - Tests actual tool execution with various inputs

### Integration Tests  
- **protocol_tests.rs** - Tests the full JSON-RPC protocol flow

### Security Tests
- **security_tests.rs** - Documents current security boundaries and risks

## Running Tests

```bash
# Run all tests
cargo test --package mcp-server

# Run specific test file
cargo test --package mcp-server --test tool_loading_tests

# Run with output for debugging
cargo test --package mcp-server -- --nocapture
```

## Test Fixtures

The `fixtures/` directory contains real YAML configurations used in tests:
- `test_tools.yaml` - Basic tool definitions
- `tools_with_include.yaml` - Tests include functionality
- `invalid_tools.yaml` - Tests error handling

## Security Notes

The security tests intentionally document vulnerabilities to ensure users understand risks:
- Path traversal is possible (by design - OS permissions are the boundary)
- No authentication exists (stdio trust model)
- Resource limits must be imposed externally

These tests prevent regressions while being honest about the security model.