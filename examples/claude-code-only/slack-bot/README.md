# Claude Code Slack Bot with MCP Tools

A Slack bot that provides controlled access to Claude Code with sandboxed MCP tool execution.

## Overview

This bot enables teams to safely interact with Claude through Slack while maintaining security through:
- Tool restrictions - only explicitly allowed MCP tools are available
- File sandboxing - each request runs in an isolated temporary directory
- Automatic cleanup - temporary files are removed after 5 minutes
- Full audit trail - all requests are logged
- Smart file handling - created files are automatically uploaded to Slack

## Architecture

```
Slack User → Slack Bot (Socket Mode) → Claude CLI → MCP Server → Sandboxed Tools
     ↓                                                                    ↓
  Slack App                                                      /tmp/slackbot_sandbox/
```

### Key Components

1. **Slack Bot** (`slack_claude_bot.py`)
   - Uses Socket Mode for real-time messaging without webhooks
   - Handles mentions, DMs, and slash commands
   - Manages sandboxed execution environments
   - Automatically uploads created files to Slack
   - Includes last 5 messages as conversation context

2. **MCP Tools** (`slack-bot-tools.yaml`)
   - Defines available tools with security restrictions
   - All file operations are relative to sandbox directory
   - No parent directory access allowed

3. **Security Features**
   - Per-request isolated directories
   - Tool allowlist enforcement
   - Request size limits
   - Optional user/channel restrictions

## Prerequisites

- Python 3.8+
- Claude Code CLI installed and authenticated
- gamecode-mcp2 v0.6.0+ installed
- GraphViz (`dot` command) - for diagram generation
- PlantUML (`plantuml` command) - for diagram generation

### Install Diagram Tools (optional, for diagram features)

```bash
# macOS
brew install graphviz plantuml

# Ubuntu/Debian
sudo apt-get install graphviz plantuml

# Check installation
dot -V
plantuml -version
```

## Setup

### 1. Create Slack App

1. Go to https://api.slack.com/apps
2. Create new app "From scratch"
3. Enable Socket Mode (Settings → Socket Mode)
4. Generate App-Level Token with `connections:write` scope
5. Add Bot Token Scopes:
   - `app_mentions:read` - respond to @mentions
   - `chat:write` - send messages
   - `im:history` - read DM history
   - `im:read` - receive DMs
   - `im:write` - send DMs
   - `files:write` - upload files
6. Install app to workspace
7. Copy both tokens for `.env`

### 2. Install Dependencies

```bash
# Create virtual environment
python3 -m venv venv
source venv/bin/activate

# Install requirements
pip install slack-sdk python-dotenv
```

### 3. Configure Environment

```bash
# Copy environment template
cp .env.example .env

# Edit .env with your tokens:
# - SLACK_APP_TOKEN (starts with xapp-)
# - SLACK_BOT_TOKEN (starts with xoxb-)
# - Adjust CLAUDE_ALLOWED_TOOLS as needed
```

### 4. Verify MCP Server

```bash
# Ensure gamecode-mcp2 is installed
which gamecode-mcp2

# Test MCP tools
gamecode-mcp2 --tools-file slack-bot-tools.yaml --list-tools
```

### 5. Run the Bot

```bash
python slack_claude_bot.py
```

## Usage

### Direct Messages
```
User: Create a CSV report of system processes
Bot: [Creates CSV file and uploads to Slack]
```

### Channel Mentions
```
User: @ClaudeBot analyze this data and create a chart
Bot: [Generates visualization and uploads as image]
```

### Diagram Generation
```
User: Create a GraphViz diagram showing a simple flowchart
Bot: [Creates DOT file, generates PNG/SVG, uploads only the image]

User: Generate a PlantUML sequence diagram for a login flow
Bot: [Creates PUML file, generates diagram, uploads the result]
```

### Slash Commands (if configured)
```
/claude summarize the latest metrics in JSON format
```

## Tool Configuration

### Available Tools

The bot uses `slack-bot-tools.yaml` which provides:
- `read_file` - Read files in the sandbox
- `write_file` - Create text files
- `create_csv` - Create CSV files with validation
- `create_json` - Create JSON files with validation
- `create_graphviz_diagram` - Create GraphViz diagrams from DOT text (PNG/SVG)
- `create_plantuml_diagram` - Create PlantUML diagrams from source text (PNG/SVG)
- `list_files` - List directory contents
- `search_files` - Find files by pattern
- `grep` - Search file contents

### Tool Security

