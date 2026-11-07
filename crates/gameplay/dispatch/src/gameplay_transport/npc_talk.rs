//! NPC talk over the chat path; validated chat text enters the NPC runtime here.

use crate::chat::{ChatPosition, ChatText};
#[allow(unused_imports, reason = "preserve the former NPC module path")]
pub use oteryn_gameplay_npc::npc_talk::{NpcReply, NpcTalkBook, NpcTalkEntry, TalkNpc};

pub fn asks_to_trade(text: &ChatText) -> bool {
    oteryn_gameplay_npc::npc_talk::asks_to_trade(text.as_str())
}

pub(super) fn hireling_bank_entry() -> NpcTalkEntry {
    NpcTalkEntry::new("Hireling", None).with_bank()
}

pub(super) fn world_minute(utc_ms: Option<u64>) -> Option<u32> {
    crate::world_clock::WorldClockSource::new(crate::world_clock::DEFAULT_WORLD_CLOCK_EPOCH_UTC_MS)
        .tibian_minute(utc_ms?)
}

/// Read the same World clock as the wire clock; unavailable facts never leak a source token.
pub(super) fn render_world_time(text: &str, minute: Option<u32>) -> Option<String> {
    if !text.contains("|TIME|") {
        return Some(text.into());
    }
    let minute = minute.filter(|minute| *minute < 1440)?;
    Some(text.replace("|TIME|", &format!("{:02}:{:02}", minute / 60, minute % 60)))
}

pub fn talk<K: Copy + PartialEq>(
    speaker: ChatPosition,
    text: &ChatText,
    npcs: &[TalkNpc<'_, K>],
    engaged: &mut Option<K>,
) -> Option<NpcReply<K>> {
    oteryn_gameplay_npc::npc_talk::talk(speaker, text.as_str(), npcs, engaged)
}

#[cfg(test)]
mod source_time_tests {
    #[test]
    fn source_time_uses_world_minutes_and_requires_a_clock_fact() {
        assert_eq!(
            super::render_world_time("At |TIME|.", Some(90)).as_deref(),
            Some("At 01:30.")
        );
        assert_eq!(super::render_world_time("At |TIME|.", None), None);
        assert_eq!(super::render_world_time("At |TIME|.", Some(1440)), None);
        assert_eq!(
            super::render_world_time("Hello.", None).as_deref(),
            Some("Hello.")
        );
    }
}
