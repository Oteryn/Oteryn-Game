use oteryn_client_runtime::cui3::store::{
    StoreCataloguePage, StoreClaimReceipt, StoreHistoryPage, StorePurchaseResult,
};
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StorePage {
    #[default]
    Catalogue,
    History,
    CoinHistory,
    Inbox,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StorePanel {
    pub cosmetic_tokens: Option<u64>,
    pub token_supported: bool,
    pub token_balance_refresh: bool,
    pub gift: super::GiftPanel,
    pub browser: super::Browser,
    pub coin_history: super::CoinHistory,
    pub gift_open: bool,
    pub gift_fields: [String; 2],
    pub gift_reviewed: Option<GiftDraft>,
    pub search: String,
    pub sort: StoreSort,
    pub selected: bool,
    pub open: bool,
    pub classic_open: bool,
    pub pending: bool,
    pub pending_action: u32,
    pub page: StorePage,
    pub category: String,
    pub selected_offer: usize,
    pub catalogue_category: String,
    pub hireling_name: String,
    pub hireling_sex: u8,
    pub hireling_id: Option<[u8; 16]>,
    /// Frozen confirmed wire request; uncertain outcomes retry these exact bytes.
    pub confirmed_purchase: Option<Vec<u8>>,
    pub purchase_request_id: [u8; 16],
    pub purchase_confirmed: bool,
    pub fulfilment_unit_id: String,
    pub catalogue: Option<StoreCataloguePage>,
    pub history: Option<StoreHistoryPage>,
    pub purchase: Option<StorePurchaseResult>,
    pub claim: Option<StoreClaimReceipt>,
    pub feedback: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StoreSort {
    #[default]
    Catalogue,
    Title,
    Price,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GiftDraft {
    pub recipient: String,
    pub amount: u64,
}
