//! The IndexedDB `KeyPrefix` — the `(database id, object store id, index id)`
//! that opens every record key.

use forensicnomicon_core::chromium_indexeddb::decode_key_prefix_lengths;

/// A decoded `KeyPrefix`: the three ids and the total prefix length in bytes so
/// the caller can locate the type-specific key tail that follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeyPrefix {
    pub database_id: u64,
    pub object_store_id: u64,
    pub index_id: u64,
    /// Byte length of the whole prefix (the descriptor byte + the three ids).
    pub len: usize,
}

/// Read a big-endian unsigned integer of `n` (1..=8) bytes at `off`; `None` if
/// out of range.
fn be_uint(data: &[u8], off: usize, n: usize) -> Option<u64> {
    let slice = data.get(off..off.checked_add(n)?)?;
    let mut v = 0u64;
    for &b in slice {
        v = (v << 8) | u64::from(b);
    }
    Some(v)
}

impl KeyPrefix {
    /// Parse the `KeyPrefix` at the start of a record key. Returns `None` if the
    /// key is shorter than the descriptor demands (panic-free).
    pub(crate) fn parse(key: &[u8]) -> Option<KeyPrefix> {
        let descriptor = *key.first()?;
        let lengths = decode_key_prefix_lengths(descriptor);
        let db = lengths.database_id_len as usize;
        let os = lengths.object_store_id_len as usize;
        let ix = lengths.index_id_len as usize;
        let database_id = be_uint(key, 1, db)?;
        let object_store_id = be_uint(key, 1 + db, os)?;
        let index_id = be_uint(key, 1 + db + os, ix)?;
        Some(KeyPrefix {
            database_id,
            object_store_id,
            index_id,
            len: 1 + db + os + ix,
        })
    }
}
