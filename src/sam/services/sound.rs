// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (PixelCoda)
// Licensed under GPLv3....see LICENSE file.

use dasp::Frame;
use hound::{WavReader, WavSpec, WavWriter};
use noise_gate::NoiseGate;
use std::{
    fs::{File},
    io::BufWriter,
    path::{Path, PathBuf},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};
use threadpool::ThreadPool;

pub fn init() {
    // Initialize sound processing stages
    s1_init();
    s2_init();
    s3_init();
}

/// Caches VWAV files for observations.
pub fn cache_vwavs() {
    thread::spawn(move || {
        let pool = ThreadPool::new(12); // Configurable thread pool size
        let mut pg_query = crate::sam::memory::PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String("HEARD".to_string()));
        pg_query.query_coulmns.push("observation_type =".to_string());
        pg_query.queries.push(crate::sam::memory::PGCol::String("%PERSON%".to_string()));
        pg_query.query_coulmns.push(" AND observation_objects ilike".to_string());

        let observations = crate::sam::memory::Observation::select_lite(None, None, None, Some(pg_query)).unwrap();

        for (xrows, observation) in observations.iter().enumerate() {
            for human in &observation.observation_humans {
                let th_obsv = observation.clone();
                pool.execute(move || {
                    log::info!("CACHE VWAV build processed observation {}/{}", xrows + 1, observations.len());
                    let tmp_file_path = format!("/opt/sam/tmp/observations/vwav/{}.wav", th_obsv.oid);
                    let cache_path = format!("{}.16.wav.mp4", tmp_file_path);

                    if !Path::new(&cache_path).exists() {
                        let xpath = format!("/opt/sam/scripts/sprec/audio/{}/{}.wav", human.oid, th_obsv.oid);
                        if Path::new(&xpath).exists() {
                            crate::sam::tools::uinx_cmd(format!("cp {} {}", xpath, tmp_file_path));
                        } else {
                            let mut full_pg_query = crate::sam::memory::PostgresQueries::default();
                            full_pg_query.queries.push(crate::sam::memory::PGCol::String(th_obsv.oid.clone()));
                            full_pg_query.query_coulmns.push("oid =".to_string());
                            let full_observation = crate::sam::memory::Observation::select(None, None, None, Some(full_pg_query)).unwrap()[0].clone();
                            std::fs::write(&tmp_file_path, full_observation.observation_file.unwrap()).unwrap();
                        }

                        crate::sam::tools::uinx_cmd(format!("ffmpeg -y -i {} -ar 16000 -ac 1 -c:a pcm_s16le {}.16.wav", tmp_file_path, tmp_file_path));
                        crate::sam::tools::uinx_cmd(format!("/opt/sam/bin/whisper -m /opt/sam/models/ggml-large.bin -f {}.16.wav -owts", tmp_file_path));
                        crate::sam::services::stt::patch_whisper_wts(format!("{}.16.wav.wts", tmp_file_path)).unwrap();
                        crate::sam::tools::uinx_cmd(format!("chmod +x {}.16.wav.wts", tmp_file_path));
                        crate::sam::tools::uinx_cmd(format!("{}.16.wav.wts", tmp_file_path));
                        crate::sam::tools::uinx_cmd(format!("rm {} {}.16.wav {}.16.wav.wts", tmp_file_path, tmp_file_path, tmp_file_path));
                    }
                });
            }
        }

        crate::sam::tools::uinx_cmd("python3 /opt/sam/scripts/sprec/build.py".to_string());
    });
}

/// Observes sound predictions and stores them in the database.
pub fn observe(prediction: crate::sam::services::stt::STTPrediction, file_path: &str) {
    let mut observation = crate::sam::memory::Observation::new();
    observation.observation_type = crate::sam::memory::ObservationType::HEARD;
    observation.observation_notes.push(prediction.stt.clone());
    observation.observation_file = Some(std::fs::read(file_path).unwrap());

    if !prediction.stt.is_empty() {
        observation.observation_objects.push(crate::sam::memory::ObservationObjects::PERSON);
    }

    if prediction.human.contains("Unknown") {
        let mut human = crate::sam::memory::Human::new();
        human.name = prediction.human.clone();
        human.heard_count = 1;
        human.save().unwrap();
        observation.observation_humans.push(human);
    } else {
        let mut pg_query = crate::sam::memory::PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String(prediction.human.clone()));
        pg_query.query_coulmns.push("oid ilike".to_string());
        let humans = crate::sam::memory::Human::select(None, None, None, Some(pg_query)).unwrap();
        if !humans.is_empty() {
            observation.observation_humans.push(humans[0].clone());
        } else {
            let mut human = crate::sam::memory::Human::new();
            human.name = "Unknown".to_string();
            human.heard_count = 1;
            human.save().unwrap();
            observation.observation_humans.push(human);
        }
    }

    observation.save().unwrap();
}

