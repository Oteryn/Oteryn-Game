//! `NPC_TRADE_INTENT` (command 8) as one durable NPC trade (NPC-TRADE-2, NPC-0 §4/§5).
//!
//! The command's occurrence, TransactionId, EventId and output identities are derived from its
//! CommandRef, so every retry of one command reuses them and the occurrence is bound 1:1 to the
//! CommandRef; the occurrence also binds the intent's NPC actor. Replay comes first and reads the
//! stored trade, never the live NPC: a trade the command committed answers `OK` when it is the
//! occurrence of this actor and the intent names what it committed, before any conversation or
//! window check, because a reconnect closes both. Otherwise [`NpcConversations::trade_in`](super::npc_command::NpcConversations)
//! resolves the offer from the server catalogue under the runtime lock, and the trade commits under
//! the session's Character item fence. A storage failure leaves the outcome unknown: the connection
//! ends before the CommandId is sequenced and the resend is answered by replay.

use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTradeIntent, NpcTradeSide};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use sha2::{Digest, Sha256};

use super::ComposedFreshAdmission;
use super::connection::UseCommand;
use super::item_ref_admission::{generation_item_facts, pinned_index};
use super::npc_command::NpcTradeRefusal;
use crate::content::ActiveGeneration;
use crate::content::item_ref::ITEM_FAMILY;
use crate::domain::currency::{COIN_DEFINITION_FAMILY, Coin};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::ItemTransferError;
use crate::durability::npc_trade::{
    self, NpcTradeCause, NpcTradeCoins, NpcTradeError, NpcTradeReceipt, NpcTradeRequest,
};
use crate::foundation::{CommandId, CommandRef, ExactActorRef, GameSessionId};

/// What the connection does with one command 8.
pub enum NpcTradeStep {
    /// The trade committed now or by replay: `OK`, then the inventory delta.
    Traded,
    /// Nothing committed: the disposition and the domain 7 and 8 payloads.
    Refused(NpcTradeRefusal),
    /// The outcome is unknown: end the connection without sequencing the CommandId.
    Unknown,
}

impl NpcTradeStep {
    fn rejected() -> Self {
        Self::Refused(NpcTradeRefusal::only(NpcIntentDisposition::Rejected))
    }
}

/// A UUIDv7-shaped identity of the command `command_id` of `session`, distinct per `tag`.
pub(super) fn derived(tag: &[u8], session: GameSessionId, command_id: u64) -> [u8; 16] {
    let hash = Sha256::new()
        .chain_update(tag)
        .chain_update(session.as_bytes())
        .chain_update(command_id.to_be_bytes())
        .finalize();
    uuid_v7_shaped(&hash)
}

fn uuid_v7_shaped(hash: &[u8]) -> [u8; 16] {
    let mut id = [0; 16];
    id.copy_from_slice(&hash[..16]);
    id[6] = (id[6] & 0x0f) | 0x70;
    id[8] = (id[8] & 0x3f) | 0x80;
    id
}

/// The occurrence of command 8 `command_id` of `session` (NPC-0 §4: one per command), bound to
/// the NPC actor the intent names: the stored occurrence is the immutable record of that actor, so
/// a resend naming another actor is another occurrence and never replays this one's trade.
pub(super) fn occurrence(
    session: GameSessionId,
    command_id: u64,
    npc_actor: EntityRef,
) -> [u8; 16] {
    let hash = Sha256::new()
        .chain_update(b"oteryn:npc-trade-occurrence:v2")
        .chain_update(session.as_bytes())
        .chain_update(command_id.to_be_bytes())
        .chain_update(npc_actor.identity)
        .chain_update(npc_actor.generation.to_be_bytes())
        .finalize();
    uuid_v7_shaped(&hash)
}

const fn side_of(side: NpcTradeSide) -> npc_trade::NpcTradeSide {
    match side {
        NpcTradeSide::Buy => npc_trade::NpcTradeSide::Buy,
        NpcTradeSide::Sell => npc_trade::NpcTradeSide::Sell,
    }
}

/// Whether the committed trade answered this intent: the same offer, side, quantity and price (the
/// occurrence already binds the NPC actor).
fn answers(receipt: &NpcTradeReceipt, intent: &NpcTradeIntent) -> bool {
    receipt.catalogue_revision == intent.catalogue_revision
        && receipt.offer_index == intent.offer_index
        && receipt.side == side_of(intent.side)
        && receipt.quantity == intent.quantity
        && receipt.unit_price == u64::from(intent.expected_unit_price)
}

pub(super) fn unix_ms_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_millis()).ok())
        .unwrap_or(0)
}

