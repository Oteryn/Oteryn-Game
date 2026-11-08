//! Replays SCENARIO_MATRIX C16b (Paralyze rune) through the owning client seam.
//! Payload consumer: `ConditionStatusPanel::finish` in
//! `crates/client-ui/src/features/0276-condition-status/state.rs`; projection: `panel_text` in
//! that feature's `view.rs`. The payload is the paralysed player's own condition-query reply.

use super::*;

#[test]
fn replay_paralyze_condition_payload() -> Result<(), Box<dyn std::error::Error>> {
    let (payload, expected) = if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        (
            std::fs::read(path.join("c16b-condition.bin"))?,
            std::fs::read_to_string(path.join("c16b-expected.txt"))?,
        )
    } else {
        (
            ConditionStatus {
                conditions: vec![ActiveCondition {
                    icon: 14,
                    sub_id: 0,
                    remaining_ms: 5_400,
                }],
            }
            .encode()
            .ok_or("condition fixture encode failed")?,
            "≪ Paralysed: 6s\r\n".to_owned(),
        )
    };

    let mut panel = ConditionStatusPanel {
        selected: true,
        pending: true,
        ..ConditionStatusPanel::default()
    };
    assert!(panel.finish(true, &payload));
    let status = panel.snapshot.clone().ok_or("snapshot absent")?;
    assert!(
        status
            .conditions
            .iter()
            .any(|condition| condition.icon == 14 && condition.remaining_ms > 0)
    );
    assert_eq!(panel_text(&panel), expected);

    // An unusable reply is refused and leaves the last authoritative snapshot untouched.
    let before = panel.snapshot.clone();
    panel.pending = true;
    assert!(!panel.finish(true, &[1, 14]));
    assert_eq!(panel.snapshot, before);
    assert_eq!(
        panel.error,
        Some("Conditions could not be refreshed. Try Refresh.")
    );
    // A refused command also keeps the snapshot.
    panel.pending = true;
    assert!(!panel.finish(false, &payload));
    assert_eq!(panel.snapshot, before);
    Ok(())
}
