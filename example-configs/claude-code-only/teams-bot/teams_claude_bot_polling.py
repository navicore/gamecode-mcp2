#!/usr/bin/env python3
"""
Microsoft Teams bot using Graph API polling - no incoming webhooks required.

This bot polls for messages mentioning it and responds with Claude.
Perfect for secure environments that don't allow incoming connections.

Requirements:
- pip install msal requests python-dotenv
- Azure AD app registration with Graph API permissions
- Claude Code CLI installed
"""

import os
import time
import json
import subprocess
import logging
from datetime import datetime, timedelta
from typing import Optional, List, Dict, Set
import requests
from msal import ConfidentialClientApplication
from dotenv import load_dotenv

# Load environment variables
load_dotenv(override=False)  # Production env vars take precedence

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

# Azure AD / Graph API Configuration
TENANT_ID = os.environ["AZURE_TENANT_ID"]
CLIENT_ID = os.environ["AZURE_CLIENT_ID"]
CLIENT_SECRET = os.environ["AZURE_CLIENT_SECRET"]

# Teams Configuration
TEAMS_USER_ID = os.environ.get("TEAMS_BOT_USER_ID")  # Bot's user ID in Teams
POLLING_INTERVAL = int(os.environ.get("POLLING_INTERVAL", "5"))  # seconds

# Claude Configuration
CLAUDE_COMMAND = os.environ.get("CLAUDE_COMMAND", "claude")
CLAUDE_MODEL = os.environ.get("CLAUDE_MODEL", "claude-3-5-sonnet-latest")
ALLOWED_TOOLS = os.environ.get("CLAUDE_ALLOWED_TOOLS", "mcp__gamecode__read_file,mcp__gamecode__list_files")
MAX_PROMPT_LENGTH = int(os.environ.get("MAX_PROMPT_LENGTH", "1000"))
TIMEOUT_SECONDS = int(os.environ.get("CLAUDE_TIMEOUT", "30"))

# Graph API endpoints
GRAPH_API_BASE = "https://graph.microsoft.com/v1.0"
GRAPH_API_BETA = "https://graph.microsoft.com/beta"


