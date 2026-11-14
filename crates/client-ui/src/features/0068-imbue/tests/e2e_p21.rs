//! P21 replay: the server's IMBUEMENT query replies applied through the production `request`,
//! `apply_result` and `panel_text`.
//!
//! The recorded capability-491 selection chooses the same reply decoder as the client
//! transport `send`; success percentages come from the captured wire payload.
use super::*;
use oteryn_client_runtime::imbue::CommandStatus;
use std::num::NonZeroU64;

type Error = Box<dyn std::error::Error>;

fn value<'a>(text: &'a str, key: &str) -> Result<&'a str, Error> {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected line {key} absent").into())
}

/// (window reply, refused reply when the server sent a payload, expected text).
type Capture = (Vec<u8>, Option<Vec<u8>>, String);

fn load() -> Result<Capture, Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        let expected = std::fs::read_to_string(root.join("p21-expected.txt"))?;
        let refused = if value(&expected, "refused_status")? == "Accepted" {
            Some(std::fs::read(root.join("p21-refused.bin"))?)
        } else {
            None
        };
        return Ok((
            std::fs::read(root.join("p21-window.bin"))?,
            refused,
            expected,
        ));
    }
    let encode = |reply| imbuement::encode_imbuement_reply(&reply).map_err(|e| format!("{e:?}"));
    let window = imbuement::ImbuementReply {
        result: imbuement::ImbuementResult::Ok,
        window: Some(imbuement::ImbuementWindow {
            slot_count: 2,
            applied: vec![],
            offers: vec![
                imbuement::ImbuementOffer {
                    family: 1,
                    tier: 1,
                    gold: 5_000,
                    requires_premium: false,
                },
                imbuement::ImbuementOffer {
                    family: 4,
                    tier: 2,
                    gold: 25_000,
                    requires_premium: true,
                },
            ],
            clear_gold: 15_000,
        }),
    };
    let refused = imbuement::ImbuementReply {
        result: imbuement::ImbuementResult::NotAllowed,
        window: None,
    };
    Ok((
        encode(window)?,
        Some(encode(refused)?),
        "success_selected=false\nslot_count=2\nclear_gold=15000\napplied=0\noffers=2\noffer_0=1,1,5000,false\noffer_1=4,2,25000,true\nrefused_status=Accepted\nrefused_result=NotAllowed\n".into(),
    ))
}

fn response(
    status: CommandStatus,
    bytes: &[u8],
    success_selected: bool,
) -> Result<ImbueResponse, Error> {
    let accepted = matches!(
        status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    );
    let (reply, success) = if !accepted {
        (None, Vec::new())
    } else if success_selected {
        let (reply, success) =
            imbuement::decode_imbuement_reply_with_success(bytes).map_err(|e| format!("{e:?}"))?;
        (Some(reply), success)
    } else {
        (
            Some(imbuement::decode_imbuement_reply(bytes).map_err(|e| format!("{e:?}"))?),
            Vec::new(),
        )
    };
    Ok(ImbueResponse {
        status,
        reply,
        success,
    })
}

