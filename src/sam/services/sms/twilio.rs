use reqwest::Client;
use std::env;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct TwilioMessage {
    sid: String,
    from: String,
    to: String,
    body: String,
    date_sent: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TwilioMessagesResponse {
    messages: Vec<TwilioMessage>,
    next_page_uri: Option<String>,
}

/// Send an SMS using Twilio's API
pub async fn send_sms(to: &str, body: &str) -> Result<(), String> {
    let account_sid = env::var("TWILIO_ACCOUNT_SID").map_err(|_| "Missing TWILIO_ACCOUNT_SID")?;
    let auth_token = env::var("TWILIO_AUTH_TOKEN").map_err(|_| "Missing TWILIO_AUTH_TOKEN")?;
    let from = env::var("TWILIO_FROM_NUMBER").map_err(|_| "Missing TWILIO_FROM_NUMBER")?;

    let url = format!(
        "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
        account_sid
    );

    let client = Client::new();
    let params = [
        ("To", to),
        ("From", &from),
        ("Body", body),
    ];

    let res = client
        .post(&url)
        .basic_auth(account_sid, Some(auth_token))
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Twilio send error: {}", e))?;

    if res.status().is_success() {
        Ok(())
    } else {
        Err(format!("Twilio send failed: {}", res.text().await.unwrap_or_default()))
    }
}

/// Poll Twilio for received SMS messages (last 50 by default, paginated)
pub async fn receive_sms() -> Result<Vec<String>, String> {
    let account_sid = env::var("TWILIO_ACCOUNT_SID").map_err(|_| "Missing TWILIO_ACCOUNT_SID")?;
    let auth_token = env::var("TWILIO_AUTH_TOKEN").map_err(|_| "Missing TWILIO_AUTH_TOKEN")?;
    let url = format!(
        "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json?PageSize=50",
        account_sid
    );

    let client = Client::new();
    let mut messages = Vec::new();
    let mut next_url = Some(url);

    // Fetch up to 3 pages (150 messages max)
    for _ in 0..3 {
        let url = match next_url {
            Some(ref u) => u,
            None => break,
        };
        let res = client
            .get(url)
            .basic_auth(&account_sid, Some(&auth_token))
            .send()
            .await
            .map_err(|e| format!("Twilio receive error: {}", e))?;

        if !res.status().is_success() {
            return Err(format!("Twilio receive failed: {}", res.text().await.unwrap_or_default()));
        }

        let resp: TwilioMessagesResponse = res.json().await.map_err(|e| format!("Twilio parse error: {}", e))?;
        for msg in &resp.messages {
            messages.push(format!(
                "[{}] From: {} To: {} Body: {}",
                msg.date_sent.clone().unwrap_or_else(|| "unknown".to_string()),
                msg.from,
                msg.to,
                msg.body
            ));
        }
        next_url = resp.next_page_uri.as_ref().map(|uri| format!("https://api.twilio.com{}", uri));
        if resp.next_page_uri.is_none() {
            break;
        }
    }
    if messages.is_empty() {
        Ok(vec!["No messages found.".to_string()])
    } else {
        Ok(messages)
    }
}
