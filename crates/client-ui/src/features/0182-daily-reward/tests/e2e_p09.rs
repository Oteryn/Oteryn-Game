//! P09 replay: the server daily-reward wall and a claim refusal, decoded by the production
//! `decode_reply` and applied through `request`, `apply_result` and `panel_text`.
use super::*;

type Error = Box<dyn std::error::Error>;

struct Captured {
    wall: Vec<u8>,
    claim: Vec<u8>,
    expected: String,
}

fn fixture() -> Result<Captured, Error> {
    let wall = daily_reward::DailyRewardWall {
        lane_day: 12,
        streak: 19,
        jokers: 2,
        claimed: false,
        claim_enabled: false,
    };
    let claim = daily_reward::DailyRewardResult {
        disposition: 4,
        transaction_id: None,
    };
    Ok(Captured {
        wall: wall.encode().map_err(|e| format!("{e:?}"))?,
        claim: claim.encode().map_err(|e| format!("{e:?}"))?,
        expected: "lane_day=12\nstreak=19\njokers=2\nclaimed=false\nclaim_enabled=false\nclaim_status=refused\nclaim_disposition=4\n".into(),
    })
}

fn captured() -> Result<Captured, Error> {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return fixture();
    };
    let path = std::path::Path::new(&path);
    Ok(Captured {
        wall: std::fs::read(path.join("p09-wall.bin"))?,
        claim: std::fs::read(path.join("p09-claim.bin"))?,
        expected: std::fs::read_to_string(path.join("p09-expected.txt"))?,
    })
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected capture line {key} absent").into())
}

/// The claim feedback the player must see per server disposition, stated independently.
fn claim_feedback(disposition: u8) -> &'static str {
    match disposition {
        1 => "Daily reward claimed",
        2 => "Daily reward already claimed",
        3 => "Daily reward request was invalid",
        4 => "Open the daily reward shrine within reach, then refresh",
        5 => "Daily reward content is unavailable",
        6 => "Daily reward content changed; refresh before claiming",
        _ => "Daily reward claim was refused",
    }
}

#[test]
fn e2e_p09_replays_wall_and_claim_refusal() -> Result<(), Error> {
    let captured = captured()?;
    let e = captured.expected.as_str();
    let mut panel = DailyRewardPanel {
        selected: true,
        ..DailyRewardPanel::default()
    };
    assert_eq!(
        request(&mut panel, Intent::Refresh),
        Some(DailyRewardCommand::Query)
    );
    let wall = decode_reply(DailyRewardCommand::Query, &captured.wall)?;
    assert!(apply_result(
        &mut panel,
        &DailyRewardResponse {
            status: CommandStatus::Accepted,
            reply: Some(wall),
        }
    ));
    let claimed = value(e, "claimed")? == "true";
    let enabled = value(e, "claim_enabled")? == "true";
    let status = if claimed {
        "claimed"
    } else if enabled {
        "ready to claim"
    } else {
        "not available"
    };
    let text = panel_text(&panel);
    let lines: Vec<&str> = text.lines().collect();
    let lane_day: u8 = value(e, "lane_day")?.parse()?;
    let cycle_day = match lane_day {
        1 | 8 | 15 | 22 | 29 => 1,
        2 | 9 | 16 | 23 | 30 => 2,
        3 | 10 | 17 | 24 => 3,
        4 | 11 | 18 | 25 => 4,
        5 | 12 | 19 | 26 => 5,
        6 | 13 | 20 | 27 => 6,
        7 | 14 | 21 | 28 => 7,
        other => return Err(format!("invalid one-based server lane day {other}").into()),
    };
    assert_eq!(panel.wall.map(|wall| wall.lane_day), Some(lane_day));
    for line in [
        format!("Day {cycle_day} of 7"),
        format!("Streak: {}", value(e, "streak")?),
        format!("Jokers: {}", value(e, "jokers")?),
        format!("Status: {status}"),
    ] {
        assert!(lines.contains(&line.as_str()), "{line}");
    }
    let shown = panel.wall.ok_or("wall")?;

    // With the claim disabled the panel never sends it.
    if !enabled {
        assert_eq!(request(&mut panel, Intent::Claim), None);
        assert_eq!(
            panel.feedback,
            Some("The daily reward cannot be claimed now")
        );
    }

    // The server's claim reply (refused while the shrine is unopened) is shown as its disposition.
    let disposition: u8 = value(e, "claim_disposition")?.parse()?;
    let result = decode_reply(DailyRewardCommand::Claim, &captured.claim)?;
    let status = if value(e, "claim_status")? == "accepted" {
        CommandStatus::Accepted
    } else {
        CommandStatus::Rejected
    };
    let succeeded = apply_result(
        &mut panel,
        &DailyRewardResponse {
            status,
            reply: Some(result),
        },
    );
    assert_eq!(succeeded, disposition == 1);
    assert_eq!(panel.feedback, Some(claim_feedback(disposition)));
    if matches!(disposition, 1 | 2) {
        assert_eq!(
            panel.wall.map(|w| (w.claimed, w.claim_enabled)),
            Some((true, false))
        );
    } else {
        assert_eq!(panel.wall, Some(shown));
    }

    // A reply without state and a refusal without a result leave the wall as it was.
    let kept = panel.wall;
    assert!(!apply_result(
        &mut panel,
        &DailyRewardResponse {
            status: CommandStatus::Accepted,
            reply: None,
        }
    ));
    assert_eq!(
        panel.feedback,
        Some("The server returned no daily reward state")
    );
    assert!(!apply_result(
        &mut panel,
        &DailyRewardResponse {
            status: CommandStatus::Rejected,
            reply: None,
        }
    ));
    assert_eq!(
        panel.feedback,
        Some("The server refused the daily reward action")
    );
    assert_eq!(panel.wall, kept);
    assert!(decode_reply(DailyRewardCommand::Query, &[0x08]).is_err());
    Ok(())
}
