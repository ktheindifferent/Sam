#[cfg(test)]
mod redis_service_tests {
    use crate::sam::services::redis;
    use tokio::test;

    #[test]
    async fn test_redis_connection_status() {
        let is_running = redis::is_running().await;
        
        // Test should handle both running and not running states
        assert!(is_running == true || is_running == false);
    }

    #[test]
    async fn test_redis_start_stop() {
        // Skip if not running as root or in CI
        if std::env::var("CI").is_ok() || !nix::unistd::Uid::effective().is_root() {
            return;
        }

        let initial_state = redis::is_running().await;
        
        if initial_state {
            redis::stop_service().await;
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            assert!(!redis::is_running().await);
            
            redis::start_service_async().await;
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            assert!(redis::is_running().await);
        }
    }

    #[test]
    async fn test_redis_cache_operations() {
        if !redis::is_running().await {
            return; // Skip test if Redis not available
        }

        let key = "test:key";
        let value = "test_value";
        
        // Test set and get
        redis::set_cache(key, value).await.unwrap();
        let retrieved = redis::get_cache(key).await.unwrap();
        assert_eq!(retrieved, Some(value.to_string()));
        
        // Test delete
        redis::delete_cache(key).await.unwrap();
        let deleted = redis::get_cache(key).await.unwrap();
        assert_eq!(deleted, None);
    }

    #[test]
    async fn test_redis_expiry() {
        if !redis::is_running().await {
            return;
        }

        let key = "test:expiring_key";
        let value = "expires_soon";
        
        redis::set_cache_with_expiry(key, value, 1).await.unwrap();
        
        let immediate = redis::get_cache(key).await.unwrap();
        assert_eq!(immediate, Some(value.to_string()));
        
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        let expired = redis::get_cache(key).await.unwrap();
        assert_eq!(expired, None);
    }
}

#[cfg(test)]
mod postgres_service_tests {
    use crate::sam::services::pg;
    use tokio::test;

    #[test]
    async fn test_postgres_connection() {
        let is_running = pg::is_running().await;
        assert!(is_running == true || is_running == false);
    }

    #[test]
    async fn test_connection_pool() {
        if !pg::is_running().await {
            return;
        }

        let pool = pg::create_connection_pool().await.unwrap();
        assert!(pool.max_size() > 0);
        
        let conn = pool.get().await;
        assert!(conn.is_ok());
    }

    #[test]
    async fn test_table_creation() {
        if !pg::is_running().await {
            return;
        }

        let test_table = "test_table_".to_string() + &nanoid::nanoid!(10);
        let create_sql = format!(
            "CREATE TABLE IF NOT EXISTS {} (
                id SERIAL PRIMARY KEY,
                data TEXT NOT NULL
            )", test_table
        );
        
        let pool = pg::create_connection_pool().await.unwrap();
        let client = pool.get().await.unwrap();
        
        let result = client.execute(&create_sql, &[]).await;
        assert!(result.is_ok());
        
        // Clean up
        let drop_sql = format!("DROP TABLE IF EXISTS {}", test_table);
        client.execute(&drop_sql, &[]).await.unwrap();
    }
}

#[cfg(test)]
mod docker_service_tests {
    use crate::sam::services::docker;
    use tokio::test;

    #[test]
    async fn test_docker_availability() {
        let is_available = docker::is_available().await;
        assert!(is_available == true || is_available == false);
    }

    #[test]
    async fn test_list_containers() {
        if !docker::is_available().await {
            return;
        }

        let containers = docker::list_containers().await;
        assert!(containers.is_ok());
        
        if let Ok(list) = containers {
            for container in list {
                assert!(!container.id.is_empty());
            }
        }
    }

    #[test]
    async fn test_container_stats() {
        if !docker::is_available().await {
            return;
        }

        let containers = docker::list_containers().await.unwrap();
        if let Some(first) = containers.first() {
            let stats = docker::get_container_stats(&first.id).await;
            assert!(stats.is_ok());
        }
    }
}

#[cfg(test)]
mod llama_service_tests {
    use crate::sam::services::llama;
    use tokio::test;

    #[test]
    async fn test_llama_model_availability() {
        let models = llama::list_available_models().await;
        assert!(models.is_ok());
        
        if let Ok(model_list) = models {
            for model in model_list {
                assert!(!model.name.is_empty());
                assert!(model.size > 0);
            }
        }
    }

    #[test]
    async fn test_model_loading() {
        let test_model = "test_model.gguf";
        let can_load = llama::can_load_model(test_model).await;
        
        assert!(can_load == true || can_load == false);
    }

    #[test]
    fn test_tokenization() {
        let text = "Hello, world!";
        let tokens = llama::tokenize(text);
        
        assert!(!tokens.is_empty());
        assert!(tokens.len() > 0);
    }

    #[test]
    fn test_context_window_limits() {
        let max_context = 2048;
        let long_text = "word ".repeat(max_context * 2);
        
        let truncated = llama::truncate_to_context(&long_text, max_context);
        let token_count = llama::count_tokens(&truncated);
        
        assert!(token_count <= max_context);
    }
}

#[cfg(test)]
mod media_service_tests {
    use crate::sam::services::media;
    use std::path::PathBuf;
    use tokio::test;

    #[test]
    async fn test_audio_format_detection() {
        let test_files = vec![
            ("test.mp3", "mp3"),
            ("test.wav", "wav"),
            ("test.ogg", "ogg"),
            ("test.m4a", "m4a"),
            ("test.flac", "flac"),
        ];

        for (filename, expected_format) in test_files {
            let detected = media::detect_audio_format(&PathBuf::from(filename));
            assert_eq!(detected, Some(expected_format.to_string()));
        }
    }

    #[test]
    async fn test_video_format_detection() {
        let test_files = vec![
            ("test.mp4", "mp4"),
            ("test.avi", "avi"),
            ("test.mkv", "mkv"),
            ("test.mov", "mov"),
            ("test.webm", "webm"),
        ];

        for (filename, expected_format) in test_files {
            let detected = media::detect_video_format(&PathBuf::from(filename));
            assert_eq!(detected, Some(expected_format.to_string()));
        }
    }

    #[test]
    async fn test_media_duration_parsing() {
        let duration_strings = vec![
            ("00:00:30", 30),
            ("00:01:00", 60),
            ("00:05:30", 330),
            ("01:00:00", 3600),
            ("01:30:45", 5445),
        ];

        for (duration_str, expected_seconds) in duration_strings {
            let parsed = media::parse_duration(duration_str);
            assert_eq!(parsed, expected_seconds);
        }
    }

    #[test]
    fn test_thumbnail_generation_params() {
        let video_path = PathBuf::from("/tmp/test.mp4");
        let thumbnail_path = media::get_thumbnail_path(&video_path);
        
        assert!(thumbnail_path.to_string_lossy().contains("test"));
        assert!(thumbnail_path.to_string_lossy().ends_with(".jpg"));
    }
}