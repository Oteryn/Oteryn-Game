//! The trough expression is unresolved in the shipped catalogue. It must
//! refuse to propose counter changes even after the admitted NPC reset.
use oteryn_gameplay_quests::quest::{QuestRefusal, QuestTrackChange, loader};
use std::collections::BTreeMap;

#[test]
fn roshamuul_unresolved_trough_refuses_credit_before_and_after_reset() -> Result<(), String> {
    const TRACK: &str =
        "oteryn:quest-progress/quest/u10_30/roshamuul_quest/roshamuul_mortar_thrown";
    const PREFIX: &str =
        "oteryn:quest-transition/quest/u10_30/roshamuul_quest/roshamuul_mortar_thrown/";
    let loaded = loader::load_embedded_quest_state("qcx5-roshamuul")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let trough = catalogue
        .transition(&format!("{PREFIX}action_1"))
        .ok_or("missing trough transition")?;
    let reset = catalogue
        .transition(&format!("{PREFIX}npc_1"))
        .ok_or("missing reset transition")?;
    let mut stored = BTreeMap::from([(TRACK.to_owned(), 7)]);
    assert_eq!(
        catalogue.evaluate(trough, &stored, 0),
        Err(QuestRefusal::NotSupported)
    );
    let changes = catalogue
        .evaluate(reset, &stored, 0)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        changes,
        [QuestTrackChange {
            track: TRACK.into(),
            before: 7,
            after: 0
        }]
    );
    for change in changes {
        stored.insert(change.track, change.after);
    }
    assert_eq!(
        catalogue.evaluate(trough, &stored, 1),
        Err(QuestRefusal::NotSupported)
    );
    assert_eq!(
        catalogue.evaluate(reset, &stored, 2),
        Ok(vec![QuestTrackChange {
            track: TRACK.into(),
            before: 0,
            after: 0
        }])
    );
    Ok(())
}
