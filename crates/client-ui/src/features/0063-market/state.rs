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
    pub selected: bool,
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
            selected: false,
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
