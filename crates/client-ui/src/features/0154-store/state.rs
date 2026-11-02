use oteryn_client_runtime::cui3::store::{
    StoreCataloguePage, StoreClaimReceipt, StoreHistoryPage, StorePurchaseResult,
};
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StorePage {
    #[default]
    Catalogue,
    History,
    Inbox,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StorePanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub pending_action: u32,
    pub page: StorePage,
    pub category: String,
    pub selected_offer: usize,
    pub purchase_request_id: [u8; 16],
    pub purchase_confirmed: bool,
    pub fulfilment_unit_id: String,
    pub catalogue: Option<StoreCataloguePage>,
    pub history: Option<StoreHistoryPage>,
    pub purchase: Option<StorePurchaseResult>,
    pub claim: Option<StoreClaimReceipt>,
    pub feedback: Option<String>,
}
