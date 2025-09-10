use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use anyhow::{Result, Context};
use log::{debug, warn};
use std::ops::{Deref, DerefMut};

/// Temporary file with automatic cleanup on drop
pub struct TempFile {
    path: PathBuf,
    cleanup_on_drop: bool,
}

impl TempFile {
    /// Create a new temporary file
    pub fn new(base_dir: &Path) -> Result<Self> {
        let filename = format!("tmp_{}", nanoid::nanoid!());
        let path = base_dir.join(filename);
        
        Ok(TempFile {
            path,
            cleanup_on_drop: true,
        })
    }
    
    /// Create a temporary file with a specific extension
    pub fn with_extension(base_dir: &Path, extension: &str) -> Result<Self> {
        let filename = format!("tmp_{}.{}", nanoid::nanoid!(), extension);
        let path = base_dir.join(filename);
        
        Ok(TempFile {
            path,
            cleanup_on_drop: true,
        })
    }
    
    /// Get the path to the temporary file
    pub fn path(&self) -> &Path {
        &self.path
    }
    
    /// Write data to the temporary file
    pub async fn write(&self, data: &[u8]) -> Result<()> {
        let mut file = fs::File::create(&self.path).await
            .with_context(|| format!("Failed to create temp file: {:?}", self.path))?;
        
        file.write_all(data).await
            .with_context(|| format!("Failed to write to temp file: {:?}", self.path))?;
        
        file.sync_all().await
            .with_context(|| format!("Failed to sync temp file: {:?}", self.path))?;
        
        Ok(())
    }
    
    /// Read data from the temporary file
    pub async fn read(&self) -> Result<Vec<u8>> {
        fs::read(&self.path).await
            .with_context(|| format!("Failed to read temp file: {:?}", self.path))
    }
    
    /// Move the temporary file to a permanent location
    pub async fn move_to(mut self, destination: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).await
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }
        
        // Move the file
        fs::rename(&self.path, destination).await
            .with_context(|| format!("Failed to move file from {:?} to {:?}", self.path, destination))?;
        
        // Disable cleanup since file was moved
        self.cleanup_on_drop = false;
        
        debug!("Moved temp file from {:?} to {:?}", self.path, destination);
        Ok(())
    }
    
    /// Keep the file (disable automatic cleanup)
    pub fn persist(mut self) -> PathBuf {
        self.cleanup_on_drop = false;
        self.path.clone()
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if self.cleanup_on_drop {
            let path = self.path.clone();
            
            // Spawn a task to clean up the file
            tokio::spawn(async move {
                if let Err(e) = fs::remove_file(&path).await {
                    if e.kind() != std::io::ErrorKind::NotFound {
                        warn!("Failed to cleanup temp file {:?}: {}", path, e);
                    }
                } else {
                    debug!("Cleaned up temp file: {:?}", path);
                }
            });
        }
    }
}

/// Resource cleanup guard using RAII pattern
pub struct CleanupGuard<T> {
    resource: Option<T>,
    cleanup: Option<Box<dyn FnOnce(T) + Send + 'static>>,
}

impl<T> CleanupGuard<T> {
    /// Create a new cleanup guard
    pub fn new<F>(resource: T, cleanup: F) -> Self
    where
        F: FnOnce(T) + Send + 'static,
    {
        CleanupGuard {
            resource: Some(resource),
            cleanup: Some(Box::new(cleanup)),
        }
    }
    
    /// Take the resource without running cleanup
    pub fn take(mut self) -> T {
        self.cleanup = None;
        self.resource.take().expect("Resource already taken")
    }
    
    /// Get a reference to the resource
    pub fn get(&self) -> &T {
        self.resource.as_ref().expect("Resource already taken")
    }
    
    /// Get a mutable reference to the resource
    pub fn get_mut(&mut self) -> &mut T {
        self.resource.as_mut().expect("Resource already taken")
    }
}

impl<T> Drop for CleanupGuard<T> {
    fn drop(&mut self) {
        if let (Some(resource), Some(cleanup)) = (self.resource.take(), self.cleanup.take()) {
            cleanup(resource);
        }
    }
}

impl<T> Deref for CleanupGuard<T> {
    type Target = T;
    
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T> DerefMut for CleanupGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

/// Resource cleanup manager for batch operations
pub struct ResourceCleanup {
    resources: Vec<Box<dyn CleanupTask>>,
}

impl Default for ResourceCleanup {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceCleanup {
    /// Create a new resource cleanup manager
    pub fn new() -> Self {
        ResourceCleanup {
            resources: Vec::new(),
        }
    }
    
