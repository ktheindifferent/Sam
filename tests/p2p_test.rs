#[cfg(test)]
mod p2p_tests {
    use sam::services::p2p::enhanced::{P2PNode, P2PMessage, MessageType};
    use sam::services::p2p::file_sharing::{FileTransfer, TransferStatus, ChunkMetadata};
    use sam::services::p2p::sync::{StateSync, StateUpdate, ConflictResolution};
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use std::collections::HashMap;
    use std::time::Duration;
    use tempfile::TempDir;
    use std::fs;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_p2p_node_creation() {
        let node = P2PNode::new("test_node".to_string(), 8080)
            .await
            .expect("Failed to create P2P node");
        
        assert_eq!(node.peer_id, "test_node");
        assert_eq!(node.port, 8080);
        assert!(node.peers.is_empty());
        assert!(node.is_running());
    }

    #[tokio::test]
    async fn test_peer_discovery() {
        let node1 = P2PNode::new("node1".to_string(), 8081)
            .await
            .expect("Failed to create node1");
        
        let node2 = P2PNode::new("node2".to_string(), 8082)
            .await
            .expect("Failed to create node2");
        
        node1.start_discovery().await.expect("Failed to start discovery");
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        node2.broadcast_presence().await.expect("Failed to broadcast presence");
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        assert!(node1.has_peer("node2"));
        assert!(node2.has_peer("node1"));
    }

