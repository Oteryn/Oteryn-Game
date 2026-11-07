#[derive(Debug, Clone, Default)]
pub struct Panel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub lines: std::collections::VecDeque<oteryn_client_runtime::CastCasterLine>,
    pub feedback: String,
}
