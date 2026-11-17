use crate::quest::loader::load_embedded_quest_state;
use std::collections::BTreeMap;

#[test]
fn completed_venore_circuit_report_preserves_kills_and_sets_clock() -> Result<(), String> {
    let loaded = load_embedded_quest_state("content-1").map_err(|e| format!("{e:?}"))?;
    let catalogue = loaded.catalogue();
    let transition = catalogue
        .transition("oteryn:quest-reward/chosen/venore-daily/report")
        .ok_or("report transition")?;
    let prefix = "oteryn:quest-progress/chosen/venore-daily/";
    let stored = BTreeMap::from([
        (format!("{prefix}stage"), 12),
        (format!("{prefix}kills"), 5),
        (format!("{prefix}last-report"), 0),
    ]);
    let changes = catalogue
        .evaluate(transition, &stored, 1_800_000_000)
        .map_err(|e| format!("{e:?}: {:?}", transition.effects))?;
    for (track, after) in [("stage", 13), ("kills", 5), ("last-report", 1_800_000_000)] {
        assert!(
            changes
                .iter()
                .any(|change| change.track == format!("{prefix}{track}") && change.after == after)
        );
    }
    Ok(())
}