/// Stage One: Removes noise and trims silence.
pub fn s1_init() {
    thread::spawn(move || {
        loop {
            let thing_paths = std::fs::read_dir("/opt/sam/tmp/sound").unwrap();
            for thing_path in thing_paths {
                let tpath = thing_path.unwrap().path().display().to_string();
                let paths = std::fs::read_dir(format!("{}/s1", tpath)).unwrap();

                for path in paths {
                    let spath = path.unwrap().path().display().to_string();
                    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;

                    if let Ok(reader) = WavReader::open(&spath) {
                        let header = reader.spec();
                        if let Ok(samples) = reader.into_samples::<i16>().map(|result| result.map(|sample| [sample])).collect::<Result<Vec<_>, _>>() {
                            let release_time = (header.sample_rate as f32 * 1.3).round();
                            let s2_path = PathBuf::from(format!("{}/s2", tpath));
                            let mut sink = Sink::new(s2_path, format!("{}-", timestamp), header);
                            let mut gate = NoiseGate::new(4000, release_time as usize);
                            gate.process_frames(&samples, &mut sink);
                            std::fs::remove_file(spath).ok();
                        }
                    }
                }
            }
        }
    });
}

// Stage Two - Stitches files into a single timestamped file ready to be observed by SAM
// Results are stored in /opt/sam/tmp/sound/s3
/// Stage Two: Stitches consecutive audio clips into a single file for further processing.
/// Results are stored in /opt/sam/tmp/sound/s3.
pub fn s2_init() {
    thread::spawn(move || {
        loop {
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
                let s2_dir = format!("{}/s2", tpath_str);

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
                    if current_group.is_empty() || ts == current_group.last().unwrap() + 1 {
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
                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
                for group in groups {
                    // Skip if any timestamp is too recent (avoid files still being written)
                    if group.iter().any(|&ts| ts >= now - 1) {
                        continue;
                    }

                    // Collect file paths for this group, sorted by timestamp
                    let mut files_to_stitch: Vec<String> = group.iter()
                        .filter_map(|ts| {
                            file_map.iter()
                                .find(|(t, _)| t == ts)
                                .map(|(_, path)| path.clone())
                        })
                        .collect();
                    files_to_stitch.sort();

                    if files_to_stitch.is_empty() {
                        continue;
                    }

                    // Output file path
                    let out_dir = format!("{}/s3", tpath_str);
                    let _ = std::fs::create_dir_all(&out_dir);
                    let out_path = format!("{}/{}.incoming.wav", out_dir, group[0]);

                    // Stitch files
                    let mut writer: Option<WavWriter<BufWriter<File>>> = None;
                    for file_path in &files_to_stitch {
                        match WavReader::open(file_path) {
                            Ok(reader) => {
                                let spec = reader.spec();
                                let samples = match reader.into_samples::<i16>()
                                    .map(|r| r.map(|s| [s]))
                                    .collect::<Result<Vec<_>, _>>() {
                                    Ok(s) => s,
                                    Err(e) => {
                                        log::error!("Failed to read samples from {}: {}", file_path, e);
                                        continue;
                                    }
                                };

                                if writer.is_none() {
                                    // Create writer with the spec of the first file
                                    let out_file = match File::create(&out_path) {
                                        Ok(f) => f,
                                        Err(e) => {
                                            log::error!("Failed to create output file {}: {}", out_path, e);
                                            break;
                                        }
                                    };
                                    writer = Some(WavWriter::new(BufWriter::new(out_file), spec).unwrap());
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
        }
    });
}


use opencl3::device::{get_all_devices, Device, CL_DEVICE_TYPE_GPU};

// Stage Three - 
/// Stage Three: Processes stitched audio files for speech-to-text (STT) and observation.
/// Consumes files from /opt/sam/tmp/sound/*/s3, runs STT, observes, and cleans up.
/// Uses a thread pool for parallel processing.
pub fn s3_init() {
    thread::spawn(move || {
        // Use a thread pool with a configurable number of threads (default: 3)
        let pool = threadpool::Builder::new()
            .num_threads(3)
            .build();

        // Track files currently being processed to avoid duplicate work
        let mut processing_queue: Vec<String> = Vec::new();

        loop {
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
                            match crate::sam::services::stt::deep_speech_process(fpath_thread.clone()) {
                                Ok(stt) if !stt.stt.is_empty() => {
                                    // Optionally play a notification sound
                                    // crate::sam::tools::uinx_cmd("aplay /opt/sam/beep.wav".to_string());

                                    // Observe the sound and prediction
                                    observe(stt, &fpath_thread);
                                }
                                Ok(_) => {} // No speech detected
                                Err(e) => {
                                    log::error!("STT processing failed for {}: {}", fpath_thread, e);
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
    });
}


pub struct Sink {
    output_dir: PathBuf,
    clip_number: usize,
    prefix: String,
    spec: WavSpec,
    writer: Option<WavWriter<BufWriter<File>>>,
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

    fn get_writer(&mut self) -> &mut WavWriter<BufWriter<File>> {
        if self.writer.is_none() {
            // Lazily initialize the writer. This lets us drop the writer when 
            // sent an end_of_transmission and have it automatically start
            // writing to a new clip when necessary.
            let filename = self
                .output_dir
                .join(format!("{}{}.wav", self.prefix, self.clip_number));
            self.clip_number += 1;
            self.writer = Some(WavWriter::create(filename, self.spec).unwrap());
        }

        self.writer.as_mut().unwrap()
    }
}

impl<F> noise_gate::Sink<F> for Sink
where
    F: Frame,
    F::Sample: hound::Sample,
{
    fn record(&mut self, frame: F) {
        let writer = self.get_writer();

        // write all the channels as interlaced audio
        for channel in frame.channels() {
            writer.write_sample(channel).unwrap();
        }
    }

    fn end_of_transmission(&mut self) {
        // if we were previously recording a transmission, remove the writer
        // and let it flush to disk
        if let Some(writer) = self.writer.take() {
            writer.finalize().unwrap();
        }
    }
}
