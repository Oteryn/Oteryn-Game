use oteryn_client_runtime::raw_feature::house_wrap::Catalogue;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Refresh,
    Previous,
    Next,
    Apply,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Panel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub pending_payload: Vec<u8>,
    pub reviewed_payload: Option<Vec<u8>>,
    pub catalogue: Option<Catalogue>,
    pub choice: String,
    pub feedback: String,
}
