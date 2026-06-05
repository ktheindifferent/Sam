// Network Monitoring Integration Example
// This example demonstrates how to use the network monitoring module

use libsam::network_config::NetworkMonitorConfig;
use libsam::network_monitor::{ConnectionStats, NetworkMonitor};
use std::time::Duration;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    env_logger::init();

    println!("Starting Network Monitoring Example");
    println!("=====================================\n");

    // Create network monitor with default configuration
    let monitor = NetworkMonitor::new();

    // Example 1: Read network interface statistics
    println!("1. Reading Network Interface Statistics:");
    println!("-----------------------------------------");

    if let Ok(interfaces) = monitor.read_network_stats().await {
        for (name, interface) in interfaces.iter() {
            println!("Interface: {}", name);
            println!("  RX Bytes: {} MB", interface.rx_bytes as f64 / 1_000_000.0);
            println!("  TX Bytes: {} MB", interface.tx_bytes as f64 / 1_000_000.0);
            println!("  RX Packets: {}", interface.rx_packets);
            println!("  TX Packets: {}", interface.tx_packets);
            println!(
                "  Errors: RX={}, TX={}",
                interface.rx_errors, interface.tx_errors
            );
            println!();
        }
    }

    // Example 2: Calculate network speeds
    println!("2. Calculating Network Speeds:");
    println!("-------------------------------");

    // Initial reading
    let _ = monitor.calculate_speeds().await;

    // Wait for some network activity
    println!("Waiting 2 seconds to measure network activity...");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Calculate speeds
    if let Ok(speeds) = monitor.calculate_speeds().await {
        for (interface, speed) in speeds.iter() {
            println!("Interface: {}", interface);
            println!(
                "  Download: {:.2} Mbps ({:.0} KB/s)",
                speed.download_speed_mbps,
                speed.download_speed_bps / 1024.0
            );
            println!(
                "  Upload: {:.2} Mbps ({:.0} KB/s)",
                speed.upload_speed_mbps,
                speed.upload_speed_bps / 1024.0
            );
            println!("  Total: {:.2} Mbps", speed.total_speed_mbps);
            println!();
        }
    }

    // Example 3: Measure network latency
    println!("3. Measuring Network Latency:");
    println!("------------------------------");

    let test_hosts = vec!["8.8.8.8", "1.1.1.1", "127.0.0.1"];

    for host in test_hosts {
        match monitor.measure_latency(host).await {
            Ok(latency) => {
                println!("Host: {}", host);
                println!("  Latency: {:.2} ms", latency.latency_ms);
                println!("  Packet Loss: {:.1}%", latency.packet_loss);
                println!("  Jitter: {:.2} ms", latency.jitter_ms);
            }
            Err(e) => {
                println!("Failed to measure latency to {}: {}", host, e);
            }
        }
        println!();
    }

    // Example 4: Get comprehensive metrics
    println!("4. Comprehensive Network Metrics:");
    println!("----------------------------------");

    // Take multiple measurements for averaging
    for _ in 0..3 {
        let _ = monitor.calculate_speeds().await;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    if let Ok(metrics) = monitor.get_metrics().await {
        println!(
            "Total Download Speed: {:.2} Mbps",
            metrics.total_download_mbps
        );
        println!("Total Upload Speed: {:.2} Mbps", metrics.total_upload_mbps);
        println!("Average Latency: {:.2} ms", metrics.average_latency_ms);
        println!("Packet Loss: {:.1}%", metrics.packet_loss_percent);
        println!();
    }

    // Example 5: Connection statistics
    println!("5. Connection Statistics:");
    println!("-------------------------");

    match ConnectionStats::gather().await {
        Ok(stats) => {
            println!("TCP Established: {}", stats.tcp_established);
            println!("TCP Listening: {}", stats.tcp_listen);
            println!("TCP Time Wait: {}", stats.tcp_time_wait);
            println!("UDP Connections: {}", stats.udp_connections);
            println!("Total Connections: {}", stats.total_connections);
        }
        Err(e) => {
            println!("Failed to gather connection stats: {}", e);
        }
    }
    println!();

    // Example 6: Configure and start continuous monitoring
    println!("6. Continuous Monitoring Configuration:");
    println!("----------------------------------------");

    let config = NetworkMonitorConfig {
        enabled: true,
        update_interval_ms: 1000,
        history_size: 30,
        latency_check_hosts: vec!["8.8.8.8".to_string()],
        latency_check_interval_ms: 5000,
        interfaces_to_monitor: vec![],
        alert_thresholds: libsam::network_config::AlertThresholds {
            high_latency_ms: 100.0,
            packet_loss_percent: 5.0,
            low_bandwidth_mbps: 1.0,
            high_error_rate: 0.01,
        },
    };

    println!("Configuration:");
    println!("  Update Interval: {} ms", config.update_interval_ms);
    println!("  History Size: {} samples", config.history_size);
    println!(
        "  Latency Check Interval: {} ms",
        config.latency_check_interval_ms
    );
    println!(
        "  High Latency Threshold: {} ms",
        config.alert_thresholds.high_latency_ms
    );
    println!();

    // Start monitoring (runs in background)
    let monitor_configured = NetworkMonitor::from_config(config.clone());
    monitor_configured
        .start_monitoring_with_config(config)
        .await;

    println!("Monitoring started in background.");
    println!("Collecting data for 5 seconds...");

    // Let it run for a few seconds
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Get final metrics
    if let Ok(final_metrics) = monitor_configured.get_metrics().await {
        println!("\nFinal Metrics After Monitoring:");
        println!("--------------------------------");
        println!(
            "Total Download: {:.2} Mbps",
            final_metrics.total_download_mbps
        );
        println!("Total Upload: {:.2} Mbps", final_metrics.total_upload_mbps);

        if !final_metrics.speeds.is_empty() {
            println!("\nPer-Interface Statistics:");
            for (interface, speed) in final_metrics.speeds.iter() {
                println!(
                    "  {} - Down: {:.2} Mbps, Up: {:.2} Mbps",
                    interface, speed.download_speed_mbps, speed.upload_speed_mbps
                );
            }
        }
    }

    println!("\nNetwork Monitoring Example Complete!");

    Ok(())
}
