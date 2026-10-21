//! NPC talk over the chat path; validated chat text enters the NPC runtime here.

use crate::chat::{ChatPosition, ChatText};
#[allow(unused_imports, reason = "preserve the former NPC module path")]
pub use oteryn_gameplay_npc::npc_talk::{NpcReply, NpcTalkBook, NpcTalkEntry, TalkNpc};

pub fn asks_to_trade(text: &ChatText) -> bool {
    oteryn_gameplay_npc::npc_talk::asks_to_trade(text.as_str())
}

pub fn talk<K: Copy + PartialEq>(
    speaker: ChatPosition,
    text: &ChatText,
    npcs: &[TalkNpc<'_, K>],
    engaged: &mut Option<K>,
) -> Option<NpcReply<K>> {
    oteryn_gameplay_npc::npc_talk::talk(speaker, text.as_str(), npcs, engaged)
}
