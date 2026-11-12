//! C20a replay exercises the owning client path:
//! `crates/client-ui/src/features/0199-cyclopedia-character/protocol.rs::request` and
//! `apply_result`, which update
//! `crates/client-ui/src/features/0199-cyclopedia-character/state.rs::CyclopediaCharacterPanel::result`
//! from the server's GeneralStats command payload. The corresponding
//! `crates/client-runtime/src/features/0199-cyclopedia-character/` path does not exist, and
//! `crates/session/src/domain_dispatch.rs::Session::apply_delta` consumes state deltas rather
//! than this command-result payload.

#[path = "general_stats_selection.rs"]
mod selection;

use super::*;
use oteryn_client_runtime::{
    CommandStatus,
    cyclopedia_character::{CharacterTab, GeneralStats, TabView},
    raw_feature::GatedCommandResult,
};

#[test]
fn e2e_c20a_replays_death_experience_payloads() -> Result<(), Box<dyn std::error::Error>> {
    let (before, after, expected, selections) =
        if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let path = std::path::Path::new(&path);
            (
                std::fs::read(path.join("c20a-before.bin"))?,
                std::fs::read(path.join("c20a-after.bin"))?,
                std::fs::read_to_string(path.join("c20a-expected.txt"))?,
                [
                    selection::read(path, "c20a-before.bin")?,
                    selection::read(path, "c20a-after.bin")?,
                ],
            )
        } else {
            let before = GeneralStats {
                experience: 18_400,
                level: 20,
                ..Default::default()
            };
            let after = GeneralStats {
                experience: 17_700,
                level: 19,
                ..Default::default()
            };
            (
                TabView::GeneralStats(before)
                    .encode()
                    .map_err(|_| "before fixture encoding failed")?,
                TabView::GeneralStats(after)
                    .encode()
                    .map_err(|_| "after fixture encoding failed")?,
                "18400 20\n17700 19\n".into(),
                [selection::legacy(); 2],
            )
        };
    let values = expected
        .split_whitespace()
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()?;
    let [before_xp, before_level, after_xp, after_level] = values.as_slice() else {
        return Err("invalid C20a expected values".into());
    };

    let mut panel = CyclopediaCharacterPanel {
        selected: true,
        tab: CharacterTab::GeneralStats,
        extensions: selections[0].extensions,
        ..Default::default()
    };
    apply(&mut panel, before, 1)?;
    selection::check(&panel, selections[0]);
    let visible = panel.result.as_deref().ok_or("missing before view")?;
    assert!(visible.contains(&format!("Experience: {before_xp}")));
    assert!(visible.contains(&format!("Level: {before_level}")));
    panel.extensions = selections[1].extensions;
    apply(&mut panel, after, 2)?;
    selection::check(&panel, selections[1]);
    let visible = panel.result.as_deref().ok_or("missing after view")?;
    assert!(visible.contains(&format!("Experience: {after_xp}")));
    assert!(visible.contains(&format!("Level: {after_level}")));
    assert!(after_xp < before_xp);

    let unchanged = panel.result.clone();
    request(&mut panel, Intent::Tab).ok_or("invalid-payload request missing")?;
    let invalid = GatedCommandResult {
        command_id: 3,
        status: CommandStatus::Accepted,
        server_sequence: 3,
        payload: vec![CharacterTab::GeneralStats as u8],
    };
    assert!(!apply_result(&mut panel, &invalid));
    assert_eq!(panel.result, unchanged);
    Ok(())
}

fn apply(
    panel: &mut CyclopediaCharacterPanel,
    payload: Vec<u8>,
    sequence: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    request(panel, Intent::Tab).ok_or("GeneralStats request missing")?;
    let reply = GatedCommandResult {
        command_id: sequence,
        status: CommandStatus::Accepted,
        server_sequence: sequence,
        payload,
    };
    if !apply_result(panel, &reply) {
        return Err("GeneralStats payload rejected".into());
    }
    Ok(())
}
