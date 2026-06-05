use super::state::ServiceStatus;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Spawns a background task that polls service statuses every 2 seconds
pub fn spawn_status_updater(service_status: Arc<Mutex<ServiceStatus>>) {
    tokio::spawn(async move {
        let mut count = 0u64;
        let mut sys = sysinfo::System::new_all();
        let mut prev_statuses: HashMap<String, String> = HashMap::new();

        loop {
            // Update system information with proper refresh cycle
            sys.refresh_cpu_all();
            sys.refresh_memory();
            sys.refresh_all();

            // Wait for CPU usage to stabilize
            tokio::time::sleep(Duration::from_millis(200)).await;
            sys.refresh_cpu_all();

            // Service statuses with better error handling
            let crawler =
                match std::panic::catch_unwind(|| crate::services::crawler::service_status()) {
                    Ok(status) => {
                        log::debug!("Crawler service status: {}", status);
                        status.to_string()
                    }
                    Err(e) => {
                        log::error!("Failed to get crawler status: {:?}", e);
                        "error".to_string()
                    }
                };

            let redis = match tokio::time::timeout(
                Duration::from_millis(500),
                crate::services::redis::status(),
            )
            .await
            {
                Ok(status) => status.to_string(),
                Err(_) => {
                    log::debug!("Redis status check timed out");
                    "timeout".to_string()
                }
            };

            // Docker status
            let docker = if std::env::var("CAPROVER").is_ok() {
                "disabled (CapRover)".to_string()
            } else {
                match tokio::time::timeout(
                    Duration::from_millis(1000),
                    crate::services::docker::is_running_async(),
                )
                .await
                {
                    Ok(Ok(true)) => "running".to_string(),
                    Ok(Ok(false)) => "stopped".to_string(),
                    Ok(Err(_)) => "error".to_string(),
                    Err(_) => "timeout".to_string(),
                }
            };

            let sms = match std::panic::catch_unwind(|| crate::services::sms::status()) {
                Ok(status) => status.to_string(),
                Err(_) => "error".to_string(),
            };

            // PostgreSQL status
            let postgres = match tokio::time::timeout(
                Duration::from_millis(500),
                crate::services::pg::health_check(),
            )
            .await
            {
                Ok(Ok(_)) => "connected".to_string(),
                Ok(Err(_)) => "disconnected".to_string(),
                Err(_) => "timeout".to_string(),
            };

            let lifx = match std::panic::catch_unwind(|| "unknown") {
                Ok(status) => status.to_string(),
                Err(_) => "error".to_string(),
            };

            let tts = match std::panic::catch_unwind(|| "unknown") {
                Ok(status) => status.to_string(),
                Err(_) => "error".to_string(),
            };

            let stt = match std::panic::catch_unwind(|| "unknown") {
                Ok(status) => status.to_string(),
                Err(_) => "error".to_string(),
            };

            // Ollama service with timeout
            let ollama = match tokio::time::timeout(Duration::from_millis(1000), async {
                let service = crate::services::llms::ollama::OllamaService::new_with_defaults();
                if service.is_installed().await {
                    if service.is_running().await {
                        "running"
                    } else {
                        "stopped"
                    }
                } else {
                    "not installed"
                }
            })
            .await
            {
                Ok(status) => status.to_string(),
                Err(_) => "timeout".to_string(),
            };

            let http_server = "running".to_string();

            let ssh_server = if crate::services::ssh::server::is_ssh_server_running().await {
                "running"
            } else {
                "stopped"
            }
            .to_string();

            let media = match crate::services::media::status() {
                Ok(_) => "running".to_string(),
                Err(_) => "stopped".to_string(),
            };

            let snapcast = match crate::services::snapcast::status() {
                Ok(true) => "running".to_string(),
                Ok(false) => "stopped".to_string(),
                Err(_) => "error".to_string(),
            };

            // System metrics
            let memory_usage = {
                let total = sys.total_memory();
                let used = sys.used_memory();
                if total > 0 {
                    let total_mb = total as f64 / 1024.0 / 1024.0;
                    let used_mb = used as f64 / 1024.0 / 1024.0;
                    let percent = (used as f64 / total as f64) * 100.0;
                    format!("{:.0}/{:.0} MB ({:.1}%)", used_mb, total_mb, percent)
                } else {
                    "N/A".to_string()
                }
            };

            let cpu_usage = {
                let cpu_percent = sys.global_cpu_usage();
                if cpu_percent.is_finite() && cpu_percent >= 0.0 {
                    format!("{:.1}%", cpu_percent)
                } else {
                    "N/A".to_string()
                }
            };

            let disk_usage = {
                let disks = sysinfo::Disks::new_with_refreshed_list();
                let mut total_space = 0u64;
                let mut available_space = 0u64;

                for disk in disks.list() {
                    total_space += disk.total_space();
                    available_space += disk.available_space();
                }

                if total_space > 0 {
                    let used_space = total_space - available_space;
                    let usage_percent = (used_space as f64 / total_space as f64) * 100.0;
                    format!(
                        "{:.1}% ({:.1}/{:.1} GB)",
                        usage_percent,
                        used_space as f64 / (1024.0 * 1024.0 * 1024.0),
                        total_space as f64 / (1024.0 * 1024.0 * 1024.0)
                    )
                } else {
                    "N/A".to_string()
                }
            };

            // Build current statuses map for change detection
            let current_statuses: HashMap<String, String> = [
                ("crawler", &crawler),
                ("redis", &redis),
                ("docker", &docker),
                ("sms", &sms),
                ("postgres", &postgres),
                ("lifx", &lifx),
                ("http_server", &http_server),
                ("ollama", &ollama),
                ("tts", &tts),
                ("stt", &stt),
                ("ssh_server", &ssh_server),
                ("media", &media),
                ("snapcast", &snapcast),
            ]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

            // Emit ServiceEvent::StatusChanged for any service whose status changed
            for (svc, new_val) in &current_statuses {
                let old_val = prev_statuses.get(svc).cloned().unwrap_or_default();
                if old_val != *new_val {
                    crate::services::events::emit(
                        crate::services::events::ServiceEvent::StatusChanged {
                            service: svc.clone(),
                            old_status: old_val,
                            new_status: new_val.clone(),
                        },
                    );
                }
            }
            prev_statuses = current_statuses;

            // Use lock with timeout to avoid deadlocks
            match tokio::time::timeout(std::time::Duration::from_millis(100), service_status.lock())
                .await
            {
                Ok(mut status) => {
                    status.crawler = crawler;
                    status.redis = redis;
                    status.docker = docker;
                    status.sms = sms;
                    status.postgres = postgres;
                    status.lifx = lifx;
                    status.http_server = http_server;
                    status.ollama = ollama;
                    status.tts = tts;
                    status.stt = stt;
                    status.ssh_server = ssh_server;
                    status.media = media;
                    status.snapcast = snapcast;
                    status.memory_usage = memory_usage;
                    status.cpu_usage = cpu_usage;
                    status.disk_usage = disk_usage;
                    status.update_count = count;
                    // Push to sparkline history
                    if let Ok(cpu_val) = status.cpu_usage.trim_end_matches('%').parse::<f64>() {
                        status.cpu_history.push(cpu_val);
                    }
                    // Parse memory percentage from "X/Y MB (Z%)" format
                    if let Some(start) = status.memory_usage.find('(') {
                        if let Some(end) = status.memory_usage.find("%)") {
                            if let Ok(mem_val) = status.memory_usage[start + 1..end].parse::<f64>()
                            {
                                status.memory_history.push(mem_val);
                            }
                        }
                    }
                    count += 1;
                }
                Err(_) => {
                    log::debug!("Service status update timed out, will retry");
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });
}
