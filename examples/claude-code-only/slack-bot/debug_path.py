#!/usr/bin/env python3
"""Debug PATH issues with subprocess"""
import os
import subprocess
import shutil

print("=== PATH Debug ===")
print(f"Python's os.environ PATH:\n{os.environ.get('PATH', 'NOT SET')}\n")

print("=== Shell PATH ===")
shell_result = subprocess.run(["echo", "$PATH"], shell=True, capture_output=True, text=True)
print(f"Shell PATH:\n{shell_result.stdout}\n")

print("=== Which claude ===")
# Using shutil.which (Python's equivalent of 'which')
claude_path = shutil.which("claude")
print(f"shutil.which('claude'): {claude_path}\n")

# Try shell which
which_result = subprocess.run(["which", "claude"], capture_output=True, text=True)
print(f"subprocess which claude: {which_result.stdout.strip()}")
print(f"stderr: {which_result.stderr.strip()}\n")

# Try common locations
common_paths = [
    "/usr/local/bin/claude",
    "/opt/homebrew/bin/claude",
    "/Applications/Claude.app/Contents/MacOS/claude",
    os.path.expanduser("~/.local/bin/claude"),
    os.path.expanduser("~/bin/claude"),
]

print("=== Checking common locations ===")
for path in common_paths:
    exists = os.path.exists(path)
    executable = os.access(path, os.X_OK) if exists else False
    print(f"{path}: exists={exists}, executable={executable}")