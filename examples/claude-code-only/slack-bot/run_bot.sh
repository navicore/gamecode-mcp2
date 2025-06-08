#!/bin/bash
# Wrapper script that ensures Python inherits full shell environment

# Source your shell profile to get all environment variables
# Adjust based on your shell (bash vs zsh)
if [ -f ~/.zshrc ]; then
    source ~/.zshrc
elif [ -f ~/.bashrc ]; then
    source ~/.bashrc
fi

# Export any additional vars Claude might need
# These are just examples - you might not need all of them
export HOME=${HOME}
export USER=${USER}
export PATH=${PATH}

# Show what we're running with (for debugging)
echo "Running bot with:"
echo "PATH=$PATH"
echo "HOME=$HOME"
echo "Claude command: $(which claude)"

# Run the Python bot with full environment
exec python slack_claude_bot.py "$@"