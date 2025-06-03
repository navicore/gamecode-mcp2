.PHONY: all build test clean install release check fmt lint help

# Default target
all: build

# Build the project
build:
	cargo build

# Build release version
release:
	cargo build --release

# Run tests
test:
	cargo test

# Run all checks (tests, lints, formatting)
check: test lint fmt-check

# Run clippy lints
lint:
	cargo clippy -- -D warnings

# Format code
fmt:
	cargo fmt

# Check formatting
fmt-check:
	cargo fmt -- --check

# Clean build artifacts
clean:
	cargo clean

# Install the binary
install:
	cargo install --path .

# Run the server (for development)
run:
	cargo run

# Test MCP initialization
test-mcp:
	@echo '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}}},"id":1}' | cargo run --quiet 2>&1 | head -1

# Update dependencies
update:
	cargo update

# Show help
help:
	@echo "Available targets:"
	@echo "  make build      - Build the project"
	@echo "  make release    - Build release version"
	@echo "  make test       - Run tests"
	@echo "  make check      - Run all checks (tests, lints, formatting)"
	@echo "  make lint       - Run clippy lints"
	@echo "  make fmt        - Format code"
	@echo "  make clean      - Clean build artifacts"
	@echo "  make install    - Install the binary"
	@echo "  make run        - Run the server (development)"
	@echo "  make test-mcp   - Test MCP initialization"
	@echo "  make update     - Update dependencies"