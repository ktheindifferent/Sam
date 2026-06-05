use super::traits::{DeviceInfo, GroupInfo, LightDevice, LocationInfo};
use lifx_rs::lan::{Message, PowerLevel, ProductInfo, HSBK};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::Serialize;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

const HOUR: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, Clone)]
pub struct RefreshableData<T> {
    pub data: Option<T>,
    pub max_age: Duration,
    pub last_updated: Instant,
    pub refresh_msg: Message,
}

impl<T> RefreshableData<T> {
    pub fn empty(max_age: Duration, refresh_msg: Message) -> RefreshableData<T> {
        RefreshableData {
            data: None,
            max_age,
            last_updated: Instant::now(),
            refresh_msg,
        }
    }

    pub fn update(&mut self, data: T) {
        self.data = Some(data);
        self.last_updated = Instant::now();
    }

    pub fn needs_refresh(&self) -> bool {
        self.data.is_none() || self.last_updated.elapsed() > self.max_age
    }
}

#[derive(Debug, Clone)]
pub enum LiColor {
    Unknown,
    Single(RefreshableData<HSBK>),
    Multi(RefreshableData<Vec<Option<HSBK>>>),
}

#[derive(Debug, Clone, Serialize)]
pub struct LifxColor {
    pub hue: u16,
    pub saturation: u16,
    pub kelvin: u16,
    pub brightness: u16,
}

#[derive(Debug, Clone, Serialize)]
pub struct LifxGroup {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LifxLocation {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BulbInfo {
    pub id: String,
    pub uuid: String,
    pub label: String,
    pub connected: bool,
    pub power: String,
    #[serde(rename = "color")]
    pub lifx_color: Option<LifxColor>,
    pub brightness: f64,
    #[serde(rename = "group")]
    pub lifx_group: Option<LifxGroup>,
    #[serde(rename = "location")]
    pub lifx_location: Option<LifxLocation>,
    pub product: Option<ProductInfo>,
    #[serde(rename = "last_seen")]
    pub lifx_last_seen: String,
    #[serde(rename = "seconds_since_seen")]
    pub seconds_since_seen: i64,
    #[serde(skip_serializing)]
    pub last_seen: Instant,
    pub source: u32,
    pub target: u64,
    pub addr: SocketAddr,
    #[serde(skip_serializing)]
    pub group: RefreshableData<LifxGroup>,
    #[serde(skip_serializing)]
    pub name: RefreshableData<String>,
    #[serde(skip_serializing)]
    pub model: RefreshableData<(u32, u32)>,
    #[serde(skip_serializing)]
    pub location: RefreshableData<String>,
    #[serde(skip_serializing)]
    pub host_firmware: RefreshableData<u32>,
    #[serde(skip_serializing)]
    pub wifi_firmware: RefreshableData<u32>,
    #[serde(skip_serializing)]
    pub power_level: RefreshableData<PowerLevel>,
    #[serde(skip_serializing)]
    pub color: LiColor,
}

impl BulbInfo {
    pub fn new(source: u32, target: u64, addr: SocketAddr) -> BulbInfo {
        let id: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(12)
            .map(char::from)
            .collect();
        let uuid: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect();

        BulbInfo {
            id: id.to_string(),
            uuid: uuid.to_string(),
            label: String::new(),
            connected: true,
            power: "off".to_string(),
            lifx_color: None,
            brightness: 0.0,
            lifx_group: None,
            lifx_location: None,
            product: None,
            lifx_last_seen: String::new(),
            seconds_since_seen: 0,
            last_seen: Instant::now(),
            source,
            target,
            addr,
            group: RefreshableData::empty(HOUR, Message::GetGroup),
            location: RefreshableData::empty(HOUR, Message::GetLocation),
            name: RefreshableData::empty(HOUR, Message::GetLabel),
            model: RefreshableData::empty(HOUR, Message::GetVersion),
            host_firmware: RefreshableData::empty(HOUR, Message::GetHostFirmware),
            wifi_firmware: RefreshableData::empty(HOUR, Message::GetWifiFirmware),
            power_level: RefreshableData::empty(Duration::from_millis(500), Message::GetPower),
            color: LiColor::Unknown,
        }
    }

    pub fn update(&mut self, addr: SocketAddr) {
        self.last_seen = Instant::now();
        self.addr = addr;
    }

