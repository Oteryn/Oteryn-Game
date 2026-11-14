use super::*;
use crate::play::play_channel;
use oteryn_client_ui::chat_private_channel::Intent;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn private_channels_actions_are_bound_and_confirmed() -> TestResult {
    let (link, mut commands, _) = play_channel();
    link.use_shared_frontend();
    lock(&link.mailbox).chat_private.selected = true;
    lock(&link.mailbox).chat_private.channel = Some(42);
    let bound = link.chat_private_bound();
    assert!(!link.chat_private_dispatch_bound(&bound, Intent::Leave, "", "", false));
    assert!(commands.try_recv().is_err());
    assert!(link.chat_private_dispatch_bound(&bound, Intent::Leave, "", "", true));
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::ChatPublic(oteryn_session::ChatIntent::PrivateChannel(
            oteryn_client_runtime::chat_private_channel::PrivateChannelIntent::Leave {
                channel_id: 42
            }
        ))
    ));
    assert_eq!(link.chat_private_panel().channel, Some(42));
    assert!(link.chat_private_panel().pending);
    assert!(!link.chat_private_dispatch_bound(&bound, Intent::Leave, "", "", true));
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn private_channels_rejected_when_session_replaced_or_target_changed() {
    let (link, mut commands, _) = play_channel();
    link.use_shared_frontend();
    lock(&link.mailbox).chat_private.selected = true;
    let bound = link.chat_private_bound();
    let (replacement, mut replacement_commands, _) = play_channel();
    replacement.use_shared_frontend();
    lock(&replacement.mailbox).chat_private = bound.panel.clone();
    assert!(!replacement.chat_private_dispatch_bound(&bound, Intent::Create, "", "", false));
    assert!(replacement_commands.try_recv().is_err());
    lock(&link.mailbox).chat_private.channel = Some(99);
    let before = link.chat_private_panel();
    assert!(!link.chat_private_dispatch_bound(&bound, Intent::Create, "", "", false));
    assert_eq!(before, link.chat_private_panel());
    assert!(commands.try_recv().is_err());
}

#[test]
fn private_channels_all_owner_actions_dispatch_once_and_settle() -> TestResult {
    use oteryn_client_runtime::chat_private_channel::PrivateChannelIntent as Wire;
    use oteryn_session::{ChatDisposition, ChatIntent, ChatOutcome, CommandStatus};
    for (intent, expected) in [
        (Intent::Create, Wire::Create),
        (Intent::List, Wire::List),
        (Intent::Join, Wire::Join { channel_id: 42 }),
        (Intent::Leave, Wire::Leave { channel_id: 42 }),
        (Intent::Invite, Wire::Invite { name: "Lin".into() }),
        (Intent::Exclude, Wire::Exclude { name: "Lin".into() }),
        (
            Intent::Send,
            Wire::Say {
                channel_id: 42,
                text: "Hello".into(),
            },
        ),
    ] {
        let (link, mut commands, outbox) = play_channel();
        link.use_shared_frontend();
        {
            let mut mailbox = lock(&link.mailbox);
            mailbox.chat_private.selected = true;
            mailbox.chat_private.channel = Some(42);
        }
        let bound = link.chat_private_bound();
        assert!(link.chat_private_dispatch_bound(&bound, intent, "Lin", "Hello", true));
        let crate::combat_input::PlayCommand::ChatPublic(ChatIntent::PrivateChannel(sent)) =
            commands.try_recv()?
        else {
            return Err("wrong dispatcher".into());
        };
        assert_eq!(sent, expected);
        assert!(commands.try_recv().is_err());
        let pending = link.chat_private_panel();
        assert!(pending.pending);
        assert_eq!(pending.channel, bound.panel.channel);
        assert_eq!(pending.list, bound.panel.list);
        assert_eq!(pending.lines, bound.panel.lines);
        assert!(outbox.chat_private_result(
            &ChatIntent::PrivateChannel(sent),
            &ChatOutcome {
                command_id: 1,
                status: CommandStatus::Rejected,
                disposition: ChatDisposition::NotAllowed,
                wait_seconds: 0,
                result_server_sequence: 1,
            }
        ));
        let settled = link.chat_private_panel();
        assert!(!settled.pending);
        assert!(settled.feedback.is_some());
        assert_eq!(settled.channel, bound.panel.channel);
    }
    Ok(())
}

#[test]
fn private_channels_close_admission_ignores_pending_capability_and_input() {
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    let bound = link.chat_private_bound();
    {
        let mut mailbox = lock(&link.mailbox);
        mailbox.chat_private.pending = true;
        mailbox.chat_private.channel = Some(99);
        mailbox.death.dead = true;
    }
    let before = link.chat_private_panel();
    assert!(link.chat_private_admit_bound(&bound, true));
    assert!(!link.chat_private_admit_bound(&bound, false));
    assert!(!link.chat_private_dispatch_bound(&bound, Intent::Create, "", "", false));
    assert_eq!(before, link.chat_private_panel());
    assert!(commands.try_recv().is_err());
    outbox.private_channels_test_end();
    assert!(!link.chat_private_admit_bound(&bound, true));
}

#[test]
fn private_channels_selection_uses_only_owner_rows_and_rejects_pending() {
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.private_channels_test_panel(oteryn_client_ui::chat_private_channel::ChatPanel {
        selected: true,
        list: Some(oteryn_session::ChatChannelList {
            own_channel: Some(42),
            invited: vec![oteryn_session::InvitedChannel {
                channel_id: 87,
                owner_name: "Ada".into(),
            }],
            ..Default::default()
        }),
        channel: Some(42),
        ..Default::default()
    });
    let bound = link.chat_private_bound();
    assert!(!link.chat_private_select_bound(&bound, 999));
    assert_eq!(link.chat_private_panel(), bound.panel);
    assert!(link.chat_private_select_bound(&bound, 87));
    assert_eq!(link.chat_private_panel().channel, Some(87));
    lock(&link.mailbox).chat_private.pending = true;
    let pending = link.chat_private_bound();
    assert!(!link.chat_private_select_bound(&pending, 42));
    assert_eq!(link.chat_private_panel(), pending.panel);
    assert!(commands.try_recv().is_err());
}

#[test]
fn private_channels_invalid_draft_shows_existing_owner_validation_without_request() {
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.private_channels_test_panel(oteryn_client_ui::chat_private_channel::ChatPanel {
        selected: true,
        channel: Some(42),
        ..Default::default()
    });
    let bound = link.chat_private_bound();
    assert!(!link.chat_private_dispatch_bound(&bound, Intent::Send, "", "", false));
    let actual = link.chat_private_panel();
    let mut native = bound.panel.clone();
    assert!(oteryn_client_ui::chat_private_channel::request(&mut native, Intent::Send).is_none());
    assert!(native.feedback.is_some());
    assert_eq!(actual.feedback, native.feedback);
    assert_eq!(actual.list, bound.panel.list);
    assert_eq!(actual.lines, bound.panel.lines);
    assert_eq!(actual.channel, bound.panel.channel);
    assert!(!actual.pending);
    assert!(commands.try_recv().is_err());
}
