// Input validation for LLM-provided arguments
// Minimal approach - document what we check and why

use anyhow::{bail, Result};
use serde_json::Value;

// Validate file paths - prevent directory traversal
pub fn validate_path(path: &str, allow_absolute: bool) -> Result<()> {
    // Reject null bytes (filesystem terminator)
    if path.contains('\0') {
        bail!("Path contains null byte");
    }
    
    // Reject path traversal patterns
    if path.contains("..") {
        bail!("Path traversal detected: '..' not allowed");
    }
    
    // Optionally reject absolute paths
    if !allow_absolute && (path.starts_with('/') || path.starts_with('~')) {
        bail!("Absolute paths not allowed");
    }
    
    Ok(())
}

// Validate command arguments for common injection patterns
pub fn validate_command_arg(arg: &str) -> Result<()> {
    // Reject null bytes
    if arg.contains('\0') {
        bail!("Argument contains null byte");
    }
    
    // These would only be dangerous with shell interpretation,
    // but checking them adds defense in depth
    const SUSPICIOUS_PATTERNS: &[&str] = &[
        "$(",      // Command substitution
        "`",       // Backtick substitution  
        "${",      // Variable expansion
        "&&",      // Command chaining
        "||",      // Command chaining
        ";",       // Command separator
        "|",       // Pipe
        ">",       // Redirect
        "<",       // Redirect
        "\n",      // Newline (command separator)
        "\r",      // Carriage return
    ];
    
    for pattern in SUSPICIOUS_PATTERNS {
        if arg.contains(pattern) {
            // Log but don't reject - these are safe without shell
            tracing::warn!("Suspicious pattern '{}' in argument: {}", pattern, arg);
        }
    }
    
    Ok(())
}

// Validate based on expected type
pub fn validate_typed_value(value: &Value, expected_type: &str) -> Result<()> {
    match (expected_type, value) {
        ("string", Value::String(s)) => {
            validate_command_arg(s)?;
        }
        ("number", Value::Number(_)) => {
            // Numbers are generally safe
        }
        ("boolean", Value::Bool(_)) => {
            // Booleans are safe
        }
        ("array", Value::Array(arr)) => {
            // Validate each element
            for item in arr {
                if let Value::String(s) = item {
                    validate_command_arg(s)?;
                }
            }
        }
        _ => {
            bail!("Type mismatch: expected {}, got {:?}", expected_type, value);
        }
    }
    Ok(())
}

// Rate limiting check (requires external state)
#[allow(dead_code)]
pub fn check_rate_limit(tool_name: &str, window_ms: u64) -> Result<()> {
    // This would need to be implemented with a time-based counter
    // For now, just document the interface
    tracing::debug!("Rate limit check for {} ({}ms window)", tool_name, window_ms);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_validation() {
        // Should pass
        assert!(validate_path("file.txt", false).is_ok());
        assert!(validate_path("dir/file.txt", false).is_ok());
        assert!(validate_path("/etc/passwd", true).is_ok());
        
        // Should fail
        assert!(validate_path("../etc/passwd", false).is_err());
        assert!(validate_path("/etc/passwd", false).is_err());
        assert!(validate_path("~/ssh/config", false).is_err());
        assert!(validate_path("file\0.txt", false).is_err());
    }

    #[test]
    fn test_command_validation() {
        // Should pass (but may log warnings)
        assert!(validate_command_arg("hello world").is_ok());
        assert!(validate_command_arg("--flag=value").is_ok());
        
        // Should pass but log warnings  
        assert!(validate_command_arg("test; ls").is_ok());
        assert!(validate_command_arg("$(whoami)").is_ok());
        
        // Should fail
        assert!(validate_command_arg("test\0null").is_err());
    }
}