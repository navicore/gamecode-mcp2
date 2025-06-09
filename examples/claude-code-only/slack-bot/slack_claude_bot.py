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
print(f"[STARTUP] CLAUDE_COMMAND={os.environ.get('CLAUDE_COMMAND', 'NOT SET')}")


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
log_level = logging.DEBUG if os.environ.get("DEBUG", "").lower() == "true" else logging.INFO
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

        # Send typing indicator
        self.send_message(channel, "_Claude is thinking..._")

        try:
            # Execute Claude with restrictions
            result = self.execute_claude(prompt)

            # Send response
            self.send_message(channel, result, code_block=True)

        except subprocess.TimeoutExpired:
            self.send_message(
                channel, "⏱️ Claude took too long to respond. Please try a simpler request.")
        except Exception as e:
            logger.error(f"Error executing Claude: {e}")
            self.send_message(channel, f"❌ Error: {str(e)}")

    def execute_claude(self, prompt: str) -> str:
        """Execute Claude Code CLI with restrictions."""
        cmd = [
            CLAUDE_COMMAND,
            "--model", CLAUDE_MODEL,
            "--allowedTools", ALLOWED_TOOLS,
            "-p", prompt
        ]

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
                logger.debug(f"Shell environment stderr: {debug_result.stderr}")

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
                return result.stdout.strip()
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
        """Send a message to Slack."""
        if code_block:
            text = f"```\n{text}\n```"

        try:
            self.web_client.chat_postMessage(
                channel=channel,
                text=text,
                mrkdwn=True
            )
        except Exception as e:
            logger.error(f"Failed to send message: {e}")

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
        logger.error("Please install Claude Code or set CLAUDE_COMMAND environment variable")
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
