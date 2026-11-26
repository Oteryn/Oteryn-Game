//! P04 replay: server TASK_BOARD replies (Query, then a refused StartWeekly) applied through the
//! production `full_request`, `apply_full_result` and `panel_text`.
use super::*;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
use task_board::full::{Bounty, Reply, Weekly, decode_request, encode_reply};

type Error = Box<dyn std::error::Error>;

struct Captured {
    query: GatedCommandResult,
    refused: GatedCommandResult,
    readmission: GatedCommandResult,
    expected: String,
}

fn fixture() -> Result<Captured, Error> {
    let weekly = Weekly {
        started: false,
        difficulty: 0,
        expanded: false,
        multiplier: 1,
        kill_experience: 0,
        delivery_experience: 0,
        pending_points: 0,
        pending_soulseals: 0,
        kills: Vec::new(),
        deliveries: Vec::new(),
    };
    let bounty = Bounty {
        elite: false,
        state: 0,
        difficulty: 0,
        selected: 0,
        current: 0,
        rerolls: 0,
        daily_reroll_at: 0,
        offers: Vec::new(),
        active: None,
        lists: Vec::new(),
    };
    let shop = vec![task_board::full::ShopOffer {
        index: 0,
        kind: 2,
        price: 100_000,
        primary: 1_907,
        secondary: 0,
        addon: 0,
        bought: false,
    }];
    let reply = |refusal: u8| Reply {
        weekly_results: None,
        refusal,
        period: 2_913,
        points: 40,
        bounty_points: 7,
        soulseals: 2,
        weekly: weekly.clone(),
        bounty: bounty.clone(),
        talisman: Default::default(),
        shop: shop.clone(),
    };
    let encode = |reply: &Reply| encode_reply(reply).map_err(|e| format!("{e:?}"));
    Ok(Captured {
        query: result(CommandStatus::Accepted, &encode(&reply(0))?),
        refused: result(CommandStatus::Accepted, &encode(&reply(3))?),
        readmission: result(CommandStatus::Accepted, &encode(&reply(0))?),
        expected: "period=2913\npoints=40\nbounty_points=7\nsoulseals=2\nstarted=false\nkills=0\ndeliveries=0\nshop_count=1\nshop_first=0,2,100000\nrefusal=3\n".into(),
    })
}

fn captured() -> Result<Captured, Error> {
    let path =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("live capture directory is required")?;
    let path = std::path::Path::new(&path);
    let binding = std::env::var("E2E_P04_BINDING")?;
    let mut fields = binding.split(';');
    let mut field = |key: &str| -> Result<&str, Error> {
        fields
            .next()
            .and_then(|entry| entry.split_once('='))
            .filter(|(name, _)| *name == key)
            .map(|(_, value)| value)
            .ok_or_else(|| format!("missing replay binding {key}").into())
    };
    let session = field("session")?;
    let after_session = field("readmission")?;
    if !is_uuid7(session) || !is_uuid7(after_session) || session == after_session {
        return Err("P04 requires two distinct UUIDv7 sessions".into());
    }
    let query = read_live_result(&path.join("p04-query.json"), session, field("query")?)?;
    let refused = read_live_result(&path.join("p04-refused.json"), session, field("refused")?)?;
    let readmission = read_live_result(
        &path.join("p04-readmission.json"),
        after_session,
        field("after")?,
    )?;
    assert_eq!(fields.next(), None);
    assert_eq!(query.command_id.checked_add(1), Some(refused.command_id));
    assert!(refused.server_sequence > query.server_sequence);
    for frame in [&query, &refused, &readmission] {
        assert_eq!(frame.status, CommandStatus::Accepted);
    }
    Ok(Captured {
        query,
        refused,
        readmission,
        expected: std::fs::read_to_string(path.join("p04-expected.txt"))?,
    })
}

fn is_uuid7(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 36
        && bytes[14] == b'7'
        && matches!(bytes[19], b'8' | b'9' | b'a' | b'b')
        && bytes.iter().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                *byte == b'-'
            } else {
                matches!(*byte, b'0'..=b'9' | b'a'..=b'f')
            }
        })
}

