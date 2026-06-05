// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use crate::services::thread_manager::{self, ThreadConfig};
use simple_websockets::{Event, Responder};
use std::collections::HashMap;
use std::sync::atomic::Ordering;

pub fn init() {
    let config = ThreadConfig {
        name: "websocket_server".to_string(),
        restart_on_panic: true,
        max_restarts: 5,
        restart_delay_ms: 3000,
        health_check_interval_ms: Some(30000),
        enable_monitoring: true,
        priority: crate::services::thread_manager::ThreadPriority::Normal,
        max_memory_mb: None,
        cpu_affinity: None,
    };

    thread_manager::spawn_with_config(config, move |shutdown_signal, _health_rx| {
        log::info!("WebSocket server starting on port 2794");

        // listen for WebSockets on port 2794:
        let event_hub = match simple_websockets::launch(2794) {
            Ok(hub) => hub,
            Err(e) => {
                log::error!("Failed to listen on port 2794: {:?}", e);
                return;
            }
        };

        // map between client ids and the client's `Responder`:
        let mut clients: HashMap<u64, Responder> = HashMap::new();

        while !shutdown_signal.load(Ordering::Relaxed) {
            match event_hub.poll_event() {
                Event::Connect(client_id, responder) => {
                    log::info!("A WSS client connected with id #{}", client_id);
                    // add their Responder to our `clients` map:
                    clients.insert(client_id, responder);
                }
                Event::Disconnect(client_id) => {
                    log::info!("WSS Client #{} disconnected.", client_id);
                    // remove the disconnected client from the clients map:
                    clients.remove(&client_id);
                }
                Event::Message(client_id, message) => {
                    log::info!(
                        "WSS Received a message from client #{}: {:?}",
                        client_id,
                        message
                    );
                    // retrieve this client's `Responder`:
                    if let Some(responder) = clients.get(&client_id) {
                        // echo the message back:
                        responder.send(message);
                    } else {
                        log::error!("Client #{} not found in clients map", client_id);
                    }
                }
            }

            // Small sleep to prevent busy-waiting
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        log::info!("WebSocket server stopped");
    });
}
