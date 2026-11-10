use super::*;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
#[test]
fn generic_modal_panel_drives_wire_and_clears_only_after_matching_ack()
-> Result<(), Box<dyn std::error::Error>> {
    let mut panel = Panel::default();
    assert!(query(&mut panel).is_none());
    panel.selected = true;
    let request = query(&mut panel).ok_or("query")?;
    assert_eq!(request.kind, 736);
    assert_eq!(
        request.payload,
        oteryn_client_runtime::server_modal::QUERY_WITH_TERMINAL_REPLY
    );
    let dialog = Dialog {
        id: 42,
        title: "Choose".into(),
        message: "Select one".into(),
        choices: vec![(7, "First".into())],
        buttons: vec![(1, "OK".into()), (2, "Cancel".into())],
        cancel_button: Some(2),
        image: None,
    };
    let result = |payload| GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload,
    };
    assert!(apply_result(
        &mut panel,
        &result(Reply::Dialog(dialog.clone()).encode()?)
    ));
    assert!(panel_text(&panel).contains("Select one"));
    let chosen = Answer {
        dialog_id: 42,
        button_id: 1,
        choice_id: Some(7),
    };
    assert!(
        answer(
            &mut panel,
            Answer {
                choice_id: Some(9),
                ..chosen
            }
        )
        .is_none()
    );
    let request = answer(&mut panel, chosen).ok_or("answer")?;
    assert_eq!(request.kind, 737);
    assert_eq!(Answer::decode(&request.payload)?, chosen);
    assert!(panel.active.is_some());
    assert!(!apply_result(
        &mut panel,
        &result(
            Reply::Answered(Answer {
                dialog_id: 43,
                ..chosen
            })
            .encode()?
        )
    ));
    assert_eq!(panel.active, Some(dialog));
    assert!(answer(&mut panel, chosen).is_some());
    assert!(apply_result(
        &mut panel,
        &result(Reply::Answered(chosen).encode()?)
    ));
    assert!(panel.active.is_none());
    assert!(!panel.logged_out);
    assert!(answer(&mut panel, chosen).is_none());
    Ok(())
}

fn result(status: CommandStatus, payload: Vec<u8>) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status,
        payload,
    }
}

fn chosen() -> Answer {
    Answer {
        dialog_id: 42,
        button_id: 1,
        choice_id: Some(7),
    }
}

fn active_panel() -> Result<Panel, Box<dyn std::error::Error>> {
    let mut panel = Panel {
        selected: true,
        ..Panel::default()
    };
    query(&mut panel).ok_or("query")?;
    assert!(apply_result(
        &mut panel,
        &result(
            CommandStatus::Accepted,
            Reply::Dialog(Dialog {
                id: 42,
                title: "Choose".into(),
                message: "Select one".into(),
                choices: vec![(7, "First".into())],
                buttons: vec![(1, "OK".into()), (2, "Cancel".into())],
                cancel_button: Some(2),
                image: None,
            })
            .encode()?
        )
    ));
    Ok(panel)
}

#[test]
fn terminal_ack_ends_only_the_negotiated_matching_answer() -> Result<(), Box<dyn std::error::Error>>
{
    let mut panel = active_panel()?;
    assert_eq!(panel.terminal_replies, TerminalReplies::Enabled);
    answer(&mut panel, chosen()).ok_or("answer")?;
    assert!(apply_result(
        &mut panel,
        &result(
            CommandStatus::Accepted,
            Reply::AnsweredAndLoggedOut(chosen()).encode()?
        )
    ));
    assert!(panel.logged_out);
    assert!(panel.active.is_none());
    assert!(query(&mut panel).is_none());
    assert!(answer(&mut panel, chosen()).is_none());
    Ok(())
}

