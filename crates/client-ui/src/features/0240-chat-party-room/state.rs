use oteryn_client_runtime::cl25a_protocol::{ChatLine, ChatRoomSet};
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChatPartyRoomPanel {
    pub selected: bool,
    pub open: bool,
    pub rooms: ChatRoomSet,
    pub lines: Vec<ChatLine>,
    pub feedback: Option<String>,
}
