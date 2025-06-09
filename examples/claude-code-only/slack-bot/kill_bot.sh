#!/bin/bash
# Kill script for the Slack bot

echo "Finding slack_claude_bot.py processes..."
PIDS=$(ps aux | grep "[s]lack_claude_bot.py" | awk '{print $2}')

if [ -z "$PIDS" ]; then
    echo "No slack_claude_bot.py processes found"
else
    echo "Found PIDs: $PIDS"
    echo "Killing processes..."
    for PID in $PIDS; do
        kill -9 $PID
        echo "Killed PID $PID"
    done
fi

# Also kill any Python processes that might be stuck
echo "Checking for stuck Python processes with Slack..."
SLACK_PIDS=$(ps aux | grep -i "[p]ython.*slack" | awk '{print $2}')
if [ ! -z "$SLACK_PIDS" ]; then
    echo "Found Slack-related Python PIDs: $SLACK_PIDS"
    echo "Kill these too? (y/n)"
    read -r response
    if [ "$response" = "y" ]; then
        for PID in $SLACK_PIDS; do
            kill -9 $PID
            echo "Killed PID $PID"
        done
    fi
fi

echo "Done"