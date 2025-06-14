#!/bin/bash

echo "Testing Ollama directly with curl..."

# Test 1: Simple request
echo -e "\n1. Simple request:"
time curl -s -X POST http://localhost:11434/api/generate \
  -H "Content-Type: application/json" \
  -d '{
    "model": "qwen3:14b",
    "prompt": "Say hello",
    "stream": false
  }' | jq -r '.response' | head -c 100

# Test 2: Request with our exact parameters
echo -e "\n\n2. Request matching our code:"
time curl -s -X POST http://localhost:11434/api/generate \
  -H "Content-Type: application/json" \
  -d '{
    "model": "qwen3:14b",
    "prompt": "List files in current directory",
    "temperature": 0.7,
    "stream": false,
    "num_predict": 4096,
    "stop": []
  }' | jq -r '.response' | head -c 100

# Test 3: Check if model is loaded
echo -e "\n\n3. Checking model status:"
curl -s http://localhost:11434/api/tags | jq '.models[] | select(.name=="qwen3:14b")'