    pub fn update_from_message(&mut self, message: Message) -> Result<(), lifx_rs::lan::Error> {
        match message {
            Message::StateLabel { label } => {
                self.name.update(label.0);
                self.label = self
                    .name
                    .data
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or_default();
            }
            Message::StateLocation {
                location,
                label,
                updated_at: _,
            } => {
                let lab = label.0;
                self.location.update(lab.clone());
                let location_info = LifxLocation {
                    id: format!("{:?}", location.0)
                        .replace(", ", "")
                        .replace("[", "")
                        .replace("]", ""),
                    name: lab,
                };
                self.lifx_location = Some(location_info);
            }
            Message::StateVersion {
                vendor, product, ..
            } => {
                self.model.update((vendor, product));
                if let Some(info) = lifx_rs::lan::get_product_info(vendor, product) {
                    self.product = Some(info.clone());
                    if info.capabilities.has_multizone {
                        self.color = LiColor::Multi(RefreshableData::empty(
                            Duration::from_secs(15),
                            Message::GetColorZones {
                                start_index: 0,
                                end_index: 255,
                            },
                        ))
                    } else {
                        self.color = LiColor::Single(RefreshableData::empty(
                            Duration::from_secs(15),
                            Message::LightGet,
                        ))
                    }
                }
            }
            Message::StatePower { level } => {
                self.power_level.update(level);
                self.power = if level == PowerLevel::Enabled {
                    "on".to_string()
                } else {
                    "off".to_string()
                };
            }
            Message::StateGroup {
                group,
                label,
                updated_at: _,
            } => {
                let group_info = LifxGroup {
                    id: format!("{:?}", group.0)
                        .replace(", ", "")
                        .replace("[", "")
                        .replace("]", ""),
                    name: label.to_string(),
                };
                self.group.update(group_info.clone());
                self.lifx_group = Some(group_info);
            }
            Message::StateHostFirmware { version, .. } => self.host_firmware.update(version),
            Message::StateWifiFirmware { version, .. } => self.wifi_firmware.update(version),
            Message::LightState {
                color,
                power,
                label,
                ..
            } => {
                if let LiColor::Single(ref mut d) = self.color {
                    d.update(color);
                    self.lifx_color = Some(LifxColor {
                        hue: color.hue,
                        saturation: color.saturation,
                        kelvin: color.kelvin,
                        brightness: color.brightness,
                    });
                    self.brightness = (color.brightness as f64) / 65535.0;
                    self.power_level.update(power);
                }
                self.name.update(label.0);
            }
            Message::StateZone {
                count,
                index,
                color,
            } => {
                if let LiColor::Multi(ref mut d) = self.color {
                    d.data.get_or_insert_with(|| {
                        let mut v = Vec::with_capacity(count as usize);
                        v.resize(count as usize, None);
                        v
                    })[index as usize] = Some(color);
                }
            }
            Message::StateMultiZone {
                count,
                index,
                color0,
                color1,
                color2,
                color3,
                color4,
                color5,
                color6,
                color7,
            } => {
                if let LiColor::Multi(ref mut d) = self.color {
                    let v = d.data.get_or_insert_with(|| {
                        let mut v = Vec::with_capacity(count as usize);
                        v.resize(count as usize, None);
                        v
                    });
                    let idx = index as usize;
                    if idx + 7 < v.len() {
                        v[idx] = Some(color0);
                        v[idx + 1] = Some(color1);
                        v[idx + 2] = Some(color2);
                        v[idx + 3] = Some(color3);
                        v[idx + 4] = Some(color4);
                        v[idx + 5] = Some(color5);
                        v[idx + 6] = Some(color6);
                        v[idx + 7] = Some(color7);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl LightDevice for BulbInfo {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_target(&self) -> u64 {
        self.target
    }

    fn get_address(&self) -> SocketAddr {
        self.addr
    }

    fn update_address(&mut self, addr: SocketAddr) {
        self.update(addr);
    }

    fn needs_refresh(&self) -> bool {
        self.name.needs_refresh()
            || self.model.needs_refresh()
            || self.power_level.needs_refresh()
            || match &self.color {
                LiColor::Unknown => false,
                LiColor::Single(d) => d.needs_refresh(),
                LiColor::Multi(d) => d.needs_refresh(),
            }
    }

    fn to_device_info(&self) -> DeviceInfo {
        let power_level = self.power_level.data.unwrap_or(PowerLevel::Standby);
        let color = match &self.color {
            LiColor::Single(d) => d.data,
            _ => None,
        };

        DeviceInfo {
            id: self.id.clone(),
            uuid: self.uuid.clone(),
            label: self.label.clone(),
            connected: self.connected,
            power: power_level,
            color,
            brightness: self.brightness,
            group: self.lifx_group.as_ref().map(|g| GroupInfo {
                id: g.id.clone(),
                name: g.name.clone(),
            }),
            location: self.lifx_location.as_ref().map(|l| LocationInfo {
                id: l.id.clone(),
                name: l.name.clone(),
            }),
            product_info: self.product.clone(),
        }
    }
}
