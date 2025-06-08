#!/usr/bin/env python3
"""
Simpler Teams bot using Outgoing Webhooks (requires one endpoint).

This is simpler than Graph API polling but requires exposing an endpoint.
However, it can be an internal endpoint not exposed to the internet.

Requirements:
- pip install flask python-dotenv
- Claude Code CLI installed
- Teams Outgoing Webhook configured
"""

import os
import subprocess
import hmac
import hashlib
import base64
import json
import logging
from flask import Flask, request, jsonify
from dotenv import load_dotenv

load_dotenv(override=False)

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

app = Flask(__name__)

# Teams Outgoing Webhook secret for HMAC validation
TEAMS_WEBHOOK_SECRET = os.environ.get("TEAMS_WEBHOOK_SECRET", "").encode()

# Claude Configuration
CLAUDE_COMMAND = os.environ.get("CLAUDE_COMMAND", "claude")
ALLOWED_TOOLS = os.environ.get("CLAUDE_ALLOWED_TOOLS", "gamecode-mcp2:*")
TIMEOUT_SECONDS = int(os.environ.get("CLAUDE_TIMEOUT", "30"))


@app.route("/webhook", methods=["POST"])
def webhook():
    """Handle Teams Outgoing Webhook requests."""
    # Validate HMAC if secret is configured
    if TEAMS_WEBHOOK_SECRET:
        auth_header = request.headers.get("Authorization", "")
        if not validate_hmac(auth_header, request.data):
            return jsonify({"error": "Unauthorized"}), 403
    
    data = request.json
    
    # Extract message details
    text = data.get("text", "")
    user_name = data.get("from", {}).get("name", "Unknown")
    
    # Remove bot mention from text
    prompt = text.replace("<at>Claude</at>", "").strip()
    
    logger.info(f"User {user_name} requested: {prompt[:100]}...")
    
    try:
        # Execute Claude
        result = execute_claude(prompt)
        
        # Return response in Teams format
        return jsonify({
            "type": "message",
            "text": result
        })
        
    except Exception as e:
        logger.error(f"Error: {e}")
        return jsonify({
            "type": "message",
            "text": f"Error: {str(e)}"
        })


def validate_hmac(auth_header: str, payload: bytes) -> bool:
    """Validate Teams HMAC signature."""
    if not auth_header.startswith("HMAC "):
        return False
    
    provided_hash = auth_header[5:]  # Remove "HMAC " prefix
    
    # Calculate expected hash
    expected_hash = base64.b64encode(
        hmac.new(TEAMS_WEBHOOK_SECRET, payload, hashlib.sha256).digest()
    ).decode()
    
    return hmac.compare_digest(provided_hash, expected_hash)


def execute_claude(prompt: str) -> str:
    """Execute Claude with restrictions."""
    cmd = [
        CLAUDE_COMMAND,
        "-p",  # Non-interactive mode
        "--allowedTools", ALLOWED_TOOLS,
        prompt  # Prompt goes last
    ]
    
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
            return f"Claude error: {result.stderr.strip()}"
            
    except subprocess.TimeoutExpired:
        return "⏱️ Claude took too long to respond."
    except Exception as e:
        return f"Error executing Claude: {str(e)}"


if __name__ == "__main__":
    # This can run on internal network only
    app.run(host="0.0.0.0", port=5000)
    
    # For production, use gunicorn:
    # gunicorn -w 2 -b 0.0.0.0:5000 teams_bot_simple:app