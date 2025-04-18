// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (PixelCoda)
// Licensed under GPLv3....see LICENSE file.

use std::thread;
use std::fs::File;
use std::io::{Write};
use std::path::Path;
use std::str::FromStr;
use serde::{Serialize, Deserialize};

/// Initializes the SPREC service (currently a placeholder).
pub fn init() {
    // Placeholder for future initialization logic.
}

/// Builds the SPREC model by processing observations and generating audio files.
pub fn build() {
    thread::spawn(move || {
        let mut pg_query = crate::sam::memory::PostgresQueries::default();
        pg_query.queries.push(crate::sam::memory::PGCol::String("HEARD".to_string()));
        pg_query.query_coulmns.push("observation_type =".to_string());

        pg_query.queries.push(crate::sam::memory::PGCol::String("%PERSON%".to_string()));
        pg_query.query_coulmns.push(" AND observation_objects ilike".to_string());

        let observations = match crate::sam::memory::Observation::select_lite(None, None, None, Some(pg_query)) {
            Ok(obs) => obs,
            Err(e) => {
                log::error!("Failed to fetch observations: {:?}", e);
                return;
            }
        };

        let mut xrows = 0;
        for observation in observations.clone() {
            xrows += 1;
            log::info!("SPREC build processed observation {}/{}", xrows, observations.len());

            for human in observation.observation_humans.clone() {
                let audio_dir = format!("/opt/sam/scripts/sprec/audio/{}", human.oid);
                if !Path::new(&audio_dir).exists() {
                    if let Err(e) = std::fs::create_dir(&audio_dir) {
                        log::error!("Failed to create directory {}: {:?}", audio_dir, e);
                        continue;
                    }
                }

                log::info!("SPREC build processed human {:?}", human);

                let audio_file = format!("{}/{}.wav", audio_dir, observation.oid);
                if !Path::new(&audio_file).exists() {
                    let mut full_pg_query = crate::sam::memory::PostgresQueries::default();
                    full_pg_query.queries.push(crate::sam::memory::PGCol::String(observation.oid.clone()));
                    full_pg_query.query_coulmns.push("oid =".to_string());

                    if let Ok(full_observations) = crate::sam::memory::Observation::select(None, None, None, Some(full_pg_query)) {
                        if let Some(full_observation) = full_observations.get(0) {
                            if let Some(file_data) = &full_observation.observation_file {
                                if let Err(e) = std::fs::write(&audio_file, file_data) {
                                    log::error!("Failed to write audio file {}: {:?}", audio_file, e);
                                }
                            }
                        }
                    }
                }
            }
        }

        crate::sam::tools::uinx_cmd("python3 /opt/sam/scripts/sprec/build.py".to_string());
    });
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SprecPrediction {
    pub human: String,
    pub confidence: f64,
}

/// Predicts the human identity from an audio file using the SPREC model.
pub fn predict(file_path: &str) -> Result<SprecPrediction, crate::sam::services::Error> {
    let test_file = "/opt/sam/scripts/sprec/test.wav";
    if Path::new(test_file).exists() {
        std::fs::remove_file(test_file)?;
    }
    std::fs::copy(file_path, test_file)?;

    let result = crate::sam::tools::cmd("python3 /opt/sam/scripts/sprec/predict.py".to_string());
    let vec: Vec<&str> = result.split(":::::").collect();

    if vec.len() > 2 {
        Ok(SprecPrediction {
            human: vec[1].to_string(),
            confidence: f64::from_str(vec[2])?,
        })
    } else {
        Ok(SprecPrediction {
            human: "".to_string(),
            confidence: 0.0,
        })
    }
}

/// Installs the SPREC model and its dependencies.
pub fn install() -> std::io::Result<()> {
    let files = [
        ("../../../scripts/sprec/build.py", "/opt/sam/scripts/sprec/build.py"),
        ("../../../scripts/sprec/predict.py", "/opt/sam/scripts/sprec/predict.py"),
        ("../../../scripts/sprec/requirements.txt", "/opt/sam/scripts/sprec/requirements.txt"),
        ("../../../scripts/sprec/model.h5", "/opt/sam/scripts/sprec/model.h5"),
        ("../../../scripts/sprec/labels.pickle", "/opt/sam/scripts/sprec/labels.pickle"),
        ("../../../scripts/sprec/audio/Unknown.zip", "/opt/sam/scripts/sprec/audio/Unknown.zip"),
        ("../../../scripts/sprec/noise/other.zip", "/opt/sam/scripts/sprec/noise/other.zip"),
        ("../../../scripts/sprec/noise/_background_noise_.zip", "/opt/sam/scripts/sprec/noise/_background_noise_.zip"),
    ];

    for (source, destination) in files.iter() {
        let data = include_bytes!(source);
        let mut buffer = File::create(destination)?;
        buffer.write_all(data)?;
    }

    crate::sam::tools::extract_zip("/opt/sam/scripts/sprec/audio/Unknown.zip", "/opt/sam/scripts/sprec/audio/".to_string());
    crate::sam::tools::extract_zip("/opt/sam/scripts/sprec/noise/other.zip", "/opt/sam/scripts/sprec/noise/".to_string());
    crate::sam::tools::extract_zip("/opt/sam/scripts/sprec/noise/_background_noise_.zip", "/opt/sam/scripts/sprec/noise/".to_string());

    crate::sam::tools::uinx_cmd("rm -rf /opt/sam/scripts/sprec/audio/Unknown.zip".to_string());
    crate::sam::tools::uinx_cmd("rm -rf /opt/sam/scripts/sprec/noise/other.zip".to_string());
    crate::sam::tools::uinx_cmd("rm -rf /opt/sam/scripts/sprec/noise/_background_noise_.zip".to_string());

    log::info!("Installing requirements for SPREC...");
    crate::sam::tools::uinx_cmd("pip3 install -r /opt/sam/scripts/sprec/requirements.txt".to_string());

    log::info!("Building initial SPREC model...");
    build();
    Ok(())
}