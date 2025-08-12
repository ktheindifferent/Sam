use async_trait::async_trait;
use lifx_rs::lan::{PowerLevel, HSBK};
use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub id: String,
    pub uuid: String,
    pub label: String,
    pub connected: bool,
    pub power: PowerLevel,
    pub color: Option<HSBK>,
    pub brightness: f64,
    pub group: Option<GroupInfo>,
    pub location: Option<LocationInfo>,
    pub product_info: Option<lifx_rs::lan::ProductInfo>,
}

#[derive(Debug, Clone)]
pub struct GroupInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct LocationInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ColorCommand {
    pub color: Option<HSBK>,
    pub duration: u32,
    pub power: Option<PowerLevel>,
    pub brightness: Option<f64>,
    pub infrared: Option<u16>,
}

#[async_trait]
pub trait LightControl: Send + Sync {
    async fn set_power(&self, device_id: &str, power: PowerLevel) -> Result<(), Box<dyn std::error::Error>>;
    async fn set_color(&self, device_id: &str, command: ColorCommand) -> Result<(), Box<dyn std::error::Error>>;
    async fn set_infrared(&self, device_id: &str, brightness: u16) -> Result<(), Box<dyn std::error::Error>>;
    async fn get_device_info(&self, device_id: &str) -> Result<DeviceInfo, Box<dyn std::error::Error>>;
    async fn list_devices(&self) -> Result<Vec<DeviceInfo>, Box<dyn std::error::Error>>;
    async fn discover_devices(&self) -> Result<(), Box<dyn std::error::Error>>;
}

pub trait LightDevice: Send + Sync {
    fn get_id(&self) -> String;
    fn get_target(&self) -> u64;
    fn get_address(&self) -> SocketAddr;
    fn update_address(&mut self, addr: SocketAddr);
    fn needs_refresh(&self) -> bool;
    fn to_device_info(&self) -> DeviceInfo;
}