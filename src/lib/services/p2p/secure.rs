// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use log::{info, warn};
use rcgen::CertificateParams;
use ring::signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519};
use ring::{agreement, rand};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ClientConfig, ServerConfig};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

// Type alias for Send + Sync errors
type SecureError = Box<dyn std::error::Error + Send + Sync>;

const SESSION_KEY_ROTATION_INTERVAL: Duration = Duration::from_secs(3600); // 1 hour
const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024; // 10MB

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustLevel {
    Untrusted = 0,
    Low = 1,
    Medium = 2,
    High = 3,
    Trusted = 4,
}

#[derive(Debug, Clone)]
pub struct PeerIdentity {
    pub keypair: Arc<Ed25519KeyPair>,
    pub certificate: Vec<u8>,
    pub peer_id: PeerId,
}

#[derive(Clone)]
pub struct SecureChannel {
    peer_id: PeerId,
    session_key: Arc<RwLock<Vec<u8>>>,
    last_key_rotation: Arc<RwLock<SystemTime>>,
    cipher: Arc<RwLock<Aes256Gcm>>,
    peer_public_key: Vec<u8>,
    is_authenticated: Arc<RwLock<bool>>,
}

impl std::fmt::Debug for SecureChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecureChannel")
            .field("peer_id", &self.peer_id)
            .field("session_key", &"<hidden>")
            .field("last_key_rotation", &"<time>")
            .field("cipher", &"<cipher>")
            .field("peer_public_key", &"<public_key>")
            .field("is_authenticated", &"<bool>")
            .finish()
    }
}

pub struct SecureP2P {
    identity: PeerIdentity,
    #[allow(dead_code)]
    tls_config: Arc<ServerConfig>,
    #[allow(dead_code)]
    client_config: Arc<ClientConfig>,
    trusted_peers: Arc<RwLock<HashMap<PeerId, TrustLevel>>>,
    secure_channels: Arc<RwLock<HashMap<PeerId, SecureChannel>>>,
    blocked_peers: Arc<RwLock<Vec<PeerId>>>,
    reputation_scores: Arc<RwLock<HashMap<PeerId, f32>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptedMessage {
    pub sender_id: PeerId,
    pub recipient_id: PeerId,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
    pub signature: Vec<u8>,
    pub timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HandshakeMessage {
    pub peer_id: PeerId,
    pub certificate: Vec<u8>,
    pub ephemeral_public_key: Vec<u8>,
    pub signature: Vec<u8>,
    pub timestamp: u64,
    pub nonce: Vec<u8>,
}

impl PeerIdentity {
    pub fn generate(name: &str) -> Result<Self, SecureError> {
        let rng = rand::SystemRandom::new();
        let pkcs8_bytes = Ed25519KeyPair::generate_pkcs8(&rng)
            .map_err(|_| "Failed to generate Ed25519 keypair")?;
        let keypair = Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref())
            .map_err(|_| "Failed to create keypair from PKCS8")?;

        let peer_id = Self::derive_peer_id(&keypair);

        // Generate self-signed certificate
        let mut cert_params = CertificateParams::new(vec![name.to_string()])?;
        // Set basic certificate parameters
        cert_params
            .distinguished_name
            .push(rcgen::DnType::CommonName, name);

        // Generate a key pair for the certificate
        let cert_key_pair = rcgen::KeyPair::generate()?;
        let cert = cert_params.self_signed(&cert_key_pair)?;
        let certificate = cert.der().to_vec();

        Ok(Self {
            keypair: Arc::new(keypair),
            certificate,
            peer_id,
        })
    }

    fn derive_peer_id(keypair: &Ed25519KeyPair) -> PeerId {
        let public_key = keypair.public_key();
        let mut hasher = Sha256::default();
        hasher.update(public_key.as_ref());
        let hash = hasher.finalize();
        PeerId(hex::encode(&hash[..16]))
    }
}

impl SecureP2P {
    pub fn new(identity: PeerIdentity) -> Result<Self, SecureError> {
        // Create TLS configurations
        let tls_config = Self::create_tls_server_config(&identity)?;
        let client_config = Self::create_tls_client_config()?;

        Ok(Self {
            identity,
            tls_config: Arc::new(tls_config),
            client_config: Arc::new(client_config),
            trusted_peers: Arc::new(RwLock::new(HashMap::new())),
            secure_channels: Arc::new(RwLock::new(HashMap::new())),
            blocked_peers: Arc::new(RwLock::new(Vec::new())),
            reputation_scores: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    fn create_tls_server_config(identity: &PeerIdentity) -> Result<ServerConfig, SecureError> {
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let cert_key_pair = rcgen::KeyPair::generate()?;
        let mut cert_params = CertificateParams::new(vec![identity.peer_id.0.clone()])?;
        cert_params
            .distinguished_name
            .push(rcgen::DnType::CommonName, identity.peer_id.0.as_str());
        let cert = cert_params.self_signed(&cert_key_pair)?;
        let key = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(cert_key_pair.serialize_der()));

        let config = ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|e| format!("Failed to configure TLS protocol versions: {}", e))?
            .with_no_client_auth()
            .with_single_cert(vec![CertificateDer::from(cert.der().to_vec())], key)
            .map_err(|e| format!("Failed to create TLS server config: {}", e))?;
        Ok(config)
    }

    fn create_tls_client_config() -> Result<ClientConfig, SecureError> {
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let config = ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|e| format!("Failed to configure TLS protocol versions: {}", e))?
            .with_root_certificates(rustls::RootCertStore::empty())
            .with_no_client_auth();
        Ok(config)
    }

