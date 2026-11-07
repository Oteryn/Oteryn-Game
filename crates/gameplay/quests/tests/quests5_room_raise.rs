//! Source room entry grants an early stage without lowering a later legal stage.
use oteryn_gameplay_quests::quest::{
    QuestRefusal, loader::load_embedded_quest_state, world_callbacks,
};
use std::collections::BTreeMap;
#[test]
fn room_entry_preserves_later_legal_progress_and_keeps_default_start_guard() -> Result<(), String> {
    let loaded =
        load_embedded_quest_state("source-room-raise-test").map_err(|e| format!("{e:?}"))?;
    let catalogue = loaded.catalogue();
    let callbacks = world_callbacks::load()?;
    let prefix = "oteryn:quest-progress/quest/u7_4/the_ancient_tombs/";
    for (suffix, uid, target) in [
        ("morguthis-entry", 40085, 1),
        ("omruc-specific", 40086, 1),
        ("omruc-room", 40087, 2),
        ("rahemos-levers", 40088, 1),
        ("rahemos-room", 40089, 2),
        ("vashresamun-specific", 40090, 1),
        ("vashresamun-room", 40091, 3),
    ] {
        let callback = callbacks
            .iter()
            .find(|c| c.key.ends_with(&format!("/ancient-tombs-{suffix}")))
            .ok_or("source room")?;
        assert_eq!(callback.selector.unique, Some(uid));
        assert!(callback.return_on_failure);
        let state = &callback.storage[1];
        assert!(state.at_least);
        assert_eq!(state.set, target);
        let transition = catalogue
            .transition(&callback.key)
            .ok_or("source transition")?;
        for before in [-1, 0, 1, 2]
            .into_iter()
            .chain((state.maximum >= 3).then_some(3))
        {
            let current = BTreeMap::from([
                (format!("{prefix}default_start"), 1),
                (state.track.clone(), before),
            ]);
            let changes = catalogue
                .evaluate(transition, &current, 0)
                .map_err(|e| format!("{e:?}"))?;
            let changed = changes
                .iter()
                .find(|c| c.track == state.track)
                .ok_or("treasure change")?;
            assert_eq!(
                (changed.before, changed.after),
                (before, before.max(target))
            );
            assert_eq!(
                changes[0].before, changes[0].after,
                "entry permission is read-only"
            );
        }
        let missing = BTreeMap::from([(state.track.clone(), 2)]);
        assert_eq!(
            catalogue.evaluate(transition, &missing, 0),
            Err(QuestRefusal::StageMismatch)
        );
    }
    Ok(())
}
