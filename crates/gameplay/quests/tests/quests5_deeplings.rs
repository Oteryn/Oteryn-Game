//! The three source gates are movement only; they do not start a boss encounter.
//! Source: zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:
//! data-global/scripts/quests/deeplings_worldchange/movements_deepling_bosses.lua:1-31.

use std::collections::BTreeMap;

use oteryn_gameplay_quests::quest::{
    QuestRefusal, loader::load_embedded_quest_state, world_callbacks,
};

const CRYSTAL: &str = "oteryn:quest-progress/deeplings_world_change/crystal";

#[test]
fn all_deepling_gates_require_exactly_thirteen_and_preserve_thirteen() -> Result<(), String> {
    let loaded = load_embedded_quest_state("quests5-deeplings-source-test")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    for uid in [28574, 28575, 28576] {
        let key = format!("oteryn:quest-callback/deeplings-boss-{uid}");
        let transition = catalogue.transition(&key).ok_or("source gate callback")?;
        for stage in [-1, 0, 1, 12, 14] {
            assert_eq!(
                catalogue.evaluate(transition, &BTreeMap::from([(CRYSTAL.into(), stage)]), 0),
                Err(QuestRefusal::StageMismatch),
                "uid {uid} must reject Crystal={stage}"
            );
        }
        let state = BTreeMap::from([(CRYSTAL.into(), 13)]);
        let changes = catalogue
            .evaluate(transition, &state, 0)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].track, CRYSTAL);
        assert_eq!((changes[0].before, changes[0].after), (13, 13));
        assert!(!transition.completes);
        assert_eq!(transition.experience, None);
    }
    Ok(())
}

#[test]
fn deepling_gate_data_only_teleports_to_source_destinations() -> Result<(), String> {
    let callbacks = world_callbacks::load().map_err(|error| format!("{error:?}"))?;
    for (uid, x, y) in [
        (28574, 33641, 31236),
        (28575, 33421, 31255),
        (28576, 33543, 31263),
    ] {
        let key = format!("oteryn:quest-callback/deeplings-boss-{uid}");
        let found = callbacks
            .iter()
            .filter(|value| value.key == key)
            .collect::<Vec<_>>();
        assert_eq!(found.len(), 1);
        let callback = found[0];
        assert_eq!(callback.trigger, world_callbacks::Trigger::StepIn);
        assert_eq!(callback.selector.unique, Some(uid));
        assert_eq!(callback.selector.action, None);
        assert_eq!(
            callback.teleport,
            Some(world_callbacks::Position { x, y, floor: -11 })
        );
        assert!(callback.return_on_failure);
        assert_eq!(callback.storage.len(), 1);
        assert_eq!(callback.storage[0].track, CRYSTAL);
        assert!(callback.passages.is_empty());
        assert!(callback.offerings.is_empty());
        assert_eq!(callback.lever, None);
        assert_eq!(callback.item_on_target, None);
        assert_eq!(callback.experience, None);
        assert_eq!(callback.monk_reward, None);
        assert!(!callback.reversible);
    }
    Ok(())
}
