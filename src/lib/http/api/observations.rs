// ███████     █████     ███    ███
// ██         ██   ██    ████  ████
// ███████    ███████    ██ ████ ██
//      ██    ██   ██    ██  ██  ██
// ███████ ██ ██   ██ ██ ██      ██ ██
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use rouille::Request;
use rouille::Response;

use std::path::Path;

pub fn handle(
    _current_session: crate::memory::cache::WebSessions,
    request: &Request,
) -> Result<Response, crate::http::Error> {
    if request.url() == "/api/observations" {
        let skip_number: usize = request
            .get_param("skip")
            .unwrap_or_default()
            .parse()
            .unwrap_or(0);

        let objects = crate::memory::Observation::select_lite(
            Some(1),
            Some(skip_number),
            Some("timestamp DESC".to_string()),
            None,
        )?;
        return Ok(Response::json(&objects));
    }

    if request.url().contains("/api/observations/file/") {
        let url = request.url();
        let split = url.split("/");
        let vec: Vec<&str> = split.collect();
        let oid = vec
            .get(4)
            .ok_or_else(|| crate::http::Error::BadRequest("Missing observation id".to_string()))?;

        // Build query
        let mut pg_query = crate::memory::PostgresQueries::default();
        pg_query
            .queries
            .push(crate::memory::PGCol::String(oid.to_string()));
        pg_query.query_columns.push("oid =".to_string());

        // Select project by oid
        let observations = crate::memory::Observation::select(None, None, None, Some(pg_query))?;
        let observation = observations
            .first()
            .ok_or_else(|| crate::http::Error::BadRequest("Observation not found".to_string()))?
            .clone();

        let file_data = observation.observation_file.ok_or_else(|| {
            crate::http::Error::BadRequest("Observation file not found".to_string())
        })?;
        let response = Response::from_data("audio/wav", file_data);

        return Ok(response);
    }

    // Visual Wav Builder
    if request.url().contains("/api/observations/vwav/") {
        let url = request.url();
        let split = url.split("/");
        let vec: Vec<&str> = split.collect();
        let oid = vec
            .get(4)
            .ok_or_else(|| crate::http::Error::BadRequest("Missing observation id".to_string()))?;

        // Build query
        let mut pg_query = crate::memory::PostgresQueries::default();
        pg_query
            .queries
            .push(crate::memory::PGCol::String(oid.to_string()));
        pg_query.query_columns.push("oid =".to_string());

        // Select project by oid
        let observations = crate::memory::Observation::select(None, None, None, Some(pg_query))?;
        let observation = observations
            .first()
            .ok_or_else(|| crate::http::Error::BadRequest("Observation not found".to_string()))?
            .clone();

        let wav_data = observation.observation_file.ok_or_else(|| {
            crate::http::Error::BadRequest("Observation file not found".to_string())
        })?;

        let tmp_file_path = format!("/opt/sam/tmp/observations/vwav/{}.wav", observation.oid);

        // Use cached tmp file if it already exists
        let cache_path = format!("{}.16.wav.mp4", tmp_file_path);
        if Path::new(&cache_path).exists() {
            let data = std::fs::read(&cache_path)?;
            let response = Response::from_data("video/mp4", data);
            return Ok(response);
        }

        std::fs::write(&tmp_file_path, wav_data)?;

        let wav_16_path = format!("{}.16.wav", tmp_file_path);

        // TODO: Fix 8000 vs 16000
        crate::tools::safe_uinx_cmd(
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
                &wav_16_path,
            ],
        );

        crate::tools::safe_uinx_cmd(
            "/opt/sam/bin/whisper",
            &[
                "-m",
                "/opt/sam/models/ggml-large.bin",
                "-f",
                &wav_16_path,
                "-owts",
            ],
        );

        crate::services::stt::patch_whisper_wts()?;

        let wts_path = format!("{}.wts", wav_16_path);
        crate::tools::safe_uinx_cmd("chmod", &["+x", &wts_path]);

        crate::tools::safe_uinx_cmd(&wts_path, &[]);

        let data = std::fs::read(&cache_path)?;

        let response = Response::from_data("video/mp4", data);

        // Cleanup using std::fs instead of shell commands
        let _ = std::fs::remove_file(&tmp_file_path);
        let _ = std::fs::remove_file(&wav_16_path);
        let _ = std::fs::remove_file(&wts_path);

        return Ok(response);
    }

    Ok(Response::empty_404())
}
