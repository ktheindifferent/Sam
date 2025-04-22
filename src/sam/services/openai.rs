use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
}

#[derive(Deserialize, Debug)]
pub struct ChatChoice {
    pub message: ChatMessage,
}

#[derive(Deserialize, Debug)]
pub struct ChatResponse {
    pub choices: Vec<ChatChoice>,
}

pub struct OpenAIClient {
    api_key: String,
    client: reqwest::Client,
}

impl OpenAIClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }

    pub async fn chat(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
    ) -> Result<ChatResponse, reqwest::Error> {
        let req_body = ChatRequest {
            model: model.to_string(),
            messages,
        };

        let res = self
            .client
            .post(OPENAI_API_URL)
            .header(AUTHORIZATION, format!("Bearer {}", self.api_key))
            .header(CONTENT_TYPE, "application/json")
            .json(&req_body)
            .send()
            .await?
            .json::<ChatResponse>()
            .await?;

        Ok(res)
    }
}

// Example usage (async context):
// let client = OpenAIClient::new("your_api_key".to_string());
// let messages = vec![ChatMessage { role: "user".to_string(), content: "Hello!".to_string() }];
// let response = client.chat("gpt-3.5-turbo", messages).await?;