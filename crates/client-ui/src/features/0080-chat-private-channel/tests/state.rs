use super::*;
use oteryn_client_runtime::chat_private_channel::{
    ChatChannelList, ChatLine, InvitedChannel, PrivateChannelIntent, PrivateChannelLine,
};

#[test]
fn private_channel_input_and_server_projection() {
    let mut panel = ChatPanel::default();
    assert!(request(&mut panel, Intent::Create).is_none());
    panel.selected = true;
    assert_eq!(
        request(&mut panel, Intent::Create),
        Some(PrivateChannelIntent::Create)
    );
    assert!(request(&mut panel, Intent::List).is_none());
    panel.pending = false;
    replace_state(
        &mut panel,
        Some(ChatChannelList {
            own_channel: Some(100),
            invited: vec![InvitedChannel {
                channel_id: 101,
                owner_name: "Friend".into(),
            }],
            ..Default::default()
        }),
        vec![ChatLine::PrivateChannel(PrivateChannelLine {
            channel_id: 101,
            speaker_name: "Friend".into(),
            text: "welcome".into(),
        })],
    );
    assert_eq!(panel.channel, Some(100));
    panel.channel = Some(101);
    assert!(panel_text(&panel).contains("Friend: welcome"));
    assert_eq!(
        request(&mut panel, Intent::Join),
        Some(PrivateChannelIntent::Join { channel_id: 101 })
    );
    panel.pending = false;
    panel.name = "Friend".into();
    assert_eq!(
        request(&mut panel, Intent::Invite),
        Some(PrivateChannelIntent::Invite {
            name: "Friend".into()
        })
    );
    panel.pending = false;
    assert_eq!(
        request(&mut panel, Intent::Exclude),
        Some(PrivateChannelIntent::Exclude {
            name: "Friend".into()
        })
    );
    panel.pending = false;
    panel.text = "hello".into();
    assert_eq!(
        request(&mut panel, Intent::Send),
        Some(PrivateChannelIntent::Say {
            channel_id: 101,
            text: "hello".into()
        })
    );
    replace_state(&mut panel, Some(ChatChannelList::default()), vec![]);
    assert_eq!(panel.channel, None);
    panel.pending = false;
    assert!(request(&mut panel, Intent::Join).is_none());
}

#[test]
fn private_channel_rejects_invalid_inputs_and_filters_transcript() {
    let mut panel = ChatPanel {
        selected: true,
        ..Default::default()
    };
    assert!(request(&mut panel, Intent::Invite).is_none());
    panel.name = "\0".into();
    assert!(request(&mut panel, Intent::Exclude).is_none());
    panel.channel = Some(100);
    panel.text = " ".into();
    assert!(request(&mut panel, Intent::Send).is_none());
    panel.text = "x".repeat(1021);
    assert!(request(&mut panel, Intent::Send).is_none());
    panel.lines = vec![ChatLine::PrivateChannel(PrivateChannelLine {
        channel_id: 101,
        speaker_name: "Other".into(),
        text: "hidden".into(),
    })];
    panel.lines.push(private_line(100, "shown"));
    assert!(!panel_text(&panel).contains("hidden"));
    assert!(panel_text(&panel).contains("shown"));
}

#[test]
fn private_channel_refusal_releases_pending_and_shows_server_reason() {
    use oteryn_client_runtime::chat_private_channel::{ChatDisposition, ChatOutcome};
    let mut panel = ChatPanel {
        selected: true,
        pending: true,
        ..Default::default()
    };
    apply_result(
        &mut panel,
        &ChatOutcome {
            command_id: 1,
            status: oteryn_client_runtime::CommandStatus::Rejected,
            disposition: ChatDisposition::NotInvited,
            wait_seconds: 0,
            result_server_sequence: 1,
        },
    );
    assert!(!panel.pending);
    assert_eq!(
        panel.feedback.as_deref(),
        Some("You are not invited to this channel")
    );
}

fn private_line(channel_id: u16, text: &str) -> ChatLine {
    ChatLine::PrivateChannel(PrivateChannelLine {
        channel_id,
        speaker_name: "Player".into(),
        text: text.into(),
    })
}

#[test]
fn reused_channel_id_hides_closed_channel_history() {
    let mut panel = ChatPanel::default();
    let list = Some(ChatChannelList {
        own_channel: Some(100),
        ..Default::default()
    });
    let old = vec![private_line(100, "old channel text")];
    replace_state(&mut panel, list.clone(), old.clone());
    assert!(panel_text(&panel).contains("old channel text"));
    replace_state(&mut panel, Some(ChatChannelList::default()), old.clone());
    replace_state(&mut panel, list.clone(), old.clone());
    assert!(!panel_text(&panel).contains("old channel text"));
    let mut lines = old;
    lines.push(private_line(100, "new channel text"));
    replace_state(&mut panel, list.clone(), lines);
    assert!(!panel_text(&panel).contains("old channel text"));
    assert!(panel_text(&panel).contains("new channel text"));
    // The shared session log evicts its oldest lines at capacity.
    replace_state(
        &mut panel,
        list,
        vec![
            private_line(100, "new channel text"),
            private_line(100, "later text"),
        ],
    );
    assert!(panel_text(&panel).contains("new channel text"));
    assert!(panel_text(&panel).contains("later text"));
}

#[test]
fn private_channel_refusals_explain_the_requested_action() {
    use oteryn_client_runtime::chat_private_channel::{ChatDisposition, ChatOutcome};
    for (intent, disposition, expected) in [
        (
            Intent::Create,
            ChatDisposition::NotAllowed,
            "Premium is required to create a private channel",
        ),
        (
            Intent::Invite,
            ChatDisposition::NotAllowed,
            "You have no own private channel",
        ),
        (
            Intent::Exclude,
            ChatDisposition::NotAllowed,
            "You have no own private channel",
        ),
        (
            Intent::Send,
            ChatDisposition::RoomNotOpen,
            "Join the channel first",
        ),
    ] {
        let mut panel = ChatPanel {
            selected: true,
            channel: Some(100),
            name: "Friend".into(),
            text: "hello".into(),
            ..Default::default()
        };
        assert!(request(&mut panel, intent).is_some());
        apply_result(
            &mut panel,
            &ChatOutcome {
                command_id: 1,
                status: oteryn_client_runtime::CommandStatus::Rejected,
                disposition,
                wait_seconds: 0,
                result_server_sequence: 1,
            },
        );
        assert!(!panel.pending);
        assert_eq!(panel.feedback.as_deref(), Some(expected));
    }
}

#[test]
fn private_channel_leave_label_warns_the_owner_that_it_closes() {
    let mut panel = ChatPanel::default();
    assert_eq!(leave_label(&panel), "Leave");
    replace_state(
        &mut panel,
        Some(ChatChannelList {
            own_channel: Some(100),
            invited: vec![InvitedChannel {
                channel_id: 101,
                owner_name: "Friend".into(),
            }],
            ..Default::default()
        }),
        vec![],
    );
    assert_eq!(leave_label(&panel), "Close own channel");
    panel.channel = Some(101);
    assert_eq!(leave_label(&panel), "Leave");
}
