//! Source: zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:
//! data-global/scripts/quests/blood_brothers_quest/movements_castle_entrace.lua:10-19.
//! The donor reads CastleHiddenEntrance == 1; entering never grants that access.
use oteryn_gameplay_quests::quest::{QuestRefusal, QuestTrackChange, loader, world_callbacks};
use std::collections::BTreeMap;

const CALLBACK: &str = "oteryn:quest-callback/blood-brothers-quest-castle-entrance";
const ACCESS: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_4/blood_brothers/castle_hidden_entrance";

#[test]
fn blood_castle_requires_existing_exact_access_without_granting_it() -> Result<(), String> {
    let callback = world_callbacks::load()?
        .into_iter()
        .find(|callback| callback.key == CALLBACK)
        .ok_or("Blood castle callback is missing")?;
    assert_eq!(callback.trigger, world_callbacks::Trigger::StepIn);
    assert!(callback.return_on_failure);
    assert_eq!(
        callback.failure_message.as_deref(),
        Some("A strange force prevents you from entering.")
    );
    assert_eq!(
        callback.teleport,
        Some(world_callbacks::Position {
            x: 32953,
            y: 31483,
            floor: -6,
        })
    );
    assert!(callback.offerings.is_empty());
    assert!(callback.item_on_target.is_none());
    assert!(callback.monk_reward.is_none());
    assert!(callback.experience.is_none());

    let loaded = loader::load_embedded_quest_state("quests5-blood-access")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let transition = catalogue.transition(CALLBACK).ok_or("Blood transition")?;
    assert!(!transition.completes);
    assert!(transition.experience.is_none());
    assert_eq!(
        catalogue.evaluate(transition, &BTreeMap::new(), 0),
        Err(QuestRefusal::StageMismatch),
        "missing access must not become a grant"
    );
    for value in [-1, 0, 2] {
        assert_eq!(
            catalogue.evaluate(transition, &BTreeMap::from([(ACCESS.into(), value)]), 0),
            Err(QuestRefusal::StageMismatch),
            "CastleHiddenEntrance={value} must refuse, not partially change state"
        );
    }
    assert_eq!(
        catalogue.evaluate(transition, &BTreeMap::from([(ACCESS.into(), 1)]), 0),
        Ok(vec![QuestTrackChange {
            track: ACCESS.into(),
            before: 1,
            after: 1,
        }]),
        "the qualified passage preserves the existing canonical access only"
    );
    Ok(())
}
