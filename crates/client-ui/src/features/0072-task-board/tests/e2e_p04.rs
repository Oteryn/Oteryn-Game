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
    master: Option<[GatedCommandResult; 3]>,
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
    let shop = shop_oracle()?
        .into_iter()
        .map(|(offer, _)| offer)
        .collect::<Vec<_>>();
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
        master: None,
        expected: "period=2913\npoints=40\nbounty_points=7\nsoulseals=2\nstarted=false\nkills=0\ndeliveries=0\nshop_count=24\nrefusal=3\n".into(),
    })
}

fn captured() -> Result<Captured, Error> {
    let path =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("live capture directory is required")?;
    let path = std::path::Path::new(&path);
    let binding = std::env::var("E2E_P04_BINDING")?;
    let (below, at) = binding.split_once(";at=").ok_or("missing Master binding")?;
    let mut sessions = std::collections::BTreeSet::new();
    let [query, refused, readmission] = read_boundary(
        path,
        below
            .strip_prefix("below=")
            .ok_or("missing refusal binding")?,
        ["p04-query.json", "p04-refused.json", "p04-readmission.json"],
        &mut sessions,
    )?;
    let master = read_boundary(
        path,
        at,
        [
            "p04-master-query.json",
            "p04-master-start.json",
            "p04-master-readmission.json",
        ],
        &mut sessions,
    )?;
    Ok(Captured {
        query,
        refused,
        readmission,
        master: Some(master),
        expected: std::fs::read_to_string(path.join("p04-expected.txt"))?,
    })
}

