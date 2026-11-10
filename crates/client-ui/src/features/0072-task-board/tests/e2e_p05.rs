//! P05 replay: server TASK_BOARD replies for a Talisman purchase and the refused repeat, applied
//! through the production `talisman::request`, `apply_full_result` and `panel_text`.
use super::*;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
use task_board::full::{Bounty, Reply, TalismanPath, Weekly, decode_request, encode_reply};

type Error = Box<dyn std::error::Error>;

struct Captured {
    query: Vec<u8>,
    buy: Vec<u8>,
    repeat: Vec<u8>,
    expected: String,
}

fn reply(refusal: u8, bounty_points: u64, damage: TalismanPath) -> Reply {
    Reply {
        weekly_results: None,
        refusal,
        period: 2_913,
        points: 0,
        bounty_points,
        soulseals: 0,
        weekly: Weekly {
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
        },
        bounty: Bounty {
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
        },
        talisman: [
            damage,
            TalismanPath::default(),
            TalismanPath::default(),
            TalismanPath::default(),
        ],
        shop: Vec::new(),
    }
}

fn fixture() -> Result<Captured, Error> {
    let encode = |reply: &Reply| encode_reply(reply).map_err(|e| format!("{e:?}"));
    let before = TalismanPath {
        level: 0,
        bonus: 0,
        next_cost: 5,
    };
    let after = TalismanPath {
        level: 1,
        bonus: 1,
        next_cost: 10,
    };
    Ok(Captured {
        query: encode(&reply(0, 30, before))?,
        buy: encode(&reply(0, 25, after))?,
        repeat: encode(&reply(2, 25, after))?,
        expected: "bounty_points_before=30\nbounty_points_after=25\ntalisman_before=0,0,5|0,0,0|0,0,0|0,0,0\ntalisman_after=1,1,10|0,0,0|0,0,0|0,0,0\nrefusal=2\n".into(),
    })
}

fn captured() -> Result<Captured, Error> {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return fixture();
    };
    let path = std::path::Path::new(&path);
    Ok(Captured {
        query: std::fs::read(path.join("p05-query.bin"))?,
        buy: std::fs::read(path.join("p05-buy.bin"))?,
        repeat: std::fs::read(path.join("p05-repeat.bin"))?,
        expected: std::fs::read_to_string(path.join("p05-expected.txt"))?,
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

/// The Talisman lines the player must see for a `level,bonus,next_cost|...` capture value.
fn talisman_lines(value: &str) -> Vec<String> {
    value
        .split('|')
        .enumerate()
        .map(|(path, fields)| {
            let f: Vec<&str> = fields.split(',').collect();
            format!(
                "Talisman {} ({}): level {}, bonus {}, next cost {}",
                path + 1,
                ["Damage", "Life leech", "Loot", "Bestiary"][path],
                f[0],
                f[1],
                f[2]
            )
        })
        .collect()
}

fn shown(panel: &TaskBoardPanel) -> Vec<String> {
    panel_text(panel)
        .lines()
        .filter(|line| line.starts_with("Talisman "))
        .map(str::to_owned)
        .collect()
}

#[test]
fn e2e_p05_replays_purchase_and_refused_repeat() -> Result<(), Error> {
    let captured = captured()?;
    let e = captured.expected.as_str();
    let mut panel = TaskBoardPanel {
        selected: true,
        ..TaskBoardPanel::default()
    };
    full_request(&mut panel, FullIntent::Refresh).ok_or("query request")?;
    apply_full_result(
        &mut panel,
        &result(CommandStatus::Accepted, &captured.query),
    );
    assert_eq!(shown(&panel), talisman_lines(value(e, "talisman_before")?));
    assert_eq!(
        panel
            .weekly
            .as_ref()
            .map(|r| r.bounty_points.to_string())
            .as_deref(),
        Some(value(e, "bounty_points_before")?)
    );

    let buy = talisman::request(&mut panel, talisman::Intent::Buy).ok_or("buy request")?;
    let sent = decode_request(&buy.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!((sent.action, sent.selection), (17, 0));
    apply_full_result(&mut panel, &result(CommandStatus::Accepted, &captured.buy));
    assert_eq!(panel.feedback, None);
    assert_eq!(shown(&panel), talisman_lines(value(e, "talisman_after")?));
    assert_eq!(
        panel
            .weekly
            .as_ref()
            .map(|r| r.bounty_points.to_string())
            .as_deref(),
        Some(value(e, "bounty_points_after")?)
    );
    assert_ne!(value(e, "talisman_before")?, value(e, "talisman_after")?);

    // The repeated purchase is refused: feedback shows and the Talisman lines do not move.
    let confirmed = panel.weekly.clone();
    talisman::request(&mut panel, talisman::Intent::Buy).ok_or("buy request")?;
    apply_full_result(
        &mut panel,
        &result(CommandStatus::Accepted, &captured.repeat),
    );
    assert_eq!(
        panel.feedback,
        Some("The server refused: task or bounty is not in the required state")
    );
    assert_eq!(shown(&panel), talisman_lines(value(e, "talisman_after")?));
    let repeated = panel.weekly.as_ref().ok_or("weekly state")?;
    assert_eq!(value(e, "refusal")?, "2");
    assert_eq!(repeated.refusal.to_string(), value(e, "refusal")?);
    assert_eq!(
        repeated.talisman,
        confirmed.ok_or("confirmed board")?.talisman
    );

    // A malformed reply keeps the confirmed board.
    let kept = panel.weekly.clone();
    talisman::request(&mut panel, talisman::Intent::Upgrade(0)).ok_or("upgrade request")?;
    apply_full_result(&mut panel, &result(CommandStatus::Accepted, &[0x07]));
    assert_eq!(
        panel.feedback,
        Some("The server returned an invalid Task Board reply")
    );
    assert_eq!(panel.weekly, kept);
    Ok(())
}
