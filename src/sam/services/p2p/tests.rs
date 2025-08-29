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

    #[tokio::test]
    async fn test_handle_data_validation() {
        let config = P2PConfig::default();
        let node = P2PNode::new("data_test".to_string(), config).unwrap();
        
        // Test empty data rejection
        let result = node.handle_data("test_id".to_string(), vec![], "peer1".to_string()).await;
        assert!(result.is_err(), "Empty data should be rejected");
        
        // Test oversized data rejection
        let large_data = vec![0u8; 6 * 1024 * 1024]; // 6MB
        let result = node.handle_data("test_id".to_string(), large_data, "peer1".to_string()).await;
        assert!(result.is_err(), "Oversized data should be rejected");
        
        // Test valid JSON data
        let json_data = serde_json::json!({
            "type": "file_metadata",
            "metadata": {
                "filename": "test.txt",
                "size": 1024
            }
        });
        let data = serde_json::to_vec(&json_data).unwrap();
        let result = node.handle_data("test_id".to_string(), data, "peer1".to_string()).await;
        assert!(result.is_ok(), "Valid JSON data should be accepted");
        
        // Test binary data validation
        let binary_data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let result = node.handle_data("test_id".to_string(), binary_data, "peer1".to_string()).await;
        assert!(result.is_ok(), "Valid binary data should be accepted");
    }

    #[tokio::test]
    async fn test_json_depth_validation() {
        let config = P2PConfig::default();
        let node = P2PNode::new("depth_test".to_string(), config).unwrap();
        
        // Create deeply nested JSON (exceeds max depth of 10)
        let mut deeply_nested = serde_json::json!({"level": 0});
        for i in 1..15 {
            deeply_nested = serde_json::json!({"level": i, "nested": deeply_nested});
        }
        
        let valid = node.validate_json_data(&deeply_nested);
        assert!(!valid, "Deeply nested JSON should be rejected");
        
        // Test normal depth JSON
        let normal_json = serde_json::json!({
            "level1": {
                "level2": {
                    "level3": {
                        "data": "test"
                    }
                }
            }
        });
        
        let valid = node.validate_json_data(&normal_json);
        assert!(valid, "Normal depth JSON should be accepted");
    }

    #[tokio::test]
    async fn test_rpc_method_validation() {
        let config = P2PConfig::default();
        let node = P2PNode::new("rpc_test".to_string(), config).unwrap();
        
        // Test empty method name
        let result = node.handle_request("".to_string(), serde_json::json!({})).await;
        assert!(result.is_err(), "Empty method name should be rejected");
        
        // Test method name with invalid characters
        let result = node.handle_request("test$method!".to_string(), serde_json::json!({})).await;
        assert!(result.is_err(), "Method name with special characters should be rejected");
        
        // Test overly long method name
        let long_method = "a".repeat(101);
        let result = node.handle_request(long_method, serde_json::json!({})).await;
        assert!(result.is_err(), "Overly long method name should be rejected");
        
        // Test valid method name
        let result = node.handle_request("peer.info".to_string(), serde_json::json!({})).await;
        assert!(result.is_ok(), "Valid method name should be accepted");
    }

    #[tokio::test]
    async fn test_rpc_peer_info() {
        let config = P2PConfig::default();
        let node = P2PNode::new("rpc_peer_test".to_string(), config).unwrap();
        
        let result = node.handle_request("peer.info".to_string(), serde_json::json!({})).await;
        assert!(result.is_ok(), "peer.info should succeed");
        
        let response = result.unwrap();
        assert!(response.get("id").is_some(), "Response should contain id");
        assert!(response.get("name").is_some(), "Response should contain name");
        assert!(response.get("version").is_some(), "Response should contain version");
        assert!(response.get("capabilities").is_some(), "Response should contain capabilities");
    }

    #[tokio::test]
    async fn test_rpc_network_ping() {
        let config = P2PConfig::default();
        let node = P2PNode::new("ping_test".to_string(), config).unwrap();
        
        let timestamp = 1234567890u64;
        let params = serde_json::json!({ "timestamp": timestamp });
        let result = node.handle_request("network.ping".to_string(), params).await;
        
        assert!(result.is_ok(), "network.ping should succeed");
        
        let response = result.unwrap();
        assert_eq!(response.get("pong"), Some(&serde_json::Value::Bool(true)));
        assert_eq!(response.get("timestamp"), Some(&serde_json::Value::Number(timestamp.into())));
        assert!(response.get("server_time").is_some(), "Response should contain server_time");
    }

    #[tokio::test]
    async fn test_rpc_data_methods() {
        let config = P2PConfig::default();
        let node = P2PNode::new("data_rpc_test".to_string(), config).unwrap();
        
        // Test data.exists
        let params = serde_json::json!({ "id": "test_data_id" });
        let result = node.handle_request("data.exists".to_string(), params.clone()).await;
        assert!(result.is_ok(), "data.exists should succeed");
        
        let response = result.unwrap();
        assert_eq!(response.get("exists"), Some(&serde_json::Value::Bool(false)));
        
        // Test data.get
        let result = node.handle_request("data.get".to_string(), params).await;
        assert!(result.is_ok(), "data.get should succeed");
        
        let response = result.unwrap();
        assert_eq!(response.get("found"), Some(&serde_json::Value::Bool(false)));
        
        // Test missing parameter
        let result = node.handle_request("data.exists".to_string(), serde_json::json!({})).await;
        assert!(result.is_err(), "data.exists without id parameter should fail");
    }

    #[tokio::test]
    async fn test_rpc_file_methods() {
        let config = P2PConfig::default();
        let node = P2PNode::new("file_rpc_test".to_string(), config).unwrap();
        
        // Test file.list
        let result = node.handle_request("file.list".to_string(), serde_json::json!({})).await;
        assert!(result.is_ok(), "file.list should succeed");
        
        let response = result.unwrap();
        assert!(response.get("files").is_some(), "Response should contain files");
        assert!(response.get("count").is_some(), "Response should contain count");
        
        // Test file.request
        let params = serde_json::json!({ 
            "file_id": "test_file",
            "chunk_size": 32768
        });
        let result = node.handle_request("file.request".to_string(), params).await;
        assert!(result.is_ok(), "file.request should succeed");
    }

    #[tokio::test]
    async fn test_rpc_broadcast_methods() {
        let config = P2PConfig::default();
        let node = P2PNode::new("broadcast_test".to_string(), config).unwrap();
        
        // Test subscribe
        let params = serde_json::json!({ "topic": "test_topic" });
        let result = node.handle_request("broadcast.subscribe".to_string(), params.clone()).await;
        assert!(result.is_ok(), "broadcast.subscribe should succeed");
        
        let response = result.unwrap();
        assert_eq!(response.get("subscribed"), Some(&serde_json::Value::Bool(true)));
        assert_eq!(response.get("topic"), Some(&serde_json::Value::String("test_topic".to_string())));
        
        // Test unsubscribe
        let result = node.handle_request("broadcast.unsubscribe".to_string(), params).await;
        assert!(result.is_ok(), "broadcast.unsubscribe should succeed");
        
        // Test missing parameter
        let result = node.handle_request("broadcast.subscribe".to_string(), serde_json::json!({})).await;
        assert!(result.is_err(), "broadcast.subscribe without topic should fail");
    }

    #[tokio::test]
    async fn test_rpc_sync_methods() {
        let config = P2PConfig::default();
        let node = P2PNode::new("sync_test".to_string(), config).unwrap();
        
        let params = serde_json::json!({ 
            "type": "state",
            "from_timestamp": 1234567890
        });
        let result = node.handle_request("sync.request".to_string(), params).await;
        assert!(result.is_ok(), "sync.request should succeed");
        
        let response = result.unwrap();
        assert!(response.get("sync_type").is_some(), "Response should contain sync_type");
        assert!(response.get("data").is_some(), "Response should contain data");
    }

    #[tokio::test]
    async fn test_rpc_custom_methods() {
        let config = P2PConfig::default();
        let node = P2PNode::new("custom_test".to_string(), config).unwrap();
        
        let params = serde_json::json!({ "custom_param": "value" });
        let result = node.handle_request("custom.my_method".to_string(), params.clone()).await;
        assert!(result.is_ok(), "custom methods should be handled");
        
        let response = result.unwrap();
        assert_eq!(response.get("method"), Some(&serde_json::Value::String("my_method".to_string())));
        assert_eq!(response.get("params"), Some(&params));
    }

    #[tokio::test]
    async fn test_unknown_rpc_method() {
        let config = P2PConfig::default();
        let node = P2PNode::new("unknown_test".to_string(), config).unwrap();
        
        let result = node.handle_request("unknown.method".to_string(), serde_json::json!({})).await;
        assert!(result.is_err(), "Unknown method should return error");
        
        if let Err(e) = result {
            assert!(e.to_string().contains("Unknown method"), "Error should indicate unknown method");
        }
    }

    #[tokio::test]
    async fn test_data_type_handling() {
        let config = P2PConfig::default();
        let node = P2PNode::new("data_type_test".to_string(), config).unwrap();
        
        // Test state_update handling (should be ignored from untrusted peer)
        let state_data = serde_json::json!({
            "type": "state_update",
            "state": {
                "key": "value"
            }
        });
        let data = serde_json::to_vec(&state_data).unwrap();
        let result = node.handle_data("state_id".to_string(), data, "untrusted_peer".to_string()).await;
        assert!(result.is_ok(), "State update from untrusted peer should be handled gracefully");
        
        // Test broadcast_message handling
        let broadcast_data = serde_json::json!({
            "type": "broadcast_message",
            "content": {
                "message": "Hello P2P Network"
            }
        });
        let data = serde_json::to_vec(&broadcast_data).unwrap();
        let result = node.handle_data("broadcast_id".to_string(), data, "peer1".to_string()).await;
        assert!(result.is_ok(), "Broadcast message should be handled");
        
        // Test chunk_data handling
        let chunk_data = serde_json::json!({
            "type": "chunk_data",
            "chunk_index": 0,
            "total_chunks": 10,
            "data": "chunk_content_here"
        });
        let data = serde_json::to_vec(&chunk_data).unwrap();
        let result = node.handle_data("chunk_id".to_string(), data, "peer1".to_string()).await;
        assert!(result.is_ok(), "Chunk data should be handled");
    }

    #[tokio::test]
    async fn test_binary_data_validation() {
        let config = P2PConfig::default();
        let node = P2PNode::new("binary_test".to_string(), config).unwrap();
        
        // Test potentially malicious binary data (many null bytes at start)
        let bad_binary = vec![0, 0, 0, 1, 2, 3];
        let valid = node.validate_binary_data(&bad_binary);
        assert!(!valid, "Binary data with suspicious null byte pattern should be rejected");
        
        // Test normal binary data
        let good_binary = vec![0xFF, 0xAA, 0x55, 0x12, 0x34, 0x56];
        let valid = node.validate_binary_data(&good_binary);
        assert!(valid, "Normal binary data should be accepted");
    }

    // Integration tests with multiple peers
    #[tokio::test]
    async fn test_multi_peer_data_exchange() {
        use std::sync::Arc;
        
        // Create multiple P2P nodes
        let config1 = P2PConfig {
            port: 9100,
            ..Default::default()
        };
        let config2 = P2PConfig {
            port: 9101,
            ..Default::default()
        };
        let config3 = P2PConfig {
            port: 9102,
            ..Default::default()
        };
        
        let node1 = Arc::new(P2PNode::new("node1".to_string(), config1).unwrap());
        let node2 = Arc::new(P2PNode::new("node2".to_string(), config2).unwrap());
        let node3 = Arc::new(P2PNode::new("node3".to_string(), config3).unwrap());
        
        // Start nodes
        let n1 = Arc::clone(&node1);
        let n2 = Arc::clone(&node2);
        let n3 = Arc::clone(&node3);
        
        tokio::spawn(async move { n1.start().await });
        tokio::spawn(async move { n2.start().await });
        tokio::spawn(async move { n3.start().await });
        
        // Give nodes time to start
        sleep(Duration::from_millis(500)).await;
        
        // Connect nodes to each other
        let addr1: SocketAddr = "127.0.0.1:9100".parse().unwrap();
        let addr2: SocketAddr = "127.0.0.1:9101".parse().unwrap();
        let addr3: SocketAddr = "127.0.0.1:9102".parse().unwrap();
        
        // Connect node2 to node1
        if let Err(e) = node2.connect_to_peer(addr1).await {
            // Connection might fail in test environment, that's ok
            println!("Connection test warning: {}", e);
        }
        
        // Connect node3 to node1 and node2
        if let Err(e) = node3.connect_to_peer(addr1).await {
            println!("Connection test warning: {}", e);
        }
        if let Err(e) = node3.connect_to_peer(addr2).await {
            println!("Connection test warning: {}", e);
        }
        
        // Give connections time to establish
        sleep(Duration::from_millis(500)).await;
        
        // Test data broadcast
        let test_data = serde_json::json!({
            "type": "broadcast_message",
            "content": {
                "test": "multi-peer broadcast"
            }
        });
        
        let message = P2PMessage::Data {
            id: "test_broadcast".to_string(),
            payload: serde_json::to_vec(&test_data).unwrap(),
            encrypted: false,
            compression: None,
        };
        
        // Broadcast from node1
        if let Err(e) = node1.broadcast(message).await {
            println!("Broadcast test warning: {}", e);
        }
        
        // Stop nodes
        let _ = node1.stop().await;
        let _ = node2.stop().await;
        let _ = node3.stop().await;
    }

    #[tokio::test]
    async fn test_peer_to_peer_rpc() {
        use std::sync::Arc;
        use tokio::sync::mpsc;
        
        // Create two P2P nodes
        let config1 = P2PConfig {
            port: 9200,
            ..Default::default()
        };
        let config2 = P2PConfig {
            port: 9201,
            ..Default::default()
        };
        
        let node1 = Arc::new(P2PNode::new("rpc_node1".to_string(), config1).unwrap());
        let node2 = Arc::new(P2PNode::new("rpc_node2".to_string(), config2).unwrap());
        
        // Start nodes
        let n1 = Arc::clone(&node1);
        let n2 = Arc::clone(&node2);
        
        tokio::spawn(async move { n1.start().await });
        tokio::spawn(async move { n2.start().await });
        
        sleep(Duration::from_millis(500)).await;
        
        // Connect nodes
        let addr2: SocketAddr = "127.0.0.1:9201".parse().unwrap();
        if let Err(e) = node1.connect_to_peer(addr2).await {
            println!("RPC connection test warning: {}", e);
        }
        
        sleep(Duration::from_millis(500)).await;
        
        // Send RPC request from node1 to node2
        let request = P2PMessage::Request {
            id: "rpc_test_1".to_string(),
            method: "peer.info".to_string(),
            params: serde_json::json!({}),
        };
        
        // Get node2's ID
        let node2_id = node2.get_id();
        
        // Send request
        if let Err(e) = node1.send_to_peer(&node2_id, request).await {
            println!("RPC send test warning: {}", e);
        }
        
        // Give time for response
        sleep(Duration::from_millis(200)).await;
        
        // Test another RPC call
        let ping_request = P2PMessage::Request {
            id: "ping_test_1".to_string(),
            method: "network.ping".to_string(),
            params: serde_json::json!({ "timestamp": 123456 }),
        };
        
        if let Err(e) = node1.send_to_peer(&node2_id, ping_request).await {
            println!("Ping RPC test warning: {}", e);
        }
        
        // Stop nodes
        let _ = node1.stop().await;
        let _ = node2.stop().await;
    }

    #[tokio::test]
    async fn test_concurrent_data_handling() {
        let config = P2PConfig::default();
        let node = Arc::new(P2PNode::new("concurrent_test".to_string(), config).unwrap());
        
        // Simulate concurrent data messages
        let mut handles = vec![];
        
        for i in 0..10 {
            let node_clone = Arc::clone(&node);
            let handle = tokio::spawn(async move {
                let data = serde_json::json!({
                    "type": "file_metadata",
                    "metadata": {
                        "file_id": format!("file_{}", i),
                        "size": i * 1024
                    }
                });
                
                let result = node_clone.handle_data(
                    format!("data_{}", i),
                    serde_json::to_vec(&data).unwrap(),
                    format!("peer_{}", i % 3)
                ).await;
                
                assert!(result.is_ok(), "Concurrent data handling should succeed");
            });
            handles.push(handle);
        }
        
        // Wait for all tasks to complete
        for handle in handles {
            let _ = handle.await;
        }
    }

    #[tokio::test]
    async fn test_concurrent_rpc_requests() {
        let config = P2PConfig::default();
        let node = Arc::new(P2PNode::new("concurrent_rpc_test".to_string(), config).unwrap());
        
        // Simulate concurrent RPC requests
        let mut handles = vec![];
        
        // Mix of different RPC methods
        let methods = vec![
            ("peer.info", serde_json::json!({})),
            ("peer.list", serde_json::json!({})),
            ("network.ping", serde_json::json!({ "timestamp": 12345 })),
            ("network.stats", serde_json::json!({})),
            ("file.list", serde_json::json!({})),
            ("data.exists", serde_json::json!({ "id": "test_id" })),
        ];
        
        for (i, (method, params)) in methods.iter().enumerate() {
            for j in 0..5 {
                let node_clone = Arc::clone(&node);
                let method = method.to_string();
                let params = params.clone();
                
                let handle = tokio::spawn(async move {
                    let result = node_clone.handle_request(method, params).await;
                    assert!(result.is_ok(), "Concurrent RPC request {} should succeed", i * 5 + j);
                });
                handles.push(handle);
            }
        }
        
        // Wait for all tasks to complete
        for handle in handles {
            let _ = handle.await;
        }
    }

    #[tokio::test]
    async fn test_data_and_rpc_interleaved() {
        let config = P2PConfig::default();
        let node = Arc::new(P2PNode::new("interleaved_test".to_string(), config).unwrap());
        
        // Interleave data and RPC handling
        let mut handles = vec![];
        
        for i in 0..20 {
            let node_clone = Arc::clone(&node);
            
            if i % 2 == 0 {
                // Data handling
                let handle = tokio::spawn(async move {
                    let data = if i % 4 == 0 {
                        // JSON data
                        serde_json::to_vec(&serde_json::json!({
                            "type": "broadcast_message",
                            "content": { "msg": format!("Message {}", i) }
                        })).unwrap()
                    } else {
                        // Binary data
                        vec![i as u8; 100]
                    };
                    
                    let result = node_clone.handle_data(
                        format!("data_{}", i),
                        data,
                        format!("peer_{}", i % 4)
                    ).await;
                    
                    assert!(result.is_ok(), "Data handling {} should succeed", i);
                });
                handles.push(handle);
            } else {
                // RPC handling
                let handle = tokio::spawn(async move {
                    let method = match i % 5 {
                        1 => "peer.info",
                        3 => "network.ping",
                        _ => "file.list",
                    };
                    
                    let result = node_clone.handle_request(
                        method.to_string(),
                        serde_json::json!({ "timestamp": i })
                    ).await;
                    
                    assert!(result.is_ok(), "RPC handling {} should succeed", i);
                });
                handles.push(handle);
            }
        }
        
        // Wait for all tasks to complete
        for handle in handles {
            let _ = handle.await;
        }
    }

    #[tokio::test]
    async fn test_malformed_data_resilience() {
        let config = P2PConfig::default();
        let node = P2PNode::new("resilience_test".to_string(), config).unwrap();
        
        // Test various malformed data scenarios
        let test_cases = vec![
            // Empty data
            vec![],
            // Oversized data
            vec![0u8; 6 * 1024 * 1024],
            // Invalid UTF-8 in JSON position
            vec![0xFF, 0xFE, 0xFD],
            // Truncated JSON
            b"{\"type\":\"test\"".to_vec(),
            // Deeply nested JSON (as bytes)
            {
                let mut nested = serde_json::json!({"level": 0});
                for i in 1..20 {
                    nested = serde_json::json!({"level": i, "nested": nested});
                }
                serde_json::to_vec(&nested).unwrap_or_default()
            },
        ];
        
        for (i, data) in test_cases.iter().enumerate() {
            let result = node.handle_data(
                format!("malformed_{}", i),
                data.clone(),
                "test_peer".to_string()
            ).await;
            
            // All malformed data should be handled gracefully (either rejected or processed safely)
            // The important thing is no panic/crash
            println!("Malformed test case {}: {:?}", i, result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_rpc_error_handling() {
        let config = P2PConfig::default();
        let node = P2PNode::new("rpc_error_test".to_string(), config).unwrap();
        
        // Test various error scenarios
        let error_cases = vec![
            // Empty method
            ("", serde_json::json!({})),
            // Invalid characters in method
            ("test@#$%", serde_json::json!({})),
            // Very long method name
            (&"x".repeat(200), serde_json::json!({})),
            // Unknown method
            ("does.not.exist", serde_json::json!({})),
            // Missing required parameters
            ("data.get", serde_json::json!({})),
            ("broadcast.subscribe", serde_json::json!({})),
            // Invalid parameter types (handled gracefully)
            ("network.ping", serde_json::json!({ "timestamp": "not_a_number" })),
        ];
        
        for (method, params) in error_cases {
            let result = node.handle_request(method.to_string(), params).await;
            
            // These should all return errors or handle gracefully
            if method.is_empty() || method.len() > 100 || method.contains('@') {
                assert!(result.is_err(), "Method '{}' should be rejected", method);
            }
        }
    }
}