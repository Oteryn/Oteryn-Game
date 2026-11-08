//! P13 replay: the server loyalty-skills snapshot and the rejection of a malformed query, applied
//! through the production `request`, `LoyaltySkillsPanel::finish` and `panel_text`.
use super::*;

type Error = Box<dyn std::error::Error>;

struct Captured {
    query: Vec<u8>,
    after: Vec<u8>,
    expected: String,
}

fn fixture() -> Result<Captured, Error> {
    let skills = [
        (10, 12),
        (11, 13),
        (12, 14),
        (13, 15),
        (14, 16),
        (15, 17),
        (16, 18),
        (17, 19),
    ]
    .map(|(base, with_loyalty)| SkillLevel { base, with_loyalty });
    let snapshot = LoyaltySkills {
        points: 2_000,
        bonus_percent: 25,
        title: Some("Warrior of Tibia".into()),
        skills,
    };
    let bytes = snapshot.encode().ok_or("fixture encoder refused")?;
    Ok(Captured {
        after: bytes.clone(),
        query: bytes,
        expected: "points=2000\nbonus_percent=25\ntitle=Warrior of Tibia\nskills=10/12,11/13,12/14,13/15,14/16,15/17,16/18,17/19\nrefused_status=Rejected\n".into(),
    })
}

fn captured() -> Result<Captured, Error> {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return fixture();
    };
    let path = std::path::Path::new(&path);
    Ok(Captured {
        query: std::fs::read(path.join("p13-query.bin"))?,
        after: std::fs::read(path.join("p13-after.bin"))?,
        expected: std::fs::read_to_string(path.join("p13-expected.txt"))?,
    })
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected capture line {key} absent").into())
}

/// The text the player must see for the captured values, stated independently of the view.
fn visible(e: &str) -> Result<String, Error> {
    let title = match value(e, "title")? {
        "" => "No loyalty title",
        title => title,
    };
    let mut text = format!(
        "Loyalty: {} points · {}% skill-point bonus\r\n{title}\r\n\r\nSkills: base / with Loyalty\r\n",
        value(e, "points")?,
        value(e, "bonus_percent")?
    );
    let labels = [
        "Magic",
        "Fist",
        "Club",
        "Sword",
        "Axe",
        "Distance",
        "Shielding",
        "Fishing",
    ];
    for (label, pair) in labels.iter().zip(value(e, "skills")?.split(',')) {
        let (base, with) = pair.split_once('/').ok_or("skill pair")?;
        text.push_str(&format!("{label}: {base} / {with}\r\n"));
    }
    Ok(text)
}

#[test]
fn e2e_p13_replays_snapshot_and_malformed_query_rejection() -> Result<(), Error> {
    let captured = captured()?;
    let e = captured.expected.as_str();
    let mut panel = LoyaltySkillsPanel {
        selected: true,
        ..LoyaltySkillsPanel::default()
    };
    assert_eq!(request(&mut panel), Some(Vec::new()));
    assert!(panel.finish(true, &captured.query));
    let shown = visible(e)?;
    assert_eq!(panel_text(&panel), shown);
    let confirmed = panel.snapshot.clone().ok_or("snapshot")?;
    assert_eq!(confirmed.points.to_string(), value(e, "points")?);

    // The server rejects a malformed query with an empty payload; the last snapshot stays.
    assert_eq!(value(e, "refused_status")?, "Rejected");
    assert!(request(&mut panel).is_some());
    assert!(!panel.finish(false, &[]));
    assert_eq!(panel.snapshot, Some(confirmed.clone()));
    assert_eq!(
        panel_text(&panel),
        "Skills could not be refreshed. Try Refresh."
    );

    // The next valid query shows the same snapshot, and an undecodable one is not applied.
    assert!(request(&mut panel).is_some());
    assert!(panel.finish(true, &captured.after));
    assert_eq!(panel_text(&panel), shown);
    assert!(request(&mut panel).is_some());
    let mut truncated = captured.query.clone();
    truncated.pop();
    assert!(!panel.finish(true, &truncated));
    assert_eq!(panel.snapshot, Some(confirmed));
    Ok(())
}
