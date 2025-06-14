use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc;

/// Detailed event logging for debugging MCP integrations
#[derive(Debug, Clone, Serialize)]
pub struct McpEvent {
    pub timestamp: SystemTime,
    pub event_type: McpEventType,
    pub details: serde_json::Value,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum McpEventType {
    // Prompt enhancement
    PromptEnhancementStart {
        original_length: usize,
    },
    PromptEnhancementComplete {
        enhanced_length: usize,
        tools_added: usize,
    },
    
    // Tool detection
    ToolDetectionStart {
        response_length: usize,
    },
    ToolDetectionComplete {
        tools_found: Vec<String>,
    },
    
    // Tool execution
    ToolExecutionStart {
        tool_name: String,
        params: serde_json::Value,
    },
    ToolExecutionComplete {
        tool_name: String,
        success: bool,
        result_preview: String, // First 200 chars
    },
    ToolExecutionError {
        tool_name: String,
        error: String,
    },
    
    // LLM interaction (if using continuation)
    ContinuationRequest {
        tool_results_count: usize,
        prompt_preview: String,
    },
    ContinuationResponse {
        response_length: usize,
        response_preview: String,
    },
    
    // Performance metrics
    PerformanceMetric {
        metric_name: String,
        value: f64,
        unit: String,
    },
}

/// Instrumentation collector
pub struct InstrumentationCollector {
    tx: mpsc::UnboundedSender<McpEvent>,
}

impl InstrumentationCollector {
    pub fn new(tx: mpsc::UnboundedSender<McpEvent>) -> Self {
        Self { tx }
    }
    
    pub fn event(&self, event_type: McpEventType) {
        let event = McpEvent {
            timestamp: SystemTime::now(),
            event_type,
            details: serde_json::Value::Null,
            duration_ms: None,
        };
        let _ = self.tx.send(event);
    }
    
    pub fn event_with_details(&self, event_type: McpEventType, details: serde_json::Value) {
        let event = McpEvent {
            timestamp: SystemTime::now(),
            event_type,
            details,
            duration_ms: None,
        };
        let _ = self.tx.send(event);
    }
    
    pub fn timed_event(&self, event_type: McpEventType, duration: Duration) {
        let event = McpEvent {
            timestamp: SystemTime::now(),
            event_type,
            details: serde_json::Value::Null,
            duration_ms: Some(duration.as_millis() as u64),
        };
        let _ = self.tx.send(event);
    }
}

/// Instrumentation writer - writes events to file in JSONL format
pub struct InstrumentationWriter {
    rx: mpsc::UnboundedReceiver<McpEvent>,
    file_path: String,
}

impl InstrumentationWriter {
    pub fn new(rx: mpsc::UnboundedReceiver<McpEvent>, file_path: String) -> Self {
        Self { rx, file_path }
    }
    
    pub async fn run(mut self) -> Result<()> {
        use tokio::io::AsyncWriteExt;
        use tokio::fs::OpenOptions;
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
            .await?;
        
        while let Some(event) = self.rx.recv().await {
            let json = serde_json::to_string(&event)?;
            file.write_all(json.as_bytes()).await?;
            file.write_all(b"\n").await?;
            file.flush().await?;
        }
        
        Ok(())
    }
}

/// Helper to time operations
pub struct TimedOperation<'a> {
    collector: &'a InstrumentationCollector,
    event_type: McpEventType,
    start: std::time::Instant,
}

impl<'a> TimedOperation<'a> {
    pub fn new(collector: &'a InstrumentationCollector, event_type: McpEventType) -> Self {
        collector.event(event_type.clone());
        Self {
            collector,
            event_type,
            start: std::time::Instant::now(),
        }
    }
    
    pub fn complete(self, complete_event: McpEventType) {
        self.collector.timed_event(complete_event, self.start.elapsed());
    }
}

/// Analysis utilities for instrumentation logs
pub mod analysis {
    use super::*;
    use std::collections::HashMap;
    
    pub struct LogAnalyzer {
        events: Vec<McpEvent>,
    }
    
    impl LogAnalyzer {
        pub fn from_file(path: &str) -> Result<Self> {
            use std::io::{BufRead, BufReader};
            use std::fs::File;
            
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            let mut events = Vec::new();
            
            for line in reader.lines() {
                let line = line?;
                if let Ok(event) = serde_json::from_str::<McpEvent>(&line) {
                    events.push(event);
                }
            }
            
            Ok(Self { events })
        }
        
        pub fn tool_execution_stats(&self) -> HashMap<String, ToolStats> {
            let mut stats = HashMap::new();
            
            for event in &self.events {
                if let McpEventType::ToolExecutionComplete { tool_name, success, .. } = &event.event_type {
                    let entry = stats.entry(tool_name.clone()).or_insert(ToolStats::default());
                    entry.total_calls += 1;
                    if *success {
                        entry.successful_calls += 1;
                    }
                    if let Some(duration) = event.duration_ms {
                        entry.total_duration_ms += duration;
                        entry.max_duration_ms = entry.max_duration_ms.max(duration);
                        entry.min_duration_ms = entry.min_duration_ms.min(duration);
                    }
                }
            }
            
            stats
        }
        
        pub fn performance_timeline(&self) -> Vec<(SystemTime, String, u64)> {
            self.events
                .iter()
                .filter_map(|e| {
                    e.duration_ms.map(|d| {
                        let label = match &e.event_type {
                            McpEventType::ToolExecutionComplete { tool_name, .. } => {
                                format!("Tool: {}", tool_name)
                            }
                            McpEventType::PromptEnhancementComplete { .. } => {
                                "Prompt Enhancement".to_string()
                            }
                            _ => "Other".to_string()
                        };
                        (e.timestamp, label, d)
                    })
                })
                .collect()
        }
    }
    
    #[derive(Default, Debug)]
    pub struct ToolStats {
        pub total_calls: usize,
        pub successful_calls: usize,
        pub total_duration_ms: u64,
        pub max_duration_ms: u64,
        pub min_duration_ms: u64,
    }
}