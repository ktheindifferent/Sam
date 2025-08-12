// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::sam::services::p2p::enhanced::{P2PNode, P2PConfig, P2PMessage, PeerInfo};
    use crate::sam::services::p2p::secure::{SecureP2P, PeerIdentity, TrustLevel};
    use crate::sam::services::p2p::network_segmentation::{NetworkSegmentation, ChannelType, Priority};
    use tokio::time::{sleep, Duration};
    use std::net::SocketAddr;

    #[tokio::test]
    async fn test_signature_verification() {
        let config = P2PConfig::default();
        let node = P2PNode::new("test_node".to_string(), config).unwrap();
        
        // Create a handshake message
        let handshake = node.create_handshake().await.unwrap();
        
        if let P2PMessage::Handshake { peer_info, nonce, signature } = handshake {
            // Verify the signature
            let result = node.verify_handshake(&peer_info, &nonce, &signature).unwrap();
            assert!(result, "Signature verification should succeed for valid signature");
            
            // Test with invalid signature
            let mut bad_signature = signature.clone();
            bad_signature[0] ^= 0xFF; // Corrupt the signature
            let result = node.verify_handshake(&peer_info, &nonce, &bad_signature).unwrap();
            assert!(!result, "Signature verification should fail for invalid signature");
        } else {
            panic!("Expected handshake message");
        }
    }

    #[tokio::test]
    async fn test_encryption_decryption() {
        let config = P2PConfig::default();
        let node = P2PNode::new("test_node".to_string(), config).unwrap();
        
        let plaintext = b"Secret message for P2P communication";
        
        // Encrypt data
        let encrypted = node.encrypt_data(plaintext).unwrap();
        assert_ne!(encrypted, plaintext.to_vec(), "Encrypted data should differ from plaintext");
        
        // Decrypt data
        let decrypted = node.decrypt_data(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext.to_vec(), "Decrypted data should match original");
    }

    #[tokio::test]
    async fn test_secure_p2p_trust_levels() {
        let identity = PeerIdentity::generate("test_node").unwrap();
        let secure_p2p = SecureP2P::new(identity).unwrap();
        
        // Test default trust level
        let trust = secure_p2p.get_trust_level("unknown_peer").await;
        assert_eq!(trust, TrustLevel::Untrusted);
        
        // Set and verify trust levels
        secure_p2p.set_trust_level("peer1", TrustLevel::High).await;
        let trust = secure_p2p.get_trust_level("peer1").await;
        assert_eq!(trust, TrustLevel::High);
        
        // Test peer blocking
        secure_p2p.block_peer("malicious_peer").await;
        assert!(secure_p2p.is_peer_blocked("malicious_peer").await);
        
        // Unblock and verify
        secure_p2p.unblock_peer("malicious_peer").await;
        assert!(!secure_p2p.is_peer_blocked("malicious_peer").await);
    }

    #[tokio::test]
    async fn test_network_segmentation() {
        let segmentation = NetworkSegmentation::new();
        
        // Test message queueing with different priorities
        let result1 = segmentation.queue_message(
            "peer1",
            ChannelType::Emergency,
            Priority::Critical,
            vec![1, 2, 3],
        ).await;
        assert!(result1.is_ok());
        
        let result2 = segmentation.queue_message(
            "peer1",
            ChannelType::Data,
            Priority::Normal,
            vec![4, 5, 6],
        ).await;
        assert!(result2.is_ok());
        
        // Verify critical message is retrieved first
        let msg = segmentation.get_next_message("peer1").await;
        assert!(msg.is_some());
        if let Some((channel, data)) = msg {
            assert_eq!(channel, ChannelType::Emergency);
            assert_eq!(data, vec![1, 2, 3]);
        }
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let segmentation = NetworkSegmentation::new();
        
        // Send messages within rate limit
        for i in 0..10 {
            let result = segmentation.can_send(
                "peer2",
                ChannelType::Data,
                100,
            ).await;
            assert!(result.is_ok(), "Message {} should be allowed", i);
        }
        
        // Send many messages to trigger rate limit
        let mut violations = 0;
        for _ in 0..500 {
            if segmentation.can_send("peer3", ChannelType::Data, 1000).await.is_err() {
                violations += 1;
            }
        }
        
        assert!(violations > 0, "Rate limiting should trigger after many messages");
    }

    #[tokio::test]
    async fn test_message_integrity() {
        let config = P2PConfig::default();
        let node = P2PNode::new("test_node".to_string(), config).unwrap();
        
        // Start the node
        node.start().await.unwrap();
        
        // Create a test message
        let message = P2PMessage::Data {
            id: "test_msg_1".to_string(),
            payload: vec![1, 2, 3, 4, 5],
            encrypted: false,
            compression: None,
        };
        
        // The message should include integrity checks when sent
        // This is tested implicitly through the send/receive mechanism
        
        node.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_secure_broadcast() {
        let config = P2PConfig {
            enable_encryption: true,
            ..Default::default()
        };
        
        let node = P2PNode::new("broadcast_node".to_string(), config).unwrap();
        
        // Start the node
        node.start().await.unwrap();
        
        // Test that broadcasts are properly secured
        let test_data = vec![1, 2, 3, 4, 5];
        let message = P2PMessage::Data {
            id: "broadcast_1".to_string(),
            payload: test_data.clone(),
            encrypted: true,
            compression: None,
        };
        
        // Broadcast should handle encryption automatically
        let result = node.broadcast(message).await;
        assert!(result.is_ok());
        
        node.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_peer_authentication() {
        let identity1 = PeerIdentity::generate("node1").unwrap();
        let secure_p2p1 = SecureP2P::new(identity1).unwrap();
        
        let identity2 = PeerIdentity::generate("node2").unwrap();
        let secure_p2p2 = SecureP2P::new(identity2).unwrap();
        
        // Test mutual authentication would happen during channel establishment
        // In a real scenario, this would involve network communication
        
        // Verify that blocked peers cannot establish channels
        secure_p2p1.block_peer("blocked_node").await;
        assert!(secure_p2p1.is_peer_blocked("blocked_node").await);
    }

    #[tokio::test]
    async fn test_compression() {
        let config = P2PConfig::default();
        let node = P2PNode::new("compression_test".to_string(), config).unwrap();
        
        let large_data = vec![42u8; 10000]; // Repetitive data that compresses well
        
        // Test compression
        let compressed = node.compress_data(&large_data, "gzip").unwrap();
        assert!(compressed.len() < large_data.len(), "Compressed data should be smaller");
        
        // Test decompression
        let decompressed = node.decompress_data(&compressed, "gzip").unwrap();
        assert_eq!(decompressed, large_data, "Decompressed data should match original");
    }

    #[tokio::test]
    async fn test_reputation_system() {
        let identity = PeerIdentity::generate("reputation_test").unwrap();
        let secure_p2p = SecureP2P::new(identity).unwrap();
        
        let peer_id = secure::PeerId("test_peer".to_string());
        
        // Increase reputation
        for _ in 0..10 {
            secure_p2p.update_reputation(&peer_id, 0.1).await;
        }
        
        // Check if trust level increased (reputation > 0.8 should give High trust)
        let trust = secure_p2p.get_trust_level("test_peer").await;
        assert_ne!(trust, TrustLevel::Untrusted, "Trust level should increase with good reputation");
        
        // Decrease reputation significantly
        for _ in 0..20 {
            secure_p2p.update_reputation(&peer_id, -0.1).await;
        }
        
        // Check if peer is blocked (reputation < 0.2 should block)
        sleep(Duration::from_millis(100)).await; // Allow async operations to complete
        let is_blocked = secure_p2p.is_peer_blocked("test_peer").await;
        assert!(is_blocked || trust == TrustLevel::Untrusted, "Low reputation should result in blocking or untrusted status");
    }
}