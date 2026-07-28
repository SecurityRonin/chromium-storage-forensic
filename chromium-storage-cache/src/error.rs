//! Error type for the Simple Cache reader.
//!
//! Every variant carries the offending value and its offset so an "invalid X" is
//! never a dead end — the raw bytes and location travel with the error
//! (fail-loud, show-the-value discipline).

use std::fmt;

/// Errors returned while parsing a Simple Cache entry file. Parsing is
/// panic-free: every malformed input becomes one of these rather than a panic or
/// over-allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CacheError {
    /// The buffer was shorter than a `SimpleFileHeader` (24 bytes).
    TooShort {
        /// Bytes actually available.
        available: usize,
    },
    /// The leading 64-bit magic was not the Simple Cache entry-header magic.
    /// Carries the eight bytes found so the caller can identify the file.
    BadMagic {
        /// The eight magic bytes found (verbatim).
        found: [u8; 8],
    },
    /// The header's `key_length` field runs past the end of the buffer.
    KeyLengthOutOfRange {
        /// The declared key length.
        key_length: u64,
        /// Bytes available after the header.
        available: usize,
    },
}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CacheError::TooShort { available } => write!(
                f,
                "buffer too short for a Simple Cache header: {available} bytes available (need 24)"
            ),
            CacheError::BadMagic { found } => write!(
                f,
                "bad Simple Cache entry magic: found {found:02x?} \
                 (expected 305c72a71b6dfbfc little-endian)"
            ),
            CacheError::KeyLengthOutOfRange {
                key_length,
                available,
            } => write!(
                f,
                "key_length {key_length} exceeds {available} available bytes after the header"
            ),
        }
    }
}

impl std::error::Error for CacheError {}