#[test]
fn e2e_p21_replays_imbuement_window_and_refusal() -> Result<(), Error> {
    let (window, refused, expected) = load()?;
    let success_selected: bool = value(&expected, "success_selected")?.parse()?;
    let mut panel = ImbuePanel {
        success_selected,
        selected: true,
        item: NonZeroU64::new(7),
        ..ImbuePanel::default()
    };
    let command = request(&mut panel, Intent::Refresh).ok_or("refresh request")?;
    assert_eq!(command.0.action, imbuement::ImbuementAction::Query);

    // The window: exact slot count, clear cost and every offer the server listed.
    apply_result(
        &mut panel,
        &response(CommandStatus::Accepted, &window, success_selected)?,
    );
    let shown = panel.window.clone().ok_or("window absent")?;
    let slots: u8 = value(&expected, "slot_count")?.parse()?;
    let clear: u32 = value(&expected, "clear_gold")?.parse()?;
    let offers: usize = value(&expected, "offers")?.parse()?;
    assert_eq!((shown.slot_count, shown.clear_gold), (slots, clear));
    assert_eq!(
        shown.applied.len(),
        value(&expected, "applied")?.parse::<usize>()?
    );
    assert_eq!(shown.offers.len(), offers);
    assert!(panel.feedback.is_none());
    let text = panel_text(&panel);
    assert_eq!(
        text.lines().next(),
        Some(format!("{slots} slots; clear cost: {clear} gold").as_str())
    );
    for (index, offer) in shown.offers.iter().enumerate() {
        let line = value(&expected, &format!("offer_{index}"))?;
        let parts: Vec<&str> = line.split(',').collect();
        let [family, tier, gold, premium] = parts.as_slice() else {
            return Err(format!("offer_{index} malformed: {line}").into());
        };
        assert_eq!(
            (offer.family.to_string(), offer.tier.to_string()),
            (family.to_string(), tier.to_string())
        );
        assert_eq!(offer.gold.to_string(), *gold);
        let name = family_name(offer.family);
        let suffix = if *premium == "true" { " (premium)" } else { "" };
        let success = if success_selected {
            let percent: u8 = value(&expected, &format!("offer_{index}_success"))?.parse()?;
            assert_eq!(panel.success.get(index), Some(&percent));
            format!(", {percent}% success")
        } else {
            assert!(panel.success.is_empty());
            String::new()
        };
        assert!(
            text.lines().any(|row| row
                == format!(
                    "Offer {index}: family {name}, tier {tier}, {gold} gold{suffix}{success}"
                )),
            "offer {index} row missing in {text}"
        );
    }

    // The refusal for an item that is not carried leaves the loaded window untouched.
    let before = panel.window.clone();
    panel.pending = true;
    let refusal = match &refused {
        Some(bytes) => response(CommandStatus::Accepted, bytes, success_selected)?,
        None => response(CommandStatus::Rejected, &[], success_selected)?,
    };
    apply_result(&mut panel, &refusal);
    assert_eq!(panel.window, before);
    let feedback = match value(&expected, "refused_result")? {
        "RequirementNotMet" => "Requirements are not met",
        "MaterialsMissing" => "Required materials are missing",
        "InsufficientFunds" => "Insufficient funds",
        "SlotOccupied" => "That slot is occupied",
        "NotAllowed" => "This imbuement is not allowed",
        "RevisionChanged" => "The item changed; refresh it",
        "None" => "The server refused the imbuement request",
        other => return Err(format!("unexpected refusal result {other}").into()),
    };
    assert_eq!(panel.feedback, Some(feedback));

    // A truncated payload is not a reply: the transport never reaches `apply_result`.
    assert!(
        response(
            CommandStatus::Accepted,
            &window[..window.len() - 1],
            success_selected
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn selected_success_decoder_preserves_server_percentages() -> Result<(), Error> {
    // Local codec regression only: these bytes are never written as a live capture.
    let reply = imbuement::ImbuementReply {
        result: imbuement::ImbuementResult::Ok,
        window: Some(imbuement::ImbuementWindow {
            slot_count: 1,
            applied: Vec::new(),
            offers: vec![imbuement::ImbuementOffer {
                family: 1,
                tier: 1,
                gold: 5_000,
                requires_premium: false,
            }],
            clear_gold: 15_000,
        }),
    };
    let bytes = imbuement::encode_imbuement_reply_with_success(&reply, &[42])
        .map_err(|error| format!("{error:?}"))?;
    let decoded = response(CommandStatus::Accepted, &bytes, true)?;
    assert_eq!(decoded.reply, Some(reply.clone()));
    assert_eq!(decoded.success, [42]);
    assert!(response(CommandStatus::Accepted, &bytes, false).is_err());
    let legacy = imbuement::encode_imbuement_reply(&reply).map_err(|error| format!("{error:?}"))?;
    let decoded = response(CommandStatus::Accepted, &legacy, false)?;
    assert_eq!(decoded.reply, Some(reply));
    assert!(decoded.success.is_empty());
    assert!(response(CommandStatus::Accepted, &legacy, true).is_err());
    Ok(())
}
