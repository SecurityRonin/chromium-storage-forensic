//! Chromium IndexedDB forensic analyzer, and the reader it grades over.
//!
//! Emits [`forensicnomicon::report`] observations over the
//! [`IndexedDbRecord`]s produced by the [`chromium_storage_indexeddb_core`]
//! reader. Every finding is grounded in a state the reader itself surfaced — a
//! value it could not deserialize, a record whose database/object-store
//! metadata it could not resolve — so it needs no external oracle, and each is
//! an *observation* ("consistent with"), never a conclusion.
//!
//! The reader surface is re-exported, so `chromium_storage_indexeddb_forensic::`
//! resolves the reader types too.
#![forbid(unsafe_code)]

pub use chromium_storage_indexeddb_core::*;

use forensicnomicon::report::{Category, Observation, Severity};

/// A graded anomaly observed in an IndexedDB record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdbAnomalyKind {
    /// The record's value could not be deserialized from Blink's V8
    /// structured-clone wire format; the reader retained the raw bytes and the
    /// decode error. Consistent with a truncated/corrupted value, a
    /// wire-format the decoder does not yet cover, or deliberate tampering.
    ValueUndecodable {
        database_id: u64,
        object_store_id: u64,
        error: String,
    },
    /// The record carries a `database_id`/`object_store_id` that resolves to no
    /// name in the store's own metadata index. Consistent with an orphaned
    /// record whose metadata row was deleted, or with a partially recovered
    /// database.
    OrphanedRecord {
        database_id: u64,
        object_store_id: u64,
        database_resolved: bool,
        object_store_resolved: bool,
    },
}

impl IdbAnomalyKind {
    /// Severity — the single source of truth.
    #[must_use]
    pub fn severity(&self) -> Severity {
        match self {
            // A value the store wrote but the format cannot round-trip is a
            // direct data-integrity signal.
            IdbAnomalyKind::ValueUndecodable { .. } => Severity::Medium,
            // An unresolved name is often benign (recovery, tombstoned meta), so
            // it is surfaced low rather than asserted as tamper.
            IdbAnomalyKind::OrphanedRecord { .. } => Severity::Low,
        }
    }

    /// Analytical lens.
    #[must_use]
    pub fn category(&self) -> Category {
        Category::Integrity
    }

    /// Stable machine-readable code (published contract; never reused/renamed).
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            IdbAnomalyKind::ValueUndecodable { .. } => "IDB-VALUE-UNDECODABLE",
            IdbAnomalyKind::OrphanedRecord { .. } => "IDB-RECORD-ORPHANED",
        }
    }

    /// Human-readable note (observation, not a conclusion).
    #[must_use]
    pub fn note(&self) -> String {
        match self {
            IdbAnomalyKind::ValueUndecodable {
                database_id,
                object_store_id,
                error,
            } => format!(
                "IndexedDB value (database {database_id}, object store {object_store_id}) could \
                 not be decoded from the Blink V8 structured-clone format: {error} — consistent \
                 with a truncated, corrupted, or tampered value"
            ),
            IdbAnomalyKind::OrphanedRecord {
                database_id,
                object_store_id,
                database_resolved,
                object_store_resolved,
            } => {
                let missing = match (database_resolved, object_store_resolved) {
                    (false, false) => "neither the database nor the object-store name",
                    (false, true) => "the database name",
                    (true, false) => "the object-store name",
                    (true, true) => "no name (unexpected)",
                };
                format!(
                    "IndexedDB record (database {database_id}, object store {object_store_id}) \
                     resolves {missing} in the store's metadata index — consistent with an \
                     orphaned record whose metadata row is missing"
                )
            }
        }
    }
}

/// A graded finding: an [`IdbAnomalyKind`] plus its derived severity/code/note.
#[derive(Debug, Clone)]
pub struct IdbAnomaly {
    pub kind: IdbAnomalyKind,
    severity: Severity,
    code: &'static str,
    note: String,
}

impl IdbAnomaly {
    #[must_use]
    pub fn new(kind: IdbAnomalyKind) -> Self {
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

impl Observation for IdbAnomaly {
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

/// Grade a single parsed IndexedDB record, returning every anomaly it exhibits.
///
/// Clean records return an empty vector: a value the reader decoded and a record
/// whose metadata resolved trip nothing.
#[must_use]
pub fn analyze_record(record: &IndexedDbRecord) -> Vec<IdbAnomaly> {
    let mut out = Vec::new();

    if let RecordValue::Undecoded { error, .. } = &record.value {
        out.push(IdbAnomaly::new(IdbAnomalyKind::ValueUndecodable {
            database_id: record.database_id,
            object_store_id: record.object_store_id,
            error: error.clone(),
        }));
    }

    let db_ok = record.database.is_some();
    let os_ok = record.object_store.is_some();
    if !db_ok || !os_ok {
        out.push(IdbAnomaly::new(IdbAnomalyKind::OrphanedRecord {
            database_id: record.database_id,
            object_store_id: record.object_store_id,
            database_resolved: db_ok,
            object_store_resolved: os_ok,
        }));
    }

    out
}

/// Grade every record in a store, flattening per-record findings.
#[must_use]
pub fn analyze(records: &[IndexedDbRecord]) -> Vec<IdbAnomaly> {
    records.iter().flat_map(analyze_record).collect()
}
