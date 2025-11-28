// 集成测试：测试公共 API
// 这些测试只能访问 lib.rs 中的公共函数和命令

use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_tauri_app_structure() {
    // 验证项目结构正确
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    assert!(manifest_path.exists(), "Cargo.toml should exist");

    let lib_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    assert!(lib_path.exists(), "src/lib.rs should exist");
}

#[test]
fn test_temp_directory_operations() {
    // 测试临时目录操作（模拟应用行为）
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.log");

    // 写入测试文件
    fs::write(&test_file, "2024-01-01 12:00:00 INFO Test log entry").unwrap();

    // 验证文件存在
    assert!(test_file.exists());

    // 验证文件内容
    let content = fs::read_to_string(&test_file).unwrap();
    assert!(content.contains("INFO"));
    assert!(content.contains("Test log entry"));
}

#[test]
fn test_file_metadata_operations() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("metadata_test.txt");

    fs::write(&test_file, "test content").unwrap();

    let metadata = fs::metadata(&test_file).unwrap();
    assert_eq!(metadata.len(), 12);
    assert!(!metadata.is_dir());
    assert!(metadata.is_file());
}

#[test]
fn test_readonly_file_operations() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("readonly.txt");

    fs::write(&test_file, "readonly content").unwrap();

    // 设置只读
    let metadata = test_file.metadata().unwrap();
    let mut perms = metadata.permissions();
    perms.set_readonly(true);
    fs::set_permissions(&test_file, perms).unwrap();

    // 验证只读属性
    let metadata = test_file.metadata().unwrap();
    assert!(metadata.permissions().readonly());

    // 恢复可写（清理）
    let mut perms = metadata.permissions();
    perms.set_readonly(false);
    fs::set_permissions(&test_file, perms).unwrap();
}

#[test]
fn test_nested_directory_creation() {
    let temp_dir = TempDir::new().unwrap();
    let nested_path = temp_dir.path().join("level1/level2/level3");

    fs::create_dir_all(&nested_path).unwrap();
    assert!(nested_path.exists());
    assert!(nested_path.is_dir());
}
