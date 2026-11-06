//! Source NPC_TALK barter over the existing atomic craft owner.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use crate::durability::item_craft::npc_exchange::NpcQuestExchange;
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::rookgaard_exchanges::exchange;
use oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent;
use oteryn_protocol_oteryn::world_object::UseDisposition;

type Context = (u64, u64, String);

#[derive(Default)]
pub(super) struct State {
    topic: Option<(Context, EntityRef)>,
    pending: Option<CommandRef>,
}

pub(in crate::gameplay_transport) enum Step {
    Reply(&'static str),
    Exchange(NpcQuestExchange),
}

pub(in crate::gameplay_transport) fn prepare(
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
    let rule = exchange(&context.2)?;
    if context.2 == "oteryn:npc.sinatuki" && intent.text.to_ascii_lowercase().contains("chuqua") {
        return Some(Step::Reply("Chuqua jamjam!! Tiyopa Sinatuki?"));
    }
    let mut sessions = admission.states.npc_conversations.sessions.lock().ok()?;
    let state = &mut sessions
        .get_mut(&command.game_session_id)?
        .quest_dialogue
        .rookgaard_exchange;
    let previous = state
        .topic
        .take()
        .filter(|(quoted, npc)| *quoted == context && *npc == intent.npc_actor);
    // The source searches the pending node's yes/catch-all children before
    // root keywords, even when a reply also contains the offer keyword.
    if previous.is_some() && !rule.confirmed_by(&intent.text) {
        return Some(Step::Reply(rule.declined));
    }
    if previous.is_none()
        && rule.offered_by(&intent.text)
        && let Some(prompt) = rule.prompt
    {
        state.topic = Some((context, intent.npc_actor));
        return Some(Step::Reply(prompt));
    }
    let confirmed = if rule.prompt.is_some() {
        previous.is_some() && rule.confirmed_by(&intent.text)
    } else {
        rule.offered_by(&intent.text)
    };
    if confirmed {
        return Some(Step::Exchange(NpcQuestExchange {
            npc_key: context.2,
            catalogue_revision: context.0,
            conversation_revision: context.1,
            npc_identity: intent.npc_actor.identity,
            npc_generation: intent.npc_actor.generation,
        }));
    }
    None
}

fn unknown() -> NpcTalkOutcome {
    NpcTalkOutcome {
        unknown: true,
        ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
    }
}

pub(in crate::gameplay_transport) async fn apply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTalkIntent,
    step: Step,
) -> NpcTalkOutcome {
    let binding = match step {
        Step::Reply(line) => return NpcConversations::bank_reply(&intent, line.into()),
        Step::Exchange(binding) => binding,
    };
    execute(admission, actor, command, Some(&intent), Some(binding))
        .await
        .unwrap_or_else(unknown)
}

pub(in crate::gameplay_transport) async fn resume(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: Option<&NpcTalkIntent>,
) -> Option<NpcTalkOutcome> {
    execute(admission, actor, command, intent, None).await
}

async fn execute(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: Option<&NpcTalkIntent>,
    binding: Option<NpcQuestExchange>,
) -> Option<NpcTalkOutcome> {
    let fence = command.item_fence?;
    let reference = CommandRef::new(
        command.game_session_id,
        CommandId::new(command.command_id).ok()?,
    );
    let native = admission.active_generation?.native_gameplay()?;
    let Some(permit) = admission.spell_lane_permit().await else {
        return Some(unknown());
    };
    let runtime = admission.runtime.lock().await;
    if let Some(binding) = &binding {
        let intent = intent?;
        let context = (
            binding.catalogue_revision,
            binding.conversation_revision,
            binding.npc_key.clone(),
        );
        let Ok(states) = admission.states.spell_states.try_lock() else {
            return Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected));
        };
        if states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, command.game_session_id)
            || !admission
                .states
                .player_trades
                .physical_service(command.game_session_id)
            || runtime.content_pin().server_artifact_digest() != native.source_digest()
            || admission.states.npc_conversations.current_npc(
                &admission.states.chat,
                &runtime,
                actor,
                command.game_session_id,
                intent,
            ) != Some(context)
            || !runtime
                .bound_attacker_lease(actor, command.game_session_id)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
        {
            return Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected));
        }
    }
    let wire = match intent.map(encode_npc_talk_intent).transpose() {
        Ok(wire) => wire,
        Err(_) => return Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected)),
    };
    if binding.is_some() {
        let mut sessions = admission.states.npc_conversations.sessions.lock().ok()?;
        sessions
            .entry(command.game_session_id)
            .or_default()
            .quest_dialogue
            .rookgaard_exchange
            .pending = Some(reference);
    }
    drop(runtime);
    let result = admission
        .root
        .exchange_npc_quest_item(
            &permit,
            admission.character,
            admission.holder,
            fence,
            reference,
            wire,
            binding,
            native,
            &super::super::item_ref_admission::NativeItemWeights(native),
        )
        .await;
    match result {
        Err(_) => Some(unknown()),
        Ok(receipt) => {
            let runtime = admission.runtime.lock().await;
            let pending = {
                let mut sessions = admission.states.npc_conversations.sessions.lock().ok()?;
                let state = &mut sessions
                    .entry(command.game_session_id)
                    .or_default()
                    .quest_dialogue
                    .rookgaard_exchange;
                if state.pending == Some(reference) {
                    state.pending.take()
                } else {
                    None
                }
            };
            let Some((binding, disposition, rewarded)) = receipt else {
                return pending.map(|_| NpcTalkOutcome::only(NpcIntentDisposition::Rejected));
            };
            let rule = exchange(&binding.npc_key)?;
            let line = match disposition {
                UseDisposition::Committed
                    if binding.npc_key == "oteryn:npc.sinatuki" && rewarded =>
                {
                    "Jinuma, suvituka siq chuqua!! Nguraka, nguraka! <happily takes the food from you and gives you five glimmering crystals>"
                }
                UseDisposition::Committed => rule.success,
                UseDisposition::NothingToUse => rule.missing,
                _ => return Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected)),
            };
            let intent = NpcTalkIntent {
                npc_actor: EntityRef {
                    identity: binding.npc_identity,
                    generation: binding.npc_generation,
                },
                text: String::new(),
            };
            let current = admission.states.npc_conversations.current_npc(
                &admission.states.chat,
                &runtime,
                actor,
                command.game_session_id,
                &intent,
            ) == Some((
                binding.catalogue_revision,
                binding.conversation_revision,
                binding.npc_key,
            ));
            let mut outcome = NpcConversations::bank_reply(&intent, line.into());
            outcome.inventory_changed = disposition == UseDisposition::Committed;
            if !current && let Some(conversation) = &mut outcome.conversation {
                conversation.open = None;
            }
            Some(outcome)
        }
    }
}
