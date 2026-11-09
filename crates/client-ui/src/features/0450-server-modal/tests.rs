use super::*;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
#[test]
fn generic_modal_panel_drives_wire_and_clears_only_after_matching_ack()
-> Result<(), Box<dyn std::error::Error>> {
    let mut panel = Panel::default();
    assert!(query(&mut panel).is_none());
    panel.selected = true;
    assert_eq!(query(&mut panel).ok_or("query")?.kind, 736);
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
    assert!(answer(&mut panel, chosen).is_none());
    Ok(())
}
