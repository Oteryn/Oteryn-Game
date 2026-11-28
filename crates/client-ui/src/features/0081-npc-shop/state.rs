//! Live server quotes. Display positions are never trade identities.
mod options;
pub use options::ShopOptions;

use super::{EntityRef, NpcIntentDisposition, NpcTradeSide, npc_service};
use oteryn_client_runtime::npc_shop::{CharacterInventory, container_tree, resources};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quote {
    pub npc: EntityRef,
    pub revision: u64,
    pub offer_index: usize,
    pub item: u32,
    pub side: NpcTradeSide,
    pub count: u32,
    pub price: u32,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LiveShop {
    pub options: ShopOptions,
    pub options_selected: bool,
    /// Captures the packed option until the queued trade receives its result.
    pub pending_pack_in_backpacks: Option<bool>,
    pub weights: std::collections::BTreeMap<u32, Option<u32>>,
    pub weight_pending: Option<Quote>,
    pub inspection_selected: bool,
    pub free_capacity: Option<u32>,
    pub selected: bool,
    pub resources_selected: bool,
    pub balances: Option<resources::ResourceBalances>,
    pub containers: container_tree::ContainerViews,
    pub conversation: Option<EntityRef>,
    pub window: npc_service::NpcTradeWindow,
    pub inventory: Option<CharacterInventory>,
    pub pending: bool,
    pub feedback: String,
}
impl LiveShop {
    pub fn apply_window(&mut self, window: npc_service::NpcTradeWindow) {
        if self.window != window {
            self.weights.clear();
        }
        self.window = window;
        self.pending = false;
    }
    pub fn quotes(&self) -> Vec<Quote> {
        let Some((npc, revision)) = self.window.open else {
            return Vec::new();
        };
        self.window
            .offers
            .iter()
            .enumerate()
            .map(|(offer_index, offer)| Quote {
                npc,
                revision,
                offer_index,
                item: offer.item_definition_ref,
                side: offer.side,
                count: offer.count,
                price: offer.unit_price,
            })
            .collect()
    }
    /// Filter presentation only; command construction resolves the original quote.
    pub fn filtered_quotes(&self, side: Option<NpcTradeSide>, query: &str) -> Vec<Quote> {
        let query = query.trim().to_lowercase();
        self.quotes()
            .into_iter()
            .filter(|quote| side.is_none_or(|side| quote.side == side))
            .filter(|quote| self.quote_text(*quote).to_lowercase().contains(&query))
            .filter(|quote| {
                !self.options.only_owned
                    || quote.side == NpcTradeSide::Buy
                    || self.sale_owned(quote.item) > 0
            })
            .collect()
    }
    pub fn owned(&self, item: u32) -> u64 {
        self.inventory.as_ref().map_or(0, |inventory| {
            inventory
                .entries
                .iter()
                .filter(|entry| entry.item_definition_ref.get() == item)
                .map(|entry| u64::from(entry.count.get()))
                .sum()
        })
    }
    fn quote_index(&self, quote: Quote) -> Option<usize> {
        if self.window.open != Some((quote.npc, quote.revision)) {
            return None;
        }
        let offer = self.window.offers.get(quote.offer_index)?;
        (offer.item_definition_ref == quote.item
            && offer.side == quote.side
            && offer.count == quote.count
            && offer.unit_price == quote.price)
            .then_some(quote.offer_index)
    }
    pub fn maximum_amount(&self, quote: Quote) -> u32 {
        let Some(index) = self.quote_index(quote) else {
            return 0;
        };
        self.window
            .details
            .get(index)
            .map_or(100, |detail| detail.maximum_quantity)
    }
    pub fn quote_name(&self, quote: Quote) -> Option<&str> {
        Some(&self.window.details.get(self.quote_index(quote)?)?.name)
    }
    pub fn quote_text(&self, quote: Quote) -> String {
        let name = self
            .quote_name(quote)
            .map_or_else(|| format!("item #{}", quote.item), str::to_owned);
        lfmt!(
            "{:?} {} × {} — {} gold / offer — carried {} — limit {}",
            quote.side,
            name,
            quote.count,
            quote.price,
            self.inventory.as_ref().map_or_else(
                || "unknown".into(),
                |_| {
                    if quote.side == NpcTradeSide::Sell {
                        self.sale_owned(quote.item)
                    } else {
                        self.owned(quote.item)
                    }
                    .to_string()
                }
            ),
            self.maximum_amount(quote)
        )
    }
    pub fn total(&self, quote: Quote, amount: u32) -> String {
        // Crystal charges 20 gold for the shopping bag on a packed purchase.
        let bag_price = if self.packs_purchase(quote.side) {
            20
        } else {
            0
        };
        lfmt!(
            "{} items; {} gold",
            u64::from(quote.count) * u64::from(amount),
            u64::from(quote.price) * u64::from(amount) + bag_price
        )
    }
    pub fn look(&self, quote: Quote) -> Option<super::npc_shop::NpcShopIntent> {
        let index = self.quote_index(quote)?;
        Some(super::npc_shop::NpcShopIntent {
            npc_actor: quote.npc,
            catalogue_revision: quote.revision,
            offer_index: u32::try_from(index).ok()?,
            side: quote.side,
            action: super::npc_shop::NpcShopAction::Look,
            expected_unit_price: quote.price,
        })
    }
    pub fn free_slots(&self) -> Option<usize> {
        let inventory = self.inventory.as_ref()?;
        let backpack = inventory.main_backpack?;
        let capacity = self
            .containers
            .views
            .iter()
            .find(|view| view.container_handle == backpack.handle && view.browse.is_none())?
            .capacity;
        Some(usize::from(capacity).saturating_sub(inventory.entries.len()))
    }
    /// Mirrors the server economy's quantity and side-specific total bounds.
    pub fn valid_amount(&self, quote: Quote, amount: u32) -> bool {
        if !self.window.details.is_empty() {
            return (1..=self.maximum_amount(quote)).contains(&amount);
        }
        let maximum = match quote.side {
            NpcTradeSide::Buy => 20_000_000 + 999_999_999_999u64,
            NpcTradeSide::Sell => 1_009_999,
        };
        (1..=100).contains(&amount) && u64::from(quote.price) * u64::from(amount) <= maximum
    }
    pub fn trade(&mut self, quote: Quote, amount: u32) -> Option<npc_service::NpcTradeIntent> {
        if !self.selected || self.pending || self.pending_pack_in_backpacks.is_some() {
            return None;
        }
        if !self.valid_amount(quote, amount) {
            self.feedback = tr!("Amount exceeds this offer's server limit").into();
            return None;
        }
        if !self.can_trade(quote, amount) {
            self.feedback = "No free backpack slot in the current view; enable Ignore capacity to ask the server".into();
            return None;
        }
        let index = self.quote_index(quote)?;
        let intent = npc_service::NpcTradeIntent {
            npc_actor: quote.npc,
            catalogue_revision: quote.revision,
            offer_index: u32::try_from(index).ok()?,
            side: quote.side,
            quantity: amount,
            expected_unit_price: quote.price,
        };
        let packed = self.packs_purchase(quote.side);
        npc_service::encode_npc_trade_intent_for(&intent, packed).ok()?;
        self.pending_pack_in_backpacks = self.options_selected.then_some(packed);
        self.pending = true;
        self.feedback = tr!("Trade pending").into();
        Some(intent)
    }
    pub fn result(&mut self, result: NpcIntentDisposition) {
        self.pending = false;
        self.pending_pack_in_backpacks = None;
        self.feedback = if result == NpcIntentDisposition::Ok {
            tr!("Trade completed; inventory updated by server").into()
        } else {
            lfmt!("Trade refused: {result:?}", result = result)
        };
    }
    pub fn text(&self) -> String {
        let state = if !self.selected {
            tr!("NPC trade unavailable in this session")
        } else if self.window.open.is_none() {
            tr!("No open shop. Talk to an NPC and ask for trade.")
        } else {
            tr!("Live NPC buy/sell quotes")
        };
        let gold = if !self.resources_selected {
            tr!("Gold balance: unavailable (capability not selected)").into()
        } else if let Some(balances) = self.balances {
            lfmt!(
                "Carried gold: {}; Bank gold: {}; Total: {}",
                balances.carried_gold,
                balances.bank_gold,
                balances.carried_gold.saturating_add(balances.bank_gold)
            )
        } else {
            tr!("Gold balance: query pending or refused").into()
        };
        let slots = self.free_slots().map_or_else(
            || tr!("Free slots: unknown").into(),
            |free| lfmt!("Free main-backpack slots: {free}", free = free),
        );
        format!(
            "{state}\n{}\n{gold}\n{slots}\n{}",
            self.feedback,
            if self.options_selected {
                "Pack purchases for 20 gold. Sell All can include equipped items."
            } else {
                "Purchases go into the main backpack. Sales use its direct entries; move equipped items there first."
            }
        )
    }
}
