#!/usr/bin/env python3
"""
Slack Socket Mode bot that invokes Claude Code with restricted MCP tools.

This bot demonstrates:
1. Safe exposure of Claude to Slack users
2. Tool restriction via --allowed-tools
3. Audit logging of all interactions

Requirements:
- pip install slack-sdk python-dotenv
- Claude Code CLI installed
- Slack app with Socket Mode enabled
"""

import os
import subprocess
import json
import logging
from datetime import datetime
from typing import Optional, List, Dict
from slack_sdk import WebClient
from slack_sdk.socket_mode import SocketModeClient
from slack_sdk.socket_mode.response import SocketModeResponse
from slack_sdk.socket_mode.request import SocketModeRequest
from dotenv import load_dotenv

# Load environment variables
# Note: In production (Docker/K8s), environment variables take precedence
# The .env file is only used for local development
load_dotenv(override=False)  # Explicit: don't override existing env vars

# Diagnostic: Print key env vars at startup
print(f"[STARTUP] DEBUG={os.environ.get('DEBUG', 'NOT SET')}")
print(f"[STARTUP] CLAUDE_COMMAND={
      os.environ.get('CLAUDE_COMMAND', 'NOT SET')}")


# For development: Ensure we have full shell PATH
# This helps find claude and its auth config
if "PATH" in os.environ:
    # Common paths where claude might be installed
    extra_paths = [
        "/usr/local/bin",
        "/opt/homebrew/bin",
        os.path.expanduser("~/.local/bin"),
    ]
    current_path = os.environ["PATH"]
    for path in extra_paths:
        if path not in current_path:
            os.environ["PATH"] = f"{path}:{current_path}"

# Configure logging
log_level = logging.DEBUG if os.environ.get(
    "DEBUG", "").lower() == "true" else logging.INFO
logging.basicConfig(
    level=log_level,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    handlers=[
        logging.FileHandler('claude_bot.log'),
        logging.StreamHandler()
    ]
)
logger = logging.getLogger(__name__)
logger.debug(f"Logging configured at {logging.getLevelName(log_level)} level")

# Configuration
SLACK_BOT_TOKEN = os.environ["SLACK_BOT_TOKEN"]
SLACK_APP_TOKEN = os.environ["SLACK_APP_TOKEN"]

# Claude Code configuration
CLAUDE_COMMAND = os.environ.get("CLAUDE_COMMAND", "claude")
CLAUDE_MODEL = os.environ.get("CLAUDE_MODEL", "sonnet")
ALLOWED_TOOLS = os.environ.get(
    "CLAUDE_ALLOWED_TOOLS", "mcp__gamecode__read_file,mcp__gamecode__list_files")
MAX_PROMPT_LENGTH = int(os.environ.get("MAX_PROMPT_LENGTH", "1000"))
TIMEOUT_SECONDS = int(os.environ.get("CLAUDE_TIMEOUT", "30"))

# Optional: Restrict to specific channels or users
ALLOWED_CHANNELS = os.environ.get("ALLOWED_CHANNELS", "").split(",")
ALLOWED_USERS = os.environ.get("ALLOWED_USERS", "").split(",")

# Debug mode to help diagnose auth issues
DEBUG_MODE = os.environ.get("DEBUG", "").lower() == "true"


