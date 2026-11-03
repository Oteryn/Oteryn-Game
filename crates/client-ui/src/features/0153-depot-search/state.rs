use oteryn_client_runtime::cui3::depot_search::Reply;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DepotSearchPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub item_id: u32,
    pub tier: u8,
    pub reply: Option<Reply>,
    pub feedback: Option<String>,
}
