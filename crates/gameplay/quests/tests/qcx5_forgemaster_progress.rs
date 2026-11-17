//! Exercise the existing quest-state transitions in the admitted catalogue.
use oteryn_gameplay_quests::quest::{QuestRefusal, QuestTrackChange, loader};
use std::collections::BTreeMap;

#[test]
fn forgemaster_report_requires_the_kill_stage_and_cannot_repeat() -> Result<(), String> {
    const TRACK: &str = "oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked/forgemaster_door/questline";
    const PREFIX: &str = "oteryn:quest-transition/crystalserver/quest/u14_10/no_rest_for_the_wicked/forgemaster_door/questline/";
    let loaded = loader::load_embedded_quest_state("qcx5-forgemaster")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let kill = catalogue
        .transition(&format!("{PREFIX}creature_event_1"))
        .ok_or("missing kill transition")?;
    let report = catalogue
        .transition(&format!("{PREFIX}npc_1"))
        .ok_or("missing report transition")?;
    let mut stored = BTreeMap::new();
    assert_eq!(
        catalogue.evaluate(report, &stored, 0),
        Err(QuestRefusal::StageMismatch)
    );

    for (transition, before, after) in [(kill, 0, 2), (report, 2, 3)] {
        let changes = catalogue
            .evaluate(transition, &stored, 0)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            changes,
            [QuestTrackChange {
                track: TRACK.into(),
                before,
                after
            }]
        );
        for change in changes {
            stored.insert(change.track, change.after);
        }
        assert_eq!(
            catalogue.evaluate(transition, &stored, 0),
            Err(QuestRefusal::StageMismatch)
        );
    }
    assert_eq!(
        catalogue.evaluate(kill, &stored, 0),
        Err(QuestRefusal::StageMismatch)
    );
    Ok(())
}
