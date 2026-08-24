//! Chromium Local Storage forensic analyzer, and the reader it grades over.
//!
//! Emits [`forensicnomicon::report`] observations over the
//! [`LocalStorageRecord`]s produced by the
//! [`chromium_storage_localstorage_core`] reader. Every finding is grounded in
//! the store's own state — a `META` timestamp that predates the format, a `DATA`
//! record with no `META` for its origin, a key matching neither shape — so it
//! needs no external oracle, and each is an *observation* ("consistent with"),
//! never a conclusion.
//!
//! The reader surface is re-exported, so
//! `chromium_storage_localstorage_forensic::` resolves the reader types too.
#![forbid(unsafe_code)]

pub use chromium_storage_localstorage_core::*;

use forensicnomicon::report::{Category, Observation, Severity};
use std::collections::HashSet;

/// WebKit-microseconds (µs since 1601-01-01 UTC) at 2010-01-01 00:00:00 UTC.
///
/// Chrome's LevelDB-backed Local Storage shipped in 2017; a `META` timestamp
/// earlier than 2010 cannot be a genuine last-modified time for this format.
/// The floor is deliberately conservative — years before the true earliest
/// value — so a real record never trips it.
const WEBKIT_MICROS_2010: u64 = 12_906_777_600_000_000;

/// A graded anomaly observed in a Local Storage store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LsAnomalyKind {
    /// A live `META` record carries a last-modified timestamp earlier than the
    /// format could possibly have produced. Consistent with a zeroed or edited
    /// timestamp.
    ImplausibleMetaTimestamp {
        origin: String,
        timestamp_webkit_micros: u64,
    },
    /// A live `DATA` record exists for an origin that has no `META` record.
    /// Chrome writes a `META` row for every origin it stores data for, so a
    /// data row without one is consistent with a deleted metadata row or a
    /// partially recovered store.
    OrphanedData { origin: String },
}

impl LsAnomalyKind {
    /// Severity — the single source of truth.
    #[must_use]
    pub fn severity(&self) -> Severity {
        match self {
            // A timestamp the format could not have written is a strong tamper
            // signal.
            LsAnomalyKind::ImplausibleMetaTimestamp { .. } => Severity::Medium,
            // An orphan is often benign (recovery, deleted metadata), so it is
            // surfaced low rather than asserted as tamper.
            LsAnomalyKind::OrphanedData { .. } => Severity::Low,
        }
    }

    /// Analytical lens.
    #[must_use]
    pub fn category(&self) -> Category {
        match self {
            LsAnomalyKind::ImplausibleMetaTimestamp { .. } => Category::History,
            LsAnomalyKind::OrphanedData { .. } => Category::Integrity,
        }
    }

    /// Stable machine-readable code (published contract; never reused/renamed).
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            LsAnomalyKind::ImplausibleMetaTimestamp { .. } => "LS-META-TIME-IMPLAUSIBLE",
            LsAnomalyKind::OrphanedData { .. } => "LS-DATA-ORPHANED",
        }
    }

    /// Human-readable note (observation, not a conclusion).
    #[must_use]
    pub fn note(&self) -> String {
        match self {
            LsAnomalyKind::ImplausibleMetaTimestamp {
                origin,
                timestamp_webkit_micros,
            } => format!(
                "Local Storage META for origin {origin} records a last-modified time of \
                 {timestamp_webkit_micros} WebKit µs, earlier than the format could have written \
                 (before 2010) — consistent with a zeroed or edited timestamp"
            ),
            LsAnomalyKind::OrphanedData { origin } => format!(
                "Local Storage DATA exists for origin {origin} with no META record — consistent \
                 with a deleted metadata row or a partially recovered store"
            ),
        }
    }
}

/// A graded finding: an [`LsAnomalyKind`] plus its derived severity/code/note.
#[derive(Debug, Clone)]
pub struct LsAnomaly {
    pub kind: LsAnomalyKind,
    severity: Severity,
    code: &'static str,
    note: String,
}

impl LsAnomaly {
    #[must_use]
    pub fn new(kind: LsAnomalyKind) -> Self {
        let severity = kind.severity();
        let code = kind.code();
        let note = kind.note();
        Self {
            kind,
            severity,
            code,
            note,
        }
    }
}

impl Observation for LsAnomaly {
    fn severity(&self) -> Option<Severity> {
        Some(self.severity)
    }
    fn code(&self) -> &'static str {
        self.code
    }
    fn note(&self) -> String {
        self.note.clone()
    }
    fn category(&self) -> Category {
        self.kind.category()
    }
}

/// Grade a whole Local Storage store, returning every anomaly it exhibits.
///
/// Cross-record checks (orphaned data) mean this operates over the full record
/// set, not one record at a time. Tombstones (`deleted`) are skipped — a
/// deletion legitimately carries no metadata and no meaningful timestamp.
#[must_use]
pub fn analyze(records: &[LocalStorageRecord]) -> Vec<LsAnomaly> {
    let mut out = Vec::new();

    // Origins that have a live META record — the set an origin's DATA needs.
    let meta_origins: HashSet<&str> = records
        .iter()
        .filter_map(|r| match r {
            LocalStorageRecord::Meta {
                origin, deleted, ..
            } if !deleted => Some(origin.as_str()),
            _ => None,
        })
        .collect();

    for record in records {
        match record {
            LocalStorageRecord::Meta {
                origin,
                timestamp_webkit_micros,
                deleted,
                ..
            } => {
                if !deleted && *timestamp_webkit_micros < WEBKIT_MICROS_2010 {
                    out.push(LsAnomaly::new(LsAnomalyKind::ImplausibleMetaTimestamp {
                        origin: origin.clone(),
                        timestamp_webkit_micros: *timestamp_webkit_micros,
                    }));
                }
            }
            LocalStorageRecord::Data {
                origin, deleted, ..
            } => {
                if !deleted && !meta_origins.contains(origin.as_str()) {
                    out.push(LsAnomaly::new(LsAnomalyKind::OrphanedData {
                        origin: origin.clone(),
                    }));
                }
            }
            // `Other` keys are the store's own housekeeping rows (`VERSION`, the
            // `META`-access family, …) — legitimately present in every real
            // store, so they are not anomalies. The reader surfaces them; the
            // analyzer does not grade them.
            LocalStorageRecord::Other { .. } => {}
        }
    }

    out
}
