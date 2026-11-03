//! Spike officers: source dialogue and existing QuestState/XP obligations only.
//! Item issuance and target progress stay with their separately qualified owners.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::quest_dialogue::{self, Choice, Step, Topic};
use super::*;
use oteryn_gameplay_quests::quest::spike_tasks as quest;

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> Option<Step> {
    let fence = command.item_fence?;
    if admission.states.level_refresh.is_marked(fence.character_id) {
        // The live level owner retries its existing durable-fact projection.
        // A pending refresh cannot turn a source level gate into an old value.
        return None;
    }
    let level = {
        let states = admission.states.spell_states.try_lock().ok()?;
        if states.is_dead(actor) {
            return None;
        }
        states
            .get(runtime, actor, command.game_session_id)?
            .character_facts()
            .level
    };
    let now = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs(),
    )
    .ok()?;
    let book = quest::dialogue::load().ok()?;
    quest_dialogue::prepare(
        admission,
        actor,
        command,
        intent,
        runtime,
        |context, topic, _, tracks| {
            if topic.number != 0 {
                return None;
            }
            let facts = quest::dialogue::Facts { tracks, level, now };
            let selected = quest::storage_only::select(
                &book,
                &context.2,
                topic.value.as_deref(),
                &intent.text,
                &facts,
            )?;
            Some(Choice {
                transition: selected.transition,
                next_topic: Some(Topic {
                    number: 0,
                    value: selected.plan.next_topic,
                }),
                reply: selected.plan.reply,
            })
        },
    )
}
