//! Ordered source NPC storage branches on the established talk and receipt owners.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use crate::domain::CharacterId;
use crate::durability::quest_state::{
    QuestTransitionOutcome,
    npc_program::{self, NpcProgramRequest},
};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::npc_rules::{NpcRuleCatalog, NpcRuleEffect, NpcRuleTrigger};
use std::sync::Arc;

type Context = (u64, u64, String);
#[derive(Clone, PartialEq, Eq)]
pub(super) struct Conversation {
    context: Context,
    actor: ExactActorRef,
    npc: EntityRef,
    npc_actor: ExactActorRef,
    topic: i32,
}
#[derive(Clone)]
pub(super) enum Pending {
    Topic(Conversation),
    Commit {
        character: CharacterId,
        request: NpcProgramRequest,
        intent: NpcTalkIntent,
        next: Conversation,
        fallback_lines: Vec<String>,
    },
}
pub(super) enum Step {
    Reply {
        lines: Vec<String>,
        end: bool,
        end_delay_ms: u64,
        npc_actor: ExactActorRef,
    },
    Refused,
    Held,
    Commit(Box<CommitStep>),
}
pub(super) struct CommitStep {
    context: Context,
    previous: Option<Conversation>,
    next: Conversation,
    request: NpcProgramRequest,
    fallback_lines: Vec<String>,
}

fn npc_needs_level(source: &NpcRuleCatalog, npc: &str) -> bool {
    source.programs().iter().any(|program| {
        program.document.rules.iter().any(|rule| {
            rule.npc == npc
                && rule.guards.iter().any(|guard| {
                    matches!(
                        guard,
                        oteryn_gameplay_quests::quest::npc_rules::NpcRuleGuard::Level { .. }
                    )
                })
        })
    })
}

pub(super) fn needs_level(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> bool {
    if admission.quest_catalogue.is_none() {
        return false;
    }
    let Some(context) = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        runtime,
        actor,
        command.game_session_id,
        intent,
    ) else {
        return false;
    };
    NpcRuleCatalog::runtime().is_ok_and(|source| npc_needs_level(source, &context.2))
}

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
    ordinary: &NpcTalkOutcome,
    level: Option<u32>,
) -> Option<Step> {
    prepare_inner(
        admission, actor, command, intent, runtime, ordinary, true, level,
    )
}

