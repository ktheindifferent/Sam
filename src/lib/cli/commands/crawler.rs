use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_crawler(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    match cmd {
        "crawler start" => {
            // Spawn the crawler start in a separate task to avoid blocking the TUI
            let output_lines_clone = output_lines.clone();
            tokio::spawn(async move {
                crate::services::crawler::start_service_async().await;
                let mut out = output_lines_clone.lock().await;
                out.push("Crawler service started.".to_string());
            });
            
            // Immediately show feedback to the user
            let mut out = output_lines.lock().await;
            out.push("Starting crawler service...".to_string());
        }
        "crawler stop" => {
            // Spawn the stop operation in a separate task
            let output_lines_clone = output_lines.clone();
            tokio::spawn(async move {
                crate::services::crawler::stop_service();
                let mut out = output_lines_clone.lock().await;
                out.push("Crawler service stopped.".to_string());
            });
            
            // Immediately show feedback
            let mut out = output_lines.lock().await;
            out.push("Stopping crawler service...".to_string());
        }
        "crawler status" => {
            // Get status without blocking
            let status = crate::services::crawler::service_status();
            let mut out = output_lines.lock().await;
            out.push(format!("Crawler service status: {}", status));
        }
        _ => {
            let mut out = output_lines.lock().await;
            out.push("Unknown crawler command.".to_string());
        }
    }
}

pub async fn handle_crawl_search(
    cmd: &str,
    output_lines: &Arc<Mutex<Vec<String>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let query = cmd.trim_start_matches("crawl search ").trim();
    if query.is_empty() {
        let mut out = output_lines.lock().await;
        out.push("Usage: crawl search <query>".to_string());
    } else {
        let query = query.to_string();
        let output_lines = output_lines.clone();
        tokio::spawn(async move {
            use crate::services::crawler::CrawledPage;

            match CrawledPage::query_by_relevance_async(&query, 10).await {
                Ok(scored_pages) if !scored_pages.is_empty() => {
                    let mut out = output_lines.lock().await;
                    out.push(format!("Found {} results:", scored_pages.len()));
                    for (page, score) in scored_pages {
                        out.push(format!("URL: {}", page.url));
                        out.push(format!("Score: {score}"));
                        if !page.tokens.is_empty() {
                            let snippet: String = page
                                .tokens
                                .iter()
                                .take(20)
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(" ");
                            out.push(format!("Tokens: {snippet}..."));
                        }
                        out.push("-----------------------------".to_string());
                    }
                }
                Ok(_) => {
                    let mut out = output_lines.lock().await;
                    out.push("No results found.".to_string());
                }
                Err(e) => {
                    let mut out = output_lines.lock().await;
                    out.push(format!("Search error: {e}"));
                }
            }
        });
    }
    Ok(())
}
