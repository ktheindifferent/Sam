use reqwest::{Client, Error};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct CopilotRequest {
    prompt: String,
    // Add other fields as required by the API
}

#[derive(Deserialize, Debug)]
pub struct CopilotResponse {
    pub completion: String,
    // Add other fields as returned by the API
}

pub struct CopilotClient {
    client: Client,
    api_key: String,
    endpoint: String,
}

impl CopilotClient {
    pub fn new(api_key: String, endpoint: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            endpoint,
        }
    }

    pub async fn complete(&self, prompt: &str) -> Result<CopilotResponse, Error> {
        let req_body = CopilotRequest {
            prompt: prompt.to_string(),
        };

        let res = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .json(&req_body)
            .send()
            .await?
            .json::<CopilotResponse>()
            .await?;

        Ok(res)
    }
}

// Example usage (async context):
// let copilot = CopilotClient::new("YOUR_API_KEY".to_string(), "https://api.copilot.microsoft.com/v1/complete".to_string());
// let response = copilot.complete("Write a Rust function to add two numbers.").await?;