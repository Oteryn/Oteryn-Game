//! SCENARIO_MATRIX C03 replay coverage.
//!
//! The owning client seam is
//! `crates/client-ui/src/features/0199-cyclopedia-character/protocol.rs`:
//! `request` builds command 388 and `apply_result` consumes its raw result payload into the
//! panel text. The expected numbers come from the server-observed reply (the capture phase
//! decodes the payload with the wire codec and writes them to `c03-expected.txt`), never from
//! the client's own rendering of the same payload.

use super::{CyclopediaCharacterPanel, Intent, apply_result, panel_text, request};
use oteryn_client_runtime::{
    CommandStatus,
    cyclopedia_character::{
        CharacterTab, TabView,
        combat::{CombatStat, CombatStatUnit, CombatStats},
    },
    raw_feature::GatedCommandResult,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Per-field distinct fixture value, so a swapped or shifted field cannot pass.
fn fixture_value(key: CombatStat) -> i64 {
    i64::from(key as u8) * 1_013 + 7
}

fn fixture(tab: CharacterTab) -> TestResult<(Vec<u8>, String)> {
    let mut stats = CombatStats::new(tab, 1_234, 2, 3).map_err(|_| "combat stats")?;
    let mut text = format!("tab={tab:?} observed=1234 character=2 equipment=3\n");
    for key in CombatStat::for_tab(tab) {
        stats
            .set(*key, fixture_value(*key))
            .map_err(|_| "combat stat")?;
        text.push_str(&format!("{key:?}={}\n", fixture_value(*key)));
    }
    let payload = TabView::Combat(stats)
        .encode()
        .map_err(|_| "combat encoding")?;
    Ok((payload, text))
}

/// Renders the panel text a player must see for the server-observed numbers in `lines`.
fn expected_text(tab: CharacterTab, lines: &[&str]) -> TestResult<String> {
    let header = lines.first().ok_or("expected header absent")?;
    let field = |name: &str| -> TestResult<&str> {
        header
            .split_whitespace()
            .find_map(|part| part.strip_prefix(name))
            .ok_or_else(|| format!("expected header lacks {name}").into())
    };
    let mut rows = vec![
        format!("Cyclopedia character — {tab:?}, offset 0, limit 20\r\n{tab:?}"),
        format!(
            "Observed at: {} ms\nCharacter revision: {}\nEquipment revision: {}",
            field("observed=")?,
            field("character=")?,
            field("equipment=")?
        ),
    ];
    for key in CombatStat::for_tab(tab) {
        let prefix = format!("{key:?}=");
        let raw = lines
            .iter()
            .find_map(|line| line.strip_prefix(prefix.as_str()))
            .ok_or_else(|| format!("expected value for {key:?} absent"))?;
        let shown = if raw == "unavailable" {
            raw.to_owned()
        } else {
            let value: i64 = raw.parse()?;
            match key.unit() {
                CombatStatUnit::PartsPerMillion => {
                    let magnitude = value.unsigned_abs();
                    format!(
                        "{}{}.{:04}%",
                        if value < 0 { "-" } else { "" },
                        magnitude / 10_000,
                        magnitude % 10_000
                    )
                }
                CombatStatUnit::Seconds => format!("{value} seconds"),
                _ => value.to_string(),
            }
        };
        rows.push(format!("{key:?}: {shown}"));
    }
    Ok(rows.join("\n"))
}

fn apply(payload: Vec<u8>, tab: CharacterTab) -> Result<CyclopediaCharacterPanel, &'static str> {
    let mut panel = CyclopediaCharacterPanel {
        selected: true,
        tab,
        ..Default::default()
    };
    request(&mut panel, Intent::Tab).ok_or("combat request")?;
    if !apply_result(
        &mut panel,
        &GatedCommandResult {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload,
        },
    ) {
        return Err("combat result");
    }
    Ok(panel)
}

#[test]
fn e2e_c03_replays_combat_pages_with_server_observed_numbers() -> TestResult {
    let (offence, defence, expected) =
        if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let directory = std::path::Path::new(&directory);
            (
                std::fs::read(directory.join("c03-offence.bin"))?,
                std::fs::read(directory.join("c03-defence.bin"))?,
                std::fs::read_to_string(directory.join("c03-expected.txt"))?,
            )
        } else {
            let (offence, offence_text) = fixture(CharacterTab::Offence)?;
            let (defence, defence_text) = fixture(CharacterTab::Defence)?;
            (offence, defence, format!("{offence_text}{defence_text}"))
        };
    let lines: Vec<&str> = expected.lines().collect();
    let split = lines
        .iter()
        .skip(1)
        .position(|line| line.starts_with("tab="))
        .map(|at| at + 1)
        .ok_or("expected file lacks the Defence section")?;
    let offence_panel = apply(offence, CharacterTab::Offence)?;
    let defence_panel = apply(defence, CharacterTab::Defence)?;
    assert_eq!(
        panel_text(&offence_panel),
        expected_text(CharacterTab::Offence, &lines[..split])?
    );
    assert_eq!(
        panel_text(&defence_panel),
        expected_text(CharacterTab::Defence, &lines[split..])?
    );

    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        // Fixture mode: pin exact literals so the formatter above cannot mirror a client bug.
        let offence_text = panel_text(&offence_panel);
        assert!(offence_text.contains("Observed at: 1234 ms\nCharacter revision: 2\n"));
        assert!(offence_text.contains("\nCriticalChance: 0.1020%\n"));
        assert!(offence_text.contains("\nCriticalDamage: 0.7098%\n"));
        assert!(offence_text.contains("\nPerfectShot1: 24319\n"));
        let defence_text = panel_text(&defence_panel);
        assert!(defence_text.contains("\nPhysicalReflection: 71930\n"));
        assert!(defence_text.contains("\nDodge: 6.4839%\n"));
    }

    // A Defence payload delivered for an Offence request is refused and changes nothing.
    let visible_before = offence_panel.result.clone();
    let mut invalid = offence_panel;
    assert!(request(&mut invalid, Intent::Tab).is_some());
    let wrong_tab = fixture(CharacterTab::Defence)?.0;
    assert!(!apply_result(
        &mut invalid,
        &GatedCommandResult {
            command_id: 2,
            status: CommandStatus::Accepted,
            server_sequence: 2,
            payload: wrong_tab,
        }
    ));
    assert_eq!(invalid.result, visible_before);
    Ok(())
}
