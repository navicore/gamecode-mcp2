# MCP Host - Technical Implementation Details

## Design Rationale

### Why Prompt-Based Instead of Native Tool APIs?

Most LLMs either don't support tools or implement them inconsistently. By using a prompt-based approach with retry logic, we achieve:

1. **Universal compatibility** - Works with any instruction-following LLM
2. **Transparency** - Can debug exactly what the LLM sees
3. **Flexibility** - Can optimize prompts per model without API changes
4. **Learning** - Retry patterns can inform future prompt improvements

### The Retry Magic

The key insight: LLMs are quite good at fixing mistakes when shown what went wrong. Our retry mechanism exploits this by:

```
Attempt 1: LLM outputs malformed JSON
           ↓
Parse fails, capture error: "Expected closing brace"
           ↓
Attempt 2: Add error to prompt: "Your previous attempt failed: Expected closing brace"
           ↓
LLM fixes the JSON formatting
           ↓
Success!
```

## Implementation Deep Dive

### Tool Call Extraction

We use regex to find JSON blocks that look like tool calls:
```rust
r"\{[^{}]*\"tool\"[^{}]*\}"
```

This is intentionally simple - we want to be forgiving about where in the response the JSON appears.

### Temperature Reduction Strategy

Each retry reduces temperature by `temperature_reduction` (default 0.1):
- Attempt 1: 0.7 (creative, might hallucinate)
- Attempt 2: 0.6 (more focused)
- Attempt 3: 0.5 (deterministic)

This progressively makes the model more likely to follow instructions exactly.

### Prompt Engineering Per Model

Different models respond better to different prompt styles:

**Generic models**: Clear, explicit instructions
```
To use a tool, output EXACTLY this JSON format on its own line:
{"tool": "tool_name", "params": {"param1": "value1"}}
```

**Llama 3.1**: Can use native format or JSON
```
You have access to tools. Use them when needed to help answer the user's request.
```

**Code-focused models**: Frame as code task
```
Output a JSON object representing the function call:
```

### Error Context Building

The retry prompt includes:
1. All previous errors in order
2. Specific guidance based on error type
3. The full tool schema again (models can "forget")

Example retry context:
```
IMPORTANT: Previous attempts failed with these errors:
Attempt 1: Invalid JSON - missing comma after "path" parameter
Attempt 2: Unknown tool "list-files" - did you mean "list_files"?

Please correct these issues. Available tools:
- list_files: Lists files in a directory
  Parameters: {"path": "string"}
```

### Safety Boundaries

Tool calls are validated at multiple levels:
1. **JSON parsing** - Must be valid JSON
2. **Schema validation** - Parameters match tool schema  
3. **Safety filters** - Tool name not in blocked patterns
4. **Rate limiting** - Per-minute request limits
5. **Token limits** - Prevent context overflow

### Future Optimizations

1. **Caching successful patterns**: When a model successfully calls a tool, cache the exact format it used
2. **Model fingerprinting**: Detect which model variant is actually being used
3. **Adaptive prompting**: Start with minimal prompt, add detail only if needed
4. **Parallel retries**: Try different prompt strategies simultaneously

## Debugging Guide

### Common Issues

**Model outputs tool description instead of calling it:**
- Add examples to prompt
- Emphasize "USE the tool, don't describe it"

**JSON buried in explanation:**
- Make regex more forgiving
- Add "Output JSON on its own line"

**Model forgets tools exist:**
- Re-include tool list in retry prompt
- Reduce context by trimming history

**Inconsistent parameter names:**
- Show exact schema in error message
- Consider fuzzy matching with confirmation

### Testing New Models

1. Start with temperature 0.3 for initial testing
2. Use simple tools first (single parameter)
3. Log all attempts for pattern analysis
4. Build model-specific prompt template if needed

### Metrics to Track

- First-attempt success rate by model
- Average retries needed
- Most common error types
- Token usage per conversation

## Code Organization

- `lib.rs` - Core orchestration and API
- `llm/` - LLM provider implementations  
- `prompts/` - Model-specific prompt engineering
- `retry/` - Retry logic and context building
- `conversation/` - Message history management

Each module is designed to be independently testable and replaceable.

## Adding New LLM Providers

1. Implement `LlmProvider` trait
2. Add to `llm/mod.rs` 
3. Create model-specific `PromptTemplate` if needed
4. Test with various retry scenarios
5. Document model-specific quirks here

## Performance Considerations

- **Streaming**: Not implemented yet, would reduce latency
- **Caching**: Tool descriptions could be cached in prompt
- **Batching**: Multiple tool calls could be executed in parallel
- **Compression**: Old conversation turns could be summarized

## Security Notes

- Never execute tools without validation
- Tool parameters are untrusted input
- Rate limits prevent resource exhaustion  
- Consider adding authentication to MCP connection
- Log all tool executions for audit trail