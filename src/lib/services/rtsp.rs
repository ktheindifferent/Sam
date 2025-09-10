// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use std::path::Path;
// use std::thread;
use std::sync::atomic::Ordering;
use tokio::sync::mpsc;
use crate::services::thread_manager::{self, ThreadConfig};

// Import deep learning and recording modules from services root
use crate::services::rtsp_dl_simple::{Alert, start_deep_learning_processor};
use crate::services::rtsp_recording::{
    RecordingManager, RecordingConfig, RecordingTrigger, VideoEncoding, 
    VideoCodec, AudioCodec, Resolution, create_recording_tables
};

pub fn init() {
    // Initialize RTSP Cameras
    // TODO - Customizable Port and Path
    
    let config = ThreadConfig {
        name: "rtsp_manager".to_string(),
        restart_on_panic: true,
        max_restarts: 5,
        restart_delay_ms: 5000,
        health_check_interval_ms: Some(30000),
        enable_monitoring: true,
        priority: crate::services::thread_manager::ThreadPriority::Normal,
        max_memory_mb: None,
        cpu_affinity: None,
    };
    
    thread_manager::spawn_with_config(config, move |shutdown_signal, _health_rx| {
        log::info!("RTSP manager thread started");
        
        let mut pg_query = crate::memory::PostgresQueries::default();
        pg_query
            .queries
            .push(crate::memory::PGCol::String("rtsp".to_string()));
        pg_query.query_columns.push("thing_type =".to_string());
        let rtsp_things = crate::memory::Thing::select(None, None, None, Some(pg_query));

        match rtsp_things {
            Ok(things) => {
                for thing in things {
                    if shutdown_signal.load(Ordering::Relaxed) {
                        log::info!("RTSP manager received shutdown signal");
                        break;
                    }
                    
                    // Convert RTSP to /streams http api
                    let rtsp_http_thing = thing.clone();
                    let http_config = ThreadConfig {
                        name: format!("rtsp_http_{}", thing.oid),
                        restart_on_panic: true,
                        max_restarts: 3,
                        restart_delay_ms: 2000,
                        health_check_interval_ms: Some(60000),
                        enable_monitoring: true,
                        priority: crate::services::thread_manager::ThreadPriority::Normal,
                        max_memory_mb: None,
                        cpu_affinity: None,
                    };
                    
                    thread_manager::spawn_with_config(http_config, move |shutdown, _health_rx| {
                        log::info!("Starting RTSP HTTP stream for {}", rtsp_http_thing.oid);
                        
                        while !shutdown.load(Ordering::Relaxed) {
                            let rtsp_address = format!(
                                "rtsp://{}:{}@{}:554/cam/realmonitor?channel=1&subtype=0",
                                rtsp_http_thing.username,
                                rtsp_http_thing.password,
                                rtsp_http_thing.ip_address
                            );
                            let script = crate::services::rtsp::gen_rtsp_to_http_stream_script(
                                rtsp_address,
                                rtsp_http_thing.oid.clone(),
                            );
                            
                            match std::panic::catch_unwind(|| {
                                crate::tools::uinx_cmd(&script)
                            }) {
                                Ok(result) => {
                                    log::debug!("RTSP HTTP stream command completed: {:?}", result);
                                }
                                Err(e) => {
                                    log::error!("RTSP HTTP stream command panicked: {:?}", e);
                                    break;
                                }
                            }
                            
                            // Check for shutdown every second while ffmpeg runs
                            std::thread::sleep(std::time::Duration::from_secs(1));
                        }
                        
                        log::info!("RTSP HTTP stream thread {} stopped", rtsp_http_thing.oid);
                    });

                    // Convert RTSP streams to wav files for sam to parse
                    let rtsp_wav_thing = thing.clone();
                    let wav_config = ThreadConfig {
                        name: format!("rtsp_wav_{}", thing.oid),
                        restart_on_panic: true,
                        max_restarts: 3,
                        restart_delay_ms: 2000,
                        health_check_interval_ms: Some(60000),
                        enable_monitoring: true,
                        priority: crate::services::thread_manager::ThreadPriority::Normal,
                        max_memory_mb: None,
                        cpu_affinity: None,
                    };
                    
                    thread_manager::spawn_with_config(wav_config, move |shutdown, _health_rx| {
                        log::info!("Starting RTSP WAV conversion for {}", rtsp_wav_thing.oid);
                        
                        while !shutdown.load(Ordering::Relaxed) {
                            let rtsp_address = format!(
                                "rtsp://{}:{}@{}:554/cam/realmonitor?channel=1&subtype=0",
                                rtsp_wav_thing.username,
                                rtsp_wav_thing.password,
                                rtsp_wav_thing.ip_address
                            );
                            let script = crate::services::rtsp::gen_rtsp_to_wav_script(
                                rtsp_address,
                                rtsp_wav_thing.oid.clone(),
                            );
                            
                            match std::panic::catch_unwind(|| {
                                crate::tools::uinx_cmd(&script)
                            }) {
                                Ok(result) => {
                                    log::debug!("RTSP WAV conversion command completed: {:?}", result);
                                }
                                Err(e) => {
                                    log::error!("RTSP WAV conversion command panicked: {:?}", e);
                                    break;
                                }
                            }
                            
                            // Check for shutdown every second while ffmpeg runs
                            std::thread::sleep(std::time::Duration::from_secs(1));
                        }
                        
                        log::info!("RTSP WAV conversion thread {} stopped", rtsp_wav_thing.oid);
                    });

                    // Perform Deep Learning on RTSP streams and log observations
                    let rtsp_dl_thing = thing.clone();
                    let dl_config = ThreadConfig {
                        name: format!("rtsp_dl_{}", rtsp_dl_thing.oid),
                        restart_on_panic: true,
                        max_restarts: 3,
                        restart_delay_ms: 2000,
                        health_check_interval_ms: Some(10000),
                        enable_monitoring: true,
                        priority: thread_manager::ThreadPriority::High,
                        max_memory_mb: None,
                        cpu_affinity: None,
                    };
                    thread_manager::spawn_with_config(dl_config, move |shutdown, _health_rx| {
                        // Create runtime for async operations
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        
                        rt.block_on(async {
                            // Create alert channel
                            let (alert_tx, mut alert_rx) = mpsc::channel::<Alert>(100);
                            
                            // Spawn alert handler
                            tokio::spawn(async move {
                                while let Some(alert) = alert_rx.recv().await {
                                    log::info!("RTSP Alert: {:?}", alert);
                                    // Here you could send notifications, update UI, etc.
                                }
                            });
                            
                            // Start deep learning processor
                            let rtsp_address = format!(
                                "rtsp://{}:{}@{}:554/cam/realmonitor?channel=1&subtype=0",
                                rtsp_dl_thing.username,
                                rtsp_dl_thing.password,
                                rtsp_dl_thing.ip_address
                            );
                            
                            if let Err(e) = start_deep_learning_processor(
                                rtsp_dl_thing.oid.clone(),
                                rtsp_address,
                                alert_tx,
                            ).await {
                                log::error!("Deep learning processor error for {}: {}", rtsp_dl_thing.oid, e);
                            }
                        });
                    });

                    // Record selected RTSP streams to a network location
                    let rtsp_rec_thing = thing.clone();
                    let rec_config = ThreadConfig {
                        name: format!("rtsp_rec_{}", rtsp_rec_thing.oid),
                        restart_on_panic: true,
                        max_restarts: 3,
                        restart_delay_ms: 2000,
                        health_check_interval_ms: Some(10000),
                        enable_monitoring: true,
                        priority: thread_manager::ThreadPriority::Normal,
                        max_memory_mb: None,
                        cpu_affinity: None,
                    };
                    thread_manager::spawn_with_config(rec_config, move |shutdown, _health_rx| {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        
                        rt.block_on(async {
                            // Create recording tables if not exists
                            if let Err(e) = create_recording_tables() {
                                log::error!("Failed to create recording tables: {}", e);
                                return;
                            }
                            
                            // Initialize recording manager
                            let storage_path = std::path::PathBuf::from(format!("/opt/sam/recordings/{}", rtsp_rec_thing.oid));
                            let mut recording_manager = match RecordingManager::new(storage_path.clone()) {
                                Ok(mgr) => mgr,
                                Err(e) => {
                                    log::error!("Failed to create recording manager: {}", e);
                                    return;
                                }
                            };
                            
                            // Configure recording for this camera
                            let rtsp_address = format!(
                                "rtsp://{}:{}@{}:554/cam/realmonitor?channel=1&subtype=0",
                                rtsp_rec_thing.username,
                                rtsp_rec_thing.password,
                                rtsp_rec_thing.ip_address
                            );
                            
                            let config = RecordingConfig {
                                thing_oid: rtsp_rec_thing.oid.clone(),
                                rtsp_url: rtsp_address,
                                storage_path,
                                network_storage: None, // Can be configured based on thing properties
                                encoding: VideoEncoding {
                                    codec: VideoCodec::H264,
                                    bitrate: 2000,
                                    fps: 25,
                                    resolution: Resolution { width: 1920, height: 1080 },
                                    audio_codec: AudioCodec::AAC,
                                    audio_bitrate: 128,
                                },
                                segment_duration: std::time::Duration::from_secs(3600), // 1 hour segments
                                retention_days: 30,
                                triggers: vec![
                                    RecordingTrigger::Motion,
                                    RecordingTrigger::Continuous,
                                ],
                                max_storage_gb: 100.0,
                            };
                            
                            if let Err(e) = recording_manager.add_camera(config) {
                                log::error!("Failed to add camera config: {}", e);
                                return;
                            }
                            
                            // Start checking triggers periodically
                            loop {
                                if let Err(e) = recording_manager.check_triggers().await {
                                    log::error!("Error checking recording triggers: {}", e);
                                }
                                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                            }
                        });
                    });
                }
            }
            Err(e) => {
                log::error!("Failed to query RTSP things: {}", e);
            }
        }
        
        log::info!("RTSP manager thread completed");
    });
}