    /// Add a file to be cleaned up
    pub fn add_file(&mut self, path: PathBuf) {
        self.resources.push(Box::new(FileCleanupTask { path }));
    }
    
    /// Add a directory to be cleaned up
    pub fn add_directory(&mut self, path: PathBuf) {
        self.resources.push(Box::new(DirectoryCleanupTask { path }));
    }
    
    /// Add a custom cleanup task
    pub fn add_task(&mut self, task: Box<dyn CleanupTask>) {
        self.resources.push(task);
    }
    
    /// Execute all cleanup tasks
    pub async fn cleanup(self) -> Result<()> {
        let mut errors = Vec::new();
        
        for task in self.resources {
            if let Err(e) = task.cleanup().await {
                errors.push(e);
            }
        }
        
        if !errors.is_empty() {
            let error_msg = errors
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; ");
            
            return Err(anyhow::anyhow!("Cleanup errors: {}", error_msg));
        }
        
        Ok(())
    }
}

/// Trait for cleanup tasks
#[async_trait::async_trait]
pub trait CleanupTask: Send {
    async fn cleanup(self: Box<Self>) -> Result<()>;
}

/// File cleanup task
struct FileCleanupTask {
    path: PathBuf,
}

#[async_trait::async_trait]
impl CleanupTask for FileCleanupTask {
    async fn cleanup(self: Box<Self>) -> Result<()> {
        if self.path.exists() {
            fs::remove_file(&self.path).await
                .with_context(|| format!("Failed to cleanup file: {:?}", self.path))?;
            debug!("Cleaned up file: {:?}", self.path);
        }
        Ok(())
    }
}

/// Directory cleanup task
struct DirectoryCleanupTask {
    path: PathBuf,
}

#[async_trait::async_trait]
impl CleanupTask for DirectoryCleanupTask {
    async fn cleanup(self: Box<Self>) -> Result<()> {
        if self.path.exists() {
            fs::remove_dir_all(&self.path).await
                .with_context(|| format!("Failed to cleanup directory: {:?}", self.path))?;
            debug!("Cleaned up directory: {:?}", self.path);
        }
        Ok(())
    }
}

/// Scopeguard for ensuring cleanup on all paths
pub struct ScopeGuard<F: FnOnce()> {
    cleanup: Option<F>,
}

impl<F: FnOnce()> ScopeGuard<F> {
    /// Create a new scope guard
    pub fn new(cleanup: F) -> Self {
        ScopeGuard {
            cleanup: Some(cleanup),
        }
    }
    
    /// Cancel the cleanup
    pub fn cancel(mut self) {
        self.cleanup = None;
    }
}

impl<F: FnOnce()> Drop for ScopeGuard<F> {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            cleanup();
        }
    }
}

/// Create a scope guard
#[macro_export]
macro_rules! scope_guard {
    ($cleanup:expr) => {
        $crate::resource_management::cleanup::ScopeGuard::new(|| $cleanup)
    };
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_temp_file_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let temp_file = TempFile::new(temp_dir.path()).unwrap();
        let path = temp_file.path().to_path_buf();
        
        // Write some data
        temp_file.write(b"test data").await.unwrap();
        assert!(path.exists());
        
        // Drop the temp file
        drop(temp_file);
        
        // Give async cleanup time to run
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // File should be cleaned up
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn test_temp_file_persist() {
        let temp_dir = TempDir::new().unwrap();
        let temp_file = TempFile::new(temp_dir.path()).unwrap();
        
        temp_file.write(b"test data").await.unwrap();
        let path = temp_file.persist();
        
        // File should still exist after persist
        assert!(path.exists());
        
        // Clean up manually
        fs::remove_file(path).await.unwrap();
    }

    #[test]
    fn test_cleanup_guard() {
        use std::sync::atomic::{AtomicBool, Ordering};
        
        let cleaned = Arc::new(AtomicBool::new(false));
        let cleaned_clone = cleaned.clone();
        
        {
            let _guard = CleanupGuard::new(
                42,
                move |_value| {
                    cleaned_clone.store(true, Ordering::SeqCst);
                },
            );
        }
        
        assert!(cleaned.load(Ordering::SeqCst));
    }

    #[test]
    fn test_scope_guard() {
        use std::sync::atomic::{AtomicBool, Ordering};
        
        let cleaned = Arc::new(AtomicBool::new(false));
        let cleaned_clone = cleaned.clone();
        
        {
            let _guard = scope_guard!(cleaned_clone.store(true, Ordering::SeqCst));
        }
        
        assert!(cleaned.load(Ordering::SeqCst));
    }
}