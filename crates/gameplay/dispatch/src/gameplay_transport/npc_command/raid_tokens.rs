//! Jorge's source dialogue uses the C8 exchange arm of the existing NPC item writer.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use crate::durability::npc_trade::{
    NpcTradeCause, NpcTradeCoins, NpcTradeError, NpcTradeRequest, NpcTradeSide,
};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::raid_tokens as source;

type Context = (u64, u64, String);
#[derive(Clone)]
pub(super) struct Conversation {
    context: Context,
    actor: ExactActorRef,
    npc: EntityRef,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
}
pub(super) enum Step {
    Reply(String),
    Exchange(Box<Conversation>, usize),
}

fn selected(text: &str) -> Option<usize> {
    let text = text.to_ascii_lowercase();
    source::offers()
        .ok()?
        .iter()
        .position(|(_, name)| text.contains(&name.to_ascii_lowercase()))
}
fn occurrence(command: UseCommand, intent: &NpcTalkIntent) -> [u8; 16] {
    let tag = format!(
        "oteryn:jorge-exchange:v1:{:?}:{}:{}",
        intent.npc_actor.identity, intent.npc_actor.generation, intent.text
    );
    super::super::npc_trade_command::derived(
        tag.as_bytes(),
        command.game_session_id,
        command.command_id,
    )
}
fn reply(intent: &NpcTalkIntent, changed: bool, text: &str) -> NpcTalkOutcome {
    NpcTalkOutcome {
        inventory_changed: changed,
        ..NpcConversations::bank_reply(intent, text.into())
    }
}

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> Option<Step> {
    let context = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        runtime,
        actor,
        command.game_session_id,
        intent,
    )?;
    let mut sessions = admission.states.npc_conversations.sessions.lock().ok()?;
    let state = sessions.get_mut(&command.game_session_id)?;
    if context.2 != source::NPC {
        state.raid_tokens = None;
        return None;
    }
    let text = intent.text.to_ascii_lowercase();
    if text.contains("souvenir") {
        state.raid_tokens = Some(Conversation {
            context,
            actor,
            npc: intent.npc_actor,
            fence: command.item_fence?,
        });
        return Some(Step::Reply(
            "Name the souvenir you want for 20 silver raid tokens.".into(),
        ));
    }
    if ["hi", "hello", "hail", "bye"].contains(&text.trim()) {
        state.raid_tokens = None;
        return None;
    }
    let previous = state.raid_tokens.as_ref()?;
    if previous.context != context
        || previous.actor != actor
        || previous.npc != intent.npc_actor
        || Some(previous.fence) != command.item_fence
    {
        state.raid_tokens = None;
        return None;
    }
    selected(&intent.text).map(|index| Step::Exchange(Box::new(previous.clone()), index))
}

pub(super) async fn resume(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
    intent: &NpcTalkIntent,
) -> Option<NpcTalkOutcome> {
    let fence = command.item_fence?;
    let reference = CommandRef::new(
        command.game_session_id,
        CommandId::new(command.command_id).ok()?,
    );
    match admission
        .root
        .read_npc_trade(
            admission.character,
            fence.character_id,
            reference,
            occurrence(command, intent),
        )
        .await
    {
        Ok(None) => None,
        Ok(Some(receipt))
            if receipt.npc_key == source::NPC
                && receipt.side == NpcTradeSide::Exchange
                && selected(&intent.text) == Some(receipt.offer_index as usize) =>
        {
            Some(reply(intent, true, "Here is your souvenir."))
        }
        Ok(Some(_)) | Err(NpcTradeError::ConflictingOccurrence) => {
            Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected))
        }
        Err(_) => Some(NpcTalkOutcome {
            unknown: true,
            ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
        }),
    }
}

