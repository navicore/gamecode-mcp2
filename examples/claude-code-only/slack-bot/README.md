# Claude Code Slack Bot with MCP Tools

A Slack bot that provides controlled access to Claude Code with restricted MCP tools.

## Overview

This bot demonstrates how to:
1. Expose Claude to Slack users safely
2. Restrict which MCP tools Claude can use
3. Audit all interactions
4. Handle timeouts and errors gracefully

## Architecture

```
Slack User → Slack Bot → Claude Code CLI → GameCode MCP2 Server → Restricted Tools
```

## Setup

### 1. Create Slack App

1. Go to https://api.slack.com/apps
2. Create new app "From scratch"
3. Enable Socket Mode (Settings → Socket Mode)
4. Generate App-Level Token with `connections:write` scope
5. Add Bot Token Scopes:
   - `app_mentions:read`
   - `chat:write`
   - `im:history`
   - `im:read`
   - `im:write`
6. Install app to workspace
7. Save tokens for `.env`

### 2. Configure Bot

```bash
# Install dependencies
pip install slack-sdk python-dotenv

# Copy environment template
cp .env.example .env

# Edit .env with your tokens
```

### 3. Configure MCP Tools

Create a restricted tools configuration:

```yaml
# restricted-tools.yaml
tools:
  # Only safe read operations
  - name: read_file
    description: Read a file
    command: cat
    validation:
      validate_paths: true
      allow_absolute_paths: false
    args:
      - name: path
        type: string
        required: true
        is_path: true
        
  # Limited diagram creation  
  - name: create_diagram
    description: Create a diagram
    command: internal
    internal_handler: write_file
    validation:
      validate_paths: true
      allow_absolute_paths: false
    args:
      - name: path
        type: string
        required: true
        is_path: true
      - name: content
        type: string
        required: true
```

### 4. Run Bot

```bash
# Set environment variable for tools
export GAMECODE_TOOLS_FILE=./restricted-tools.yaml

# Run the bot
python slack_claude_bot.py
```

## Usage

### Direct Message
```
User: Create a class diagram for a user authentication system
Bot: [Claude creates PlantUML diagram using allowed tools]
```

### Channel Mention
```
User: @ClaudeBot explain the files in this directory
Bot: [Claude uses read_file tool to examine and explain files]
```

### Slash Command
```
/claude analyze the README.md file
```

## Security Features

### 1. Tool Restrictions
- Only tools specified in `CLAUDE_ALLOWED_TOOLS` are available
- Format: `mcp__<server>__<tool>` with comma separation
- Example: `mcp__gamecode__read_file,mcp__gamecode__list_files`
- No wildcards allowed - must list each tool explicitly

### 2. Access Control
- Optional channel restrictions via `ALLOWED_CHANNELS`
- Optional user restrictions via `ALLOWED_USERS`
- Empty = no restrictions

### 3. Safety Limits
- Maximum prompt length (default: 1000 chars)
- Execution timeout (default: 30 seconds)
- Non-interactive mode enforced

### 4. Audit Logging
All requests logged to `claude_audit.jsonl`:
```json
{
  "timestamp": "2024-01-10T10:30:00Z",
  "user": "U123456",
  "channel": "C789012",
  "prompt": "Create a diagram...",
  "allowed_tools": "mcp__gamecode__read_file,mcp__gamecode__list_files"
}
```

## Advanced Configuration

### Tool-Specific Bots

Create specialized bots by changing `CLAUDE_ALLOWED_TOOLS`:

```bash
# Diagram-only bot
CLAUDE_ALLOWED_TOOLS=mcp__gamecode__create_diagram,mcp__gamecode__save_plantuml

# Read-only analysis bot
CLAUDE_ALLOWED_TOOLS=mcp__gamecode__read_file,mcp__gamecode__list_files,mcp__gamecode__grep

# Documentation bot
CLAUDE_ALLOWED_TOOLS=mcp__gamecode__create_diagram,mcp__gamecode__write_markdown
```

### Multiple Bots

Run multiple bots with different configurations:

```python
# diagram_bot.py
ALLOWED_TOOLS = "mcp__gamecode__save_plantuml,mcp__gamecode__save_mermaid"

# code_review_bot.py  
ALLOWED_TOOLS = "mcp__gamecode__read_file,mcp__gamecode__list_files"

# doc_bot.py
ALLOWED_TOOLS = "mcp__gamecode__write_markdown,mcp__gamecode__create_diagram"
```

