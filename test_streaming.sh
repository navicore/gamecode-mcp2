#!/bin/bash

echo "Testing Ollama streaming..."
echo "Each line is a separate JSON object when streaming:"
echo

curl -X POST http://localhost:11434/api/generate \
  -H "Content-Type: application/json" \
  -d '{
    "model": "qwen3:14b",
    "prompt": "Count to 5 slowly",
    "stream": true
  }' 2>/dev/null | head -10