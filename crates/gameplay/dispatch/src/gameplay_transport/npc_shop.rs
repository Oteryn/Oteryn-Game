//! NPC-TRADE-SELLALL-1 `NPC_SHOP` (command 97) over the open NPC trade window of capability 3.
//!
//! The intent names one projected offer exactly as command 8 does, and
//! [`NpcConversations::trade_in`](super::npc_command::NpcConversations) resolves it under the same
//! conversation, range, window and stale checks. LOOK answers the offer item's description from the
//! active generation's Item policy and writes nothing (Canary 47dfd51 game.cpp:6328-6354
//! `playerLookInShop`: "You see " plus the near description of one item of the offer's count).
//! SELL_ALL commits one durable NPC trade through the command 8 owner that sells every carried
//! stack of a count-1 SELL offer, replay first, and answers the exact units and gold (Canary
//! npc.cpp:910-1017 `onPlayerSellItem`: every carried unit of the type up to the amount, paid
//! `sellPrice * soldAmount`; the loot-pouch sweep of npc.cpp:798-908 is not this command).

use super::connection::UseCommand;
use super::npc_command::NpcTradeRefusal;
use super::npc_trade_command::{self, NpcTradeStep};
use super::{ComposedFreshAdmission, item_ref_admission::pinned_index};
use crate::foundation::ExactActorRef;
use crate::content::ActiveGeneration;
use oteryn_protocol_oteryn::item_look::MAX_ITEM_LOOK_TEXT_BYTES;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTradeIntent, NpcTradeSide};
use oteryn_protocol_oteryn::npc_shop::{NpcShopAction, NpcShopIntent, NpcShopResult};

/// Canary game.cpp:6352 `Item::getDescription(it, 1, ...)`: the shop look is a near look.
const SHOP_LOOK_DISTANCE: i32 = 1;

/// What the connection does with one command 97.
pub enum NpcShopStep {
    /// LOOK answered; nothing changed.
    Looked(String),
    /// SELL_ALL committed now or by replay: `OK`, then the inventory delta.
    Sold { units: u32, gold: u64 },
    /// Nothing committed: the disposition and the domain 7 and 8 payloads.
    Refused(NpcTradeRefusal),
    /// The outcome is unknown: end the connection without sequencing the CommandId.
    Unknown,
}

impl NpcShopStep {
    pub fn rejected() -> Self {
        Self::Refused(NpcTradeRefusal::only(NpcIntentDisposition::Rejected))
    }

    /// The command result: the encoded `NpcShopResultV1`.
    pub fn result(&self) -> NpcShopResult {
        match self {
            Self::Looked(text) => NpcShopResult {
                disposition: NpcIntentDisposition::Ok,
                look_text: text.clone(),
                sold_units: 0,
                total_gold_units: 0,
            },
            Self::Sold { units, gold } => NpcShopResult {
                disposition: NpcIntentDisposition::Ok,
                look_text: String::new(),
                sold_units: *units,
                total_gold_units: *gold,
            },
            Self::Refused(refusal) => NpcShopResult::refused(refusal.disposition),
            Self::Unknown => NpcShopResult::refused(NpcIntentDisposition::Rejected),
        }
    }
}

/// The command 8 intent the shop intent names: one unit of the same offer.
const fn trade_intent(intent: &NpcShopIntent) -> NpcTradeIntent {
    NpcTradeIntent {
        npc_actor: intent.npc_actor,
        catalogue_revision: intent.catalogue_revision,
        offer_index: intent.offer_index,
        side: intent.side,
        quantity: 1,
        expected_unit_price: intent.expected_unit_price,
    }
}

/// `text` cut at a character boundary to the look text bound.
fn bounded(mut text: String) -> String {
    let mut end = text.len().min(MAX_ITEM_LOOK_TEXT_BYTES);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    text
}

/// Command 97 of the admitted `actor`.
pub(super) async fn npc_shop(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcShopIntent,
) -> NpcShopStep {
    let trade = trade_intent(&intent);
    match intent.action {
        NpcShopAction::Look => look(admission, actor, command, &trade).await,
        NpcShopAction::SellAll => {
            if intent.side != NpcTradeSide::Sell {
                return NpcShopStep::rejected();
            }
            match npc_trade_command::trade(admission, actor, command, trade, true)
                .await
                .map_err(|step| *step)
            {
                Ok(sold) => NpcShopStep::Sold {
                    units: sold.units,
                    gold: sold.gold,
                },
                Err(NpcTradeStep::Refused(refusal)) => NpcShopStep::Refused(refusal),
                Err(NpcTradeStep::Unknown) => NpcShopStep::Unknown,
                Err(NpcTradeStep::Traded) => NpcShopStep::rejected(),
            }
        }
    }
}

async fn look(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    trade: &NpcTradeIntent,
) -> NpcShopStep {
    let offer = {
        let runtime = admission.runtime.lock().await;
        let index = pinned_index(&runtime, admission.active_generation);
        match admission.states.npc_conversations.trade_in(
            &admission.states.chat,
            &runtime,
            actor,
            command.game_session_id,
            trade,
            index,
        ) {
            Ok(offer) => offer,
            Err(refusal) => return NpcShopStep::Refused(*refusal),
        }
    };
    let Some(native) = admission
        .active_generation
        .and_then(ActiveGeneration::native_gameplay)
    else {
        return NpcShopStep::rejected();
    };
    let Some(policy) = native.item_policy(&offer.item_key, &offer.item_revision) else {
        return NpcShopStep::rejected();
    };
    match std::num::NonZeroU32::new(offer.count).and_then(|count| {
        oteryn_gameplay_items::look::item_text(
            &policy.record().semantics,
            count,
            SHOP_LOOK_DISTANCE,
        )
    }) {
        Some(text) => NpcShopStep::Looked(bounded(text)),
        None => NpcShopStep::rejected(),
    }
}

macro_rules! authority_methods {
    () => {
        /// NPC-TRADE-SELLALL-1: one `NPC_SHOP` (command 97) of the admitted actor.
        fn npc_shop(
            &self,
            _actor: $crate::foundation::ExactActorRef,
            _command: $crate::gameplay_transport::connection::UseCommand,
            _intent: oteryn_protocol_oteryn::npc_shop::NpcShopIntent,
        ) -> impl std::future::Future<Output = $crate::gameplay_transport::npc_shop::NpcShopStep>
        {
            async { $crate::gameplay_transport::npc_shop::NpcShopStep::rejected() }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn npc_shop(
            &self,
            actor: crate::foundation::ExactActorRef,
            command: connection::UseCommand,
            intent: oteryn_protocol_oteryn::npc_shop::NpcShopIntent,
        ) -> npc_shop::NpcShopStep {
            npc_shop::npc_shop(self, actor, command, intent).await
        }
    };
}
pub(super) use composed_methods;

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test fixtures")]
#[path = "npc_shop_tests.rs"]
mod tests;
