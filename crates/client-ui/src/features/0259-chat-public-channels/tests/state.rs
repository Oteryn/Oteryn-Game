use super::*;
use oteryn_client_runtime::chat_public_channels::{ChatLine, ChatRoom, ChatRoomSet};

#[test]
fn server_room_projection_and_lines_replace_panel_state() {
    let mut rooms = ChatRoomSet::default();
    rooms.insert(ChatRoom::AdvertisingRookgaard);
    rooms.insert(ChatRoom::Tutor);
    let lines = vec![ChatLine::Room {
        room: ChatRoom::Tutor,
        speaker_name: "Tutor One".into(),
        text: "Please ask here".into(),
    }];
    let mut panel = ChatPanel::default();
    replace_state(&mut panel, rooms, lines);
    assert!(panel.rooms.contains(ChatRoom::AdvertisingRookgaard));
    assert!(panel.rooms.contains(ChatRoom::Tutor));
    assert!(panel_text(&panel).contains("[Tutor] Tutor One: Please ask here"));
}

#[test]
fn every_extended_room_builds_the_real_existing_chat_intent() {
    for room in [
        ChatRoom::AdvertisingRookgaard,
        ChatRoom::Tutor,
        ChatRoom::Gamemaster,
        ChatRoom::GuildLeaders,
    ] {
        let mut panel = ChatPanel {
            selected: true,
            text: "hello".into(),
            ..ChatPanel::default()
        };
        assert_eq!(
            request(&mut panel, Intent::Open(room)),
            Some(oteryn_client_runtime::chat_public_channels::ChatIntent::OpenRoom(room))
        );
        panel.pending = false;
        assert_eq!(
            request(&mut panel, Intent::Close(room)),
            Some(oteryn_client_runtime::chat_public_channels::ChatIntent::CloseRoom(room))
        );
        panel.pending = false;
        assert_eq!(
            request(&mut panel, Intent::Send(room)),
            Some(
                oteryn_client_runtime::chat_public_channels::ChatIntent::Room {
                    room,
                    text: "hello".into()
                }
            )
        );
    }
}
