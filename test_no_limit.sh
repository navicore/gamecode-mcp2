#!/bin/bash

echo "Testing without num_predict..."
time curl -s -X POST http://localhost:11434/api/generate \
  -H "Content-Type: application/json" \
  -d '{
    "model": "qwen3:14b",
    "prompt": "Please list all markdown files in the current directory and read the first one you find. To use a tool, output EXACTLY this JSON format on its own line: {\"tool\": \"tool_name\", \"params\": {\"param1\": \"value1\"}}",
    "temperature": 0.7,
    "stream": false
  }' | jq -r '.response' | head -c 500