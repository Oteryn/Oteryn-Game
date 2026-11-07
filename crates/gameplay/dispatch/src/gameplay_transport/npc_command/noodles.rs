//! Donor King's `noodles` return uses the same fenced world action as collar use.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::quest_dialogue::{self, Choice, Step};
use super::*;
use oteryn_gameplay_quests::quest::noodles as quest;

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> Option<Step> {
    quest_dialogue::prepare(
        admission,
        actor,
        command,
        intent,
        runtime,
        |context, _, _, tracks| {
            quest::returns_to_king(
                &context.2,
                &intent.text,
                tracks.get(quest::ESCORT_TRACK).copied().unwrap_or(-1),
            )
            .then(|| Choice {
                transition: Some(quest::RETURN.into()),
                next_topic: Some(0.into()),
                reply: vec!["You brought Noodles back! Thank you for finding him.".into()],
            })
        },
    )
}
