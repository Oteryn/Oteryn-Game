//! P18 replay: the server's Exaltation Forge JSON replies applied by the production client path.
//!
//! The consumer is `apply_result` in `crates/client-ui/src/features/0152-item-forge/protocol.rs`
//! (decoding through `reply::decode`), projected by `panel_text` in `view.rs`.
use super::*;
use crate::item_forge::panel_text;
use oteryn_client_runtime::cui3::item_forge::Operation;
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
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        let read = |name: &str| std::fs::read(root.join(name));
        return Ok((
            read("p18-history.bin")?,
            vec![
                ("fusion", read("p18-fusion.bin")?),
                ("dust_to_slivers", read("p18-dust-to-slivers.bin")?),
                ("dust_limit", read("p18-dust-limit.bin")?),
            ],
            std::fs::read_to_string(root.join("p18-expected.txt"))?,
        ));
    }
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

#[test]
fn e2e_p18_replays_forge_history_and_operation_receipts() -> Result<(), Box<dyn std::error::Error>>
{
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
