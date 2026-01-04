use std::path::Path;

fn main() {
    let temp = std::env::temp_dir();
    let test_path = temp.join("test.txt");
    std::fs::write(&test_path, "test").unwrap();
    
    let canonical = std::fs::canonicalize(&test_path).unwrap();
    println!("Canonical path: {:?}", canonical);
    println!("Canonical string: {}", canonical.display());
    println!("Starts with \\?\: {}", canonical.to_string_lossy().starts_with("\\?\\"));
    
    // 使用 dunce
    let dunce_canonical = dunce::canonicalize(&test_path).unwrap();
    println!("Dunce canonical: {:?}", dunce_canonical);
    println!("Dunce string: {}", dunce_canonical.display());
    println!("Dunce starts with \\?\: {}", dunce_canonical.to_string_lossy().starts_with("\\?\\"));
    
    std::fs::remove_file(&test_path).ok();
}
