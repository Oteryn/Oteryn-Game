use super::{CyclopediaTrackerPanel, apply_snapshot, bestiary_rows};
use std::path::Path;

#[test]
fn e2e_p22c_replays_tracker_reads_after_fresh_admissions() -> Result<(), std::io::Error> {
    let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    for (name, tracked) in [
        ("p22c-after-add-relog.bin", true),
        ("p22c-after-remove-relog.bin", false),
    ] {
        let bytes = std::fs::read(Path::new(&directory).join(name))?;
        let mut panel = CyclopediaTrackerPanel::default();
        assert!(apply_snapshot(&mut panel, &bytes), "{name}");
        assert_eq!(
            bestiary_rows(&panel)
                .iter()
                .any(|entry| entry.creature_key == "oteryn:creature.rat"),
            tracked,
            "{name}",
        );
    }
    Ok(())
}
