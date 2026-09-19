use indexmap::IndexMap;

/// In-memory i-logs
pub(crate) struct ILog {
    records: IndexMap<String, String>
}