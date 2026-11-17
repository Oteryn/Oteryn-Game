//! P07 replay: server Atelier current-state frames (gems, quotes, balances) and a stale-revision
//! Reveal refusal, applied through the production `request` and `apply_result`; the visible text
//! comes from `panel_text`.
use super::*;

type Error = Box<dyn std::error::Error>;

struct Frames {
    receipt: Vec<u8>,
    wheel: Vec<u8>,
    atelier: Vec<u8>,
    resources: Option<Vec<u8>>,
}

struct Captured {
    query: Frames,
    stale: Frames,
    expected: String,
}

fn occurrence(byte: u8) -> [u8; 16] {
    let mut id = [byte; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}

fn wire<T>(result: Result<T, impl std::fmt::Debug>) -> Result<T, Error> {
    result.map_err(|error| format!("{error:?}").into())
}

fn atelier_reply(refused: bool, revision: u64) -> atelier::AtelierReply {
    atelier::AtelierReply {
        refused,
        revision,
        gems: Vec::new(),
        grades: Vec::new(),
        quality_quotes: (0..3)
            .map(|quality| atelier::QualityQuote {
                quality,
                reveal_gold: 50_000 + u64::from(quality),
                switch_gold: 70_000,
                fragment_kind: 0,
                fragment_min: 1,
                fragment_max: 2,
            })
            .collect(),
        grade_quotes: Vec::new(),
    }
}

fn frames(receipt: bool, revision: u64, balances: bool) -> Result<Frames, Error> {
    Ok(Frames {
        receipt: wire(atelier::encode_reply(&atelier_reply(receipt, revision)))?,
        wheel: wire(wheel::encode_wheel_reply(&wheel::WheelAllocationReply {
            refused: false,
            revision: 1,
            points: [0; 36],
            vessels: None,
        }))?,
        atelier: wire(atelier::encode_reply(&atelier_reply(false, revision)))?,
        resources: balances.then(|| {
            atelier::resources::encode_reply(&atelier::resources::ResourceBalances {
                carried_gold: 123_456,
                ..Default::default()
            })
        }),
    })
}

fn fixture() -> Result<Captured, Error> {
    Ok(Captured {
        query: frames(false, 3, true)?,
        stale: frames(true, 3, true)?,
        expected: "revision=3\ngems=0\ngrades=0\nreveal_gold_q0=50000\ncarried_gold=123456\nstale_refused=true\nstale_current_revision=3\n".into(),
    })
}

fn captured() -> Result<Captured, Error> {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return fixture();
    };
    let path = std::path::Path::new(&path);
    let frames = |name: &str| -> Result<Frames, Error> {
        Ok(Frames {
            receipt: std::fs::read(path.join(format!("p07-{name}-receipt.bin")))?,
            wheel: std::fs::read(path.join(format!("p07-{name}-wheel.bin")))?,
            atelier: std::fs::read(path.join(format!("p07-{name}-atelier.bin")))?,
            resources: std::fs::read(path.join(format!("p07-{name}-resources.bin"))).ok(),
        })
    };
    Ok(Captured {
        query: frames("query")?,
        stale: frames("stale")?,
        expected: std::fs::read_to_string(path.join("p07-expected.txt"))?,
    })
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected capture line {key} absent").into())
}

fn response(kind: u32, frames: &Frames) -> WheelResponse {
    WheelResponse {
        kind,
        status: CommandStatus::Accepted,
        payload: frames.receipt.clone(),
        wheel: Some(frames.wheel.clone()),
        atelier: Some(frames.atelier.clone()),
        resources: frames.resources.clone(),
    }
}

#[test]
fn e2e_p07_replays_atelier_state_and_stale_reveal() -> Result<(), Error> {
    let captured = captured()?;
    let e = captured.expected.as_str();
    let revision: u64 = value(e, "revision")?.parse()?;
    let mut panel = WheelPanel {
        selected: true,
        ..WheelPanel::default()
    };
    request(&mut panel, Intent::Refresh, occurrence(1)).ok_or("refresh")?;
    assert!(apply_result(&mut panel, &response(161, &captured.query)));
    let state = panel.confirmed_atelier.clone().ok_or("atelier state")?;
    assert_eq!(state.revision, revision);
    assert_eq!(state.gems.len().to_string(), value(e, "gems")?);
    assert_eq!(state.grades.len().to_string(), value(e, "grades")?);
    let quote = state
        .quality_quotes
        .iter()
        .find(|q| q.quality == 0)
        .ok_or("quote")?;
    assert_eq!(quote.reveal_gold.to_string(), value(e, "reveal_gold_q0")?);
    let text = panel_text(&panel);
    let lines: Vec<&str> = text.split("\r\n").collect();
    let reveal = format!(
        "Reveal Lesser gem: {} gold and one matching unrevealed gem.",
        value(e, "reveal_gold_q0")?
    );
    assert!(lines.contains(&reveal.as_str()));
    let carried = panel
        .confirmed_resources
        .map(|b| b.carried_gold.to_string());
    assert_eq!(carried.as_deref(), Some(value(e, "carried_gold")?));
    let gold = lines
        .iter()
        .find(|l| l.starts_with("Gold: "))
        .ok_or("gold line")?;
    assert!(gold.starts_with(&format!("Gold: {} carried;", value(e, "carried_gold")?)));

    // Reveal carries the confirmed revision; the server refuses the stale one and nothing moves.
    let reveal = request(&mut panel, Intent::Reveal, occurrence(2)).ok_or("reveal")?;
    assert_eq!(reveal.kind, 424);
    let sent = wire(atelier::decode_mutate(&reveal.payload))?;
    assert_eq!(sent.expected_revision, revision);
    assert!(apply_result(&mut panel, &response(424, &captured.stale)));
    assert_eq!(value(e, "stale_refused")?, "true");
    assert_eq!(
        panel.feedback,
        Some("The server refused the change; check Premium, points, location and resources")
    );
    let after = panel.confirmed_atelier.clone().ok_or("atelier state")?;
    assert_eq!(
        after.revision.to_string(),
        value(e, "stale_current_revision")?
    );
    assert_eq!(after.gems, state.gems);
    assert_eq!(
        panel
            .confirmed_resources
            .map(|b| b.carried_gold.to_string())
            .as_deref(),
        Some(value(e, "carried_gold")?)
    );

    // A receipt of the wrong command kind is not applied to the confirmed Atelier state.
    request(&mut panel, Intent::Reveal, occurrence(3)).ok_or("reveal")?;
    assert!(!apply_result(&mut panel, &response(309, &captured.stale)));
    assert_eq!(
        panel.feedback,
        Some("The response did not match this request; refresh and retry")
    );
    assert_eq!(panel.confirmed_atelier.as_ref(), Some(&after));
    Ok(())
}
