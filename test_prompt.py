#!/usr/bin/env python3
import json
import requests
import time

# Test what Ollama actually returns for a multi-tool request
prompt = """You are a helpful AI assistant with access to tools. Use the available tools to help answer user questions and complete tasks. Always validate your tool parameters match the schema before calling.

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
- read_file: Read the contents of a file
  Parameters: {
  "properties": {
    "path": {
      "description": "The file path to read",
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

Important:
- Output the JSON on its own line
- Ensure the JSON is valid
- Use the exact parameter names from the tool schema
- You can use multiple tools in one response
User: Please list all markdown files in the current directory and read the first one you find.
Assistant: """

request_data = {
    "model": "qwen3:14b",
    "prompt": prompt,
    "stream": False,
    "options": {
        "temperature": 0.7,
        "num_predict": 500
    }
}

print("Sending request to Ollama...")
start = time.time()

try:
    response = requests.post(
        "http://localhost:11434/api/generate",
        json=request_data,
        timeout=30
    )
    
    elapsed = time.time() - start
    print(f"\nGot response in {elapsed:.1f}s")
    
    if response.status_code == 200:
        data = response.json()
        print("\nResponse text:")
        print("-" * 80)
        print(data['response'])
        print("-" * 80)
    else:
        print(f"Error: {response.status_code}")
        print(response.text)
        
except Exception as e:
    print(f"Error: {e}")