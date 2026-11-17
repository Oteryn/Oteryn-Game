//! NPC greeting in a `say` (CHAT-0 §3).
//!
//! A `say` that contains, as whole words, a greeting of an NPC within that NPC's talk range also
//! starts that NPC's conversation, as the NPC talk command would. With several NPCs greeted, the
//! nearest wins, then the lowest actor id. The runtime applies this only when the speaker's
//! client has the NPC talk capability; the line is shown to every listener either way.

use crate::foundation::speech::ChatPosition;

/// `CHAT0-RL-08`: an NPC hears a greeting within 4 tiles until NPC-0 sets its own range.
pub const DEFAULT_TALK_RANGE: u32 = 4;

/// An NPC on the speaker's Channel that can be greeted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GreetingNpc<'a> {
    pub actor_id: u64,
    pub position: ChatPosition,
    /// The NPC's greetings from its content, for example `hi` or `hail king`.
    pub greetings: &'a [String],
    pub talk_range: u32,
}

/// The actor id of the NPC that `text`, said at `speaker`, greets; `None` when it greets none.
pub fn greeted_npc(speaker: ChatPosition, text: &str, npcs: &[GreetingNpc<'_>]) -> Option<u64> {
    let said = words(text);
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
pub fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Whether `needle` occurs in `haystack` as consecutive words.
pub fn contains_words(haystack: &[String], needle: &[String]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

pub const BAA_LEAL: &str = "oteryn:npc.baa_leal";

/// The Efreet transcript specifies 150 delayed fire ticks over twelve and a half minutes.
pub fn soulfire_on_greeting(
    npc: &str,
    text: &str,
    passwords: &[String],
) -> Option<(u32, u32, u32)> {
    if npc != BAA_LEAL || passwords.is_empty() {
        return None;
    }
    let said = words(text);
    (said
        .iter()
        .any(|word| matches!(word.as_str(), "hi" | "hello" | "hail"))
        && !passwords
            .iter()
            .any(|password| contains_words(&said, &words(password))))
    .then_some((1_500, 10, 5_000))
}

/// The generated first-greeting transition for the Orc King.
pub const ORC_KING_GREETING_TRANSITION: &str = "oteryn:quest-reward/l2-efreet-orc-king-greeting";

/// Crystal's Orc King greets each character with this wave once.
pub fn orc_king_guards(transition: &str, newly_committed: bool) -> &'static [&'static str] {
    if !newly_committed || transition != ORC_KING_GREETING_TRANSITION {
        return &[];
    }
    &[
        "oteryn:creature.slime",
        "oteryn:creature.slime",
        "oteryn:creature.slime",
        "oteryn:creature.orc_warlord",
        "oteryn:creature.orc_warlord",
        "oteryn:creature.orc_leader",
        "oteryn:creature.orc_leader",
        "oteryn:creature.orc_leader",
    ]
}

#[cfg(test)]
mod efreet_tests {
    use super::*;

    #[test]
    fn orc_king_creates_the_source_guard_wave_on_first_greeting() {
        let greeting = "oteryn:quest-reward/l2-efreet-orc-king-greeting";
        assert_eq!(
            orc_king_guards(greeting, true),
            &[
                "oteryn:creature.slime",
                "oteryn:creature.slime",
                "oteryn:creature.slime",
                "oteryn:creature.orc_warlord",
                "oteryn:creature.orc_warlord",
                "oteryn:creature.orc_leader",
                "oteryn:creature.orc_leader",
                "oteryn:creature.orc_leader",
            ]
        );
        assert!(orc_king_guards(greeting, false).is_empty());
        assert!(orc_king_guards("oteryn:quest-reward/l2-efreet-lamp-issue", true).is_empty());
    }

    #[test]
    fn baa_leal_punishes_only_the_wrong_greeting() {
        let npc = BAA_LEAL;
        let passwords = ["djanni'hah".to_owned()];
        assert_eq!(
            soulfire_on_greeting(npc, "hello", &passwords),
            Some((1_500, 10, 5_000))
        );
        assert_eq!(
            soulfire_on_greeting(npc, "Hi, DJANNI'HAH", &passwords),
            None
        );
        assert_eq!(soulfire_on_greeting(npc, "mission", &passwords), None);
        assert_eq!(
            soulfire_on_greeting("oteryn:npc.alesar", "hi", &passwords),
            None
        );
    }
}
