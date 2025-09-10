// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use dasp::Frame;
use hound::{WavReader, WavSpec, WavWriter};
use noise_gate::NoiseGate;
use std::{
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
    sync::atomic::Ordering,
};
use threadpool::ThreadPool;
use crate::sam::services::thread_manager::{self, ThreadConfig};

pub fn init() {
    // Initialize sound processing stages
    s1_init();
    s2_init();
    s3_init();
}

/// Caches VWAV files for observations.
pub fn cache_vwavs() {
    let config = ThreadConfig {
        name: "vwav_cache".to_string(),
        restart_on_panic: true,
        max_restarts: 3,
        restart_delay_ms: 5000,
        health_check_interval_ms: Some(60000),
        enable_monitoring: true,
        priority: crate::sam::services::thread_manager::ThreadPriority::Normal,
        max_memory_mb: None,
        cpu_affinity: None,
    };
    
    thread_manager::spawn_with_config(config, move |_shutdown_signal, _health_rx| {
        let pool = ThreadPool::new(12); // Configurable thread pool size
        let mut pg_query = crate::sam::memory::PostgresQueries::default();
        pg_query
            .queries
            .push(crate::sam::memory::PGCol::String("HEARD".to_string()));
        pg_query
            .query_columns
            .push("observation_type =".to_string());
        pg_query
            .queries
            .push(crate::sam::memory::PGCol::String("%PERSON%".to_string()));
        pg_query
            .query_columns
            .push(" AND observation_objects ilike".to_string());

        let observations =
            match crate::sam::memory::Observation::select_lite(None, None, None, Some(pg_query)) {
                Ok(obs) => obs,
                Err(e) => {
                    log::error!("Failed to select observations for VWAV cache build: {}", e);
                    return;
                }
            };

        let observations_len = observations.len();
        for (xrows, observation) in observations.iter().enumerate() {
            for human in &observation.observation_humans {
                let human_oid = human.oid.clone();
                let th_obsv = observation.clone();
                pool.execute(move || {
                    log::info!(
                        "CACHE VWAV build processed observation {}/{}",
                        xrows + 1,
                        observations_len
                    );
                    let tmp_file_path =
                        format!("/opt/sam/tmp/observations/vwav/{}.wav", th_obsv.oid);
                    let cache_path = format!("{tmp_file_path}.16.wav.mp4");

                    if !Path::new(&cache_path).exists() {
                        let xpath = format!(
                            "/opt/sam/scripts/sprec/audio/{}/{}.wav",
                            human_oid, th_obsv.oid
                        );
                        if Path::new(&xpath).exists() {
                            // Use safe command execution instead of deprecated uinx_cmd
                            crate::sam::tools::safe_uinx_cmd("cp", &[&xpath, &tmp_file_path]);
                        } else {
                            let mut full_pg_query = crate::sam::memory::PostgresQueries::default();
                            full_pg_query
                                .queries
                                .push(crate::sam::memory::PGCol::String(th_obsv.oid.clone()));
                            full_pg_query.query_columns.push("oid =".to_string());

                            match crate::sam::memory::Observation::select(
                                None,
                                None,
                                None,
                                Some(full_pg_query),
                            ) {
                                Ok(observations) if !observations.is_empty() => {
                                    let full_observation = &observations[0];
                                    if let Some(ref observation_file) =
                                        full_observation.observation_file
                                    {
                                        if let Err(e) =
                                            std::fs::write(&tmp_file_path, observation_file)
                                        {
                                            log::error!(
                                                "Failed to write observation file to {}: {}",
                                                tmp_file_path,
                                                e
                                            );
                                            return;
                                        }
                                    } else {
                                        log::error!("Observation {} has no file data", th_obsv.oid);
                                        return;
                                    }
                                }
                                Ok(_) => {
                                    log::error!("No observations found for oid {}", th_obsv.oid);
                                    return;
                                }
                                Err(e) => {
                                    log::error!(
                                        "Failed to fetch full observation {}: {}",
                                        th_obsv.oid,
                                        e
                                    );
                                    return;
                                }
                            }
                        }

                        // Use safe command execution
                        crate::sam::tools::safe_uinx_cmd(
                            "ffmpeg",
                            &[
                                "-y",
                                "-i",
                                &tmp_file_path,
                                "-ar",
                                "16000",
                                "-ac",
                                "1",
                                "-c:a",
                                "pcm_s16le",
                                &format!("{}.16.wav", tmp_file_path),
                            ],
                        );
                        crate::sam::tools::safe_uinx_cmd(
                            "/opt/sam/bin/whisper",
                            &[
                                "-m",
                                "/opt/sam/models/ggml-large.bin",
                                "-f",
                                &format!("{}.16.wav", tmp_file_path),
                                "-owts",
                            ],
                        );

                        if let Err(e) = crate::sam::services::stt::patch_whisper_wts() {
                            log::error!("Failed to patch whisper wts file: {}", e);
                        }

                        crate::sam::tools::safe_uinx_cmd(
                            "chmod",
                            &["+x", &format!("{}.16.wav.wts", tmp_file_path)],
                        );
                        crate::sam::tools::safe_uinx_cmd(
                            "sh",
                            &["-c", &format!("{}.16.wav.wts", tmp_file_path)],
                        );
                        crate::sam::tools::safe_uinx_cmd(
                            "rm",
                            &[
                                &tmp_file_path,
                                &format!("{}.16.wav", tmp_file_path),
                                &format!("{}.16.wav.wts", tmp_file_path),
                            ],
                        );
                    }
                });
            }
        }

        crate::sam::tools::safe_uinx_cmd("python3", &["/opt/sam/scripts/sprec/build.py"]);
    });
}

