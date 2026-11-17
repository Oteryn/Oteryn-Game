//! C11 native-client replay.
//!
//! Payload-consumer search:
//! - `crates/client-ui/src/features/0278-battle-instinct-status/state.rs`:
//!   `BattleInstinctStatusPanel::finish` applies the command-result payload tested here.
//! - `crates/client-ui/src/features/local-spell-action-bar/view.rs`: `vitals_text` renders Harmony
//!   and Serene after session application; it does not apply payloads.
//! - `crates/client-runtime/src/features/0278-battle-instinct-status/mod.rs` re-exports the codec
//!   but has no state reducer.
//! - `crates/session/src/domain_dispatch.rs`: `Session::apply_delta` decodes and stores actor-vitals
//!   snapshots/deltas exposed by `tools/dev-client/src/lib.rs::DevClientSession::actor_vitals`.

use super::*;

#[test]
fn replays_server_battle_instinct_status_and_preserves_state_on_invalid_payload()
-> Result<(), Box<dyn std::error::Error>> {
    let (payload, expected) = if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        (
            std::fs::read(path.join("c11-battle-instinct.bin"))?,
            std::fs::read_to_string(path.join("c11-expected.txt"))?,
        )
    } else {
        let status = BattleInstinctStatus {
            active: true,
            melee: 4,
            shielding: 24,
        };
        (
            status.encode().ok_or("C11 fixture did not encode")?,
            "active=1 melee=4 shielding=24\n".to_owned(),
        )
    };

    let mut panel = BattleInstinctStatusPanel {
        selected: true,
        pending: true,
        ..BattleInstinctStatusPanel::default()
    };
    assert!(panel.finish(true, &payload));
    let snapshot = panel
        .snapshot
        .ok_or("C11 payload did not install a snapshot")?;
    assert_eq!(
        format!(
            "active={} melee={} shielding={}\n",
            u8::from(snapshot.active),
            snapshot.melee,
            snapshot.shielding
        ),
        expected
    );
    let visible = panel_text(&panel);
    assert!(visible.contains(if snapshot.active {
        if snapshot.melee == 0 && snapshot.shielding == 0 {
            "Battle Instinct: Eligible"
        } else {
            "Battle Instinct: Active"
        }
    } else {
        "Battle Instinct: Inactive"
    }));

    panel.pending = true;
    assert!(!panel.finish(true, &[2]));
    assert_eq!(panel.snapshot, Some(snapshot));
    assert_eq!(
        panel.error,
        Some("Battle Instinct could not be refreshed. Try Refresh.")
    );
    Ok(())
}
