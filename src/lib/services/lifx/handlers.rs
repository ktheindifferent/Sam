use super::bulb::BulbInfo;
use super::protocol::ProtocolHandler;
use colors_transform::{Color as TransformColor, Rgb as TransformRgb};
use lifx_rs::lan::{PowerLevel, HSBK};
use palette::{FromColor, Hsv};
use rouille::{post_input, try_or_400, Request, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::UdpSocket;
use serde_json::json;

#[derive(Debug, Deserialize)]
pub struct StateInput {
    pub power: Option<String>,
    pub color: Option<String>,
    pub brightness: Option<f64>,
    pub duration: Option<f64>,
    pub infrared: Option<f64>,
    pub fast: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct StateResult {
    pub id: String,
    pub label: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct StateResults {
    pub results: Vec<StateResult>,
}

pub struct HttpHandlers {
    protocol: ProtocolHandler,
}

impl HttpHandlers {
    pub fn new(source: u32) -> Self {
        Self {
            protocol: ProtocolHandler::new(source),
        }
    }

    pub fn handle_list_lights(&self, bulbs: &HashMap<u64, BulbInfo>, selector: &str) -> Response {
        let mut bulbs_vec: Vec<&BulbInfo> = bulbs.values().collect();

        if selector.contains("group_id:") {
            let group_id = selector.replace("group_id:", "");
            bulbs_vec.retain(|b| {
                b.lifx_group
                    .as_ref()
                    .map(|g| g.id.contains(&group_id))
                    .unwrap_or(false)
            });
        }

        if selector.contains("location_id:") {
            let location_id = selector.replace("location_id:", "");
            bulbs_vec.retain(|b| {
                b.lifx_location
                    .as_ref()
                    .map(|l| l.id.contains(&location_id))
                    .unwrap_or(false)
            });
        }

        if selector.contains("id:") {
            let id = selector.replace("id:", "");
            bulbs_vec.retain(|b| b.id.contains(&id));
        }

        Response::json(&bulbs_vec)
    }

    pub fn handle_set_state(
        &self,
        request: &Request,
        bulbs: &HashMap<u64, BulbInfo>,
        selector: &str,
        sock: &UdpSocket,
    ) -> Response {
        let input = try_or_400!(post_input!(request, {
            power: Option<String>,
            color: Option<String>,
            brightness: Option<f64>,
            duration: Option<f64>,
            infrared: Option<f64>,
            fast: Option<bool>
        }));

        let mut bulbs_vec: Vec<&BulbInfo> = bulbs.values().collect();

        if selector != "all" {
            if selector.contains("group_id:") {
                let group_id = selector.replace("group_id:", "");
                bulbs_vec.retain(|b| {
                    b.lifx_group
                        .as_ref()
                        .map(|g| g.id.contains(&group_id))
                        .unwrap_or(false)
                });
            }

            if selector.contains("location_id:") {
                let location_id = selector.replace("location_id:", "");
                bulbs_vec.retain(|b| {
                    b.lifx_location
                        .as_ref()
                        .map(|l| l.id.contains(&location_id))
                        .unwrap_or(false)
                });
            }

            if selector.contains("id:") {
                let id = selector.replace("id:", "");
                bulbs_vec.retain(|b| b.id.contains(&id));
            }
        }

        let duration = input.duration.unwrap_or(0.0) as u32;
        let mut results = Vec::new();

        for bulb in &bulbs_vec {
            let mut status = "ok";

            if let Some(ref power) = input.power {
                let power_level = if power == "on" {
                    PowerLevel::Enabled
                } else {
                    PowerLevel::Standby
                };
                if let Err(_) = self.protocol.send_power_command(sock, bulb.target, bulb.addr, power_level) {
                    status = "error";
                }
            }

            if let Some(ref color_str) = input.color {
                if let Some(color_cmd) = self.parse_color_command(color_str, bulb, duration) {
                    if let Err(_) = self.protocol.send_color_command(
                        sock,
                        bulb.target,
                        bulb.addr,
                        color_cmd,
                        duration,
                    ) {
                        status = "error";
                    }
                }
            }

            if let Some(brightness) = input.brightness {
                let current_color = bulb.lifx_color.as_ref().map(|c| HSBK {
                    hue: c.hue,
                    saturation: c.saturation,
                    brightness: (brightness * 65535.0) as u16,
                    kelvin: c.kelvin,
                }).unwrap_or(HSBK {
                    hue: 0,
                    saturation: 0,
                    brightness: (brightness * 65535.0) as u16,
                    kelvin: 6500,
                });

                if let Err(_) = self.protocol.send_color_command(
                    sock,
                    bulb.target,
                    bulb.addr,
                    current_color,
                    duration,
                ) {
                    status = "error";
                }
            }

            if let Some(infrared) = input.infrared {
                let brightness = (infrared * 65535.0) as u16;
                if let Err(_) = self.protocol.send_infrared_command(sock, bulb.target, bulb.addr, brightness) {
                    status = "error";
                }
            }

            results.push(StateResult {
                id: bulb.id.clone(),
                label: bulb.label.clone(),
                status: status.to_string(),
            });
        }

        Response::json(&StateResults { results })
    }

    fn parse_color_command(&self, color_str: &str, bulb: &BulbInfo, duration: u32) -> Option<HSBK> {
        let kelvin = bulb.lifx_color.as_ref().map(|c| c.kelvin).unwrap_or(6500);
        let brightness = bulb.lifx_color.as_ref().map(|c| c.brightness).unwrap_or(65535);
        let saturation = bulb.lifx_color.as_ref().map(|c| c.saturation).unwrap_or(0);
        let hue = bulb.lifx_color.as_ref().map(|c| c.hue).unwrap_or(0);

        if color_str.contains("white") {
            return Some(HSBK {
                hue: 0,
                saturation: 0,
                brightness,
                kelvin,
            });
        }

        if color_str.contains("red") {
            return Some(HSBK {
                hue: 0,
                saturation: 65535,
                brightness,
                kelvin,
            });
        }

        if color_str.contains("orange") {
            return Some(HSBK {
                hue: 7098,
                saturation: 65535,
                brightness,
                kelvin,
            });
        }

        if color_str.contains("yellow") {
            return Some(HSBK {
                hue: 10920,
                saturation: 65535,
                brightness,
                kelvin,
            });
        }

        if color_str.contains("cyan") {
            return Some(HSBK {
                hue: 32760,
                saturation: 65535,
                brightness,
                kelvin,
            });
        }

        if color_str.contains("green") {
            return Some(HSBK {
                hue: 21840,
                saturation: 65535,
                brightness,
                kelvin,
            });
        }

        if color_str.contains("blue") {
            return Some(HSBK {
                hue: 43680,
                saturation: 65535,
                brightness,
                kelvin,
            });
        }

        if color_str.contains("purple") {
            return Some(HSBK {
                hue: 50050,
                saturation: 65535,
                brightness,
                kelvin,
            });
        }

        if color_str.contains("pink") {
            return Some(HSBK {
                hue: 63700,
                saturation: 25000,
                brightness,
                kelvin,
            });
        }

        if color_str.contains("hue:") {
            if let Some(hue_val) = self.extract_value(color_str, "hue:") {
                return Some(HSBK {
                    hue: hue_val.parse().ok()?,
                    saturation,
                    brightness,
                    kelvin,
                });
            }
        }

        if color_str.contains("saturation:") {
            if let Some(sat_val) = self.extract_value(color_str, "saturation:") {
                let sat_float: f64 = sat_val.parse().ok()?;
                return Some(HSBK {
                    hue,
                    saturation: (sat_float * 655.35) as u16,
                    brightness,
                    kelvin,
                });
            }
        }

        if color_str.contains("brightness:") {
            if let Some(bright_val) = self.extract_value(color_str, "brightness:") {
                let bright_float: f64 = bright_val.parse().ok()?;
                return Some(HSBK {
                    hue,
                    saturation,
                    brightness: (bright_float * 65535.0) as u16,
                    kelvin,
                });
            }
        }

        if color_str.contains("kelvin:") {
            if let Some(kelvin_val) = self.extract_value(color_str, "kelvin:") {
                return Some(HSBK {
                    hue,
                    saturation: 0,
                    brightness,
                    kelvin: kelvin_val.parse().ok()?,
                });
            }
        }

        if color_str.contains("rgb:") {
            if let Some(rgb_val) = self.extract_value(color_str, "rgb:") {
                let parts: Vec<&str> = rgb_val.split(',').collect();
                if parts.len() == 3 {
                    let r: f32 = parts[0].parse().ok()?;
                    let g: f32 = parts[1].parse().ok()?;
                    let b: f32 = parts[2].parse().ok()?;
                    let rgb = palette::rgb::Rgb::<palette::encoding::Srgb, f32>::new(r, g, b);
                    let hsv = Hsv::from_color(rgb);
                    return Some(HSBK {
                        hue: (hsv.hue.into_positive_degrees() * 182.0) as u16,
                        saturation: (hsv.saturation * 65535.0) as u16,
                        brightness,
                        kelvin,
                    });
                }
            }
        }

        if color_str.contains("#") {
            if let Some(hex) = self.extract_value(color_str, "#") {
                if let Ok(rgb) = TransformRgb::from_hex_str(&format!("#{}", hex)) {
                    let r = rgb.get_red();
                    let g = rgb.get_green();
                    let b = rgb.get_blue();
                    let rgb = palette::rgb::Rgb::<palette::encoding::Srgb, f32>::new(r, g, b);
                    let hsv = Hsv::from_color(rgb);
                    return Some(HSBK {
                        hue: (hsv.hue.into_positive_degrees() * 182.0) as u16,
                        saturation: (hsv.saturation * 65535.0) as u16,
                        brightness,
                        kelvin,
                    });
                }
            }
        }

        None
    }

    fn extract_value<'a>(&self, input: &'a str, prefix: &str) -> Option<&'a str> {
        input.find(prefix).map(|pos| {
            let start = pos + prefix.len();
            let rest = &input[start..];
            rest.split_whitespace().next().unwrap_or(rest)
        })
    }
}

/// Handle LIFX API requests from the main HTTP server
pub fn handle_api_request(request: &Request) -> Response {
    use crate::services::lifx::{get_status_json, get_global_discovery};

    // Status endpoint (already handled in mod.rs, but here for completeness)
    if request.url().contains("/status") {
        return Response::json(&get_status_json());
    }

    // List bulbs endpoint - now uses real discovery service
    if request.url().contains("/bulbs") || request.url().contains("/lights") {
        if let Some(discovery_arc) = get_global_discovery() {
            if let Ok(discovery) = discovery_arc.lock() {
                if let Ok(bulbs_arc) = discovery.get_bulbs().lock() {
                    let bulbs_vec: Vec<&BulbInfo> = bulbs_arc.values().collect();
                    return Response::json(&json!({
                        "bulbs": bulbs_vec,
                        "count": bulbs_vec.len()
                    }));
                }
            }
        }
        // Fallback if discovery service not available
        return Response::json(&json!({
            "bulbs": [],
            "count": 0,
            "message": "Discovery service not initialized"
        }));
    }

    // Set color endpoint - now uses real bulb control
    if request.url().contains("/set_color") {
        let input = try_or_400!(post_input!(request, {
            use_public: Option<String>,
            selector: String,
            color: String
        }));

        log::info!("LIFX set_color request: selector={}, color={}", input.selector, input.color);

        // Get discovery service and execute command
        if let Some(discovery_arc) = get_global_discovery() {
            if let Ok(discovery) = discovery_arc.lock() {
                if let Ok(bulbs_arc) = discovery.get_bulbs().lock() {
                    let sock = discovery.get_socket();
                    let handlers = HttpHandlers::new(0);

                    let mut bulbs_vec: Vec<&BulbInfo> = bulbs_arc.values().collect();

                    // Apply selector filtering
                    if input.selector.contains("group_id:") {
                        let gid = input.selector.replace("group_id:", "");
                        bulbs_vec.retain(|b| {
                            b.lifx_group.as_ref()
                                .map(|g| g.id.contains(&gid))
                                .unwrap_or(false)
                        });
                    } else if input.selector.contains("id:") {
                        let id = input.selector.replace("id:", "");
                        bulbs_vec.retain(|b| b.id.contains(&id));
                    }

                    // Execute color change for each bulb
                    let mut success_count = 0;
                    for bulb in &bulbs_vec {
                        if let Some(hsbk) = handlers.parse_color_command(&input.color, bulb, 0) {
                            if handlers.protocol.send_color_command(sock, bulb.target, bulb.addr, hsbk, 0).is_ok() {
                                success_count += 1;
                            }
                        }
                    }

                    return Response::json(&json!({
                        "success": success_count > 0,
                        "message": format!("Color command sent to {} bulbs", success_count),
                        "color": input.color,
                        "selector": input.selector
                    }));
                }
            }
        }

        return Response::json(&json!({
            "success": false,
            "message": "Discovery service not available"
        }));
    }

    // Set state endpoint - now uses real bulb control
    if request.url().contains("/set_state") {
        let input = try_or_400!(post_input!(request, {
            use_public: Option<String>,
            selector: String,
            power: String
        }));

        log::info!("LIFX set_state request: selector={}, power={}", input.selector, input.power);

        // Get discovery service and execute command
        if let Some(discovery_arc) = get_global_discovery() {
            if let Ok(discovery) = discovery_arc.lock() {
                if let Ok(bulbs_arc) = discovery.get_bulbs().lock() {
                    let sock = discovery.get_socket();

                    let mut bulbs_vec: Vec<&BulbInfo> = bulbs_arc.values().collect();

                    // Apply selector filtering
                    if input.selector.contains("group_id:") {
                        let gid = input.selector.replace("group_id:", "");
                        bulbs_vec.retain(|b| {
                            b.lifx_group.as_ref()
                                .map(|g| g.id.contains(&gid))
                                .unwrap_or(false)
                        });
                    } else if input.selector.contains("id:") {
                        let id = input.selector.replace("id:", "");
                        bulbs_vec.retain(|b| b.id.contains(&id));
                    }

                    // Execute power change for each bulb
                    let power_level = if input.power == "on" {
                        PowerLevel::Enabled
                    } else {
                        PowerLevel::Standby
                    };

                    let mut success_count = 0;
                    for bulb in &bulbs_vec {
                        let protocol = ProtocolHandler::new(0);
                        if protocol.send_power_command(sock, bulb.target, bulb.addr, power_level).is_ok() {
                            success_count += 1;
                        }
                    }

                    return Response::json(&json!({
                        "success": success_count > 0,
                        "message": format!("Power {} command sent to {} bulbs", input.power, success_count),
                        "power": input.power,
                        "selector": input.selector
                    }));
                }
            }
        }

        return Response::json(&json!({
            "success": false,
            "message": "Discovery service not available"
        }));
    }

    Response::empty_404()
}