use std::sync::Arc;
use tokio::sync::Mutex;
use crate::sam::services::matter::MatterDeviceController;
use crate::sam::memory::Thing;


// matter pair 192.168.86.163:5540 33134851532 100 300 0.0.0.0:5555
pub async fn handle_matter(cmd: &str, output_lines: &Arc<Mutex<Vec<String>>>) {
    let mut parts = cmd.trim().split_whitespace();
    parts.next(); // skip 'matter'
    match parts.next() {
        Some("pair") => {
            // Usage: matter pair <device_address> <pin> <controller_id> <device_id> [cert_path] [local_address]
            let device_address = parts.next();
            let pin = parts.next();
            // let controller_id = parts.next();
            // let device_id = parts.next();
            let cert_path = "/opt/sam/keys/matter/";
            // let cert_path = parts.next().unwrap_or("./pem");
            // let local_address = parts.next().unwrap_or("0.0.0.0:5555");
            if let (Some(device_address), Some(pin), Some(controller_id), Some(device_id)) = (device_address, pin, controller_id, device_id) {
                // Ensure cert_path directory exists, even if no parent
                
                let pin = pin.parse::<u32>().unwrap_or(20202021);
                let controller_id = 100;
                let device_id = 100;
                let res = MatterDeviceController::pair_device(
                    device_address,
                    pin,
                    controller_id,
                    device_id,
                    cert_path,
                    local_address,
                ).await;
                let mut out = output_lines.lock().await;
                match res {
                    Ok(thing) => out.push(format!("Paired Matter device: {} (oid: {})", thing.name, thing.oid)),
                    Err(e) => out.push(format!("Pairing failed: {e}")),
                }
            } else {
                let mut out = output_lines.lock().await;
                out.push("Usage: matter pair <device_address> <pin> <controller_id> <device_id> [cert_path] [local_address]".to_string());
            }
        }
        Some("set") => {
            match parts.next() {
                Some("on/off") => {
                    // Usage: matter set on/off <thing_oid> <on|off> <controller_id> [cert_path] [local_address]
                    let oid = parts.next();
                    let state = parts.next();
                    let controller_id = parts.next();
                    let cert_path = parts.next().unwrap_or("./pem");
                    let local_address = parts.next().unwrap_or("0.0.0.0:5555");
                    if let (Some(oid), Some(state), Some(controller_id)) = (oid, state, controller_id) {
                        let on = match state.to_lowercase().as_str() {
                            "on" => true,
                            "off" => false,
                            _ => {
                                let mut out = output_lines.lock().await;
                                out.push("Usage: matter set on/off <thing_oid> <on|off> <controller_id> [cert_path] [local_address]".to_string());
                                return;
                            }
                        };
                        let controller_id = controller_id.parse::<u64>().unwrap_or(100);
                        let thing = Thing::select(None, None, None, None).unwrap_or_default().into_iter().find(|t| t.oid == oid);
                        match thing {
                            Some(t) => {
                                let res = MatterDeviceController::set_device_on_off(&t, on, controller_id, cert_path, local_address).await;
                                let mut out = output_lines.lock().await;
                                match res {
                                    Ok(_) => out.push(format!("Set device {} {}", t.name, if on {"ON"} else {"OFF"})),
                                    Err(e) => out.push(format!("Failed to set device: {e}")),
                                }
                            }
                            None => {
                                let mut out = output_lines.lock().await;
                                out.push(format!("Thing with oid {oid} not found."));
                            }
                        }
                    } else {
                        let mut out = output_lines.lock().await;
                        out.push("Usage: matter set on/off <thing_oid> <on|off> <controller_id> [cert_path] [local_address]".to_string());
                    }
                }
                Some("level") => {
                    // Usage: matter set level <thing_oid> <level> <controller_id> [cert_path] [local_address]
                    let oid = parts.next();
                    let level_str = parts.next();
                    let controller_id = parts.next();
                    let cert_path = parts.next().unwrap_or("./pem");
                    let local_address = parts.next().unwrap_or("0.0.0.0:5555");
                    if let (Some(oid), Some(level_str), Some(controller_id)) = (oid, level_str, controller_id) {
                        match level_str.parse::<u8>() {
                            Ok(level) => {
                                let controller_id = controller_id.parse::<u64>().unwrap_or(100);
                                let thing = Thing::select(None, None, None, None).unwrap_or_default().into_iter().find(|t| t.oid == oid);
                                match thing {
                                    Some(t) => {
                                        let res = MatterDeviceController::set_device_level(&t, level, controller_id, cert_path, local_address).await;
                                        let mut out = output_lines.lock().await;
                                        match res {
                                            Ok(_) => out.push(format!("Set device {} level to {}", t.name, level)),
                                            Err(e) => out.push(format!("Failed to set level: {e}")),
                                        }
                                    }
                                    None => {
                                        let mut out = output_lines.lock().await;
                                        out.push(format!("Thing with oid {oid} not found."));
                                    }
                                }
                            }
                            Err(_) => {
                                let mut out = output_lines.lock().await;
                                out.push("Usage: matter set level <thing_oid> <level> <controller_id> [cert_path] [local_address]".to_string());
                            }
                        }
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
            // Usage: matter discover [timeout_seconds]
            let timeout = parts.next().and_then(|t| t.parse::<u64>().ok()).unwrap_or(5);
            let mut out = output_lines.lock().await;
            out.push(format!("Discovering Matter devices (timeout: {timeout}s)..."));
            drop(out); // Release lock before await

            let output_lines = output_lines.clone();
            tokio::spawn(async move {
                use matc::discover;
                use std::time::Duration;
                match discover::discover_commissionable(Duration::from_secs(timeout)).await {
                    Ok(infos) => {
                        let mut out = output_lines.lock().await;
                        if infos.is_empty() {
                            out.push("No Matter devices found.".to_string());
                        } else {
                            out.push(format!("Discovered {} Matter device(s):", infos.len()));
                            for info in infos {
                                out.push(format!("{:#?}", info));
                            }
                        }
                    }
                    Err(e) => {
                        let mut out = output_lines.lock().await;
                        out.push(format!("Discovery error: {e}"));
                    }
                }
            });
        }
        _ => {
            let mut out = output_lines.lock().await;
            out.push("Matter commands: pair, set on/off, set level".to_string());
            out.push("Usage: matter pair <device_address> <pin> <controller_id> <device_id> [cert_path] [local_address]".to_string());
            out.push("       matter set on/off <thing_oid> <on|off> <controller_id> [cert_path] [local_address]".to_string());
            out.push("       matter set level <thing_oid> <level> <controller_id> [cert_path] [local_address]".to_string());
        }
    }
}