## Error Handling

The bot handles:
- Claude timeouts → User-friendly timeout message
- Tool errors → Error details in response
- Missing permissions → Authorization error
- Invalid prompts → Validation message

## Monitoring

Monitor bot health via:
- `claude_bot.log` - Application logs
- `claude_audit.jsonl` - Request audit trail
- Slack's app monitoring dashboard

## Extending

To add custom behavior:

1. **Pre-processing**: Modify prompts before sending to Claude
2. **Post-processing**: Format Claude's output
3. **Custom commands**: Add slash commands for specific workflows
4. **Webhooks**: Integrate with external systems

## Example Workflows

### Code Documentation Bot
```yaml
# tools.yaml
include:
  - examples/diagrams/plantuml/source-only.yaml
  - examples/documentation/markdown/basic.yaml

# Bot responds to: "@bot document the auth module"
```

### Security Audit Bot
```yaml
# tools.yaml  
include:
  - examples/security/scanning/basic.yaml
  - examples/core/readonly.yaml

# Bot responds to: "@bot audit this directory for security issues"
```

## Production Deployment

### Docker/Kubernetes

The bot is designed to work seamlessly in containerized environments.

**Note on Authentication**:
- **Development**: Uses Claude Desktop with inherited shell environment
- **Production**: Typically uses AWS Bedrock or Vertex AI with proper IAM/service account auth

For production with Bedrock:

1. **Environment Variables**: In production, environment variables from K8s ConfigMaps/Secrets take precedence over `.env` files
2. **No .env in Production**: The Docker image doesn't include `.env` files
3. **Configuration**: All config comes from K8s manifests

#### Quick Deploy to K8s

```bash
# Build and push image
docker build -t your-registry/claude-slack-bot:latest .
docker push your-registry/claude-slack-bot:latest

# Deploy to Kubernetes
kubectl apply -f k8s-deployment.yaml

# Check logs
kubectl logs -l app=claude-slack-bot
```

#### Environment Variable Priority

1. **K8s/Docker Environment**: Takes precedence
2. **.env file**: Only used if env var not set (local dev)
3. **Default values**: Used if neither is set

This is handled automatically by `python-dotenv` with `override=False`.

#### Production with AWS Bedrock

In production, you'd typically:
1. Use a different `CLAUDE_COMMAND` that wraps Bedrock API calls
2. Configure AWS credentials via K8s secrets/IAM roles
3. No need for Claude Desktop authentication

Example K8s secret for Bedrock:
```yaml
apiVersion: v1
kind: Secret
metadata:
  name: bedrock-credentials
data:
  AWS_ACCESS_KEY_ID: <base64>
  AWS_SECRET_ACCESS_KEY: <base64>
  AWS_REGION: <base64>
```

## Troubleshooting

### Bot not responding
1. Check Socket Mode is enabled
2. Verify tokens in `.env`
3. Check `claude_bot.log`

### Claude errors

#### "Credit balance is too low" error
If you have a MAX account and get this error:
1. Check if you have an old `ANTHROPIC_API_KEY` set in your environment
2. Remove/unset any `ANTHROPIC_API_KEY` - MAX accounts use Desktop auth, not API keys
3. Having both MAX account and an API key causes authentication conflicts

To fix:
```bash
unset ANTHROPIC_API_KEY
# Remove from .env, .bashrc, .zshrc, etc.
```

#### Other Claude errors
1. Verify `claude` command works locally
2. Check allowed tools configuration
3. Review timeout settings

### Permission errors
1. Verify bot has correct Slack scopes
2. Check channel/user restrictions
3. Ensure bot is in the channel

### Known Issues

#### Ctrl+C doesn't stop the bot
The Slack SDK's Socket Mode client may not respond to Ctrl+C properly. Workarounds:
- Use `kill -9 <PID>` to force stop
- Run in Docker and use `docker stop`
- Close the terminal window

## Security Considerations

1. **Never expose write tools to untrusted users**
2. **Always use path validation for file operations**
3. **Set appropriate timeouts to prevent DoS**
4. **Regularly review audit logs**
5. **Use channel/user restrictions in production**

This bot demonstrates safe LLM exposure patterns that can be adapted for other chat platforms or internal tools.