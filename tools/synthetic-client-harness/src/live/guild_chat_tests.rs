use super::*;
#[test]
fn guild_chat_ranks_and_login_message_render_without_identity_or_location() {
    for (rank, label) in [(1, " Leader"), (2, " Vice Leader"), (20, "")] {
        let line = GuildChatLine {
            guild_id: [9; 16],
            rank,
            speaker_name: "Knight".into(),
            text: "Hello".into(),
        };
        assert_eq!(render_line(&line), format!("[Guild{label}] Knight: Hello"));
    }
    let mut out = String::new();
    render_room(
        &mut out,
        &GuildRoomState::Member {
            guild_id: [9; 16],
            rank: 1,
            motd: "Welcome".into(),
        },
    );
    assert_eq!(out, "guild: Welcome\n");
    assert!(
        matches!(super::super::cli::parse_line("guild Hello"),Some(super::super::cli::LineCommand::Chat(oteryn_dev_client::ChatIntent::Guild { text })) if text=="Hello")
    );
}
