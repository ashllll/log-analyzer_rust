//! WorkspaceId — validated workspace identifier.

use once_cell::sync::Lazy;
use regex::Regex;

static WORKSPACE_ID_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z0-9]([a-zA-Z0-9_-]*[a-zA-Z0-9])?$").unwrap());

pub const MAX_WORKSPACE_ID_LENGTH: usize = 50;

/// A workspace identifier that has passed format and length validation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkspaceId(String);

impl WorkspaceId {
    pub fn new(id: &str) -> Result<Self, String> {
        let trimmed = id.trim();
        if trimmed.is_empty() {
            return Err("Workspace ID cannot be empty".to_string());
        }
        if trimmed.len() > MAX_WORKSPACE_ID_LENGTH {
            return Err(format!(
                "Workspace ID too long (max {MAX_WORKSPACE_ID_LENGTH} chars)"
            ));
        }
        if !WORKSPACE_ID_REGEX.is_match(trimmed) {
            return Err(format!(
                "Invalid workspace ID '{trimmed}': must contain only alphanumeric characters, hyphens, and underscores"
            ));
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_ids_accepted() {
        assert!(WorkspaceId::new("ws-1").is_ok());
        assert!(WorkspaceId::new("my_project").is_ok());
        assert!(WorkspaceId::new("abc123").is_ok());
    }

    #[test]
    fn rejects_empty() {
        assert!(WorkspaceId::new("").is_err());
        assert!(WorkspaceId::new("   ").is_err());
    }

    #[test]
    fn rejects_too_long() {
        let long = "a".repeat(MAX_WORKSPACE_ID_LENGTH + 1);
        assert!(WorkspaceId::new(&long).is_err());
    }

    #[test]
    fn rejects_special_chars() {
        assert!(WorkspaceId::new("../etc").is_err());
        assert!(WorkspaceId::new("foo/bar").is_err());
        assert!(WorkspaceId::new("a b").is_err());
    }

    #[test]
    fn display_returns_inner() {
        let id = WorkspaceId::new("ws-1").unwrap();
        assert_eq!(id.to_string(), "ws-1");
        assert_eq!(id.as_str(), "ws-1");
    }
}
