//! NPC greeting in a `say` (CHAT-0 §3).
//!
//! A `say` that contains, as whole words, a greeting of an NPC within that NPC's talk range also
//! starts that NPC's conversation, as the NPC talk command would. With several NPCs greeted, the
//! nearest wins, then the lowest actor id. The runtime applies this only when the speaker's
//! client has the NPC talk capability; the line is shown to every listener either way.

use super::{ChatPosition, ChatText};

/// `CHAT0-RL-08`: an NPC hears a greeting within 4 tiles until NPC-0 sets its own range.
pub(crate) const DEFAULT_TALK_RANGE: u32 = 4;

/// An NPC on the speaker's Channel that can be greeted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GreetingNpc<'a> {
    pub(crate) actor_id: u64,
    pub(crate) position: ChatPosition,
    /// The NPC's greetings from its content, for example `hi` or `hail king`.
    pub(crate) greetings: &'a [String],
    pub(crate) talk_range: u32,
}

/// The actor id of the NPC that `text`, said at `speaker`, greets; `None` when it greets none.
pub(crate) fn greeted_npc(
    speaker: ChatPosition,
    text: &ChatText,
    npcs: &[GreetingNpc<'_>],
) -> Option<u64> {
    let said = words(text.as_str());
    npcs.iter()
        .filter_map(|npc| {
            let distance = speaker.distance(npc.position)?;
            (distance <= npc.talk_range
                && npc
                    .greetings
                    .iter()
                    .any(|greeting| contains_words(&said, &words(greeting))))
            .then_some((distance, npc.actor_id))
        })
        .min()
        .map(|(_, actor_id)| actor_id)
}

/// Lower-cased words: runs of letters and digits.
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Whether `needle` occurs in `haystack` as consecutive words.
fn contains_words(haystack: &[String], needle: &[String]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}
