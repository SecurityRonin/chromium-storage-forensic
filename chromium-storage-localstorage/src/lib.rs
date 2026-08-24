//! Backward-compatible facade for the Chromium localstorage reader.
//!
//! The reader was split into [`chromium_storage_localstorage_core`] (reader) and `chromium-storage-localstorage-forensic`
//! (analyzer). This crate keeps the original `chromium-storage-localstorage` name resolving by
//! re-exporting the whole reader surface, so existing consumers need no change.
//! New consumers should depend on `chromium-storage-localstorage-core` directly.
#![forbid(unsafe_code)]

pub use chromium_storage_localstorage_core::*;
