#!/bin/bash
# Setup script for Slack bot MCP sandbox environment

# Create the sandbox base directory
SANDBOX_BASE="/tmp/slackbot_sandbox"
mkdir -p "$SANDBOX_BASE"

echo "Setting up MCP sandbox at $SANDBOX_BASE"
echo ""
echo "How it works:"
echo "1. Each request creates a subdirectory like $SANDBOX_BASE/<request-id>/"
echo "2. Bot copies mcp-config.json to that directory"
echo "3. Bot changes to that directory before running Claude"
echo "4. Claude uses --mcp-config to load the local MCP configuration"
echo "5. All files are created in the sandboxed directory"
echo "6. Directories are cleaned up automatically after 5 minutes"
echo ""
echo "Requirements:"
echo "- mcp-config-template.json must exist in the bot directory"
echo "- gamecode-mcp2 must be installed at ~/.cargo/bin/gamecode-mcp2"
echo "- No need for 'claude mcp add' - we use local configs!"
echo ""
echo "✓ Sandbox directory created at: $SANDBOX_BASE"