//! Regression for the existing Oteryn item-use stages and their repeat guards.
use oteryn_gameplay_quests::quest::{QuestRefusal, loader, world_callbacks};
use std::collections::BTreeMap;

#[test]
fn gravedigger_item_stages_require_the_predecessor_and_refuse_a_fresh_repeat() -> Result<(), String>
{
    let callbacks = world_callbacks::load()?;
    let loaded = loader::load_embedded_quest_state("gravedigger-existing-item-guards")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    for (suffix, action, item, count, predecessor, next) in [
        (
            "ashes",
            4638,
            "oteryn:item.tibia.i19129",
            1,
            "mission28",
            "mission29",
        ),
        (
            "hallowed",
            4634,
            "oteryn:item.tibia.i19089",
            1,
            "mission19",
            "mission20",
        ),
        (
            "monk-4641",
            4641,
            "oteryn:item.tibia.i18931",
            1,
            "mission32",
            "mission32a",
        ),
        (
            "monk-4642",
            4642,
            "oteryn:item.tibia.i18931",
            1,
            "mission32a",
            "mission32b",
        ),
        (
            "monk-4643",
            4643,
            "oteryn:item.tibia.i18931",
            1,
            "mission32b",
            "mission33",
        ),
        (
            "tears",
            4632,
            "oteryn:item.tibia.i19084",
            3,
            "mission14",
            "mission15",
        ),
    ] {
        let key = format!("oteryn:quest-callback/gravedigger-{suffix}");
        let callback = callbacks
            .iter()
            .find(|callback| callback.key == key)
            .ok_or_else(|| format!("existing callback missing: {key}"))?;
        assert_eq!(callback.trigger, world_callbacks::Trigger::Use);
        assert_eq!(callback.selector.action, Some(action));
        let input = callback
            .item_on_target
            .as_ref()
            .ok_or("existing item input")?;
        assert_eq!(input.item, item);
        assert_eq!(input.count, count);
        assert!(input.replacement.is_none());
        assert!(input.target_replacement.is_none());
        let transition = catalogue.transition(&key).ok_or("existing transition")?;
        let prefix = "oteryn:quest-progress/quest/u10_10/the_gravedigger_of_drefia/";
        let predecessor = format!("{prefix}{predecessor}");
        let next = format!("{prefix}{next}");
        for value in [-1, 0, 2] {
            assert_eq!(
                catalogue.evaluate(
                    transition,
                    &BTreeMap::from([(predecessor.clone(), value)]),
                    0
                ),
                Err(QuestRefusal::StageMismatch),
                "wrong predecessor for {key}"
            );
        }
        for value in [-1, 0] {
            let mut stored = BTreeMap::from([(predecessor.clone(), 1), (next.clone(), value)]);
            let changes = catalogue
                .evaluate(transition, &stored, 0)
                .map_err(|error| format!("{key}: {error:?}"))?;
            assert_eq!(changes.len(), 2);
            assert!(changes.iter().any(|change| change.track == predecessor
                && change.before == 1
                && change.after == 1));
            assert!(
                changes.iter().any(|change| change.track == next
                    && change.before == value
                    && change.after == 1)
            );
            for change in changes {
                stored.insert(change.track, change.after);
            }
            assert_eq!(
                catalogue.evaluate(transition, &stored, 0),
                Err(QuestRefusal::StageMismatch),
                "a fresh repeated use cannot advance or consume another input for {key}"
            );
        }
    }
    Ok(())
}
