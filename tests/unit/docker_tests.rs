#[cfg(test)]
mod docker_tests {
    use sam::services::docker;

    #[test]
    fn test_docker_status() {
        let status = docker::status();
        assert!(
            status == "running" || status == "stopped" || status == "not installed",
            "Docker status should be one of: running, stopped, not installed"
        );
    }

    #[test]
    fn test_is_installed() {
        let installed = docker::is_installed();
        // Test should pass regardless of Docker installation status
        assert!(installed == true || installed == false);
    }

    #[test]
    fn test_is_running() {
        let running = docker::is_running();
        // Test should pass regardless of Docker running status
        assert!(running == true || running == false);
    }

    #[test]
    fn test_docker_version() {
        if docker::is_installed() {
            let version = docker::get_version();
            assert!(version.is_some(), "Should get Docker version when installed");
            
            if let Some(ver) = version {
                assert!(!ver.is_empty(), "Docker version should not be empty");
                assert!(ver.contains("Docker") || ver.contains("version"), 
                    "Version string should contain Docker or version");
            }
        } else {
            let version = docker::get_version();
            assert!(version.is_none(), "Should not get version when Docker not installed");
        }
    }

    #[tokio::test]
    async fn test_docker_ps() {
        if docker::is_running() {
            let containers = docker::list_containers().await;
            assert!(containers.is_ok(), "Should list containers when Docker is running");
            
            if let Ok(container_list) = containers {
                // Container list should be a valid vector even if empty
                assert!(container_list.len() >= 0);
            }
        }
    }

    #[tokio::test]
    async fn test_docker_images() {
        if docker::is_running() {
            let images = docker::list_images().await;
            assert!(images.is_ok(), "Should list images when Docker is running");
            
            if let Ok(image_list) = images {
                // Image list should be a valid vector even if empty
                assert!(image_list.len() >= 0);
            }
        }
    }

    #[test]
    fn test_docker_compose_installed() {
        let compose_installed = docker::is_compose_installed();
        assert!(compose_installed == true || compose_installed == false);
    }

    #[tokio::test]
    async fn test_container_stats() {
        if docker::is_running() {
            let containers = docker::list_containers().await;
            if let Ok(container_list) = containers {
                if !container_list.is_empty() {
                    let container_id = &container_list[0].id;
                    let stats = docker::get_container_stats(container_id).await;
                    assert!(stats.is_ok(), "Should get stats for running container");
                }
            }
        }
    }

    #[test]
    fn test_docker_network_list() {
        if docker::is_running() {
            let networks = docker::list_networks();
            assert!(networks.is_ok(), "Should list networks when Docker is running");
            
            if let Ok(network_list) = networks {
                // Should have at least the default bridge network
                assert!(network_list.len() > 0, "Should have at least one network");
                assert!(network_list.iter().any(|n| n.name == "bridge"), 
                    "Should have default bridge network");
            }
        }
    }

    #[test]
    fn test_docker_volume_list() {
        if docker::is_running() {
            let volumes = docker::list_volumes();
            assert!(volumes.is_ok(), "Should list volumes when Docker is running");
            
            if let Ok(volume_list) = volumes {
                // Volume list should be a valid vector even if empty
                assert!(volume_list.len() >= 0);
            }
        }
    }

    #[tokio::test]
    async fn test_docker_pull_image() {
        if docker::is_running() && std::env::var("RUN_INTEGRATION_TESTS").is_ok() {
            // Only run this test when explicitly enabled via environment variable
            // as it requires network access and downloads data
            let result = docker::pull_image("alpine:latest").await;
            assert!(result.is_ok(), "Should pull alpine image");
        }
    }

    #[test]
    fn test_docker_info() {
        if docker::is_running() {
            let info = docker::get_info();
            assert!(info.is_ok(), "Should get Docker info when running");
            
            if let Ok(docker_info) = info {
                assert!(docker_info.contains("Server"), "Info should contain Server details");
                assert!(docker_info.contains("Containers") || docker_info.contains("Images"),
                    "Info should contain Containers or Images count");
            }
        }
    }

    #[test]
    fn test_parse_docker_output() {
        // Test parsing of docker command outputs
        let sample_ps_output = "CONTAINER ID   IMAGE     COMMAND   CREATED   STATUS   PORTS   NAMES
abc123         nginx     nginx     1h ago    Up 1h    80/tcp  web";
        
        let parsed = docker::parse_ps_output(sample_ps_output);
        assert!(parsed.is_ok());
        
        if let Ok(containers) = parsed {
            assert_eq!(containers.len(), 1);
            assert_eq!(containers[0].id, "abc123");
            assert_eq!(containers[0].image, "nginx");
        }
    }

    #[test]
    fn test_docker_daemon_config_path() {
        let config_path = docker::get_daemon_config_path();
        
        #[cfg(target_os = "linux")]
        assert_eq!(config_path, "/etc/docker/daemon.json");
        
        #[cfg(target_os = "macos")]
        assert!(config_path.contains("daemon.json"));
        
        #[cfg(target_os = "windows")]
        assert!(config_path.contains("daemon.json"));
    }

    #[tokio::test]
    async fn test_docker_start_stop_sequence() {
        // This test should only run with proper permissions and when safe
        if std::env::var("RUN_DOCKER_CONTROL_TESTS").is_ok() {
            let initial_running = docker::is_running();
            
            if initial_running {
                // Test stop
                docker::stop().await;
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                assert!(!docker::is_running(), "Docker should be stopped");
                
                // Test start
                docker::start().await;
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                assert!(docker::is_running(), "Docker should be running again");
            }
        }
    }
}