//! Replays C15 GeneralStats payloads through the owning client UI seam.
//!
//! The payload consumer is `protocol::apply_result` / `protocol::decode_tab` in
//! `crates/client-ui/src/features/0199-cyclopedia-character/protocol.rs`; the
//! visible state is `CyclopediaCharacterPanel::result` in `state.rs`. Searches
//! also covered `crates/client-runtime/src/features/0199-cyclopedia-character/`
//! (request/codec only) and the dev-client/session snapshot and delta reducers
//! used by `tools/dev-client/src/lib.rs` (no GeneralStats payload consumer).

use super::general_stats_selection as selection;

use super::{CyclopediaCharacterPanel, Intent, apply_result, request};
use oteryn_client_runtime::{
    CommandStatus,
    cyclopedia_character::{GeneralStats, TabView},
    raw_feature::GatedCommandResult,
};

fn reply(payload: Vec<u8>) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload,
    }
}

#[test]
fn e2e_c15_replays_hunt_and_offline_recovery_payloads() -> Result<(), Box<dyn std::error::Error>> {
    let (payloads, expected, selections) =
        if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let path = std::path::Path::new(&path);
            (
                [
                    std::fs::read(path.join("c15-before.bin"))?,
                    std::fs::read(path.join("c15-after.bin"))?,
                    std::fs::read(path.join("c15-recovered.bin"))?,
                ],
                std::fs::read_to_string(path.join("c15-expected.txt"))?,
                [
                    selection::read(path, "c15-before.bin")?,
                    selection::read(path, "c15-after.bin")?,
                    selection::read(path, "c15-recovered.bin")?,
                ],
            )
        } else {
            let values = [(100, 7, 2_520), (105, 7, 2_519), (105, 7, 2_520)];
            let payloads = values.map(|(experience, level, stamina_minutes)| {
                TabView::GeneralStats(GeneralStats {
                    experience,
                    level,
                    stamina_minutes,
                    ..GeneralStats::default()
                })
                .encode()
                .map_err(|error| format!("fixture encode: {error:?}"))
            });
            (
                [
                    payloads[0].clone()?,
                    payloads[1].clone()?,
                    payloads[2].clone()?,
                ],
                "100 7 2520\n105 7 2519\n105 7 2520\n".into(),
                [selection::legacy(); 3],
            )
        };
    let expected = expected
        .lines()
        .map(|line| {
            line.split_whitespace()
                .map(str::parse::<u64>)
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    if expected.len() != 3 || expected.iter().any(|row| row.len() != 3) {
        return Err("invalid C15 expected capture".into());
    }

    let mut panel = CyclopediaCharacterPanel {
        selected: true,
        ..Default::default()
    };
    for ((payload, values), selected) in payloads.into_iter().zip(expected).zip(selections) {
        panel.extensions = selected.extensions;
        if request(&mut panel, Intent::Tab).is_none() || !apply_result(&mut panel, &reply(payload))
        {
            return Err("C15 payload was not applied".into());
        }
        selection::check(&panel, selected);
        let visible = panel.result.as_deref().ok_or("missing C15 visible state")?;
        assert!(visible.contains(&format!("Experience: {}", values[0])));
        assert!(visible.contains(&format!("Level: {}", values[1])));
        assert!(visible.contains(&format!(
            "Stamina: {}:{:02}",
            values[2] / 60,
            values[2] % 60
        )));
    }

    let unchanged = panel.result.clone();
    if request(&mut panel, Intent::Tab).is_none() {
        return Err("invalid-payload request was not issued".into());
    }
    assert!(!apply_result(&mut panel, &reply(vec![0])));
    assert_eq!(panel.result, unchanged);
    Ok(())
}
