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

fn check_paid_confirmation(
    index: usize,
    dialog: &crate::item_forge::resource_dialog::ResourceDialog,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::item_forge::resource_dialog::dialog_text;
    let (operation, resources, expected_text) = if index == 1 {
        (
            Operation::DustToSlivers,
            (true, 100, 40, 100, 100, 0, 3),
            "Convert dust to slivers?\n\nDust: 100 → 40\nDust limit: 100 → 100\nSlivers: 0 → 3\nCores: 0 → 0",
        )
    } else {
        (
            Operation::DustLimit,
            (false, 40, 15, 100, 120, 0, 0),
            "Raise the dust limit?\n\nDust: 40 → 15\nDust limit: 100 → 120",
        )
    };
    let Some(
        content @ Content::Confirm {
            operation: shown,
            quote: Some(quote),
        },
    ) = dialog.content.as_ref()
    else {
        return Err("Forge did not display the live confirmation".into());
    };
    if *shown != operation
        || quote.operation != operation
        || (
            quote.full_resources,
            quote.before.dust,
            quote.after_dust,
            quote.before.dust_limit,
            quote.after_limit,
            quote.before.slivers,
            quote.after_slivers,
        ) != resources
        || (quote.before.cores, quote.after_cores) != (0, 0)
        || dialog_text(content, 0)
            != format!(
                "{expected_text}\n\nThe server checks your current resources when applying the operation."
            )
    {
        return Err(format!("Forge displayed wrong resource confirmation {quote:?}").into());
    }
    Ok(())
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
        || requests.len() != 15
    {
        return Err("Forge replay requires fifteen frames from a live UUIDv7 session".into());
    }
    let mut panel = ItemForgePanel {
        selected: true,
        ..Default::default()
    };
    panel.preview.selected = true;
    let (id, sequence, payload) = &requests[14];
    let browse = read_envelope(std::fs::File::open(root.join("p18-frame-14.json"))?)?;
    let opening =
        crate::item_forge::begin_open(&mut panel).ok_or("Forge opening browse did not start")?;
    if browse.game_session_id != session
        || browse.command_id != *id
        || browse.server_sequence != *sequence
        || browse.status != 1
        || opening.payload != *payload
    {
        return Err("Forge opening browse did not reproduce the live request".into());
    }
    use oteryn_client_runtime::raw_feature;
    crate::item_forge::preview::apply(
        &mut panel,
        &opening,
        &raw_feature::GatedCommandResult {
            command_id: *id,
            server_sequence: *sequence,
            status: raw_feature::CommandStatus::Accepted,
            payload: browse.payload,
        },
    );
    if !panel.open || panel.preview.page.is_none() {
        return Err("Forge did not open with its captured browse page".into());
    }
    let mut previous = (*id, *sequence);
    for (index, (command_id, server_sequence, submitted_bytes)) in
        requests.iter().take(8).cloned().enumerate()
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
        if index == 3 {
            use oteryn_client_runtime::{forge_preview as wire, raw_feature};
            let (id, sequence, payload) = &requests[11];
            let browse = read_envelope(std::fs::File::open(root.join("p18-frame-11.json"))?)?;
            if browse.game_session_id != session
                || browse.command_id != *id
                || browse.server_sequence != *sequence
                || browse.status != 1
                || !(previous.0 < *id && *id < requests[10].0)
                || !(previous.1 < *sequence && *sequence < requests[10].1)
                || !panel.choose_operation(Operation::Fusion)
            {
                return Err("Fusion browse was not bound to the reviewed pair quote".into());
            }
            let request = preview_request(&mut panel, wire::Selection::Browse { after: 0 })
                .ok_or("Forge picker browse did not start")?;
            if request.payload != *payload {
                return Err("Forge picker browse did not reproduce the live request".into());
            }
            crate::item_forge::preview::apply(
                &mut panel,
                &request,
                &raw_feature::GatedCommandResult {
                    command_id: browse.command_id,
                    server_sequence: browse.server_sequence,
                    status: raw_feature::CommandStatus::Accepted,
                    payload: browse.payload,
                },
            );
            previous = (*id, *sequence);
        }
        if matches!(index, 1..=5) {
            use oteryn_client_runtime::{forge_preview as wire, raw_feature};
            let quote_index = if index <= 3 { index + 7 } else { index + 8 };
            let (quote_id, quote_sequence, quote_request) = &requests[quote_index];
            let quote = read_envelope(std::fs::File::open(
                root.join(format!("p18-frame-{quote_index}.json")),
            )?)?;
            if quote.game_session_id != session
                || quote.command_id != *quote_id
                || quote.server_sequence != *quote_sequence
                || quote.status != 1
                || !(previous.0 < *quote_id && *quote_id < command_id)
                || !(previous.1 < *quote_sequence && *quote_sequence < server_sequence)
            {
                return Err(
                    "Forge review was not bound to its original command and session".into(),
                );
            }
            let submitted =
                Request::decode(&submitted_bytes).ok_or("captured Forge request invalid")?;
            let mut browsed_pair = None;
            let selection = if index == 3 {
                let first = submitted.first_item.ok_or("Fusion receiver absent")?;
                let second = submitted.second_item.ok_or("Fusion donor absent")?;
                let browse_item = |handle| {
                    panel
                        .preview
                        .item(handle)
                        .filter(|item| {
                            item.name == "dagger"
                                && item.appearance == 3267
                                && item.classification == 1
                                && item.tier == 0
                                && item.maximum_tier == 1
                                && !item.container
                        })
                        .cloned()
                        .ok_or("Forge picker rendered incorrect browse metadata")
                };
                browsed_pair = Some((browse_item(first.get())?, browse_item(second.get())?));
                if !panel.choose_item(true, first.get()) || !panel.choose_item(false, second.get())
                {
                    return Err("Forge picker could not select the captured daggers".into());
                }
                if panel.paid_request(submitted.occurrence).is_some() {
                    return Err("Fusion replay submitted before review".into());
                }
                panel.selection().ok_or("Fusion pair selection missing")?
            } else {
                wire::Selection::Resource(submitted.operation)
            };
            let request =
                preview_request(&mut panel, selection).ok_or("Forge UI refused preview request")?;
            if request.payload != *quote_request
                || (index != 3
                    && panel
                        .confirm_resource(panel.resource_dialog.serial, submitted.occurrence)
                        .is_some())
            {
                return Err("Forge UI did not reproduce the captured preview request".into());
            }
            crate::item_forge::preview::apply(
                &mut panel,
                &request,
                &raw_feature::GatedCommandResult {
                    command_id: quote.command_id,
                    server_sequence: quote.server_sequence,
                    status: raw_feature::CommandStatus::Accepted,
                    payload: quote.payload,
                },
            );
            if index == 3 {
                let expected = concat!(
                    "Receiver: dagger — class 1, tier 0 / 1\n",
                    "Source / tier donor: dagger — class 1, tier 0 / 1\n",
                    "Server-verified pair: target tier 1\n",
                    "Base cost before outcome bonuses: 25000 gold, 100 dust, 0 cores\n",
                    "Success: 50%; tier-loss protection: 0%\n",
                    "Available: 30050 carried gold; 0 bank gold; 15 / 120 dust; 0 cores; 3 slivers\n",
                    "The original server writer independently rechecks the current state."
                );
                let quote = panel
                    .preview
                    .quote
                    .as_ref()
                    .ok_or("Fusion quote missing after review")?;
                let (first, second) = browsed_pair.ok_or("Fusion browse pair missing")?;
                if quote.first != first || quote.second != second {
                    return Err("Fusion quote changed the picker item metadata".into());
                }
                if crate::item_forge::selected_pair_text(&panel) != expected
                    || panel.paid_request(submitted.occurrence) != Some(submitted)
                {
                    return Err(
                        "Fusion review did not render or submit the exact live quote".into(),
                    );
                }
            } else if index >= 4 {
                let dialog = panel
                    .resource_dialog
                    .content
                    .as_ref()
                    .ok_or("resource refusal dialog missing")?;
                if crate::item_forge::resource_dialog::dialog_text(dialog, 0)
                    != "This resource conversion is unavailable with your current resources or dust limit."
                    || panel
                        .confirm_resource(panel.resource_dialog.serial, submitted.occurrence)
                        .is_some()
                {
                    return Err("Underfunded resource preview showed an incorrect refusal or allowed confirmation".into());
                }
            } else {
                check_paid_confirmation(index, &panel.resource_dialog)?;
                let serial = panel.resource_dialog.serial;
                let confirmed = panel
                    .confirm_resource(serial, submitted.occurrence)
                    .ok_or("Forge UI refused the displayed confirmation")?;
                if confirmed != submitted
                    || panel
                        .confirm_resource(serial, submitted.occurrence)
                        .is_some()
                {
                    return Err(
                        "Forge confirmation submitted a different or duplicate action".into(),
                    );
                }
            }
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
            let payload = if index == 3 {
                let reviewed = panel
                    .paid_request(submitted.occurrence)
                    .and_then(|request| request.encode())
                    .ok_or("reviewed Fusion request missing")?;
                if reviewed.as_slice() != submitted_bytes {
                    return Err("reviewed Fusion request differs from the submitted frame".into());
                }
                reviewed.to_vec()
            } else {
                submitted_bytes
            };
            let outgoing =
                request(&mut panel, command(payload)).ok_or("Forge UI refused submission")?;
            if (outgoing.capability, outgoing.command_type)
                != (
                    oteryn_client_runtime::cui3::item_forge::CAPABILITY_ITEM_FORGE_V1,
                    oteryn_client_runtime::cui3::item_forge::COMMAND_TYPE_ITEM_FORGE,
                )
            {
                return Err("Forge UI produced incorrect command routing".into());
            }
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
                || dialog_text(0) != "Current dust: 15 / 120"
                || !text.lines().any(|line| line == "Current dust: 15 / 120")
            {
                return Err("Forge current dust dialog rendered incorrect resources".into());
            }
            continue;
        }
        const CAP_COMPLETED: &str =
            "Raise the dust limit: Completed\nDust after operation: 15 / 120";
        const CONVERSION_COMPLETED: &str = "Convert dust to slivers: Completed\nDust after operation: 40 / 100\nCreated: 3 slivers";
        if index == 6 {
            let Some(reply::Reply::History(rows)) = &panel.decoded else {
                return Err("populated Forge history did not decode".into());
            };
            let cap = dialog_text(0);
            let conversion = dialog_text(1);
            if panel.resource_dialog.serial == previous_dialog
                || rows.len() != 2
                || cap != format!("History #2\n{CAP_COMPLETED}")
                || conversion != format!("History #1\n{CONVERSION_COMPLETED}")
                || !text
                    .split_once("History #2\n")
                    .and_then(|(_, rows)| rows.split_once("History #1\n"))
                    .is_some_and(|(cap, conversion)| {
                        cap.contains("Dust: 15 / 120")
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
        let completed = match index {
            1 => Some(CONVERSION_COMPLETED),
            2 => Some(CAP_COMPLETED),
            _ => None,
        };
        if completed.is_some_and(|expected| dialog_text(0) != expected) {
            return Err(
                format!("Forge dialog rendered an incorrect result at frame {index}").into(),
            );
        }
        let expected_resources = match index {
            1 => &["Dust: 40 / 100", "Created: 3 slivers, 0 cores"][..],
            2 => &["Dust: 15 / 120"][..],
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
        if index == 0 && dialog_text(0) != "No Forge operations recorded" {
            return Err("Empty Forge history dialog rendered incorrect text".into());
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
        let refusal_dialog = match index {
            3 => Some("Fuse item pair\nNot applied\nServer reason: INSUFFICIENT_DUST"),
            4 => Some("Convert dust to slivers: Not applied\nNot enough dust"),
            5 => Some("Raise the dust limit: Not applied\nNot enough dust"),
            _ => None,
        };
        if refusal_dialog.is_some_and(|expected| dialog_text(0) != expected) {
            return Err(
                format!("Forge refusal dialog rendered incorrect text at frame {index}").into(),
            );
        }
        if index == 5 {
            check_invalid_results(&mut panel.clone(), &payload);
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

    let (_, last) = operations.last().ok_or("no operation replies")?;
    check_invalid_results(&mut panel, last);
    Ok(())
}

fn check_invalid_results(panel: &mut ItemForgePanel, last: &[u8]) {
    // A rejected command and an undecodable accepted payload leave the last receipt unchanged.
    let kept = panel.decoded.clone();
    panel.pending = true;
    apply_result(panel, &response(CommandStatus::Rejected, last));
    assert!(!panel.pending);
    assert_eq!(panel.decoded, kept);
    assert_eq!(
        panel.feedback.as_deref(),
        Some("Server refused forge action: Rejected")
    );
    panel.pending = true;
    apply_result(
        panel,
        &response(CommandStatus::Accepted, br#"{"operation":"fusion"}"#),
    );
    assert_eq!(panel.decoded, kept);
    assert_eq!(
        panel.feedback.as_deref(),
        Some("Invalid forge reply; refresh to retry")
    );
}

#[test]
fn p18_confirmation_rejects_unexpected_core_creation() -> Result<(), Box<dyn std::error::Error>> {
    use crate::item_forge::resource_dialog::ResourceDialog;
    use oteryn_client_runtime::forge_preview::{ResourceQuote, Resources};

    let mut dialog = ResourceDialog::default();
    let mut quote = ResourceQuote {
        operation: Operation::DustToSlivers,
        full_resources: true,
        before: Resources {
            dust: 100,
            dust_limit: 100,
            slivers: 0,
            cores: 0,
            ..Resources::default()
        },
        after_dust: 40,
        after_limit: 100,
        after_slivers: 3,
        after_cores: 0,
    };
    dialog.offer(quote.operation, Some(quote));
    check_paid_confirmation(1, &dialog)?;
    quote.after_cores = 1;
    dialog.offer(quote.operation, Some(quote));
    assert!(check_paid_confirmation(1, &dialog).is_err());
    Ok(())
}
