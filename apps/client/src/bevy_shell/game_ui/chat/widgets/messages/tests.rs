use super::*;
use oteryn_client_ui::chat::{ChannelTab, ChatPanel};
#[test]
fn outgoing_private_message_targets_recipient() {
    let panel = ChatPanel {
        lines: vec![oteryn_session::ChatLine::Private {
            speaker_name: "Alice".into(),
            text: "hello".into(),
        }],
        recipients: vec![Some("Bob".into())],
        ..default()
    };
    let rows = transcript_rows(&panel);
    let target = rows
        .iter()
        .find(|row| row.text.contains("To Bob"))
        .and_then(RenderedRow::target);
    assert_eq!(target, Some("Bob"));
}

#[test]
fn transcript_preserves_entry_boundaries_and_feedback_order() {
    let mut panel = ChatPanel {
        lines: vec![oteryn_session::ChatLine::Private {
            speaker_name: "Alice".into(),
            text: "hello".into(),
        }],
        feedback: Some(String::from("feedback")),
        ..default()
    };
    let texts = |panel: &ChatPanel| {
        transcript_rows(panel)
            .into_iter()
            .map(|row| row.text)
            .collect::<Vec<_>>()
    };
    assert_eq!(texts(&panel).last().map(String::as_str), Some("feedback\n"));
    panel.console.current = Some(ChannelTab::Npc);
    panel.console.npc_lines = vec!["first".into(), "second".into()];
    assert_eq!(texts(&panel), ["first\n", "second\n", "feedback\n"]);
    use oteryn_client_ui::chat::NoticeClass;
    panel
        .console
        .notices
        .push(NoticeClass::Loot, "first\n[Loot] still first", 0);
    panel.console.notices.push(NoticeClass::Loot, "second", 1);
    for time in [false, true] {
        panel.console.display.time = time;
        for tab in [ChannelTab::Loot, ChannelTab::ServerLog] {
            panel.console.current = Some(tab);
            let rows = texts(&panel);
            assert_eq!(rows.len(), 3);
            assert!(rows[0].ends_with("[Loot] first\n[Loot] still first\n"));
            assert!(rows[1].ends_with("[Loot] second\n"));
            assert_eq!(rows[2], "feedback\n");
        }
    }
    panel.console.current = None;
    let rows = texts(&panel);
    assert!(rows[0].contains("hello"));
    assert!(rows[1].contains("still first"));
    use oteryn_client_ui::chat_channel_participants::{NoticeKind, NoticeLine};
    panel.console.current = Some(ChannelTab::Private(7));
    panel.console.participants.insert(7, vec!["Bob".into()]);
    panel.console.channel_notices.push(NoticeLine {
        serial: 1,
        channel_id: 7,
        kind: NoticeKind::Joined,
        name: "Bob".into(),
    });
    panel.lines.push(oteryn_session::ChatLine::PrivateChannel(
        oteryn_session::PrivateChannelLine {
            channel_id: 7,
            speaker_name: "Alice".into(),
            text: "channel speech".into(),
        },
    ));
    let rows = texts(&panel);
    assert_eq!(rows.len(), 4);
    assert!(rows[0].contains("Participants"));
    assert!(rows[1].contains("channel speech"));
    assert!(rows[2].contains("has joined"));
    assert_eq!(rows[3], "feedback\n");
}
