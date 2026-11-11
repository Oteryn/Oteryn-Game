//! Closed native Benevola NPC callback; durable debit and recovery use existing owners.
use super::*;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::quest_state::QuestTransitionOutcome;
use crate::durability::quest_world_callback::QuestWorldContext;
use crate::durability::world_event::changes::counters::benevola::{
    BenevolaHandover, BenevolaSource, NPC,
};
use crate::foundation::{CommandId, CommandRef};

#[derive(Clone)]
pub(super) struct Pending {
    source: BenevolaSource,
    fence: CurrentCharacterItemFence,
    opened: u64,
    retained: Option<BenevolaHandover>,
}
pub(super) enum Step {
    Reply(&'static str),
    Commit(Box<Pending>),
}
fn encoded(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn observed(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    command: super::super::connection::UseCommand,
    intent: &NpcTalkIntent,
) -> Option<BenevolaSource> {
    let context = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        runtime,
        actor,
        command.game_session_id,
        intent,
    )?;
    let npc = service_context::live_npc(runtime, actor, command.game_session_id, intent.npc_actor)?;
    if context.2 != NPC
        || (npc.position.x, npc.position.y, npc.position.floor) != (32596, 31745, 7)
        || !runtime
            .player_control_facts(actor, command.game_session_id)
            .is_ok_and(|f| f.control_loss.is_none())
    {
        return None;
    }
    let position = runtime.read_actor_position(actor).ok()?;
    Some(BenevolaSource {
        npc_identity: intent.npc_actor.identity,
        npc_generation: intent.npc_actor.generation,
        catalogue_revision: context.0,
        conversation_revision: context.1,
        position: [32596, 31745, 7],
        world: QuestWorldContext {
            party: Vec::new(),
            map_revision: format!(
                "sha256:{}",
                encoded(&runtime.content_pin().map_revision_digest())
            ),
            content_revision: format!(
                "sha256:{}",
                encoded(&runtime.content_pin().server_artifact_digest())
            ),
            placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
            reset_epoch: admission.reset_epoch,
            actor_placement: actor.placement_identity(),
            movement_revision: position.revision(),
        },
    })
}
pub(super) fn prepare(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: super::super::connection::UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> Option<Step> {
    let source = observed(admission, runtime, actor, command, intent)?;
    let fence = command.item_fence?;
    let now = admission.owner_now().get();
    let mut sessions = admission.states.npc_conversations.sessions.lock().ok()?;
    let state = sessions.get_mut(&command.game_session_id)?;
    let words: Vec<_> = intent
        .text
        .split(|c: char| !c.is_ascii_alphanumeric())
        .map(str::to_ascii_lowercase)
        .collect();
    let says = |word: &str| words.iter().any(|w| w == word);
    if let Some(pending) = &state.benevola
        && pending.retained.is_some()
    {
        return Some(Step::Commit(Box::new(pending.clone())));
    }
    // Preserve the donor's wolf topic precedence over trap/hunger/mission.
    if says("wolf") || says("wolves") {
        return Some(Step::Reply(
            "You might have occasionally encountered a few wolves and probably even fought them. Those are the few wolves that got too hungry and daring so they left their hiding place. ...",
        ));
    }
    if ["trap", "hunger", "mission"].iter().any(|w| says(w)) {
        state.benevola = Some(Pending {
            source,
            fence,
            opened: now,
            retained: None,
        });
        return Some(Step::Reply("Have you captured any wolves?"));
    }
    let mut pending = state.benevola.take()?;
    if pending.fence != fence
        || now < pending.opened
        || now - pending.opened >= 30_000_000
        || pending.source.npc_identity != source.npc_identity
        || pending.source.npc_generation != source.npc_generation
        || pending.source.catalogue_revision != source.catalogue_revision
        || pending.source.conversation_revision != source.conversation_revision
        || pending.source.world.reset_epoch != source.world.reset_epoch
        || pending.source.world.content_revision != source.world.content_revision
        || pending.source.world.placement_context != source.world.placement_context
    {
        return Some(Step::Reply(
            "Perhaps another time then. The forest needs protectors.",
        ));
    }
    if !says("yes") {
        return Some(Step::Reply(
            "Perhaps another time then. The forest needs protectors.",
        ));
    }
    pending.retained = Some(BenevolaHandover {
        command: CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).ok()?,
        ),
        source: pending.source.clone(),
    });
    state.benevola = Some(pending.clone());
    Some(Step::Commit(Box::new(pending)))
}
pub(super) async fn apply(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: super::super::connection::UseCommand,
    intent: NpcTalkIntent,
    step: Step,
) -> NpcTalkOutcome {
    let rejected = || NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    let pending = match step {
        Step::Reply(line) => return NpcConversations::bank_reply(&intent, line.into()),
        Step::Commit(pending) => pending,
    };
    let Some(handover) = pending.retained.clone() else {
        return rejected();
    };
    let Some(permit) = admission.spell_lane_permit().await else {
        return rejected();
    };
    if !admission
        .states
        .player_trades
        .physical_service(command.game_session_id)
    {
        return rejected();
    }
    let mut slot = admission
        .revision_sequencer
        .acquire(pending.fence.character_id)
        .await;
    let Ok(Some(fence)) = admission.current_quest_fence(command.game_session_id).await else {
        return rejected();
    };
    if fence.item_fence() != pending.fence {
        return rejected();
    }
    {
        let runtime = admission.runtime.lock().await;
        let Ok(states) = admission.states.spell_states.try_lock() else {
            return rejected();
        };
        let Some(current) = observed(admission, &runtime, actor, command, &intent) else {
            return rejected();
        };
        if current.npc_identity != pending.source.npc_identity
            || current.npc_generation != pending.source.npc_generation
            || current.catalogue_revision != pending.source.catalogue_revision
            || current.conversation_revision != pending.source.conversation_revision
            || current.world.reset_epoch != pending.source.world.reset_epoch
            || current.world.content_revision != pending.source.world.content_revision
            || current.world.placement_context != pending.source.world.placement_context
            || states.is_dead(actor)
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
    let Some(content) = admission.quest_catalogue.as_ref() else {
        return rejected();
    };
    let Ok(catalogue) = BenevolaHandover::catalogue(content.content_revision()) else {
        return rejected();
    };
    let result = slot
        .commit_benevola_handover(
            admission.root,
            admission.character,
            admission.holder,
            fence,
            &permit,
            handover,
            catalogue,
        )
        .await;
    drop(slot);
    drop(permit);
    match result {
        Ok(QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)) => {
            if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
                && let Some(state) = sessions.get_mut(&command.game_session_id)
            {
                state.benevola = None;
            }
            admission
                .refresh_quest_session(command.game_session_id, false)
                .await;
            let mut reply = NpcConversations::bank_reply(
                &intent,
                "Thank you for helping maintain the balance!".into(),
            );
            reply.inventory_changed = true;
            reply
        }
        Ok(QuestTransitionOutcome::Refused(_)) => {
            if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
                && let Some(state) = sessions.get_mut(&command.game_session_id)
            {
                state.benevola = None;
            }
            NpcConversations::bank_reply(
                &intent,
                "You do not have any captured wolves with you.".into(),
            )
        }
        Ok(_) => rejected(),
        Err(_) => NpcTalkOutcome {
            unknown: true,
            ..rejected()
        },
    }
}

