//! Crystal 201189f871d23597b82caf88fd611454a2ddefb8:
//! data-global/scripts/quests/demon_oak/movements_squares.lua:19-25.
use oteryn_gameplay_quests::quest::{QuestRefusal, loader, world_callbacks};
use std::collections::BTreeMap;

#[test]
fn demon_oak_sequence_accepts_unset_or_zero_but_never_skips_a_square() -> Result<(), String> {
    let lowered =
        loader::load_embedded_quest_state("quests5-test").map_err(|error| format!("{error:?}"))?;
    let catalogue = lowered.catalogue();
    let track = "oteryn:quest-progress/quest/u8_2/the_demon_oak/squares";
    for start in [-1, 0] {
        let mut stored = BTreeMap::from([(track.to_owned(), start)]);
        for square in 1..=5 {
            let key = format!("oteryn:quest-callback/demon-oak-square-{square}");
            let transition = catalogue.transition(&key).ok_or("square transition")?;
            let changes = catalogue
                .evaluate(transition, &stored, 0)
                .map_err(|error| format!("square {square}, start {start}: {error:?}"))?;
            assert_eq!(changes.len(), 1);
            assert_eq!(changes[0].after, square);
            assert_eq!(changes[0].track, track);
            stored.insert(track.to_owned(), changes[0].after);
            assert_eq!(
                catalogue.evaluate(transition, &stored, 0),
                Err(QuestRefusal::StageMismatch)
            );
        }
    }
    let second = catalogue
        .transition("oteryn:quest-callback/demon-oak-square-2")
        .ok_or("second square")?;
    assert_eq!(
        catalogue.evaluate(second, &BTreeMap::from([(track.to_owned(), -1)]), 0),
        Err(QuestRefusal::StageMismatch)
    );
    Ok(())
}

/// actions_demon_oak_gravestone.lua:2-11 requires Oldrak's exact Done=2.
#[test]
fn demon_oak_grave_does_not_complete_or_claim_the_reward() -> Result<(), String> {
    let callback = world_callbacks::load()?
        .into_iter()
        .find(|callback| callback.key == "oteryn:quest-callback/demon-oak-reward-grave")
        .ok_or("grave callback")?;
    let lowered =
        loader::load_embedded_quest_state("quests5-test").map_err(|error| format!("{error:?}"))?;
    let catalogue = lowered.catalogue();
    let transition = catalogue
        .transition(&callback.key)
        .ok_or("grave transition")?;
    let track = "oteryn:quest-progress/quest/u8_2/the_demon_oak/done";
    for value in [-1, 0, 1, 3] {
        assert_eq!(
            catalogue.evaluate(transition, &BTreeMap::from([(track.to_owned(), value)]), 0),
            Err(QuestRefusal::StageMismatch)
        );
    }
    let changes = catalogue
        .evaluate(transition, &BTreeMap::from([(track.to_owned(), 2)]), 0)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!((changes[0].before, changes[0].after), (2, 2));
    assert!(!transition.completes);
    assert!(transition.experience.is_none());
    Ok(())
}
