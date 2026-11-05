//! Captured source programs share the current NPC conversation and QuestState writer.
//! Literal text is content; a source reply never acts as evidence that its mutation committed.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use crate::domain::CharacterId;
use crate::durability::npc_quest_program::NpcProgramContext;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::npc_rules::{
    NpcInventorySnapshot, NpcRuleCatalog, NpcRuleInput, NpcRuleTrigger,
};
use std::collections::BTreeMap;
use std::sync::OnceLock;

type Context = (u64, u64, String);

pub(super) type State = Option<Pending>;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Conversation {
    context: Context,
    actor: ExactActorRef,
    npc: EntityRef,
    topic: i32,
}

#[derive(Clone)]
pub(super) enum Pending {
    Topic(Conversation),
    Commit {
        character: CharacterId,
        request: QuestTransitionRequest,
        source: NpcProgramContext,
        intent: NpcTalkIntent,
        reply: Vec<String>,
        previous: Option<Conversation>,
        next: Option<Conversation>,
    },
}

pub(super) struct CommitStep {
    context: Context,
    previous: Option<Conversation>,
    next: Option<Conversation>,
    transition: String,
    source: NpcProgramContext,
    reply: Vec<String>,
}

pub(super) enum Step {
    Reply {
        context: Context,
        lines: Vec<String>,
    },
    Refused,
    Commit(Box<CommitStep>),
    Held,
}

fn source_catalogue() -> Option<&'static NpcRuleCatalog> {
    static CATALOGUE: OnceLock<Result<NpcRuleCatalog, String>> = OnceLock::new();
    CATALOGUE
        .get_or_init(|| NpcRuleCatalog::load_embedded().map_err(|error| error.to_string()))
        .as_ref()
        .ok()
}

/// These captured closing callbacks have only a topic reset. The ordinary NPC
/// close owner ends the generation, which also invalidates this adapter's topic.
pub(in crate::gameplay_transport) fn closing_reply(
    npc: &str,
    trigger: NpcRuleTrigger,
) -> Option<Vec<String>> {
    if !matches!(trigger, NpcRuleTrigger::Farewell | NpcRuleTrigger::Walkaway) {
        return None;
    }
    let tracks = BTreeMap::new();
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    let plan = source_catalogue()?
        .plan_dialogue(&NpcRuleInput {
            npc,
            trigger,
            message: "",
            topic: 0,
            tracks: &tracks,
            inventory: &inventory,
            now_epoch_seconds: 0,
            calendar_month: 0,
            level: 0,
            vocation: "",
            sex: 0,
            outfits: &outfits,
        })
        .ok()??;
    (plan.next_topic == 0).then_some(plan.reply)
}

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
    ordinary: &NpcTalkOutcome,
) -> Option<Step> {
    let context = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        runtime,
        actor,
        command.game_session_id,
        intent,
    )?;
    let Some(catalogue) = source_catalogue() else {
        return Some(Step::Refused);
    };
    if !catalogue.programs().iter().any(|program| {
        program
            .document
            .rules
            .iter()
            .any(|rule| rule.npc == context.2)
    }) {
        return None;
    }
    let mut tracks = {
        let sessions = admission.states.quest_sessions.lock().ok()?;
        sessions
            .get(&command.game_session_id)?
            .copy
            .as_ref()?
            .tracks()
            .clone()
    };
    if let Some(state_catalogue) = admission.quest_catalogue.as_ref() {
        for program in catalogue.programs() {
            if program
                .document
                .rules
                .iter()
                .any(|rule| rule.npc == context.2)
            {
                for rule in &program.document.rules {
                    for guard in &rule.guards {
                        if let oteryn_gameplay_quests::quest::npc_rules::NpcRuleGuard::Storage {
                            track,
                            ..
                        }
                        | oteryn_gameplay_quests::quest::npc_rules::NpcRuleGuard::StorageTime {
                            track,
                            ..
                        } = guard
                            && !program.document.tracks.iter().any(|own| own.key == *track)
                            && let Some(definition) = state_catalogue.track(track)
                        {
                            tracks.entry(track.clone()).or_insert(definition.initial);
                        }
                    }
                }
            }
        }
    }
    let mut sessions = admission.states.npc_conversations.sessions.lock().ok()?;
    let state = sessions.get_mut(&command.game_session_id)?;
    if matches!(state.programs.as_ref(), Some(Pending::Commit { .. })) {
        return Some(Step::Held);
    }
    let previous = match state.programs.as_ref() {
        Some(Pending::Topic(previous))
            if previous.context == context
                && previous.actor == actor
                && previous.npc == intent.npc_actor =>
        {
            Some(previous.clone())
        }
        _ => None,
    };
    if previous.is_none() {
        state.programs = None;
    }
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())?;
    let greeting = previous.is_none();
    let mut input = NpcRuleInput {
        npc: &context.2,
        trigger: if greeting {
            NpcRuleTrigger::Greet
        } else {
            NpcRuleTrigger::Say
        },
        message: &intent.text,
        topic: previous.as_ref().map_or(0, |p| p.topic),
        tracks: &tracks,
        inventory: &inventory,
        now_epoch_seconds: now,
        calendar_month: 0,
        level: 0,
        vocation: "",
        sex: 0,
        outfits: &outfits,
    };
    let mut planned = catalogue.plan_storage(&input);
    if greeting && matches!(planned, Ok(None)) {
        input.trigger = NpcRuleTrigger::Say;
        planned = catalogue.plan_storage(&input);
    }
    let plan = match planned {
        Ok(Some(plan)) => plan,
        Ok(None) => {
            state.programs = Some(Pending::Topic(Conversation {
                context,
                actor,
                npc: intent.npc_actor,
                topic: previous
                    .as_ref()
                    .map_or(0, |conversation| conversation.topic),
            }));
            return None;
        }
        Err(_) => return Some(Step::Refused),
    };
    if !oteryn_gameplay_quests::quest::in_runtime_scope(&plan.quest) {
        return Some(Step::Refused);
    }
    let reply = if input.trigger == NpcRuleTrigger::Greet && plan.reply.is_empty() {
        ordinary
            .conversation
            .as_ref()
            .map(|conversation| conversation.lines.clone())
            .unwrap_or_default()
    } else {
        plan.reply
    };
    if response(
        admission,
        command.game_session_id,
        &context,
        intent,
        reply.clone(),
    )
    .disposition
        != NpcIntentDisposition::Ok
    {
        return Some(Step::Refused);
    }
    let next = Some(Conversation {
        context: context.clone(),
        actor,
        npc: intent.npc_actor,
        topic: plan.next_topic,
    });
    if plan.track_changes.is_empty() {
        state.programs = next.map(Pending::Topic);
        Some(Step::Reply {
            context,
            lines: reply,
        })
    } else {
        let source = NpcProgramContext {
            quest: plan.quest,
            program_digest: plan.program_digest,
            npc: context.2.clone(),
            message: intent.text.clone(),
            topic: input.topic,
            trigger: input.trigger,
            selected_rule_keys: plan.selected_rule_keys.clone(),
        };
        Some(Step::Commit(Box::new(CommitStep {
            context,
            previous,
            next,
            transition: oteryn_gameplay_quests::quest::npc_rules::program_transition_key(
                plan.program_digest,
            ),
            source,
            reply,
        })))
    }
}