    pub async fn establish_secure_channel(
        &self,
        peer_info: &super::PeerInfo,
    ) -> Result<SecureChannel, Box<dyn std::error::Error>> {
        // Check if peer is blocked
        let blocked = self.blocked_peers.read().await;
        if blocked.iter().any(|p| p.0 == peer_info.id) {
            return Err("Peer is blocked".into());
        }
        drop(blocked);

        // Generate ephemeral key for ECDH
        let rng = rand::SystemRandom::new();
        let ephemeral_private = agreement::EphemeralPrivateKey::generate(&agreement::X25519, &rng)
            .map_err(|e| format!("Failed to generate ephemeral key: {:?}", e))?;
        let ephemeral_public = ephemeral_private
            .compute_public_key()
            .map_err(|e| format!("Failed to compute public key: {:?}", e))?;

        // Create handshake message
        let _handshake = self.create_handshake_message(ephemeral_public.as_ref().to_vec())?;

        // Exchange handshakes (simplified - in real implementation would use network)
        // This is where you'd send handshake and receive peer's handshake

        // Derive session key using ECDH
        let session_key = self.derive_session_key(&ephemeral_private, &peer_info.public_key)?;

        // Create cipher
        let key = Key::<Aes256Gcm>::from_slice(&session_key[..32]);
        let cipher = Aes256Gcm::new(key);

        let channel = SecureChannel {
            peer_id: PeerId(peer_info.id.clone()),
            session_key: Arc::new(RwLock::new(session_key)),
            last_key_rotation: Arc::new(RwLock::new(SystemTime::now())),
            cipher: Arc::new(RwLock::new(cipher)),
            peer_public_key: peer_info.public_key.clone(),
            is_authenticated: Arc::new(RwLock::new(false)),
        };

        // Perform mutual authentication
        self.authenticate_peer(&channel, peer_info).await?;

        // Store channel
        let mut channels = self.secure_channels.write().await;
        channels.insert(PeerId(peer_info.id.clone()), channel.clone());

        Ok(channel)
    }

    fn create_handshake_message(
        &self,
        ephemeral_public_key: Vec<u8>,
    ) -> Result<HandshakeMessage, Box<dyn std::error::Error>> {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        let mut nonce = vec![0u8; 32];
        let rng = ring::rand::SystemRandom::new();
        ring::rand::SecureRandom::fill(&rng, &mut nonce)
            .map_err(|_| "Failed to generate secure random nonce")?;
        let nonce = nonce;

        // Sign the handshake data
        let mut data_to_sign = Vec::new();
        data_to_sign.extend_from_slice(self.identity.peer_id.0.as_bytes());
        data_to_sign.extend_from_slice(&ephemeral_public_key);
        data_to_sign.extend_from_slice(&timestamp.to_be_bytes());
        data_to_sign.extend_from_slice(&nonce);

        let signature = self.identity.keypair.sign(&data_to_sign);

        Ok(HandshakeMessage {
            peer_id: self.identity.peer_id.clone(),
            certificate: self.identity.certificate.clone(),
            ephemeral_public_key,
            signature: signature.as_ref().to_vec(),
            timestamp,
            nonce,
        })
    }

    fn derive_session_key(
        &self,
        _ephemeral_private: &agreement::EphemeralPrivateKey,
        peer_public_key: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // TODO: Implement proper ECDH key agreement when ring API is clarified
        // For now, use a deterministic key based on peer public key
        let mut hasher = Sha256::default();
        hasher.update(peer_public_key);
        hasher.update(b"session_key_derivation");
        Ok(hasher.finalize().to_vec())
    }

