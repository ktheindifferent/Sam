use crate::services::lifx::bulb::BulbInfo;
use crate::services::lifx::protocol::ProtocolHandler;
use get_if_addrs::{get_if_addrs, IfAddr, Ifv4Addr};
use lifx_rs::lan::{Message, RawMessage};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

pub struct DiscoveryService {
    bulbs: Arc<Mutex<HashMap<u64, BulbInfo>>>,
    sock: UdpSocket,
    protocol: ProtocolHandler,
    last_discovery: Instant,
}

impl DiscoveryService {
    pub fn new(source: u32) -> Result<Self, Box<dyn std::error::Error>> {
        let sock = UdpSocket::bind("0.0.0.0:56700")?;
        sock.set_broadcast(true)?;

        let recv_sock = sock.try_clone()?;
        let bulbs = Arc::new(Mutex::new(HashMap::new()));
        let receiver_bulbs = bulbs.clone();

        thread::spawn(move || Self::discovery_worker(recv_sock, source, receiver_bulbs));

        let mut service = Self {
            bulbs,
            sock,
            protocol: ProtocolHandler::new(source),
            last_discovery: Instant::now(),
        };

        service.discover()?;
        Ok(service)
    }

    pub fn discover(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Starting device discovery");
        let bytes = self.protocol.build_discovery_message()?;

        for addr in get_if_addrs()? {
            if let IfAddr::V4(Ifv4Addr {
                broadcast: Some(bcast),
                ..
            }) = addr.addr
            {
                if addr.ip().is_loopback() {
                    continue;
                }
                let addr = SocketAddr::new(IpAddr::V4(bcast), 56700);
                log::info!("Discovering bulbs on LAN {:?}", addr);
                self.sock.send_to(&bytes, addr)?;
            }
        }

        self.last_discovery = Instant::now();
        Ok(())
    }

    pub fn get_bulbs(&self) -> Arc<Mutex<HashMap<u64, BulbInfo>>> {
        self.bulbs.clone()
    }

    pub fn get_socket(&self) -> &UdpSocket {
        &self.sock
    }

    pub fn get_protocol(&self) -> &ProtocolHandler {
        &self.protocol
    }

    fn discovery_worker(
        recv_sock: UdpSocket,
        source: u32,
        receiver_bulbs: Arc<Mutex<HashMap<u64, BulbInfo>>>,
    ) {
        let mut buf = [0; 1024];
        loop {
            match recv_sock.recv_from(&mut buf) {
                Ok((0, addr)) => log::info!("Received a zero-byte datagram from {:?}", addr),
                Ok((nbytes, addr)) => match RawMessage::unpack(&buf[0..nbytes]) {
                    Ok(raw) => {
                        if raw.frame_addr.target == 0 {
                            continue;
                        }
                        if let Ok(mut bulbs) = receiver_bulbs.lock() {
                            let bulb = bulbs
                                .entry(raw.frame_addr.target)
                                .and_modify(|bulb| bulb.update(addr))
                                .or_insert_with(|| BulbInfo::new(source, raw.frame_addr.target, addr));

                            if let Ok(message) = Message::from_raw(&raw) {
                                if let Err(e) = bulb.update_from_message(message) {
                                    log::info!("Error handling message from {}: {}", addr, e);
                                }
                            }
                        }
                    }
                    Err(e) => log::info!("Error unpacking raw message from {}: {}", addr, e),
                },
                Err(e) => {
                    log::error!("recv_from network error: {:?}", e);
                    match e.kind() {
                        std::io::ErrorKind::Interrupted | std::io::ErrorKind::WouldBlock => continue,
                        _ => {
                            log::error!("Fatal network error in LIFX discovery, breaking loop");
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn refresh_devices(&self) {
        if let Ok(bulbs) = self.bulbs.lock() {
            for bulb in bulbs.values() {
                if bulb.name.needs_refresh() {
                    let _ = self.protocol.send_refresh_message(
                        &self.sock,
                        bulb.target,
                        bulb.addr,
                        bulb.name.refresh_msg.clone(),
                    );
                }
                if bulb.power_level.needs_refresh() {
                    let _ = self.protocol.send_refresh_message(
                        &self.sock,
                        bulb.target,
                        bulb.addr,
                        bulb.power_level.refresh_msg.clone(),
                    );
                }
                if bulb.model.needs_refresh() {
                    let _ = self.protocol.send_refresh_message(
                        &self.sock,
                        bulb.target,
                        bulb.addr,
                        bulb.model.refresh_msg.clone(),
                    );
                }
                if bulb.group.needs_refresh() {
                    let _ = self.protocol.send_refresh_message(
                        &self.sock,
                        bulb.target,
                        bulb.addr,
                        bulb.group.refresh_msg.clone(),
                    );
                }
                if bulb.location.needs_refresh() {
                    let _ = self.protocol.send_refresh_message(
                        &self.sock,
                        bulb.target,
                        bulb.addr,
                        bulb.location.refresh_msg.clone(),
                    );
                }
            }
        }
    }
}