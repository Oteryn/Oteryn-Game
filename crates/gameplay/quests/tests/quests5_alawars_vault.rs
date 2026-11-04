//! Source: zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:
//! data-global/startup/tables/teleport.lua:671-684 and
//! data-global/scripts/movements/others/teleport.lua:4-20.
//! Both source portals use item 1949 and perform an unconditional player teleport.
use oteryn_gameplay_quests::quest::world_callbacks::{self, Position, Trigger};

#[test]
fn alawars_vault_teleport_pair_preserves_source_selectors_and_destinations() -> Result<(), String> {
    let callbacks = world_callbacks::load()?;
    for (key, origin, destination, source) in [
        (
            "oteryn:quest-callback/alawars-vault-entrance",
            Position {
                x: 32187,
                y: 31622,
                floor: -8,
            },
            Position {
                x: 32107,
                y: 31567,
                floor: -9,
            },
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/teleport.lua:671-677",
        ),
        (
            "oteryn:quest-callback/alawars-vault-exit",
            Position {
                x: 32107,
                y: 31566,
                floor: -9,
            },
            Position {
                x: 32189,
                y: 31625,
                floor: -4,
            },
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/teleport.lua:679-684",
        ),
    ] {
        let callback = callbacks
            .iter()
            .find(|callback| callback.key == key)
            .ok_or_else(|| format!("Alawar portal is missing: {key}"))?;
        assert_eq!(callback.quest, "oteryn:quest.alawar_s_vault_quest");
        assert_eq!(callback.trigger, Trigger::StepIn);
        assert_eq!(callback.selector.position, Some(origin));
        assert_eq!(callback.selector.appearance, Some(1949));
        assert!(callback.selector.unique.is_none());
        assert!(callback.selector.action.is_none());
        assert_eq!(callback.teleport, Some(destination));
        assert!(callback.storage.is_empty());
        assert!(callback.minimum_level.is_none());
        assert!(callback.offerings.is_empty());
        assert!(callback.item_on_target.is_none());
        assert!(callback.experience.is_none());
        assert!(callback.monk_reward.is_none());
        assert!(callback.lever.is_none());
        assert!(!callback.reversible);
        assert!(callback.passages.is_empty());
        assert!(!callback.return_on_failure);
        assert!(callback.failure_message.is_none());
        assert!(callback.sources.iter().any(|entry| entry == source));
    }
    Ok(())
}
