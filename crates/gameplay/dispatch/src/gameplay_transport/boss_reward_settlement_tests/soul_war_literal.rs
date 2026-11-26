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
                    // Ordinary recipients must also damage the boss; this one
                    // qualifies while healing and blocking the other attacks.
                    participant(20, 1, 100, 100),
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

#[test]
fn pinned_private_reward_rows_have_active_item_policies() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::gameplay_transport::item_ref_admission::generation_item_facts;
    use oteryn_gameplay_domain::domain::loot_data::LootTableEntry;

    fn missing_items(
        native: &crate::content::native_gameplay::NativeGameplayState,
        creature: &str,
        entries: &[LootTableEntry],
        missing: &mut Vec<String>,
    ) {
        for entry in entries {
            let definition = TypedDefinitionRef {
                family: entry.item.family.clone(),
                production_key: entry.item.production_key.clone(),
                revision_ref: entry.item.revision_ref.clone(),
            };
            match generation_item_facts(native, &definition) {
                Ok(facts) if facts.stack != ItemStackClass::Unknown => {}
                outcome => missing.push(format!(
                    "{creature}/{}: {outcome:?}",
                    entry.item.production_key
                )),
            }
            missing_items(native, creature, &entry.details.children, missing);
        }
    }

    std::thread::Builder::new()
        .name("pinned-private-reward-closure".into())
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let world =
                oteryn_protocol_oteryn::WorldId::decode(&id(1)).map_err(|e| format!("{e:?}"))?;
            let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../content/spells.manifest.json");
            let native = crate::content::native_gameplay::retained_callback_native_fixture_at(
                world, &manifest,
            );
            let rewards = native.reward_table();
            for (creature, item) in [
                ("oteryn:creature.king_zelos", "oteryn:item.tibia.i2852"),
                (
                    "oteryn:creature.the_enraged_thorn_knight",
                    "oteryn:item.tibia.i3295",
                ),
                ("oteryn:creature.the_many", "oteryn:item.tibia.i9606"),
            ] {
                let reward = rewards
                    .row(creature)
                    .map_err(|reason| format!("{creature}: {reason:?}"))?;
                assert!(
                    reward.loot_table.entries.iter().any(|entry| {
                        entry.item.production_key == item
                            && entry.probability_ppm.is_some_and(|chance| chance > 0)
                    }),
                    "{creature}: missing authored reward {item}"
                );
            }
            let mut missing = Vec::new();
            for policy in native
                .policies()
                .policies()
                .iter()
                .filter(|p| p.reward_boss == Some(true))
            {
                if let Ok(reward) = rewards.row(&policy.definition_key) {
                    missing_items(
                        &native,
                        &policy.definition_key,
                        &reward.loot_table.entries,
                        &mut missing,
                    );
                }
            }
            assert!(missing.is_empty(), "private reward Items: {missing:?}");
            Ok(())
        })?
        .join()
        .map_err(|_| "private reward closure test thread failed")??;
    Ok(())
}
