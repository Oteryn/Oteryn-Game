//! NPC talk over the chat path; validated chat text enters the NPC runtime here.

use crate::chat::{ChatPosition, ChatText};
use crate::world_clock::{DEFAULT_WORLD_CLOCK_EPOCH_UTC_MS, WorldClockSource, now_utc_ms};
#[allow(unused_imports, reason = "preserve the former NPC module path")]
pub use oteryn_gameplay_npc::npc_talk::{NpcReply, NpcTalkBook, NpcTalkEntry, TalkNpc};

pub fn asks_to_trade(text: &ChatText) -> bool {
    oteryn_gameplay_npc::npc_talk::asks_to_trade(text.as_str())
}

pub(super) fn hireling_bank_entry() -> NpcTalkEntry {
    NpcTalkEntry::new("Hireling", None).with_bank()
}

/// Crystal 201189f8 npc.lua:89-95 substitutes the current World time in NPC replies.
pub(super) fn render_dialogue(text: &str, player_name: &str) -> String {
    let clock = WorldClockSource::new(DEFAULT_WORLD_CLOCK_EPOCH_UTC_MS);
    let time_rendered = if text.contains("|TIME|")
        && let Some(minute) = now_utc_ms().and_then(|now| clock.tibian_minute(now))
    {
        let time = format!("{:02}:{:02}", minute / 60, minute % 60);
        Some(text.replace("|TIME|", &time))
    } else {
        None
    };
    oteryn_gameplay_npc::quest_greetings::render_player_name(
        time_rendered.as_deref().unwrap_or(text),
        player_name,
    )
}

pub fn talk<K: Copy + PartialEq>(
    speaker: ChatPosition,
    text: &ChatText,
    npcs: &[TalkNpc<'_, K>],
    engaged: &mut Option<K>,
) -> Option<NpcReply<K>> {
    oteryn_gameplay_npc::npc_talk::talk(speaker, text.as_str(), npcs, engaged)
}