class ClaudeSlackBot:
    def __init__(self):
        self.web_client = WebClient(token=SLACK_BOT_TOKEN)
        self.socket_client = SocketModeClient(
            app_token=SLACK_APP_TOKEN,
            web_client=self.web_client
        )
        self.stop_event = None

    def start(self):
        """Start the Socket Mode client."""
        logger.info("Starting Claude Slack Bot...")

        # Register event handlers
        self.socket_client.socket_mode_request_listeners.append(
            self.process_socket_mode_request
        )

        # Connect to Slack
        self.socket_client.connect()
        logger.info("Bot is running. Press Ctrl+C to stop.")

    def stop(self):
        """Stop the bot gracefully."""
        logger.info("Stopping bot...")
        try:
            # First shut down the interval runners to stop reconnection attempts
            if hasattr(self.socket_client, 'message_processor') and self.socket_client.message_processor:
                logger.debug("Shutting down message_processor")
                self.socket_client.message_processor.shutdown()

            if hasattr(self.socket_client, 'current_app_monitor') and self.socket_client.current_app_monitor:
                logger.debug("Shutting down current_app_monitor")
                self.socket_client.current_app_monitor.shutdown()

            if hasattr(self.socket_client, 'current_session_runner') and self.socket_client.current_session_runner:
                logger.debug("Shutting down current_session_runner")
                self.socket_client.current_session_runner.shutdown()

            # Disable auto-reconnect before disconnecting
            if hasattr(self.socket_client, 'auto_reconnect_enabled'):
                self.socket_client.auto_reconnect_enabled = False

            # Mark as closed to prevent any new operations
            if hasattr(self.socket_client, 'closed'):
                self.socket_client.closed = True

            # Now try to disconnect and close
            self.socket_client.disconnect()
            self.socket_client.close()
        except Exception as e:
            logger.debug(f"Error during shutdown: {e}")

    def process_socket_mode_request(self, client: SocketModeClient, req: SocketModeRequest):
        """Process Socket Mode requests from Slack."""
        logger.info(f"Received request type: {req.type}")

        if req.type == "events_api":
            # Acknowledge the request immediately
            response = SocketModeResponse(envelope_id=req.envelope_id)
            client.send_socket_mode_response(response)

            # Process the event
            self.handle_event(req.payload)

        elif req.type == "slash_commands":
            # Acknowledge the request immediately
            response = SocketModeResponse(envelope_id=req.envelope_id)
            client.send_socket_mode_response(response)

            # Process the slash command
            self.handle_slash_command(req.payload)

    def handle_event(self, event_payload: Dict):
        """Handle Slack events."""
        event = event_payload.get("event", {})
        
        # Skip bot's own messages and system messages
        if event.get("user") == self.get_bot_user_id():
            return
            
        # Skip message subtypes (edits, deletes, etc) - these often have empty text
        if event.get("subtype"):
            return

        if event.get("type") == "app_mention":
            self.handle_mention(event)
        elif event.get("type") == "message" and event.get("channel_type") == "im":
            self.handle_direct_message(event)

    def handle_mention(self, event: Dict):
        """Handle @bot mentions."""
        channel = event.get("channel")
        user = event.get("user")
        text = event.get("text", "")

        # Remove bot mention from text
        text = self.clean_mention(text)

        if not self.is_authorized(user, channel):
            self.send_message(
                channel, "Sorry, you're not authorized to use this bot.")
            return

        self.process_claude_request(channel, user, text)

    def handle_direct_message(self, event: Dict):
        """Handle direct messages."""
        # Skip bot's own messages
        if event.get("user") == self.get_bot_user_id():
            return

        channel = event.get("channel")
        user = event.get("user")
        text = event.get("text", "")

        if not self.is_authorized(user, channel):
            self.send_message(
                channel, "Sorry, you're not authorized to use this bot.")
            return

        self.process_claude_request(channel, user, text)

    def handle_slash_command(self, payload: Dict):
        """Handle slash commands like /claude."""
        channel = payload.get("channel_id")
        user = payload.get("user_id")
        text = payload.get("text", "")

        if not self.is_authorized(user, channel):
            self.send_message(
                channel, "Sorry, you're not authorized to use this command.")
            return

        self.process_claude_request(channel, user, text)

    def process_claude_request(self, channel: str, user: str, prompt: str):
        """Process a request to Claude."""
        # Validate prompt
        if not prompt.strip():
            self.send_message(channel, "Please provide a prompt for Claude.")
            return

        if len(prompt) > MAX_PROMPT_LENGTH:
            self.send_message(
                channel,
                f"Prompt too long. Maximum length is {
                    MAX_PROMPT_LENGTH} characters."
            )
            return

        # Log the request
        logger.info(f"User {user} in channel {
                    channel} requested: {prompt[:100]}...")
        self.log_audit(user, channel, prompt)

        # Get conversation history for context
        context = self.get_conversation_context(channel)

        # Send typing indicator and capture timestamp
        processing_ts = self.send_message(channel, "_processing..._")

        # Create a clean working directory for this request within the sandbox
        import tempfile
        import time
        import shutil
        
        # Use the configured sandbox base directory
        sandbox_base = os.environ.get("MCP_SANDBOX_BASE", "/tmp/slackbot_sandbox")
        
        # Ensure sandbox base exists
        os.makedirs(sandbox_base, exist_ok=True)
        
        request_id = f"{int(time.time() * 1000)}_{user[-4:]}"
        working_dir = os.path.join(sandbox_base, request_id)
        
        # Clean up any old directories first
        try:
            if os.path.exists(sandbox_base):
                # Remove directories older than 1 hour
                now = time.time()
                for item in os.listdir(sandbox_base):
                    item_path = os.path.join(sandbox_base, item)
                    if os.path.isdir(item_path) and item != '.mcp':  # Don't delete MCP config
                        try:
                            age = now - os.path.getmtime(item_path)
                            if age > 3600:
                                shutil.rmtree(item_path)
                                logger.debug(f"Cleaned up old directory: {item} (age: {age/60:.1f} minutes)")
                        except:
                            pass
        except Exception as e:
            logger.warning(f"Error during cleanup: {e}")
        
        # Create fresh working directory
        os.makedirs(working_dir, exist_ok=True)
        logger.info(f"Created working directory: {working_dir}")

        # Create MCP config dynamically with absolute paths
        bot_dir = os.path.dirname(os.path.abspath(__file__))
        default_tools_file_path = os.path.join(bot_dir, 'slack-bot-tools.yaml')
        tools_file_path = os.environ.get("SLACK_BOT_TOOLS_FILE", default_tools_file_path)
        
        mcp_config = {
            "mcpServers": {
                "gamecode": {
                    "command": os.path.expanduser("~/.cargo/bin/gamecode-mcp2"),
                    "args": ["--tools-file", tools_file_path],
                    "type": "stdio"
                }
            }
        }
        
        # Convert to JSON string for passing directly to claude
        mcp_config_json = json.dumps(mcp_config)
        logger.debug(f"MCP config JSON: {mcp_config_json}")
        
        # Save current directory
        original_dir = os.getcwd()
        
        try:
            # Change to working directory so all file operations happen there
            os.chdir(working_dir)
            logger.debug(f"Changed to working directory: {working_dir}")
            
            # Execute Claude with MCP config JSON and context
            result = self.execute_claude(prompt, working_dir, mcp_config_json, context)

            # Find all files created in the working directory
            created_files = []
            source_files = []  # Track source files separately
            for item in os.listdir('.'):
                if os.path.isfile(item):
                    _, ext = os.path.splitext(item.lower())
                    if ext in ['.dot', '.puml']:
                        source_files.append(item)
                    else:
                        created_files.append(item)
            
            # Delete the processing message now that we have results
            self.delete_message(channel, processing_ts)
            
            if created_files:
                logger.info(f"Files created in working directory: {created_files}")
                if source_files:
                    logger.info(f"Source files created (not uploaded): {source_files}")
                
                # Send each file to Slack (excluding source files)
                for file_name in created_files:
                    file_path = os.path.abspath(file_name)
                    self.send_file_to_slack(channel, file_path, f"📎 Created: {file_name}")
            
            # Also check if the result mentions a file that we might have missed
            mentioned_files = self.extract_filenames_from_text(result)
            for file_name in mentioned_files:
                if file_name not in created_files and os.path.exists(file_name):
                    file_path = os.path.abspath(file_name)
                    self.send_file_to_slack(channel, file_path, f"📎 Created: {file_name}")

            # Send the text response only if no files were created, or if it contains more than just file creation info
            if not created_files:
                # No files created, send full response
                self.send_formatted_content(channel, result, prompt)
            else:
                # Files were created - only send text if it's not just about file creation
                # Simple heuristic: if the response is short and mentions the filename, skip it
                result_lower = result.lower()
                is_just_file_notification = (
                    len(result) < 200 and 
                    any(fname.lower() in result_lower for fname in created_files) and
                    any(word in result_lower for word in ['created', 'saved', 'generated', 'wrote'])
                )
                
                if not is_just_file_notification:
                    # Response contains additional information beyond file creation
                    self.send_formatted_content(channel, result, prompt)
            
        except subprocess.TimeoutExpired:
            self.delete_message(channel, processing_ts)
            self.send_message(
                channel, "⏱️ Claude took too long to respond. Please try a simpler request.")
        except Exception as e:
            logger.error(f"Error executing Claude: {e}")
            self.delete_message(channel, processing_ts)
            self.send_message(channel, f"❌ Error: {str(e)}")
        finally:
            # Always return to original directory
            os.chdir(original_dir)
            
            # Clean up working directory after a delay
            import threading
            def cleanup():
                time.sleep(300)  # Keep for 5 minutes in case needed for debugging
                try:
                    if os.path.exists(working_dir):
                        shutil.rmtree(working_dir)
                        logger.debug(f"Cleaned up working directory: {working_dir}")
                except Exception as e:
                    logger.error(f"Error cleaning up {working_dir}: {e}")
            threading.Thread(target=cleanup, daemon=True).start()

    def execute_claude(self, prompt: str, working_dir: str = None, mcp_config_json: str = None, context: str = None) -> str:
        """Execute Claude Code CLI with restrictions."""
        # Build the full prompt with context
        if context:
            full_prompt = f"{context}\n\nCurrent request: {prompt}"
        else:
            full_prompt = prompt
            
        cmd = [
            CLAUDE_COMMAND,
            "--model", CLAUDE_MODEL,
            "--allowedTools", ALLOWED_TOOLS,
        ]
        
        # Add MCP config if provided
        if mcp_config_json:
            cmd.extend(["--mcp-config", mcp_config_json])
            logger.debug(f"Using MCP config JSON")
        
        cmd.extend(["-p", full_prompt])
        
        # Log the command with proper quoting for debugging
        import shlex
        logger.info(f"Executing: {shlex.join(cmd)}")

        if DEBUG_MODE:
            # Log environment variables that might affect Claude auth
            claude_env_vars = {k: v for k, v in os.environ.items()
                               if 'CLAUDE' in k or 'ANTHROPIC' in k or 'CONFIG' in k}
            logger.debug(f"Claude-related env vars: {claude_env_vars}")
            logger.debug(f"HOME: {os.environ.get('HOME')}")
            logger.debug(f"USER: {os.environ.get('USER')}")
            logger.debug(f"PATH: {os.environ.get('PATH')}")

            # Also check what the shell sees
            debug_result = subprocess.run(
                ["zsh", "-i", "-l", "-c", "echo HOME=$HOME && echo USER=$USER && which claude && ls -la ~/.claude 2>/dev/null || echo 'No .claude dir'"],
                capture_output=True,
                text=True
            )
            logger.debug(f"Shell environment check: {debug_result.stdout}")
            if debug_result.stderr:
                logger.debug(f"Shell environment stderr: {
                             debug_result.stderr}")

        try:
            # Run through interactive login zsh shell to get aliases
            # -i = interactive (loads aliases)
            # -l = login shell (loads full environment)
            zsh_cmd = shlex.join(cmd)

            result = subprocess.run(
                ["zsh", "-i", "-l", "-c", zsh_cmd],
                capture_output=True,
                text=True,
                timeout=TIMEOUT_SECONDS
            )

            if result.returncode == 0:
                output = result.stdout.strip()
                logger.debug(f"Claude output: {output[:200]}...")
                return output
            else:
                error_msg = result.stderr.strip()
                stdout_msg = result.stdout.strip()

                # Build detailed error message
                error_parts = [f"Exit code: {result.returncode}"]
                if error_msg:
                    error_parts.append(f"Stderr: {error_msg}")
                if stdout_msg:
                    error_parts.append(f"Stdout: {stdout_msg}")

                # Log full command for debugging
                logger.error(f"Command failed: {shlex.join(cmd)}")

                raise Exception(" | ".join(error_parts))

        except subprocess.TimeoutExpired:
            raise
        except Exception as e:
            logger.error(f"Failed to execute Claude: {e}")
            raise

    def send_message(self, channel: str, text: str, code_block: bool = False):
        """Send a message to Slack. Returns the message timestamp."""
        if code_block:
            text = f"```\n{text}\n```"

        try:
            response = self.web_client.chat_postMessage(
                channel=channel,
                text=text,
                mrkdwn=True
            )
            return response.get('ts')  # Return the timestamp
        except Exception as e:
            logger.error(f"Failed to send message: {e}")
            return None
    
    def delete_message(self, channel: str, timestamp: str):
        """Delete a message from Slack."""
        if not timestamp:
            return
        
        try:
            self.web_client.chat_delete(
                channel=channel,
                ts=timestamp
            )
        except Exception as e:
            logger.debug(f"Failed to delete message: {e}")
    
    def send_formatted_content(self, channel: str, content: str, prompt: str):
        """Send content with appropriate formatting based on type detection."""
        # First check if this is a file path to an image or data file
        content_stripped = content.strip()
        
        # Check for file paths (absolute or relative)
        # Only treat as file path if it's a short string that looks like a path
        is_likely_path = (
            content_stripped.startswith('/') or 
            (len(content_stripped.split('\n')) == 1 and 
             len(content_stripped) < 100 and  # File paths shouldn't be very long
             ' ' not in content_stripped and  # File paths typically don't have spaces
             '.' in content_stripped and 
             any(ext in content_stripped.lower() for ext in ['.png', '.jpg', '.jpeg', '.gif', '.svg', '.csv', '.json', '.yaml', '.yml']))
        )
        
        if is_likely_path:
            # This looks like a file path
            file_path = content_stripped
            
            # If it's a relative path, make it absolute
            if not file_path.startswith('/'):
                file_path = os.path.abspath(file_path)
            
            # Check if file exists
            if os.path.exists(file_path):
                _, ext = os.path.splitext(file_path.lower())
                
                if ext in ['.png', '.jpg', '.jpeg', '.gif', '.svg']:
                    self.send_image_file(channel, file_path)
                    return
                elif ext in ['.csv', '.json', '.yaml', '.yml']:
                    # Read and send the file content with appropriate formatting
                    try:
                        with open(file_path, 'r') as f:
                            file_content = f.read()
                        
                        # Send a message about the file
                        self.send_message(channel, f"📄 Created file: `{os.path.basename(file_path)}`")
                        
                        # Send the content formatted appropriately
                        if ext == '.csv':
                            self.send_csv_formatted(channel, file_content)
                        elif ext == '.json':
                            self.send_json_formatted(channel, file_content)
                        elif ext in ['.yaml', '.yml']:
                            self.send_yaml_formatted(channel, file_content)
                        
                        # Also offer to download
                        self.send_as_file(channel, file_content, prompt)
                        return
                    except Exception as e:
                        logger.error(f"Error reading file {file_path}: {e}")
            else:
                # File doesn't exist, but it looks like a file path
                logger.warning(f"File not found: {file_path}")
                # Continue with normal content processing
        
        # Try to detect what type of content we have
        content_lower = content.lower()
        prompt_lower = prompt.lower()
        
        # Check if user explicitly asked for a specific format
        wants_json = any(word in prompt_lower for word in ['json', 'as json'])
        wants_yaml = any(word in prompt_lower for word in ['yaml', 'yml', 'as yaml'])
        wants_csv = any(word in prompt_lower for word in ['csv', 'comma separated', 'as csv'])
        wants_file = any(word in prompt_lower for word in ['file', 'download', 'attachment'])
        
        # Auto-detect content type if not explicitly requested
        is_json = False
        is_yaml = False
        is_csv = False
        is_svg = False
        is_markdown = False
        
        # Try to parse as JSON
        if content.strip().startswith('{') or content.strip().startswith('['):
            try:
                json.loads(content)
                is_json = True
            except:
                pass
        
        # Check for YAML indicators
        if content.strip().startswith('---') or ': ' in content:
            is_yaml = True
        
        # Check for CSV (has comma-separated values with consistent columns)
        if ',' in content and '\n' in content:
            lines = content.strip().split('\n')
            if len(lines) > 1:
                first_commas = lines[0].count(',')
                if all(line.count(',') == first_commas for line in lines[1:5]):
                    is_csv = True
        
        # Check for SVG
        if '<svg' in content_lower and '</svg>' in content_lower:
            is_svg = True
        
        # Check for markdown elements
        if any(marker in content for marker in ['#', '**', '```', '|', '-', '*']):
            is_markdown = True
        
        # Determine how to send the content
        if is_svg or (wants_file and (is_json or is_yaml or is_csv)):
            # Send as a file attachment
            self.send_as_file(channel, content, prompt, is_svg)
        elif is_json and not wants_yaml and not wants_csv:
            # Send JSON with syntax highlighting
            self.send_json_formatted(channel, content)
        elif is_yaml and not wants_json and not wants_csv:
            # Send YAML with syntax highlighting
            self.send_yaml_formatted(channel, content)
        elif is_csv:
            # Send CSV as a table or code block
            self.send_csv_formatted(channel, content)
        elif is_markdown:
            # Send as markdown
            self.send_message(channel, content)
        else:
            # Default to code block for structured data
            self.send_message(channel, content, code_block=True)
    
    def send_json_formatted(self, channel: str, json_content: str):
        """Send JSON with proper formatting."""
        try:
            # Pretty print the JSON
            parsed = json.loads(json_content)
            pretty_json = json.dumps(parsed, indent=2)
            
            # Use JSON syntax highlighting
            formatted = f"```json\n{pretty_json}\n```"
            self.send_message(channel, formatted)
        except:
            # Fallback to plain code block
            self.send_message(channel, json_content, code_block=True)
    
    def send_yaml_formatted(self, channel: str, yaml_content: str):
        """Send YAML with proper formatting."""
        # Use YAML syntax highlighting
        formatted = f"```yaml\n{yaml_content}\n```"
        self.send_message(channel, formatted)
    
    def send_csv_formatted(self, channel: str, csv_content: str):
        """Send CSV as a formatted table or code block."""
        lines = csv_content.strip().split('\n')
        
        # If small enough, convert to a Slack table
        if len(lines) <= 10 and all(len(line.split(',')) <= 5 for line in lines):
            # Convert to markdown table
            headers = lines[0].split(',')
            table = "| " + " | ".join(headers) + " |\n"
            table += "|" + "|".join(["---"] * len(headers)) + "|\n"
            
            for line in lines[1:]:
                cells = line.split(',')
                table += "| " + " | ".join(cells) + " |\n"
            
            self.send_message(channel, table)
        else:
            # Too large for table, use code block
            formatted = f"```csv\n{csv_content}\n```"
            self.send_message(channel, formatted)
    
    def send_as_file(self, channel: str, content: str, prompt: str, is_svg: bool = False):
        """Send content as a file attachment."""
        import tempfile
        import time
        
        # Determine file extension and mime type
        if is_svg:
            ext = '.svg'
            mime_type = 'image/svg+xml'
            filename = f"diagram_{int(time.time())}.svg"
        elif 'json' in prompt.lower() or content.strip().startswith('{'):
            ext = '.json'
            mime_type = 'application/json'
            filename = f"data_{int(time.time())}.json"
        elif 'yaml' in prompt.lower():
            ext = '.yaml'
            mime_type = 'text/yaml'
            filename = f"data_{int(time.time())}.yaml"
        elif 'csv' in prompt.lower():
            ext = '.csv'
            mime_type = 'text/csv'
            filename = f"data_{int(time.time())}.csv"
        else:
            ext = '.txt'
            mime_type = 'text/plain'
            filename = f"output_{int(time.time())}.txt"
        
        try:
            # Create a temporary file
            with tempfile.NamedTemporaryFile(mode='w', suffix=ext, delete=False) as tmp_file:
                tmp_file.write(content)
                tmp_path = tmp_file.name
            
            # Upload the file to Slack
            response = self.web_client.files_upload_v2(
                channel=channel,
                file=tmp_path,
                filename=filename,
                initial_comment=f"Here's the {ext[1:].upper()} output you requested:"
            )
            
            # Clean up
            os.unlink(tmp_path)
            
            if not response["ok"]:
                raise Exception(f"File upload failed: {response.get('error', 'Unknown error')}")
                
        except Exception as e:
            logger.error(f"Failed to upload file: {e}")
            # Fallback to text message
            self.send_message(channel, content, code_block=True)
    
    def send_image_file(self, channel: str, file_path: str):
        """Send an image file that was created by a tool."""
        if not os.path.exists(file_path):
            self.send_message(channel, f"❌ File not found: {file_path}")
            return
        
        try:
            # Get file extension
            _, ext = os.path.splitext(file_path)
            ext = ext.lower()
            
            # Map extensions to mime types
            mime_types = {
                '.png': 'image/png',
                '.jpg': 'image/jpeg',
                '.jpeg': 'image/jpeg',
                '.gif': 'image/gif',
                '.svg': 'image/svg+xml'
            }
            
            mime_type = mime_types.get(ext, 'application/octet-stream')
            
            # Upload the file to Slack
            with open(file_path, 'rb') as file_content:
                response = self.web_client.files_upload_v2(
                    channel=channel,
                    file=file_content,
                    filename=os.path.basename(file_path),
                    initial_comment="Here's the generated image:"
                )
            
            if not response["ok"]:
                raise Exception(f"File upload failed: {response.get('error', 'Unknown error')}")
                
        except Exception as e:
            logger.error(f"Failed to upload image: {e}")
            self.send_message(channel, f"❌ Failed to upload image: {str(e)}")
    
    def extract_filenames_from_text(self, text: str) -> List[str]:
        """Extract potential filenames from Claude's response."""
        import re
        filenames = []
        
        # Look for quoted filenames
        # Matches 'filename.ext' or "filename.ext" or `filename.ext`
        pattern = r'[\'"`]([^\'"`]+\.[a-zA-Z0-9]+)[\'"`]'
        matches = re.findall(pattern, text)
        
        for match in matches:
            # Filter out URLs, paths, and source files
            _, ext = os.path.splitext(match.lower())
            if not match.startswith('http') and '/' not in match and ext not in ['.dot', '.puml']:
                filenames.append(match)
        
        # Also look for common file patterns without quotes
        # e.g., "Created filename.csv" or "saved to data.json"
        # Exclude source file extensions
        word_pattern = r'\b(\w+\.(?:csv|json|yaml|yml|txt|png|jpg|jpeg|svg|gif))\b'
        word_matches = re.findall(word_pattern, text, re.IGNORECASE)
        
        for match in word_matches:
            if match not in filenames:
                filenames.append(match)
        
        return filenames
    
    def send_file_to_slack(self, channel: str, file_path: str, message: str = "", skip_preview: bool = True):
        """Send a file to Slack with appropriate formatting."""
        if not os.path.exists(file_path):
            logger.warning(f"File not found for upload: {file_path}")
            return
        
        file_name = os.path.basename(file_path)
        _, ext = os.path.splitext(file_name.lower())
        
        # Read file content for small text files - but only show preview if not skipping
        if not skip_preview and ext in ['.csv', '.json', '.yaml', '.yml', '.txt'] and os.path.getsize(file_path) < 50000:  # 50KB
            try:
                with open(file_path, 'r') as f:
                    content = f.read()
                
                # Send a preview based on file type
                if ext == '.csv':
                    self.send_message(channel, f"📄 {message}")
                    self.send_csv_formatted(channel, content)
                elif ext == '.json':
                    self.send_message(channel, f"📄 {message}")
                    self.send_json_formatted(channel, content)
                elif ext in ['.yaml', '.yml']:
                    self.send_message(channel, f"📄 {message}")
                    self.send_yaml_formatted(channel, content)
                else:
                    self.send_message(channel, f"📄 {message}")
                    self.send_message(channel, content, code_block=True)
            except Exception as e:
                logger.error(f"Error reading file {file_path}: {e}")
        
        # Upload the actual file
        try:
            with open(file_path, 'rb') as file_content:
                response = self.web_client.files_upload_v2(
                    channel=channel,
                    file=file_content,
                    filename=file_name,
                    initial_comment=message or f"File: {file_name}"
                )
            
            if not response["ok"]:
                raise Exception(f"File upload failed: {response.get('error', 'Unknown error')}")
                
        except Exception as e:
            logger.error(f"Failed to upload file: {e}")
            # If upload fails, at least tell them about the file
            self.send_message(channel, f"📎 Created file: `{file_name}` (upload failed: {str(e)})")

    def is_authorized(self, user: str, channel: str) -> bool:
        """Check if user/channel is authorized."""
        if ALLOWED_USERS and ALLOWED_USERS[0]:  # Check if list is not empty
            if user not in ALLOWED_USERS:
                return False

        # Check if list is not empty
        if ALLOWED_CHANNELS and ALLOWED_CHANNELS[0]:
            if channel not in ALLOWED_CHANNELS:
                return False

        return True

    def clean_mention(self, text: str) -> str:
        """Remove bot mention from text."""
        # Remove <@BOT_ID> pattern
        import re
        bot_id = self.get_bot_user_id()
        if bot_id:
            text = re.sub(f'<@{bot_id}>', '', text).strip()
        return text

    def get_bot_user_id(self) -> Optional[str]:
        """Get the bot's user ID."""
        try:
            response = self.web_client.auth_test()
            return response.get("user_id")
        except:
            return None
    
    def get_conversation_context(self, channel: str) -> Optional[str]:
        """Get recent conversation history for context."""
        try:
            # Fetch last 5 messages from the channel
            response = self.web_client.conversations_history(
                channel=channel,
                limit=6  # Get 6 to account for the current "processing..." message
            )
            
            if not response["ok"]:
                logger.warning(f"Failed to fetch conversation history: {response.get('error')}")
                return None
            
            messages = response.get("messages", [])
            if not messages:
                return None
            
            # Get bot user ID for identifying bot messages
            bot_user_id = self.get_bot_user_id()
            
            # Build context from messages (newest first in Slack API, so reverse)
            context_parts = ["Recent conversation history:"]
            
            for msg in reversed(messages[1:]):  # Skip the most recent (processing...) message
                # Skip system messages and subtypes
                if msg.get("subtype"):
                    continue
                    
                # Get user info
                user_id = msg.get("user", "Unknown")
                text = msg.get("text", "")
                
                # Clean bot mentions from text
                if bot_user_id:
                    text = text.replace(f"<@{bot_user_id}>", "").strip()
                
                # Format message based on whether it's from the bot
                if user_id == bot_user_id:
                    context_parts.append(f"Assistant: {text}")
                else:
                    # Get username if possible
                    try:
                        user_info = self.web_client.users_info(user=user_id)
                        username = user_info["user"]["name"] if user_info["ok"] else "User"
                    except:
                        username = "User"
                    context_parts.append(f"{username}: {text}")
            
            # Only return context if we have meaningful history
            if len(context_parts) > 1:
                return "\n".join(context_parts)
            else:
                return None
                
        except Exception as e:
            logger.error(f"Error fetching conversation context: {e}")
            return None

    def log_audit(self, user: str, channel: str, prompt: str):
        """Log audit trail of all requests."""
        audit_entry = {
            "timestamp": datetime.utcnow().isoformat(),
            "user": user,
            "channel": channel,
            "prompt": prompt,
            "allowed_tools": ALLOWED_TOOLS
        }

        # Append to audit log
        with open("claude_audit.jsonl", "a") as f:
            f.write(json.dumps(audit_entry) + "\n")


