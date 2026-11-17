use crate::chat::{ChatPanel, Mode, request};
use oteryn_client_runtime::{chat::ChatIntent, chat_guild_room::GuildRoomState};
#[test]
fn guild_composer_requires_live_membership() {
    let mut panel = ChatPanel {
        selected: true,
        guild_selected: true,
        text: "hello".into(),
        ..Default::default()
    };
    for state in [GuildRoomState::Absent, GuildRoomState::Unavailable] {
        super::replace_state(&mut panel, true, state);
        assert!(!super::available(&panel));
        assert!(request(&mut panel, Mode::Guild, "").is_none());
    }
    panel.guild = GuildRoomState::Member {
        guild_id: [1; 16],
        rank: 1,
        motd: "Welcome".into(),
    };
    assert!(super::available(&panel));
    panel.guild_selected = false;
    assert!(request(&mut panel, Mode::Guild, "").is_none());
    panel.guild_selected = true;
    for text in [" ".to_owned(), "x".repeat(1021), "bad\0text".to_owned()] {
        panel.text = text;
        assert!(request(&mut panel, Mode::Guild, "").is_none());
    }
    panel.text = "hello".into();
    assert_eq!(
        request(&mut panel, Mode::Guild, ""),
        Some(ChatIntent::Guild {
            text: "hello".into()
        })
    );
    assert!(request(&mut panel, Mode::Guild, "").is_none());
    panel.pending = false;
    super::replace_state(&mut panel, true, GuildRoomState::Absent);
    assert!(request(&mut panel, Mode::Guild, "").is_none());
}
