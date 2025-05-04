use std::sync::Arc;
use matc::{certmanager::{self, FileCertManager}, controller, transport, tlv, clusters, onboarding};
use crate::sam::memory::Thing;
use crate::sam::memory::Result;
use rouille::input::json_input;
use serde::Deserialize;
use std::time::Duration;
use serde::Serialize;

const DEFAULT_FABRIC: u64 = 0x110;
const DEFAULT_LOCAL_ADDRESS: &str = "0.0.0.0:5555";
const DEFAULT_CERT_PATH: &str = "./pem";
const DEFAULT_ENDPOINT: u16 = 1;

pub struct MatterDeviceController;
// matter pair 192.168.86.164:5540 2006-121-5962 [local_address]
impl MatterDeviceController {
    /// Commission (pair) a new Matter device and store it in the Things table.
    pub async fn pair_device(
        device_address: &str,
        pin: &str,
        local_address: &str,
    ) -> Result<Thing> {
        let cert_path = "/opt/sam/keys/matter/";
        let code = onboarding::decode_manual_pairing_code(&pin).unwrap();
        let device_id = 300;
        let cm: Arc<dyn certmanager::CertManager> = match FileCertManager::load(cert_path) {
            Ok(cm) => cm,
            Err(_) => {
                let fcm = FileCertManager::new(100, cert_path);
                fcm.bootstrap()?;
                fcm.create_user(100)?; //controller_id, default to 100 for now
                fcm // fcm is already Arc<FileCertManager>
            },
        };
        
        let transport = transport::Transport::new(local_address).await?;
        let controller = controller::Controller::new(&cm, &transport, cm.get_fabric_id())?;
        let connection = transport.create_connection(device_address).await;

        log::info!("Connecting to device at {}", device_address);
        log::info!("Pairing with device ID: {}", device_id);
        log::info!("Using passcode: {}", code.passcode);
        log::info!("Using local address: {}", local_address);
        log::info!("Using cert path: {}", cert_path);
        log::info!("Using fabric ID: {}", DEFAULT_FABRIC);
        log::info!("Using controller ID: {}", 100);

        let mut con = controller
            .commission(&connection, code.passcode, device_id, 100)
            .await?;
        // You may want to read device info here (e.g., product name, type)
        // For now, just store the device_id and address
        let mut thing = Thing::new();
        thing.name = format!("Matter Device {}", device_id);
        thing.thing_type = "matter".to_string();
        thing.ip_address = device_address.to_string();
        thing.online_identifiers = vec![device_id.to_string()];
        thing.save_async().await?;

        Ok(thing)
    }

    /// Turn a Matter device on or off.
    pub async fn set_device_on_off(
        thing: &Thing,
        on: bool,
        local_address: &str,
    ) -> Result<()> {
        let cert_path = "/opt/sam/keys/matter/";
        let controller_id = 100; // Default controller ID
        let device_id = thing
            .online_identifiers
            .get(0)
            .ok_or_else(|| anyhow::anyhow!("No device_id found"))?
            .parse::<u64>()?;
        let device_address = &thing.ip_address;
        let cm: Arc<dyn certmanager::CertManager> = FileCertManager::load(cert_path)?;
        let transport = transport::Transport::new(local_address).await?;
        let controller = controller::Controller::new(&cm, &transport, cm.get_fabric_id())?;
        let connection = transport.create_connection(device_address).await;
        let mut con = controller
            .auth_sigma(&connection, device_id, controller_id)
            .await?;
        let endpoint = DEFAULT_ENDPOINT;
        let (cluster, command_id) = (0x6, if on { 1 } else { 0 }); // On/Off cluster
        let res = con.invoke_request(endpoint, cluster, command_id, &[]).await?;
        res.tlv.dump(1);
        Ok(())
    }

