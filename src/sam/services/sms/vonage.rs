use reqwest::Client;
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
struct VonageMessage {
    message_id: Option<String>,
    to: Option<String>,
    msisdn: Option<String>,
    text: Option<String>,
    date_received: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VonageMessagesResponse {
    items: Option<Vec<VonageMessage>>,
}

/// Send an SMS using Vonage's API
pub async fn send_sms(to: &str, body: &str) -> Result<(), String> {
    let api_key = env::var("VONAGE_API_KEY").map_err(|_| "Missing VONAGE_API_KEY")?;
    let api_secret = env::var("VONAGE_API_SECRET").map_err(|_| "Missing VONAGE_API_SECRET")?;
    let from = env::var("VONAGE_FROM_NUMBER").unwrap_or_else(|_| "SamBot".to_string());

    let url = "https://rest.nexmo.com/sms/json";
    let client = Client::new();
    let params = [
        ("api_key", api_key.as_str()),
        ("api_secret", api_secret.as_str()),
        ("to", to),
        ("from", &from),
        ("text", body),
    ];

    let res = client
        .post(url)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Vonage send error: {e}"))?;

    if res.status().is_success() {
        Ok(())
    } else {
        Err(format!(
            "Vonage send failed: {}",
            res.text().await.unwrap_or_default()
        ))
    }
}

/// Poll Vonage for received SMS messages (last 50, paginated)
pub async fn receive_sms() -> Result<Vec<String>, String> {
    let api_key = env::var("VONAGE_API_KEY").map_err(|_| "Missing VONAGE_API_KEY")?;
    let api_secret = env::var("VONAGE_API_SECRET").map_err(|_| "Missing VONAGE_API_SECRET")?;
    let url = format!(
        "https://api.nexmo.com/search/messages?api_key={}&api_secret={}&date={}",
        api_key,
        api_secret,
        chrono::Utc::now().format("%Y-%m-%d")
    );

    let client = Client::new();
    let res = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Vonage receive error: {e}"))?;

    if !res.status().is_success() {
        return Err(format!(
            "Vonage receive failed: {}",
            res.text().await.unwrap_or_default()
        ));
    }

    let resp: VonageMessagesResponse = res
        .json()
        .await
        .map_err(|e| format!("Vonage parse error: {e}"))?;
    let mut messages = Vec::new();
    if let Some(items) = resp.items {
        for msg in items {
            messages.push(format!(
                "[{}] From: {} To: {} Body: {}",
                msg.date_received.unwrap_or_else(|| "unknown".to_string()),
                msg.msisdn.unwrap_or_else(|| "unknown".to_string()),
                msg.to.unwrap_or_else(|| "unknown".to_string()),
                msg.text.unwrap_or_else(|| "".to_string())
            ));
        }
    }
    if messages.is_empty() {
        Ok(vec!["No messages found.".to_string()])
    } else {
        Ok(messages)
    }
}
