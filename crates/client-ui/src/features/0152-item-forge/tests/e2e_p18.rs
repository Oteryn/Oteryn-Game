//! P18 replay: the server's Exaltation Forge JSON replies applied by the production client path.
//!
//! The consumer is `apply_result` in `crates/client-ui/src/features/0152-item-forge/protocol.rs`
//! (decoding through `reply::decode`), projected by `panel_text` in `view.rs`.
use super::*;
use crate::item_forge::panel_text;
use crate::item_forge::resource_dialog::Content;
use oteryn_client_runtime::cui3::item_forge::{COMMAND_TYPE_FORGE_DUST_QUERY, Operation};
use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};

fn value<'a>(text: &'a str, key: &str) -> Result<&'a str, Box<dyn std::error::Error>> {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected line {key} absent").into())
}

fn response(status: CommandStatus, payload: &[u8]) -> ServerMechanicResponse {
    ServerMechanicResponse {
        command_id: 1,
        status,
        server_sequence: 1,
        payload: payload.to_vec(),
    }
}

/// (history, [(name, reply bytes)], expected text).
type Replies = (Vec<u8>, Vec<(&'static str, Vec<u8>)>, String);

fn replies() -> Result<Replies, Box<dyn std::error::Error>> {
    let json = |text: &str| text.as_bytes().to_vec();
    Ok((
        json("[]"),
        vec![
            (
                "fusion",
                json(r#"{"operation":"fusion","succeeded":false,"applied":false,"reason":"WRONG_ITEMS","bonus":0,"result":null}"#),
            ),
            (
                "dust_to_slivers",
                json(r#"{"operation":"dust_to_slivers","succeeded":false,"applied":false,"reason":"INSUFFICIENT_DUST","bonus":0,"result":null}"#),
            ),
            (
                "dust_limit",
                json(r#"{"operation":"dust_limit","succeeded":true,"reason":null,"bonus":0,"result":{"dust":0,"dust_limit":100}}"#),
            ),
        ],
        "history_rows=0\nfusion.succeeded=false\nfusion.applied=false\nfusion.reason=WRONG_ITEMS\ndust_to_slivers.succeeded=false\ndust_to_slivers.applied=false\ndust_to_slivers.reason=INSUFFICIENT_DUST\ndust_limit.succeeded=true\ndust_limit.applied=true\ndust_limit.reason=\n".into(),
    ))
}

fn operation(name: &str) -> Result<Operation, Box<dyn std::error::Error>> {
    Ok(match name {
        "fusion" => Operation::Fusion,
        "dust_to_slivers" => Operation::DustToSlivers,
        "dust_limit" => Operation::DustLimit,
        _ => return Err(format!("unknown operation {name}").into()),
    })
}

struct ForgeEnvelope {
    command_id: u64,
    game_session_id: String,
    payload: Vec<u8>,
    server_sequence: u64,
    status: u32,
}

fn read_envelope(file: std::fs::File) -> Result<ForgeEnvelope, Box<dyn std::error::Error>> {
    let mut frame: serde_json::Map<String, serde_json::Value> = serde_json::from_reader(file)?;
    if frame.len() != 5 {
        return Err("Forge envelope must contain exactly five fields".into());
    }
    let mut take = |key| {
        frame
            .remove(key)
            .ok_or_else(|| format!("Forge envelope lacks {key}"))
    };
    Ok(ForgeEnvelope {
        command_id: serde_json::from_value(take("command_id")?)?,
        game_session_id: serde_json::from_value(take("game_session_id")?)?,
        payload: serde_json::from_value(take("payload")?)?,
        server_sequence: serde_json::from_value(take("server_sequence")?)?,
        status: serde_json::from_value(take("status")?)?,
    })
}

fn replay_live(root: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    type Submitted = (u64, u64, Vec<u8>);
    let file = std::fs::File::open(root.join("p18-requests.json"))?;
    let (session, requests): (String, Vec<Submitted>) = serde_json::from_reader(file)?;
    if session.len() != 36
        || !session
            .bytes()
            .enumerate()
            .all(|(index, byte)| match index {
                8 | 13 | 18 | 23 => byte == b'-',
                14 => byte == b'7',
                19 => matches!(byte, b'8' | b'9' | b'a' | b'b'),
                _ => byte.is_ascii_hexdigit(),
            })
        || requests.len() != 8
    {
        return Err("Forge replay requires eight frames from a live UUIDv7 session".into());
    }
    let mut panel = ItemForgePanel {
        selected: true,
        ..Default::default()
    };
    let mut previous = (0, 0);
    for (index, (command_id, server_sequence, submitted_bytes)) in requests.into_iter().enumerate()
    {
        let file = std::fs::File::open(root.join(format!("p18-frame-{index}.json")))?;
        let envelope = read_envelope(file)?;
        let status = match envelope.status {
            1 => CommandStatus::Accepted,
            2 => CommandStatus::Rejected,
            3 => CommandStatus::DuplicateReplay,
            4 => CommandStatus::DuplicateOutcomeExpired,
            _ => return Err(format!("unknown Forge reply status at frame {index}").into()),
        };
        if envelope.game_session_id != session
            || envelope.command_id != command_id
            || envelope.server_sequence != server_sequence
            || command_id <= previous.0
            || server_sequence <= previous.1
            || status != CommandStatus::Accepted
        {
            return Err(format!("invalid Forge reply metadata at frame {index}").into());
        }
        previous = (command_id, server_sequence);
        let payload = envelope.payload;
        let submitted = if index == 7 {
            if !submitted_bytes.is_empty() {
                return Err("invalid Forge dust query capture".into());
            }
            panel.query_current_dust = true;
            let query = control_request(&mut panel).ok_or("Forge UI refused dust query")?;
            if query.command_type != COMMAND_TYPE_FORGE_DUST_QUERY || !query.payload.is_empty() {
                return Err("Forge UI encoded an incorrect dust query".into());
            }
            None
        } else {
            let submitted =
                Request::decode(&submitted_bytes).ok_or("captured Forge request invalid")?;
            request(&mut panel, command(submitted_bytes)).ok_or("Forge UI refused submission")?;
            Some(submitted)
        };
        if !panel.pending || panel.submitted != submitted {
            return Err("Forge UI did not track the submitted request".into());
        }
        let previous_dialog = panel.resource_dialog.serial;
        apply_result(
            &mut panel,
            &ServerMechanicResponse {
                command_id,
                server_sequence,
                status,
                payload: payload.clone(),
            },
        );
        if panel.pending
            || panel.submitted.is_some()
            || panel.retry.is_some()
            || panel.feedback.is_some()
            || panel.response != payload
        {
            return Err("Forge panel did not consume the live reply".into());
        }
        let text = panel_text(&panel);
        let dialog = panel
            .resource_dialog
            .content
            .as_ref()
            .ok_or("Forge dialog missing")?;
        let dialog_text = |row| crate::item_forge::resource_dialog::dialog_text(dialog, row);
        if index == 7 {
            if panel.resource_dialog.serial == previous_dialog
                || dialog_text(0) != "Current dust: 15 / 101"
                || !text.lines().any(|line| line == "Current dust: 15 / 101")
            {
                return Err("Forge current dust dialog rendered incorrect resources".into());
            }
            continue;
        }
        if index == 6 {
            let Some(reply::Reply::History(rows)) = &panel.decoded else {
                return Err("populated Forge history did not decode".into());
            };
            let cap = dialog_text(0);
            let conversion = dialog_text(1);
            if panel.resource_dialog.serial == previous_dialog
                || rows.len() != 2
                || !cap.starts_with("History #2\n")
                || !cap.contains("Dust after operation: 15 / 101")
                || !conversion.starts_with("History #1\n")
                || !conversion.contains("Dust after operation: 40 / 100\nCreated: 3 slivers")
                || !text
                    .split_once("History #2\n")
                    .and_then(|(_, rows)| rows.split_once("History #1\n"))
                    .is_some_and(|(cap, conversion)| {
                        cap.contains("Dust: 15 / 101")
                            && conversion.contains("Dust: 40 / 100")
                            && conversion.contains("Created: 3 slivers")
                    })
            {
                return Err(
                    "populated Forge history rendered incorrect resources or entries".into(),
                );
            }
            continue;
        }
        for expected in match index {
            1 => &["Dust after operation: 40 / 100", "Created: 3 slivers"][..],
            2 => &["Dust after operation: 15 / 101"][..],
            _ => &[],
        } {
            if !dialog_text(0).lines().any(|line| line == *expected) {
                return Err(format!("Forge dialog missing exact resource line {expected}").into());
            }
        }
        let expected_resources = match index {
            1 => &["Dust: 40 / 100", "Created: 3 slivers, 0 cores"][..],
            2 => &["Dust: 15 / 101"][..],
            _ => &[],
        };
        if !expected_resources
            .iter()
            .all(|expected| text.lines().any(|line| line == *expected))
        {
            return Err(
                format!("Forge panel rendered incorrect resources at frame {index}").into(),
            );
        }
        let expected = match (index, &panel.decoded) {
            (0, Some(reply::Reply::History(rows))) if rows.is_empty() => "No forge operations yet",
            (1, Some(reply::Reply::Result(_))) => "DustToSlivers: succeeded; attempt applied",
            (2, Some(reply::Reply::Result(_))) => "DustLimit: succeeded; attempt applied",
            (3, Some(reply::Reply::Result(_))) => {
                "Fusion: failed; no changes applied (INSUFFICIENT_DUST)"
            }
            (4, Some(reply::Reply::Result(_))) => {
                "DustToSlivers: failed; no changes applied (INSUFFICIENT_DUST)"
            }
            (5, Some(reply::Reply::Result(_))) => {
                "DustLimit: failed; no changes applied (INSUFFICIENT_DUST)"
            }
            _ => return Err(format!("unexpected Forge panel decode at frame {index}").into()),
        };
        let dialog_matches = match (&panel.resource_dialog.content, &panel.decoded) {
            (
                Some(Content::Exaltation {
                    receipt,
                    target: None,
                    donor: None,
                }),
                Some(reply::Reply::Result(decoded)),
            ) if index == 3 => receipt == decoded,
            (Some(Content::Result(dialog)), Some(decoded)) if index != 3 => dialog == decoded,
            _ => false,
        };
        if panel.resource_dialog.serial == previous_dialog || !dialog_matches {
            return Err(
                format!("Forge dialog did not show the submitted result at frame {index}").into(),
            );
        }
        if panel_text(&panel).lines().nth(3) != Some(expected) {
            return Err(format!("unexpected Forge panel text at frame {index}").into());
        }
    }
    Ok(())
}

#[test]
fn e2e_p18_replays_forge_history_and_operation_receipts() -> Result<(), Box<dyn std::error::Error>>
{
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        return replay_live(std::path::Path::new(&root));
    }
    let (history, operations, expected) = replies()?;
    let mut panel = ItemForgePanel {
        selected: true,
        ..ItemForgePanel::default()
    };

    // HISTORY: the server's receipt count is exactly what the panel shows.
    let rows: usize = value(&expected, "history_rows")?.parse()?;
    panel.pending = true;
    apply_result(&mut panel, &response(CommandStatus::Accepted, &history));
    assert!(!panel.pending && panel.feedback.is_none());
    assert_eq!(panel.response, history);
    match &panel.decoded {
        Some(reply::Reply::History(entries)) => assert_eq!(entries.len(), rows),
        other => return Err(format!("history decoded as {other:?}").into()),
    }
    if rows == 0 {
        assert_eq!(
            panel_text(&panel).lines().nth(3),
            Some("No forge operations yet")
        );
    }

    for (name, bytes) in &operations {
        let succeeded = value(&expected, &format!("{name}.succeeded"))? == "true";
        let applied = value(&expected, &format!("{name}.applied"))? == "true";
        let reason = value(&expected, &format!("{name}.reason"))?;
        panel.pending = true;
        apply_result(&mut panel, &response(CommandStatus::Accepted, bytes));
        let Some(reply::Reply::Result(receipt)) = &panel.decoded else {
            return Err(format!("{name} did not decode to a receipt: {:?}", panel.decoded).into());
        };
        assert_eq!(receipt.operation, operation(name)?);
        assert_eq!(receipt.succeeded, succeeded);
        assert_eq!(receipt.applied, applied);
        assert_eq!(
            receipt.reason.as_deref().unwrap_or(""),
            reason,
            "{name} reason"
        );
        let header = format!(
            "{:?}: {}; {}{}",
            operation(name)?,
            if succeeded { "succeeded" } else { "failed" },
            if applied {
                "attempt applied"
            } else {
                "no changes applied"
            },
            if reason.is_empty() {
                String::new()
            } else {
                format!(" ({reason})")
            }
        );
        assert_eq!(panel_text(&panel).lines().nth(3), Some(header.as_str()));
    }

    // A rejected command and an undecodable accepted payload leave the last receipt unchanged.
    let kept = panel.decoded.clone();
    let (_, last) = operations.last().ok_or("no operation replies")?;
    panel.pending = true;
    apply_result(&mut panel, &response(CommandStatus::Rejected, last));
    assert!(!panel.pending);
    assert_eq!(panel.decoded, kept);
    assert_eq!(
        panel.feedback.as_deref(),
        Some("Server refused forge action: Rejected")
    );
    panel.pending = true;
    apply_result(
        &mut panel,
        &response(CommandStatus::Accepted, br#"{"operation":"fusion"}"#),
    );
    assert_eq!(panel.decoded, kept);
    assert_eq!(
        panel.feedback.as_deref(),
        Some("Invalid forge reply; refresh to retry")
    );
    Ok(())
}
