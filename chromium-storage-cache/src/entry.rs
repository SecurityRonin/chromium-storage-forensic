//! Simple Cache `_0` entry-file parser. (RED stub — see the GREEN commit.)

use crate::{CacheEntry, CacheError};

pub(crate) fn parse(bytes: &[u8]) -> Result<CacheEntry, CacheError> {
    Err(CacheError::TooShort {
        available: bytes.len(),
    })
}