fn read_boundary<'a>(
    path: &std::path::Path,
    binding: &'a str,
    names: [&str; 3],
    sessions: &mut std::collections::BTreeSet<&'a str>,
) -> Result<[GatedCommandResult; 3], Error> {
    let fields: Vec<_> = binding.split(',').collect();
    let [session, after, qid, qseq, sid, sseq, aid, aseq] = fields.as_slice() else {
        return Err("invalid P04 boundary binding".into());
    };
    if ![session, after]
        .into_iter()
        .all(|id| is_uuid7(id) && sessions.insert(*id))
    {
        return Err("P04 requires four distinct UUIDv7 sessions".into());
    }
    let query = read_live_result(&path.join(names[0]), session, &format!("{qid},{qseq}"))?;
    let start = read_live_result(&path.join(names[1]), session, &format!("{sid},{sseq}"))?;
    let relog = read_live_result(&path.join(names[2]), after, &format!("{aid},{aseq}"))?;
    assert_eq!(query.command_id.checked_add(1), Some(start.command_id));
    assert!(start.server_sequence > query.server_sequence);
    for frame in [&query, &start, &relog] {
        assert_eq!(frame.status, CommandStatus::Accepted);
    }
    Ok([query, start, relog])
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

fn shop_oracle() -> Result<Vec<(task_board::full::ShopOffer, &'static str)>, Error> {
    include_str!("p04_shop_oracle.txt")
        .lines()
        .map(|line| {
            let fields: Vec<_> = line.splitn(7, ',').collect();
            let [index, kind, primary, secondary, addon, price, name] = fields.as_slice() else {
                return Err("malformed P04 shop oracle".into());
            };
            Ok((
                task_board::full::ShopOffer {
                    index: index.parse()?,
                    kind: kind.parse()?,
                    primary: primary.parse()?,
                    secondary: secondary.parse()?,
                    addon: addon.parse()?,
                    price: price.parse()?,
                    bought: false,
                },
                *name,
            ))
        })
        .collect()
}

fn verify_rendered_shop(text: &str, reply: &Reply) -> Result<(), Error> {
    let expected = shop_oracle()?;
    if !reply
        .shop
        .iter()
        .eq(expected.iter().map(|(offer, _)| offer))
    {
        return Err("P04 decoded shop differs from the authored catalogue".into());
    }
    let (_, shop) = text
        .split_once("\nTask shop\n")
        .ok_or("missing P04 shop heading")?;
    let (shop, _) = shop
        .split_once("\nBounty Talisman upgrade paths\n")
        .ok_or("missing P04 shop terminator")?;
    let expected = expected.into_iter().map(|(offer, label)| {
        let availability = match u64::from(offer.price).checked_sub(reply.points) {
            Some(shortfall) if shortfall > 0 => format!("{shortfall} points short"),
            _ => "Available — server confirms purchase".into(),
        };
        format!("Shop {} — {label}\n  Reward {} / {} (addon {})\n  {} task points — {availability}\n",
            offer.index + 1, offer.primary, offer.secondary, offer.addon, offer.price)
    }).collect::<String>();
    if shop != expected {
        return Err("P04 rendered shop differs from the authored offer sequence".into());
    }
    Ok(())
}

fn verify_master_replay(
    [query, start, after]: [GatedCommandResult; 3],
    baseline: &Reply,
) -> Result<(), Error> {
    let mut panel = TaskBoardPanel {
        selected: true,
        ..TaskBoardPanel::default()
    };
    let request = full_request(&mut panel, FullIntent::Refresh).ok_or("Master query request")?;
    verify_route(&request);
    assert_eq!(
        decode_request(&request.payload).map_err(|e| format!("{e:?}"))?,
        task_board::full::Request::query()
    );
    apply_full_result(&mut panel, &query);
    assert_eq!(panel.weekly.as_ref(), Some(baseline));
    let request =
        full_request(&mut panel, FullIntent::StartWeekly(3)).ok_or("Master start request")?;
    verify_route(&request);
    let sent = decode_request(&request.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!((sent.action, sent.selection), (5, 3));
    apply_full_result(&mut panel, &start);
    let active = panel.weekly.as_ref().ok_or("started Master board")?;
    assert_eq!(
        (
            active.refusal,
            active.weekly.started,
            active.weekly.difficulty
        ),
        (0, true, 3)
    );
    assert_eq!(active.weekly.kills.len(), 6);
    assert_eq!(active.weekly.deliveries.len(), 6);
    assert!(panel.feedback.is_none());
    let mut fresh = TaskBoardPanel {
        selected: true,
        ..TaskBoardPanel::default()
    };
    let request =
        full_request(&mut fresh, FullIntent::Refresh).ok_or("Master readmission query request")?;
    verify_route(&request);
    assert_eq!(
        decode_request(&request.payload).map_err(|e| format!("{e:?}"))?,
        task_board::full::Request::query()
    );
    apply_full_result(&mut fresh, &after);
    assert_eq!(fresh.weekly.as_ref(), Some(active));
    assert!(!fresh.pending);
    assert!(fresh.feedback.is_none());
    assert_eq!(panel_text(&fresh), panel_text(&panel));
    Ok(())
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
    verify_rendered_shop(&text, &weekly)?;

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

    if let Some(master) = captured.master {
        verify_master_replay(master, &weekly)?;
    }
    assert_invalid_results(&mut panel, &refused)
}

#[test]
fn p04_unit_projection_checks_rendering_and_invalid_results() -> Result<(), Error> {
    verify_replay(fixture()?)
}

#[test]
fn p04_shop_oracle_rejects_omitted_or_reordered_rendered_offers() -> Result<(), Error> {
    let mut panel = TaskBoardPanel {
        selected: true,
        ..TaskBoardPanel::default()
    };
    full_request(&mut panel, FullIntent::Refresh).ok_or("query request")?;
    apply_full_result(&mut panel, &fixture()?.query);
    let board = panel.weekly.as_ref().ok_or("query board")?;
    let rendered = panel_text(&panel);
    verify_rendered_shop(&rendered, board)?;
    let missing = rendered
        .lines()
        .filter(|line| !line.starts_with("Shop 24 — "))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(verify_rendered_shop(&missing, board).is_err());
    assert!(verify_rendered_shop(&rendered.replace("Shop 2 —", "Shop 3 —"), board).is_err());
    Ok(())
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