/// Probe the same planner without advancing a source topic. A held commit still
/// owns its conversation; an unsupported source branch does not claim a keyword.
pub(super) fn has_eligible_program(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
    level: Option<u32>,
) -> bool {
    matches!(
        prepare_inner(
            admission,
            actor,
            command,
            intent,
            runtime,
            &NpcTalkOutcome::only(NpcIntentDisposition::Ok),
            false,
            level,
        ),
        Some(Step::Reply { .. } | Step::Commit(_) | Step::Held)
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "the observed level joins the existing exact NPC context, reply and topic mode"
)]
fn prepare_inner(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
    ordinary: &NpcTalkOutcome,
    advance_topic: bool,
    level: Option<u32>,
) -> Option<Step> {
    let context = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        runtime,
        actor,
        command.game_session_id,
        intent,
    )?;
    let source = NpcRuleCatalog::runtime().ok()?;
    if !source
        .programs()
        .iter()
        .any(|p| p.document.rules.iter().any(|r| r.npc == context.2))
    {
        return None;
    }
    let catalogue = admission.quest_catalogue.as_ref()?;
    let tracks = {
        let sessions = admission.states.quest_sessions.lock().ok()?;
        let copy = sessions.get(&command.game_session_id)?.copy.as_ref()?;
        let mut tracks = copy.tracks().clone();
        for (quest, state) in copy.states() {
            if let Some(defaults) = oteryn_gameplay_quests::quest::npc_rules::legacy_defaults(
                catalogue,
                quest,
                &state.definition_hash,
            ) {
                for (key, value) in defaults {
                    tracks.entry(key.clone()).or_insert(*value);
                }
            }
        }
        tracks
    };
    let mut sessions = admission.states.npc_conversations.sessions.lock().ok()?;
    let state = sessions.get_mut(&command.game_session_id)?;
    if matches!(state.catalogue, Some(Pending::Commit { .. })) {
        return Some(Step::Held);
    }
    if npc_needs_level(source, &context.2) && level.is_none() {
        return Some(Step::Refused);
    }
    let previous = match &state.catalogue {
        Some(Pending::Topic(p))
            if p.context == context && p.actor == actor && p.npc == intent.npc_actor =>
        {
            Some(p.clone())
        }
        _ => None,
    };
    if previous.is_none() && advance_topic {
        state.catalogue = None;
    }
    let greeting = intent
        .text
        .split(|c: char| !c.is_ascii_alphabetic())
        .any(|word| {
            ["hi", "hello", "hail"]
                .iter()
                .any(|g| word.eq_ignore_ascii_case(g))
        });
    let mut request = NpcProgramRequest {
        command: CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).ok()?,
        ),
        npc: context.2.clone(),
        trigger: if greeting {
            NpcRuleTrigger::Greet
        } else {
            NpcRuleTrigger::Say
        },
        message: intent.text.clone(),
        topic: if greeting {
            0
        } else {
            previous.as_ref().map_or(0, |p| p.topic)
        },
        transition_key: String::new(),
    };
    // StorageTime is checked again at database transaction time before committing.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())?;
    let plan = match npc_program::preview(&request, &tracks, catalogue, now, level.unwrap_or(0)) {
        Ok(Some(plan)) => plan,
        Ok(None) => {
            if greeting && advance_topic {
                state.catalogue = None;
            }
            return None;
        }
        Err(_) => return Some(Step::Refused),
    };
    let next = Conversation {
        context: context.clone(),
        actor,
        npc: intent.npc_actor,
        npc_actor: admission.states.chat.engaged(command.game_session_id)?,
        topic: plan.next_topic,
    };
    if plan.track_changes.is_empty()
        && plan
            .effects
            .iter()
            .any(|effect| matches!(effect, NpcRuleEffect::Experience { .. }))
    {
        return Some(Step::Refused);
    }
    if plan.track_changes.is_empty() && request.npc != "oteryn:npc.pemaret" {
        let end = plan
            .effects
            .iter()
            .any(|effect| matches!(effect, NpcRuleEffect::EndConversation { .. }));
        let end_delay_ms = plan
            .effects
            .iter()
            .filter_map(|effect| match effect {
                NpcRuleEffect::EndConversation { delay_ms } => Some(*delay_ms),
                _ => None,
            })
            .min()
            .unwrap_or(0);
        let npc_actor = next.npc_actor;
        if advance_topic {
            state.catalogue = (!end).then_some(Pending::Topic(next));
        }
        return if plan.reply.is_empty() && !end {
            None
        } else {
            Some(Step::Reply {
                lines: plan.reply,
                end,
                end_delay_ms,
                npc_actor,
            })
        };
    }
    request.transition_key = plan.selected_rule_keys.first()?.clone();
    let fallback_lines = if greeting && plan.reply.is_empty() {
        ordinary
            .conversation
            .as_ref()
            .map(|conversation| conversation.lines.clone())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    Some(Step::Commit(Box::new(CommitStep {
        context,
        previous,
        next,
        request,
        fallback_lines,
    })))
}

