#!/usr/bin/env python3
import time
import subprocess
import json

print("Comparing /api/chat vs /api/generate endpoints\n")

# Test 1: Using /api/chat endpoint (what interactive UI might use)
print("Test 1: /api/chat endpoint")
start = time.time()
result = subprocess.run([
    'curl', '-s', '-X', 'POST', 'http://localhost:11434/api/chat',
    '-H', 'Content-Type: application/json',
    '-d', json.dumps({
        "model": "qwen3:14b",
        "messages": [
            {"role": "user", "content": "List files in current directory and check if README.md exists"}
        ],
        "stream": False
    })
], capture_output=True, text=True)
end = time.time()
if result.stdout:
    response = json.loads(result.stdout)
    print(f"Time: {end - start:.1f}s")
    msg = response.get('message', {}).get('content', '')
    print(f"Response length: {len(msg)} chars")
    print(f"First 200 chars: {msg[:200]}...")
else:
    print(f"Failed: {result.stderr}")

# Test 2: Using /api/generate with options
print("\n\nTest 2: /api/generate with options limiting response")
start = time.time()
result = subprocess.run([
    'curl', '-s', '-X', 'POST', 'http://localhost:11434/api/generate',
    '-H', 'Content-Type: application/json',
    '-d', json.dumps({
        "model": "qwen3:14b",
        "prompt": "List files in current directory and check if README.md exists",
        "stream": False,
        "options": {
            "temperature": 0.7,
            "top_k": 40,
            "top_p": 0.9,
            "num_predict": 200  # Reasonable limit
        }
    })
], capture_output=True, text=True)
end = time.time()
if result.stdout:
    response = json.loads(result.stdout)
    print(f"Time: {end - start:.1f}s")
    print(f"Response length: {len(response.get('response', ''))} chars")
else:
    print(f"Failed: {result.stderr}")

# Test 3: Check default Ollama settings
print("\n\nTest 3: Checking model info for defaults")
result = subprocess.run([
    'curl', '-s', 'http://localhost:11434/api/show',
    '-d', json.dumps({"name": "qwen3:14b"})
], capture_output=True, text=True)
if result.stdout:
    info = json.loads(result.stdout)
    params = info.get('parameters', '')
    if 'num_predict' in params:
        print("Model has num_predict default")
    print(f"Parameters preview: {params[:200]}...")