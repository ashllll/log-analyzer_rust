use la_core::error::{AppError, Result};
use std::io::ErrorKind;
use std::path::Path;

fn metadata_if_exists(path: &Path) -> Result<Option<std::fs::Metadata>> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(AppError::archive_error(
            format!(
                "Failed to inspect extraction path {}: {}",
                path.display(),
                error
            ),
            Some(path.to_path_buf()),
        )),
    }
}

fn ensure_path_is_not_symlink(path: &Path) -> Result<()> {
    if let Some(metadata) = metadata_if_exists(path)? {
        if metadata.file_type().is_symlink() {
            return Err(AppError::archive_error(
                format!(
                    "Refusing to extract through symbolic link path: {}",
                    path.display()
                ),
                Some(path.to_path_buf()),
            ));
        }
    }

    Ok(())
}

pub fn ensure_no_symlink_components(root: &Path, candidate: &Path) -> Result<()> {
    let relative = candidate.strip_prefix(root).map_err(|_| {
        AppError::archive_error(
            format!(
                "Extraction target {} is outside the allowed root {}",
                candidate.display(),
                root.display()
            ),
            Some(candidate.to_path_buf()),
        )
    })?;

    let mut current = root.to_path_buf();
    ensure_path_is_not_symlink(&current)?;

    for component in relative.components() {
        current.push(component.as_os_str());
        ensure_path_is_not_symlink(&current)?;
    }

    Ok(())
}

fn remove_symlink(path: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        AppError::archive_error(
            format!(
                "Failed to inspect extracted path {}: {}",
                path.display(),
                error
            ),
            Some(path.to_path_buf()),
        )
    })?;

    if !metadata.file_type().is_symlink() {
        return Ok(());
    }

    std::fs::remove_file(path).map_err(|error| {
        AppError::archive_error(
            format!(
                "Failed to remove extracted symbolic link {}: {}",
                path.display(),
                error
            ),
            Some(path.to_path_buf()),
        )
    })?;

    Ok(())
}

pub fn reject_extracted_symlink(path: &Path) -> Result<()> {
    if let Some(metadata) = metadata_if_exists(path)? {
        if metadata.file_type().is_symlink() {
            remove_symlink(path)?;
            return Err(AppError::archive_error(
                format!(
                    "Refusing extracted symbolic link entry at {}",
                    path.display()
                ),
                Some(path.to_path_buf()),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn rejects_existing_symlink_in_parent_chain() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let root = temp_dir.path().join("root");
        let real = root.join("real");
        let link = root.join("link");

        std::fs::create_dir_all(&real).unwrap();
        std::fs::create_dir_all(&root).unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();

        let candidate = link.join("nested.txt");
        let error = ensure_no_symlink_components(&root, &candidate).unwrap_err();

        assert!(error.to_string().contains("symbolic link"));
    }

    #[cfg(unix)]
    #[test]
    fn removes_extracted_symlink_and_returns_error() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let target = temp_dir.path().join("target.txt");
        let linked = temp_dir.path().join("linked.txt");

        std::fs::write(&target, "content").unwrap();
        std::os::unix::fs::symlink(&target, &linked).unwrap();

        let error = reject_extracted_symlink(&linked).unwrap_err();

        assert!(error.to_string().contains("symbolic link"));
        assert!(!linked.exists());
        assert!(std::fs::symlink_metadata(&linked).is_err());
    }
}
