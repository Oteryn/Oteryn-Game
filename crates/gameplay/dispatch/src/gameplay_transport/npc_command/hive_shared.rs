//! Orockle's full-objective turn-in selects existing fenced QuestState transitions.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::quest_dialogue::{self, Choice, Step};
use super::*;
use oteryn_gameplay_quests::quest::shared_world_progress::{BINDINGS, SharedAction};
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
            if context.2 != "oteryn:npc.orockle" {
                return None;
            }
            let text = intent.text.trim().to_ascii_lowercase();
            if matches!(
                text.as_str(),
                "pheromone" | "pheromones" | "pheronome" | "permit"
            ) {
                let enough = tracks
                    .get(oteryn_gameplay_quests::quest::hive_access::FAVOUR)
                    .copied()
                    .unwrap_or(0)
                    >= 50;
                return Some(Choice {transition:enough.then(||oteryn_gameplay_quests::quest::hive_access::BUY.into()),next_topic:None,
                    reply:vec![if enough {"For fifty Favour Points, your scent will let you pass the open Hive gates for seven days."} else {"Complete missions to earn fifty Favour Points before buying the seven-day scent."}.into()]});
            }
            let labels = [
                "antenna",
                "digestion",
                "chitin",
                "pores",
                "kollos",
                "monolith",
            ];
            if let Some(index) = labels.iter().position(|label| text == *label) {
                // The durable owner distinguishes an active mission in this
                // cycle from stale progress that must restart in a new cycle.
                return Some(Choice{transition:Some(oteryn_gameplay_quests::quest::shared_world_progress::accept_key(index)),next_topic:None,reply:vec!["Mission accepted. Ask for trade to buy its reusable tools. Report after completing every objective; dissolved chitin requires eight crawler collections and eight swarmer-drum gatherings.".into()]});
            }
            if !matches!(text.as_str(), "mission" | "report" | "completed" | "yes") {
                return None;
            }
            let completed = BINDINGS.iter().find(|binding| {
                matches!(binding.action, SharedAction::HiveTask(task) if tracks.get(oteryn_gameplay_quests::quest::shared_world_progress::ACTIVE_TRACKS[usize::from(task)]).copied().unwrap_or(0)==1)
                    && tracks.get(binding.track).copied().unwrap_or(0) == binding.required
                    && (binding.action!=SharedAction::HiveTask(2) || tracks.get(oteryn_gameplay_quests::quest::shared_world_progress::HATCHLINGS_TRACK).copied().unwrap_or(0)==8)
            });
            Some(Choice{transition:completed.map(|binding|binding.key.into()),next_topic:None,reply:vec![if completed.is_some(){"Your completed mission has weakened the Hive."}else{"Complete one mission before reporting: blind eight antennae, disrupt seven blobs, collect eight dissolved chitin, stuff seven pores, kill nine Kollos, or destroy five monoliths."}.into()]})
        },
    )
}
