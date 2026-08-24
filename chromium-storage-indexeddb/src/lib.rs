//! Backward-compatible facade for the Chromium indexeddb reader.
//!
//! The reader was split into [`chromium_storage_indexeddb_core`] (reader) and `chromium-storage-indexeddb-forensic`
//! (analyzer). This crate keeps the original `chromium-storage-indexeddb` name resolving by
//! re-exporting the whole reader surface, so existing consumers need no change.
//! New consumers should depend on `chromium-storage-indexeddb-core` directly.
#![forbid(unsafe_code)]

pub use chromium_storage_indexeddb_core::*;
