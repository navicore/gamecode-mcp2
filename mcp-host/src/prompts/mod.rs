use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool: String,
    pub params: Value,
}

#[derive(Debug, Clone)]
pub struct PromptTemplate {
    system_prompt: String,
    tool_format: ToolFormat,
}

#[derive(Debug, Clone)]
pub enum ToolFormat {
    JsonOnly,
    NativeTools,
    Custom(String),
}

impl PromptTemplate {
    pub fn new(model_name: &str) -> Self {
        let (system_prompt, tool_format) = match model_name {
            "llama3.1:70b" | "llama3.1:8b" => (
                Self::llama_system_prompt(),
                ToolFormat::NativeTools,
            ),
            "mistral:7b-instruct" => (
                Self::mistral_system_prompt(),
                ToolFormat::JsonOnly,
            ),
            _ => (
                Self::generic_system_prompt(),
                ToolFormat::JsonOnly,
            ),
        };

        Self {
            system_prompt,
            tool_format,
        }
    }

    pub fn format_with_tools(&self, tools: &[mcp_client::protocol::Tool], conversation: &[(String, String)], user_message: &str) -> String {
        let mut prompt = self.system_prompt.clone();
        
        // Add tool definitions
        prompt.push_str("\n\nAvailable tools:\n");
        for tool in tools {
            prompt.push_str(&format!(
                "- {}: {}\n  Parameters: {}\n",
                tool.name,
                tool.description,
                serde_json::to_string_pretty(&tool.input_schema).unwrap_or_default()
            ));
        }

        // Add tool usage instructions
        prompt.push_str(&self.tool_usage_instructions());

        // Add conversation history
        if !conversation.is_empty() {
            prompt.push_str("\n\nConversation history:\n");
            for (role, message) in conversation {
                prompt.push_str(&format!("{}: {}\n", role, message));
            }
        }

        // Add current message
        prompt.push_str(&format!("\nUser: {}\nAssistant: ", user_message));
        
        prompt
    }

    pub fn parse_tool_calls(&self, response: &str) -> Result<Vec<ToolCall>> {
        match &self.tool_format {
            ToolFormat::JsonOnly => self.parse_json_tool_calls(response),
            ToolFormat::NativeTools => {
                // Native tool support would be handled differently
                self.parse_json_tool_calls(response)
            }
            ToolFormat::Custom(pattern) => self.parse_custom_tool_calls(response, pattern),
        }
    }

    fn parse_json_tool_calls(&self, response: &str) -> Result<Vec<ToolCall>> {
        let mut tool_calls = Vec::new();
        
        // Look for JSON blocks in the response
        let json_regex = regex::Regex::new(r"\{[^{}]*\"tool\"[^{}]*\}")?;
        
        for capture in json_regex.find_iter(response) {
            match serde_json::from_str::<ToolCall>(capture.as_str()) {
                Ok(tool_call) => tool_calls.push(tool_call),
                Err(e) => {
                    tracing::debug!("Failed to parse tool call JSON: {}", e);
                }
            }
        }
        
        Ok(tool_calls)
    }

    fn parse_custom_tool_calls(&self, response: &str, pattern: &str) -> Result<Vec<ToolCall>> {
        // Implement custom parsing based on pattern
        todo!("Custom tool call parsing")
    }

    fn tool_usage_instructions(&self) -> &str {
        match &self.tool_format {
            ToolFormat::JsonOnly => {
                "\nTo use a tool, output EXACTLY this JSON format on its own line:\n\
                {\"tool\": \"tool_name\", \"params\": {\"param1\": \"value1\"}}\n\
                \n\
                Important:\n\
                - Output the JSON on its own line\n\
                - Ensure the JSON is valid\n\
                - Use the exact parameter names from the tool schema\n\
                - You can use multiple tools in one response"
            }
            ToolFormat::NativeTools => {
                "\nYou have access to tools. Use them when needed to help answer the user's request."
            }
            ToolFormat::Custom(instructions) => instructions,
        }
    }

    fn generic_system_prompt() -> String {
        "You are a helpful AI assistant with access to tools. \
        Use the available tools to help answer user questions and complete tasks. \
        Always validate your tool parameters match the schema before calling.".to_string()
    }

    fn llama_system_prompt() -> String {
        "You are a helpful AI assistant. You have access to a set of tools to help answer questions and complete tasks. \
        When you need to use a tool, make sure to validate the parameters match the required schema.".to_string()
    }

    fn mistral_system_prompt() -> String {
        "You are a helpful AI assistant with tool access. When using tools, output valid JSON following the specified format. \
        Think step by step and use tools when they would help provide better answers.".to_string()
    }

    pub fn validate_response(&self, response: &str) -> ValidationResult {
        let tool_calls = self.parse_tool_calls(response).unwrap_or_default();
        
        ValidationResult {
            is_valid: !response.is_empty(),
            has_tool_calls: !tool_calls.is_empty(),
            tool_calls,
            errors: vec![],
        }
    }
}

#[derive(Debug)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub has_tool_calls: bool,
    pub tool_calls: Vec<ToolCall>,
    pub errors: Vec<String>,
}