pub fn gen_rtsp_to_http_stream_script(address: String, identifier: String) -> String {
    let mut script = "#!/bin/bash\n".to_string();
    script = format!("{script}VIDSOURCE=\"{address}\"\n");
    script = format!("{script}AUDIO_OPTS=\"-c:a aac -b:a 160000 -ac 2\"\n");
    script = format!("{script}VIDEO_OPTS=\"-s 854x480 -c:v libx264 -b:v 800000\"\n");
    script = format!("{script}OUTPUT_HLS=\"-hls_time 10 -hls_list_size 10 -start_number 1\"\n");
    script = format!("{script}ffmpeg -i \"$VIDSOURCE\" -y $AUDIO_OPTS $VIDEO_OPTS $OUTPUT_HLS /opt/sam/streams/{identifier}.m3u8");
    script
}

pub fn gen_rtsp_to_wav_script(address: String, identifier: String) -> String {
    let p = format!("/opt/sam/tmp/sound/{identifier}");
    if !Path::new(&p).exists() {
        crate::tools::uinx_cmd(&format!("mkdir -p {p}/s1 {p}/s2 {p}/s3")); // Fixed path creation
    }

    let mut script = "#!/bin/bash\n".to_string();
    script = format!("{script}VIDSOURCE=\"{address}\"\n");
    script = format!("{script}ffmpeg -i \"$VIDSOURCE\" -f segment -segment_time 1 -reset_timestamps 1 -strftime 1 -map 0:a /opt/sam/tmp/sound/{identifier}/s1/%Y%m%d-%H%M%S.wav");
    script
}