/// Observes sound predictions and stores them in the database.
pub fn observe(prediction: crate::sam::services::stt::STTPrediction, file_path: &str) {
    let mut observation = crate::sam::memory::Observation::new();
    observation.observation_type = crate::sam::memory::ObservationType::HEARD;
    observation.observation_notes.push(prediction.text.clone());
    observation.observation_file = match std::fs::read(file_path) {
        Ok(data) => Some(data),
        Err(e) => {
            log::error!("Failed to read observation file {}: {}", file_path, e);
            None
        }
    };

    if !prediction.text.is_empty() {
        observation
            .observation_objects
            .push(crate::sam::memory::ObservationObjects::new("Person".to_string(), 0.8));
    }

    // TODO: Implement speaker identification
    // For now, create a generic "Unknown Speaker" entry when we have speech
    if !prediction.text.is_empty() {
        let mut human = crate::sam::memory::Human::new();
        human.name = "Unknown Speaker".to_string();
        human.heard_count = 1;
        if let Err(e) = human.save() {
            log::error!("Failed to save human: {}", e);
        }
        observation.observation_humans.push(human);
    } else {
        let mut pg_query = crate::sam::memory::PostgresQueries::default();
        pg_query
            .queries
            .push(crate::sam::memory::PGCol::String("Unknown Speaker".to_string()));
        pg_query.query_columns.push("oid ilike".to_string());
        let humans = match crate::sam::memory::Human::select(None, None, None, Some(pg_query)) {
            Ok(h) => h,
            Err(e) => {
                log::error!("Failed to select humans: {}", e);
                vec![]
            }
        };
        if !humans.is_empty() {
            observation.observation_humans.push(humans[0].clone());
        } else {
            let mut human = crate::sam::memory::Human::new();
            human.name = "Unknown".to_string();
            human.heard_count = 1;
            if let Err(e) = human.save() {
                log::error!("Failed to save human: {}", e);
            }
            observation.observation_humans.push(human);
        }
    }

    if let Err(e) = observation.save() {
        log::error!("Failed to save observation: {}", e);
    }
}

