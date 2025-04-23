use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_crawler(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    match cmd {
        "crawler start" => {
            crate::sam::services::crawler::start_service_async().await;
            let mut out = output_lines.lock().await;
            out.push("Crawler service started.".to_string());
        }
        "crawler stop" => {
            crate::cli::spinner::run_with_spinner(
                output_lines,
                "Stopping crawler service...",
                |lines, _| lines.push("Crawler service stopped.".to_string()),
                || async {
                    crate::sam::services::crawler::stop_service();
                    "done".to_string()
                },
            ).await;
        }
        "crawler status" => {
            crate::cli::spinner::run_with_spinner(
                output_lines,
                "Checking crawler service status...",
                |lines, status| lines.push(format!("Crawler service status: {}", status)),
                || async {
                    crate::sam::services::crawler::service_status().to_string()
                },
            ).await;
        }
        _ => {
            let mut out = output_lines.lock().await;
            out.push("Unknown crawler command.".to_string());
        }
    }
}

pub async fn handle_crawl_search(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    let query = cmd.trim_start_matches("crawl search ").trim();
    if query.is_empty() {
        let mut out = output_lines.lock().await;
        out.push("Usage: crawl search <query>".to_string());
    } else {
        let query = query.to_string();
        let output_lines = output_lines.clone();
        tokio::spawn(async move {
            use crate::sam::services::crawler::CrawledPage;
            match CrawledPage::query_by_relevance_async(&query, 10).await {
                Ok(scored_pages) if !scored_pages.is_empty() => {
                    let mut out = output_lines.lock().await;
                    out.push(format!("Found {} results:", scored_pages.len()));
                    for (page, score) in scored_pages {
                        out.push(format!("URL: {}", page.url));
                        out.push(format!("Score: {}", score));
                        if !page.tokens.is_empty() {
                            let snippet: String = page.tokens.iter().take(20).cloned().collect::<Vec<_>>().join(" ");
                            out.push(format!("Tokens: {}...", snippet));
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
                    out.push(format!("Search error: {}", e));
                }
            }
        });
    }
}
