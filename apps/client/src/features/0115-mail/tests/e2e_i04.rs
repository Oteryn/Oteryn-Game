//! I04 replay: server MAIL command replies applied by the production client reducer.
//!
//! The consumer is `apply` in `apps/client/src/features/0115-mail/protocol.rs`, reached through
//! `Outbox::properties_result` for capability 115. The request is built by the production
//! `command` function, so the reducer sees the exact kind and intent byte the app sends.
use super::*;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
use oteryn_client_ui::mail::Intent;

const WRITE_KIND: u32 = 124;

struct Replies {
    write_once: Vec<u8>,
    refused: Vec<u8>,
    too_long: Vec<u8>,
    rewrite_first: Vec<u8>,
    rewrite_second: Vec<u8>,
}

fn hex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .filter_map(|at| u8::from_str_radix(text.get(at * 2..at * 2 + 2)?, 16).ok())
        .collect()
}

fn replies() -> Result<Replies, Box<dyn std::error::Error>> {
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        let read = |name: &str| std::fs::read(path.join(name));
        let expected = std::fs::read_to_string(path.join("i04-expected.txt"))?;
        // The expected file lists the captured payloads once more as hex; they must agree.
        for (key, name) in [
            ("write_once", "i04-write-once.bin"),
            ("rewrite_refusal", "i04-write-once-refused.bin"),
            ("too_long", "i04-too-long.bin"),
            ("rewrite_first", "i04-rewrite-first.bin"),
            ("rewrite_second", "i04-rewrite-second.bin"),
        ] {
            let line = expected
                .lines()
                .find_map(|line| line.strip_prefix(&format!("{key}=")))
                .ok_or("expected capture line absent")?;
            assert_eq!(hex(line), read(name)?, "{key} capture disagrees");
        }
        return Ok(Replies {
            write_once: read("i04-write-once.bin")?,
            refused: read("i04-write-once-refused.bin")?,
            too_long: read("i04-too-long.bin")?,
            rewrite_first: read("i04-rewrite-first.bin")?,
            rewrite_second: read("i04-rewrite-second.bin")?,
        });
    }
    Ok(Replies {
        write_once: vec![2],
        refused: vec![9],
        too_long: vec![10],
        rewrite_first: vec![2],
        rewrite_second: vec![8],
    })
}

/// The text the player must see for a mail reply code, stated independently of the reducer.
fn visible(code: u8) -> &'static str {
    match code {
        2 => "Item text saved",
        8 => "Wait before writing again",
        9 => "This item cannot be written on",
        10 => "The letter text is too long",
        6 => "This item cannot be mailed",
        11 => "The server refused the mail request",
        _ => "unexpected reply code",
    }
}

fn pending_panel(text: &str) -> Panel {
    let mut panel = Panel {
        selected: true,
        open: true,
        pending: true,
        pending_kind: WRITE_KIND,
        ..Panel::default()
    };
    panel.fields = ["4242".into(), String::new(), text.into()];
    panel
}

fn result(status: CommandStatus, payload: &[u8]) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status,
        payload: payload.to_vec(),
    }
}

fn feedback_after(text: &str, status: CommandStatus, payload: &[u8]) -> Result<String, String> {
    let mut panel = pending_panel(text);
    let command = command(&panel, Intent::Write)?;
    assert_eq!((command.capability, command.kind), (115, WRITE_KIND));
    apply(&mut panel, &command, &result(status, payload));
    assert!(!panel.pending);
    Ok(panel.feedback)
}

#[test]
fn e2e_i04_replays_mail_write_replies() -> Result<(), Box<dyn std::error::Error>> {
    let replies = replies()?;
    for (text, payload) in [
        ("One inscription: żółw 🐢", &replies.write_once),
        ("first", &replies.rewrite_first),
    ] {
        assert_eq!(payload.as_slice(), [2]);
        assert_eq!(
            feedback_after(text, CommandStatus::Accepted, payload)?,
            "Item text saved"
        );
    }
    let [refused] = replies.refused.as_slice() else {
        return Err("refusal reply is not one byte".into());
    };
    assert_ne!(*refused, 2);
    assert_eq!(
        feedback_after("second", CommandStatus::Accepted, &replies.refused)?,
        visible(*refused)
    );
    assert_eq!(replies.too_long.as_slice(), [10]);
    assert_eq!(
        feedback_after("x", CommandStatus::Accepted, &replies.too_long)?,
        "The letter text is too long"
    );
    let [second] = replies.rewrite_second.as_slice() else {
        return Err("rewrite reply is not one byte".into());
    };
    assert_eq!(
        feedback_after("second", CommandStatus::Accepted, &replies.rewrite_second)?,
        visible(*second)
    );

    // A rejected command status never reads the payload as success.
    assert_eq!(
        feedback_after("first", CommandStatus::Rejected, &replies.write_once)?,
        "The server refused the mail request"
    );
    // A post-success byte on a write request, and an empty or long payload, are invalid replies.
    for payload in [&[1][..], &[], &[2, 2], &[99]] {
        assert_eq!(
            feedback_after("first", CommandStatus::Accepted, payload)?,
            "The server returned an invalid mail reply"
        );
    }

    // A reply for another request kind, or with nothing pending, changes nothing.
    let panel = pending_panel("first");
    let command = command(&panel, Intent::Write)?;
    let mut other_kind = Panel {
        pending_kind: WRITE_KIND + 1,
        ..panel.clone()
    };
    apply(&mut other_kind, &command, &result(CommandStatus::Accepted, &[2]));
    assert!(other_kind.pending && other_kind.feedback.is_empty());
    let mut idle = Panel {
        pending: false,
        ..panel
    };
    apply(&mut idle, &command, &result(CommandStatus::Accepted, &[2]));
    assert!(idle.feedback.is_empty());
    Ok(())
}
