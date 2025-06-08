#!/usr/bin/env python3
"""Debug environment differences between shell and Python subprocess"""
import subprocess
import os

print("=== Direct Python environment ===")
print(f"PATH: {os.environ.get('PATH', 'NOT SET')}\n")

print("=== Shell environment (zsh -l -c) ===")
result = subprocess.run(
    ["zsh", "-l", "-c", "echo PATH=$PATH && which claude && claude --version"],
    capture_output=True,
    text=True
)
print(f"stdout:\n{result.stdout}")
print(f"stderr:\n{result.stderr}")
print(f"return code: {result.returncode}\n")

print("=== Interactive shell test ===")
# Try with -i for interactive
result = subprocess.run(
    ["zsh", "-i", "-c", "echo PATH=$PATH && which claude"],
    capture_output=True,
    text=True
)
print(f"stdout:\n{result.stdout}")
print(f"stderr:\n{result.stderr}")

print("\n=== Environment variables that might matter ===")
for key in sorted(os.environ.keys()):
    if any(x in key.upper() for x in ['CLAUDE', 'PATH', 'HOME', 'SHELL', 'ANTHROPIC', 'CONFIG']):
        print(f"{key}={os.environ[key]}")