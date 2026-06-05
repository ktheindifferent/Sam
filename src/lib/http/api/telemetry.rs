use crate::services::crawler::CrawledContent;
use crate::services::telemetry::{TelemetryContent, TelemetryPayload};
use log::{debug, error, info, warn};
use rouille::{Request, Response};
use serde::{Deserialize, Serialize};

/// Response for telemetry submission
#[derive(Serialize, Deserialize)]
pub struct TelemetryResponse {
    pub success: bool,
    pub message: String,
    pub received_items: usize,
    pub processed_items: usize,
    pub duplicate_items: usize,
}

/// Handle telemetry API requests
pub fn handle(request: &Request) -> Result<Response, crate::http::Error> {
    let url = request.url();
    let method = request.method();

    debug!("Telemetry API request: {} {}", method, url);

    match (method, url.as_str()) {
        ("POST", "/api/telemetry/submit") => handle_submit_telemetry(request),
        ("GET", "/api/telemetry/status") => handle_telemetry_status(request),
        ("POST", "/api/telemetry/health") => handle_health_check(request),
        _ => Ok(Response::empty_404()),
    }
}

/// Handle telemetry data submission from remote SAM instances
fn handle_submit_telemetry(request: &Request) -> Result<Response, crate::http::Error> {
    info!("Received telemetry submission request");

    // Parse the JSON payload
    let payload: TelemetryPayload = match rouille::input::json_input(request) {
        Ok(payload) => payload,
        Err(_) => {
            warn!("Invalid telemetry payload received");
            return Ok(Response::json(&TelemetryResponse {
                success: false,
                message: "Invalid JSON payload".to_string(),
                received_items: 0,
                processed_items: 0,
                duplicate_items: 0,
            })
            .with_status_code(400));
        }
    };

    let received_count = payload.content.len();
    info!(
        "Processing telemetry from instance {} with {} items",
        payload.instance_id, received_count
    );

    // Process the telemetry data asynchronously
    let payload_clone = payload.clone();
    tokio::task::spawn(async move {
        match process_telemetry_payload(payload_clone).await {
            Ok((processed, duplicates)) => {
                info!(
                    "Successfully processed telemetry: {} new items, {} duplicates",
                    processed, duplicates
                );
            }
            Err(e) => {
                error!("Failed to process telemetry: {}", e);
            }
        }
    });

    // Return immediate response
    Ok(Response::json(&TelemetryResponse {
        success: true,
        message: "Telemetry data received and queued for processing".to_string(),
        received_items: received_count,
        processed_items: 0, // Will be processed asynchronously
        duplicate_items: 0,
    })
    .with_status_code(202)) // 202 Accepted
}

/// Process telemetry payload asynchronously
async fn process_telemetry_payload(
    payload: TelemetryPayload,
) -> Result<(usize, usize), anyhow::Error> {
    let mut processed = 0;
    let mut duplicates = 0;

    // Convert telemetry content to CrawledContent and save
    for telemetry_content in payload.content {
        // Check if this content already exists (deduplication)
        let existing = match check_content_exists(&telemetry_content.content_hash).await {
            Ok(exists) => exists,
            Err(e) => {
                warn!(
                    "Error checking content existence for {}: {}",
                    telemetry_content.url, e
                );
                continue;
            }
        };

        if existing {
            duplicates += 1;
            debug!("Skipping duplicate content: {}", telemetry_content.url);
            continue;
        }

        // Convert TelemetryContent to CrawledContent
        let mut crawled_content = convert_telemetry_to_crawled(&telemetry_content);
        // Mark as already shared since it came from telemetry
        crawled_content.telemetry_shared = true;

        // Save the content
        match crawled_content.save().await {
            Ok(true) => {
                processed += 1;
                debug!("Saved telemetry content: {}", crawled_content.url);
            }
            Ok(false) => {
                duplicates += 1;
                debug!("Content was duplicate on save: {}", crawled_content.url);
            }
            Err(e) => {
                warn!(
                    "Failed to save telemetry content {}: {}",
                    crawled_content.url, e
                );
            }
        }
    }

    info!(
        "Telemetry processing completed: {} processed, {} duplicates",
        processed, duplicates
    );
    Ok((processed, duplicates))
}