class TeamsClaudeBot:
    def __init__(self):
        # Initialize MSAL client
        self.app = ConfidentialClientApplication(
            CLIENT_ID,
            authority=f"https://login.microsoftonline.com/{TENANT_ID}",
            client_credential=CLIENT_SECRET,
        )
        
        self.access_token = None
        self.token_expiry = None
        self.processed_messages: Set[str] = set()  # Track processed message IDs
        
    def start(self):
        """Start polling for messages."""
        logger.info("Starting Teams Claude Bot (Polling Mode)...")
        
        while True:
            try:
                # Ensure we have a valid token
                self.ensure_token()
                
                # Poll for new messages
                self.poll_messages()
                
                # Wait before next poll
                time.sleep(POLLING_INTERVAL)
                
            except KeyboardInterrupt:
                logger.info("Bot stopped by user")
                break
            except Exception as e:
                logger.error(f"Error in main loop: {e}")
                time.sleep(POLLING_INTERVAL * 2)  # Back off on error
    
    def ensure_token(self):
        """Ensure we have a valid access token."""
        if self.access_token and self.token_expiry > datetime.utcnow():
            return
        
        logger.info("Acquiring new access token...")
        
        result = self.app.acquire_token_silent(
            ["https://graph.microsoft.com/.default"], account=None
        )
        
        if not result:
            result = self.app.acquire_token_for_client(
                scopes=["https://graph.microsoft.com/.default"]
            )
        
        if "access_token" in result:
            self.access_token = result["access_token"]
            # Token typically valid for 1 hour, refresh a bit early
            self.token_expiry = datetime.utcnow() + timedelta(minutes=55)
            logger.info("Access token acquired successfully")
        else:
            raise Exception(f"Failed to acquire token: {result.get('error_description')}")
    
    def poll_messages(self):
        """Poll for new messages mentioning the bot."""
        headers = {
            "Authorization": f"Bearer {self.access_token}",
            "Content-Type": "application/json"
        }
        
        # Get recent messages from all teams/channels the bot is in
        # Note: This requires appropriate Graph API permissions
        
        # Option 1: Get messages from specific teams/channels (more efficient)
        # You'd need to configure TEAM_ID and CHANNEL_ID
        
        # Option 2: Search for mentions (requires search permissions)
        # This searches for messages mentioning the bot
        
        try:
            # Get recent chat messages (last 5 minutes)
            # Adjust the filter based on your needs
            since = (datetime.utcnow() - timedelta(minutes=5)).isoformat() + "Z"
            
            # This is a simplified example - you'd need to iterate through
            # teams/channels the bot has access to
            response = requests.get(
                f"{GRAPH_API_BASE}/me/chats",
                headers=headers,
                params={
                    "$expand": "messages",
                    "$filter": f"lastMessageReceivedDateTime ge {since}"
                }
            )
            
            if response.status_code == 200:
                chats = response.json().get("value", [])
                for chat in chats:
                    self.process_chat_messages(chat, headers)
            else:
                logger.error(f"Failed to get chats: {response.status_code} - {response.text}")
                
        except Exception as e:
            logger.error(f"Error polling messages: {e}")
    
    def process_chat_messages(self, chat: Dict, headers: Dict):
        """Process messages from a chat."""
        chat_id = chat.get("id")
        
        # Get recent messages from this chat
        response = requests.get(
            f"{GRAPH_API_BASE}/chats/{chat_id}/messages",
            headers=headers,
            params={
                "$top": 10,  # Last 10 messages
                "$orderby": "createdDateTime desc"
            }
        )
        
        if response.status_code == 200:
            messages = response.json().get("value", [])
            
            for message in messages:
                message_id = message.get("id")
                
                # Skip if already processed
                if message_id in self.processed_messages:
                    continue
                
                # Check if bot is mentioned
                if self.is_bot_mentioned(message):
                    self.handle_mention(chat_id, message, headers)
                    self.processed_messages.add(message_id)
                    
                    # Limit memory usage
                    if len(self.processed_messages) > 1000:
                        self.processed_messages = set(list(self.processed_messages)[-500:])
    
    def is_bot_mentioned(self, message: Dict) -> bool:
        """Check if the bot is mentioned in the message."""
        mentions = message.get("mentions", [])
        
        for mention in mentions:
            mentioned_id = mention.get("mentioned", {}).get("user", {}).get("id")
            if mentioned_id == TEAMS_USER_ID:
                return True
        
        # Also check for @bot in content
        content = message.get("body", {}).get("content", "")
        return "@claude" in content.lower() or f"<at>{TEAMS_USER_ID}</at>" in content
    
    def handle_mention(self, chat_id: str, message: Dict, headers: Dict):
        """Handle a message mentioning the bot."""
        user = message.get("from", {}).get("user", {})
        user_name = user.get("displayName", "Unknown")
        user_id = user.get("id")
        
        # Extract text content
        content = message.get("body", {}).get("content", "")
        prompt = self.clean_message_content(content)
        
        if not prompt:
            return
        
        logger.info(f"User {user_name} requested: {prompt[:100]}...")
        
        # Send typing indicator (if supported)
        self.send_typing_indicator(chat_id, headers)
        
        try:
            # Execute Claude
            result = self.execute_claude(prompt)
            
            # Send response
            self.send_message(chat_id, result, headers)
            
        except subprocess.TimeoutExpired:
            self.send_message(chat_id, "⏱️ Claude took too long to respond.", headers)
        except Exception as e:
            logger.error(f"Error executing Claude: {e}")
            self.send_message(chat_id, f"❌ Error: {str(e)}", headers)
    
    def execute_claude(self, prompt: str) -> str:
        """Execute Claude Code CLI with restrictions."""
        cmd = [
            CLAUDE_COMMAND,
            "--model", CLAUDE_MODEL,
            "--allowedTools", ALLOWED_TOOLS,
            "-p", prompt  # -p flag with prompt
        ]
        
        logger.info(f"Executing: {' '.join(cmd)}")
        
        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=TIMEOUT_SECONDS,
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
    
    def send_message(self, chat_id: str, content: str, headers: Dict):
        """Send a message to Teams chat."""
        # Format as code block if it looks like code
        if "```" in content or len(content.split('\n')) > 3:
            body_content = f"<pre>{content}</pre>"
        else:
            body_content = content
        
        message = {
            "body": {
                "contentType": "html",
                "content": body_content
            }
        }
        
        response = requests.post(
            f"{GRAPH_API_BASE}/chats/{chat_id}/messages",
            headers=headers,
            json=message
        )
        
        if response.status_code != 201:
            logger.error(f"Failed to send message: {response.status_code} - {response.text}")
    
    def send_typing_indicator(self, chat_id: str, headers: Dict):
        """Send typing indicator (if supported by Graph API)."""
        # Note: This might not be available in all API versions
        pass
    
    def clean_message_content(self, content: str) -> str:
        """Clean message content to extract prompt."""
        # Remove HTML tags
        import re
        content = re.sub('<[^>]+>', '', content)
        
        # Remove bot mentions
        content = re.sub(r'@claude\s*', '', content, flags=re.IGNORECASE)
        
        return content.strip()


def main():
    """Main entry point."""
    # Validate environment
    required_vars = ["AZURE_TENANT_ID", "AZURE_CLIENT_ID", "AZURE_CLIENT_SECRET"]
    missing = [var for var in required_vars if not os.environ.get(var)]
    
    if missing:
        logger.error(f"Missing required environment variables: {missing}")
        return
    
    # Start bot
    bot = TeamsClaudeBot()
    try:
        bot.start()
    except KeyboardInterrupt:
        logger.info("Bot stopped by user")
    except Exception as e:
        logger.error(f"Bot crashed: {e}", exc_info=True)


if __name__ == "__main__":
    main()