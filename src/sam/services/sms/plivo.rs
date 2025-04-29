use reqwest::Client;
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
struct PlivoMessage {
    message_uuid: Option<String>,
    from_number: Option<String>,
    to_number: Option<String>,
    message_state: Option<String>,
    message_direction: Option<String>,
    message_time: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlivoMessagesResponse {
    objects: Vec<PlivoMessage>,
    next: Option<String>,
}

/// Send an SMS using Plivo's API
pub async fn send_sms(to: &str, body: &str) -> Result<(), String> {
    let auth_id = env::var("PLIVO_AUTH_ID").map_err(|_| "Missing PLIVO_AUTH_ID")?;
    let auth_token = env::var("PLIVO_AUTH_TOKEN").map_err(|_| "Missing PLIVO_AUTH_TOKEN")?;
    let from = env::var("PLIVO_FROM_NUMBER").map_err(|_| "Missing PLIVO_FROM_NUMBER")?;

    let url = format!("https://api.plivo.com/v1/Account/{auth_id}/Message/");

    let client = Client::new();
    let params = serde_json::json!({
        "src": from,
        "dst": to,
        "text": body,
    });

    let res = client
        .post(&url)
        .basic_auth(&auth_id, Some(&auth_token))
        .json(&params)
        .send()
        .await
        .map_err(|e| format!("Plivo send error: {e}"))?;

    if res.status().is_success() {
        Ok(())
    } else {
        Err(format!(
            "Plivo send failed: {}",
            res.text().await.unwrap_or_default()
        ))
    }
}

/// Poll Plivo for received SMS messages (last 50, paginated)
pub async fn receive_sms() -> Result<Vec<String>, String> {
    let auth_id = env::var("PLIVO_AUTH_ID").map_err(|_| "Missing PLIVO_AUTH_ID")?;
    let auth_token = env::var("PLIVO_AUTH_TOKEN").map_err(|_| "Missing PLIVO_AUTH_TOKEN")?;
    let mut url = format!("https://api.plivo.com/v1/Account/{auth_id}/Message/?limit=50&offset=0");

    let client = Client::new();
    let mut messages = Vec::new();
    let mut pages = 0;

    // Fetch up to 3 pages (150 messages max)
    while pages < 3 {
        let res = client
            .get(&url)
            .basic_auth(&auth_id, Some(&auth_token))
            .send()
            .await
            .map_err(|e| format!("Plivo receive error: {e}"))?;

        if !res.status().is_success() {
            return Err(format!(
                "Plivo receive failed: {}",
                res.text().await.unwrap_or_default()
            ));
        }

        let resp: PlivoMessagesResponse = res
            .json()
            .await
            .map_err(|e| format!("Plivo parse error: {e}"))?;
        for msg in &resp.objects {
            messages.push(format!(
                "[{}] From: {} To: {} Body: {}",
                msg.message_time
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                msg.from_number
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                msg.to_number
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                msg.message.clone().unwrap_or_else(|| "".to_string())
            ));
        }
        if let Some(next_url) = &resp.next {
            url = next_url.clone();
            pages += 1;
        } else {
            break;
        }
    }
    if messages.is_empty() {
        Ok(vec!["No messages found.".to_string()])
    } else {
        Ok(messages)
    }
}
