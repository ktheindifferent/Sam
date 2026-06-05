//! Modern async file I/O and path handling

use anyhow::{Context, Result};
use futures::stream::{Stream, StreamExt};
use log::{debug, error, info, warn};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

// TODO: Implement these modules
// pub mod cache;
// pub mod watcher;
// pub mod temp;

// pub use cache::FileCache;
// pub use watcher::FileWatcher;
// pub use temp::TempFileManager;

/// Modern async file operations
pub struct AsyncFileOps;

impl AsyncFileOps {
    /// Read file with size limit
    pub async fn read_file(path: &Path, max_size: Option<usize>) -> Result<String> {
        debug!("Reading file: {:?}", path);

        // Check file size first
        let metadata = fs::metadata(path)
            .await
            .with_context(|| format!("Failed to get metadata for {:?}", path))?;

        if let Some(max) = max_size {
            if metadata.len() > max as u64 {
                return Err(anyhow::anyhow!(
                    "File {:?} exceeds max size: {} > {}",
                    path,
                    metadata.len(),
                    max
                ));
            }
        }

        let content = fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read file {:?}", path))?;

        debug!("Successfully read {} bytes from {:?}", content.len(), path);
        Ok(content)
    }

    /// Read file as bytes
    pub async fn read_bytes(path: &Path, max_size: Option<usize>) -> Result<Vec<u8>> {
        debug!("Reading bytes from: {:?}", path);

        let metadata = fs::metadata(path).await?;

        if let Some(max) = max_size {
            if metadata.len() > max as u64 {
                return Err(anyhow::anyhow!(
                    "File {:?} exceeds max size: {} > {}",
                    path,
                    metadata.len(),
                    max
                ));
            }
        }

        fs::read(path)
            .await
            .with_context(|| format!("Failed to read bytes from {:?}", path))
    }

    /// Write file atomically
    pub async fn write_atomic(path: &Path, content: &str) -> Result<()> {
        debug!("Writing file atomically: {:?}", path);

        // Write to temp file first
        let temp_path = path.with_extension("tmp");

        fs::write(&temp_path, content)
            .await
            .with_context(|| format!("Failed to write temp file {:?}", temp_path))?;

        // Rename atomically
        fs::rename(&temp_path, path)
            .await
            .with_context(|| format!("Failed to rename {:?} to {:?}", temp_path, path))?;

        info!("Successfully wrote {} bytes to {:?}", content.len(), path);
        Ok(())
    }

    /// Append to file
    pub async fn append(path: &Path, content: &str) -> Result<()> {
        use tokio::fs::OpenOptions;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await
            .with_context(|| format!("Failed to open {:?} for appending", path))?;

        file.write_all(content.as_bytes())
            .await
            .with_context(|| format!("Failed to append to {:?}", path))?;

        Ok(())
    }

    /// Stream file lines
    pub async fn stream_lines(path: &Path) -> Result<impl Stream<Item = Result<String>>> {
        let file = fs::File::open(path)
            .await
            .with_context(|| format!("Failed to open {:?}", path))?;

        let reader = BufReader::new(file);
        let lines = reader.lines();
        let stream = futures::stream::unfold(lines, |mut lines| async move {
            match lines.next_line().await {
                Ok(Some(line)) => Some((Ok(line), lines)),
                Ok(None) => None,
                Err(e) => Some((Err(anyhow::anyhow!("Failed to read line: {}", e)), lines)),
            }
        });

        Ok(stream)
    }

    /// Copy with progress
    pub async fn copy_with_progress<F>(from: &Path, to: &Path, mut progress: F) -> Result<u64>
    where
        F: FnMut(u64) + Send,
    {
        use tokio::io::AsyncRead;

        let mut source = fs::File::open(from)
            .await
            .with_context(|| format!("Failed to open source {:?}", from))?;

        let mut dest = fs::File::create(to)
            .await
            .with_context(|| format!("Failed to create dest {:?}", to))?;

        let mut buffer = vec![0u8; 8192];
        let mut total = 0u64;

        loop {
            let n = source
                .read(&mut buffer)
                .await
                .with_context(|| format!("Failed to read from {:?}", from))?;

            if n == 0 {
                break;
            }

            dest.write_all(&buffer[..n])
                .await
                .with_context(|| format!("Failed to write to {:?}", to))?;

            total += n as u64;
            progress(total);
        }

        Ok(total)
    }

    /// Ensure directory exists
    pub async fn ensure_dir(path: &Path) -> Result<()> {
        if !path.exists() {
            fs::create_dir_all(path)
                .await
                .with_context(|| format!("Failed to create directory {:?}", path))?;
            info!("Created directory: {:?}", path);
        }
        Ok(())
    }

    /// Remove directory recursively with safety check
    pub async fn remove_dir_safe(path: &Path, require_empty: bool) -> Result<()> {
        if require_empty {
            let mut entries = fs::read_dir(path).await?;
            if entries.next_entry().await?.is_some() {
                return Err(anyhow::anyhow!("Directory {:?} is not empty", path));
            }
            fs::remove_dir(path).await?;
        } else {
            fs::remove_dir_all(path).await?;
        }
        info!("Removed directory: {:?}", path);
        Ok(())
    }

