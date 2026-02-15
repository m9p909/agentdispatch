use askama::Template;
use serde::Serialize;
use uuid::Uuid;

// Layout
#[derive(Template)]
#[template(path = "layout.html")]
pub struct Layout;

// Providers
#[derive(Template)]
#[template(path = "providers/list.html")]
pub struct ProvidersList {
    pub providers: Vec<ProviderListItem>,
}

#[derive(Template)]
#[template(path = "providers/form.html")]
pub struct ProvidersForm {
    pub provider: Option<ProviderFormItem>,
    pub errors: Vec<String>,
}

#[derive(Serialize)]
pub struct ProviderListItem {
    pub id: Uuid,
    pub name: String,
    pub provider_type: String,
}

#[derive(Serialize)]
pub struct ProviderFormItem {
    pub id: Uuid,
    pub name: String,
    pub provider_type: String,
    pub base_url: Option<String>,
}

// Models
#[derive(Template)]
#[template(path = "models/list.html")]
pub struct ModelsList {
    pub models: Vec<ModelListItem>,
}

#[derive(Template)]
#[template(path = "models/form.html")]
pub struct ModelsForm {
    pub model: Option<ModelFormItem>,
    pub providers: Vec<ProviderFormItem>,
    pub errors: Vec<String>,
}

#[derive(Serialize)]
pub struct ModelListItem {
    pub id: Uuid,
    pub name: String,
    pub model_identifier: String,
    pub provider_id: Uuid,
}

#[derive(Serialize)]
pub struct ModelFormItem {
    pub id: Uuid,
    pub name: String,
    pub model_identifier: String,
    pub provider_id: Uuid,
}

// Agents
#[derive(Template)]
#[template(path = "agents/list.html")]
pub struct AgentsList {
    pub agents: Vec<AgentListItem>,
}

#[derive(Template)]
#[template(path = "agents/form.html")]
pub struct AgentsForm {
    pub agent: Option<AgentFormItem>,
    pub models: Vec<ModelFormItem>,
    pub errors: Vec<String>,
}

#[derive(Serialize)]
pub struct AgentListItem {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub model_id: Uuid,
}

#[derive(Serialize)]
pub struct AgentFormItem {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: String,
    pub model_id: Uuid,
}

// Sessions
#[derive(Template)]
#[template(path = "sessions/list.html")]
pub struct SessionsList {
    pub sessions: Vec<SessionListItem>,
}

#[derive(Template)]
#[template(path = "sessions/new.html")]
pub struct SessionsNew {
    pub agents: Vec<AgentListItem>,
}

#[derive(Template)]
#[template(path = "sessions/chat.html")]
pub struct SessionsChat {
    pub session: SessionFormItem,
    pub messages: Vec<MessageItem>,
}

#[derive(Serialize)]
pub struct SessionListItem {
    pub id: Uuid,
    pub title: Option<String>,
    pub agent_id: Uuid,
}

#[derive(Serialize)]
pub struct SessionFormItem {
    pub id: Uuid,
    pub title: Option<String>,
    pub agent_id: Uuid,
}

#[derive(Serialize)]
pub struct MessageItem {
    pub id: Uuid,
    pub content: String,
    pub role: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_views_module() {
        assert!(true);
    }
}