fn read_live_result(
    path: &std::path::Path,
    session: &str,
    frame: &str,
) -> Result<GatedCommandResult, Error> {
    let (command_id, server_sequence) = frame.split_once(',').ok_or("missing frame binding")?;
    parse_live_result(
        &std::fs::read(path)?,
        session,
        command_id.parse()?,
        server_sequence.parse()?,
    )
}

fn parse_live_result(
    bytes: &[u8],
    session: &str,
    command_id: u64,
    server_sequence: u64,
) -> Result<GatedCommandResult, Error> {
    let envelope: serde_json::Value = serde_json::from_slice(bytes)?;
    if envelope.as_object().is_none_or(|object| object.len() != 5)
        || envelope["game_session_id"].as_str() != Some(session)
        || command_id == 0
        || envelope["command_id"].as_u64() != Some(command_id)
        || server_sequence == 0
        || envelope["server_sequence"].as_u64() != Some(server_sequence)
    {
        return Err("P04 frame did not match the live session, command and sequence".into());
    }
    let status = match envelope["status"].as_u64() {
        Some(1) => CommandStatus::Accepted,
        Some(2) => CommandStatus::Rejected,
        Some(3) => CommandStatus::DuplicateReplay,
        Some(4) => CommandStatus::DuplicateOutcomeExpired,
        _ => return Err("P04 frame has unknown command status".into()),
    };
    Ok(GatedCommandResult {
        command_id,
        status,
        server_sequence,
        payload: serde_json::from_value(envelope["payload"].clone())?,
    })
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected capture line {key} absent").into())
}

fn result(status: CommandStatus, payload: &[u8]) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        status,
        server_sequence: 1,
        payload: payload.to_vec(),
    }
}

#[test]
#[ignore = "requires the live node-boot capture"]
fn e2e_p04_replays_query_and_refused_start_weekly() -> Result<(), Error> {
    verify_replay(captured()?)
}

fn verify_route(cmd: &oteryn_client_runtime::raw_feature::FeatureCommand) {
    assert_eq!((cmd.capability, cmd.kind), (72, 89));
}

fn verify_replay(captured: Captured) -> Result<(), Error> {
    let e = captured.expected.as_str();
    let mut panel = TaskBoardPanel {
        selected: true,
        ..TaskBoardPanel::default()
    };
    let query = full_request(&mut panel, FullIntent::Refresh).ok_or("query request")?;
    verify_route(&query);
    assert!(panel.pending);
    assert_eq!(
        decode_request(&query.payload).map_err(|e| format!("{e:?}"))?,
        task_board::full::Request::query()
    );
    apply_full_result(&mut panel, &captured.query);
    assert!(!panel.pending);
    assert_eq!(panel.feedback, None);

    let text = panel_text(&panel);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines.first().copied(),
        Some(format!("Task Board — week {}", value(e, "period")?).as_str())
    );
    assert_eq!(
        lines.get(1).copied(),
        Some(
            format!(
                "Points: {}  Bounty points: {}  Soulseals: {}",
                value(e, "points")?,
                value(e, "bounty_points")?,
                value(e, "soulseals")?
            )
            .as_str()
        )
    );
    let weekly = panel.weekly.clone().ok_or("weekly state")?;
    assert_eq!(weekly.weekly_results, None);
    assert_eq!(weekly.weekly.started.to_string(), value(e, "started")?);
    assert_eq!(weekly.weekly.kills.len().to_string(), value(e, "kills")?);
    assert_eq!(
        weekly.weekly.deliveries.len().to_string(),
        value(e, "deliveries")?
    );
    assert_eq!(weekly.shop.len().to_string(), value(e, "shop_count")?);
    let first: Vec<&str> = value(e, "shop_first")?.split(',').collect();
    let offer = weekly.shop.first().ok_or("shop offer")?;
    assert_eq!(
        [
            offer.index.to_string(),
            offer.kind.to_string(),
            offer.price.to_string()
        ],
        [first[0], first[1], first[2]]
    );
    assert_eq!(
        (offer.primary, offer.secondary, offer.addon, offer.bought),
        (1907, 0, 0, false)
    );
    let shop_header = "Shop 1 — Feral Trapper (Base Outfit)";
    let reward = "  Reward 1907 / 0 (addon 0)";
    let affordability = if offer.bought {
        "Already owned".to_owned()
    } else if weekly.points >= u64::from(offer.price) {
        "Available — server confirms purchase".to_owned()
    } else {
        format!("{} points short", u64::from(offer.price) - weekly.points)
    };
    let price_line = format!("  {} task points — {affordability}", first[2]);
    let at = lines
        .iter()
        .position(|line| *line == shop_header)
        .ok_or("shop header line")?;
    assert_eq!(lines.get(at + 1).copied(), Some(reward));
    assert_eq!(lines.get(at + 2).copied(), Some(price_line.as_str()));

    // StartWeekly above the character’s level is refused and the weekly state stays the same.
    let start = full_request(&mut panel, FullIntent::StartWeekly(3)).ok_or("start request")?;
    verify_route(&start);
    let sent = decode_request(&start.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!((sent.action, sent.selection), (5, 3));
    apply_full_result(&mut panel, &captured.refused);
    assert_eq!(
        panel.feedback,
        Some("The server refused: level too low for this difficulty")
    );
    let refused = panel.weekly.clone().ok_or("weekly state")?;
    assert_eq!(value(e, "refusal")?, "3");
    assert_eq!(refused.refusal.to_string(), value(e, "refusal")?);
    let mut unchanged = refused.clone();
    unchanged.refusal = 0;
    assert_eq!(unchanged, weekly);
    assert!(panel_text(&panel).ends_with("The server refused: level too low for this difficulty"));

    let mut fresh = TaskBoardPanel {
        selected: true,
        ..TaskBoardPanel::default()
    };
    let readmission =
        full_request(&mut fresh, FullIntent::Refresh).ok_or("readmission query request")?;
    verify_route(&readmission);
    assert_eq!(
        decode_request(&readmission.payload).map_err(|e| format!("{e:?}"))?,
        task_board::full::Request::query()
    );
    apply_full_result(&mut fresh, &captured.readmission);
    assert!(!fresh.pending);
    assert_eq!(fresh.feedback, None);
    assert_eq!(fresh.weekly.as_ref(), Some(&weekly));

    assert_invalid_results(&mut panel, &refused)
}

