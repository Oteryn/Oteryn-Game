//! Lynda's current wedding subgraph takes precedence only while it owns this conversation.
use super::*;
use crate::durability::lynda_wedding::{Action, Request};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::UseCommand;
#[derive(Clone)]
pub(super) struct State {
    context: (u64, u64, String),
    actor: ExactActorRef,
    npc: EntityRef,
    scope: crate::foundation::RuntimeScopeRefV1,
    generation: u64,
    stage: u8,
}
pub(super) struct Step {
    state: State,
    action: Choice,
}
enum Choice {
    Start,
    AskName,
    Propose(String),
    AskCancel,
    Cancel,
    No,
}
fn word(text: &str, key: &str) -> bool {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .any(|part| part.eq_ignore_ascii_case(key))
}
fn clear(admission: &ComposedFreshAdmission<'_, '_, '_>, session: GameSessionId) {
    if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
        && let Some(s) = sessions.get_mut(&session)
    {
        s.lynda_wedding = None
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
    if context.2 != "oteryn:npc.lynda" {
        return None;
    }
    let binding = runtime.binding();
    let prior = admission
        .states
        .npc_conversations
        .sessions
        .lock()
        .ok()?
        .get(&command.game_session_id)?
        .lynda_wedding
        .clone();
    let mut state = prior
        .filter(|s| {
            s.context == context
                && s.actor == actor
                && s.npc == intent.npc_actor
                && s.scope == binding.runtime_scope()
                && s.generation == binding.scope_generation().get()
        })
        .unwrap_or(State {
            context,
            actor,
            npc: intent.npc_actor,
            scope: binding.runtime_scope(),
            generation: binding.scope_generation().get(),
            stage: 0,
        });
    let text = intent.text.trim();
    if ["bye", "farewell", "hi"].iter().any(|key| word(text, key)) {
        clear(admission, command.game_session_id);
        return None;
    }

    let action = if word(text, "marry") {
        // Starting a fresh named graph clears only this NPC's prior generic Mage
        // topic. A stale Mage yes can never become reciprocal wedding consent.
        if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
            && let Some(s) = sessions.get_mut(&command.game_session_id)
        {
            s.cosmetics = None;
        }
        Choice::Start
    } else if word(text, "remove") {
        if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
            && let Some(s) = sessions.get_mut(&command.game_session_id)
        {
            s.cosmetics = None;
        }
        Choice::AskCancel
    } else if [
        "outfit", "rod", "wand", "sulphur", "soul", "ankh", "ritual", "angelina",
    ]
    .iter()
    .any(|key| word(text, key))
    {
        clear(admission, command.game_session_id);
        return None;
    } else {
        match state.stage {
            1 if word(text, "yes") => Choice::AskName,
            1 if word(text, "no") => Choice::No,
            2 => Choice::Propose(text.into()),
            3 if word(text, "yes") => Choice::Cancel,
            3 if word(text, "no") => Choice::No,
            _ => return None,
        }
    };
    state.stage = match action {
        Choice::Start => 1,
        Choice::AskName => 2,
        Choice::AskCancel => 3,
        _ => 0,
    };
    Some(Step { state, action })
}
fn install(admission: &ComposedFreshAdmission<'_, '_, '_>, session: GameSessionId, state: State) {
    if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
        && let Some(s) = sessions.get_mut(&session)
    {
        s.lynda_wedding = (state.stage > 0).then_some(state)
    }
}
fn reply(intent: &NpcTalkIntent, text: String, changed: bool) -> NpcTalkOutcome {
    NpcTalkOutcome {
        unknown: false,
        inventory_changed: changed,
        disposition: NpcIntentDisposition::Ok,
        conversation: Some(NpcConversation {
            open: Some((intent.npc_actor, NpcConversationState::Open)),
            lines: vec![text],
        }),
        window: None,
    }
}
pub(super) async fn apply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTalkIntent,
    step: Step,
) -> NpcTalkOutcome {
    let unknown = || NpcTalkOutcome {
        unknown: true,
        ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
    };
    let Some(item) = command.item_fence else {
        return NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    };
    let Ok(Some(fence)) = admission.current_quest_fence(command.game_session_id).await else {
        return unknown();
    };
    if item != fence.item_fence() {
        return NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    }
    {
        let runtime = admission.runtime.lock().await;
        let Ok(spells) = admission.states.spell_states.try_lock() else {
            return unknown();
        };
        if spells.is_dead(actor)
            || !spells.spell_idle(&runtime, actor, command.game_session_id)
            || !runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|p| p.control_loss.is_none())
            || runtime.binding().runtime_scope() != step.state.scope
            || runtime.binding().scope_generation().get() != step.state.generation
            || admission.states.npc_conversations.current_npc(
                &admission.states.chat,
                &runtime,
                actor,
                command.game_session_id,
                &intent,
            ) != Some(step.state.context.clone())
        {
            return NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
        }
    }
    let mut state = step.state;
    let action = match step.action {
        Choice::Start => {
            install(admission, command.game_session_id, state);
            return reply(&intent,"Would you like to get married? Make sure you have a wedding ring and the wedding outfit box with you.".into(),false);
        }
        Choice::AskName => {
            install(admission, command.game_session_id, state);
            return reply(&intent, "And who would you like to marry?".into(), false);
        }
        Choice::No => {
            clear(admission, command.game_session_id);
            return reply(&intent, "Ok, let's keep it then.".into(), false);
        }
        Choice::AskCancel => match admission
            .root
            .read_lynda_pending_name(admission.character, fence.character_id)
            .await
        {
            Ok(Some(name)) => {
                install(admission, command.game_session_id, state);
                return reply(
                    &intent,
                    format!(
                        "Are you sure you want to remove your wedding proposal with {{{name}}}?"
                    ),
                    false,
                );
            }
            Ok(None) => {
                clear(admission, command.game_session_id);
                return reply(
                    &intent,
                    "You don't have any pending proposal to be removed.".into(),
                    false,
                );
            }
            Err(_) => return unknown(),
        },
        Choice::Propose(name) => Action::Propose(name),
        Choice::Cancel => Action::Cancel,
    };
    let Some(permit) = admission.spell_lane_permit().await else {
        return unknown();
    };
    let mut slot = admission
        .revision_sequencer
        .acquire(item.character_id)
        .await;
    let Some(native) = admission
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
    else {
        return unknown();
    };
    let Ok(id) = crate::foundation::CommandId::new(command.command_id) else {
        return NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    };
    let outcome = admission
        .root
        .commit_lynda_wedding(
            admission.character,
            admission.holder,
            fence,
            Request {
                command: crate::foundation::CommandRef::new(command.game_session_id, id),
                action,
            },
            &permit,
            native,
        )
        .await;
    slot.observe_endgame_progression();
    drop(slot);
    let Ok(outcome) = outcome else {
        return unknown();
    };
    if outcome.inventory_changed
        && !super::npc_cosmetics::refresh_equipment(admission, actor, command.game_session_id).await
    {
        return unknown();
    }
    admission
        .refresh_quest_session(command.game_session_id, false)
        .await;
    state.stage = 0;
    install(admission, command.game_session_id, state);
    reply(&intent, outcome.reply, outcome.inventory_changed)
}
