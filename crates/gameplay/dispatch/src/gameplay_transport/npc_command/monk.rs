//! The pilgrimage uses the existing NPC conversation, quest writer and replay receipt.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use crate::domain::CharacterId;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::monk_shrines as quest;

pub(super) struct Step {
    context: (u64, u64, String),
    transition: Option<&'static str>,
    reply: String,
}

#[derive(Clone)]
pub(super) struct Pending {
    character: CharacterId,
    request: QuestTransitionRequest,
    intent: NpcTalkIntent,
    reply: String,
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
    let manop = context.2 == "oteryn:npc.ambassador_manop";
    if !manop && context.2 != "oteryn:npc.enpa_deia_pema" {
        return None;
    }
    let text = intent.text.trim().to_ascii_lowercase();
    if ![
        "pilgrimage",
        "progress",
        "report",
        "seeker",
        "three-fold path",
    ]
    .contains(&text.as_str())
    {
        return None;
    }
    let sessions = admission.states.quest_sessions.lock().ok()?;
    let copy = sessions.get(&command.game_session_id)?.copy.as_ref()?;
    let tracks = copy.tracks();
    let completed = quest::completed_count(tracks);
    let started = tracks.get(quest::PILGRIMAGE_TRACK).copied().unwrap_or(0) >= 3;
    let reported = tracks.get(quest::FINAL_TRACK) == Some(&1);
    let (transition, reply) = if manop {
        (if tracks.get(quest::START_TRACK).copied().unwrap_or(0) < 1 { Some(quest::INVITE) } else { None },
            "I invite you to follow the Three-Fold Path of the Merudri. Seek Enpa-Deia Pema in Blue Valley and ask about the pilgrimage.".into())
    } else if reported {
        (None, "You have honoured our legacy and reported your pilgrimage. The Merudri thank you, venerable traveller.".into())
    } else if !started {
        (Some(quest::START), "Your pilgrimage begins. Honour the ten mainland shrines in order, beginning with Guidance in Targuna, and return to me after Eternity.".into())
    } else if completed == 10 {
        (Some(quest::FINISH), "You have honoured all ten shrines. Your pilgrimage is complete, and your final promotion points are now available in the Wheel of Destiny.".into())
    } else {
        let next = quest::NAMES
            .iter()
            .find(|name| tracks.get(&format!("{}{name}", quest::FLAG_PREFIX)) != Some(&1))?;
        (
            None,
            format!(
                "You have honoured {completed}/10 mainland shrines. Your next shrine is {next}. Follow the Three-Fold Path and return after Eternity."
            ),
        )
    };
    Some(Step {
        context,
        transition,
        reply,
    })
}

fn clear(admission: &ComposedFreshAdmission<'_, '_, '_>, session: GameSessionId) {
    if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
        && let Some(state) = sessions.get_mut(&session)
    {
        state.monk = None;
    }
}

pub(super) async fn apply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTalkIntent,
    step: Step,
) -> NpcTalkOutcome {
    let rejected = || NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    let unknown = || NpcTalkOutcome {
        unknown: true,
        ..rejected()
    };
    let Some(transition) = step.transition else {
        return NpcConversations::bank_reply(&intent, step.reply);
    };
    let (Some(item_fence), Ok(id), Some(catalogue)) = (
        command.item_fence,
        CommandId::new(command.command_id),
        admission.quest_catalogue.as_ref(),
    ) else {
        return rejected();
    };
    let mut slot = admission
        .revision_sequencer
        .acquire(item_fence.character_id)
        .await;
    let Ok(Some(fence)) = admission.current_quest_fence(command.game_session_id).await else {
        return unknown();
    };
    if fence.item_fence() != item_fence {
        return rejected();
    }
    {
        let runtime = admission.runtime.lock().await;
        let states = admission.states.spell_states.lock().await;
        if states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, command.game_session_id)
            || admission.states.npc_conversations.current_npc(
                &admission.states.chat,
                &runtime,
                actor,
                command.game_session_id,
                &intent,
            ) != Some(step.context)
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
    let request = QuestTransitionRequest {
        transition_key: transition.into(),
        cause: QuestCause::Command(CommandRef::new(command.game_session_id, id)),
    };
    {
        let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock() else {
            return rejected();
        };
        let state = sessions.entry(command.game_session_id).or_default();
        if state.monk.is_some() {
            return unknown();
        }
        state.monk = Some(Pending {
            character: fence.character_id,
            request: request.clone(),
            intent: intent.clone(),
            reply: step.reply.clone(),
        });
    }
    let outcome = slot
        .commit_quest_transition(
            admission.root,
            admission.character,
            admission.holder,
            fence,
            request,
            std::sync::Arc::clone(catalogue),
        )
        .await;
    drop(slot);
    match outcome {
        Ok(QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)) => {
            clear(admission, command.game_session_id);
            admission
                .refresh_quest_session(command.game_session_id, false)
                .await;
            NpcConversations::bank_reply(&intent, step.reply)
        }
        Err(crate::durability::character_progression::CharacterProgressionError::Unavailable(
            _,
        )) => unknown(),
        _ => {
            clear(admission, command.game_session_id);
            rejected()
        }
    }
}

pub(in crate::gameplay_transport) async fn resume(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
) -> Option<NpcTalkOutcome> {
    let pending = admission
        .states
        .npc_conversations
        .sessions
        .lock()
        .ok()?
        .get(&command.game_session_id)?
        .monk
        .clone()?;
    let Ok(id) = CommandId::new(command.command_id) else {
        return None;
    };
    if pending.request.cause != QuestCause::Command(CommandRef::new(command.game_session_id, id)) {
        return None;
    }
    match admission
        .root
        .reconcile_character_quest_transition(
            admission.character,
            pending.character,
            pending.request,
        )
        .await
    {
        Ok(receipt) => {
            clear(admission, command.game_session_id);
            if receipt.is_some() {
                admission
                    .refresh_quest_session(command.game_session_id, false)
                    .await;
                Some(NpcConversations::bank_reply(&pending.intent, pending.reply))
            } else {
                Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected))
            }
        }
        Err(_) => Some(NpcTalkOutcome {
            unknown: true,
            ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
        }),
    }
}
