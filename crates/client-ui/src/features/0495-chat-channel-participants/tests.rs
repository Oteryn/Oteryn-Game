use super::*;
use crate::chat::{ChannelTab, ChatExtras, ChatPanel, panel_text};
use oteryn_client_runtime::chat_private_channel::{ChatChannelList, InvitedChannel};

fn list() -> ChatChannelList {
    ChatChannelList {
        public_rooms: Default::default(),
        own_channel: Some(120),
        invited: vec![InvitedChannel {
            channel_id: 300,
            owner_name: "Bo".into(),
        }],
    }
}

fn notice(serial: u64, channel_id: u16, kind: NoticeKind, name: &str) -> NoticeLine {
    NoticeLine {
        serial,
        channel_id,
        kind,
        name: name.into(),
    }
}

fn extras() -> ChatExtras {
    ChatExtras {
        participants: Some(vec![
            (120, vec!["Ari".into()]),
            (300, vec!["Ari".into(), "Bo".into()]),
        ]),
        notices: Some(vec![
            notice(1, 300, NoticeKind::Joined, "Cy"),
            notice(2, 300, NoticeKind::Left, "Cy"),
            notice(3, 300, NoticeKind::Excluded, "Di"),
            notice(4, 120, NoticeKind::Joined, "Eve"),
        ]),
        ..Default::default()
    }
}

#[test]
fn c0495_tabs_read_owners_channel_with_participants_and_notices() {
    let mut panel = ChatPanel::default();
    panel.console.open_channel(ChannelTab::Private(120));
    panel.console.open_channel(ChannelTab::Private(300));
    panel.apply_extras("Ari", Some(&list()), extras());
    let labels = panel.console.tab_labels();
    assert_eq!(labels[4..], ["Ari's channel", "Bo's channel"]);
    let text = panel_text(&panel);
    assert!(text.contains("Participants: Ari, Bo"), "{text}");
    assert!(text.contains("Cy has joined the channel"), "{text}");
    assert!(text.contains("Cy has left the channel"), "{text}");
    assert!(
        text.contains("Di has been excluded from the channel"),
        "{text}"
    );
    assert!(!text.contains("Eve"), "{text}");
}

#[test]
fn c0495_notices_are_taken_once_and_reset_with_the_session() {
    let mut panel = ChatPanel::default();
    panel.apply_extras("Ari", Some(&list()), extras());
    panel.apply_extras("Ari", Some(&list()), extras());
    assert_eq!(panel.console.channel_notices.len(), 4);
    panel.console.reset_extensions();
    assert!(panel.console.channel_notices.is_empty());
    panel.apply_extras("Ari", Some(&list()), extras());
    assert_eq!(panel.console.channel_notices.len(), 4);
}

#[test]
fn c0495_without_the_capability_the_tab_reads_channel_and_id() {
    let mut panel = ChatPanel::default();
    panel.console.open_channel(ChannelTab::Private(300));
    panel.apply_extras("Ari", Some(&list()), ChatExtras::default());
    assert_eq!(panel.console.tab_labels()[4], "Channel 300");
    let text = panel_text(&panel);
    assert!(!text.contains("Participants"), "{text}");
    assert!(panel.console.channel_notices.is_empty());
}
