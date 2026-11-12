#[test]
fn panel_reports_selection() {
    let panel = oteryn_client_ui::chat_party_room::ChatPartyRoomPanel {
        selected: true,
        ..Default::default()
    };
    assert!(!oteryn_client_ui::chat_party_room::panel_text(&panel).contains("unavailable"));
}

#[test]
fn chat_snapshot_clears_retained_lines_before_later_party_messages() {
    use crate::play::{mailbox, play_channel};
    use oteryn_session::{AppliedDelta, ChatLine, ChatLog, SessionEvent};
    let line = |text: &str| ChatLine::Party {
        speaker_name: "Leader".into(),
        text: text.into(),
    };
    let (link, _commands, outbox) = play_channel();
    mailbox::lock(&outbox.0)
        .chat_party_room
        .lines
        .push(line("stale"));
    outbox.push(vec![
        SessionEvent::ChatLine(AppliedDelta {
            server_sequence: 1,
            base_revision: 1,
            new_revision: 2,
            value: line("older"),
        }),
        SessionEvent::ChatSnapshot(AppliedDelta {
            server_sequence: 2,
            base_revision: 2,
            new_revision: 9,
            value: ChatLog::default(),
        }),
        SessionEvent::ChatLine(AppliedDelta {
            server_sequence: 3,
            base_revision: 9,
            new_revision: 10,
            value: line("new"),
        }),
    ]);
    assert_eq!(
        mailbox::lock(&outbox.0).chat_party_room.lines,
        vec![line("new")]
    );
    drop(link);
}
