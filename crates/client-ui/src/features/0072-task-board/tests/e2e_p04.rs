//! P04 replay: server TASK_BOARD replies (Query, then a refused StartWeekly) applied through the
//! production `full_request`, `apply_full_result` and `panel_text`.
use super::*;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
use task_board::full::{Bounty, Reply, Weekly, decode_request, encode_reply};

type Error = Box<dyn std::error::Error>;

struct Captured {
    query: GatedCommandResult,
    refused: GatedCommandResult,
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
        kind: 1,
        price: 25,
        primary: 3_000,
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
        expected: "period=2913\npoints=40\nbounty_points=7\nsoulseals=2\nstarted=false\nkills=0\ndeliveries=0\nshop_count=1\nshop_first=0,1,25\nrefusal=3\n".into(),
    })
}

fn captured() -> Result<Captured, Error> {
    let path =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("live capture directory is required")?;
    let path = std::path::Path::new(&path);
    let session = std::env::var("E2E_P04_SESSION")?;
    let query = read_live_result(&path.join("p04-query.bin"), &session)?;
    let refused = read_live_result(&path.join("p04-refused.bin"), &session)?;
    assert_eq!(query.command_id.checked_add(1), Some(refused.command_id));
    assert!(query.server_sequence > 0 && refused.server_sequence > query.server_sequence);
    Ok(Captured {
        query,
        refused,
        expected: std::fs::read_to_string(path.join("p04-expected.txt"))?,
    })
}

fn read_live_result(path: &std::path::Path, session: &str) -> Result<GatedCommandResult, Error> {
    let bytes = std::fs::read(path)?;
    let newline = bytes
        .iter()
        .position(|&byte| byte == b'\n')
        .ok_or("missing result header")?;
    let mut fields = std::str::from_utf8(&bytes[..newline])?.split(' ');
    assert_eq!(fields.next(), Some(session));
    let command_id = fields.next().ok_or("missing command id")?.parse()?;
    let server_sequence = fields.next().ok_or("missing server sequence")?.parse()?;
    let status: u8 = fields.next().ok_or("missing command status")?.parse()?;
    assert_eq!(status, CommandStatus::Accepted as u8);
    assert_eq!(fields.next(), None);
    Ok(GatedCommandResult {
        command_id,
        status: CommandStatus::Accepted,
        server_sequence,
        payload: bytes[newline + 1..].to_vec(),
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
    let captured = captured()?;
    let e = captured.expected.as_str();
    let mut panel = TaskBoardPanel {
        selected: true,
        ..TaskBoardPanel::default()
    };
    let query = full_request(&mut panel, FullIntent::Refresh).ok_or("query request")?;
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
    let shop_header = format!(
        "Shop {} — {}",
        offer.index + 1,
        super::shop_names::name(offer)
    );
    let reward = format!(
        "  Reward {} / {} (addon {})",
        offer.primary, offer.secondary, offer.addon
    );
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
    assert_eq!(lines.get(at + 1).copied(), Some(reward.as_str()));
    assert_eq!(lines.get(at + 2).copied(), Some(price_line.as_str()));

    // StartWeekly above the character’s level is refused and the weekly state stays the same.
    let start = full_request(&mut panel, FullIntent::StartWeekly(3)).ok_or("start request")?;
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

    assert_invalid_results(&mut panel, &refused)
}

#[test]
fn p04_invalid_results_preserve_the_confirmed_board() -> Result<(), Error> {
    let captured = fixture()?;
    let mut panel = TaskBoardPanel {
        selected: true,
        ..TaskBoardPanel::default()
    };
    apply_full_result(&mut panel, &captured.refused);
    let refused = panel.weekly.clone().ok_or("weekly state")?;
    assert_invalid_results(&mut panel, &refused)
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