/// Command 8 of the admitted `actor`: replay first, then the offer the server catalogue holds,
/// committed as one NPC trade.
pub(super) async fn npc_trade(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTradeIntent,
) -> NpcTradeStep {
    let session = command.game_session_id;
    // Without an item fence no trade can commit or have committed for the session.
    let (Some(fence), Ok(command_id)) = (command.item_fence, CommandId::new(command.command_id))
    else {
        return NpcTradeStep::rejected();
    };
    let Some(_permit) = admission.spell_lane_permit().await else {
        return NpcTradeStep::Unknown;
    };
    let command_ref = CommandRef::new(session, command_id);
    let occurrence = occurrence(session, command.command_id, intent.npc_actor);
    match admission
        .root
        .read_npc_trade(
            admission.character,
            fence.character_id,
            command_ref,
            occurrence,
        )
        .await
    {
        Ok(Some(receipt)) if answers(&receipt, &intent) => return NpcTradeStep::Traded,
        Ok(None) => {}
        Err(NpcTradeError::Unavailable(_)) => return NpcTradeStep::Unknown,
        Ok(Some(_)) | Err(_) => return NpcTradeStep::rejected(),
    }

    if !admission.states.player_trades.physical_service(session) {
        return NpcTradeStep::rejected();
    }
    let (offer, coins) = {
        let runtime = admission.runtime.lock().await;
        let index = pinned_index(&runtime, admission.active_generation);
        let offer = match admission.states.npc_conversations.trade_in(
            &admission.states.chat,
            &runtime,
            actor,
            session,
            &intent,
            index,
        ) {
            Ok(offer) => offer,
            Err(refusal) => return NpcTradeStep::Refused(*refusal),
        };
        let coin = |coin: Coin| {
            let production_key = coin.production_key();
            index
                .and_then(|index| index.revision(production_key))
                .map(|revision| TypedDefinitionRef {
                    family: COIN_DEFINITION_FAMILY.to_owned(),
                    production_key: production_key.to_owned(),
                    revision_ref: revision.to_owned(),
                })
        };
        let (Some(gold), Some(platinum), Some(crystal)) =
            (coin(Coin::Gold), coin(Coin::Platinum), coin(Coin::Crystal))
        else {
            return NpcTradeStep::rejected();
        };
        (
            offer,
            NpcTradeCoins {
                gold,
                platinum,
                crystal,
            },
        )
    };
    let Some(native) = admission
        .active_generation
        .and_then(ActiveGeneration::native_gameplay)
    else {
        return NpcTradeStep::rejected();
    };
    let Ok(item) = generation_item_facts(
        native,
        &TypedDefinitionRef {
            family: ITEM_FAMILY.to_owned(),
            production_key: offer.item_key.clone(),
            revision_ref: offer.item_revision.clone(),
        },
    ) else {
        return NpcTradeStep::rejected();
    };
    let backpack = match admission
        .root
        .read_character_backpack(admission.character, fence.character_id)
        .await
    {
        Ok(Some(current)) => match generation_item_facts(native, &current.backpack.definition) {
            Ok(facts) => facts,
            Err(_) => return NpcTradeStep::rejected(),
        },
        Ok(None) => return NpcTradeStep::rejected(),
        Err(ItemTransferError::Unavailable(_)) => return NpcTradeStep::Unknown,
        Err(_) => return NpcTradeStep::rejected(),
    };
    let id = |tag: &[u8]| derived(tag, session, command.command_id);
    let request = NpcTradeRequest {
        cause: NpcTradeCause {
            npc_key: offer.npc_key,
            catalogue_revision: offer.catalogue_revision,
            offer_index: offer.offer_index,
            side: side_of(offer.side),
            occurrence,
        },
        command: command_ref,
        quantity: offer.quantity,
        unit_price: u64::from(offer.unit_price),
        offer_count: offer.count,
        item,
        transaction_id: id(b"oteryn:npc-trade-transaction:v1"),
        event_id: id(b"oteryn:npc-trade-event:v1"),
        occurred_at_unix_ms: unix_ms_now(),
        output_item_instance_ids: [
            id(b"oteryn:npc-trade-output-1:v1"),
            id(b"oteryn:npc-trade-output-2:v1"),
            id(b"oteryn:npc-trade-output-3:v1"),
        ],
        coins,
        backpack,
    };
    step_of(
        admission
            .root
            .commit_npc_trade(admission.character, admission.holder, fence, request)
            .await,
    )
}

/// The step of a commit result: any committed outcome is `OK`; a storage failure is unknown; every
/// refusal wrote nothing and is `REJECTED`.
fn step_of(result: Result<npc_trade::NpcTradeOutcome, NpcTradeError>) -> NpcTradeStep {
    match result {
        Ok(_) => NpcTradeStep::Traded,
        Err(NpcTradeError::Unavailable(_)) => NpcTradeStep::Unknown,
        Err(_) => NpcTradeStep::rejected(),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, reason = "test fixtures")]
#[path = "npc_trade_command_tests.rs"]
mod tests;
