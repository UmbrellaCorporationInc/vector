use crate::RuntimeResult;

/// Stateless UTF-8 encoding/decoding primitive.
///
/// `Encoding` provides a canonical boundary for text conversion within the runtime,
/// ensuring consistent UTF-8 enforcement across all crates.
#[non_exhaustive]
pub struct Encoding;

impl Encoding {
    /// Encodes a string slice into a UTF-8 byte vector.
    ///
    /// This operation is infallible as all Rust strings are guaranteed to be valid UTF-8.
    #[must_use]
    pub fn encode(text: &str) -> Vec<u8> {
        text.as_bytes().to_vec()
    }

    /// Decodes a byte slice into a UTF-8 string.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::Encoding`] if the bytes are not valid UTF-8.
    pub fn decode(bytes: &[u8]) -> RuntimeResult<String> {
        String::from_utf8(bytes.to_vec()).map_err(|e| crate::RuntimeError::Encoding(e.to_string()))
    }
}

#[cfg(test)]
#[path = "encoding_test.rs"]
mod tests;
