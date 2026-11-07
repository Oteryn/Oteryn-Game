//! Add the existing live Premium view to item capacity decisions.
use super::NativeItemWeights;
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{ItemDefinitionFacts, ItemWeightLookup, ItemWeightRefused};
use crate::durability::player_trade::PlayerTradeContent;
use crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator;

pub(in crate::gameplay_transport) struct WheelItemWeights<'a> {
    content: NativeItemWeights<'a>,
    premium: Option<&'a SpellPremiumCoordinator>,
}

impl<'a> NativeItemWeights<'a> {
    pub(in crate::gameplay_transport) fn with_wheel(
        self,
        premium: Option<&'a SpellPremiumCoordinator>,
    ) -> WheelItemWeights<'a> {
        WheelItemWeights {
            content: self,
            premium,
        }
    }
}

impl ItemWeightLookup for WheelItemWeights<'_> {
    fn generation_digest(&self) -> [u8; 32] {
        self.content.generation_digest()
    }
    fn weight_of(&self, definition: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        self.content.weight_of(definition)
    }
    fn wheel_premium_current(&self, account: [u8; 16]) -> bool {
        // Delivery is settled by session admission/refresh. Missing delivery, stale authority
        // and clock loss remain closed; durable Premium evidence cannot reopen this view.
        self.premium
            .is_some_and(|owner| owner.view(account).premium_gate().premium_current())
    }
}

impl ItemLoopContent for WheelItemWeights<'_> {
    fn loop_content_digest(&self) -> [u8; 32] {
        self.content.loop_content_digest()
    }
    fn loop_facts(&self, definition: &TypedDefinitionRef) -> Option<ItemDefinitionFacts> {
        self.content.loop_facts(definition)
    }
    fn loop_definition(&self, reference: u32) -> Option<TypedDefinitionRef> {
        self.content.loop_definition(reference)
    }
    fn is_quiver(&self, definition: &TypedDefinitionRef) -> bool {
        self.content.is_quiver(definition)
    }
    fn is_ammunition(&self, definition: &TypedDefinitionRef) -> bool {
        self.content.is_ammunition(definition)
    }
    fn loot_category(
        &self,
        definition: &TypedDefinitionRef,
    ) -> oteryn_gameplay_items::quick_loot::LootCategory {
        self.content.loot_category(definition)
    }
}

impl PlayerTradeContent for WheelItemWeights<'_> {
    fn player_trade_flags(
        &self,
        definition: &TypedDefinitionRef,
    ) -> Option<(oteryn_gameplay_economy::player_trade::TradeFlag, bool)> {
        self.content.player_trade_flags(definition)
    }
}
