use super::{TestResult, debug, fixture::id};

#[test]
fn questpc7_wayland_callbacks_require_premium_and_level_without_make_believe() -> TestResult {
    use oteryn_gameplay_quests::quest::{loader, reward_callbacks};

    let loaded = loader::load_embedded_quest_state("content-1").map_err(debug)?;
    let callbacks = reward_callbacks::offered_for(loaded.catalogue()).map_err(debug)?;
    for stage in ["intro", "basic", "insignia", "halo"] {
        let key = format!("oteryn:quest-reward/questpc7-illuminated-{stage}");
        let callback = callbacks.iter().find(|row| row.key == key).ok_or(key)?;
        let requirements = callback
            .requirements
            .as_ref()
            .ok_or("Wayland requirements")?;
        assert_eq!(requirements.min_level, 1000);
        assert!(requirements.premium);
        assert!(
            requirements.completed_quests.is_empty(),
            "Wayland {stage} must be available without Make Believe"
        );
    }
    Ok(())
}

#[test]
fn questpc7_wayland_scrap_bosses_admit_personal_loot_and_empty_corpses() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("wayland-scrap-bosses", || {
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, _) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let native = controller
            .active()
            .and_then(|generation| generation.native_gameplay())
            .ok_or("native gameplay")?;
        let rewards = native.reward_table();
        for (boss, corpse) in [
            ("oteryn:creature.maior_domus", "oteryn:item.tibia.i54433"),
            ("oteryn:creature.mimar_haffar", "oteryn:item.tibia.i54346"),
        ] {
            let row = rewards
                .row(boss)
                .map_err(|reason| format!("{boss}: personal reward row refused: {reason:?}"))?;
            assert_eq!(
                row.corpse_item
                    .as_ref()
                    .map(|item| item.production_key.as_str()),
                Some(corpse)
            );
            assert!(
                row.loot_table
                    .entries
                    .iter()
                    .any(|entry| entry.item.production_key == "oteryn:item.tibia.i54640"),
                "{boss}: personal loot must include Radiant Attire scraps"
            );
            let empty = rewards
                .reward_boss_corpse(boss)
                .ok_or("empty boss corpse")?;
            assert_eq!(empty.corpse_item, row.corpse_item);
            assert!(empty.loot_table.entries.is_empty());
        }
        Ok(())
    })
}
