// Minimal MCP server implementation for auditable LLM-to-system interaction.
// No external text processing - all JSON handling is explicit and traceable.

use anyhow::Result;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, error, info, warn};

mod handlers;
mod protocol;
mod tools;
mod validation;

use handlers::RequestHandler;
use protocol::*;
use tools::ToolManager;

#[tokio::main]
async fn main() -> Result<()> {
    // Handle command-line arguments
    let args: Vec<String> = std::env::args().collect();
    let mut tools_file_override: Option<String> = None;
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "--version" | "-V" => {
                println!("gamecode-mcp2 {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--tools-file" | "-t" => {
                if i + 1 < args.len() {
                    tools_file_override = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --tools-file requires an argument");
                    std::process::exit(1);
                }
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                eprintln!("Try 'gamecode-mcp2 --help' for more information.");
                std::process::exit(1);
            }
        }
    }
    
    // Tracing to stderr only - stdout is reserved for JSON-RPC protocol
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("mcp_server=info".parse().unwrap()),
        )
        .with_writer(std::io::stderr)
        .init();

    info!("Starting GameCode MCP Server v2...");

    // Tool loading with clear precedence
    let mut tool_manager = ToolManager::new();
    
    // Load tools with new precedence order
    if let Err(e) = tool_manager.load_with_precedence(tools_file_override).await {
        warn!("Failed to load tools: {}", e);
        warn!("The server will start but no tools will be available.");
    }

    let handler = RequestHandler::new(tool_manager);

    // Stdio is our only transport - no network, no files
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut stdout = stdout;

    info!("MCP server ready, waiting for requests...");

    // Single-threaded message loop - one request at a time
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                info!("Client disconnected");
                break;
            }
            Ok(_) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                debug!("Received: {}", line);

                // Parse as generic Value first - no implicit deserialization
                match serde_json::from_str::<Value>(line) {
                    Ok(value) => {
                        // Explicit request/notification discrimination by id field
                        if value.get("id").is_some() {
                            // It's a request
                            match serde_json::from_value::<JsonRpcRequest>(value) {
                                Ok(request) => {
                                    let response = handler.handle_request(request).await;
                                    let response_str = serde_json::to_string(&response)?;
                                    debug!("Sending: {}", response_str);
                                    stdout.write_all(response_str.as_bytes()).await?;
                                    stdout.write_all(b"\n").await?;
                                    stdout.flush().await?;
                                }
                                Err(e) => {
                                    error!("Invalid request: {}", e);
                                    let error_response = JsonRpcResponse {
                                        jsonrpc: "2.0".to_string(),
                                        id: serde_json::Value::Null,
                                        result: None,
                                        error: Some(JsonRpcError {
                                            code: INVALID_REQUEST,
                                            message: "Invalid request".to_string(),
                                            data: None,
                                        }),
                                    };
                                    let response_str = serde_json::to_string(&error_response)?;
                                    stdout.write_all(response_str.as_bytes()).await?;
                                    stdout.write_all(b"\n").await?;
                                    stdout.flush().await?;
                                }
                            }
                        } else {
                            // It's a notification
                            match serde_json::from_value::<JsonRpcNotification>(value) {
                                Ok(notification) => {
                                    handler.handle_notification(notification).await;
                                }
                                Err(e) => {
                                    error!("Invalid notification: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Parse error: {}", e);
                        let error_response = JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: serde_json::Value::Null,
                            result: None,
                            error: Some(JsonRpcError {
                                code: PARSE_ERROR,
                                message: "Parse error".to_string(),
                                data: None,
                            }),
                        };
                        let response_str = serde_json::to_string(&error_response)?;
                        stdout.write_all(response_str.as_bytes()).await?;
                        stdout.write_all(b"\n").await?;
                        stdout.flush().await?;
                    }
                }
            }
            Err(e) => {
                error!("Read error: {}", e);
                break;
            }
        }
    }

    info!("MCP server shutting down");
    Ok(())
}

fn print_help() {
    println!("gamecode-mcp2 {}", env!("CARGO_PKG_VERSION"));
    println!("{}", env!("CARGO_PKG_DESCRIPTION"));
    println!();
    println!("USAGE:");
    println!("    gamecode-mcp2 [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help               Print help information");
    println!("    -V, --version            Print version information");
    println!("    -t, --tools-file <FILE>  Specify tools configuration file");
    println!();
    println!("DESCRIPTION:");
    println!("    An MCP server that communicates via stdio (stdin/stdout).");
    println!("    Configure tools in tools.yaml or via GAMECODE_TOOLS_FILE.");
    println!("    ");
    println!("    This server is designed to be spawned by MCP clients like");
    println!("    Claude Desktop. It does not accept network connections.");
    println!();
    println!("ENVIRONMENT:");
    println!("    GAMECODE_TOOLS_FILE    Path to tools YAML configuration");
    println!("    GAMECODE_MODE          Load a specific mode/profile");
    println!("    RUST_LOG               Set logging level (default: info)");
    println!();
    println!("EXAMPLES:");
    println!("    # Run with default tool detection");
    println!("    gamecode-mcp2");
    println!();
    println!("    # Run with specific tools file");
    println!("    GAMECODE_TOOLS_FILE=~/my-tools.yaml gamecode-mcp2");
    println!();
    println!("    # Run in Python development mode");
    println!("    GAMECODE_MODE=python-dev gamecode-mcp2");
    println!();
    println!("MORE INFO:");
    println!("    Repository: {}", env!("CARGO_PKG_REPOSITORY"));
}