All tools enforce:
- Relative paths only (no absolute paths)
- No parent directory access (`..` blocked)
- Operations confined to sandbox directory
- File size limits (10MB default)

### Customizing Tools

To modify available tools, edit `CLAUDE_ALLOWED_TOOLS` in `.env`:
```bash
# Example: Only allow read operations
CLAUDE_ALLOWED_TOOLS="mcp__gamecode__read_file,mcp__gamecode__list_files"

# Example: Full file operations
CLAUDE_ALLOWED_TOOLS="mcp__gamecode__write_file,mcp__gamecode__read_file,mcp__gamecode__create_csv,mcp__gamecode__create_json"
```

## Conversation Context

The bot includes the last 5 messages from the channel as context, enabling:
- Follow-up questions ("make it bigger", "add a title")
- References to previous outputs ("update that CSV")
- Natural multi-turn conversations
- Iterative refinement of diagrams and data

### Context Format
```
Recent conversation history:
User: Create a simple flowchart
Assistant: Created flowchart.png
User: Add a legend to it
```

## File Handling

### Automatic File Detection

The bot automatically detects and uploads files that Claude creates:
- CSV files are previewed as formatted tables
- JSON files are syntax highlighted
- Images (PNG, SVG, JPG) are uploaded directly
- Diagram source files (.dot, .puml) are created but not uploaded
- Large files are uploaded without preview

### Sandbox Lifecycle

1. Request received → new directory created
2. Claude executes in sandbox directory
3. Created files are detected and uploaded
4. Directory cleaned up after 5 minutes

## Security

### Request Validation
- Maximum prompt length enforced (default: 1000 chars)
- Execution timeout (default: 30 seconds)
- Optional user/channel allowlists

### Audit Trail
- All requests logged to `claude_audit.jsonl`
- Includes timestamp, user, channel, prompt, and allowed tools
- Application logs in `claude_bot.log`

### Environment Isolation
- Each request runs in unique directory
- No access to parent directories
- Temporary files automatically cleaned up

## Troubleshooting

### Bot Not Responding

1. Check Socket Mode is enabled in Slack app settings
2. Verify tokens in `.env` are correct
3. Check `claude_bot.log` for errors
4. Ensure bot is invited to channels/DMs

### Files Not Created

1. Verify MCP server is installed: `which gamecode-mcp2`
2. Check tool configuration in `slack-bot-tools.yaml`
3. Ensure `CLAUDE_ALLOWED_TOOLS` includes write tools
4. Check sandbox directory permissions

### Ctrl+C Not Working

The Slack SDK has known issues with signal handling. Use the provided kill script:
```bash
./kill_bot.sh
```

### File Upload Errors

If you see `method_deprecated` errors, ensure you're using the latest version of this bot which uses `files_upload_v2`.

## Development

### Adding New Tools

1. Edit `slack-bot-tools.yaml` to add tool definition
2. Add tool name to `CLAUDE_ALLOWED_TOOLS` in `.env`
3. Restart bot

### Testing Tools

Test tools directly with Claude:
```bash
# Create test directory
mkdir test && cd test

# Test with specific tool
claude --allowedTools "mcp__gamecode__write_file" \
  --mcp-config '{"mcpServers":{"gamecode":{"command":"gamecode-mcp2","args":["--tools-file","../slack-bot-tools.yaml"],"type":"stdio"}}}' \
  -p "Create a test file"
```

### Debug Mode

Enable debug logging in `.env`:
```bash
DEBUG=true
```

## Examples

See the `examples/` directory for:
- Alternative bot implementations
- Different tool configurations
- Integration patterns

## Architecture Decisions

### Why Socket Mode?
- No public webhook URL required
- Works behind firewalls
- Real-time bidirectional communication
- Simpler setup for internal tools

### Why Sandbox Directories?
- Prevents file conflicts between requests
- Easy cleanup of temporary files
- Clear audit trail per request
- Natural security boundary

### Why MCP?
- Standardized tool interface
- Language-agnostic tool definitions
- Built-in validation and security
- Extensible for future tools

## Known Limitations

1. **Signal Handling**: Ctrl+C doesn't cleanly stop the bot due to Slack SDK threading
2. **File Size**: Slack limits file uploads (varies by plan)
3. **Execution Time**: Long-running operations may timeout
4. **Concurrency**: Each request is processed sequentially

## Contributing

When contributing:
1. Test with various file types and sizes
2. Ensure error messages are user-friendly
3. Maintain security restrictions
4. Update documentation for new features

## License

See the parent repository's LICENSE file.
