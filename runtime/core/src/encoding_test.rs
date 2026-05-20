use super::*;
use crate::RuntimeError;

#[test]
fn test_encoding_round_trip() -> crate::RuntimeResult<()> {
    let original = "Hello, Vector! 🦀";
    let encoded = Encoding::encode(original);
    let decoded = Encoding::decode(&encoded)?;

    assert_eq!(original, decoded);
    Ok(())
}

#[test]
fn test_encoding_invalid_utf8() {
    // 0xFF is invalid UTF-8
    let invalid_bytes = vec![0xFF, 0x00];
    let result = Encoding::decode(&invalid_bytes);

    assert!(result.is_err());
    if let Err(RuntimeError::Encoding(msg)) = result {
        assert!(msg.to_lowercase().contains("utf-8"));
    } else {
        // Use a standard assert that doesn't trigger clippy::panic if possible,
        // or just allow it for the test.
        #[allow(clippy::panic)]
        {
            panic!("Expected Encoding error");
        }
    }
}

#[test]
fn test_encoding_empty() -> crate::RuntimeResult<()> {
    let encoded = Encoding::encode("");
    let decoded = Encoding::decode(&encoded)?;

    assert_eq!(decoded, "");
    Ok(())
}
