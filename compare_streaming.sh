#!/bin/bash

echo "Comparing streaming vs non-streaming..."

echo -e "\n1. Non-streaming (what we use now):"
time curl -s -X POST http://localhost:11434/api/generate \
  -H "Content-Type: application/json" \
  -d '{
    "model": "qwen3:14b",
    "prompt": "List 3 colors",
    "stream": false
  }' | jq -r '.response' | head -c 200

echo -e "\n\n2. Streaming (collecting all chunks):"
time curl -s -X POST http://localhost:11434/api/generate \
  -H "Content-Type: application/json" \
  -d '{
    "model": "qwen3:14b",
    "prompt": "List 3 colors",
    "stream": true
  }' | jq -rs 'map(.response) | join("")' | head -c 200