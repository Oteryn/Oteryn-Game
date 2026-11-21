use super::*;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};

fn result(status: CommandStatus, payload: Vec<u8>) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status,
        payload,
    }
}

#[test]
fn a_write_is_sent_only_for_the_read_writable_text_of_the_same_item() -> Result<(), &'static str> {
    let mut state = Inscription {
        selected: true,
        ..Default::default()
    };
    assert_eq!(write_check(&state, 7, "new"), WriteCheck::Read);
    let read = request(&mut state, 7).ok_or("read")?;
    assert_eq!(read.kind, COMMAND_TYPE_ITEM_INSCRIPTION_READ);
    assert_eq!(
        Request::decode(&read.payload),
        Some(Request { item_handle: 7 })
    );
    let reply = Reply {
        text: "Old inscription".into(),
        max_length: 20,
        writable: true,
    };
    let accepted = result(CommandStatus::Accepted, reply.encode().ok_or("encode")?);
    assert_eq!(apply_result(&mut state, &read, &accepted), Some(reply));
    assert!(
        apply_result(&mut state, &read, &accepted).is_none(),
        "one reply per read"
    );
    assert_eq!(write_check(&state, 7, ""), WriteCheck::Allowed);
    assert_eq!(
        write_check(&state, 7, "Old inscription."),
        WriteCheck::Allowed
    );
    assert!(matches!(
        write_check(&state, 7, &"x".repeat(21)),
        WriteCheck::Refused(_)
    ));
    assert_eq!(write_check(&state, 8, "other item"), WriteCheck::Read);

    let read_only = Reply {
        writable: false,
        ..Reply {
            text: "Fixed".into(),
            max_length: 20,
            writable: true,
        }
    };
    let read = request(&mut state, 8).ok_or("read")?;
    let accepted = result(CommandStatus::Accepted, read_only.encode().ok_or("encode")?);
    assert!(apply_result(&mut state, &read, &accepted).is_some());
    assert!(matches!(
        write_check(&state, 8, "Fixed"),
        WriteCheck::Refused(_)
    ));

    let read = request(&mut state, 9).ok_or("read")?;
    assert!(apply_result(&mut state, &read, &result(CommandStatus::Rejected, vec![])).is_none());
    assert_eq!(state.known, Known::Refused(9));
    assert!(matches!(
        write_check(&state, 9, "text"),
        WriteCheck::Refused(_)
    ));
    Ok(())
}

#[test]
fn without_the_capability_no_read_is_sent_and_an_empty_write_is_refused() {
    let mut state = Inscription::default();
    assert!(request(&mut state, 7).is_none());
    assert!(matches!(write_check(&state, 7, ""), WriteCheck::Refused(_)));
    assert_eq!(write_check(&state, 7, "typed text"), WriteCheck::Allowed);
}

#[test]
fn an_unshown_too_long_text_blocks_only_the_same_items_write() {
    let state = Inscription {
        selected: true,
        known: Known::TooLong(7),
    };
    assert_eq!(
        write_check(&state, 7, "replacement"),
        WriteCheck::Refused("The letter text is too long")
    );
    assert_eq!(
        write_check(&state, 7, ""),
        WriteCheck::Refused("The letter text is too long")
    );
    assert_eq!(write_check(&state, 8, "other item"), WriteCheck::Read);
}
