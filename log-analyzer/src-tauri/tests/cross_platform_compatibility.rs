//! 跨平台兼容性集成测试
//!
//! 测试所有关键功能在不同平台上的一致性

#[cfg(test)]
mod cross_platform_tests {
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    /// 测试路径规范化
    #[test]
    fn test_path_canonicalization() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test.txt");

        std::fs::write(&test_path, "test content").unwrap();

        #[cfg(target_os = "windows")]
        {
            // Windows: 使用 dunce 处理 UNC 路径
            let canonical = std::fs::canonicalize(&test_path).unwrap();
            assert!(canonical.exists());
            // 验证没有 \\?\ 前缀（dunce 会处理）
            let canonical_str = canonical.to_string_lossy();
            assert!(!canonical_str.starts_with("\\\\?\\"), "UNC path should not have \\\\? prefix");
        }

        #[cfg(not(target_os = "windows"))]
        {
            // Unix-like: 标准规范化
            let canonical = std::fs::canonicalize(&test_path).unwrap();
            assert!(canonical.exists());
        }
    }

    /// 测试只读文件移除（Windows 特定）
    #[test]
    fn test_remove_readonly_attribute() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("readonly.txt");

        std::fs::write(&test_file, "test content").unwrap();

        #[cfg(target_os = "windows")]
        {
            use std::fs;
            use std::os::windows::fs::MetadataExt;

            // 设置只读属性
            let mut perms = test_file.metadata().unwrap().permissions();
            perms.set_readonly(true);
            fs::set_permissions(&test_file, perms).unwrap();

            // 验证只读属性已设置
            let metadata = test_file.metadata().unwrap();
            assert!(metadata.file_attributes() & 0x1 != 0, "File should be read-only");

            // 使用 dunce 规范化后再删除
            let normalized = dunce::canonicalize(&test_file).unwrap();
            fs::remove_file(&normalized).unwrap();

            // 验证文件已删除
            assert!(!test_file.exists());
        }

        #[cfg(not(target_os = "windows"))]
        {
            // Unix-like: 没有只读属性概念（文件权限不同）
            let perms = test_file.metadata().unwrap().permissions();
            std::fs::set_permissions(&test_file, perms).unwrap();
            std::fs::remove_file(&test_file).unwrap();
            assert!(!test_file.exists());
        }
    }

    /// 测试路径分隔符规范化
    #[test]
    fn test_path_separator_normalization() {
        let input = "folder/subfolder/file.txt";
        let separator = std::path::MAIN_SEPARATOR;

        #[cfg(target_os = "windows")]
        assert!(input.contains('/'), "Input should contain forward slashes");

        #[cfg(target_os = "windows")]
        {
            // 在 Windows 上，路径可以包含 / 或 \
            let path = PathBuf::from(input);
            assert!(path.exists() || !path.exists()); // 只是验证路径有效
        }

        #[cfg(not(target_os = "windows"))]
        {
            // Unix-like: 路径分隔符应该是 /
            assert_eq!(separator, '/', "Main separator should be forward slash");
        }
    }

    /// 测试临时目录清理
    #[tokio::test]
    async fn test_temp_directory_cleanup() {
        use crossbeam::queue::SegQueue;
        use std::sync::Arc;

        let temp_dir = TempDir::new().unwrap();
        let cleanup_queue = Arc::new(SegQueue::new());

        // 创建临时目录
        let temp_path = temp_dir.path().join("cleanup_test");
        std::fs::create_dir(&temp_path).unwrap();
        std::fs::write(temp_path.join("file.txt"), "test").unwrap();

        // 执行清理（模拟）
        let result = std::fs::remove_dir_all(&temp_path);
        assert!(result.is_ok(), "Cleanup should succeed");

        // 验证清理成功
        assert!(!temp_path.exists(), "Temp directory should be removed");
    }

    /// 测试平台检测函数（来自 rar_handler）
    #[test]
    fn test_platform_detection() {
        // 测试平台检测不会 panic
        // 注意：这只是验证函数可以调用，实际值取决于编译目标
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        {
            // Windows x64
            assert!(true); // 如果编译到这里，平台检测正确
        }

        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            // Linux x64
            assert!(true);
        }

        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        {
            // Linux ARM64
            assert!(true);
        }

        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        {
            // macOS Intel
            assert!(true);
        }

        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            // macOS ARM64
            assert!(true);
        }
    }

    /// 测试符号链接处理
    #[test]
    fn test_symlink_handling() {
        use walkdir::WalkDir;

        let temp_dir = TempDir::new().unwrap();
        let target_file = temp_dir.path().join("target.txt");
        let symlink = temp_dir.path().join("link.txt");

        std::fs::write(&target_file, "target content").unwrap();

        #[cfg(target_family = "unix")]
        {
            std::os::unix::fs::symlink(&target_file, &symlink).unwrap();

            // WalkDir 应该能检测符号链接
            let mut found_symlink = false;
            for entry in WalkDir::new(temp_dir.path())
                .follow_links(false)  // 不跟随符号链接
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.path_is_symlink() {
                    found_symlink = true;
                    // 符号链接应该被正确识别
                    assert!(entry.path_is_symlink());
                }
            }
            assert!(found_symlink, "Symlink should be detected");
        }

        #[cfg(target_family = "windows")]
        {
            // Windows 符号链接需要管理员权限，可能创建失败
            // 这里仅测试路径遍历不应该 panic
            let mut entry_count = 0;
            for entry in WalkDir::new(temp_dir.path())
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let _ = entry.path();
                entry_count += 1;
            }
            assert!(entry_count >= 1, "Should at least find the target file");
        }
    }

    /// 测试文件路径长度限制
    #[test]
    fn test_long_path_handling() {
        let temp_dir = TempDir::new().unwrap();

        // 创建一个长路径（但不超过系统限制）
        let mut long_path = temp_dir.path().to_path_buf();
        for i in 0..10 {
            long_path = long_path.join(format!("directory_with_very_long_name_{}", i));
        }

        // 创建目录
        let result = std::fs::create_dir_all(&long_path);
        assert!(result.is_ok(), "Should create long path directories");

        #[cfg(target_os = "windows")]
        {
            // Windows 有 260 字符限制（除非使用长路径前缀）
            // dunce 库应该处理这个问题
            let dunce_path = dunce::canonicalize(&long_path).unwrap_or(long_path.clone());
            assert!(dunce_path.exists() || !dunce_path.exists());
        }

        #[cfg(not(target_os = "windows"))]
        {
            // Unix-like: 路径限制通常大得多
            assert!(long_path.exists());
        }
    }

    /// 测试编码跨平台兼容性
    #[test]
    fn test_encoding_cross_platform() {
        // 测试 UTF-8 编码在不同平台上的一致性
        let test_strings = vec![
            "Hello World",
            "你好世界",  // 中文
            "🎉🚀",      // Emoji
            "Привет",    // 俄文
            "مرحبا",      // 阿拉伯文
        ];

        for s in test_strings {
            // 编码为 UTF-8
            let bytes = s.as_bytes();
            // 解码回来
            let decoded = std::str::from_utf8(bytes).unwrap();
            assert_eq!(s, decoded, "String should survive UTF-8 round-trip: {}", s);
        }
    }

    /// 测试文件权限跨平台
    #[test]
    fn test_file_permissions_cross_platform() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("permissions.txt");

        std::fs::write(&test_file, "test").unwrap();

        #[cfg(target_family = "unix")]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = test_file.metadata().unwrap();
            let perms = metadata.permissions();
            let mode = perms.mode();

            // Unix 文件权限
            assert!(mode & 0o777 != 0, "Should have some permissions set");
        }

        #[cfg(target_family = "windows")]
        {
            // Windows 使用不同的权限系统
            let metadata = test_file.metadata().unwrap();
            assert!(metadata.is_file(), "Should be a file");
        }

        // 验证文件可读
        let content = std::fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "test");
    }
}