#[test]
fn p04_unit_projection_checks_rendering_and_invalid_results() -> Result<(), Error> {
    verify_replay(fixture()?)
}

#[test]
fn p04_envelope_parser_requires_exact_binding_and_shape() -> Result<(), Error> {
    let session = "01890f4c-3b2a-7c01-8d11-9a321b7c0004";
    let mut envelope = serde_json::json!({
        "command_id": 7,
        "game_session_id": session,
        "payload": [0, 255],
        "server_sequence": 12,
        "status": 2,
    });
    let bytes = serde_json::to_vec(&envelope)?;
    let parsed = parse_live_result(&bytes, session, 7, 12)?;
    assert_eq!(parsed.status, CommandStatus::Rejected);
    assert_eq!(parsed.payload, [0, 255]);
    assert!(parse_live_result(&bytes, session, 8, 12).is_err());
    assert!(parse_live_result(&bytes, session, 7, 11).is_err());
    assert!(parse_live_result(&bytes, "wrong session", 7, 12).is_err());
    envelope["extra"] = serde_json::Value::Null;
    assert!(parse_live_result(&serde_json::to_vec(&envelope)?, session, 7, 12).is_err());
    Ok(())
}

fn assert_invalid_results(panel: &mut TaskBoardPanel, refused: &Reply) -> Result<(), Error> {
    // Undecodable and rejected replies leave the confirmed board untouched.
    full_request(panel, FullIntent::Refresh).ok_or("query request")?;
    apply_full_result(panel, &result(CommandStatus::Accepted, &[0xff, 0x00]));
    assert_eq!(
        panel.feedback,
        Some("The server returned an invalid Task Board reply")
    );
    assert_eq!(panel.weekly.as_ref(), Some(refused));
    full_request(panel, FullIntent::Refresh).ok_or("query request")?;
    apply_full_result(panel, &result(CommandStatus::Rejected, &[]));
    assert_eq!(
        panel.feedback,
        Some("The server refused the Task Board action")
    );
    assert_eq!(panel.weekly.as_ref(), Some(refused));
    Ok(())
}
