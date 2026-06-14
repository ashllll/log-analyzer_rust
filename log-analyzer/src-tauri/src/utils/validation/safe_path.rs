//! SafePath — validated, canonical path that has passed traversal checks.

use std::path::{Path, PathBuf};

/// A filesystem path that has been validated against path-traversal attacks
/// and canonicalized relative to a trusted base directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafePath(PathBuf);

impl SafePath {
    /// Validate and canonicalize `path` relative to `base_dir`.
    ///
    /// # Errors
    /// - Path contains traversal sequences (`..`, `~`, null bytes)
    /// - After canonicalization, path is outside `base_dir`
    pub fn new(path: &str, base_dir: &Path) -> Result<Self, String> {
        // Block null bytes
        if path.contains('\0') {
            return Err("Path contains null byte".to_string());
        }
        // Block home-directory expansion
        if path.contains('~') {
            return Err("Path contains '~', which is not allowed".to_string());
        }
        // Block traversal sequences in raw input
        if path.contains("..") {
            return Err("Path traversal detected ('..')".to_string());
        }

        let candidate = PathBuf::from(path);
        let full = if candidate.is_absolute() {
            candidate
        } else {
            base_dir.join(&candidate)
        };

        // Canonicalize both paths (symlink-aware)
        let base_canonical = base_dir
            .canonicalize()
            .map_err(|e| format!("Cannot resolve base directory '{}': {e}", base_dir.display()))?;
        let canonical = full
            .canonicalize()
            .map_err(|e| format!("Cannot resolve path '{path}': {e}"))?;

        // Verify the canonical path stays inside the canonical base
        if !canonical.starts_with(&base_canonical) {
            return Err(format!(
                "Path '{path}' escapes base directory '{}'",
                base_dir.display()
            ));
        }

        Ok(Self(canonical))
    }

    /// Create from an already-validated path (skip checks).
    pub(crate) fn from_validated(path: PathBuf) -> Self {
        Self(path)
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }

    pub fn into_inner(self) -> PathBuf {
        self.0
    }
}

impl AsRef<Path> for SafePath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl std::fmt::Display for SafePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.display().fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_relative_path_within_base() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();

        let sp = SafePath::new("sub", dir.path()).unwrap();
        assert!(sp.as_path().ends_with("sub"));
    }

    #[test]
    fn rejects_null_byte() {
        let dir = tempfile::tempdir().unwrap();
        assert!(SafePath::new("foo\0bar", dir.path()).is_err());
    }

    #[test]
    fn rejects_tilde() {
        let dir = tempfile::tempdir().unwrap();
        assert!(SafePath::new("~/secret", dir.path()).is_err());
    }

    #[test]
    fn rejects_parent_traversal_in_raw_input() {
        let dir = tempfile::tempdir().unwrap();
        // Traversal in raw string is caught before filesystem access
        assert!(SafePath::new("../etc/passwd", dir.path()).is_err());
    }

    #[test]
    fn rejects_escape_via_symlink() {
        // This test is Linux-specific (symlinks). On Windows we skip.
        let dir = tempfile::tempdir().unwrap();
        let link = dir.path().join("escape");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("/etc", &link).unwrap();
            assert!(SafePath::new("escape/passwd", dir.path()).is_err());
        }
        #[cfg(not(unix))]
        {
            // On Windows, just verify the test compiles
            let _ = link;
        }
    }

    #[test]
    fn as_ref_returns_correct_path() {
        let dir = tempfile::tempdir().unwrap();
        let sp = SafePath::new(".", dir.path()).unwrap();
        let p: &Path = sp.as_ref();
        assert!(p.is_absolute());
    }
}