    /// Find files matching pattern
    pub async fn find_files(
        root: &Path,
        pattern: &str,
        max_depth: Option<usize>,
    ) -> Result<Vec<PathBuf>> {
        use glob::Pattern;

        let pattern =
            Pattern::new(pattern).with_context(|| format!("Invalid pattern: {}", pattern))?;

        let mut files = Vec::new();
        let mut stack = vec![(root.to_path_buf(), 0)];

        while let Some((path, depth)) = stack.pop() {
            if let Some(max) = max_depth {
                if depth > max {
                    continue;
                }
            }

            if path.is_file() && pattern.matches_path(&path) {
                files.push(path);
            } else if path.is_dir() {
                let mut entries = fs::read_dir(&path).await?;
                while let Some(entry) = entries.next_entry().await? {
                    stack.push((entry.path(), depth + 1));
                }
            }
        }

        Ok(files)
    }

    /// Get file info
    pub async fn file_info(path: &Path) -> Result<FileInfo> {
        let metadata = fs::metadata(path)
            .await
            .with_context(|| format!("Failed to get metadata for {:?}", path))?;

        Ok(FileInfo {
            path: path.to_path_buf(),
            size: metadata.len(),
            is_file: metadata.is_file(),
            is_dir: metadata.is_dir(),
            is_symlink: metadata.is_symlink(),
            modified: metadata.modified().ok(),
            created: metadata.created().ok(),
            permissions: {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    Some(metadata.permissions().mode())
                }
                #[cfg(not(unix))]
                None
            },
        })
    }
}

/// File information
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub is_file: bool,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub modified: Option<std::time::SystemTime>,
    pub created: Option<std::time::SystemTime>,
    pub permissions: Option<u32>,
}

/// Path utilities
pub struct PathUtils;

impl PathUtils {
    /// Resolve path with home directory expansion
    pub fn resolve_path(path: &str) -> PathBuf {
        if path.starts_with("~/") {
            if let Ok(home) = std::env::var("HOME") {
                return PathBuf::from(home).join(&path[2..]);
            }
        }
        PathBuf::from(path)
    }

    /// Get relative path
    pub fn relative_path(path: &Path, base: &Path) -> Option<PathBuf> {
        // Simple relative path calculation
        path.strip_prefix(base).ok().map(|p| p.to_path_buf())
    }

    /// Normalize path (remove . and ..)
    pub fn normalize_path(path: &Path) -> PathBuf {
        use std::path::Component;

        let mut components = Vec::new();

        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    components.pop();
                }
                c => components.push(c),
            }
        }

        components.iter().collect()
    }

    /// Check if path is safe (no traversal)
    pub fn is_safe_path(path: &Path, base: &Path) -> bool {
        if let Ok(canonical_path) = path.canonicalize() {
            if let Ok(canonical_base) = base.canonicalize() {
                return canonical_path.starts_with(canonical_base);
            }
        }
        false
    }

    /// Get unique filename
    pub fn unique_filename(path: &Path) -> PathBuf {
        if !path.exists() {
            return path.to_path_buf();
        }

        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");

        let extension = path.extension().and_then(|s| s.to_str());

        let parent = path.parent().unwrap_or(Path::new("."));

        for i in 1..1000 {
            let new_name = if let Some(ext) = extension {
                format!("{}_{}.{}", stem, i, ext)
            } else {
                format!("{}_{}", stem, i)
            };

            let new_path = parent.join(new_name);
            if !new_path.exists() {
                return new_path;
            }
        }

        // Fallback with timestamp
        let timestamp = chrono::Utc::now().timestamp();
        let new_name = if let Some(ext) = extension {
            format!("{}_{}.{}", stem, timestamp, ext)
        } else {
            format!("{}_{}", stem, timestamp)
        };

        parent.join(new_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_read_write() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Write
        AsyncFileOps::write_atomic(&file_path, "Hello, World!")
            .await
            .unwrap();

        // Read
        let content = AsyncFileOps::read_file(&file_path, None).await.unwrap();

        assert_eq!(content, "Hello, World!");
    }

    #[test]
    fn test_path_normalization() {
        let path = Path::new("/home/user/../user/./documents");
        let normalized = PathUtils::normalize_path(path);
        assert_eq!(normalized, Path::new("/home/user/documents"));
    }

    #[test]
    fn test_unique_filename() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().join("test.txt");

        // First should be unchanged
        let unique1 = PathUtils::unique_filename(&base_path);
        assert_eq!(unique1, base_path);

        // Create the file
        std::fs::write(&base_path, "test").unwrap();

        // Second should have suffix
        let unique2 = PathUtils::unique_filename(&base_path);
        assert_ne!(unique2, base_path);
        assert!(unique2.to_string_lossy().contains("test_"));
    }
}
