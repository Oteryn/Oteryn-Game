//! Memorial preview state. Offers and balances must come from a confirmed server catalogue.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MemorialTab {
    #[default]
    GoldenOutfits,
    TokenExchange,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorialOffer {
    pub outfit_key: String,
    pub look_type: u32,
    pub token_key: String,
    pub cost: u64,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemorialGallery {
    pub tab: MemorialTab,
    pub offers: Vec<MemorialOffer>,
    pub balances: std::collections::BTreeMap<String, u64>,
    pub selected: usize,
    pub pending: bool,
}
impl MemorialGallery {
    pub fn preview(&self) -> Option<&MemorialOffer> {
        self.offers.get(self.selected)
    }
    /// Enables a request only; the server must atomically spend tokens and grant ownership.
    pub fn can_exchange(&self) -> bool {
        !self.pending
            && self.preview().is_some_and(|offer| {
                offer.cost > 0
                    && !offer.token_key.is_empty()
                    && self
                        .balances
                        .get(&offer.token_key)
                        .is_some_and(|balance| *balance >= offer.cost)
            })
    }
}
