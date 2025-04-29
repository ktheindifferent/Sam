use colored::Colorize;
use std::env;
use std::io::BufRead;
use std::io::BufReader;
use std::io::{self, Write};
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn append_line(output_lines: &Mutex<Vec<String>>, line: String) {
    let mut lines = output_lines.lock().await;
    lines.push(line);
}

pub async fn append_lines<I: IntoIterator<Item = String>>(
    output_lines: &Mutex<Vec<String>>,
    lines: I,
) {
    let mut guard = output_lines.lock().await;
    guard.extend(lines);
}

pub fn get_human_name() -> String {
    std::fs::read_to_string("/opt/sam/whoismyhuman")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "sam".to_string())
}

pub fn check_postgres_env() {
    let vars = ["PG_DBNAME", "PG_USER", "PG_PASS", "PG_ADDRESS"];
    let mut missing = vec![];
    for v in vars.iter() {
        match std::env::var(v) {
            Ok(val) if !val.trim().is_empty() => {}
            _ => missing.push(*v),
        }
    }
    if !missing.is_empty() {
        log::info!("{}", "Postgres credentials missing:".red().bold());
        for v in missing {
            loop {
                print!("{}", format!("Enter value for {v}: ").cyan().bold());
                io::stdout().flush().unwrap();
                let mut val = String::new();
                if io::stdin().read_line(&mut val).is_ok() {
                    let val = val.trim();
                    if !val.is_empty() {
                        env::set_var(v, val);
                        break;
                    }
                }
                log::info!("{}", format!("{v} cannot be empty.").red());
            }
        }
    }
}

pub fn run_command_stream_lines<F>(mut cmd: Command, mut on_line: F) -> io::Result<i32>
where
    F: FnMut(String) + Send + 'static,
{
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn()?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let mut reader = BufReader::new(stdout);
    let mut err_reader = BufReader::new(stderr);

    let mut buf = String::new();
    let mut err_buf = String::new();

    loop {
        buf.clear();
        err_buf.clear();
        let stdout_read = reader.read_line(&mut buf)?;
        let stderr_read = err_reader.read_line(&mut err_buf)?;

        if stdout_read == 0 && stderr_read == 0 {
            break;
        }
        if !buf.trim().is_empty() {
            on_line(buf.trim_end().to_string());
        }
        if !err_buf.trim().is_empty() {
            on_line(err_buf.trim_end().to_string());
        }
    }
    let status = child.wait()?;
    Ok(status.code().unwrap_or(-1))
}

// Add this function for TTS output
pub async fn append_and_tts(output_lines: Arc<Mutex<Vec<String>>>, text: String) {
    append_line(&output_lines, text.clone()).await;
    match crate::sam::services::tts::get(text.clone().replace("┌─[sam]─>", "")) {
        Ok(wav_bytes) => {
            if let Err(e) = play_wav_from_bytes_send(&wav_bytes) {
                append_line(&output_lines, format!("TTS playback error: {e}")).await;
            }
        }
        Err(e) => {
            append_line(&output_lines, format!("TTS error: {e}")).await;
        }
    }
}

// Add this helper for TTS playback
fn play_wav_from_bytes_send(
    wav_bytes: &[u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use rodio::{Decoder, OutputStream, Sink};
    use std::io::Cursor;
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;
    let cursor = Cursor::new(wav_bytes.to_vec());
    let source = Decoder::new(cursor)?;
    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}
