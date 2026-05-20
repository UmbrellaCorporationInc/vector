//! Slug validation for document frontmatter.

use std::fmt;

/// Error returned when a slug fails validation.
///
/// # DTO(Internal error indicator — not a plugin operation contract)
#[non_exhaustive]
pub struct SlugError {
    /// The reason why the slug validation failed.
    pub reason: String,
}

impl fmt::Debug for SlugError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SlugError: {}", self.reason)
    }
}

impl fmt::Display for SlugError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.reason)
    }
}

impl std::error::Error for SlugError {}

const fn is_valid_slug_char(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'
}

fn has_consecutive_hyphens(s: &str) -> bool {
    s.contains("--")
}

fn starts_or_ends_with_hyphen(s: &str) -> bool {
    s.starts_with('-') || s.ends_with('-')
}

/// Validates a slug string.
///
/// # Errors
/// Returns an error if the slug:
///     - Is empty
///     - Starts or ends with a hyphen
///     - Contains consecutive hyphens
///     - Contains non-lowercase ASCII letters, digits, or hyphens
pub fn validate_slug(slug: &str) -> Result<(), SlugError> {
    if slug.is_empty() {
        return Err(SlugError { reason: "Slug cannot be empty".to_string() });
    }

    if starts_or_ends_with_hyphen(slug) {
        return Err(SlugError { reason: "Slug must not start or end with a hyphen".to_string() });
    }

    if has_consecutive_hyphens(slug) {
        return Err(SlugError { reason: "Slug must not contain consecutive hyphens".to_string() });
    }

    if !slug.chars().all(is_valid_slug_char) {
        return Err(SlugError {
            reason: "Slug must contain only lowercase ASCII letters, ASCII digits, and hyphens"
                .to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
#[path = "slug_test.rs"]
mod tests;