    #[tokio::test]
    async fn test_message_routing() {
        let node1 = P2PNode::new("router1".to_string(), 8083)
            .await
            .expect("Failed to create router1");
        
        let node2 = P2PNode::new("router2".to_string(), 8084)
            .await
            .expect("Failed to create router2");
        
        node1.connect_to_peer("127.0.0.1:8084").await
            .expect("Failed to connect to peer");
        
        let test_message = P2PMessage {
            msg_type: MessageType::Data,
            sender: "router1".to_string(),
            recipient: Some("router2".to_string()),
            payload: b"test data".to_vec(),
            timestamp: chrono::Utc::now().timestamp() as u64,
            signature: vec![],
        };
        
        node1.send_message(test_message.clone()).await
            .expect("Failed to send message");
        
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let received = node2.get_received_messages().await;
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].payload, b"test data");
    }

    #[tokio::test]
    async fn test_cryptographic_signatures() {
        let node = P2PNode::new("crypto_node".to_string(), 8085)
            .await
            .expect("Failed to create crypto node");
        
        let message = P2PMessage {
            msg_type: MessageType::Handshake,
            sender: "crypto_node".to_string(),
            recipient: None,
            payload: b"secure data".to_vec(),
            timestamp: chrono::Utc::now().timestamp() as u64,
            signature: vec![],
        };
        
        let signed_message = node.sign_message(message)
            .expect("Failed to sign message");
        
        assert!(!signed_message.signature.is_empty());
        
        let is_valid = node.verify_signature(&signed_message)
            .expect("Failed to verify signature");
        
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_heartbeat_monitoring() {
        let node1 = P2PNode::new("heartbeat1".to_string(), 8086)
            .await
            .expect("Failed to create heartbeat1");
        
        let node2 = P2PNode::new("heartbeat2".to_string(), 8087)
            .await
            .expect("Failed to create heartbeat2");
        
        node1.connect_to_peer("127.0.0.1:8087").await
            .expect("Failed to connect");
        
        node1.start_heartbeat().await
            .expect("Failed to start heartbeat");
        
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        let health = node2.get_peer_health("heartbeat1")
            .expect("Failed to get peer health");
        
        assert!(health.is_healthy);
        assert!(health.last_heartbeat > 0);
    }

    #[tokio::test]
    async fn test_file_transfer_chunking() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let test_file = temp_dir.path().join("test.txt");
        let content = "a".repeat(10000);
        fs::write(&test_file, &content).expect("Failed to write test file");
        
        let transfer = FileTransfer::new(
            test_file.clone(),
            "sender".to_string(),
            "receiver".to_string(),
            1024
        ).expect("Failed to create transfer");
        
        assert_eq!(transfer.total_chunks, 10);
        assert_eq!(transfer.chunk_size, 1024);
        assert_eq!(transfer.status, TransferStatus::Pending);
        
        let chunk = transfer.get_chunk(0)
            .expect("Failed to get chunk");
        
        assert_eq!(chunk.index, 0);
        assert_eq!(chunk.data.len(), 1024);
    }

    #[tokio::test]
    async fn test_file_transfer_resume() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let test_file = temp_dir.path().join("resume_test.txt");
        let content = "b".repeat(5000);
        fs::write(&test_file, &content).expect("Failed to write test file");
        
        let mut transfer = FileTransfer::new(
            test_file.clone(),
            "sender".to_string(),
            "receiver".to_string(),
            1000
        ).expect("Failed to create transfer");
        
        transfer.mark_chunk_received(0);
        transfer.mark_chunk_received(1);
        transfer.mark_chunk_received(2);
        
        let missing = transfer.get_missing_chunks();
        assert_eq!(missing, vec![3, 4]);
        
        transfer.mark_chunk_received(3);
        transfer.mark_chunk_received(4);
        
        assert!(transfer.is_complete());
        assert_eq!(transfer.status, TransferStatus::Completed);
    }

    #[tokio::test]
    async fn test_bandwidth_limiting() {
        let node = P2PNode::new("bandwidth_node".to_string(), 8088)
            .await
            .expect("Failed to create bandwidth node");
        
        node.set_bandwidth_limit(1024 * 1024)
            .expect("Failed to set bandwidth limit");
        
        let current_limit = node.get_bandwidth_limit();
        assert_eq!(current_limit, 1024 * 1024);
        
        let large_data = vec![0u8; 2 * 1024 * 1024];
        let start = std::time::Instant::now();
        
        node.throttled_send(large_data).await
            .expect("Failed to send throttled data");
        
        let duration = start.elapsed();
        assert!(duration.as_secs() >= 2);
    }

    #[tokio::test]
    async fn test_state_synchronization() {
        let sync1 = StateSync::new("sync1".to_string());
        let sync2 = StateSync::new("sync2".to_string());
        
        sync1.update_state("key1", "value1").await;
        sync1.update_state("key2", "value2").await;
        
        let updates = sync1.get_updates_since(0).await;
        assert_eq!(updates.len(), 2);
        
        for update in updates {
            sync2.apply_update(update).await
                .expect("Failed to apply update");
        }
        
        assert_eq!(sync2.get_value("key1").await, Some("value1".to_string()));
        assert_eq!(sync2.get_value("key2").await, Some("value2".to_string()));
    }

    #[tokio::test]
    async fn test_conflict_resolution() {
        let sync1 = StateSync::new("conflict1".to_string());
        let sync2 = StateSync::new("conflict2".to_string());
        
        sync1.update_state("shared_key", "value_from_1").await;
        sync2.update_state("shared_key", "value_from_2").await;
        
        let update1 = StateUpdate {
            key: "shared_key".to_string(),
            value: "value_from_1".to_string(),
            timestamp: 100,
            node_id: "conflict1".to_string(),
        };
        
        let update2 = StateUpdate {
            key: "shared_key".to_string(),
            value: "value_from_2".to_string(),
            timestamp: 200,
            node_id: "conflict2".to_string(),
        };
        
        let resolved = ConflictResolution::resolve(update1, update2);
        assert_eq!(resolved.value, "value_from_2");
        assert_eq!(resolved.timestamp, 200);
    }

    #[tokio::test]
    async fn test_max_peer_connections() {
        let node = P2PNode::new("max_peers".to_string(), 8089)
            .await
            .expect("Failed to create node");
        
        node.set_max_peers(5).expect("Failed to set max peers");
        
        for i in 0..10 {
            let peer_id = format!("peer_{}", i);
            let result = node.add_peer(peer_id.clone(), format!("127.0.0.1:{}", 9000 + i)).await;
            
            if i < 5 {
                assert!(result.is_ok());
            } else {
                assert!(result.is_err());
            }
        }
        
        assert_eq!(node.get_peer_count(), 5);
    }

    #[tokio::test]
    async fn test_message_broadcasting() {
        let node1 = P2PNode::new("broadcast1".to_string(), 8090)
            .await
            .expect("Failed to create broadcast1");
        
        let node2 = P2PNode::new("broadcast2".to_string(), 8091)
            .await
            .expect("Failed to create broadcast2");
        
        let node3 = P2PNode::new("broadcast3".to_string(), 8092)
            .await
            .expect("Failed to create broadcast3");
        
        node1.connect_to_peer("127.0.0.1:8091").await.expect("Failed to connect");
        node1.connect_to_peer("127.0.0.1:8092").await.expect("Failed to connect");
        
        let broadcast_msg = P2PMessage {
            msg_type: MessageType::Broadcast,
            sender: "broadcast1".to_string(),
            recipient: None,
            payload: b"broadcast data".to_vec(),
            timestamp: chrono::Utc::now().timestamp() as u64,
            signature: vec![],
        };
        
        node1.broadcast(broadcast_msg).await
            .expect("Failed to broadcast");
        
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        let msgs2 = node2.get_received_messages().await;
        let msgs3 = node3.get_received_messages().await;
        
        assert_eq!(msgs2.len(), 1);
        assert_eq!(msgs3.len(), 1);
        assert_eq!(msgs2[0].payload, b"broadcast data");
        assert_eq!(msgs3[0].payload, b"broadcast data");
    }

    #[tokio::test]
    async fn test_peer_reconnection() {
        let node1 = P2PNode::new("reconnect1".to_string(), 8093)
            .await
            .expect("Failed to create reconnect1");
        
        let node2 = P2PNode::new("reconnect2".to_string(), 8094)
            .await
            .expect("Failed to create reconnect2");
        
        node1.connect_to_peer("127.0.0.1:8094").await
            .expect("Failed to connect");
        
        assert!(node1.has_peer("reconnect2"));
        
        node2.disconnect_peer("reconnect1").await
            .expect("Failed to disconnect");
        
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(!node1.has_peer("reconnect2"));
        
        node1.enable_auto_reconnect(Duration::from_millis(500)).await;
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        assert!(node1.has_peer("reconnect2"));
    }

    #[tokio::test]
    async fn test_data_integrity() {
        let node = P2PNode::new("integrity".to_string(), 8095)
            .await
            .expect("Failed to create integrity node");
        
        let original_data = b"important data that must not be corrupted";
        let checksum = node.calculate_checksum(original_data);
        
        let is_valid = node.verify_checksum(original_data, &checksum);
        assert!(is_valid);
        
        let corrupted_data = b"important data that must not be corruptex";
        let is_invalid = node.verify_checksum(corrupted_data, &checksum);
        assert!(!is_invalid);
    }

    #[tokio::test]
    async fn test_network_partition_handling() {
        let node1 = P2PNode::new("partition1".to_string(), 8096)
            .await
            .expect("Failed to create partition1");
        
        let node2 = P2PNode::new("partition2".to_string(), 8097)
            .await
            .expect("Failed to create partition2");
        
        let node3 = P2PNode::new("partition3".to_string(), 8098)
            .await
            .expect("Failed to create partition3");
        
        node1.connect_to_peer("127.0.0.1:8097").await.expect("Failed");
        node2.connect_to_peer("127.0.0.1:8098").await.expect("Failed");
        
        node1.simulate_network_partition(vec!["partition3".to_string()]).await;
        
        assert!(node1.has_peer("partition2"));
        assert!(!node1.can_reach("partition3"));
        assert!(node2.has_peer("partition3"));
    }

    #[test]
    fn test_message_serialization() {
        let message = P2PMessage {
            msg_type: MessageType::Data,
            sender: "test_sender".to_string(),
            recipient: Some("test_recipient".to_string()),
            payload: b"test payload".to_vec(),
            timestamp: 1234567890,
            signature: vec![1, 2, 3, 4],
        };
        
        let serialized = message.serialize()
            .expect("Failed to serialize message");
        
        let deserialized = P2PMessage::deserialize(&serialized)
            .expect("Failed to deserialize message");
        
        assert_eq!(deserialized.sender, message.sender);
        assert_eq!(deserialized.recipient, message.recipient);
        assert_eq!(deserialized.payload, message.payload);
        assert_eq!(deserialized.timestamp, message.timestamp);
    }

    #[tokio::test]
    async fn test_concurrent_file_transfers() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let node = P2PNode::new("file_node".to_string(), 8099)
            .await
            .expect("Failed to create file node");
        
        let mut handles = vec![];
        
        for i in 0..5 {
            let file_path = temp_dir.path().join(format!("file_{}.txt", i));
            let content = format!("content_{}", i).repeat(1000);
            fs::write(&file_path, &content).expect("Failed to write file");
            
            let node_clone = node.clone();
            let handle = tokio::spawn(async move {
                node_clone.send_file(
                    file_path,
                    format!("receiver_{}", i)
                ).await
            });
            
            handles.push(handle);
        }
        
        for handle in handles {
            let result = handle.await.expect("Task panicked");
            assert!(result.is_ok());
        }
        
        let active_transfers = node.get_active_transfers().await;
        assert_eq!(active_transfers.len(), 0);
    }
}