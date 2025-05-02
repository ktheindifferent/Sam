use std::sync::Arc;
use tokio::sync::Mutex;
use crate::sam::services::matter::MatterDeviceController;
use crate::sam::memory::Thing;
use crate::sam::cli::spinner::run_with_spinner;

// matter pair 192.168.86.163:5540 33134851532 100 300 0.0.0.0:5555
pub async fn handle_matter(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    let mut parts = cmd.trim().split_whitespace();
    parts.next(); // skip 'matter'
    match parts.next() {
        Some("pair") => {
            let device_address = parts.next();
            let pin = parts.next();
            let local_address = parts.next().unwrap_or("0.0.0.0:5555");
            if let (Some(device_address), Some(pin)) = (device_address, pin) {
                let output_lines = output_lines.clone();
                let device_address = device_address.to_string();
                let pin = pin.to_string();
                let local_address = local_address.to_string();
                run_with_spinner(
                    &output_lines,
                    &format!("Pairing Matter device at {device_address}..."),
                    |lines, msg| lines.push(msg.to_string()),
                    move || async move {
                        match MatterDeviceController::pair_device(&device_address, &pin, &local_address).await {
                            Ok(thing) => format!("Paired Matter device: {} (oid: {})", thing.name, thing.oid),
                            Err(e) => format!("Pairing failed: {e}"),
                        }
                    },
                ).await;
            } else {
                let mut out = output_lines.lock().await;
                out.push("Usage: matter pair <device_address> <pin> [local_address]".to_string());
            }
        }
        Some("set") => {
            match parts.next() {
                Some("power") => {
                    let oid = parts.next();
                    let state = parts.next();
                    let local_address = parts.next().unwrap_or("0.0.0.0:5555");
                    if let (Some(oid), Some(state)) = (oid, state) {
                        let on = match state.to_lowercase().as_str() {
                            "on" => true,
                            "off" => false,
                            _ => {
                                let mut out = output_lines.lock().await;
                                out.push("Usage: matter set power <thing_oid> <on|off> [local_address]".to_string());
                                return;
                            }
                        };
                        let oid = oid.to_string();
                        let local_address = local_address.to_string();
                        let output_lines = output_lines.clone();
                        run_with_spinner(
                            &output_lines,
                            &format!("Setting device {oid} power {}...", if on {"ON"} else {"OFF"}),
                            |lines, msg| lines.push(msg.to_string()),
                            move || async move {
                                let thing = Thing::select_async(None, None, None, None).await.unwrap_or_default().into_iter().find(|t| t.oid == oid);
                                match thing {
                                    Some(t) => {
                                        match MatterDeviceController::set_device_on_off(&t, on, &local_address).await {
                                            Ok(_) => format!("Set device {} {}", t.name, if on {"ON"} else {"OFF"}),
                                            Err(e) => format!("Failed to set device: {e}"),
                                        }
                                    }
                                    None => format!("Thing with oid {oid} not found."),
                                }
                            },
                        ).await;
                    } else {
                        let mut out = output_lines.lock().await;
                        out.push("Usage: matter set on/off <thing_oid> <on|off> <controller_id> [cert_path] [local_address]".to_string());
                    }
                }
                Some("level") => {
                    let oid = parts.next();
                    let level_str = parts.next();
                    let local_address = parts.next().unwrap_or("0.0.0.0:5555");
                    if let (Some(oid), Some(level_str)) = (oid, level_str) {
                        let oid = oid.to_string();
                        let level_str = level_str.to_string();
                        let local_address = local_address.to_string();
                        let output_lines = output_lines.clone();
                        run_with_spinner(
                            &output_lines,
                            &format!("Setting device {oid} level..."),
                            |lines, msg| lines.push(msg.to_string()),
                            move || async move {
                                match level_str.parse::<u8>() {
                                    Ok(level) => {
                                        let thing = Thing::select_async(None, None, None, None).await.unwrap_or_default().into_iter().find(|t| t.oid == oid);
                                        match thing {
                                            Some(t) => {
                                                match MatterDeviceController::set_device_level(&t, level, &local_address).await {
                                                    Ok(_) => format!("Set device {} level to {}", t.name, level),
                                                    Err(e) => format!("Failed to set level: {e}"),
                                                }
                                            }
                                            None => format!("Thing with oid {oid} not found."),
                                        }
                                    }
                                    Err(_) => "Invalid level value".to_string(),
                                }
                            },
                        ).await;
                    } else {
                        let mut out = output_lines.lock().await;
                        out.push("Usage: matter set level <thing_oid> <level> <controller_id> [cert_path] [local_address]".to_string());
                    }
                }
                _ => {
                    let mut out = output_lines.lock().await;
                    out.push("Usage: matter set on/off <thing_oid> <on|off> <controller_id> [cert_path] [local_address] | matter set level <thing_oid> <level> <controller_id> [cert_path] [local_address]".to_string());
                }
            }
        }
        Some("discover") => {
            let timeout = parts.next().and_then(|t| t.parse::<u64>().ok()).unwrap_or(5);
            let output_lines = output_lines.clone();
            run_with_spinner(
                &output_lines,
                &format!("Discovering Matter devices (timeout: {timeout}s)..."),
                |lines, msg| lines.push(msg.to_string()),
                move || async move {
                    use matc::discover;
                    use std::time::Duration;
                    match discover::discover_commissionable(Duration::from_secs(timeout)).await {
                        Ok(infos) => {
                            if infos.is_empty() {
                                "No Matter devices found.".to_string()
                            } else {
                                let mut msg = format!("Discovered {} Matter device(s):\n", infos.len());
                                for info in infos {
                                    msg.push_str(&format!("{:#?}\n", info));
                                }
                                msg
                            }
                        }
                        Err(e) => format!("Discovery error: {e}"),
                    }
                },
            ).await;
        }
        _ => {
            let mut out = output_lines.lock().await;
            out.push("Matter commands: pair, set power, set level".to_string());
            out.push("Usage: matter pair <device_address> <pin> [local_address]".to_string());
            out.push("       matter set power <thing_oid> <on|off> [local_address]".to_string());
            out.push("       matter set level <thing_oid> <0-100> [local_address]".to_string());
        }
    }
}
