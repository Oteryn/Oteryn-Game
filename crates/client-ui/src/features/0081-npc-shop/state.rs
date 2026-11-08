//! Live server quotes. Display positions are never trade identities.
mod options;
pub use options::ShopOptions;

use super::{EntityRef, NpcIntentDisposition, NpcTradeSide, npc_service};
use oteryn_client_runtime::npc_shop::{CharacterInventory, container_tree, resources};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quote {
    pub npc: EntityRef,
    pub revision: u64,
    pub item: u32,
    pub side: NpcTradeSide,
    pub count: u32,
    pub price: u32,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LiveShop {
    pub options: ShopOptions,
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
            .map(|offer| Quote {
                npc,
                revision,
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
                    || self.owned(quote.item) > 0
            })
            .collect()
    }
    pub fn owned(&self, item: u32) -> u64 {
        self.inventory.as_ref().map_or(0, |inventory| {
            inventory
                .entries
                .iter()
                .chain(
                    inventory
                        .main_backpack
                        .iter()
                        .filter(|_| !self.options.ignore_equipped),
                )
                .chain(
                    inventory
                        .equipment
                        .iter()
                        .filter(|_| !self.options.ignore_equipped)
                        .map(|entry| &entry.item),
                )
                .filter(|entry| entry.item_definition_ref.get() == item)
                .map(|entry| u64::from(entry.count.get()))
                .sum()
        })
    }
    pub fn quote_text(&self, quote: Quote) -> String {
        format!(
            "{:?} item #{} × {} — {} gold / offer — carried {}",
            quote.side,
            quote.item,
            quote.count,
            quote.price,
            self.inventory
                .as_ref()
                .map_or_else(|| "unknown".into(), |_| self.owned(quote.item).to_string())
        )
    }
    pub fn total(&self, quote: Quote, amount: u32) -> String {
        format!(
            "{} items; {} gold",
            u64::from(quote.count) * u64::from(amount),
            u64::from(quote.price) * u64::from(amount)
        )
    }
    pub fn look(&self, quote: Quote) -> Option<super::npc_shop::NpcShopIntent> {
        let index = self.quotes().iter().position(|live| *live == quote)?;
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
        let maximum = match quote.side {
            NpcTradeSide::Buy => 20_000_000 + 999_999_999_999u64,
            NpcTradeSide::Sell => 1_009_999,
        };
        (1..=100).contains(&amount) && u64::from(quote.price) * u64::from(amount) <= maximum
    }
    pub fn trade(&mut self, quote: Quote, amount: u32) -> Option<npc_service::NpcTradeIntent> {
        if !self.selected || self.pending {
            return None;
        }
        if !self.valid_amount(quote, amount) {
            self.feedback = "Amount must be 1–100 and within the server's gold limit".into();
            return None;
        }
        if !self.can_trade(quote, amount) {
            self.feedback = "No free backpack slot in the current view; enable Ignore capacity to ask the server".into();
            return None;
        }
        let index = self.quotes().iter().position(|live| *live == quote)?;
        let intent = npc_service::NpcTradeIntent {
            npc_actor: quote.npc,
            catalogue_revision: quote.revision,
            offer_index: u32::try_from(index).ok()?,
            side: quote.side,
            quantity: amount,
            expected_unit_price: quote.price,
        };
        npc_service::encode_npc_trade_intent(&intent).ok()?;
        self.pending = true;
        self.feedback = "Trade pending".into();
        Some(intent)
    }
    pub fn result(&mut self, result: NpcIntentDisposition) {
        self.pending = false;
        self.feedback = if result == NpcIntentDisposition::Ok {
            "Trade completed; inventory updated by server".into()
        } else {
            format!("Trade refused: {result:?}")
        };
    }
    pub fn text(&self) -> String {
        let state = if !self.selected {
            "NPC trade unavailable in this session"
        } else if self.window.open.is_none() {
            "No open shop. Talk to an NPC and ask for trade."
        } else {
            "Live NPC buy/sell quotes"
        };
        let gold = if !self.resources_selected {
            "Gold balance: unavailable (capability not selected)".into()
        } else if let Some(balances) = self.balances {
            format!(
                "Carried gold: {}; Bank gold: {}; Total: {}",
                balances.carried_gold,
                balances.bank_gold,
                balances.carried_gold.saturating_add(balances.bank_gold)
            )
        } else {
            "Gold balance: query pending or refused".into()
        };
        let slots = self.free_slots().map_or_else(
            || "Free slots: unknown".into(),
            |free| format!("Free main-backpack slots: {free}"),
        );
        format!(
            "{state}\n{}\n{gold}\n{slots}\n{}",
            self.feedback,
            "Purchases go into the main backpack. Sales use its direct entries; move equipped items there first."
        )
    }
}
