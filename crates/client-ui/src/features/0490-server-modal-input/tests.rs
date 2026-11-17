use super::*;
use crate::server_modal::{Dialog, Panel, answer, answer_with, apply_result};
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
use oteryn_client_runtime::server_modal::input::encode_dialog_with_input;

fn dialog() -> Dialog {
    Dialog {
        id: 5,
        title: "Amount".into(),
        message: "How many?".into(),
        choices: Vec::new(),
        buttons: vec![(1, "Ok".into()), (2, "Cancel".into())],
        cancel_button: Some(2),
        image: None,
    }
}
fn amount() -> DialogInput {
    DialogInput {
        kind: InputKind::Amount,
        minimum: 1,
        maximum: 50,
        initial: Some(InputValue::Amount(10)),
    }
}
fn result(payload: Vec<u8>) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload,
    }
}
fn open(input: bool, reply: Vec<u8>) -> Panel {
    let mut panel = Panel {
        selected: true,
        input,
        pending: Some(crate::server_modal::Pending::Query),
        ..Panel::default()
    };
    assert!(apply_result(&mut panel, &result(reply)));
    panel
}
fn press(button_id: u8) -> Answer {
    Answer {
        dialog_id: 5,
        button_id,
        choice_id: None,
    }
}

#[test]
fn c0490_selected_dialog_exposes_its_input_and_answers_with_the_value() -> Result<(), &'static str>
{
    let reply = encode_dialog_with_input(&dialog(), Some(&amount()))?;
    let mut panel = open(true, reply);
    assert_eq!(panel.active_input, Some(amount()));
    assert_eq!(prompt(&amount()), "Amount (1 to 50)");
    assert_eq!(initial_text(&amount()), "10");
    assert_eq!(parse_value(&amount(), " 7 "), Some(InputValue::Amount(7)));
    assert_eq!(parse_value(&amount(), "51"), None);
    assert_eq!(parse_value(&amount(), "many"), None);
    let entry = Entry {
        value: Some(InputValue::Amount(7)),
        dont_show_again: true,
    };
    let command = answer_with(&mut panel, press(1), &entry).ok_or("answer")?;
    let sent = InputAnswer::decode(&command.payload)?;
    assert_eq!(sent.answer, press(1));
    assert_eq!(sent.value, Some(InputValue::Amount(7)));
    assert!(sent.dont_show_again);
    Ok(())
}

#[test]
fn c0490_an_unacceptable_or_missing_value_sends_nothing_but_cancel_still_does()
-> Result<(), &'static str> {
    let reply = encode_dialog_with_input(&dialog(), Some(&amount()))?;
    let mut panel = open(true, reply);
    let too_big = Entry {
        value: Some(InputValue::Amount(99)),
        dont_show_again: false,
    };
    assert!(answer_with(&mut panel, press(1), &too_big).is_none());
    assert!(answer(&mut panel, press(1)).is_none());
    assert!(panel.pending.is_none());
    let cancel = Entry {
        value: Some(InputValue::Amount(3)),
        dont_show_again: true,
    };
    let command = answer_with(&mut panel, press(2), &cancel).ok_or("cancel")?;
    let sent = InputAnswer::decode(&command.payload)?;
    assert_eq!((sent.value, sent.dont_show_again), (None, false));
    Ok(())
}

#[test]
fn c0490_selected_dialog_without_input_still_answers_with_the_extended_bytes()
-> Result<(), &'static str> {
    let mut panel = open(true, encode_dialog_with_input(&dialog(), None)?);
    assert_eq!(panel.active_input, None);
    let command = answer(&mut panel, press(1)).ok_or("answer")?;
    assert_eq!(
        command.payload,
        press(1)
            .encode()?
            .into_iter()
            .chain([0, 0])
            .collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn c0490_unselected_session_keeps_the_v1_dialog_and_answer() -> Result<(), &'static str> {
    let v1 = Reply::Dialog(dialog()).encode()?;
    let mut panel = open(false, v1.clone());
    assert_eq!(panel.active_input, None);
    let entry = Entry {
        value: Some(InputValue::Amount(7)),
        dont_show_again: true,
    };
    let command = answer_with(&mut panel, press(1), &entry).ok_or("answer")?;
    assert_eq!(command.payload, press(1).encode()?);
    // An input block is never read without the capability, nor missing with it.
    let with_input = encode_dialog_with_input(&dialog(), Some(&amount()))?;
    assert!(decode_reply(false, &with_input).is_err());
    assert!(decode_reply(true, &v1).is_err());
    Ok(())
}
