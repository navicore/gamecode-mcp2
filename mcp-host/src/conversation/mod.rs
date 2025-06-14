use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct ConversationManager {
    messages: VecDeque<Message>,
    max_context_tokens: usize,
    current_tokens: usize,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
    pub token_count: usize,
    pub tool_calls: Option<Vec<crate::prompts::ToolCall>>,
    pub tool_results: Option<Vec<ToolResult>>,
}

#[derive(Debug, Clone)]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub tool_name: String,
    pub success: bool,
    pub result: serde_json::Value,
}

impl ConversationManager {
    pub fn new(max_context_tokens: usize) -> Self {
        Self {
            messages: VecDeque::new(),
            max_context_tokens,
            current_tokens: 0,
        }
    }

    pub fn add_user_message(&mut self, content: &str) {
        self.add_message(Role::User, content, None, None);
    }

    pub fn add_assistant_message(&mut self, content: &str) {
        self.add_message(Role::Assistant, content, None, None);
    }

    pub fn add_assistant_message_with_tools(
        &mut self,
        content: &str,
        tool_calls: Vec<crate::prompts::ToolCall>,
        tool_results: Vec<ToolResult>,
    ) {
        self.add_message(
            Role::Assistant,
            content,
            Some(tool_calls),
            Some(tool_results),
        );
    }

    pub fn add_system_message(&mut self, content: &str) {
        // System messages are always kept at the beginning
        let message = Message {
            role: Role::System,
            content: content.to_string(),
            token_count: self.estimate_tokens(content),
            tool_calls: None,
            tool_results: None,
        };

        // Remove any existing system message
        self.messages.retain(|m| !matches!(m.role, Role::System));

        // Add at the beginning
        self.messages.push_front(message);
        self.recalculate_tokens();
    }

    fn add_message(
        &mut self,
        role: Role,
        content: &str,
        tool_calls: Option<Vec<crate::prompts::ToolCall>>,
        tool_results: Option<Vec<ToolResult>>,
    ) {
        let token_count = self.estimate_tokens(content);

        let message = Message {
            role,
            content: content.to_string(),
            token_count,
            tool_calls,
            tool_results,
        };

        self.messages.push_back(message);
        self.current_tokens += token_count;

        // Trim old messages if we exceed token limit
        self.trim_to_fit();
    }

    fn trim_to_fit(&mut self) {
        while self.current_tokens > self.max_context_tokens && self.messages.len() > 1 {
            // Keep system messages, remove oldest non-system message
            if let Some(pos) = self
                .messages
                .iter()
                .position(|m| !matches!(m.role, Role::System))
            {
                if let Some(removed) = self.messages.remove(pos) {
                    self.current_tokens = self.current_tokens.saturating_sub(removed.token_count);
                }
            } else {
                break;
            }
        }
    }

    fn recalculate_tokens(&mut self) {
        self.current_tokens = self.messages.iter().map(|m| m.token_count).sum();
    }

    fn estimate_tokens(&self, content: &str) -> usize {
        // Simple estimation: ~4 characters per token on average
        // This should be replaced with proper tokenization for production
        content.len() / 4
    }

    pub fn get_conversation_history(&self) -> Vec<(String, String)> {
        self.messages
            .iter()
            .filter_map(|msg| match msg.role {
                Role::User => Some(("User".to_string(), msg.content.clone())),
                Role::Assistant => Some(("Assistant".to_string(), msg.content.clone())),
                Role::System => None, // System messages handled separately
            })
            .collect()
    }

    pub fn get_recent_messages(&self, count: usize) -> Vec<&Message> {
        self.messages.iter().rev().take(count).rev().collect()
    }

    pub fn clear(&mut self) {
        // Keep system messages
        self.messages.retain(|m| matches!(m.role, Role::System));
        self.recalculate_tokens();
    }

    pub fn get_tool_history(&self) -> Vec<(Vec<crate::prompts::ToolCall>, Vec<ToolResult>)> {
        self.messages
            .iter()
            .filter_map(|msg| match (&msg.tool_calls, &msg.tool_results) {
                (Some(calls), Some(results)) => Some((calls.clone(), results.clone())),
                _ => None,
            })
            .collect()
    }
}
