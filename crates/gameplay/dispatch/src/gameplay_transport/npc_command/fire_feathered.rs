//! Guardian is the retained native NPC, never an invented boss.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::quest_dialogue::{self, Choice, Step};
use super::*;
use oteryn_gameplay_quests::quest::fire_feathered_native as quest;
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
            if context.2 != "oteryn:npc.guardian_of_the_deep_seas" {
                return None;
            }
            let text = intent.text.trim().to_ascii_lowercase();
            if !matches!(
                text.as_str(),
                "mission" | "orbs" | "energies" | "completed" | "yes"
            ) {
                return None;
            }
            let returned = quest::TRACKS
                .iter()
                .filter(|track| tracks.get(**track) == Some(&1))
                .count();
            // Explicit Fire-Feathered help is available before any orb is returned.
            if returned == 0 && !matches!(text.as_str(), "orbs" | "energies") {
                return None;
            }
            let complete = returned == quest::TRACKS.len();
            let reply = if complete {
                "You have returned the three orb energies. The Fire-Feathered Sea Serpent's task is complete."
            } else {
                "Bring the darkness, life and light energies. The darkness orb needs one mind stone, the life orb one seacrest pearl, and the light orb a bottle of gloo. You keep the bottle."
            };
            Some(Choice {
                transition: complete.then(|| quest::COMPLETE.into()),
                next_topic: None,
                reply: vec![reply.into()],
            })
        },
    )
}
