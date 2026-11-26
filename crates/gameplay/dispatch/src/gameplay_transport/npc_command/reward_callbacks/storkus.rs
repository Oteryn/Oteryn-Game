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
    if focus_intercepts(text) {
        return None;
    }
    ["mission", "vampire lord token", "yes"]
        .into_iter()
        .find(|word| {
            classic_talk::contains(text, word)
                && (*word != "vampire lord token"
                    || track(admission, command, QUESTLINE).is_some_and(|value| value >= 11))
        })
}

// Crystal's FocusModule handles these before CALLBACK_MESSAGE_DEFAULT.
// modules.lua:379-387 rejects a focus word if any occurrence touches [%w+].
fn focus_intercepts(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    let bytes = text.as_bytes();
    let touches_word = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'+';
    ["hi", "hello", "bye", "farewell", "trade", "offers"]
        .into_iter()
        .any(|word| {
            let mut found = false;
            for (start, _) in text.match_indices(word) {
                let end = start + word.len();
                if (start > 0 && touches_word(bytes[start - 1]))
                    || (end < bytes.len() && touches_word(bytes[end]))
                {
                    return false;
                }
                found = true;
            }
            found
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