fn response(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    intent: &NpcTalkIntent,
    mut lines: Vec<String>,
) -> NpcTalkOutcome {
    if let Some(name) = admission.states.chat.speaker_name(session) {
        for line in &mut lines {
            *line = oteryn_gameplay_npc::quest_greetings::render_player_name(line, &name);
        }
    }
    NpcTalkOutcome {
        unknown: false,
        inventory_changed: false,
        disposition: NpcIntentDisposition::Ok,
        conversation: Some(NpcConversation {
            open: Some((intent.npc_actor, NpcConversationState::Open)),
            lines,
        }),
        window: None,
    }
}
fn end_response(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    npc_actor: ExactActorRef,
    delay_ms: u64,
    response: &mut NpcTalkOutcome,
) {
    if delay_ms != 0 {
        let Some(generation) = admission.states.chat.npc_conversation_generation(session) else {
            return;
        };
        if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
            && let Some(state) = sessions.get_mut(&session)
        {
            let deadline = std::time::Instant::now() + std::time::Duration::from_millis(delay_ms);
            if state.delayed_end.as_ref().is_none_or(|end| {
                end.generation != generation
                    || end.npc_actor != npc_actor
                    || deadline < end.deadline
            }) {
                state.delayed_end = Some(DelayedEnd {
                    npc_actor,
                    generation,
                    deadline,
                });
            }
        }
        return;
    }
    admission.states.chat.end_engagement(session, npc_actor);
    if let Some(conversation) = &mut response.conversation {
        conversation.open = None;
    }
}

