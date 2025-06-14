#!/bin/bash

echo "Testing with magistral:24b (your chat model)..."

# Test 1: Simple request with magistral
echo -e "\n1. magistral:24b response time:"
time curl -s -X POST http://localhost:11434/api/generate \
  -H "Content-Type: application/json" \
  -d '{
    "model": "magistral:24b",
    "prompt": "List 3 colors",
    "stream": false,
    "options": {"num_predict": 100}
  }' | jq -r '.response' | head -c 200

# Test 2: Same request with qwen3:14b
echo -e "\n\n2. qwen3:14b response time (for comparison):"
time curl -s -X POST http://localhost:11434/api/generate \
  -H "Content-Type: application/json" \
  -d '{
    "model": "qwen3:14b",
    "prompt": "List 3 colors",
    "stream": false,
    "options": {"num_predict": 100}
  }' | jq -r '.response' | head -c 200

# Test 3: Check which models are loaded
echo -e "\n\n3. Currently loaded models:"
curl -s http://localhost:11434/api/ps | jq