pub(super) async fn apply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTalkIntent,
    step: Step,
) -> NpcTalkOutcome {
    let (conversation, index) = match step {
        Step::Reply(text) => return reply(&intent, false, &text),
        Step::Exchange(c, i) => (*c, i),
    };
    let rejected = || NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    let unknown = || NpcTalkOutcome {
        unknown: true,
        ..rejected()
    };
    let Some(fence) = command.item_fence else {
        return rejected();
    };
    let Some(_permit) = admission.spell_lane_permit().await else {
        return unknown();
    };
    if !admission
        .states
        .player_trades
        .physical_service(command.game_session_id)
    {
        return rejected();
    }
    let _slot = admission
        .revision_sequencer
        .acquire(fence.character_id)
        .await;
    let Ok(Some(current)) = admission.current_quest_fence(command.game_session_id).await else {
        return unknown();
    };
    if current.item_fence() != fence || conversation.fence != fence {
        return rejected();
    }
    let runtime = admission.runtime.lock().await;
    {
        let Ok(states) = admission.states.spell_states.try_lock() else {
            return rejected();
        };
        if states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, command.game_session_id)
            || !runtime
                .bound_attacker_lease(actor, command.game_session_id)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
        {
            return rejected();
        }
    }
    if admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        &runtime,
        actor,
        command.game_session_id,
        &intent,
    ) != Some(conversation.context)
    {
        return rejected();
    }
    let Some((gold, platinum, crystal)) = bank_coin_definitions(admission, &runtime) else {
        return rejected();
    };
    let Some(pin) =
        super::super::item_ref_admission::pinned_index(&runtime, admission.active_generation)
    else {
        return rejected();
    };
    let Some(native) = admission
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
    else {
        return rejected();
    };
    let facts = |id: u16| {
        let key = format!("oteryn:item.tibia.i{id}");
        let definition = crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            revision_ref: pin.revision(&key)?.into(),
            production_key: key,
        };
        super::super::item_ref_admission::generation_item_facts(native, &definition).ok()
    };
    let Ok(offers) = source::offers() else {
        return rejected();
    };
    let Some((output, _)) = offers.get(index) else {
        return rejected();
    };
    let (Some(token), Some(item)) = (facts(source::TOKEN), facts(*output)) else {
        return rejected();
    };
    drop(runtime);
    let Ok(Some(backpack)) = admission
        .root
        .read_character_backpack(admission.character, fence.character_id)
        .await
    else {
        return rejected();
    };
    let Ok(backpack) = super::super::item_ref_admission::generation_item_facts(
        native,
        &backpack.backpack.definition,
    ) else {
        return rejected();
    };
    let Ok(command_id) = CommandId::new(command.command_id) else {
        return rejected();
    };
    let derive = |slot: u8| {
        super::super::npc_trade_command::derived(
            format!("oteryn:jorge-exchange-id:v1:{slot}").as_bytes(),
            command.game_session_id,
            command.command_id,
        )
    };
    let request = NpcTradeRequest {
        cause: NpcTradeCause {
            npc_key: source::NPC.into(),
            catalogue_revision: 1,
            offer_index: index as u32,
            side: NpcTradeSide::Exchange,
            occurrence: occurrence(command, &intent),
        },
        command: CommandRef::new(command.game_session_id, command_id),
        quantity: 1,
        unit_price: 0,
        offer_count: 1,
        item,
        transaction_id: derive(1),
        event_id: derive(2),
        occurred_at_unix_ms: super::super::npc_trade_command::unix_ms_now(),
        output_item_instance_ids: [derive(3), derive(4), derive(5)],
        coins: NpcTradeCoins {
            gold,
            platinum,
            crystal,
        },
        backpack,
    };
    let weights = super::super::item_ref_admission::NativeItemWeights(native)
        .with_wheel(admission.premium_coordinator);
    match admission
        .root
        .commit_raid_token_exchange(
            admission.character,
            admission.holder,
            fence,
            request,
            token,
            &weights,
        )
        .await
    {
        Ok(_) => {
            if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
                && let Some(state) = sessions.get_mut(&command.game_session_id)
            {
                state.raid_tokens = None;
            }
            reply(&intent, true, "Here is your souvenir.")
        }
        Err(NpcTradeError::InsufficientFunds) => {
            if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
                && let Some(state) = sessions.get_mut(&command.game_session_id)
            {
                state.raid_tokens = None;
            }
            reply(
                &intent,
                false,
                "You need 20 silver raid tokens in one stack.",
            )
        }
        Err(NpcTradeError::Unavailable(_)) => unknown(),
        Err(error) => {
            super::super::operator_event(&format!("jorge_exchange_refused reason={error}"));
            rejected()
        }
    }
}