    /// Set a level (e.g., brightness) for a Matter device.
    pub async fn set_device_level(
        thing: &Thing,
        level: u8,
        local_address: &str,
    ) -> Result<()> {
        let cert_path = "/opt/sam/keys/matter/";
        let controller_id = 100; // Default controller ID
        let device_id = thing
            .online_identifiers
            .get(0)
            .ok_or_else(|| anyhow::anyhow!("No device_id found"))?
            .parse::<u64>()?;
        let device_address = &thing.ip_address;
        let cm: Arc<dyn certmanager::CertManager> = FileCertManager::load(cert_path)?;
        let transport = transport::Transport::new(local_address).await?;
        let controller = controller::Controller::new(&cm, &transport, cm.get_fabric_id())?;
        let connection = transport.create_connection(device_address).await;
        let mut con = controller
            .auth_sigma(&connection, device_id, controller_id)
            .await?;
        let endpoint = DEFAULT_ENDPOINT;
        let tlv = tlv::TlvItemEnc {
            tag: 0,
            value: tlv::TlvItemValueEnc::StructInvisible(vec![
                tlv::TlvItemEnc {
                    tag: 0,
                    value: tlv::TlvItemValueEnc::UInt8(level),
                }, // level
                tlv::TlvItemEnc {
                    tag: 1,
                    value: tlv::TlvItemValueEnc::UInt16(10),
                }, // transition time
                tlv::TlvItemEnc {
                    tag: 2,
                    value: tlv::TlvItemValueEnc::UInt8(0),
                }, // options mask
                tlv::TlvItemEnc {
                    tag: 3,
                    value: tlv::TlvItemValueEnc::UInt8(0),
                }, // options override
            ]),
        }
        .encode()?;
        let res = con
            .invoke_request(
                endpoint,
                clusters::defs::CLUSTER_ID_LEVEL_CONTROL,
                clusters::defs::CLUSTER_LEVEL_CONTROL_CMD_ID_MOVETOLEVEL,
                &tlv,
            )
            .await?;
        res.tlv.dump(1);
        Ok(())
    }


}

    /// HTTP handler for Matter device operations.
    pub fn handle(request: &rouille::Request) -> rouille::Response {

        #[derive(Deserialize)]
        struct PairRequest {
            new_thing_name: String,
            new_thing_ip: String,
            new_thing_password: String,
            local_address: Option<String>,
        }

        #[derive(Deserialize)]
        struct OnOffRequest {
            thing_id: String,
            on: bool,
            local_address: Option<String>,
        }

        #[derive(Deserialize)]
        struct LevelRequest {
            thing_id: String,
            level: u8,
            local_address: Option<String>,
        }

        match request.url().as_str() {

            "/api/things/matter/commissionable" if request.method() == "GET" => {

                #[derive(Deserialize)]
                struct Query {
                    timeout: Option<u64>,
                }

                #[derive(Serialize)]
                struct CommissionableDevice {
                    name: String,
                    ip: String,
                    port: u16,
                    discriminator: Option<u16>,
                    vendor_id: Option<u16>,
                    product_id: Option<u16>,
                }

                let timeout = request.get_param("timeout").and_then(|s| s.parse().ok()).unwrap_or(5);

                match futures::executor::block_on(matc::discover::discover_commissionable(Duration::from_secs(timeout))) {
                    Ok(devices) => {
                        let result: Vec<CommissionableDevice> = devices.into_iter().map(|d| CommissionableDevice {
                            name: d.name.unwrap_or_default(),
                            ip: d.ips.get(0).map(|ip| ip.to_string()).unwrap_or_default(),
                            port: d.port.unwrap_or(0),
                            discriminator: d.discriminator.and_then(|s| s.parse().ok()),
                            vendor_id: d.vendor_id.and_then(|s| s.parse().ok()),
                            product_id: d.product_id.and_then(|s| s.parse().ok()),
                        }).collect();
                        rouille::Response::json(&result)
                    }
                    Err(e) => rouille::Response::text(format!("Discovery failed: {}", e)).with_status_code(500),
                }
            }
            "/api/things/matter/pair" if request.method() == "POST" => {
                if let Some(input) = json_input::<PairRequest>(request).ok() {
                    let local_address = input.local_address.unwrap_or_else(|| DEFAULT_LOCAL_ADDRESS.to_string());
                    let fut = MatterDeviceController::pair_device(
                        &input.new_thing_ip,
                        &input.new_thing_password,
                        &local_address,
                    );
                    match futures::executor::block_on(fut) {
                        Ok(thing) => rouille::Response::json(&thing),
                        Err(e) => rouille::Response::text(format!("Pairing failed: {}", e)).with_status_code(500),
                    }
                } else {
                    rouille::Response::text("Invalid input").with_status_code(400)
                }
            }
            "/api/things/matter/power" if request.method() == "POST" => {
                if let Some(input) = json_input::<OnOffRequest>(request).ok() {
                    // You need to load Thing from your storage by thing_id
                    match Thing::load_by_id(&input.thing_id) {
                        Ok(thing) => {
                            let local_address = input.local_address.unwrap_or_else(|| DEFAULT_LOCAL_ADDRESS.to_string());
                            let fut = MatterDeviceController::set_device_on_off(&thing, input.on, &local_address);
                            match futures::executor::block_on(fut) {
                                Ok(_) => rouille::Response::text("Success"),
                                Err(e) => rouille::Response::text(format!("On/Off failed: {}", e)).with_status_code(500),
                            }
                        }
                        Err(e) => rouille::Response::text(format!("Thing not found: {}", e)).with_status_code(404),
                    }
                } else {
                    rouille::Response::text("Invalid input").with_status_code(400)
                }
            }
            "/api/things/matter/level" if request.method() == "POST" => {
                if let Some(input) = json_input::<LevelRequest>(request).ok() {
                    match Thing::load_by_id(&input.thing_id) {
                        Ok(thing) => {
                            let local_address = input.local_address.unwrap_or_else(|| DEFAULT_LOCAL_ADDRESS.to_string());
                            let fut = MatterDeviceController::set_device_level(&thing, input.level, &local_address);
                            match futures::executor::block_on(fut) {
                                Ok(_) => rouille::Response::text("Success"),
                                Err(e) => rouille::Response::text(format!("Set level failed: {}", e)).with_status_code(500),
                            }
                        }
                        Err(e) => rouille::Response::text(format!("Thing not found: {}", e)).with_status_code(404),
                    }
                } else {
                    rouille::Response::text("Invalid input").with_status_code(400)
                }
            }
            _ => rouille::Response::empty_404(),
        }
    }