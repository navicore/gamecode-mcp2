#!/usr/bin/env python3
"""
Test script to debug Claude CLI syntax
"""
import subprocess
import sys

def test_claude_syntax(prompt):
    """Test different Claude CLI syntax variations"""
    
    # Test different command formats
    variations = [
        # Format 1: -p as flag with prompt as next arg
        ["claude", "--model", "sonnet", "--allowedTools", "mcp__gamecode__read_file", "-p", prompt],
        
        # Format 2: -p=prompt combined
        ["claude", "--model", "sonnet", "--allowedTools", "mcp__gamecode__read_file", f"-p={prompt}"],
        
        # Format 3: prompt at the very end without -p
        ["claude", "--model", "sonnet", "--allowedTools", "mcp__gamecode__read_file", prompt],
        
        # Format 4: Using shell with proper quoting
        f'claude --model sonnet --allowedTools "mcp__gamecode__read_file" -p "{prompt}"'
    ]
    
    for i, cmd in enumerate(variations):
        print(f"\n--- Test {i+1} ---")
        
        if isinstance(cmd, str):
            # Shell command
            print(f"Shell command: {cmd}")
            result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
        else:
            # List command
            import shlex
            print(f"Command: {shlex.join(cmd)}")
            result = subprocess.run(cmd, capture_output=True, text=True)
        
        print(f"Return code: {result.returncode}")
        print(f"Stdout: {result.stdout[:100]}...")
        print(f"Stderr: {result.stderr[:100]}...")

if __name__ == "__main__":
    test_prompt = sys.argv[1] if len(sys.argv) > 1 else "List files in current directory"
    test_claude_syntax(test_prompt)