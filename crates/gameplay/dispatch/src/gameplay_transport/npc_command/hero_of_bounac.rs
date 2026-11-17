//! The real Kesar conversation reaches the existing fenced, replayable quest owner.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use oteryn_gameplay_quests::quest::hero_of_bounac as quest;

pub(super) enum Step {
    Refused,
    Dialogue(quest_dialogue::Step),
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
    if context.2 != quest::NPC || !quest::is_report(&intent.text) {
        return None;
    }
    let available = admission
        .states
        .quest_sessions
        .lock()
        .ok()
        .is_some_and(|sessions| {
            sessions
                .get(&command.game_session_id)
                .is_some_and(|session| session.copy.is_some())
        });
    if !available {
        return Some(Step::Refused);
    }
    quest_dialogue::prepare(
        admission,
        actor,
        command,
        intent,
        runtime,
        |_, _, _, tracks| {
            if tracks.get(quest::MISSION).copied().unwrap_or(0) < 5 {
                return None;
            }
            Some(quest_dialogue::Choice {
                transition: Some(quest::COMPLETE.into()),
                next_topic: None,
                reply: quest::reply(),
            })
        },
    )
    .map(Step::Dialogue)
}

pub(super) async fn apply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTalkIntent,
    step: Step,
) -> NpcTalkOutcome {
    match step {
        Step::Refused => NpcTalkOutcome::only(NpcIntentDisposition::Rejected),
        Step::Dialogue(step) => {
            quest_dialogue::apply(admission, actor, command, intent, step).await
        }
    }
}
