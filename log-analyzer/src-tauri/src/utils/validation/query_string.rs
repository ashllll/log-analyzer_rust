//! QueryString — validated search query string.

pub const MAX_QUERY_LENGTH: usize = 1000;

/// A search query string that has passed length validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryString(String);

impl QueryString {
    pub fn new(query: &str) -> Result<Self, String> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err("Search query cannot be empty".to_string());
        }
        if trimmed.len() > MAX_QUERY_LENGTH {
            return Err(format!(
                "Search query too long (max {MAX_QUERY_LENGTH} characters)"
            ));
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for QueryString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_query_accepted() {
        assert!(QueryString::new("error").is_ok());
        assert!(QueryString::new("error|timeout").is_ok());
    }

    #[test]
    fn rejects_empty() {
        assert!(QueryString::new("").is_err());
        assert!(QueryString::new("   ").is_err());
    }

    #[test]
    fn rejects_too_long() {
        let long = "x".repeat(MAX_QUERY_LENGTH + 1);
        assert!(QueryString::new(&long).is_err());
    }

    #[test]
    fn max_length_accepted() {
        let max = "x".repeat(MAX_QUERY_LENGTH);
        assert!(QueryString::new(&max).is_ok());
    }

    #[test]
    fn display_returns_trimmed() {
        let q = QueryString::new("  hello  ").unwrap();
        assert_eq!(q.as_str(), "hello");
    }
}
