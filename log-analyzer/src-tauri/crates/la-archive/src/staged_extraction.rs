use la_core::error::{AppError, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use walkdir::WalkDir;

/// Isolates archive output until all post-extraction checks have passed.
pub(crate) struct StagedExtraction {
    directory: TempDir,
    target_dir: PathBuf,
}

impl StagedExtraction {
    pub(crate) async fn new(target_dir: &Path) -> Result<Self> {
        let requested_parent = usable_parent(target_dir);
        tokio::fs::create_dir_all(&requested_parent)
            .await
            .map_err(|error| {
                AppError::archive_error(
                    format!("Failed to create extraction parent directory: {error}"),
                    Some(requested_parent.clone()),
                )
            })?;
        let parent = tokio::fs::canonicalize(&requested_parent)
            .await
            .map_err(|error| {
                AppError::archive_error(
                    format!("Failed to resolve extraction parent directory: {error}"),
                    Some(requested_parent),
                )
            })?;
        let target_name = target_dir.file_name().ok_or_else(|| {
            AppError::validation_error(format!(
                "Extraction target must name a directory: {}",
                target_dir.display()
            ))
        })?;
        let target_dir = parent.join(target_name);

        let directory = tempfile::Builder::new()
            .prefix(".archive-extract-")
            .tempdir_in(&parent)
            .map_err(|error| {
                AppError::archive_error(
                    format!("Failed to create staged extraction directory: {error}"),
                    Some(parent),
                )
            })?;

        Ok(Self {
            directory,
            target_dir,
        })
    }

    pub(crate) fn path(&self) -> &Path {
        self.directory.path()
    }

    pub(crate) async fn commit(self) -> Result<()> {
        let target_dir = self.target_dir.clone();
        let failure_path = target_dir.clone();
        tokio::task::spawn_blocking(move || publish(self.directory, &target_dir))
            .await
            .map_err(|error| {
                AppError::archive_error(
                    format!("Staged extraction publish task failed: {error}"),
                    Some(failure_path),
                )
            })?
    }
}

fn usable_parent(path: &Path) -> PathBuf {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

fn publish(staging: TempDir, target_dir: &Path) -> Result<()> {
    let Some(target_metadata) = metadata_if_exists(target_dir)? else {
        return fs::rename(staging.path(), target_dir).map_err(|error| {
            AppError::archive_error(
                format!("Failed to publish staged extraction: {error}"),
                Some(target_dir.to_path_buf()),
            )
        });
    };
    if target_metadata.file_type().is_symlink() || !target_metadata.is_dir() {
        return Err(AppError::security_error(format!(
            "Extraction target must be a real directory: {}",
            target_dir.display()
        )));
    }

    let parent = usable_parent(target_dir);
    let backup = tempfile::Builder::new()
        .prefix(".archive-backup-")
        .tempdir_in(&parent)
        .map_err(|error| {
            AppError::archive_error(
                format!("Failed to create extraction rollback directory: {error}"),
                Some(target_dir.to_path_buf()),
            )
        })?;
    let detached_target = backup.path().join("original");

    fs::rename(target_dir, &detached_target).map_err(|error| {
        AppError::archive_error(
            format!("Failed to isolate existing extraction output: {error}"),
            Some(target_dir.to_path_buf()),
        )
    })?;

    if let Err(error) = prepare_merged_staging(&detached_target, staging.path()) {
        return Err(restore_original(
            backup,
            &detached_target,
            target_dir,
            error.to_string(),
        ));
    }

    if let Err(error) = fs::rename(staging.path(), target_dir) {
        return Err(restore_original(
            backup,
            &detached_target,
            target_dir,
            format!("Failed to publish merged extraction output: {error}"),
        ));
    }

    Ok(())
}

fn prepare_merged_staging(existing_dir: &Path, staging_dir: &Path) -> Result<()> {
    validate_real_tree(existing_dir, "existing extraction output")?;
    validate_real_tree(staging_dir, "staged extraction")?;
    copy_missing_entries(existing_dir, staging_dir)
}

fn validate_real_tree(root: &Path, description: &str) -> Result<()> {
    for entry in WalkDir::new(root).min_depth(1) {
        let entry = entry.map_err(|error| {
            AppError::archive_error(
                format!("Failed to inspect {description}: {error}"),
                Some(root.to_path_buf()),
            )
        })?;
        if entry.file_type().is_symlink() {
            return Err(AppError::security_error(format!(
                "{description} contains a symbolic link: {}",
                entry.path().display()
            )));
        }
        if !entry.file_type().is_dir() && !entry.file_type().is_file() {
            return Err(AppError::security_error(format!(
                "{description} contains an unsupported file type: {}",
                entry.path().display()
            )));
        }
    }
    Ok(())
}

fn copy_missing_entries(existing_dir: &Path, staging_dir: &Path) -> Result<()> {
    for entry in fs::read_dir(existing_dir).map_err(|error| {
        AppError::archive_error(
            format!("Failed to read existing extraction output: {error}"),
            Some(existing_dir.to_path_buf()),
        )
    })? {
        let entry = entry.map_err(|error| {
            AppError::archive_error(
                format!("Failed to inspect existing extraction entry: {error}"),
                Some(existing_dir.to_path_buf()),
            )
        })?;
        let source = entry.path();
        let destination = staging_dir.join(entry.file_name());
        let source_metadata = entry.metadata().map_err(|error| {
            AppError::archive_error(
                format!("Failed to inspect existing extraction entry: {error}"),
                Some(source.clone()),
            )
        })?;

        match metadata_if_exists(&destination)? {
            Some(destination_metadata)
                if source_metadata.is_dir() && destination_metadata.is_dir() =>
            {
                copy_missing_entries(&source, &destination)?;
            }
            Some(_) => {}
            None if source_metadata.is_dir() => {
                fs::create_dir(&destination).map_err(|error| {
                    AppError::archive_error(
                        format!("Failed to preserve extraction directory: {error}"),
                        Some(destination.clone()),
                    )
                })?;
                fs::set_permissions(&destination, source_metadata.permissions()).map_err(
                    |error| {
                        AppError::archive_error(
                            format!("Failed to preserve extraction permissions: {error}"),
                            Some(destination.clone()),
                        )
                    },
                )?;
                copy_missing_entries(&source, &destination)?;
            }
            None => {
                fs::hard_link(&source, &destination)
                    .or_else(|_| fs::copy(&source, &destination).map(|_| ()))
                    .map_err(|error| {
                        AppError::archive_error(
                            format!("Failed to preserve existing extraction file: {error}"),
                            Some(destination),
                        )
                    })?;
            }
        }
    }
    Ok(())
}

fn metadata_if_exists(path: &Path) -> Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(AppError::archive_error(
            format!("Failed to inspect extraction path: {error}"),
            Some(path.to_path_buf()),
        )),
    }
}

