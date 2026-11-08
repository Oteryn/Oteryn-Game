use super::Intent;
use oteryn_client_runtime::chat_private_channel::{ChatChannelList, ChatLine};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChatPanel {
    pub communication: crate::chat::CommunicationLists,
    pub selected: bool,
    pub list: Option<ChatChannelList>,
    pub channel: Option<u16>,
    pub lines: Vec<ChatLine>,
    /// First visible position for closed channel IDs in the current bounded session log.
    pub history_start: BTreeMap<u16, usize>,
    pub name: String,
    pub text: String,
    pub pending: bool,
    pub pending_intent: Option<Intent>,
    pub feedback: Option<String>,
}