fn install(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    next: Option<Conversation>,
) {
    if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
        && let Some(state) = sessions.get_mut(&session)
    {
        state.catalogue = next.map(Pending::Topic);
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
    let CommitStep {
        context,
        previous,
        next,
        request,
        fallback_lines,
    } = match step {
        Step::Reply {
            lines,
            end,
            end_delay_ms,
            npc_actor,
        } => {
            let mut response = response(admission, command.game_session_id, &intent, lines);
            if end {
                end_response(
                    admission,
                    command.game_session_id,
                    npc_actor,
                    end_delay_ms,
                    &mut response,
                );
            }
            return response;
        }
        Step::Refused => return rejected(),
        Step::Held => return unknown(),
        Step::Commit(c) => *c,
    };
    let (Some(item_fence), Some(catalogue)) =
        (command.item_fence, admission.quest_catalogue.as_ref())
    else {
        return rejected();
    };
    // Acquire the physical item lane before the revision cursor and SQL transaction.
    let Some(item_permit) = admission.spell_lane_permit().await else {
        return unknown();
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
        let Ok(states) = admission.states.spell_states.try_lock() else {
            return rejected();
        };
        if states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, command.game_session_id)
            || admission.states.npc_conversations.current_npc(
                &admission.states.chat,
                &runtime,
                actor,
                command.game_session_id,
                &intent,
            ) != Some(context)
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
    {
        let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock() else {
            return rejected();
        };
        let Some(state) = sessions.get_mut(&command.game_session_id) else {
            return rejected();
        };
        if matches!(state.catalogue, Some(Pending::Commit { .. })) {
            return unknown();
        }
        let current = match &state.catalogue {
            Some(Pending::Topic(p)) => Some(p),
            _ => None,
        };
        if current != previous.as_ref() {
            return rejected();
        }
        state.catalogue = Some(Pending::Commit {
            character: fence.character_id,
            request: request.clone(),
            intent: intent.clone(),
            next: next.clone(),
            fallback_lines: fallback_lines.clone(),
        });
    }
    let moves_items = (request.npc == "oteryn:npc.pemaret"
        && request.transition_key.ends_with("/deliver"))
        || request.npc == "oteryn:npc.tereban"
            && (request.transition_key.ends_with("-deliver")
                || request.transition_key.ends_with("/mission-complete"));
    let outcome = slot
        .commit_npc_program_with_premium(
            admission.root,
            admission.character,
            admission.holder,
            &item_permit,
            fence,
            request,
            Arc::clone(catalogue),
            &super::super::item_ref_admission::NativeItemWeights(
                match admission
                    .active_generation
                    .and_then(|generation| generation.native_gameplay())
                {
                    Some(native) => native,
                    None => {
                        install(admission, command.game_session_id, None);
                        return rejected();
                    }
                },
            )
            .with_wheel(admission.premium_coordinator),
            &admission.premium_coordinator,
        )
        .await;
    drop(slot);
    drop(item_permit);
    match outcome {
        Ok(commit)
            if commit.read_only
                || matches!(
                    commit.transition,
                    QuestTransitionOutcome::Committed(_)
                        | QuestTransitionOutcome::AlreadyCommitted(_)
                ) =>
        {
            install(
                admission,
                command.game_session_id,
                (!commit.end_conversation).then_some(Conversation {
                    topic: commit.next_topic,
                    ..next.clone()
                }),
            );
            admission
                .refresh_quest_session(command.game_session_id, false)
                .await;
            let mut response = response(
                admission,
                command.game_session_id,
                &intent,
                if commit.reply.is_empty() {
                    fallback_lines
                } else {
                    commit.reply
                },
            );
            response.inventory_changed =
                !commit.read_only && (commit.inventory_changed || moves_items);
            if commit.end_conversation {
                end_response(
                    admission,
                    command.game_session_id,
                    next.npc_actor,
                    commit.end_delay_ms,
                    &mut response,
                );
            }
            response
        }
        Err(crate::durability::character_progression::CharacterProgressionError::Unavailable(
            _,
        )) => unknown(),
        _ => {
            install(admission, command.game_session_id, None);
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
        .catalogue
        .clone()?;
    let Pending::Commit {
        character,
        request,
        intent,
        next,
        fallback_lines,
    } = pending
    else {
        return None;
    };
    if request.command
        != CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).ok()?,
        )
    {
        return None;
    }
    let moves_items = (request.npc == "oteryn:npc.pemaret"
        && request.transition_key.ends_with("/deliver"))
        || request.npc == "oteryn:npc.tereban"
            && (request.transition_key.ends_with("-deliver")
                || request.transition_key.ends_with("/mission-complete"));
    Some(
        match admission
            .root
            .reconcile_npc_program(admission.character, character, request)
            .await
        {
            Ok(Some(commit)) => {
                install(
                    admission,
                    command.game_session_id,
                    (!commit.end_conversation).then_some(Conversation {
                        topic: commit.next_topic,
                        ..next.clone()
                    }),
                );
                admission
                    .refresh_quest_session(command.game_session_id, false)
                    .await;
                let mut response = response(
                    admission,
                    command.game_session_id,
                    &intent,
                    if commit.reply.is_empty() {
                        fallback_lines
                    } else {
                        commit.reply
                    },
                );
                response.inventory_changed = commit.inventory_changed || moves_items;
                if commit.end_conversation {
                    end_response(
                        admission,
                        command.game_session_id,
                        next.npc_actor,
                        commit.end_delay_ms,
                        &mut response,
                    );
                }
                response
            }
            Ok(None) => {
                install(admission, command.game_session_id, None);
                NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
            }
            Err(_) => NpcTalkOutcome {
                unknown: true,
                ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
            },
        },
    )
}

pub(super) struct DelayedEnd {
    npc_actor: ExactActorRef,
    generation: u64,
    deadline: std::time::Instant,
}

pub(super) fn follow_end(
    conversations: &NpcConversations,
    chat: &ChatRuntime,
    session: GameSessionId,
) {
    let generation = chat.npc_conversation_generation(session);
    let engaged = chat.engaged(session);
    let due = {
        let Ok(mut sessions) = conversations.sessions.lock() else {
            return;
        };
        let Some(state) = sessions.get_mut(&session) else {
            return;
        };
        let Some(end) = &state.delayed_end else {
            return;
        };
        if generation != Some(end.generation) || engaged != Some(end.npc_actor) {
            state.delayed_end = None;
            return;
        }
        if std::time::Instant::now() < end.deadline {
            return;
        }
        state.delayed_end.take()
    };
    if let Some(end) = due
        && chat.npc_conversation_generation(session) == Some(end.generation)
    {
        chat.end_engagement(session, end.npc_actor);
    }
}
