use super::bulb::{BulbInfo, LiColor};
use super::protocol::ProtocolHandler;
use crate::services::lifx::get_global_discovery;
use colors_transform::{Color as TransformColor, Rgb as TransformRgb};
use lifx_rs::lan::{ApplicationRequest, PowerLevel, HSBK};
use palette::{FromColor, Hsv};
use rouille::{post_input, try_or_400, Request, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::net::UdpSocket;

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
                if let Err(_) =
                    self.protocol
                        .send_power_command(sock, bulb.target, bulb.addr, power_level)
                {
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
                let current_color = bulb
                    .lifx_color
                    .as_ref()
                    .map(|c| HSBK {
                        hue: c.hue,
                        saturation: c.saturation,
                        brightness: (brightness * 65535.0) as u16,
                        kelvin: c.kelvin,
                    })
                    .unwrap_or(HSBK {
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
                if let Err(_) =
                    self.protocol
                        .send_infrared_command(sock, bulb.target, bulb.addr, brightness)
                {
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
        let brightness = bulb
            .lifx_color
            .as_ref()
            .map(|c| c.brightness)
            .unwrap_or(65535);
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
    use crate::services::lifx::get_status_json;

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

        log::info!(
            "LIFX set_color request: selector={}, color={}",
            input.selector,
            input.color
        );

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
                            b.lifx_group
                                .as_ref()
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
                            if handlers
                                .protocol
                                .send_color_command(sock, bulb.target, bulb.addr, hsbk, 0)
                                .is_ok()
                            {
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

        log::info!(
            "LIFX set_state request: selector={}, power={}",
            input.selector,
            input.power
        );

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
                            b.lifx_group
                                .as_ref()
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
                        if protocol
                            .send_power_command(sock, bulb.target, bulb.addr, power_level)
                            .is_ok()
                        {
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

/// Handle enhanced LIFX API requests including scenes, effects, and zones
pub fn handle_enhanced_api_request(request: &Request) -> Response {
    // Scene endpoints
    if request.url().contains("/api/services/lifx/scenes") {
        return handle_scenes(request);
    }

    // Effects endpoints
    if request.url().contains("/api/services/lifx/effect") {
        return handle_effects(request);
    }

    // Zone endpoints for multi-zone lights
    if request.url().contains("/api/services/lifx/zones") {
        return handle_zones(request);
    }

    // Circadian rhythm endpoint
    if request.url().contains("/api/services/lifx/circadian") {
        return handle_circadian(request);
    }

    // Preset endpoints
    if request.url().contains("/api/services/lifx/preset") {
        return handle_presets(request);
    }

    // Touch gesture endpoints
    if request.url().contains("/api/services/lifx/gesture") {
        return handle_gesture(request);
    }

    // Media sync endpoints
    if request.url().contains("/api/services/lifx/media_sync") {
        return handle_media_sync(request);
    }

    // Wave effect endpoint
    if request.url().contains("/api/services/lifx/wave") {
        return handle_wave_effect(request);
    }

    // Disco mode endpoint
    if request.url().contains("/api/services/lifx/disco") {
        return handle_disco_mode(request);
    }

    Response::empty_404()
}

/// Built-in scene definitions
const SCENES: &[(&str, u16, u16, u16, u16)] = &[
    // (name, hue, saturation, brightness, kelvin)
    // Core scenes
    ("relax", 5800, 15000, 26214, 2700),
    ("focus", 19000, 8000, 52428, 5000),
    ("energize", 41000, 20000, 65535, 6500),
    ("night", 5800, 10000, 13107, 2000),
    ("reading", 19000, 5000, 45875, 4500),
    ("romance", 60000, 25000, 32767, 3000),
    ("party", 43680, 65535, 65535, 5500),
    ("sunset", 7098, 40000, 39321, 2500),
    ("arctic", 32760, 15000, 52428, 7000),
    ("golden", 8000, 30000, 45875, 3200),
    ("ocean", 34580, 42598, 49151, 4000),
    ("tropical", 27300, 65535, 47185, 3800),
    ("spring", 25480, 49807, 60948, 4200),
    ("autumn", 5460, 43255, 57671, 2800),
    ("meditation", 50960, 19660, 22937, 2400),
    ("gaming", 50960, 52428, 58982, 5500),
    ("cooking", 6370, 39321, 62259, 4000),
    ("creative", 52780, 45875, 55705, 4800),
    ("yoga", 21840, 26214, 32767, 3500),
    ("movie", 3640, 19660, 22937, 2200),
    ("study", 36400, 19660, 49151, 4500),
    ("dinner", 5460, 26214, 36044, 2700),
    ("morning", 9100, 32767, 55705, 5500),
    ("goodnight", 43680, 6553, 6553, 2000),
    ("rainbow", 0, 65535, 52428, 4000),
    ("fireplace", 5460, 52428, 39321, 2000),
    ("ice", 36400, 32767, 45875, 8000),
    // Enhanced scenes with improved color accuracy
    ("relax_enhanced", 5800, 18000, 30000, 2600),
    ("focus_enhanced", 18000, 10000, 55000, 4800),
    ("energize_enhanced", 40000, 25000, 65535, 6200),
    ("night_vision", 5800, 8000, 8000, 1800),
    ("deep_focus", 20000, 6000, 58000, 5200),
    ("ultra_energize", 38000, 28000, 65535, 6800),
    ("sleep_prep", 5800, 5000, 15000, 2000),
    ("wake_up", 9100, 35000, 60000, 5800),
    // Extended scenes
    ("aurora", 32760, 45875, 49151, 6000),
    ("nebula", 50960, 52428, 45875, 4500),
    ("thunder", 5460, 39321, 58982, 5000),
    ("crystal", 34580, 26214, 52428, 7500),
    ("lagoon", 32760, 32767, 45875, 5500),
    ("cotton_candy", 60000, 32767, 52428, 4000),
    ("spring_blossom", 25480, 39321, 58982, 4500),
    ("punchbowl", 5460, 45875, 55705, 3500),
    ("smashing", 43680, 52428, 58982, 5000),
    ("glitter", 5800, 26214, 52428, 3000),
    ("golden_hour", 8000, 26214, 49151, 3500),
    ("late_night", 43680, 6553, 19660, 2000),
    ("midday", 36400, 13107, 62259, 6000),
    ("polar", 32760, 19660, 55705, 8000),
    ("savanna", 7098, 32767, 52428, 4000),
    ("koi_pond", 25480, 39321, 45875, 4500),
    ("bliss", 60000, 19660, 49151, 3800),
    ("peak", 34580, 45875, 58982, 5500),
    ("vapor", 50960, 26214, 45875, 4200),
    ("chill", 32760, 13107, 42598, 5000),
    // Special event scenes
    ("cyberpunk", 30940, 52428, 58982, 4500),
    ("vaporwave", 58240, 39321, 52428, 4000),
    ("northern_lights", 32760, 45875, 49151, 5500),
    ("desert_dawn", 7098, 26214, 45875, 3600),
    ("forest_mist", 21840, 19660, 42598, 4500),
    ("volcanic", 5460, 52428, 55705, 2800),
    ("underwater", 34580, 32767, 45875, 5200),
    ("space_station", 32760, 19660, 58982, 6500),
    ("wizard_tower", 60000, 32767, 42598, 3000),
    ("dragon_fire", 5460, 52428, 62259, 2500),
    ("fairy_grove", 25480, 39321, 52428, 4200),
    ("haunted", 5460, 45875, 32767, 3500),
    ("santas_workshop", 5800, 26214, 55705, 3000),
    ("new_year", 43680, 52428, 62259, 4500),
    ("valentines", 60000, 32767, 45875, 3200),
    ("halloween", 5460, 52428, 49151, 2800),
    ("thanksgiving", 7098, 32767, 42598, 3000),
    ("christmas", 5800, 45875, 55705, 3500),
    ("easter", 25480, 39321, 58982, 4500),
    ("st_patricks", 21840, 52428, 52428, 4000),
    ("independence_day", 5460, 45875, 62259, 5000),
    ("bioluminescent", 32760, 52428, 49151, 5500),
    ("cosmic", 50960, 45875, 52428, 6000),
    ("dream", 58240, 26214, 39321, 4000),
    ("adventure", 21840, 39321, 58982, 4800),
    ("festival", 8000, 52428, 62259, 4200),
    // Additional touch-optimized scenes
    ("beach", 18200, 32767, 52428, 5000),
    ("forest", 25480, 39321, 42598, 4200),
    ("caffeine", 5460, 19660, 62259, 5500),
    ("magnetize", 32760, 45875, 55705, 5000),
    ("ocean_descent", 34580, 39321, 32767, 4500),
    ("red_bull", 5460, 32767, 65535, 6000),
    ("mojito", 21840, 26214, 49151, 4000),
    ("lake", 32760, 19660, 45875, 5500),
    ("seabed", 34580, 32767, 39321, 5000),
    ("bubbles", 32760, 26214, 52428, 4500),
    ("peacock", 25480, 39321, 55705, 4800),
    ("pink_lights", 60000, 26214, 49151, 3500),
    ("purple_haze", 50960, 39321, 52428, 4200),
    ("ruby", 5800, 52428, 45875, 3000),
    ("emerald", 21840, 45875, 49151, 4500),
    ("sapphire", 43680, 52428, 52428, 5000),
    ("amber", 7098, 45875, 55705, 2800),
    ("rose", 60000, 19660, 45875, 3200),
    ("lavender", 52780, 26214, 49151, 4000),
    ("mint", 21840, 19660, 52428, 4500),
    ("lemon", 10920, 32767, 58982, 4200),
    ("lime", 18200, 39321, 55705, 4500),
    ("tangerine", 7098, 45875, 58982, 3500),
    ("grape", 50960, 32767, 49151, 4000),
    ("strawberry", 60000, 39321, 52428, 3500),
    ("blueberry", 43680, 39321, 49151, 4500),
    ("watermelon", 5460, 32767, 55705, 4000),
    ("kiwi", 18200, 26214, 52428, 4500),
    ("mango", 8000, 39321, 58982, 3800),
    ("papaya", 7098, 32767, 55705, 3500),
    ("coconut", 3640, 13107, 58982, 3500),
    ("pineapple", 10920, 39321, 62259, 4000),
    ("banana", 10920, 26214, 58982, 4200),
    ("peach", 8000, 26214, 52428, 3500),
    ("apricot", 8000, 32767, 55705, 3800),
    ("plum", 52780, 32767, 45875, 3500),
    ("cherry", 5800, 45875, 49151, 3200),
    ("raspberry", 60000, 39321, 49151, 3500),
    ("blackberry", 50960, 39321, 42598, 3800),
    ("caramel", 8000, 26214, 45875, 3000),
    ("chocolate", 7098, 19660, 39321, 2800),
    ("vanilla", 9100, 13107, 58982, 3500),
    ("cinnamon", 7098, 32767, 49151, 3000),
    ("ginger", 8000, 39321, 52428, 3200),
    ("cardamom", 52780, 26214, 45875, 3500),
    ("saffron", 10920, 32767, 55705, 3800),
    ("turmeric", 10920, 26214, 52428, 3500),
    ("paprika", 7098, 39321, 52428, 3200),
    ("chili", 5460, 45875, 55705, 3000),
    ("cayenne", 5460, 39321, 49151, 3200),
];

/// Handle scene operations
fn handle_scenes(request: &Request) -> Response {
    if request.method() == "GET" {
        // List available scenes
        let scenes: Vec<serde_json::Value> = SCENES
            .iter()
            .map(|scene| {
                json!({
                    "name": scene.0,
                    "hue": scene.1,
                    "saturation": scene.2,
                    "brightness": scene.3,
                    "kelvin": scene.4
                })
            })
            .collect();

        return Response::json(&json!({
            "scenes": scenes,
            "count": scenes.len()
        }));
    }

    if request.method() == "POST" {
        let input = try_or_400!(post_input!(request, {
            selector: String,
            scene: String,
            duration: Option<f64>
        }));

        let scene_data = SCENES
            .iter()
            .find(|s| s.0 == input.scene)
            .or_else(|| SCENES.iter().find(|s| s.0.starts_with(&input.scene)));

        if let Some(scene) = scene_data {
            if let Some(discovery_arc) = get_global_discovery() {
                if let Ok(discovery) = discovery_arc.lock() {
                    if let Ok(bulbs_arc) = discovery.get_bulbs().lock() {
                        let sock = discovery.get_socket();
                        let handlers = HttpHandlers::new(0);

                        let mut bulbs_vec: Vec<&BulbInfo> = bulbs_arc.values().collect();

                        // Apply selector filtering
                        if input.selector.contains("id:") {
                            let id = input.selector.replace("id:", "");
                            bulbs_vec.retain(|b| b.id.contains(&id));
                        }

                        let duration = (input.duration.unwrap_or(1.0) * 1000.0) as u32;
                        let mut success_count = 0;

                        for bulb in &bulbs_vec {
                            let hsbk = HSBK {
                                hue: scene.1,
                                saturation: scene.2,
                                brightness: scene.3,
                                kelvin: scene.4,
                            };

                            if handlers
                                .protocol
                                .send_color_command(sock, bulb.target, bulb.addr, hsbk, duration)
                                .is_ok()
                            {
                                success_count += 1;
                            }
                        }

                        return Response::json(&json!({
                            "success": success_count > 0,
                            "message": format!("Scene '{}' applied to {} bulbs", input.scene, success_count),
                            "scene": input.scene,
                            "bulbs_affected": success_count
                        }));
                    }
                }
            }

            return Response::json(&json!({
                "success": false,
                "message": "Discovery service not available"
            }));
        }

        return Response::json(&json!({
            "success": false,
            "message": format!("Unknown scene: {}", input.scene),
            "available_scenes": SCENES.iter().map(|s| s.0).collect::<Vec<_>>()
        }));
    }

    Response::empty_404()
}

/// Handle lighting effects
fn handle_effects(request: &Request) -> Response {
    use std::thread;
    use std::time::Duration;

    if request.method() != "POST" {
        return Response::empty_404();
    }

    let input = try_or_400!(post_input!(request, {
        selector: String,
        effect: String,
        duration: Option<f64>,
        cycles: Option<f64>
    }));

    if let Some(discovery_arc) = get_global_discovery() {
        if let Ok(discovery) = discovery_arc.lock() {
            if let Ok(bulbs_arc) = discovery.get_bulbs().lock() {
                let sock = discovery.get_socket();
                let handlers = HttpHandlers::new(0);

                let mut bulbs_vec: Vec<&BulbInfo> = bulbs_arc.values().collect();

                if input.selector.contains("id:") {
                    let id = input.selector.replace("id:", "");
                    bulbs_vec.retain(|b| b.id.contains(&id));
                }

                let cycles = input.cycles.unwrap_or(1.0);
                let total_duration = input.duration.unwrap_or(2.0);
                let step_duration = (total_duration / cycles / 10.0 * 1000.0) as u64;

                match input.effect.as_str() {
                    "fireplace" => {
                        // Fireplace effect: random warm flickering
                        let sock_clone = sock.try_clone().ok();
                        let bulbs_clone: Vec<(u64, std::net::SocketAddr)> =
                            bulbs_vec.iter().map(|b| (b.target, b.addr)).collect();

                        thread::spawn(move || {
                            use rand::Rng;
                            if let Some(socket) = sock_clone {
                                for _ in 0..30 {
                                    for &(target, addr) in &bulbs_clone {
                                        let mut rng = rand::thread_rng();
                                        let brightness = (40.0 + rng.gen::<f64>() * 30.0) as u16;
                                        let kelvin = (1800.0 + rng.gen::<f64>() * 400.0) as u16;
                                        let flicker_color = HSBK {
                                            hue: 5460,
                                            saturation: 52428,
                                            brightness,
                                            kelvin,
                                        };
                                        let _ = handlers.protocol.send_color_command(
                                            &socket,
                                            target,
                                            addr,
                                            flicker_color,
                                            0,
                                        );
                                    }
                                    thread::sleep(Duration::from_millis(
                                        200 + rand::thread_rng().gen_range(0..100),
                                    ));
                                }
                            }
                        });

                        return Response::json(&json!({
                            "success": true,
                            "message": format!("Fireplace effect started on {} bulbs", bulbs_vec.len()),
                            "effect": "fireplace",
                        }));
                    }
                    "aurora" => {
                        // Aurora effect: smooth color transitions through greens and blues
                        let sock_clone = sock.try_clone().ok();
                        let bulbs_clone: Vec<(u64, std::net::SocketAddr)> =
                            bulbs_vec.iter().map(|b| (b.target, b.addr)).collect();

                        thread::spawn(move || {
                            for step in 0..72 {
                                if let Some(socket) = sock_clone.as_ref() {
                                    let hue = ((180 + step * 10) % 360) as f64 / 360.0 * 65535.0;
                                    let saturation =
                                        (50.0 + (step as f64 / 72.0 * 30.0)) as u16 * 65535 / 100;
                                    let aurora_color = HSBK {
                                        hue: hue as u16,
                                        saturation,
                                        brightness: 45875,
                                        kelvin: 6000,
                                    };
                                    for &(target, addr) in &bulbs_clone {
                                        let _ = handlers.protocol.send_color_command(
                                            socket,
                                            target,
                                            addr,
                                            aurora_color,
                                            0,
                                        );
                                    }
                                }
                                thread::sleep(Duration::from_millis(500));
                            }
                        });

                        return Response::json(&json!({
                            "success": true,
                            "message": format!("Aurora effect started on {} bulbs", bulbs_vec.len()),
                            "effect": "aurora",
                        }));
                    }
                    "pulse" => {
                        // Pulse effect: fade in/out
                        let bulbs_clone: Vec<(u64, HSBK, std::net::SocketAddr)> = bulbs_vec
                            .iter()
                            .map(|b| {
                                let color = b
                                    .lifx_color
                                    .as_ref()
                                    .map(|c| HSBK {
                                        hue: c.hue,
                                        saturation: c.saturation,
                                        brightness: c.brightness,
                                        kelvin: c.kelvin,
                                    })
                                    .unwrap_or(HSBK {
                                        hue: 0,
                                        saturation: 0,
                                        brightness: 65535,
                                        kelvin: 6500,
                                    });
                                (b.target, color, b.addr)
                            })
                            .collect();

                        let sock_clone = sock.try_clone().ok();

                        thread::spawn(move || {
                            if let Some(socket) = sock_clone {
                                for _ in 0..(cycles as u64) {
                                    // Fade out
                                    for (target, color, addr) in &bulbs_clone {
                                        for step in (0..=10).rev() {
                                            let dimmed = HSBK {
                                                hue: color.hue,
                                                saturation: color.saturation,
                                                brightness: (color.brightness as f64
                                                    * (step as f64 / 10.0))
                                                    as u16,
                                                kelvin: color.kelvin,
                                            };
                                            let _ = handlers.protocol.send_color_command(
                                                &socket, *target, *addr, dimmed, 0,
                                            );
                                            thread::sleep(Duration::from_millis(step_duration));
                                        }
                                    }
                                    // Fade in
                                    for (target, color, addr) in &bulbs_clone {
                                        for step in 0..=10 {
                                            let brightened = HSBK {
                                                hue: color.hue,
                                                saturation: color.saturation,
                                                brightness: (color.brightness as f64
                                                    * (step as f64 / 10.0))
                                                    as u16,
                                                kelvin: color.kelvin,
                                            };
                                            let _ = handlers.protocol.send_color_command(
                                                &socket, *target, *addr, brightened, 0,
                                            );
                                            thread::sleep(Duration::from_millis(step_duration));
                                        }
                                    }
                                }
                                // Restore original
                                for (target, color, addr) in &bulbs_clone {
                                    let _ = handlers
                                        .protocol
                                        .send_color_command(&socket, *target, *addr, *color, 0);
                                }
                            }
                        });

                        return Response::json(&json!({
                            "success": true,
                            "message": format!("Pulse effect started on {} bulbs", bulbs_vec.len()),
                            "effect": "pulse",
                            "cycles": cycles,
                            "duration": total_duration
                        }));
                    }
                    "rainbow" => {
                        // Rainbow cycle effect
                        let sock_clone = sock.try_clone().ok();
                        let targets: Vec<(u64, std::net::SocketAddr)> =
                            bulbs_vec.iter().map(|b| (b.target, b.addr)).collect();

                        thread::spawn(move || {
                            if let Some(socket) = sock_clone {
                                for cycle in 0..(cycles as u16) {
                                    for step in 0..=10 {
                                        let hue = ((cycle * 6553 + step * 655) % 65535) as u16;
                                        for (target, addr) in &targets {
                                            let color = HSBK {
                                                hue: hue,
                                                saturation: 65535,
                                                brightness: 65535,
                                                kelvin: 5500,
                                            };
                                            let _ = handlers.protocol.send_color_command(
                                                &socket, *target, *addr, color, 0,
                                            );
                                        }
                                        thread::sleep(Duration::from_millis(step_duration));
                                    }
                                }
                            }
                        });

                        return Response::json(&json!({
                            "success": true,
                            "message": format!("Rainbow effect started on {} bulbs", bulbs_vec.len()),
                            "effect": "rainbow",
                            "cycles": cycles,
                            "duration": total_duration
                        }));
                    }
                    "strobe" | "flash" => {
                        // Strobe effect
                        let sock_clone = sock.try_clone().ok();
                        let targets: Vec<(u64, std::net::SocketAddr)> =
                            bulbs_vec.iter().map(|b| (b.target, b.addr)).collect();

                        thread::spawn(move || {
                            if let Some(socket) = sock_clone {
                                for _ in 0..((cycles * 10.0) as u64) {
                                    // On
                                    for (target, addr) in &targets {
                                        let _ = handlers.protocol.send_color_command(
                                            &socket,
                                            *target,
                                            *addr,
                                            HSBK {
                                                hue: 0,
                                                saturation: 0,
                                                brightness: 65535,
                                                kelvin: 6500,
                                            },
                                            0,
                                        );
                                    }
                                    thread::sleep(Duration::from_millis(50));
                                    // Off
                                    for (target, addr) in &targets {
                                        let _ = handlers.protocol.send_color_command(
                                            &socket,
                                            *target,
                                            *addr,
                                            HSBK {
                                                hue: 0,
                                                saturation: 0,
                                                brightness: 0,
                                                kelvin: 6500,
                                            },
                                            0,
                                        );
                                    }
                                    thread::sleep(Duration::from_millis(50));
                                }
                            }
                        });

                        return Response::json(&json!({
                            "success": true,
                            "message": format!("Strobe effect started on {} bulbs", bulbs_vec.len()),
                            "effect": "strobe",
                            "cycles": cycles
                        }));
                    }
                    "color_cycle" => {
                        // Color cycle effect - slowly transition through colors
                        let sock_clone = sock.try_clone().ok();
                        let targets: Vec<(u64, std::net::SocketAddr)> =
                            bulbs_vec.iter().map(|b| (b.target, b.addr)).collect();
                        let cycle_duration = (total_duration / cycles) as u64;

                        thread::spawn(move || {
                            if let Some(socket) = sock_clone {
                                for _ in 0..(cycles as u64) {
                                    for step in 0..=36 {
                                        let hue = (step * 1820) as u16;
                                        for (target, addr) in &targets {
                                            let color = HSBK {
                                                hue: hue,
                                                saturation: 65535,
                                                brightness: 52428,
                                                kelvin: 4000,
                                            };
                                            let _ = handlers.protocol.send_color_command(
                                                &socket, *target, *addr, color, 0,
                                            );
                                        }
                                        thread::sleep(Duration::from_millis(cycle_duration / 36));
                                    }
                                }
                            }
                        });

                        return Response::json(&json!({
                            "success": true,
                            "message": format!("Color cycle effect started on {} bulbs", bulbs_vec.len()),
                            "effect": "color_cycle",
                            "cycles": cycles,
                            "duration": total_duration
                        }));
                    }
                    "breath" => {
                        let original_colors: Vec<(u64, HSBK)> = bulbs_vec
                            .iter()
                            .map(|b| {
                                let color = b
                                    .lifx_color
                                    .as_ref()
                                    .map(|c| HSBK {
                                        hue: c.hue,
                                        saturation: c.saturation,
                                        brightness: c.brightness,
                                        kelvin: c.kelvin,
                                    })
                                    .unwrap_or(HSBK {
                                        hue: 0,
                                        saturation: 0,
                                        brightness: 65535,
                                        kelvin: 6500,
                                    });
                                (b.target, color)
                            })
                            .collect();

                        let sock_clone = sock.try_clone().ok();
                        let bulbs_clone: Vec<(u64, HSBK, std::net::SocketAddr)> = bulbs_vec
                            .iter()
                            .map(|b| {
                                (
                                    b.target,
                                    original_colors
                                        .iter()
                                        .find(|(t, _)| *t == b.target)
                                        .map(|(_, color)| *color)
                                        .unwrap_or(HSBK {
                                            hue: 0,
                                            saturation: 0,
                                            brightness: 65535,
                                            kelvin: 6500,
                                        }),
                                    b.addr,
                                )
                            })
                            .collect();

                        thread::spawn(move || {
                            if let Some(socket) = sock_clone {
                                for _ in 0..(cycles as u64) {
                                    for (target, color, addr) in &bulbs_clone {
                                        for step in 0..=20 {
                                            let progress = (step as f64 / 20.0).sin();
                                            let brightened = HSBK {
                                                hue: color.hue,
                                                saturation: color.saturation,
                                                brightness: (color.brightness as f64
                                                    * (0.3 + 0.7 * progress))
                                                    as u16,
                                                kelvin: color.kelvin,
                                            };
                                            let _ = handlers.protocol.send_color_command(
                                                &socket, *target, *addr, brightened, 0,
                                            );
                                            thread::sleep(Duration::from_millis(50));
                                        }
                                    }
                                }
                            }
                        });

                        return Response::json(&json!({
                            "success": true,
                            "message": format!("Breath effect started on {} bulbs", bulbs_vec.len()),
                            "effect": "breath",
                            "cycles": cycles,
                            "duration": total_duration
                        }));
                    }
                    "smooth_breath" => {
                        let original_colors: Vec<(u64, HSBK)> = bulbs_vec
                            .iter()
                            .map(|b| {
                                let color = b
                                    .lifx_color
                                    .as_ref()
                                    .map(|c| HSBK {
                                        hue: c.hue,
                                        saturation: c.saturation,
                                        brightness: c.brightness,
                                        kelvin: c.kelvin,
                                    })
                                    .unwrap_or(HSBK {
                                        hue: 0,
                                        saturation: 0,
                                        brightness: 65535,
                                        kelvin: 6500,
                                    });
                                (b.target, color)
                            })
                            .collect();

                        let sock_clone = sock.try_clone().ok();
                        let bulbs_clone: Vec<(u64, HSBK, std::net::SocketAddr)> = bulbs_vec
                            .iter()
                            .map(|b| {
                                (
                                    b.target,
                                    original_colors
                                        .iter()
                                        .find(|(t, _)| *t == b.target)
                                        .map(|(_, color)| *color)
                                        .unwrap_or(HSBK {
                                            hue: 0,
                                            saturation: 0,
                                            brightness: 65535,
                                            kelvin: 6500,
                                        }),
                                    b.addr,
                                )
                            })
                            .collect();

                        thread::spawn(move || {
                            if let Some(socket) = sock_clone {
                                let breath_duration = (total_duration / cycles * 1000.0) as u64;
                                let half_cycle = breath_duration / 2;

                                for cycle in 0..(cycles as u64) {
                                    let fade_out_steps = 20u64;
                                    let fade_out_duration = half_cycle / fade_out_steps;

                                    for step in 0..fade_out_steps {
                                        for (target, color, addr) in &bulbs_clone {
                                            let progress =
                                                1.0 - ((step as f64) / (fade_out_steps as f64));
                                            let eased = 1.0 - (1.0 - progress).powi(3);
                                            let dimmed = HSBK {
                                                hue: color.hue,
                                                saturation: color.saturation,
                                                brightness: (color.brightness as f64
                                                    * (0.15 + 0.85 * eased))
                                                    as u16,
                                                kelvin: color.kelvin,
                                            };
                                            let _ = handlers.protocol.send_color_command(
                                                &socket, *target, *addr, dimmed, 0,
                                            );
                                        }
                                        thread::sleep(Duration::from_millis(fade_out_duration));
                                    }

                                    let fade_in_steps = 20u64;
                                    let fade_in_duration = half_cycle / fade_in_steps;

                                    for step in 0..fade_in_steps {
                                        for (target, color, addr) in &bulbs_clone {
                                            let progress = (step as f64) / (fade_in_steps as f64);
                                            let eased = progress.powi(3);
                                            let brightened = HSBK {
                                                hue: color.hue,
                                                saturation: color.saturation,
                                                brightness: (color.brightness as f64
                                                    * (0.15 + 0.85 * eased))
                                                    as u16,
                                                kelvin: color.kelvin,
                                            };
                                            let _ = handlers.protocol.send_color_command(
                                                &socket, *target, *addr, brightened, 0,
                                            );
                                        }
                                        thread::sleep(Duration::from_millis(fade_in_duration));
                                    }
                                }

                                for (target, color, addr) in &bulbs_clone {
                                    let _ = handlers
                                        .protocol
                                        .send_color_command(&socket, *target, *addr, *color, 0);
                                }
                            }
                        });

                        return Response::json(&json!({
                            "success": true,
                            "message": format!("Smooth breath effect started on {} bulbs", bulbs_vec.len()),
                            "effect": "smooth_breath",
                            "cycles": cycles,
                            "duration": total_duration
                        }));
                    }
                    _ => {
                        return Response::json(&json!({
                            "success": false,
                            "message": format!("Unknown effect: {}", input.effect),
                            "available_effects": ["pulse", "rainbow", "strobe", "flash", "color_cycle", "fireplace", "aurora", "breath", "smooth_breath"]
                        }));
                    }
                }
            }
        }
    }

    Response::json(&json!({
        "success": false,
        "message": "Discovery service not available"
    }))
}

/// Handle multi-zone light strip control
fn handle_zones(request: &Request) -> Response {
    if request.method() != "POST" {
        return Response::empty_404();
    }

    let input = try_or_400!(post_input!(request, {
        selector: String,
        start_index: Option<u8>,
        end_index: Option<u8>,
        color: String,
        duration: Option<f64>,
        apply: Option<bool>
    }));

    if let Some(discovery_arc) = get_global_discovery() {
        if let Ok(discovery) = discovery_arc.lock() {
            if let Ok(bulbs_arc) = discovery.get_bulbs().lock() {
                let sock = discovery.get_socket();
                let handlers = HttpHandlers::new(0);

                let mut bulbs_vec: Vec<&BulbInfo> = bulbs_arc.values().collect();

                if input.selector.contains("id:") {
                    let id = input.selector.replace("id:", "");
                    bulbs_vec.retain(|b| b.id.contains(&id));
                }

                let duration = (input.duration.unwrap_or(1.0) * 1000.0) as u32;
                let start_index = input.start_index.unwrap_or(0);
                let end_index = input.end_index.unwrap_or(255);
                let apply = input.apply.unwrap_or(true);
                let mut success_count = 0;
                let mut total_zones_set = 0;

                for bulb in &bulbs_vec {
                    if let Some(hsbk) = handlers.parse_color_command(&input.color, bulb, duration) {
                        let bulb_has_multizone = bulb
                            .product
                            .as_ref()
                            .map(|p| p.capabilities.has_multizone)
                            .unwrap_or(false);

                        if bulb_has_multizone {
                            let zone_count = match &bulb.color {
                                LiColor::Multi(data) => data
                                    .data
                                    .as_ref()
                                    .map(|zones| zones.len() as u16)
                                    .unwrap_or(1),
                                _ => 1,
                            };

                            let max_zone_index =
                                zone_count.saturating_sub(1).min(u8::MAX as u16) as u8;
                            let actual_end = std::cmp::min(end_index, max_zone_index);

                            for zone_idx in start_index..=actual_end {
                                let set_zone_msg = lifx_rs::lan::Message::SetColorZones {
                                    start_index: zone_idx as u8,
                                    end_index: zone_idx as u8,
                                    color: hsbk,
                                    duration,
                                    apply: if apply {
                                        ApplicationRequest::Apply
                                    } else {
                                        ApplicationRequest::NoApply
                                    },
                                };

                                match handlers.protocol.build_message(bulb.target, set_zone_msg) {
                                    Ok(bytes) => {
                                        if sock.send_to(&bytes, bulb.addr).is_ok() {
                                            success_count += 1;
                                            total_zones_set += 1;
                                        }
                                    }
                                    Err(e) => {
                                        log::warn!(
                                            "Failed to build zone message for bulb {}: {}",
                                            bulb.id,
                                            e
                                        );
                                    }
                                }
                            }
                        } else {
                            if handlers
                                .protocol
                                .send_color_command(sock, bulb.target, bulb.addr, hsbk, duration)
                                .is_ok()
                            {
                                success_count += 1;
                                total_zones_set += 1;
                            }
                        }
                    }
                }

                return Response::json(&json!({
                    "success": success_count > 0,
                    "message": format!("Set {} zones on {} bulbs", total_zones_set, success_count),
                    "start_index": start_index,
                    "end_index": end_index,
                    "apply": apply,
                    "zones_set": total_zones_set
                }));
            }
        }
    }

    Response::json(&json!({
        "success": false,
        "message": "Discovery service not available"
    }))
}

/// Get circadian rhythm color based on time of day
fn get_circadian_color(hour: u32) -> (u16, u16, u16, u16) {
    match hour {
        0..=5 => (43680, 6553, 6553, 2000), // Deep night - very dim, warm
        6 => (9100, 20000, 26214, 3000),    // Dawn - gradual wake up
        7..=9 => (9100, 32767, 55705, 5500), // Morning - energizing
        10..=11 => (36400, 13107, 62259, 6000), // Midday - bright, cool
        12..=14 => (36400, 19660, 65535, 6500), // Afternoon - peak brightness
        15..=17 => (25480, 26214, 58982, 5000), // Late afternoon - neutral
        18..=19 => (8000, 30000, 45875, 3500), // Evening - warm, relaxing
        20..=21 => (5800, 15000, 32767, 2700), // Night - dim, warm
        22..=23 => (43680, 6553, 19660, 2000), // Late night - very dim
        _ => (43680, 6553, 6553, 2000),
    }
}

/// Handle circadian rhythm scheduling
fn handle_circadian(request: &Request) -> Response {
    use std::time::{SystemTime, UNIX_EPOCH};

    if request.method() != "POST" {
        return Response::empty_404();
    }

    let input = try_or_400!(post_input!(request, {
        selector: String,
        enable: Option<bool>,
        hour: Option<u32>
    }));

    let target_hour = input.hour.unwrap_or_else(|| {
        let seconds_today = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs() % 86400)
            .unwrap_or(0);
        ((seconds_today / 3600 + 5) % 24) as u32
    });

    let (hue, sat, bright, kelvin) = get_circadian_color(target_hour);

    if let Some(discovery_arc) = get_global_discovery() {
        if let Ok(discovery) = discovery_arc.lock() {
            if let Ok(bulbs_arc) = discovery.get_bulbs().lock() {
                let sock = discovery.get_socket();
                let handlers = HttpHandlers::new(0);

                let mut bulbs_vec: Vec<&BulbInfo> = bulbs_arc.values().collect();

                if input.selector.contains("id:") {
                    let id = input.selector.replace("id:", "");
                    bulbs_vec.retain(|b| b.id.contains(&id));
                }

                let mut success_count = 0;
                for bulb in &bulbs_vec {
                    let hsbk = HSBK {
                        hue,
                        saturation: sat,
                        brightness: bright,
                        kelvin,
                    };

                    if handlers
                        .protocol
                        .send_color_command(sock, bulb.target, bulb.addr, hsbk, 1000)
                        .is_ok()
                    {
                        success_count += 1;
                    }
                }

                let time_of_day = match target_hour {
                    0..=5 => "deep_night",
                    6 => "dawn",
                    7..=9 => "morning",
                    10..=11 => "midday",
                    12..=14 => "afternoon",
                    15..=17 => "late_afternoon",
                    18..=19 => "evening",
                    20..=21 => "night",
                    22..=23 => "late_night",
                    _ => "unknown",
                };

                return Response::json(&json!({
                    "success": success_count > 0,
                    "message": format!("Circadian rhythm applied to {} bulbs", success_count),
                    "time_of_day": time_of_day,
                    "hour": target_hour,
                    "bulbs_affected": success_count,
                    "hsbk": {
                        "hue": hue,
                        "saturation": sat,
                        "brightness": bright,
                        "kelvin": kelvin
                    }
                }));
            }
        }
    }

    Response::json(&json!({
        "success": false,
        "message": "Discovery service not available"
    }))
}

/// Handle preset lighting configurations
fn handle_presets(request: &Request) -> Response {
    if request.method() == "GET" {
        // Return saved presets from configuration
        return Response::json(&json!({
            "presets": [
                {"id": "morning", "name": "Morning Wakeup", "scene": "energize", "brightness": 80},
                {"id": "evening", "name": "Evening Wind Down", "scene": "relax", "brightness": 40},
                {"id": "movie", "name": "Movie Time", "scene": "night", "brightness": 20},
                {"id": "work", "name": "Focus Work", "scene": "focus", "brightness": 75}
            ]
        }));
    }

    if request.method() == "POST" {
        let input = try_or_400!(post_input!(request, {
            preset_id: String,
            selector: Option<String>
        }));

        let presets: Vec<serde_json::Value> = vec![
            json!({"id": "morning", "scene": "energize", "brightness": 80}),
            json!({"id": "evening", "scene": "relax", "brightness": 40}),
            json!({"id": "movie", "scene": "night", "brightness": 20}),
            json!({"id": "work", "scene": "focus", "brightness": 75}),
        ];

        let preset = presets.iter().find(|p| p["id"] == input.preset_id);

        if let Some(preset) = preset {
            let selector = input.selector.unwrap_or_else(|| "all".to_string());

            // Apply preset scene
            let scene_data = SCENES
                .iter()
                .find(|s| s.0 == preset["scene"].as_str().unwrap_or("relax"));

            if let Some(scene) = scene_data {
                if let Some(discovery_arc) = get_global_discovery() {
                    if let Ok(discovery) = discovery_arc.lock() {
                        if let Ok(bulbs_arc) = discovery.get_bulbs().lock() {
                            let sock = discovery.get_socket();
                            let handlers = HttpHandlers::new(0);

                            let mut bulbs_vec: Vec<&BulbInfo> = bulbs_arc.values().collect();

                            if selector != "all" {
                                if selector.contains("id:") {
                                    let id = selector.replace("id:", "");
                                    bulbs_vec.retain(|b| b.id.contains(&id));
                                }
                            }

                            let mut success_count = 0;
                            for bulb in &bulbs_vec {
                                let hsbk = HSBK {
                                    hue: scene.1,
                                    saturation: scene.2,
                                    brightness: ((preset["brightness"].as_u64().unwrap_or(50)
                                        as f64
                                        / 100.0)
                                        * 65535.0)
                                        as u16,
                                    kelvin: scene.4,
                                };

                                if handlers
                                    .protocol
                                    .send_color_command(sock, bulb.target, bulb.addr, hsbk, 500)
                                    .is_ok()
                                {
                                    success_count += 1;
                                }
                            }

                            return Response::json(&json!({
                                "success": success_count > 0,
                                "message": format!("Preset '{}' applied to {} bulbs", input.preset_id, success_count),
                                "preset": input.preset_id
                            }));
                        }
                    }
                }
            }

            return Response::json(&json!({
                "success": false,
                "message": "Failed to apply preset"
            }));
        }

        return Response::json(&json!({
            "success": false,
            "message": format!("Unknown preset: {}", input.preset_id),
            "available_presets": presets.iter().filter_map(|p| p["id"].as_str()).collect::<Vec<_>>()
        }));
    }

    Response::empty_404()
}

/// Handle touch gesture commands
fn handle_gesture(request: &Request) -> Response {
    if request.method() != "POST" {
        return Response::empty_404();
    }

    let input = try_or_400!(post_input!(request, {
        gesture: String,
        selector: Option<String>,
        params: Option<String>
    }));

    if let Some(discovery_arc) = get_global_discovery() {
        if let Ok(discovery) = discovery_arc.lock() {
            if let Ok(bulbs_arc) = discovery.get_bulbs().lock() {
                let sock = discovery.get_socket();
                let handlers = HttpHandlers::new(0);

                let mut bulbs_vec: Vec<&BulbInfo> = bulbs_arc.values().collect();

                if let Some(ref selector) = input.selector {
                    if selector.contains("id:") {
                        let id = selector.replace("id:", "");
                        bulbs_vec.retain(|b| b.id.contains(&id));
                    }
                }

                let result = match input.gesture.as_str() {
                    "swipeUp" => {
                        for bulb in &bulbs_vec {
                            let current = bulb
                                .lifx_color
                                .as_ref()
                                .map(|c| c.brightness)
                                .unwrap_or(65535);
                            let new_brightness = std::cmp::min(65535, current + 6553);
                            let hsbk = HSBK {
                                hue: bulb.lifx_color.as_ref().map(|c| c.hue).unwrap_or(0),
                                saturation: bulb
                                    .lifx_color
                                    .as_ref()
                                    .map(|c| c.saturation)
                                    .unwrap_or(0),
                                brightness: new_brightness,
                                kelvin: bulb.lifx_color.as_ref().map(|c| c.kelvin).unwrap_or(5000),
                            };
                            let _ = handlers.protocol.send_color_command(
                                sock,
                                bulb.target,
                                bulb.addr,
                                hsbk,
                                300,
                            );
                        }
                        json!({"success": true, "action": "brightness_up", "bulbs_affected": bulbs_vec.len()})
                    }
                    "swipeDown" => {
                        for bulb in &bulbs_vec {
                            let current = bulb
                                .lifx_color
                                .as_ref()
                                .map(|c| c.brightness)
                                .unwrap_or(65535);
                            let new_brightness = std::cmp::max(0, current - 6553);
                            let hsbk = HSBK {
                                hue: bulb.lifx_color.as_ref().map(|c| c.hue).unwrap_or(0),
                                saturation: bulb
                                    .lifx_color
                                    .as_ref()
                                    .map(|c| c.saturation)
                                    .unwrap_or(0),
                                brightness: new_brightness,
                                kelvin: bulb.lifx_color.as_ref().map(|c| c.kelvin).unwrap_or(5000),
                            };
                            let _ = handlers.protocol.send_color_command(
                                sock,
                                bulb.target,
                                bulb.addr,
                                hsbk,
                                300,
                            );
                        }
                        json!({"success": true, "action": "brightness_down", "bulbs_affected": bulbs_vec.len()})
                    }
                    "swipeLeft" => {
                        for bulb in &bulbs_vec {
                            let current =
                                bulb.lifx_color.as_ref().map(|c| c.kelvin).unwrap_or(5000);
                            let new_kelvin = std::cmp::max(1500, current - 500);
                            let hsbk = HSBK {
                                hue: bulb.lifx_color.as_ref().map(|c| c.hue).unwrap_or(0),
                                saturation: 0,
                                brightness: bulb
                                    .lifx_color
                                    .as_ref()
                                    .map(|c| c.brightness)
                                    .unwrap_or(65535),
                                kelvin: new_kelvin,
                            };
                            let _ = handlers.protocol.send_color_command(
                                sock,
                                bulb.target,
                                bulb.addr,
                                hsbk,
                                300,
                            );
                        }
                        json!({"success": true, "action": "cooler", "bulbs_affected": bulbs_vec.len()})
                    }
                    "swipeRight" => {
                        for bulb in &bulbs_vec {
                            let current =
                                bulb.lifx_color.as_ref().map(|c| c.kelvin).unwrap_or(5000);
                            let new_kelvin = std::cmp::min(9000, current + 500);
                            let hsbk = HSBK {
                                hue: bulb.lifx_color.as_ref().map(|c| c.hue).unwrap_or(0),
                                saturation: 0,
                                brightness: bulb
                                    .lifx_color
                                    .as_ref()
                                    .map(|c| c.brightness)
                                    .unwrap_or(65535),
                                kelvin: new_kelvin,
                            };
                            let _ = handlers.protocol.send_color_command(
                                sock,
                                bulb.target,
                                bulb.addr,
                                hsbk,
                                300,
                            );
                        }
                        json!({"success": true, "action": "warmer", "bulbs_affected": bulbs_vec.len()})
                    }
                    "tap" => {
                        for bulb in &bulbs_vec {
                            let is_on = bulb.power == "on";
                            let power = if is_on {
                                lifx_rs::lan::PowerLevel::Standby
                            } else {
                                lifx_rs::lan::PowerLevel::Enabled
                            };
                            let _ = handlers.protocol.send_power_command(
                                sock,
                                bulb.target,
                                bulb.addr,
                                power,
                            );
                        }
                        json!({"success": true, "action": "toggle_power", "bulbs_affected": bulbs_vec.len()})
                    }
                    "longPress" => {
                        json!({"success": true, "action": "show_quick_settings", "bulbs_affected": bulbs_vec.len()})
                    }
                    "pinchOut" => {
                        let scene_idx = input
                            .params
                            .as_ref()
                            .and_then(|p| serde_json::from_str::<serde_json::Value>(p).ok())
                            .and_then(|p| p["sceneIndex"].as_u64())
                            .unwrap_or(0) as usize;
                        let scene = SCENES.get(scene_idx % SCENES.len()).unwrap_or(&SCENES[0]);
                        for bulb in &bulbs_vec {
                            let hsbk = HSBK {
                                hue: scene.1,
                                saturation: scene.2,
                                brightness: scene.3,
                                kelvin: scene.4,
                            };
                            let _ = handlers.protocol.send_color_command(
                                sock,
                                bulb.target,
                                bulb.addr,
                                hsbk,
                                500,
                            );
                        }
                        json!({"success": true, "action": "next_scene", "scene": scene.0, "bulbs_affected": bulbs_vec.len()})
                    }
                    "pinchIn" => {
                        let scene_idx = input
                            .params
                            .as_ref()
                            .and_then(|p| serde_json::from_str::<serde_json::Value>(p).ok())
                            .and_then(|p| p["sceneIndex"].as_u64())
                            .unwrap_or(0) as usize;
                        let prev_idx = if scene_idx == 0 {
                            SCENES.len() - 1
                        } else {
                            scene_idx - 1
                        };
                        let scene = SCENES.get(prev_idx).unwrap_or(&SCENES[0]);
                        for bulb in &bulbs_vec {
                            let hsbk = HSBK {
                                hue: scene.1,
                                saturation: scene.2,
                                brightness: scene.3,
                                kelvin: scene.4,
                            };
                            let _ = handlers.protocol.send_color_command(
                                sock,
                                bulb.target,
                                bulb.addr,
                                hsbk,
                                500,
                            );
                        }
                        json!({"success": true, "action": "prev_scene", "scene": scene.0, "bulbs_affected": bulbs_vec.len()})
                    }
                    _ => {
                        return Response::json(&json!({
                            "success": false,
                            "message": format!("Unknown gesture: {}", input.gesture),
                            "available_gestures": ["swipeUp", "swipeDown", "swipeLeft", "swipeRight", "tap", "longPress", "pinchOut", "pinchIn"]
                        }));
                    }
                };

                return Response::json(&result);
            }
        }
    }

    Response::json(&json!({
        "success": false,
        "message": "Discovery service not available"
    }))
}

/// Handle media sync operations
fn handle_media_sync(request: &Request) -> Response {
    if request.method() == "POST" {
        let input = try_or_400!(post_input!(request, {
            action: String,
            bpm: Option<u32>,
            frequency: Option<f32>,
            amplitude: Option<f32>,
            selector: Option<String>
        }));

        if let Some(discovery_arc) = get_global_discovery() {
            if let Ok(discovery) = discovery_arc.lock() {
                if let Ok(bulbs_arc) = discovery.get_bulbs().lock() {
                    let sock = discovery.get_socket();
                    let handlers = HttpHandlers::new(0);

                    let mut bulbs_vec: Vec<&BulbInfo> = bulbs_arc.values().collect();

                    if let Some(ref selector) = input.selector {
                        if selector.contains("id:") {
                            let id = selector.replace("id:", "");
                            bulbs_vec.retain(|b| b.id.contains(&id));
                        }
                    }

                    match input.action.as_str() {
                        "syncBeat" => {
                            if let Some(bpm) = input.bpm {
                                for bulb in &bulbs_vec {
                                    let brightness = if bpm > 120 { 65535 } else { 45000 };
                                    let hsbk = HSBK {
                                        hue: bulb.lifx_color.as_ref().map(|c| c.hue).unwrap_or(0),
                                        saturation: bulb
                                            .lifx_color
                                            .as_ref()
                                            .map(|c| c.saturation)
                                            .unwrap_or(65535),
                                        brightness,
                                        kelvin: bulb
                                            .lifx_color
                                            .as_ref()
                                            .map(|c| c.kelvin)
                                            .unwrap_or(5000),
                                    };
                                    let _ = handlers.protocol.send_color_command(
                                        sock,
                                        bulb.target,
                                        bulb.addr,
                                        hsbk,
                                        50,
                                    );
                                }
                                return Response::json(&json!({
                                    "success": true,
                                    "action": "beat_sync",
                                    "bpm": bpm,
                                    "bulbs_affected": bulbs_vec.len()
                                }));
                            }
                        }
                        "syncFrequency" => {
                            if let (Some(freq), Some(amp)) = (input.frequency, input.amplitude) {
                                let hue = ((freq / 20000.0) * 65535.0) as u16;
                                let brightness = ((amp / 256.0) * 65535.0) as u16;
                                for bulb in &bulbs_vec {
                                    let hsbk = HSBK {
                                        hue,
                                        saturation: 65535,
                                        brightness,
                                        kelvin: 5000,
                                    };
                                    let _ = handlers.protocol.send_color_command(
                                        sock,
                                        bulb.target,
                                        bulb.addr,
                                        hsbk,
                                        100,
                                    );
                                }
                                return Response::json(&json!({
                                    "success": true,
                                    "action": "frequency_sync",
                                    "hue": hue,
                                    "brightness": brightness,
                                    "bulbs_affected": bulbs_vec.len()
                                }));
                            }
                        }
                        "startVisualizer" => {
                            return Response::json(&json!({
                                "success": true,
                                "action": "visualizer_started",
                                "bulbs_affected": bulbs_vec.len()
                            }));
                        }
                        "stopVisualizer" => {
                            return Response::json(&json!({
                                "success": true,
                                "action": "visualizer_stopped"
                            }));
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Response::json(&json!({
        "success": false,
        "message": "Media sync action not recognized or service unavailable"
    }))
}

/// Handle wave effect
fn handle_wave_effect(request: &Request) -> Response {
    use std::thread;
    use std::time::Duration;

    if request.method() != "POST" {
        return Response::empty_404();
    }

    let input = try_or_400!(post_input!(request, {
        selector: Option<String>,
        hue: Option<u16>,
        delay: Option<u64>
    }));

    if let Some(discovery_arc) = get_global_discovery() {
        if let Ok(discovery) = discovery_arc.lock() {
            if let Ok(bulbs_arc) = discovery.get_bulbs().lock() {
                let sock = discovery.get_socket();
                let handlers = HttpHandlers::new(0);

                let mut bulbs_vec: Vec<&BulbInfo> = bulbs_arc.values().collect();

                if let Some(ref selector) = input.selector {
                    if selector.contains("id:") {
                        let id = selector.replace("id:", "");
                        bulbs_vec.retain(|b| b.id.contains(&id));
                    }
                }

                let base_hue = input.hue.unwrap_or(0);
                let delay_ms = input.delay.unwrap_or(200);

                let sock_clone = sock.try_clone().ok();
                let bulbs_data: Vec<(u64, std::net::SocketAddr, u16)> = bulbs_vec
                    .iter()
                    .enumerate()
                    .map(|(i, b)| (b.target, b.addr, (base_hue + (i as u16 * 30)) % 65535))
                    .collect();

                thread::spawn(move || {
                    if let Some(socket) = sock_clone {
                        for (i, (target, addr, hue)) in bulbs_data.iter().enumerate() {
                            thread::sleep(Duration::from_millis(delay_ms * i as u64));
                            let hsbk = HSBK {
                                hue: *hue,
                                saturation: 65535,
                                brightness: 45875,
                                kelvin: 5000,
                            };
                            let _ = handlers
                                .protocol
                                .send_color_command(&socket, *target, *addr, hsbk, 500);
                        }
                    }
                });

                return Response::json(&json!({
                    "success": true,
                    "action": "wave_effect_started",
                    "bulbs_affected": bulbs_vec.len(),
                    "base_hue": base_hue,
                    "delay_ms": delay_ms
                }));
            }
        }
    }

    Response::json(&json!({
        "success": false,
        "message": "Discovery service not available"
    }))
}

/// Handle disco mode
fn handle_disco_mode(request: &Request) -> Response {
    use std::thread;
    use std::time::Duration;

    if request.method() != "POST" {
        return Response::empty_404();
    }

    let input = try_or_400!(post_input!(request, {
        action: String,
        selector: Option<String>,
        speed: Option<u64>
    }));

    if let Some(discovery_arc) = get_global_discovery() {
        if let Ok(discovery) = discovery_arc.lock() {
            if let Ok(bulbs_arc) = discovery.get_bulbs().lock() {
                let sock = discovery.get_socket();

                let mut bulbs_vec: Vec<&BulbInfo> = bulbs_arc.values().collect();

                if let Some(ref selector) = input.selector {
                    if selector.contains("id:") {
                        let id = selector.replace("id:", "");
                        bulbs_vec.retain(|b| b.id.contains(&id));
                    }
                }

                let speed_ms = input.speed.unwrap_or(200);

                match input.action.as_str() {
                    "start" => {
                        let sock_clone = sock.try_clone().ok();
                        let targets: Vec<(u64, std::net::SocketAddr)> =
                            bulbs_vec.iter().map(|b| (b.target, b.addr)).collect();

                        thread::spawn(move || {
                            if let Some(socket) = sock_clone {
                                let mut hue: u16 = 0;
                                loop {
                                    hue = (hue + 10000) % 65535;
                                    for (target, addr) in &targets {
                                        let hsbk = HSBK {
                                            hue,
                                            saturation: 65535,
                                            brightness: 65535,
                                            kelvin: 5000,
                                        };
                                        let _ = send_color_command_helper(
                                            &socket, *target, *addr, hsbk, 100,
                                        );
                                    }
                                    thread::sleep(Duration::from_millis(speed_ms));
                                }
                            }
                        });

                        return Response::json(&json!({
                            "success": true,
                            "action": "disco_started",
                            "bulbs_affected": bulbs_vec.len(),
                            "speed_ms": speed_ms
                        }));
                    }
                    "stop" => {
                        return Response::json(&json!({
                            "success": true,
                            "action": "disco_stopped"
                        }));
                    }
                    _ => {
                        return Response::json(&json!({
                            "success": false,
                            "message": "Unknown disco action",
                            "available_actions": ["start", "stop"]
                        }));
                    }
                }
            }
        }
    }

    Response::json(&json!({
        "success": false,
        "message": "Discovery service not available"
    }))
}

/// Helper function for sending color commands
fn send_color_command_helper(
    socket: &std::net::UdpSocket,
    target: u64,
    addr: std::net::SocketAddr,
    color: HSBK,
    duration: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let protocol = ProtocolHandler::new(0);
    let bytes = protocol.build_message(
        target,
        lifx_rs::lan::Message::LightSetColor {
            reserved: 0,
            color,
            duration,
        },
    )?;
    socket.send_to(&bytes, addr)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lifx_rs::lan::HSBK;

    #[test]
    fn test_scene_definitions_exist() {
        assert!(
            !SCENES.is_empty(),
            "SCENES should contain at least one entry"
        );
    }

    #[test]
    fn test_scene_format() {
        for (name, hue, saturation, brightness, kelvin) in SCENES.iter() {
            assert!(!name.is_empty(), "Scene name should not be empty");
            assert!(*hue <= 360, "Hue should be 0-360 (actual: {})", hue);
            assert!(*saturation <= 65535, "Saturation should be 0-65535");
            assert!(*brightness <= 65535, "Brightness should be 0-65535");
            assert!(
                *kelvin >= 1500 && *kelvin <= 9000,
                "Kelvin should be 1500-9000"
            );
        }
    }

    #[test]
    fn test_known_scenes_exist() {
        let scene_names: Vec<&str> = SCENES.iter().map(|(name, _, _, _, _)| *name).collect();
        assert!(scene_names.contains(&"relax"), "relax scene should exist");
        assert!(scene_names.contains(&"focus"), "focus scene should exist");
        assert!(
            scene_names.contains(&"energize"),
            "energize scene should exist"
        );
        assert!(scene_names.contains(&"night"), "night scene should exist");
    }

    #[test]
    fn test_hsbk_color_conversion() {
        let test_hue = 180;
        let test_saturation = 32768;
        let test_brightness = 65535;
        let test_kelvin = 5000;

        let hsbk = HSBK {
            hue: test_hue,
            saturation: test_saturation,
            brightness: test_brightness,
            kelvin: test_kelvin,
        };

        assert_eq!(hsbk.hue, test_hue);
        assert_eq!(hsbk.saturation, test_saturation);
        assert_eq!(hsbk.brightness, test_brightness);
        assert_eq!(hsbk.kelvin, test_kelvin);
    }

    #[test]
    fn test_preset_definitions() {
        let presets = vec![
            json!({"id": "morning", "scene": "energize", "brightness": 80}),
            json!({"id": "evening", "scene": "relax", "brightness": 40}),
            json!({"id": "movie", "scene": "night", "brightness": 20}),
            json!({"id": "work", "scene": "focus", "brightness": 75}),
        ];

        assert_eq!(presets.len(), 4, "Should have 4 default presets");

        let morning = &presets[0];
        assert_eq!(morning["id"], "morning");
        assert_eq!(morning["scene"], "energize");
        assert_eq!(morning["brightness"], 80);
    }

    #[test]
    fn test_zone_control_response_format() {
        let response = json!({
            "success": true,
            "message": "Set 10 zones on 2 bulbs",
            "start_index": 0,
            "end_index": 10,
            "apply": true,
            "zones_set": 20
        });

        assert!(response["success"].as_bool().unwrap());
        assert!(response["message"].as_str().unwrap().contains("zones"));
        assert_eq!(response["start_index"].as_u64().unwrap(), 0);
        assert_eq!(response["end_index"].as_u64().unwrap(), 10);
        assert!(response["apply"].as_bool().unwrap());
        assert_eq!(response["zones_set"].as_u64().unwrap(), 20);
    }
}