fn response(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    context: &Context,
    intent: &NpcTalkIntent,
    lines: Vec<String>,
) -> NpcTalkOutcome {
    quest_dialogue::reply(&admission.states.chat, session, context, intent, lines)
}

async fn publish_committed(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
    actor: ExactActorRef,
    context: &Context,
    intent: &NpcTalkIntent,
    reply: Vec<String>,
    next: Option<Conversation>,
) -> NpcTalkOutcome {
    let runtime = admission.runtime.lock().await;
    let current = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        &runtime,
        actor,
        command.game_session_id,
        intent,
    );
    if current.as_ref() != Some(context) {
        if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
            && let Some(state) = sessions.get_mut(&command.game_session_id)
            && matches!(state.programs.as_ref(), Some(Pending::Commit { next: Some(next), .. }) if &next.context == context)
        {
            state.programs = None;
        }
        return NpcTalkOutcome::only(NpcIntentDisposition::Ok);
    }
    install_topic(admission, command.game_session_id, next);
    response(admission, command.game_session_id, context, intent, reply)
}

fn install_topic(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    next: Option<Conversation>,
) {
    if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
        && let Some(state) = sessions.get_mut(&session)
    {
        state.programs = next.map(Pending::Topic);
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
        transition,
        source,
        reply,
    } = match step {
        Step::Reply { context, lines } => {
            return response(admission, command.game_session_id, &context, &intent, lines);
        }
        Step::Refused => return rejected(),
        Step::Held => return unknown(),
        Step::Commit(commit) => *commit,
    };
    if response(
        admission,
        command.game_session_id,
        &context,
        &intent,
        reply.clone(),
    )
    .disposition
        != NpcIntentDisposition::Ok
    {
        return rejected();
    }
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
            ) != Some(context.clone())
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
        let Some(state) = sessions.get_mut(&command.game_session_id) else {
            return rejected();
        };
        if matches!(state.programs.as_ref(), Some(Pending::Commit { .. })) {
            return unknown();
        }
        let current = match state.programs.as_ref() {
            Some(Pending::Topic(topic)) => Some(topic),
            _ => None,
        };
        if current != previous.as_ref() {
            return rejected();
        }
        state.programs = Some(Pending::Commit {
            character: fence.character_id,
            request: request.clone(),
            source: source.clone(),
            intent: intent.clone(),
            reply: reply.clone(),
            previous: previous.clone(),
            next: next.clone(),
        });
    }
    let outcome = slot
        .commit_npc_program(
            admission.root,
            admission.character,
            admission.holder,
            fence,
            request,
            std::sync::Arc::clone(catalogue),
            source,
        )
        .await;
    drop(slot);
    match outcome {
        Ok(QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)) => {
            admission
                .refresh_quest_session(command.game_session_id, false)
                .await;
            publish_committed(admission, command, actor, &context, &intent, reply, next).await
        }
        Err(crate::durability::character_progression::CharacterProgressionError::Unavailable(
            _,
        )) => unknown(),
        _ => {
            install_topic(admission, command.game_session_id, previous);
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
        .programs
        .clone()?;
    let Pending::Commit {
        character,
        request,
        source,
        intent,
        reply,
        previous,
        next,
    } = pending
    else {
        return None;
    };
    let id = CommandId::new(command.command_id).ok()?;
    if request.cause != QuestCause::Command(CommandRef::new(command.game_session_id, id)) {
        return None;
    }
    match admission
        .root
        .reconcile_character_npc_program(admission.character, character, request, source)
        .await
    {
        Ok(Some(_)) => {
            let conversation = next.as_ref()?;
            let context = conversation.context.clone();
            let actor = conversation.actor;
            admission
                .refresh_quest_session(command.game_session_id, false)
                .await;
            Some(publish_committed(admission, command, actor, &context, &intent, reply, next).await)
        }
        Ok(None) => {
            install_topic(admission, command.game_session_id, previous);
            Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected))
        }
        Err(_) => Some(NpcTalkOutcome {
            unknown: true,
            ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
        }),
    }
}