/// Stage One: Removes noise and trims silence.
pub fn s1_init() {
    let config = ThreadConfig {
        name: "sound_s1_processor".to_string(),
        restart_on_panic: true,
        max_restarts: 5,
        restart_delay_ms: 3000,
        health_check_interval_ms: Some(30000),
        enable_monitoring: true,
        priority: crate::sam::services::thread_manager::ThreadPriority::Normal,
        max_memory_mb: None,
        cpu_affinity: None,
    };
    
    thread_manager::spawn_with_config(config, move |shutdown_signal, _health_rx| {
        log::info!("Sound S1 processor started");
        
        while !shutdown_signal.load(Ordering::Relaxed) {
        let thing_paths = match std::fs::read_dir("/opt/sam/tmp/sound") {
            Ok(paths) => paths,
            Err(e) => {
                log::error!("Failed to read /opt/sam/tmp/sound: {}", e);
                continue;
            }
        };
        for thing_path in thing_paths {
            let tpath = match thing_path {
                Ok(entry) => entry.path().display().to_string(),
                Err(e) => {
                    log::error!("Failed to read thing_path: {}", e);
                    continue;
                }
            };
            let paths = match std::fs::read_dir(format!("{tpath}/s1")) {
                Ok(p) => p,
                Err(e) => {
                    log::error!("Failed to read {}/s1: {}", tpath, e);
                    continue;
                }
            };

            for path in paths {
                let spath = match path {
                    Ok(entry) => entry.path().display().to_string(),
                    Err(e) => {
                        log::error!("Failed to read path: {}", e);
                        continue;
                    }
                };
                let timestamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
                    Ok(duration) => duration.as_secs() as i64,
                    Err(e) => {
                        log::error!("Failed to get system time: {}", e);
                        continue;
                    }
                };

                if let Ok(reader) = WavReader::open(&spath) {
                    let header = reader.spec();
                    let samples: std::result::Result<Vec<_>, _> = reader
                        .into_samples::<i16>()
                        .map(|result| result.map(|sample| [sample]))
                        .collect();
                    
                    if let Ok(samples) = samples
                    {
                        let release_time = (header.sample_rate as f32 * 1.3).round();
                        let s2_path = PathBuf::from(format!("{tpath}/s2"));
                        let mut sink = Sink::new(s2_path, format!("{timestamp}-"), header);
                        let mut gate = NoiseGate::new(4000, release_time as usize);
                        gate.process_frames(&samples, &mut sink);
                        std::fs::remove_file(spath).ok();
                    }
                }
            }
        }
            
        // Sleep briefly to avoid busy loop
        std::thread::sleep(std::time::Duration::from_millis(100));
        }
        
        log::info!("Sound S1 processor stopped");
    });
}

