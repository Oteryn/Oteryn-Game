//! Actual production personal-loot planner, exact retained source profile.
use super::*;

#[test]
fn srx3_soul_war_personal_loot_keeps_literal_local_chance_for_every_recipient()
-> Result<(), Box<dyn std::error::Error>> {
    std::thread::Builder::new()
        .name("srx3-soul-war-literal-loot".into())
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let world =
                oteryn_protocol_oteryn::WorldId::decode(&id(1)).map_err(|e| format!("{e:?}"))?;
            let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../content/spells.manifest.json");
            let native = crate::content::native_gameplay::retained_callback_native_fixture_at(
                world, &manifest,
            );
            let creature = "oteryn:creature.goshnar_s_megalomania_blue";
            let reward = native
                .reward_table()
                .row(creature)
                .map_err(|e| format!("{e:?}"))?;
            let bag = "oteryn:item.tibia.i34109";
            let probability = reward
                .loot_table
                .entries
                .iter()
                .find(|entry| entry.item.production_key == bag)
                .and_then(|entry| entry.probability_ppm)
                .ok_or("actual source Bag you desire leaf")?;
            assert_eq!(
                probability, 1000,
                "source chance100/100000 is1000ppm; ignored metadata is not the roll chance"
            );
            let frozen = FrozenBossDeath {
                participants: vec![
                    participant(10, 100, 0, 0),
                    participant(20, 0, 100, 100),
                    participant(30, 1, 0, 0),
                ],
                reward,
                died_at_unix_ms: 1_000_000,
            };
            let plan = |frozen: &FrozenBossDeath| {
                plan_personal_death_with_slots(
                    id(90),
                    creature,
                    frozen,
                    |definition| {
                        crate::gameplay_transport::item_ref_admission::generation_item_facts(
                            &native, definition,
                        )
                        .ok()
                    },
                    &[],
                    |key| native.reward_table().is_daily_boss_equipment(key),
                )
                .map_err(|e| format!("{e:?}"))
            };
            let actual = plan(&frozen)?;
            let replay = plan(&frozen)?;
            assert_eq!(
                actual, replay,
                "all personal draws retain original immutable source chance"
            );
            assert!(
                actual
                    .chests
                    .iter()
                    .any(|chest| chest.character_id.as_bytes() == &id(20)),
                "healing/blocking recipient also uses the same source table"
            );
            assert!(
                actual.chests.iter().all(|chest| chest
                    .items
                    .iter()
                    .all(|item| item.definition.production_key != bag)),
                "this retained death's real source rolls do not award a bag"
            );
            // Source helper at counter9990 would report chance100000, but that
            // metadata is never used. A balance repair would observably change
            // this SAME production planner and SAME frozen death/recipients.
            let mut repaired = frozen.clone();
            let mut row = (*repaired.reward).clone();
            row.loot_table
                .entries
                .iter_mut()
                .find(|entry| entry.item.production_key == bag)
                .ok_or("source leaf")?
                .probability_ppm = Some(1_000_000);
            repaired.reward = Arc::new(row);
            let counterfactual = plan(&repaired)?;
            assert!(
                counterfactual.chests.iter().any(|chest| chest
                    .items
                    .iter()
                    .any(|item| item.definition.production_key == bag)),
                "using the ignored pity metadata would alter actual personal loot"
            );
            Ok(())
        })?
        .join()
        .map_err(|_| "literal source loot test thread failed")??;
    Ok(())
}
