use lifx_rs::lan::{BuildOptions, Message, PowerLevel, RawMessage, HSBK};
use std::net::{SocketAddr, UdpSocket};

pub struct ProtocolHandler {
    source: u32,
}

impl ProtocolHandler {
    pub fn new(source: u32) -> Self {
        Self { source }
    }

    pub fn build_message(&self, target: u64, message: Message) -> Result<Vec<u8>, lifx_rs::lan::Error> {
        let options = BuildOptions {
            target: Some(target),
            res_required: true,
            source: self.source,
            ..Default::default()
        };
        let raw_message = RawMessage::build(&options, message)?;
        raw_message.pack()
    }

    pub fn build_discovery_message(&self) -> Result<Vec<u8>, lifx_rs::lan::Error> {
        let options = BuildOptions {
            source: self.source,
            ..Default::default()
        };
        let raw_message = RawMessage::build(&options, Message::GetService)?;
        raw_message.pack()
    }

    pub fn send_power_command(
        &self,
        sock: &UdpSocket,
        target: u64,
        addr: SocketAddr,
        power: PowerLevel,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let message = self.build_message(target, Message::SetPower { level: power })?;
        sock.send_to(&message, addr)?;
        Ok(())
    }

    pub fn send_color_command(
        &self,
        sock: &UdpSocket,
        target: u64,
        addr: SocketAddr,
        color: HSBK,
        duration: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let message = self.build_message(
            target,
            Message::LightSetColor {
                reserved: 0,
                color,
                duration,
            },
        )?;
        sock.send_to(&message, addr)?;
        Ok(())
    }

    pub fn send_infrared_command(
        &self,
        sock: &UdpSocket,
        target: u64,
        addr: SocketAddr,
        brightness: u16,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let message = self.build_message(target, Message::LightSetInfrared { brightness })?;
        sock.send_to(&message, addr)?;
        Ok(())
    }

    pub fn send_refresh_message(
        &self,
        sock: &UdpSocket,
        target: u64,
        addr: SocketAddr,
        refresh_msg: Message,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let message = self.build_message(target, refresh_msg)?;
        sock.send_to(&message, addr)?;
        Ok(())
    }
}