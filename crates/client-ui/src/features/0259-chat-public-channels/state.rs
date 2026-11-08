use oteryn_client_runtime::chat_public_channels::{ChatLine, ChatRoom, ChatRoomSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatPanel {
    pub console: crate::chat::Console,
    pub guild_selected: bool,
    pub guild_open: bool,
    pub guild: oteryn_client_runtime::chat_guild_room::GuildRoomState,
    pub selected: bool,
    pub speech_selected: bool,
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
            console: Default::default(),
            guild_selected: false,
            guild_open: false,
            guild: Default::default(),
            selected: false,
            speech_selected: false,
            open: false,
            rooms: ChatRoomSet::default(),
            lines: Vec::new(),
            room: ChatRoom::World,
            text: String::new(),
            pending: false,
            feedback: None,
        }
    }
}