def main():
    """Main entry point."""

    # Validate environment
    required_vars = ["SLACK_BOT_TOKEN", "SLACK_APP_TOKEN"]
    missing = [var for var in required_vars if not os.environ.get(var)]

    if missing:
        logger.error(f"Missing required environment variables: {missing}")
        logger.error("Please set them in .env file or environment")
        return

    # Validate Claude is available (using same shell method as execute)
    try:
        test_cmd = f"{CLAUDE_COMMAND} --version"
        result = subprocess.run(
            ["zsh", "-i", "-l", "-c", test_cmd],
            capture_output=True,
            text=True
        )
        if result.returncode != 0:
            raise Exception(f"Command failed: {result.stderr}")
        logger.info(f"Claude version: {result.stdout.strip()}")
    except Exception as e:
        logger.error(f"Claude Code CLI not found or not working: {e}")
        logger.error(f"Tried command: {CLAUDE_COMMAND}")
        logger.error(
            "Please install Claude Code or set CLAUDE_COMMAND environment variable")
        return

    # Create bot instance
    bot = ClaudeSlackBot()

    # Set up interruptible wait with signal handling
    import signal
    import threading
    import time

    stop_event = threading.Event()

    def signal_handler(signum, frame):
        logger.info("\n[Shutting down] Ctrl+C received...")
        stop_event.set()
        # Stop the bot immediately (not in a thread)
        try:
            bot.stop()
        except:
            pass
        # Force exit after brief delay
        threading.Timer(0.5, lambda: os._exit(0)).start()

    # Register signal handlers
    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)

    try:
        # Start the bot
        bot.start()

        # Use interruptible wait instead of infinite sleep
        while not stop_event.is_set():
            stop_event.wait(0.1)  # Check every 100ms

    except Exception as e:
        logger.error(f"Bot crashed: {e}", exc_info=True)
    finally:
        bot.stop()
        logger.info("Cleanup complete")


if __name__ == "__main__":
    main()
