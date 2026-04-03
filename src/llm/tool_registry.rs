use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

pub struct ToolResult {
    pub id: String,
    pub output: String,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn schema(&self) -> serde_json::Value;
    async fn execute(&self, arguments: &str) -> String;
}

#[derive(Clone)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut reg = Self { tools: HashMap::new() };
        let echo = Arc::new(EchoTool);
        reg.tools.insert(echo.name().to_string(), echo);
        reg
    }

    pub fn get_schemas(&self) -> Vec<serde_json::Value> {
        self.tools.values().map(|t| t.schema()).collect()
    }

    pub async fn execute(&self, call: &ToolCall) -> ToolResult {
        match self.tools.get(&call.name) {
            Some(tool) => {
                let output = tool.execute(&call.arguments).await;
                ToolResult { id: call.id.clone(), output }
            }
            None => ToolResult {
                id: call.id.clone(),
                output: format!("error: unknown tool '{}'", call.name),
            },
        }
    }
}

struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "echo",
                "description": "Echoes back the provided text",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "text": { "type": "string", "description": "Text to echo" }
                    },
                    "required": ["text"]
                }
            }
        })
    }

    async fn execute(&self, arguments: &str) -> String {
        match serde_json::from_str::<serde_json::Value>(arguments) {
            Ok(v) => v
                .get("text")
                .and_then(|t| t.as_str())
                .unwrap_or("error: missing text field")
                .to_string(),
            Err(_) => "error: invalid arguments".to_string(),
        }
    }
}
