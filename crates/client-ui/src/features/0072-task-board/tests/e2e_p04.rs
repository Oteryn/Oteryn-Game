//! P04 replay: server TASK_BOARD replies (Query, then a refused StartWeekly) applied through the
//! production `full_request`, `apply_full_result` and `panel_text`.
use super::*;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
use task_board::full::{Bounty, Reply, Weekly, decode_request, encode_reply};

type Error = Box<dyn std::error::Error>;

struct Captured {
    query: Vec<u8>,
    refused: Vec<u8>,
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
        query: encode(&reply(0))?,
        refused: encode(&reply(2))?,
        expected: "period=2913\npoints=40\nbounty_points=7\nsoulseals=2\nstarted=false\nkills=0\ndeliveries=0\nshop_count=1\nshop_first=0,1,25\nrefusal=2\n".into(),
    })
}

fn captured() -> Result<Captured, Error> {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return fixture();
    };
    let path = std::path::Path::new(&path);
    Ok(Captured {
        query: std::fs::read(path.join("p04-query.bin"))?,
        refused: std::fs::read(path.join("p04-refused.bin"))?,
        expected: std::fs::read_to_string(path.join("p04-expected.txt"))?,
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
    apply_full_result(
        &mut panel,
        &result(CommandStatus::Accepted, &captured.query),
    );
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
        "Already owned"
    } else if weekly.points >= u64::from(offer.price) {
        "Available — server confirms purchase"
    } else {
        "points short"
    };
    let price_line = format!("  {} task points — {affordability}", first[2]);
    let at = lines
        .iter()
        .position(|line| *line == shop_header)
        .ok_or("shop header line")?;
    assert_eq!(lines.get(at + 1).copied(), Some(reward.as_str()));
    assert_eq!(lines.get(at + 2).copied(), Some(price_line.as_str()));

    // StartWeekly with an unoffered difficulty is refused and the weekly state stays the same.
    let start = full_request(&mut panel, FullIntent::StartWeekly(3)).ok_or("start request")?;
    let sent = decode_request(&start.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!((sent.action, sent.selection), (5, 3));
    apply_full_result(
        &mut panel,
        &result(CommandStatus::Accepted, &captured.refused),
    );
    assert_eq!(
        panel.feedback,
        Some("The server refused: task or bounty is not in the required state")
    );
    let refused = panel.weekly.clone().ok_or("weekly state")?;
    assert_eq!(value(e, "refusal")?, "2");
    assert_eq!(refused.refusal.to_string(), value(e, "refusal")?);
    assert_eq!(refused.weekly, weekly.weekly);
    assert_eq!(refused.points, weekly.points);
    assert!(
        panel_text(&panel)
            .ends_with("The server refused: task or bounty is not in the required state")
    );

    // Undecodable and rejected replies leave the confirmed board untouched.
    full_request(&mut panel, FullIntent::Refresh).ok_or("query request")?;
    apply_full_result(&mut panel, &result(CommandStatus::Accepted, &[0xff, 0x00]));
    assert_eq!(
        panel.feedback,
        Some("The server returned an invalid Task Board reply")
    );
    assert_eq!(panel.weekly.as_ref(), Some(&refused));
    full_request(&mut panel, FullIntent::Refresh).ok_or("query request")?;
    apply_full_result(&mut panel, &result(CommandStatus::Rejected, &[]));
    assert_eq!(
        panel.feedback,
        Some("The server refused the Task Board action")
    );
    assert_eq!(panel.weekly.as_ref(), Some(&refused));
    Ok(())
}
