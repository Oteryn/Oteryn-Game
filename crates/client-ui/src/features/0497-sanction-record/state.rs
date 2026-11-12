use oteryn_client_runtime::sanction_record::SanctionRecord;

/// The window of the own sanction record; `record` is the latest server reply.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SanctionsPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub record: Option<SanctionRecord>,
    pub feedback: Option<&'static str>,
}
