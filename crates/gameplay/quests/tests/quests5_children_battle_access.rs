//! Source: zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:
//! data-global/scripts/quests/children_of_the_revolution/movements_teleport.lua:1-25.
//! Both directions read teleportAccess == 1; neither grants access, items or XP.
use oteryn_gameplay_quests::quest::{QuestRefusal, QuestTrackChange, loader, world_callbacks};
use std::collections::BTreeMap;

const ACCESS: &str = "oteryn:quest-progress/quest/u8_54/children_of_the_revolution/teleport_access";

#[test]
fn both_children_battle_passages_preserve_the_same_existing_access() -> Result<(), String> {
    let callbacks = world_callbacks::load()?;
    let loaded = loader::load_embedded_quest_state("quests5-children-access")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    for (key, destination) in [
        (
            "oteryn:quest-callback/children-of-the-revolution-battle-passage-1",
            world_callbacks::Position {
                x: 33261,
                y: 31077,
                floor: -8,
            },
        ),
        (
            "oteryn:quest-callback/children-of-the-revolution-battle-passage-2",
            world_callbacks::Position {
                x: 33356,
                y: 31126,
                floor: -7,
            },
        ),
    ] {
        let callback = callbacks
            .iter()
            .find(|callback| callback.key == key)
            .ok_or_else(|| format!("Children battle passage is missing: {key}"))?;
        assert_eq!(callback.trigger, world_callbacks::Trigger::StepIn);
        assert_eq!(callback.teleport, Some(destination));
        assert!(callback.return_on_failure);
        assert!(callback.offerings.is_empty());
        assert!(callback.item_on_target.is_none());
        assert!(callback.monk_reward.is_none());
        assert!(callback.experience.is_none());
        let transition = catalogue.transition(key).ok_or("Children transition")?;
        assert!(!transition.completes);
        assert!(transition.experience.is_none());
        assert_eq!(
            catalogue.evaluate(transition, &BTreeMap::new(), 0),
            Err(QuestRefusal::StageMismatch)
        );
        for value in [-1, 0, 2] {
            assert_eq!(
                catalogue.evaluate(transition, &BTreeMap::from([(ACCESS.into(), value)]), 0),
                Err(QuestRefusal::StageMismatch),
                "{key}: teleportAccess={value} refuses the whole transition"
            );
        }
        assert_eq!(
            catalogue.evaluate(transition, &BTreeMap::from([(ACCESS.into(), 1)]), 0),
            Ok(vec![QuestTrackChange {
                track: ACCESS.into(),
                before: 1,
                after: 1,
            }]),
            "{key}: no other state change or invented access is permitted"
        );
    }
    Ok(())
}