/// Reconcile retained history before conversation/calendar checks. A changed source
/// can acknowledge the old receipt, but can never execute its old debit or credit.
pub(super) async fn resume(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: super::super::connection::UseCommand,
    intent: &NpcTalkIntent,
) -> Option<NpcTalkOutcome> {
    let pending = admission
        .states
        .npc_conversations
        .sessions
        .lock()
        .ok()?
        .get(&command.game_session_id)?
        .benevola
        .as_ref()?
        .clone();
    let handover = pending.retained.as_ref()?.clone();
    // A retained Benevola result must never intercept another NPC's conversation.
    {
        let runtime = admission.runtime.lock().await;
        let npc =
            service_context::live_npc(&runtime, actor, command.game_session_id, intent.npc_actor)?;
        if npc.key != NPC {
            return None;
        }
    }
    let result = admission
        .root
        .reconcile_benevola_handover(admission.character, pending.fence.character_id, handover)
        .await;
    match result {
        Ok(Some(_)) => {
            if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
                && let Some(state) = sessions.get_mut(&command.game_session_id)
            {
                state.benevola = None;
            }
            admission
                .refresh_quest_session(command.game_session_id, false)
                .await;
            let mut reply = NpcConversations::bank_reply(
                intent,
                "Thank you for helping maintain the balance!".into(),
            );
            reply.inventory_changed = true;
            Some(reply)
        }
        Ok(None) => {
            let current = {
                let runtime = admission.runtime.lock().await;
                observed(admission, &runtime, actor, command, intent)
            };
            let original_context = current.is_some_and(|source| {
                command.item_fence == Some(pending.fence)
                    && source.npc_identity == pending.source.npc_identity
                    && source.npc_generation == pending.source.npc_generation
                    && source.catalogue_revision == pending.source.catalogue_revision
                    && source.conversation_revision == pending.source.conversation_revision
                    && source.world.reset_epoch == pending.source.world.reset_epoch
                    && source.world.content_revision == pending.source.world.content_revision
                    && source.world.placement_context == pending.source.world.placement_context
            });
            if original_context {
                return None;
            }
            // Absence is definitive while holding the existing receipt-key lock. Drop an
            // expired invocation; a fresh mission must use a new command/source binding.
            if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
                && let Some(state) = sessions.get_mut(&command.game_session_id)
            {
                state.benevola = None;
            }
            Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected))
        }
        Err(_) => Some(NpcTalkOutcome {
            unknown: true,
            ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
        }),
    }
}

#[cfg(test)]
impl NpcConversations {
    /// Fault injection retains exactly the original invocation after an ambiguous acknowledgement.
    pub(in crate::gameplay_transport) fn retain_benevola_for_test(
        &self,
        fence: CurrentCharacterItemFence,
        handover: BenevolaHandover,
    ) {
        let session = handover.command.game_session_id();
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.entry(session).or_default().benevola = Some(Pending {
                source: handover.source.clone(),
                fence,
                opened: 0,
                retained: Some(handover),
            });
        }
    }
}
