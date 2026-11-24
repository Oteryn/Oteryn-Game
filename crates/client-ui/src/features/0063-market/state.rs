use super::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MarketView {
    #[default]
    Browse,
    Own,
    History,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketPanel {
    pub hand_filter: Option<u8>,
    pub hand_positions: std::collections::BTreeMap<u32, Option<u8>>,
    pub hand_pending: Option<u32>,
    pub funds: Option<(u64, u64)>,
    pub funds_pending: bool,
    pub coins: CoinPanel,
    pub details: super::details::Details,
    pub selected: bool,
    /// Zero shows both sides; one buy, two sell. Applies to the loaded page.
    pub side_filter: u8,
    pub open: bool,
    pub view: MarketView,
    pub definition: u32,
    pub amount: u32,
    pub price: u64,
    pub sell: bool,
    pub anonymous: bool,
    pub page: u32,
    pub selected_offer: Option<[u8; 16]>,
    pub rows: Vec<market::MarketRow>,
    pub pending: Option<MarketCommand>,
    pub feedback: Option<&'static str>,
}
impl Default for MarketPanel {
    fn default() -> Self {
        Self {
            hand_filter: None,
            hand_positions: Default::default(),
            hand_pending: None,
            funds: None,
            funds_pending: false,
            coins: CoinPanel::default(),
            details: Default::default(),
            selected: false,
            side_filter: 0,
            open: false,
            view: MarketView::Browse,
            definition: 0,
            amount: 1,
            price: 1,
            sell: true,
            anonymous: false,
            page: 0,
            selected_offer: None,
            rows: Vec::new(),
            pending: None,
            feedback: None,
        }
    }
}

impl MarketPanel {
    pub fn matches_hand_filter(&self, definition: u32) -> bool {
        self.hand_filter
            .is_none_or(|hand| self.hand_positions.get(&definition) == Some(&Some(hand)))
    }
}
