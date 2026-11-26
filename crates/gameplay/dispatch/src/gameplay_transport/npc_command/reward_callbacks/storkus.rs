//! Storkus's default callback checks mission, unlocked tokens, then yes.
use super::*;
use oteryn_gameplay_quests::quest::classic_talk;

pub(super) const NPC: &str = "oteryn:npc.storkus";
pub(super) const QUESTLINE: &str =
    "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/questline";
pub(super) const DUST: &str =
    "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/storkus_vampiredust";
const DUST_ITEM: &str = "oteryn:item.tibia.i5905";
pub(super) const COLLECTION_TOPIC: u16 = 1;

pub(super) fn track(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
    key: &str,
) -> Option<i64> {
    admission
        .states
        .quest_sessions
        .lock()
        .ok()?
        .get(&command.game_session_id)?
        .copy
        .as_ref()?
        .tracks()
        .get(key)
        .copied()
        .or_else(|| {
            admission
                .quest_catalogue
                .as_deref()?
                .track(key)
                .map(|track| track.initial)
        })
}

pub(super) fn keyword(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
    text: &str,
) -> Option<&'static str> {
    ["mission", "vampire lord token", "yes"]
        .into_iter()
        .find(|word| {
            classic_talk::contains(text, word)
                && (*word != "vampire lord token"
                    || track(admission, command, QUESTLINE).is_some_and(|value| value >= 11))
        })
}

pub(super) fn collection_prompt(
    lines: &[String],
    facts: &super::super::reward_facts::RewardFacts,
) -> Vec<String> {
    let count = facts
        .quantities
        .get(DUST_ITEM)
        .copied()
        .unwrap_or(0)
        .to_string();
    lines
        .iter()
        .map(|line| line.replace("{carried_dust}", &count))
        .collect()
}
