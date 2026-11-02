use oteryn_client_runtime::chat_public_channels::{ChatLine, ChatRoom, ChatRoomSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatPanel {
    pub selected: bool,
    pub open: bool,
    pub rooms: ChatRoomSet,
    pub lines: Vec<ChatLine>,
    pub room: ChatRoom,
    pub text: String,
    pub pending: bool,
    pub feedback: Option<String>,
}

impl Default for ChatPanel {
    fn default() -> Self {
        Self {
            selected: false,
            open: false,
            rooms: ChatRoomSet::default(),
            lines: Vec::new(),
            room: ChatRoom::AdvertisingRookgaard,
            text: String::new(),
            pending: false,
            feedback: None,
        }
    }
}
