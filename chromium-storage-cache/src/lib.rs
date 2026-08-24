//! Backward-compatible facade for the Chromium Simple Cache reader.
//!
//! The reader was split into [`chromium_storage_cache_core`] (the reader) and
//! `chromium-storage-cache-forensic` (the analyzer), so each Chromium storage
//! format follows the fleet's reader/analyzer (core/forensic) pattern. This
//! crate keeps the original `chromium-storage-cache` name resolving — it
//! re-exports the entire reader surface, so existing consumers
//! (`discord-desktop-forensic`, `whatsapp-desktop-forensic`, …) need no change.
//!
//! New consumers should depend on `chromium-storage-cache-core` directly.
#![forbid(unsafe_code)]

pub use chromium_storage_cache_core::*;