#[test]
fn refused_unnegotiated_stale_or_cancelled_answers_never_log_out()
-> Result<(), Box<dyn std::error::Error>> {
    let selected = chosen();
    let cancel = Answer {
        button_id: 2,
        choice_id: None,
        ..selected
    };
    for (sent, received, status, mode) in [
        (
            selected,
            selected,
            CommandStatus::Rejected,
            TerminalReplies::Enabled,
        ),
        (
            selected,
            selected,
            CommandStatus::DuplicateReplay,
            TerminalReplies::Enabled,
        ),
        (
            selected,
            selected,
            CommandStatus::Accepted,
            TerminalReplies::Probing,
        ),
        (
            selected,
            selected,
            CommandStatus::Accepted,
            TerminalReplies::Unsupported,
        ),
        (
            selected,
            Answer {
                dialog_id: 43,
                ..selected
            },
            CommandStatus::Accepted,
            TerminalReplies::Enabled,
        ),
        (
            selected,
            Answer {
                choice_id: Some(8),
                ..selected
            },
            CommandStatus::Accepted,
            TerminalReplies::Enabled,
        ),
        (
            cancel,
            cancel,
            CommandStatus::Accepted,
            TerminalReplies::Enabled,
        ),
    ] {
        let mut panel = active_panel()?;
        panel.terminal_replies = mode;
        answer(&mut panel, sent).ok_or("answer")?;
        assert!(!apply_result(
            &mut panel,
            &result(status, Reply::AnsweredAndLoggedOut(received).encode()?)
        ));
        assert!(!panel.logged_out);
        assert!(panel.active.is_some());
    }
    for sent in [selected, cancel] {
        let mut panel = active_panel()?;
        answer(&mut panel, sent).ok_or("answer")?;
        assert!(apply_result(
            &mut panel,
            &result(CommandStatus::Accepted, Reply::Answered(sent).encode()?)
        ));
        assert!(!panel.logged_out);
        assert!(query(&mut panel).is_some());
    }
    Ok(())
}

#[test]
fn only_explicit_initial_query_rejection_selects_the_legacy_query()
-> Result<(), Box<dyn std::error::Error>> {
    let mut panel = Panel {
        selected: true,
        ..Panel::default()
    };
    let opt_in = oteryn_client_runtime::server_modal::QUERY_WITH_TERMINAL_REPLY;
    assert_eq!(query(&mut panel).ok_or("query")?.payload, opt_in);
    assert!(!apply_result(
        &mut panel,
        &result(CommandStatus::Accepted, vec![255])
    ));
    assert_eq!(panel.terminal_replies, TerminalReplies::Probing);
    assert_eq!(query(&mut panel).ok_or("retry query")?.payload, opt_in);
    assert!(apply_result(
        &mut panel,
        &result(CommandStatus::DuplicateReplay, Reply::None.encode()?)
    ));
    assert_eq!(panel.terminal_replies, TerminalReplies::Probing);
    assert_eq!(query(&mut panel).ok_or("query")?.payload, opt_in);
    assert!(!apply_result(
        &mut panel,
        &result(CommandStatus::Rejected, vec![])
    ));
    assert_eq!(panel.terminal_replies, TerminalReplies::Unsupported);
    for _ in 0..2 {
        assert!(query(&mut panel).ok_or("legacy query")?.payload.is_empty());
        assert!(apply_result(
            &mut panel,
            &result(CommandStatus::Accepted, Reply::None.encode()?)
        ));
        assert_eq!(panel.terminal_replies, TerminalReplies::Unsupported);
        assert!(!panel.logged_out);
    }
    Ok(())
}

#[test]
fn terminal_payload_requires_an_answer_and_canonical_bytes()
-> Result<(), Box<dyn std::error::Error>> {
    let mut panel = Panel {
        selected: true,
        ..Panel::default()
    };
    query(&mut panel).ok_or("query")?;
    assert!(!apply_result(
        &mut panel,
        &result(
            CommandStatus::Accepted,
            Reply::AnsweredAndLoggedOut(chosen()).encode()?
        )
    ));
    assert_eq!(panel.terminal_replies, TerminalReplies::Probing);
    assert!(!panel.logged_out);
    let mut panel = active_panel()?;
    answer(&mut panel, chosen()).ok_or("answer")?;
    let mut trailing = Reply::AnsweredAndLoggedOut(chosen()).encode()?;
    trailing.push(0);
    assert!(!apply_result(
        &mut panel,
        &result(CommandStatus::Accepted, trailing)
    ));
    assert!(!panel.logged_out);
    assert!(panel.active.is_some());
    Ok(())
}
