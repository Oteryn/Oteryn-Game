//! Source-scoped Isle dialogue selection; the shared QuestState caller owns execution.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::quest_dialogue::{self, Choice, Step, Topic};
use super::*;
use oteryn_gameplay_quests::quest::isle_of_evil as quest;
use oteryn_gameplay_quests::quest::npc_rules::NpcRuleCatalog;

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> Option<Step> {
    let book = quest::load().ok()?;
    quest_dialogue::prepare(
        admission,
        actor,
        command,
        intent,
        runtime,
        |context, topic, _fresh, tracks| {
            let decision = book.respond(&context.2, &intent.text, topic.number, tracks)?;
            if decision.held {
                return None;
            }
            // Crystal rapanaio.lua:90-100,206-235. Keep the prompt and its
            // confirmation on the same topic owner once the intake is authored.
            if context.2 == "oteryn:npc.rapanaio"
                && matches!(decision.next_topic, Some(10 | 11))
                && admission.quest_catalogue.as_ref().is_some_and(|catalogue| {
                    NpcRuleCatalog::for_state(catalogue)
                        .ok()
                        .and_then(|source| source.admitted_for_npc(&context.2, catalogue).ok()?)
                        .is_some_and(|source| source.item_exchange_keys(&context.2).is_some())
                })
            {
                return None;
            }
            Some(Choice {
                transition: decision.transition,
                next_topic: decision.next_topic.map(Topic::from),
                reply: decision.lines,
            })
        },
    )
}
