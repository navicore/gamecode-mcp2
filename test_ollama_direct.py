#!/usr/bin/env python3
import time
import subprocess
import json

# Test 1: Simple curl command (like interactive chat)
print("Test 1: Simple request (like interactive chat)")
start = time.time()
result = subprocess.run([
    'curl', '-s', '-X', 'POST', 'http://localhost:11434/api/generate',
    '-H', 'Content-Type: application/json',
    '-d', json.dumps({
        "model": "qwen3:14b",
        "prompt": "List files in current directory and check if README.md exists",
        "stream": False
    })
], capture_output=True, text=True)
end = time.time()
response = json.loads(result.stdout)
print(f"Time: {end - start:.1f}s")
print(f"Response length: {len(response.get('response', ''))} chars")
print(f"First 100 chars: {response.get('response', '')[:100]}...")

# Test 2: With our exact parameters
print("\n\nTest 2: With our integration parameters")
start = time.time()
result = subprocess.run([
    'curl', '-s', '-X', 'POST', 'http://localhost:11434/api/generate',
    '-H', 'Content-Type: application/json',
    '-d', json.dumps({
        "model": "qwen3:14b",
        "prompt": """You are a helpful AI assistant with access to tools. Use the available tools to help answer user questions and complete tasks. Always validate your tool parameters match the schema before calling.

IMPORTANT: Do NOT include your thinking process in your response. Only output your final answer or tool calls.

Available tools:
- list_files: List files in a directory
  Parameters: {
  "properties": {
    "path": {
      "description": "The directory path to list files from",
      "type": "string"
    }
  },
  "required": [
    "path"
  ],
  "type": "object"
}

To use a tool, output EXACTLY this JSON format on its own line:
{"tool": "tool_name", "params": {"param1": "value1"}}

User: List files in current directory and check if README.md exists
Assistant: """,
        "temperature": 0.7,
        "stream": False
    })
], capture_output=True, text=True)
end = time.time()
if result.stdout:
    response = json.loads(result.stdout)
    print(f"Time: {end - start:.1f}s")
    print(f"Response length: {len(response.get('response', ''))} chars")
else:
    print(f"Failed after {end - start:.1f}s")
    print(f"Error: {result.stderr}")

# Test 3: Check if model is already loaded
print("\n\nTest 3: Model load status")
result = subprocess.run([
    'curl', '-s', 'http://localhost:11434/api/show',
    '-d', json.dumps({"name": "qwen3:14b"})
], capture_output=True, text=True)
show_response = json.loads(result.stdout)
print(f"Model loaded: {'model_info' in show_response}")

# Test 4: Quick generation test
print("\n\nTest 4: Minimal request")
start = time.time()
result = subprocess.run([
    'curl', '-s', '-X', 'POST', 'http://localhost:11434/api/generate',
    '-H', 'Content-Type: application/json',
    '-d', json.dumps({
        "model": "qwen3:14b",
        "prompt": "Say hello",
        "stream": False,
        "options": {
            "num_predict": 10
        }
    })
], capture_output=True, text=True)
end = time.time()
if result.stdout:
    response = json.loads(result.stdout)
    print(f"Time: {end - start:.1f}s")
    print(f"Response: {response.get('response', '')}")