fn restore_original(
    backup: TempDir,
    detached_target: &Path,
    target_dir: &Path,
    message: String,
) -> AppError {
    if metadata_if_exists(target_dir).ok().flatten().is_none() {
        match fs::rename(detached_target, target_dir) {
            Ok(()) => return AppError::archive_error(message, Some(target_dir.to_path_buf())),
            Err(error) => {
                let recovery_path = backup.keep();
                return AppError::archive_error(
                    format!(
                        "{message}; failed to restore original output: {error}; recovery data retained at {}",
                        recovery_path.display()
                    ),
                    Some(target_dir.to_path_buf()),
                );
            }
        }
    }

    let recovery_path = backup.keep();
    AppError::archive_error(
        format!(
            "{message}; target was recreated concurrently; original output retained at {}",
            recovery_path.display()
        ),
        Some(target_dir.to_path_buf()),
    )
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[tokio::test]
    async fn commit_rejects_symlinks_inside_existing_output_and_restores_it() {
        let temp_dir = tempfile::tempdir().unwrap();
        let target_dir = temp_dir.path().join("output");
        let external_dir = temp_dir.path().join("external");
        fs::create_dir_all(&target_dir).unwrap();
        fs::create_dir_all(&external_dir).unwrap();
        symlink(&external_dir, target_dir.join("escape")).unwrap();

        let staged = StagedExtraction::new(&target_dir).await.unwrap();
        fs::write(staged.path().join("new.log"), b"new").unwrap();

        let error = staged
            .commit()
            .await
            .expect_err("existing symlinks must abort publication");

        assert!(error.to_string().contains("symbolic link"));
        assert!(target_dir.join("escape").is_symlink());
        assert!(!target_dir.join("new.log").exists());
        assert!(fs::read_dir(&external_dir).unwrap().next().is_none());
    }
}
