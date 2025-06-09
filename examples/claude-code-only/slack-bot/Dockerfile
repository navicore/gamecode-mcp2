# Dockerfile for Claude Slack Bot
# Multi-stage build for security and size optimization

# Build stage - install dependencies
FROM python:3.11-slim as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Copy requirements and install Python dependencies
COPY requirements.txt .
RUN pip install --user --no-cache-dir -r requirements.txt

# Runtime stage
FROM python:3.11-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    # For Claude Code CLI
    curl \
    # For tools that might need these
    git \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 botuser

WORKDIR /app

# Copy Python dependencies from builder
COPY --from=builder /root/.local /home/botuser/.local

# Copy application
COPY --chown=botuser:botuser slack_claude_bot.py .

# Install Claude Code CLI (adjust based on your installation method)
# This is an example - replace with actual Claude Code installation
# RUN curl -fsSL https://claude.ai/install.sh | sh
# OR copy from your build context if you have it locally
# COPY --from=claude-installer /usr/local/bin/claude /usr/local/bin/claude

# Switch to non-root user
USER botuser

# Ensure Python can find installed packages
ENV PATH=/home/botuser/.local/bin:$PATH

# Note: No .env file copied - configuration comes from environment
# This makes the image stateless and suitable for K8s

# Run the bot
CMD ["python", "slack_claude_bot.py"]

# Optional: Add health check
# HEALTHCHECK --interval=30s --timeout=3s --start-period=40s --retries=3 \
#   CMD python -c "import requests; requests.get('http://localhost:8080/health')"