/// Check if content with the given hash already exists
async fn check_content_exists(content_hash: &str) -> Result<bool, anyhow::Error> {
    let client = crate::services::crawler::get_db_connection()
        .await
        .ok_or_else(|| anyhow::anyhow!("Failed to get database connection"))?;

    let result = client
        .query(
            "SELECT 1 FROM crawled_content WHERE content_hash = $1 LIMIT 1",
            &[&content_hash],
        )
        .await?;

    Ok(!result.is_empty())
}

/// Convert TelemetryContent to CrawledContent
fn convert_telemetry_to_crawled(telemetry: &TelemetryContent) -> CrawledContent {
    // Create a new CrawledContent from the telemetry data
    let mut content = CrawledContent::new(
        telemetry.url.clone(),
        &telemetry.content_text,
        None, // We don't receive HTML in telemetry for bandwidth reasons
        telemetry.status_code as u16,
    );

    // Override with telemetry data
    content.content_hash = telemetry.content_hash.clone();
    content.title = telemetry.title.clone();
    content.description = telemetry.description.clone();
    content.content_type = telemetry.content_type.clone();
    content.content_length = telemetry.content_length;
    content.language = telemetry.language.clone();
    content.crawled_at = telemetry.crawled_at;
    content.updated_at = telemetry.crawled_at; // Use same timestamp
    content.telemetry_shared = true; // Mark as already shared

    content
}

/// Handle telemetry status requests
fn handle_telemetry_status(_request: &Request) -> Result<Response, crate::http::Error> {
    let is_osf = std::env::var("IS_OSF")
        .unwrap_or_else(|_| "false".to_string())
        .to_lowercase()
        == "true";

    #[derive(Serialize)]
    struct TelemetryStatus {
        is_osf_server: bool,
        accepting_submissions: bool,
        version: String,
        endpoint: String,
    }

    let status = TelemetryStatus {
        is_osf_server: is_osf,
        accepting_submissions: is_osf, // Only accept submissions if we're the OSF server
        version: "0.0.2".to_string(),
        endpoint: "/api/telemetry/submit".to_string(),
    };

    Ok(Response::json(&status))
}

/// Handle health check requests
fn handle_health_check(_request: &Request) -> Result<Response, crate::http::Error> {
    #[derive(Serialize)]
    struct HealthCheck {
        status: String,
        timestamp: i64,
        database_connected: bool,
    }

    // Check database connectivity
    let db_connected = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            crate::services::crawler::get_db_connection()
                .await
                .is_some()
        })
    });

    let health = HealthCheck {
        status: if db_connected {
            "healthy".to_string()
        } else {
            "unhealthy".to_string()
        },
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64,
        database_connected: db_connected,
    };

    let status_code = if db_connected { 200 } else { 503 };

    Ok(Response::json(&health).with_status_code(status_code))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::telemetry::TelemetryContent;

    #[test]
    fn test_convert_telemetry_to_crawled() {
        let telemetry = TelemetryContent {
            url: "https://example.com".to_string(),
            content_hash: "abc123".to_string(),
            title: Some("Test Title".to_string()),
            description: Some("Test Description".to_string()),
            content_text: "Test content".to_string(),
            status_code: 200,
            content_type: Some("text/html".to_string()),
            content_length: 1024,
            language: Some("en".to_string()),
            crawled_at: 1640995200, // 2022-01-01
        };

        let crawled = convert_telemetry_to_crawled(&telemetry);

        assert_eq!(crawled.url, "https://example.com");
        assert_eq!(crawled.content_hash, "abc123");
        assert_eq!(crawled.title, Some("Test Title".to_string()));
        assert_eq!(crawled.status_code, 200);
        assert!(crawled.telemetry_shared);
        assert_eq!(crawled.crawled_at, 1640995200);
    }
}
