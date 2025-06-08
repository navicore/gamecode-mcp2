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

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    handlers=[
        logging.FileHandler('claude_bot.log'),
        logging.StreamHandler()
    ]
)
logger = logging.getLogger(__name__)

# Configuration
SLACK_BOT_TOKEN = os.environ["SLACK_BOT_TOKEN"]
SLACK_APP_TOKEN = os.environ["SLACK_APP_TOKEN"]

# Claude Code configuration
CLAUDE_COMMAND = os.environ.get("CLAUDE_COMMAND", "claude")
ALLOWED_TOOLS = os.environ.get(
    "CLAUDE_ALLOWED_TOOLS", "mcp__gamecode__read_file,mcp__gamecode__list_files")
MAX_PROMPT_LENGTH = int(os.environ.get("MAX_PROMPT_LENGTH", "1000"))
TIMEOUT_SECONDS = int(os.environ.get("CLAUDE_TIMEOUT", "30"))

# Optional: Restrict to specific channels or users
ALLOWED_CHANNELS = os.environ.get("ALLOWED_CHANNELS", "").split(",")
ALLOWED_USERS = os.environ.get("ALLOWED_USERS", "").split(",")


class ClaudeSlackBot:
    def __init__(self):
        self.web_client = WebClient(token=SLACK_BOT_TOKEN)
        self.socket_client = SocketModeClient(
            app_token=SLACK_APP_TOKEN,
            web_client=self.web_client
        )

    def start(self):
        """Start the Socket Mode client."""
        logger.info("Starting Claude Slack Bot...")

        # Register event handlers
        self.socket_client.socket_mode_request_listeners.append(
            self.process_socket_mode_request
        )

        # Connect to Slack
        self.socket_client.connect()

        # Keep the program running
        from threading import Event
        Event().wait()

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
            "--allowedTools", ALLOWED_TOOLS,
            "-p", prompt
        ]

        logger.info(f"Executing: {' '.join(cmd)}")

        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=TIMEOUT_SECONDS,
                # Disable color output
                env={**os.environ, "CLAUDE_NO_COLOR": "1"}
            )

            if result.returncode == 0:
                return result.stdout.strip()
            else:
                error_msg = result.stderr.strip() or "Unknown error"
                raise Exception(f"Claude returned error: {error_msg}")

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

    # Validate Claude is available
    try:
        subprocess.run([CLAUDE_COMMAND, "--version"],
                       capture_output=True, check=True)
    except:
        logger.error(f"Claude Code CLI not found at '{CLAUDE_COMMAND}'")
        logger.error(
            "Please install Claude Code or set CLAUDE_COMMAND environment variable")
        return

    # Start bot
    bot = ClaudeSlackBot()
    try:
        bot.start()
    except KeyboardInterrupt:
        logger.info("Bot stopped by user")
    except Exception as e:
        logger.error(f"Bot crashed: {e}", exc_info=True)


if __name__ == "__main__":
    main()
