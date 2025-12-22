use std::path::Path;

#[test]
fn debug_create_multi_file_archive() {
    let temp_dir = tempfile::tempdir().unwrap();
    let archive_path = temp_dir.path().join("test.zip");
    
    // Create archive
    let file_names = super::create_multi_file_archive(&archive_path, 5, 1000).unwrap();
    
    println!("Created archive with {} files", file_names.len());
    println!("Archive path: {:?}", archive_path);
    println!("Archive exists: {}", archive_path.exists());
    println!("Archive size: {} bytes", std::fs::metadata(&archive_path).unwrap().len());
    
    // Try to read the ZIP
    let file = std::fs::File::open(&archive_path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    
    println!("ZIP contains {} files", archive.len());
    
    for i in 0..archive.len() {
        let file = archive.by_index(i).unwrap();
        println!("  File {}: {} ({} bytes)", i, file.name(), file.size());
    }
    
    assert_eq!(archive.len(), 5, "ZIP should contain 5 files");
}
