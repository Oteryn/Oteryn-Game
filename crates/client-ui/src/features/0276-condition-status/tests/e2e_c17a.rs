//! Replays SCENARIO_MATRIX C17a server payloads through the owning client seam.
//! Payload consumer: `ConditionStatusPanel::finish` in
//! `crates/client-ui/src/features/0276-condition-status/state.rs`; client-visible projection:
//! `panel_text`/`status_text` in `crates/client-ui/src/features/0276-condition-status/view.rs`.

use super::*;

#[test]
fn replay_haste_and_strong_haste_condition_payloads() -> Result<(), Box<dyn std::error::Error>> {
    let (haste, strong, expected) = if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        (
            std::fs::read(path.join("c17a-haste.bin"))?,
            std::fs::read(path.join("c17a-strong-haste.bin"))?,
            std::fs::read_to_string(path.join("c17a-expected.txt"))?,
        )
    } else {
        let encode = |remaining_ms| {
            ConditionStatus {
                conditions: vec![ActiveCondition {
                    icon: 13,
                    sub_id: 0,
                    remaining_ms,
                }],
            }
            .encode()
            .ok_or("condition fixture encode failed")
        };
        (
            encode(10_000)?,
            encode(20_000)?,
            "» Haste: 10s\r\n\n---\n» Haste: 20s\r\n\n".into(),
        )
    };

    let mut panel = ConditionStatusPanel {
        selected: true,
        pending: true,
        ..ConditionStatusPanel::default()
    };
    assert!(panel.finish(true, &haste));
    let haste_text = panel_text(&panel);
    panel.pending = true;
    assert!(panel.finish(true, &strong));
    let strong_text = panel_text(&panel);
    assert_eq!(format!("{haste_text}\n---\n{strong_text}\n"), expected);

    let before = panel.snapshot.clone();
    panel.pending = true;
    assert!(!panel.finish(true, &[1, 13]));
    assert_eq!(panel.snapshot, before);
    assert_eq!(
        panel.error,
        Some("Conditions could not be refreshed. Try Refresh.")
    );
    Ok(())
}
