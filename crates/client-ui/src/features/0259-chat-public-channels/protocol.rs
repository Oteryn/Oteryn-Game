use super::ChatPanel;
use oteryn_client_runtime::chat_public_channels::{
    ChatDisposition, ChatIntent, ChatLine, ChatOutcome, ChatRoom, ChatRoomSet,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Open(ChatRoom),
    Close(ChatRoom),
    Send(ChatRoom),
}

pub fn request(panel: &mut ChatPanel, intent: Intent) -> Option<ChatIntent> {
    if !panel.selected {
        panel.feedback = Some(tr!("Public channels are unavailable in this session").into());
        return None;
    }
    if panel.pending {
        panel.feedback = Some(tr!("Waiting for the server").into());
        return None;
    }
    let command = match intent {
        Intent::Open(room) => ChatIntent::OpenRoom(room),
        Intent::Close(room) => ChatIntent::CloseRoom(room),
        Intent::Send(room) => ChatIntent::Room {
            room,
            text: panel.text.clone(),
        },
    };
    if matches!(&command, ChatIntent::Room { text, .. } if text.is_empty() || text.len() > 1_020) {
        panel.feedback = Some(tr!("Enter a message of at most 1,020 bytes").into());
        return None;
    }
    panel.pending = true;
    panel.feedback = None;
    Some(command)
}

pub fn apply_result(panel: &mut ChatPanel, outcome: &ChatOutcome) {
    panel.pending = false;
    panel.feedback = match outcome.disposition {
        ChatDisposition::Ok => None,
        ChatDisposition::Muted | ChatDisposition::Exhausted => Some(lfmt!(
            "Wait {} seconds before speaking",
            outcome.wait_seconds
        )),
        ChatDisposition::LevelTooLow => Some(tr!("Your level is too low").into()),
        ChatDisposition::NoVocation => Some(tr!("This room requires a vocation").into()),
        ChatDisposition::RoomNotOpen => Some(tr!("Open the room before speaking").into()),
        ChatDisposition::ChatUnavailable => Some(tr!("Chat is unavailable").into()),
        ChatDisposition::NotOnline => Some(tr!("The named player is not online").into()),
        ChatDisposition::Rejected => Some(tr!("The server rejected the chat command").into()),
        ChatDisposition::NotAllowed => Some(tr!("You are not allowed in this room").into()),
        ChatDisposition::NotInvited => Some(tr!("You are not invited to this channel").into()),
    };
}

pub fn replace_state(panel: &mut ChatPanel, rooms: ChatRoomSet, lines: Vec<ChatLine>) {
    panel.rooms = rooms;
    panel.lines = lines;
}
