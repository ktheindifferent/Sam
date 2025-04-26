// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;
use std::os::unix::fs::PermissionsExt; // Added for `from_mode`
use error_chain::error_chain;
use crate::sam::tools; // Add missing import for tools module

error_chain! {
    foreign_links {
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
        Postgres(postgres::Error);
        Hound(hound::Error);
    }
}
pub static MIME_MAP: [(&str, &str); 157] = [
    (".3gp", "video/3gpp"),
    (".3g2", "video/3gpp2"),
    (".7z", "application/x-7z-compressed"),
    (".aac", "audio/aac"),
    (".ai", "application/postscript"),
    (".aif", "audio/x-aiff"),
    (".aiff", "audio/x-aiff"),
    (".amr", "audio/amr"),
    (".apk", "application/vnd.android.package-archive"),
    (".apng", "image/apng"),
    (".arj", "application/x-arj"),
    (".asf", "video/x-ms-asf"),
    (".asp", "application/x-aspx"),
    (".aspx", "application/x-aspx"),
    (".avi", "video/x-msvideo"),
    (".azw3", "application/vnd.amazon.ebook"),
    (".bat", "application/x-msdownload"),
    (".bin", "application/octet-stream"),
    (".bmp", "image/bmp"),
    (".bz2", "application/x-bzip2"),
    (".cab", "application/vnd.ms-cab-compressed"),
    (".c", "text/x-c"),
    (".cc", "text/x-c++src"),
    (".chm", "application/vnd.ms-htmlhelp"),
    (".class", "application/java"),
    (".clj", "text/x-clojure"),
    (".cpp", "text/x-c++src"),
    (".cjs", "application/javascript"),
    (".conf", "text/plain"),
    (".cpio", "application/x-cpio"),
    (".css", "text/css"),
    (".csv", "text/csv"),
    (".cue", "application/x-cue"),
    (".cxx", "text/x-c++src"),
    (".dart", "application/dart"),
    (".deb", "application/x-debian-package"),
    (".dll", "application/x-msdownload"),
    (".dmg", "application/x-apple-diskimage"),
    (".doc", "application/msword"),
    (".docm", "application/vnd.ms-word.document.macroEnabled.12"),
    (".docx", "application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
    (".dot", "application/msword"),
    (".dotx", "application/vnd.openxmlformats-officedocument.wordprocessingml.template"),
    (".dylib", "application/x-dylib"),
    (".eot", "application/vnd.ms-fontobject"),
    (".epub", "application/epub+zip"),
    (".exe", "application/vnd.microsoft.portable-executable"),
    (".fb2", "application/x-fictionbook+xml"),
    (".flac", "audio/flac"),
    (".flv", "video/x-flv"),
    (".gif", "image/gif"),
    (".go", "application/x-go"),
    (".gz", "application/gzip"),
    (".h", "text/x-c++hdr"),
    (".h++", "text/x-c++hdr"),
    (".heic", "image/heic"),
    (".heif", "image/heif"),
    (".hh", "text/x-c++hdr"),
    (".htm", "text/html"),
    (".html", "text/html"),
    (".hpp", "text/x-c++hdr"),
    (".hxx", "text/x-c++hdr"),
    (".ico", "image/x-icon"),
    (".ini", "text/plain"),
    (".iso", "application/x-iso9660-image"),
    (".jar", "application/java-archive"),
    (".java", "application/java-vm"),
    (".jpeg", "image/jpeg"),
    (".jpg", "image/jpeg"),
    (".js", "application/javascript"),
    (".json", "application/json"),
    (".jsp", "application/x-jsp"),
    (".jsx", "application/javascript"),
    (".key", "application/x-iwork-keynote-sffkey"),
    (".less", "text/x-less"),
    (".log", "text/plain"),
    (".lua", "application/lua"),
    (".m", "text/x-objective-c"),
    (".m3u", "audio/mpegurl"),
    (".m3u8", "application/vnd.apple.mpegurl"),
    (".m4a", "audio/mp4"),
    (".m4v", "video/x-m4v"),
    (".md", "text/markdown"),
    (".midi", "audio/midi"),
    (".mid", "audio/midi"),
    (".mjs", "application/javascript"),
    (".mkv", "video/x-matroska"),
    (".mm", "text/x-objective-c++"),
    (".mobi", "application/x-mobipocket-ebook"),
    (".mov", "video/quicktime"),
    (".mp3", "audio/mpeg"),
    (".mp4", "video/mp4"),
    (".mpg", "video/mpeg"),
    (".mpeg", "video/mpeg"),
    (".msi", "application/x-msdownload"),
    (".odp", "application/vnd.oasis.opendocument.presentation"),
    (".ods", "application/vnd.oasis.opendocument.spreadsheet"),
    (".odc", "application/vnd.oasis.opendocument.chart"),
    (".odf", "application/vnd.oasis.opendocument.formula"),
    (".odg", "application/vnd.oasis.opendocument.graphics"),
    (".odm", "application/vnd.oasis.opendocument.text-master"),
    (".odt", "application/vnd.oasis.opendocument.text"),
    (".oga", "audio/ogg"),
    (".ogg", "audio/ogg"),
    (".ogv", "video/ogg"),
    (".opus", "audio/opus"),
    (".otf", "font/otf"),
    (".pdf", "application/pdf"),
    (".php", "application/x-httpd-php"),
    (".pl", "application/x-perl"),
    (".pls", "audio/x-scpls"),
    (".png", "image/png"),
    (".ppt", "application/vnd.ms-powerpoint"),
    (".pptm", "application/vnd.ms-powerpoint.presentation.macroEnabled.12"),
    (".pptx", "application/vnd.openxmlformats-officedocument.presentationml.presentation"),
    (".ps", "application/postscript"),
    (".py", "application/x-python"),
    (".rar", "application/x-rar-compressed"),
    (".rb", "application/x-ruby"),
    (".rst", "text/x-rst"),
    (".rs", "application/rust"),
    (".rtf", "application/rtf"),
    (".sass", "text/x-sass"),
    (".scss", "text/x-scss"),
    (".sh", "application/x-sh"),
    (".so", "application/x-sharedlib"),
    (".sql", "application/sql"),
    (".svg", "image/svg+xml"),
    (".swf", "application/x-shockwave-flash"),
    (".tar", "application/x-tar"),
    (".tex", "application/x-tex"),
    (".tif", "image/tiff"),
    (".tiff", "image/tiff"),
    (".toast", "application/x-toast"),
    (".toml", "application/toml"),
    (".torrent", "application/x-bittorrent"),
    (".ts", "application/typescript"),
    (".tsv", "text/tab-separated-values"),
    (".ttf", "font/ttf"),
    (".txt", "text/plain"),
    (".vcd", "application/x-cd-image"),
    (".wav", "audio/wav"),
    (".webm", "video/webm"),
    (".webp", "image/webp"),
    (".woff", "font/woff"),
    (".woff2", "font/woff2"),
    (".wsdl", "application/wsdl+xml"),
    (".xhtml", "application/xhtml+xml"),
    (".xls", "application/vnd.ms-excel"),
    (".xlsm", "application/vnd.ms-excel.sheet.macroEnabled.12"),
    (".xlsx", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
    (".xml", "application/xml"),
    (".xps", "application/vnd.ms-xpsdocument"),
    (".xz", "application/x-xz"),
    (".yaml", "application/x-yaml"),
    (".yml", "application/x-yaml"),
    (".zip", "application/zip"),
];


impl From<zip::result::ZipError> for tools::Error {
    fn from(err: zip::result::ZipError) -> Self {
        tools::Error::from(err.to_string())
    }
}

/// Executes a Python 3 command and returns its output as a `String`.
pub fn python3(command: &str) -> Result<String> {
    let output = Command::new("python3")
        .arg(command)
        .output()
        .map_err(Error::from)?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Executes a shell command and returns its output as a `String`.
pub fn cmd(command: &str) -> Result<String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .map_err(Error::from)?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Executes a Linux shell command and logs the result.
pub fn uinx_cmd(command: &str) {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output();

    match output {
        Ok(cmd) if cmd.status.success() => {
            log::info!("{}:{}", command, String::from_utf8_lossy(&cmd.stdout));
        }
        Ok(cmd) => {
            log::error!("{}:{}", command, String::from_utf8_lossy(&cmd.stderr));
        }
        Err(e) => {
            log::error!("Failed to execute command '{}': {}", command, e);
        }
    }
}

/// Checks if a WAV file contains sounds above a certain threshold.
pub fn does_wav_have_sounds(audio_filename: &str) -> Result<bool> {
    let threshold = 14000_i16;
    let mut has_sounds = false;

    let mut audio_file = hound::WavReader::open(audio_filename)?; // Add `mut` to fix borrow issue
    let raw_samples = audio_file.samples::<i16>().filter_map(|result| result.ok()); // Fixed closure type

    for (i, sample) in raw_samples.enumerate() {
        if i % 100 == 0 && (sample > threshold || sample < -threshold) {
            has_sounds = true;
            break;
        }
    }

    Ok(has_sounds)
}

/// Extracts a ZIP file to the specified directory.
pub fn extract_zip(zip_path: &str, extract_path: &str) -> Result<()> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = Path::new(extract_path).join(file.enclosed_name().ok_or("Invalid path")?);

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            let mut outfile = fs::File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;

            #[cfg(unix)]
            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
            }
        }
    }

    Ok(())
}

