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

#[test]
fn party_chat_results_show_refusals_and_retry_delay() {
    use crate::combat_input::PlayCommand;
    use crate::play::play_channel;
    use oteryn_session::{ChatDisposition, ChatOutcome, CommandStatus};
    let (link, mut commands, outbox) = play_channel();
    outbox.chat_party_room_initialize(true);
    for (disposition, expected) in [
        (ChatDisposition::Rejected, "rejected"),
        (ChatDisposition::ChatUnavailable, "unavailable"),
        (ChatDisposition::Exhausted, "17 seconds"),
        (ChatDisposition::Muted, "17 seconds"),
        (ChatDisposition::Ok, ""),
    ] {
        link.chat_party_room_text("hello".into());
        link.chat_party_room_send();
        let Ok(PlayCommand::ChatPublic(intent)) = commands.try_recv() else {
            unreachable!()
        };
        outbox.chat_result(
            &intent,
            &ChatOutcome {
                command_id: 1,
                status: CommandStatus::Accepted,
                disposition,
                wait_seconds: 17,
                result_server_sequence: 1,
            },
        );
        let panel = link.chat_party_room_panel();
        assert!(!panel.pending);
        if expected.is_empty() {
            assert!(panel.feedback.is_none());
        } else {
            assert!(
                panel
                    .feedback
                    .as_deref()
                    .is_some_and(|text| text.contains(expected))
            );
            assert!(oteryn_client_ui::chat_party_room::panel_text(&panel).contains(expected));
        }
    }
}

#[test]
fn both_composers_clear_only_accepted_unchanged_drafts() -> Result<(), Box<dyn std::error::Error>> {
    use crate::combat_input::PlayCommand;
    use crate::play::play_channel;
    use oteryn_client_ui::chat_public_channels::Intent;
    use oteryn_session::{ChatDisposition, ChatOutcome, ChatRoom, CommandStatus};
    let (link, mut commands, outbox) = play_channel();
    outbox.chat_party_room_initialize(true);
    outbox.chat_public_initialize(true, None);
    for party in [true, false] {
        for newer in [false, true] {
            for (status, disposition) in [
                (CommandStatus::Accepted, ChatDisposition::Ok),
                (CommandStatus::DuplicateReplay, ChatDisposition::Ok),
                (CommandStatus::Rejected, ChatDisposition::Rejected),
                (CommandStatus::Accepted, ChatDisposition::Exhausted),
            ] {
                let set_text = |text: &str| {
                    if party {
                        link.chat_party_room_text(text.into());
                    } else {
                        link.chat_public_text(text.into());
                    }
                };
                set_text("submitted");
                if party {
                    link.chat_party_room_send();
                } else {
                    link.chat_public_request(Intent::Send(ChatRoom::World));
                }
                let PlayCommand::ChatPublic(intent) = commands.try_recv()? else {
                    unreachable!()
                };
                if newer {
                    set_text("newer edit");
                }
                outbox.chat_result(
                    &intent,
                    &ChatOutcome {
                        command_id: 1,
                        status,
                        disposition,
                        wait_seconds: 2,
                        result_server_sequence: 1,
                    },
                );
                let text = if party {
                    link.chat_party_room_panel().text
                } else {
                    link.chat_public_panel().text
                };
                let expected = if newer {
                    "newer edit"
                } else if disposition == ChatDisposition::Ok {
                    ""
                } else {
                    "submitted"
                };
                assert_eq!(
                    text, expected,
                    "party={party}, newer={newer}, {status:?}, {disposition:?}"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn party_rejections_preserve_disposition_and_retry_delay() {
    use crate::play::{mailbox::lock, play_channel};
    use oteryn_session::{ChatDisposition, ChatIntent, ChatOutcome};
    let (_link, _commands, outbox) = play_channel();
    for disposition in [
        ChatDisposition::Muted,
        ChatDisposition::Exhausted,
        ChatDisposition::NotAllowed,
    ] {
        lock(&outbox.0).chat_party_room.pending = true;
        outbox.chat_result(
            &ChatIntent::Party {
                text: "hello".into(),
            },
            &ChatOutcome {
                command_id: 1,
                status: oteryn_session::CommandStatus::Rejected,
                disposition,
                wait_seconds: 17,
                result_server_sequence: 1,
            },
        );
        let mailbox = lock(&outbox.0);
        assert!(!mailbox.chat_party_room.pending);
        let feedback = mailbox.chat_party_room.feedback.as_deref().unwrap_or("");
        assert!(feedback.contains(&format!("{disposition:?}")), "{feedback}");
        assert!(feedback.contains("17"), "{feedback}");
    }
}
