//! 资源管理器 - 使用 RAII 模式自动清理资源
//!
//! 本模块提供基于 scopeguard 的自动资源管理功能，确保资源在作用域结束时自动清理。

use crate::utils::cleanup::{try_cleanup_temp_dir, CleanupItem, CleanupQueue};
use scopeguard::{guard, ScopeGuard};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, warn};

/// 文件锁守卫
pub struct FileLockGuard {
    path: PathBuf,
    locked: bool,
}

impl FileLockGuard {
    pub fn new(path: PathBuf) -> std::io::Result<Self> {
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(_) => {
                info!("File lock acquired: {}", path.display());
                Ok(Self { path, locked: true })
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                warn!("Lock file exists, waiting: {}", path.display());
                Err(e)
            }
            Err(e) => Err(e),
        }
    }

    pub fn try_new(path: PathBuf) -> Option<Self> {
        Self::new(path).ok()
    }

    pub fn is_locked(&self) -> bool {
        self.locked
    }
}

impl Drop for FileLockGuard {
    fn drop(&mut self) {
        if self.locked && self.path.exists() {
            if let Err(e) = fs::remove_file(&self.path) {
                warn!(
                    "Failed to release file lock: {} - {}",
                    self.path.display(),
                    e
                );
            } else {
                info!("File lock released: {}", self.path.display());
            }
        }
    }
}

/// 并发控制令牌
pub struct ConcurrencyToken {
    _phantom: std::marker::PhantomData<()>,
}

impl ConcurrencyToken {
    pub fn new(_max_concurrent: usize) -> std::io::Result<Self> {
        Ok(Self {
            _phantom: std::marker::PhantomData,
        })
    }

    pub fn try_new(_max_concurrent: usize) -> Option<Self> {
        Self::new(_max_concurrent).ok()
    }
}

/// 临时目录资源守卫
pub struct TempDirGuard {
    path: PathBuf,
    cleanup_queue: Arc<CleanupQueue>,
    cleaned: bool,
    cleanup_attempts: Arc<std::sync::atomic::AtomicUsize>,
}

impl TempDirGuard {
    pub fn new(path: PathBuf, cleanup_queue: Arc<CleanupQueue>) -> Self {
        info!("TempDirGuard created for path: {}", path.display());
        Self {
            path,
            cleanup_queue,
            cleaned: false,
            cleanup_attempts: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn cleanup(&mut self) {
        if self.cleaned {
            warn!("TempDirGuard already cleaned: {}", self.path.display());
            return;
        }

        info!(
            "Manually cleaning up temp directory: {}",
            self.path.display()
        );
        self.cleanup_attempts
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        try_cleanup_temp_dir(&self.path, &self.cleanup_queue);
        self.cleaned = true;
    }

    pub fn force_cleanup(&mut self) -> std::io::Result<()> {
        self.cleanup_attempts
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if self.path.exists() {
            match fs::remove_dir_all(&self.path) {
                Ok(_) => {
                    info!("Temp directory cleaned up: {}", self.path.display());
                    self.cleaned = true;
                    Ok(())
                }
                Err(e) => {
                    warn!(
                        "Failed to cleanup temp directory: {} - {}",
                        self.path.display(),
                        e
                    );
                    self.cleanup_queue.push(CleanupItem {
                        path: self.path.clone(),
                        retry_count: 0,
                    });
                    Err(e)
                }
            }
        } else {
            self.cleaned = true;
            Ok(())
        }
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        if !self.cleaned && self.path.exists() {
            let attempts = self
                .cleanup_attempts
                .load(std::sync::atomic::Ordering::Acquire);
            info!(
                "Cleaning up temp directory: {} (attempt: {})",
                self.path.display(),
                attempts + 1
            );

            try_cleanup_temp_dir(&self.path, &self.cleanup_queue);

            if self.path.exists() {
                let _ = fs::remove_dir_all(&self.path);
            }

            self.cleaned = true;
        }
    }
}

/// 资源管理器
pub struct ResourceManager {
    cleanup_queue: Arc<CleanupQueue>,
}

impl ResourceManager {
    pub fn new(cleanup_queue: Arc<CleanupQueue>) -> Self {
        info!("ResourceManager initialized");
        Self { cleanup_queue }
    }

    pub fn guard_temp_dir(&self, path: PathBuf) -> TempDirGuard {
        TempDirGuard::new(path, self.cleanup_queue.clone())
    }

    pub fn defer_cleanup<F>(&self, cleanup_fn: F) -> ScopeGuard<(), F>
    where
        F: FnOnce(()),
    {
        guard((), cleanup_fn)
    }

    pub fn cleanup_batch(&self, paths: &[PathBuf]) -> usize {
        let mut success_count = 0;

        for path in paths {
            if !path.exists() {
                success_count += 1;
                continue;
            }

            match fs::remove_dir_all(path) {
                Ok(_) => {
                    info!("Successfully cleaned up: {}", path.display());
                    success_count += 1;
                }
                Err(e) => {
                    warn!(
                        "Failed to clean up {}: {}, adding to queue",
                        path.display(),
                        e
                    );
                    self.cleanup_queue.push(CleanupItem {
                        path: path.clone(),
                        retry_count: 0,
                    });
                }
            }
        }

        info!(
            "Batch cleanup: {}/{} successful",
            success_count,
            paths.len()
        );
        success_count
    }
}

/// 创建带自动清理的临时目录
pub fn create_guarded_temp_dir(
    base_dir: &Path,
    prefix: &str,
    cleanup_queue: Arc<CleanupQueue>,
) -> Result<TempDirGuard, String> {
    if !base_dir.exists() {
        fs::create_dir_all(base_dir)
            .map_err(|e| format!("Failed to create base directory: {}", e))?;
    }

    let temp_name = format!("{}{}", prefix, uuid::Uuid::new_v4());
    let temp_path = base_dir.join(temp_name);

    fs::create_dir_all(&temp_path)
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;

    info!("Created guarded temp directory: {}", temp_path.display());

    Ok(TempDirGuard::new(temp_path, cleanup_queue))
}
