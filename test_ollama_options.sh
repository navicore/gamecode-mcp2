#!/bin/bash

echo "Testing different Ollama options to match UI performance..."

# Test with very aggressive limits
echo -e "\n1. Aggressive limits (num_predict=50, repeat_penalty):"
time curl -s -X POST http://localhost:11434/api/generate \
  -H "Content-Type: application/json" \
  -d '{
    "model": "magistral:24b",
    "prompt": "List 3 colors",
    "stream": false,
    "options": {
      "num_predict": 50,
      "temperature": 0.7,
      "repeat_penalty": 1.1,
      "stop": ["\n\n", "User:", "Human:"]
    }
  }' | jq -r '.response'

# Test with streaming to see token generation rate
echo -e "\n\n2. Streaming to check token rate:"
echo "First 10 tokens:"
time curl -s -X POST http://localhost:11434/api/generate \
  -H "Content-Type: application/json" \
  -d '{
    "model": "magistral:24b",
    "prompt": "Count from 1 to 10",
    "stream": true
  }' 2>/dev/null | head -10 | jq -r '.response' | tr -d '\n'