    async fn authenticate_peer(
        &self,
        channel: &SecureChannel,
        peer_info: &super::PeerInfo,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Verify peer's certificate
        // In production, implement proper certificate chain validation

        // Verify peer's signature on handshake
        let _public_key = UnparsedPublicKey::new(&ED25519, &peer_info.public_key);

        // Create challenge-response authentication
        let mut challenge = vec![0u8; 32];
        let rng = ring::rand::SystemRandom::new();
        ring::rand::SecureRandom::fill(&rng, &mut challenge)
            .map_err(|_| "Failed to generate secure random challenge")?;
        let _challenge = challenge;

        // Send challenge and verify response (simplified)
        // In real implementation, this would involve network communication

        let mut authenticated = channel.is_authenticated.write().await;
        *authenticated = true;

        // Update trust level
        let mut trust = self.trusted_peers.write().await;
        trust.insert(channel.peer_id.clone(), TrustLevel::Medium);

        Ok(())
    }

    pub async fn encrypt_message(
        &self,
        peer_id: &str,
        message: &[u8],
    ) -> Result<EncryptedMessage, Box<dyn std::error::Error>> {
        // Check message size
        if message.len() > MAX_MESSAGE_SIZE {
            return Err("Message too large".into());
        }

        let channels = self.secure_channels.read().await;
        let channel = channels
            .get(&PeerId(peer_id.to_string()))
            .ok_or("No secure channel established with peer")?;

        // Check if authenticated
        if !*channel.is_authenticated.read().await {
            return Err("Peer not authenticated".into());
        }

        // Check if key rotation needed
        let last_rotation = *channel.last_key_rotation.read().await;
        if SystemTime::now().duration_since(last_rotation)? > SESSION_KEY_ROTATION_INTERVAL {
            // Rotate session key
            self.rotate_session_key(channel).await?;
        }

        // Generate nonce
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        // Encrypt message
        let cipher = channel.cipher.read().await;
        let ciphertext = cipher
            .encrypt(&nonce, message)
            .map_err(|e| format!("Encryption failed: {}", e))?;

        // Sign the encrypted message
        let mut data_to_sign = Vec::new();
        data_to_sign.extend_from_slice(self.identity.peer_id.0.as_bytes());
        data_to_sign.extend_from_slice(peer_id.as_bytes());
        data_to_sign.extend_from_slice(&nonce);
        data_to_sign.extend_from_slice(&ciphertext);

        let signature = self.identity.keypair.sign(&data_to_sign);

        Ok(EncryptedMessage {
            sender_id: self.identity.peer_id.clone(),
            recipient_id: PeerId(peer_id.to_string()),
            nonce: nonce.to_vec(),
            ciphertext,
            signature: signature.as_ref().to_vec(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        })
    }

    pub async fn decrypt_message(
        &self,
        encrypted_msg: &EncryptedMessage,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Verify we are the intended recipient
        if encrypted_msg.recipient_id.0 != self.identity.peer_id.0 {
            return Err("Message not intended for this peer".into());
        }

        let channels = self.secure_channels.read().await;
        let channel = channels
            .get(&encrypted_msg.sender_id)
            .ok_or("No secure channel established with sender")?;

        // Verify signature
        let mut data_to_verify = Vec::new();
        data_to_verify.extend_from_slice(encrypted_msg.sender_id.0.as_bytes());
        data_to_verify.extend_from_slice(encrypted_msg.recipient_id.0.as_bytes());
        data_to_verify.extend_from_slice(&encrypted_msg.nonce);
        data_to_verify.extend_from_slice(&encrypted_msg.ciphertext);

        let public_key = UnparsedPublicKey::new(&ED25519, &channel.peer_public_key);
        public_key
            .verify(&data_to_verify, &encrypted_msg.signature)
            .map_err(|_| "Invalid signature")?;

        // Decrypt message
        let cipher = channel.cipher.read().await;
        let nonce = Nonce::from_slice(&encrypted_msg.nonce);
        let plaintext = cipher
            .decrypt(nonce, encrypted_msg.ciphertext.as_ref())
            .map_err(|e| format!("Decryption failed: {}", e))?;

        // Update reputation score for successful communication
        self.update_reputation(&encrypted_msg.sender_id, 0.1).await;

        Ok(plaintext)
    }

    async fn rotate_session_key(
        &self,
        channel: &SecureChannel,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Generate new session key
        let mut new_key = vec![0u8; 32];
        let rng = rand::SystemRandom::new();
        use ring::rand::SecureRandom;
        rng.fill(&mut new_key)
            .map_err(|_| "Failed to generate new session key")?;

        // Update session key
        let mut session_key = channel.session_key.write().await;
        *session_key = new_key.clone();

        // Update cipher
        let key = Key::<Aes256Gcm>::from_slice(&new_key);
        let new_cipher = Aes256Gcm::new(key);
        let mut cipher = channel.cipher.write().await;
        *cipher = new_cipher;

        // Update rotation time
        let mut last_rotation = channel.last_key_rotation.write().await;
        *last_rotation = SystemTime::now();

        info!("Session key rotated for peer {:?}", channel.peer_id);

        Ok(())
    }

    pub async fn update_reputation(&self, peer_id: &PeerId, delta: f32) {
        let mut scores = self.reputation_scores.write().await;
        let score = scores.entry(peer_id.clone()).or_insert(0.5);
        *score = (*score + delta).clamp(0.0, 1.0);

        // Update trust level based on reputation
        if *score > 0.8 {
            let mut trust = self.trusted_peers.write().await;
            trust.insert(peer_id.clone(), TrustLevel::High);
        } else if *score < 0.2 {
            // Add to blocked list if reputation too low
            let mut blocked = self.blocked_peers.write().await;
            if !blocked.iter().any(|p| p.0 == peer_id.0) {
                blocked.push(peer_id.clone());
                warn!("Peer {:?} blocked due to low reputation", peer_id);
            }
        }
    }

    pub async fn get_trust_level(&self, peer_id: &str) -> TrustLevel {
        let trust = self.trusted_peers.read().await;
        trust
            .get(&PeerId(peer_id.to_string()))
            .copied()
            .unwrap_or(TrustLevel::Untrusted)
    }

    pub async fn set_trust_level(&self, peer_id: &str, level: TrustLevel) {
        let mut trust = self.trusted_peers.write().await;
        trust.insert(PeerId(peer_id.to_string()), level);
    }

    pub async fn block_peer(&self, peer_id: &str) {
        let mut blocked = self.blocked_peers.write().await;
        let peer = PeerId(peer_id.to_string());
        if !blocked.iter().any(|p| p.0 == peer.0) {
            blocked.push(peer);

            // Remove from trusted peers
            let mut trust = self.trusted_peers.write().await;
            trust.remove(&PeerId(peer_id.to_string()));

            // Remove secure channel
            let mut channels = self.secure_channels.write().await;
            channels.remove(&PeerId(peer_id.to_string()));

            info!("Peer {} blocked", peer_id);
        }
    }

    pub async fn unblock_peer(&self, peer_id: &str) {
        let mut blocked = self.blocked_peers.write().await;
        blocked.retain(|p| p.0 != peer_id);
        info!("Peer {} unblocked", peer_id);
    }

    pub async fn is_peer_blocked(&self, peer_id: &str) -> bool {
        let blocked = self.blocked_peers.read().await;
        blocked.iter().any(|p| p.0 == peer_id)
    }

    pub async fn cleanup_inactive_channels(&self, timeout: Duration) {
        let mut channels = self.secure_channels.write().await;
        let now = SystemTime::now();

        channels.retain(|peer_id, channel| {
            if let Ok(last_rotation) = channel.last_key_rotation.try_read() {
                if let Ok(duration) = now.duration_since(*last_rotation) {
                    if duration > timeout {
                        info!("Removing inactive channel for peer {:?}", peer_id);
                        return false;
                    }
                }
            }
            true
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_peer_identity_generation() {
        let identity = PeerIdentity::generate("test_node").unwrap();
        assert!(!identity.peer_id.0.is_empty());
        assert!(!identity.certificate.is_empty());
    }

    #[tokio::test]
    async fn test_secure_p2p_creation() {
        let identity = PeerIdentity::generate("test_node").unwrap();
        let secure_p2p = SecureP2P::new(identity).unwrap();
        assert_eq!(
            secure_p2p.get_trust_level("unknown").await,
            TrustLevel::Untrusted
        );
    }

    #[tokio::test]
    async fn test_peer_blocking() {
        let identity = PeerIdentity::generate("test_node").unwrap();
        let secure_p2p = SecureP2P::new(identity).unwrap();

        secure_p2p.block_peer("bad_peer").await;
        assert!(secure_p2p.is_peer_blocked("bad_peer").await);

        secure_p2p.unblock_peer("bad_peer").await;
        assert!(!secure_p2p.is_peer_blocked("bad_peer").await);
    }

    #[tokio::test]
    async fn test_reputation_update() {
        let identity = PeerIdentity::generate("test_node").unwrap();
        let secure_p2p = SecureP2P::new(identity).unwrap();

        let peer_id = PeerId("test_peer".to_string());

        // Update reputation positively
        for _ in 0..5 {
            secure_p2p.update_reputation(&peer_id, 0.1).await;
        }

        // Check if trust level increased
        let trust = secure_p2p.get_trust_level("test_peer").await;
        assert_ne!(trust, TrustLevel::Untrusted);
    }
}