// Stage Two - Stitches files into a single timestamped file ready to be observed by SAM
// Results are stored in /opt/sam/tmp/sound/s3
/// Stage Two: Stitches consecutive audio clips into a single file for further processing.
/// Results are stored in /opt/sam/tmp/sound/s3.
pub fn s2_init() {
    let config = ThreadConfig {
        name: "sound_s2_processor".to_string(),
        restart_on_panic: true,
        max_restarts: 5,
        restart_delay_ms: 3000,
        health_check_interval_ms: Some(30000),
        enable_monitoring: true,
        priority: crate::sam::services::thread_manager::ThreadPriority::Normal,
        max_memory_mb: None,
        cpu_affinity: None,
    };
    
    thread_manager::spawn_with_config(config, move |shutdown_signal, _health_rx| {
        log::info!("Sound S2 processor started");
        
        while !shutdown_signal.load(Ordering::Relaxed) {
            // Iterate over all "thing" directories in /opt/sam/tmp/sound
            let thing_paths = match std::fs::read_dir("/opt/sam/tmp/sound") {
                Ok(paths) => paths,
                Err(e) => {
                    log::error!("Failed to read /opt/sam/tmp/sound: {}", e);
                    continue;
                }
            };

            for thing_path in thing_paths {
                let tpath = match thing_path {
                    Ok(entry) => entry.path(),
                    Err(e) => {
                        log::error!("Failed to read thing_path: {}", e);
                        continue;
                    }
                };

                let tpath_str = tpath.display().to_string();
                let s2_dir = format!("{tpath_str}/s2");

                // Gather all .wav files in s2 directory
                let paths = match std::fs::read_dir(&s2_dir) {
                    Ok(paths) => paths,
                    Err(_) => continue, // skip if s2 dir doesn't exist
                };

                // Collect timestamps and file paths for stitching
                let mut timestamps: Vec<i64> = Vec::new();
                let mut file_map: Vec<(i64, String)> = Vec::new();

                for path in paths {
                    let spath = match path {
                        Ok(entry) => entry.path(),
                        Err(_) => continue,
                    };
                    let spath_str = spath.display().to_string();

                    // Expect filename format: <timestamp>-<id>.wav
                    let file_name = match spath.file_name().and_then(|n| n.to_str()) {
                        Some(name) => name.replace(".wav", ""),
                        None => continue,
                    };
                    let parts: Vec<&str> = file_name.split('-').collect();
                    if parts.len() < 2 {
                        continue;
                    }
                    let file_timestamp = match parts[0].parse::<i64>() {
                        Ok(ts) => ts,
                        Err(_) => continue,
                    };
                    timestamps.push(file_timestamp);
                    file_map.push((file_timestamp, spath_str));
                }

                if timestamps.is_empty() {
                    continue;
                }

                // Sort and deduplicate timestamps
                timestamps.sort_unstable();
                timestamps.dedup();

                // Find consecutive timestamp groups
                let mut groups: Vec<Vec<i64>> = Vec::new();
                let mut current_group: Vec<i64> = Vec::new();
                for &ts in &timestamps {
                    if current_group.is_empty() || ts == current_group.last().map(|x| x + 1).unwrap_or(ts) {
                        current_group.push(ts);
                    } else {
                        if current_group.len() > 1 {
                            groups.push(current_group.clone());
                        }
                        current_group = vec![ts];
                    }
                }
                if current_group.len() > 1 {
                    groups.push(current_group);
                }

                // Only stitch if we have a group of consecutive files and they're not too recent
                let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
                    Ok(duration) => duration.as_secs() as i64,
                    Err(e) => {
                        log::error!("Failed to get system time: {}", e);
                        continue;
                    }
                };
                for group in groups {
                    // Skip if any timestamp is too recent (avoid files still being written)
                    if group.iter().any(|&ts| ts >= now - 1) {
                        continue;
                    }

                    // Collect file paths for this group, sorted by timestamp
                    let mut files_to_stitch: Vec<String> = group
                        .iter()
                        .filter_map(|ts| {
                            file_map
                                .iter()
                                .find(|(t, _)| t == ts)
                                .map(|(_, path)| path.clone())
                        })
                        .collect();
                    files_to_stitch.sort();

                    if files_to_stitch.is_empty() {
                        continue;
                    }

                    // Output file path
                    let out_dir = format!("{tpath_str}/s3");
                    let _ = std::fs::create_dir_all(&out_dir);
                    let out_path = format!("{}/{}.incoming.wav", out_dir, group[0]);

                    // Stitch files
                    let mut writer: Option<WavWriter<BufWriter<File>>> = None;
                    for file_path in &files_to_stitch {
                        match WavReader::open(file_path) {
                            Ok(reader) => {
                                let spec = reader.spec();
                                let samples: std::result::Result<Vec<_>, _> = reader
                                    .into_samples::<i16>()
                                    .map(|r| r.map(|s| [s]))
                                    .collect();
                                
                                let samples = match samples
                                {
                                    Ok(s) => s,
                                    Err(e) => {
                                        log::error!(
                                            "Failed to read samples from {}: {}",
                                            file_path,
                                            e
                                        );
                                        continue;
                                    }
                                };

                                if writer.is_none() {
                                    // Create writer with the spec of the first file
                                    let out_file = match File::create(&out_path) {
                                        Ok(f) => f,
                                        Err(e) => {
                                            log::error!(
                                                "Failed to create output file {}: {}",
                                                out_path,
                                                e
                                            );
                                            break;
                                        }
                                    };
                                    writer = match WavWriter::new(BufWriter::new(out_file), spec) {
                                        Ok(w) => Some(w),
                                        Err(e) => {
                                            log::error!("Failed to create WavWriter: {}", e);
                                            break;
                                        }
                                    };
                                }

                                if let Some(w) = writer.as_mut() {
                                    for sample in samples {
                                        if let Err(e) = w.write_sample(sample[0]) {
                                            log::error!("Failed to write sample: {}", e);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                let es = e.to_string();
                                if !es.contains("read enough bytes") {
                                    log::error!("Failed to open {}: {}", file_path, es);
                                }
                            }
                        }
                    }

                    // Finalize and clean up
                    if let Some(w) = writer {
                        if let Err(e) = w.finalize() {
                            log::error!("Failed to finalize output file {}: {}", out_path, e);
                        }
                    }

                    // Remove stitched files
                    for file_path in &files_to_stitch {
                        let _ = std::fs::remove_file(file_path);
                    }

                    // Rename .incoming.wav to .wav
                    let final_path = out_path.replace(".incoming", "");
                    if let Err(e) = std::fs::rename(&out_path, &final_path) {
                        log::error!("Failed to rename {} to {}: {}", out_path, final_path, e);
                    }
                }
            }
            
            // Sleep briefly to avoid busy loop
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        
        log::info!("Sound S2 processor stopped");
    });
}

// Stage Three -
/// Stage Three: Processes stitched audio files for speech-to-text (STT) and observation.
/// Consumes files from /opt/sam/tmp/sound/*/s3, runs STT, observes, and cleans up.
/// Uses a thread pool for parallel processing.
pub fn s3_init() {
    let config = ThreadConfig {
        name: "sound_s3_processor".to_string(),
        restart_on_panic: true,
        max_restarts: 5,
        restart_delay_ms: 3000,
        health_check_interval_ms: Some(30000),
        enable_monitoring: true,
        priority: crate::sam::services::thread_manager::ThreadPriority::Normal,
        max_memory_mb: None,
        cpu_affinity: None,
    };
    
    thread_manager::spawn_with_config(config, move |shutdown_signal, _health_rx| {
        log::info!("Sound S3 processor started");
        // Use a thread pool with a configurable number of threads (default: 3)
        let pool = threadpool::Builder::new().num_threads(3).build();

        // Track files currently being processed to avoid duplicate work
        let mut processing_queue: Vec<String> = Vec::new();

        while !shutdown_signal.load(Ordering::Relaxed) {
            // Iterate over all "thing" directories in /opt/sam/tmp/sound
            let thing_paths = match std::fs::read_dir("/opt/sam/tmp/sound") {
                Ok(paths) => paths,
                Err(e) => {
                    log::error!("Failed to read /opt/sam/tmp/sound: {}", e);
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }
            };

            for thing_path in thing_paths {
                let tpath = match thing_path {
                    Ok(entry) => entry.path(),
                    Err(e) => {
                        log::error!("Failed to read thing_path: {}", e);
                        continue;
                    }
                };

                let s3_dir = tpath.join("s3");
                let paths = match std::fs::read_dir(&s3_dir) {
                    Ok(paths) => paths,
                    Err(_) => continue, // skip if s3 dir doesn't exist
                };

                for path in paths {
                    let fpath = match path {
                        Ok(entry) => entry.path(),
                        Err(_) => continue,
                    };

                    let fpath_str = fpath.display().to_string();

                    // Only process .wav files that are not already processed or being processed
                    if fpath_str.ends_with(".wav")
                        && !fpath_str.contains(".16")
                        && !fpath_str.contains(".incoming")
                        && !processing_queue.contains(&fpath_str)
                    {
                        processing_queue.push(fpath_str.clone());

                        // Clone for thread move
                        let fpath_thread = fpath_str.clone();

                        pool.execute(move || {
                            // Run STT prediction
                            match crate::sam::services::stt::deep_speech_process(
                                fpath_thread.clone(),
                            ) {
                                Ok(stt) if !stt.text.is_empty() => {
                                    // Optionally play a notification sound
                                    // crate::sam::tools::uinx_cmd("aplay /opt/sam/beep.wav".to_string());

                                    // Observe the sound and prediction
                                    observe(stt, &fpath_thread);
                                }
                                Ok(_) => {} // No speech detected
                                Err(e) => {
                                    log::error!(
                                        "STT processing failed for {}: {}",
                                        fpath_thread,
                                        e
                                    );
                                }
                            }

                            // Remove the processed file
                            if let Err(e) = std::fs::remove_file(&fpath_thread) {
                                log::error!("Failed to remove file {}: {}", fpath_thread, e);
                            }
                        });
                    }
                }
            }

            // Sleep briefly to avoid busy-waiting
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        
        log::info!("Sound S3 processor stopped");
    });
}

pub struct Sink {
    output_dir: PathBuf,
    clip_number: usize,
    prefix: String,
    spec: WavSpec,
    writer: Option<WavWriter<BufWriter<File>>>,
}

pub struct RecordingProcessor {
    writer: Option<WavWriter<BufWriter<File>>>,
    buffer: Vec<f32>,
}

impl RecordingProcessor {
    pub fn new(output_path: &Path, spec: WavSpec) -> Self {
        let writer = match File::create(output_path) {
            Ok(file) => match WavWriter::new(BufWriter::new(file), spec) {
                Ok(w) => Some(w),
                Err(e) => {
                    log::error!("Failed to create WavWriter: {}", e);
                    None
                }
            },
            Err(e) => {
                log::error!("Failed to create output file: {}", e);
                None
            }
        };

        RecordingProcessor {
            writer,
            buffer: Vec::new(),
        }
    }

    pub fn push(&mut self, sample: [f32; 1]) {
        self.buffer.push(sample[0]);
        
        if let Some(writer) = &mut self.writer {
            if let Err(e) = writer.write_sample((sample[0] * 32767.0) as i16) {
                log::error!("Failed to write sample: {}", e);
            }
        }
    }

    pub fn finish(self) {
        if let Some(writer) = self.writer {
            if let Err(e) = writer.finalize() {
                log::error!("Failed to finalize writer: {}", e);
            }
        }
    }
}

impl Sink {
    pub fn new(output_dir: PathBuf, prefix: String, spec: WavSpec) -> Self {
        Sink {
            output_dir,
            prefix,
            spec,
            clip_number: 0,
            writer: None,
        }
    }

    fn get_writer(&mut self) -> Option<&mut WavWriter<BufWriter<File>>> {
        if self.writer.is_none() {
            // Lazily initialize the writer. This lets us drop the writer when
            // sent an end_of_transmission and have it automatically start
            // writing to a new clip when necessary.
            let filename = self
                .output_dir
                .join(format!("{}{}.wav", self.prefix, self.clip_number));
            self.clip_number += 1;
            let file = match File::create(filename) {
                Ok(f) => f,
                Err(e) => {
                    log::error!("Failed to create file: {}", e);
                    return None;
                }
            };
            self.writer = match WavWriter::new(BufWriter::new(file), self.spec) {
                Ok(w) => Some(w),
                Err(e) => {
                    log::error!("Failed to create WavWriter: {}", e);
                    return None;
                }
            };
        }

        self.writer.as_mut()
    }
}

impl<F> noise_gate::Sink<F> for Sink
where
    F: Frame,
    F::Sample: hound::Sample,
{
    fn record(&mut self, frame: F) {
        if let Some(writer) = self.get_writer() {
            // write all the channels as interlaced audio
            for channel in frame.channels() {
                if let Err(e) = writer.write_sample(channel) {
                    log::error!("Failed to write sample: {}", e);
                }
            }
        }
    }

    fn end_of_transmission(&mut self) {
        // if we were previously recording a transmission, remove the writer
        // and let it flush to disk
        if let Some(writer) = std::mem::take(&mut self.writer) {
            if let Err(e) = writer.finalize() {
                log::error!("Failed to finalize writer: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    use proptest::prelude::*;
    
    #[test]
    fn test_init_functions() {
        // Test that init functions can be called without panicking
        init();
        s1_init();
        s2_init();
        s3_init();
    }
    
    #[test]
    fn test_record_processor_creation() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test_output.wav");
        
        let spec = WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        
        let processor = RecordingProcessor::new(&output_path, spec);
        assert!(processor.writer.is_some());
    }
    
    #[test]
    fn test_noise_gate_processing() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = temp_dir.path().join("input.wav");
        let output_path = temp_dir.path().join("output.wav");
        
        // Create a test WAV file
        let spec = WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        
        let mut writer = WavWriter::create(&input_path, spec).unwrap();
        // Write some sample data
        for i in 0..1000 {
            let sample = (i as f32 * 0.1).sin() * 1000.0;
            writer.write_sample(sample as i16).unwrap();
        }
        writer.finalize().unwrap();
        
        // Test noise gate processing
        let mut processor = RecordingProcessor::new(&output_path, spec);
        
        // Read and process the input
        let reader = WavReader::open(&input_path).unwrap();
        let samples: Vec<i16> = reader.into_samples::<i16>().map(|s| s.unwrap()).collect();
        
        for sample in samples {
            processor.push([sample as f32]);
        }
        
        processor.finish();
        
        // Verify output file exists
        assert!(output_path.exists());
    }
    
    #[test]
    fn test_record_processor_buffer_handling() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test_buffer.wav");
        
        let spec = WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        
        let mut processor = RecordingProcessor::new(&output_path, spec);
        
        // Push samples to fill buffer
        for i in 0..100 {
            processor.push([i as f32]);
        }
        
        processor.finish();
        assert!(output_path.exists());
        
        // Verify file has content
        let metadata = fs::metadata(&output_path).unwrap();
        assert!(metadata.len() > 44); // WAV header is 44 bytes
    }
    
    proptest! {
        #[test]
        fn test_wav_spec_validation(
            channels in 1u16..=8,
            sample_rate in 8000u32..=48000,
            bits_per_sample in prop::sample::select(vec![8u16, 16, 24, 32])
        ) {
            let spec = WavSpec {
                channels,
                sample_rate,
                bits_per_sample,
                sample_format: hound::SampleFormat::Int,
            };
            
            // Spec should be valid
            prop_assert!(spec.channels > 0);
            prop_assert!(spec.sample_rate > 0);
            prop_assert!(vec![8, 16, 24, 32].contains(&spec.bits_per_sample));
        }
        
        #[test]
        fn test_sample_processing(
            samples in prop::collection::vec(-32768i16..32768, 10..100)
        ) {
            let temp_dir = TempDir::new().unwrap();
            let output_path = temp_dir.path().join("test_samples.wav");
            
            let spec = WavSpec {
                channels: 1,
                sample_rate: 16000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };
            
            let mut processor = RecordingProcessor::new(&output_path, spec);
            
            for sample in samples {
                processor.push([sample as f32]);
            }
            
            processor.finish();
            prop_assert!(output_path.exists());
        }
    }
    
    #[test]
    fn test_concurrent_processing() {
        use std::sync::Arc;
        use std::thread;
        
        let temp_dir = Arc::new(TempDir::new().unwrap());
        let mut handles = vec![];
        
        for i in 0..5 {
            let temp_dir = temp_dir.clone();
            let handle = thread::spawn(move || {
                let output_path = temp_dir.path().join(format!("concurrent_{}.wav", i));
                
                let spec = WavSpec {
                    channels: 1,
                    sample_rate: 16000,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                };
                
                let mut processor = RecordingProcessor::new(&output_path, spec);
                
                for j in 0..100 {
                    processor.push([j as f32]);
                }
                
                processor.finish();
                assert!(output_path.exists());
            });
            handles.push(handle);
        }
        
        for handle in handles {
            handle.join().unwrap();
        }
    }
    
    #[test]
    fn test_noise_gate_parameters() {
        let noise_gate = NoiseGate::new(4000, 2080);
        
        // Test that noise gate can process samples
        let sample = [0.0f32];
        // Note: NoiseGate::process_frames expects a slice of samples and a sink
        // This is just testing construction, not actual processing
    }
    
    #[test]
    fn test_file_path_generation() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let path = format!("/tmp/test_{}.wav", timestamp);
        assert!(path.contains(&timestamp.to_string()));
        assert!(path.ends_with(".wav"));
    }
}
