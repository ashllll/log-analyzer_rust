//! 路径追踪器模块
//!
//! 使用HashSet在内存中追踪已解压的路径,优化冲突检测性能

use std::collections::HashSet;
use std::path::PathBuf;

/// 路径追踪器
///
/// 用于在解压过程中追踪已解压的路径,避免重复的文件系统调用
#[derive(Debug)]
pub struct PathTracker {
    /// 已解压的所有路径(文件和目录)
    extracted_paths: HashSet<PathBuf>,
}

impl PathTracker {
    /// 创建新的路径追踪器
    pub fn new() -> Self {
        Self {
            extracted_paths: HashSet::new(),
        }
    }

    /// 检查路径是否已存在
    ///
    /// # Arguments
    ///
    /// * `path` - 要检查的路径
    ///
    /// # Returns
    ///
    /// 如果路径已存在返回true,否则返回false
    pub fn exists(&self, path: &PathBuf) -> bool {
        self.extracted_paths.contains(path)
    }

    /// 添加已解压路径
    ///
    /// # Arguments
    ///
    /// * `path` - 已解压的路径
    pub fn add(&mut self, path: PathBuf) {
        self.extracted_paths.insert(path);
    }

    /// 获取已追踪路径数量
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.extracted_paths.len()
    }

    /// 检查追踪器是否为空
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.extracted_paths.is_empty()
    }
}

impl Default for PathTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let tracker = PathTracker::new();
        assert_eq!(tracker.len(), 0);
        assert!(tracker.is_empty());
    }

    #[test]
    fn test_add_and_exists() {
        let mut tracker = PathTracker::new();
        let path = PathBuf::from("test/file.log");

        assert!(!tracker.exists(&path));

        tracker.add(path.clone());
        assert!(tracker.exists(&path));
        assert_eq!(tracker.len(), 1);
    }

    #[test]
    fn test_multiple_paths() {
        let mut tracker = PathTracker::new();

        tracker.add(PathBuf::from("test/file1.log"));
        tracker.add(PathBuf::from("test/file2.log"));
        tracker.add(PathBuf::from("test/dir/file3.log"));

        assert_eq!(tracker.len(), 3);
        assert!(tracker.exists(&PathBuf::from("test/file1.log")));
        assert!(tracker.exists(&PathBuf::from("test/file2.log")));
        assert!(tracker.exists(&PathBuf::from("test/dir/file3.log")));
        assert!(!tracker.exists(&PathBuf::from("test/file4.log")));
    }

    #[test]
    fn test_duplicate_add() {
        let mut tracker = PathTracker::new();
        let path = PathBuf::from("test/file.log");

        tracker.add(path.clone());
        tracker.add(path.clone()); // 重复添加

        assert_eq!(tracker.len(), 1); // HashSet自动去重
    }
}
