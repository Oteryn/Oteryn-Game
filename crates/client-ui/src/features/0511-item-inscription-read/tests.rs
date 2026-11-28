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
        ..Default::default()
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

#[test]
fn gap_0596_item_inscription_reduces_only_the_in_flight_attribution_read()
-> Result<(), &'static str> {
    let mut state = Inscription {
        selected: true,
        attribution_selected: true,
        ..Default::default()
    };
    let read = request(&mut state, 7).ok_or("read")?;
    assert_eq!(read.capability, CAPABILITY_ITEM_INSCRIPTION_ATTRIBUTION_V1);
    assert_eq!(read.kind, COMMAND_TYPE_ITEM_INSCRIPTION_ATTRIBUTION);

    let second = request(&mut state, 8).ok_or("second read")?;
    let date_only = result(
        CommandStatus::DuplicateReplay,
        br#"{"text":"Epitaph","max_length":20,"writable":false,"written_at_unix_ms":-1}"#.to_vec(),
    );
    assert!(apply_result(&mut state, &read, &date_only).is_none());
    assert_eq!(state.known, Known::Reading(8));
    assert!(apply_result(&mut state, &second, &date_only).is_some());
    assert_eq!(
        attribution_text(&state.known, 8).as_deref(),
        Some("1969-12-31 23:59:59 UTC")
    );
    assert!(attribution_text(&state.known, 7).is_none());

    let blank = request(&mut state, 7).ok_or("blank read")?;
    let accepted = result(
        CommandStatus::Accepted,
        br#"{"text":"","max_length":20,"writable":true}"#.to_vec(),
    );
    assert!(apply_result(&mut state, &blank, &accepted).is_some());
    assert!(attribution_text(&state.known, 7).is_none());
    assert_eq!(write_check(&state, 7, "new"), WriteCheck::Allowed);
    Ok(())
}
