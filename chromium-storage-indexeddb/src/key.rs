//! Decoded IndexedDB primary key (`IDBKey`).

/// A decoded IndexedDB key. The encoding is a one-byte type tag followed by a
/// type-specific body (Chromium `indexed_db_leveldb_coding.cc`
/// `EncodeIDBKey`/`DecodeIDBKey`).
#[derive(Clone, Debug, PartialEq)]
pub enum IdbKey {
    /// `kIndexedDBKeyNumberTypeByte` — an IEEE-754 double.
    Number(f64),
    /// `kIndexedDBKeyDateTypeByte` — a date as milliseconds since the Unix epoch.
    Date(f64),
    /// `kIndexedDBKeyStringTypeByte` — a UTF-16BE string.
    String(String),
    /// `kIndexedDBKeyBinaryTypeByte` — raw bytes.
    Binary(Vec<u8>),
    /// `kIndexedDBKeyArrayTypeByte` — an array of keys.
    Array(Vec<IdbKey>),
    /// `kIndexedDBKeyNullTypeByte` — the null/none key.
    Null,
    /// `kIndexedDBKeyMinKeyTypeByte` — the synthetic minimum key.
    Min,
    /// An unrecognised type tag or truncated body — raw bytes surfaced verbatim.
    Invalid(Vec<u8>),
}
