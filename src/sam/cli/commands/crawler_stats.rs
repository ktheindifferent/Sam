use std::sync::Arc;
use tokio::sync::Mutex;

/// Handle crawler statistics commands
pub async fn handle_crawler_stats(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    match cmd {
        "crawler rejections" | "crawler rejected" => {
            show_rejection_stats(output_lines).await;
        }
        cmd if cmd.starts_with("crawler rejections ") => {
            let domain = cmd.trim_start_matches("crawler rejections ").trim();
            show_domain_rejections(domain, output_lines).await;
        }
        _ => {
            let mut out = output_lines.lock().await;
            out.push("Unknown crawler stats command.".to_string());
            out.push("Available commands:".to_string());
            out.push("  crawler rejections - Show rejection statistics".to_string());
            out.push("  crawler rejections <domain> - Show rejections for a specific domain".to_string());
        }
    }
}

/// Show overall rejection statistics
async fn show_rejection_stats(output_lines: &Arc<Mutex<Vec<String>>>) {
    match crate::sam::services::crawler::CrawlRejected::get_stats().await {
        Ok(stats) => {
            let mut out = output_lines.lock().await;
            out.push("=== Crawler Rejection Statistics ===".to_string());
            out.push(format!("Total rejections: {}", stats["total_rejections"]));
            out.push(format!("Unique URLs: {}", stats["unique_urls"]));
            out.push(format!("Rejections in last hour: {}", stats["rejections_last_hour"]));
            out.push("".to_string());
            
            out.push("Rejections by reason:".to_string());
            if let Some(reasons) = stats["by_reason"].as_array() {
                for reason in reasons {
                    out.push(format!("  {} - {}", 
                        reason["reason"].as_str().unwrap_or("unknown"),
                        reason["count"]
                    ));
                }
            }
            out.push("".to_string());
            
            out.push("Top rejected domains:".to_string());
            if let Some(domains) = stats["top_rejected_domains"].as_array() {
                for domain in domains {
                    out.push(format!("  {} - {} rejections", 
                        domain["domain"].as_str().unwrap_or("unknown"),
                        domain["count"]
                    ));
                }
            }
        }
        Err(e) => {
            let mut out = output_lines.lock().await;
            out.push(format!("Error fetching rejection stats: {}", e));
        }
    }
}

/// Show rejections for a specific domain
async fn show_domain_rejections(domain: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    match crate::sam::services::crawler::CrawlRejected::get_domain_rejections(domain).await {
        Ok(rejections) => {
            let mut out = output_lines.lock().await;
            out.push(format!("=== Rejections for domain: {} ===", domain));
            out.push(format!("Found {} rejections", rejections.len()));
            out.push("".to_string());
            
            for (i, rejection) in rejections.iter().take(20).enumerate() {
                out.push(format!("{}. {}", i + 1, rejection.url));
                out.push(format!("   Reason: {:?}", rejection.reason));
                if let Some(rule) = &rejection.robots_rule {
                    out.push(format!("   Rule: {}", rule));
                }
                out.push(format!("   Count: {} times", rejection.rejection_count));
                
                // Format timestamp
                let dt = chrono::NaiveDateTime::from_timestamp_opt(rejection.rejected_at, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                out.push(format!("   Last rejected: {}", dt));
                out.push("".to_string());
            }
            
            if rejections.len() > 20 {
                out.push(format!("... and {} more", rejections.len() - 20));
            }
        }
        Err(e) => {
            let mut out = output_lines.lock().await;
            out.push(format!("Error fetching domain rejections: {}", e));
        }
    }
}