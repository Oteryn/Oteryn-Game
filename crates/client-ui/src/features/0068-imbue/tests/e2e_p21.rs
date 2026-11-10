//! P21 replay: the server's IMBUEMENT query replies applied through the production `request`,
//! `apply_result` and `panel_text`.
//!
//! The reply bytes are decoded by `imbuement::decode_imbuement_reply`, exactly as the client
//! transport `send` in `crates/client-runtime/src/features/0068-imbue/protocol.rs` does before it
//! hands the reply to `apply_result`.
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
        "slot_count=2\nclear_gold=15000\napplied=0\noffers=2\noffer_0=1,1,5000,false\noffer_1=4,2,25000,true\nrefused_status=Accepted\nrefused_result=NotAllowed\n".into(),
    ))
}

fn response(status: CommandStatus, bytes: &[u8]) -> Result<ImbueResponse, Error> {
    let accepted = matches!(
        status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    );
    Ok(ImbueResponse {
        status,
        reply: if accepted {
            Some(imbuement::decode_imbuement_reply(bytes).map_err(|e| format!("{e:?}"))?)
        } else {
            None
        },
    })
}

#[test]
fn e2e_p21_replays_imbuement_window_and_refusal() -> Result<(), Error> {
    let (window, refused, expected) = load()?;
    let mut panel = ImbuePanel {
        selected: true,
        item: NonZeroU64::new(7),
        ..ImbuePanel::default()
    };
    let command = request(&mut panel, Intent::Refresh).ok_or("refresh request")?;
    assert_eq!(command.0.action, imbuement::ImbuementAction::Query);

    // The window: exact slot count, clear cost and every offer the server listed.
    apply_result(&mut panel, &response(CommandStatus::Accepted, &window)?);
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
    for index in 0..offers {
        let line = value(&expected, &format!("offer_{index}"))?;
        let parts: Vec<&str> = line.split(',').collect();
        let [family, tier, gold, premium] = parts.as_slice() else {
            return Err(format!("offer_{index} malformed: {line}").into());
        };
        assert_eq!(
            (
                shown.offers[index].family.to_string(),
                shown.offers[index].tier.to_string()
            ),
            (family.to_string(), tier.to_string())
        );
        assert_eq!(shown.offers[index].gold.to_string(), *gold);
        let name = family_name(shown.offers[index].family);
        let suffix = if *premium == "true" { " (premium)" } else { "" };
        assert!(
            text.lines().any(|row| row
                == format!("Offer {index}: family {name}, tier {tier}, {gold} gold{suffix}")),
            "offer {index} row missing in {text}"
        );
    }

    // The refusal for an item that is not carried leaves the loaded window untouched.
    let before = panel.window.clone();
    panel.pending = true;
    let refusal = match &refused {
        Some(bytes) => response(CommandStatus::Accepted, bytes)?,
        None => response(CommandStatus::Rejected, &[])?,
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
    assert!(response(CommandStatus::Accepted, &window[..window.len() - 1]).is_err());
    Ok(())
}
