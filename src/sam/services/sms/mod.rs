pub mod twilio;
pub mod vonage;
pub mod plivo;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex as StdMutex;


static SERVICE_RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone)]
pub enum Provider {
    Twilio,
    Vonage,
    Plivo,
}

lazy_static::lazy_static! {
    static ref CURRENT_PROVIDER: StdMutex<Provider> = StdMutex::new(Provider::Twilio);
}

/// Start the SMS service
pub async fn start() {
    SERVICE_RUNNING.store(true, Ordering::SeqCst);
    log::info!("SMS service started.");
}

/// Stop the SMS service
pub async fn stop() {
    SERVICE_RUNNING.store(false, Ordering::SeqCst);
    log::info!("SMS service stopped.");
}

/// Get the status of the SMS service
pub fn status() -> &'static str {
    if SERVICE_RUNNING.load(Ordering::SeqCst) {
        "running"
    } else {
        "stopped"
    }
}

/// Set the current SMS provider
pub fn set_provider(provider: Provider) {
    let mut guard = CURRENT_PROVIDER.lock().unwrap();
    *guard = provider;
}

/// Send an SMS message using the current provider
pub async fn send_sms(to: &str, body: &str) -> Result<(), String> {
    if !SERVICE_RUNNING.load(Ordering::SeqCst) {
        return Err("SMS service is not running".to_string());
    }
    let provider = CURRENT_PROVIDER.lock().unwrap().clone();
    match provider {
        Provider::Twilio => twilio::send_sms(to, body).await,
        Provider::Vonage => vonage::send_sms(to, body).await,
        Provider::Plivo => plivo::send_sms(to, body).await,
    }
}

/// Receive SMS messages using the current provider
pub async fn receive_sms() -> Result<Vec<String>, String> {
    if !SERVICE_RUNNING.load(Ordering::SeqCst) {
        return Err("SMS service is not running".to_string());
    }
    let provider = CURRENT_PROVIDER.lock().unwrap().clone();
    match provider {
        Provider::Twilio => twilio::receive_sms().await,
        Provider::Vonage => vonage::receive_sms().await,
        Provider::Plivo => plivo::receive_sms().await,
    }
}
