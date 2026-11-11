//! Reused native activation fixture and LOOP-MONSTER source-profile variants.
use super::*;
#[cfg(test)]
#[path = "source_loot_assertions.rs"]
mod source_loot_assertions;
#[cfg(test)]
#[path = "source_loot_refusals.rs"]
mod source_loot_refusals;

fn manifest_input() -> NativeGameplayInput {
    let manifest = Path::new(if env!("CARGO_PKG_NAME") == "oteryn-game-server" {
        env!("CARGO_MANIFEST_DIR")
    } else {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../../apps/game-server")
    })
    .join("../../content/spells.manifest.json");
    NativeGameplayInput::from_manifest(&manifest).unwrap()
}
fn activate(
    world: oteryn_protocol_oteryn::WorldId,
    supplied: &NativeGameplayInput,
) -> (
    ContentActivationController,
    crate::content::NativeEntryContentPin,
) {
    try_activate_input(world, supplied).unwrap()
}

/// Qualify a fixture once, then use the same sealed activation checks as the node entry path.
/// Fixture issuance is derived from that qualified room; production issuances remain independent.
pub fn try_activate_input(
    world: oteryn_protocol_oteryn::WorldId,
    supplied: &NativeGameplayInput,
) -> Result<
    (
        ContentActivationController,
        crate::content::NativeEntryContentPin,
    ),
    crate::content::NativeEntryActivationError,
> {
    use crate::content::NativeEntryActivationError;
    supplied
        .validate_cosmetics_pin()
        .map_err(crate::content::ProjectError::from)
        .map_err(NativeEntryActivationError::Qualification)?;
    let room = crate::content::qualify_selected_native_gameplay_room(world, supplied)
        .map_err(NativeEntryActivationError::Qualification)?;
    let issuance = NativeEntryActivationIssuance {
        world_id: world,
        activation_sequence: 1,
        server_artifact_digest: room.compiled().server_digest(),
        client_artifact_digest: room.compiled().client_digest(),
        frame_binding_digest: room.frame_binding().digest(),
    };
    let mut controller = ContentActivationController::new();
    let pin = crate::content::activation::activate_qualified_native_entry_room(
        &mut controller,
        &NodeBootQuiescence::before_channel_runtime(),
        &issuance,
        room,
    )?;
    Ok((controller, pin))
}

pub fn activated_with_loot_tables(
    world: oteryn_protocol_oteryn::WorldId,
    source_world: Option<&[u8]>,
) -> (
    ContentActivationController,
    crate::content::NativeEntryContentPin,
) {
    activated_with_custom_loot_tables(world, source_world, None)
}

/// Explicit native reward fixture, compiled and pinned before runtime creation.
/// Reclassify the already admitted rat only in this test input; retain its source HP, XP, corpse,
/// loot probabilities, quantities and all Item definitions without modification.
pub fn activated_reward_boss_fixture(
    world: oteryn_protocol_oteryn::WorldId,
    source_world: Option<&[u8]>,
) -> (
    ContentActivationController,
    crate::content::NativeEntryContentPin,
) {
    activated_reward_boss_corpse_capacity_fixture(world, source_world, None)
}

/// Test-only pinned loot metadata variant; ordinary fixture semantics stay unchanged.
pub fn activated_reward_boss_corpse_capacity_fixture(
    world: oteryn_protocol_oteryn::WorldId,
    source_world: Option<&[u8]>,
    capacity: Option<u32>,
) -> (
    ContentActivationController,
    crate::content::NativeEntryContentPin,
) {
    let mut supplied = manifest_input();
    let mut creatures: CreatureProfilesDocument =
        serde_json::from_slice(&supplied.creature_profiles.bytes).unwrap();
    let rat = creatures
        .records
        .iter_mut()
        .find(|record| record.profile.target.key == "oteryn:creature.rat")
        .expect("source-admitted rat");
    let ProjectV2AuthoringProfileData::Creature(profile) = &mut rat.profile.data else {
        panic!("rat Creature profile");
    };
    profile
        .details
        .as_mut()
        .expect("source rat details")
        .system_eligibility
        .reward_boss = true;
    if let Some(capacity) = capacity {
        let corpse = profile
            .details
            .as_ref()
            .expect("rat details")
            .corpse_item
            .as_ref()
            .expect("rat corpse");
        let mut loot: serde_json::Value =
            serde_json::from_slice(&supplied.loot_tables.as_ref().expect("loot tables").bytes)
                .unwrap();
        let facts = loot["items"]
            .as_array_mut()
            .expect("loot items")
            .iter_mut()
            .find(|facts| facts["item"]["key"] == corpse.key)
            .expect("rat corpse facts");
        facts["container_capacity"] = serde_json::json!(capacity);
        supplied.loot_tables = Some(pinned(&serde_json::to_vec(&loot).unwrap()));
    }
    refresh_fixture_attack_roots(rat);
    supplied.creature_profiles = pinned(&serde_json::to_vec(&creatures).unwrap());
    supplied.source_world = source_world.map(pinned);
    activate(world, &supplied)
}

/// Native kneebiter fixture with a materializable corpse and deterministic equipment loot.
pub fn activated_boss_slot_loot_fixture(
    world: oteryn_protocol_oteryn::WorldId,
    source_world: Option<&[u8]>,
) -> (
    ContentActivationController,
    crate::content::NativeEntryContentPin,
) {
    let mut supplied = manifest_input();
    let mut creatures: CreatureProfilesDocument =
        serde_json::from_slice(&supplied.creature_profiles.bytes).unwrap();
    let rat_corpse = creatures
        .records
        .iter()
        .find(|record| record.profile.target.key == "oteryn:creature.rat")
        .and_then(|record| match &record.profile.data {
            ProjectV2AuthoringProfileData::Creature(profile) => profile.details.as_ref(),
            _ => None,
        })
        .and_then(|details| details.corpse_item.clone())
        .expect("source rat corpse");
    let kneebiter = creatures
        .records
        .iter_mut()
        .find(|record| record.profile.target.key == "oteryn:creature.foreman_kneebiter")
        .expect("source kneebiter");
    let ProjectV2AuthoringProfileData::Creature(profile) = &mut kneebiter.profile.data else {
        panic!("kneebiter Creature profile");
    };
    profile
        .details
        .as_mut()
        .expect("source kneebiter details")
        .corpse_item = Some(rat_corpse);
    refresh_fixture_attack_roots(kneebiter);
    supplied.creature_profiles = pinned(&serde_json::to_vec(&creatures).unwrap());

    let mut loot: serde_json::Value = serde_json::from_slice(
        &supplied
            .loot_tables
            .as_ref()
            .expect("manifest loot tables")
            .bytes,
    )
    .unwrap();
    let table = loot["tables"]
        .as_array_mut()
        .expect("loot tables array")
        .iter_mut()
        .find(|table| table["identity"]["key"] == "oteryn:loot.creature.foreman_kneebiter")
        .expect("kneebiter loot table");
    table["entries"] = serde_json::json!([
        {
            "item": {"family": "Item", "key": "oteryn:item.tibia.i3051", "revision": "definition-r1"},
            "min_count": 1,
            "max_count": 1,
            "probability_ppm": 1_000_000
        },
        {
            "item": {"family": "Item", "key": "oteryn:item.tibia.i3031", "revision": "definition-r1"},
            "min_count": 1,
            "max_count": 1,
            "probability_ppm": 1_000_000
        }
    ]);
    supplied.loot_tables = Some(pinned(&serde_json::to_vec(&loot).unwrap()));
    supplied.source_world = source_world.map(pinned);
    activate(world, &supplied)
}

#[cfg(test)]
#[test]
fn boss_slot_loot_fixture_compiles_kneebiter_row() {
    let world = test_source(1).unwrap().world_id;
    let (fixture, _) = activated_boss_slot_loot_fixture(world, None);
    let native = fixture.active().unwrap().native_gameplay().unwrap();
    let row = native
        .reward_table()
        .row("oteryn:creature.foreman_kneebiter")
        .unwrap();
    assert_eq!(row.boss.as_ref().unwrap().category, "nemesis");
    assert!(
        native
            .reward_table()
            .is_daily_boss_equipment("oteryn:item.tibia.i3051")
    );
    let policy = native
        .policies()
        .policies()
        .iter()
        .find(|policy| policy.definition_key == "oteryn:creature.foreman_kneebiter")
        .unwrap();
    assert_eq!(policy.reward_boss, Some(false));
}

#[cfg(test)]
#[test]
fn reward_boss_test_fixture_preserves_source_loot_and_production_unavailability()
-> Result<(), String> {
    use crate::content::creature_reward::{LootTablesSection, NoSettlementReason};
    use oteryn_gameplay_domain::domain::loot_data::{LootDefinitionRef, LootSelectionAlgorithm};
    let supplied = manifest_input();
    let loot: LootTablesSection = serde_json::from_slice(
        &supplied
            .loot_tables
            .as_ref()
            .ok_or("source loot tables")?
            .bytes,
    )
    .map_err(|error| error.to_string())?;
    let world = test_source(1).unwrap().world_id;
    let (source, _) = activated_with_loot_tables(world, None);
    let source = source.active().unwrap().native_gameplay().unwrap();
    let bosses = source
        .policies()
        .policies()
        .iter()
        .filter(|policy| policy.reward_boss == Some(true))
        .collect::<Vec<_>>();
    assert!(!bosses.is_empty());
    // Read the actual reward table before fixed inventory expectations. This diagnostic
    // keeps each refusal distinct while a newly generated source cohort is qualified.
    let mut reward_inventory = std::collections::BTreeMap::new();
    for policy in &bosses {
        let outcome = match source.reward_table().row(&policy.definition_key) {
            Ok(_) => "Admitted".to_owned(),
            Err(reason) => format!("{reason:?}"),
        };
        reward_inventory.insert(policy.definition_key.as_str(), outcome);
    }
    eprintln!(
        "NATIVE_REWARD_BOSS_QUALIFICATION {}",
        serde_json::to_string(&reward_inventory).map_err(|error| error.to_string())?
    );
    // Exact native corpse/yield producers qualify complete source rows.
    // A newly admitted body still requires independent private-loot closure;
    // no missing yield may be silently dropped from a boss reward.
    // Keep each exact admission reason:
    // a missing corpse is different from an authored but inadmissible corpse.
    // Change these source-qualification expectations only with admitted content.
    let newly_admitted = [
        "oteryn:creature.demodras",
        "oteryn:creature.the_percht_queen",
        "oteryn:creature.goshnar_s_spite",
    ];
    let mut missing = 0;
    let mut inadmissible = 0;
    let mut inadmissible_loot = 0;
    let mut admitted = Vec::new();
    let mut shared_body = Vec::new();
    let mut refreshed_body_loot_refusals = Vec::new();
    for policy in &bosses {
        let record = source
            .creature_profiles()
            .records
            .iter()
            .find(|record| record.profile.target.key == policy.definition_key)
            .ok_or("source reward-boss profile")?;
        let ProjectV2AuthoringProfileData::Creature(profile) = &record.profile.data else {
            return Err("source reward-boss Creature profile".into());
        };
        let corpse = profile
            .details
            .as_ref()
            .and_then(|details| details.corpse_item.as_ref());
        let materializable_corpse = corpse.filter(|corpse| {
            loot.items.iter().any(|facts| {
                facts.item.family == "Item"
                    && facts.item.key == corpse.key
                    && facts.item.revision == corpse.revision
                    && facts.materializable
                    // Reward-boss Ground bodies use their exact positive source capacity,
                    // independently of the portable container-entry admission limit.
                    && facts.container_capacity.is_some_and(|capacity| capacity > 0)
            })
        });
        if let Some(corpse) = materializable_corpse.filter(|_| {
            matches!(
                policy.definition_key.as_str(),
                "oteryn:creature.bazir"
                    | "oteryn:creature.infernatil"
                    | "oteryn:creature.the_old_widow"
            )
        }) {
            // Retained native-goal, Hireling and BossYield owners qualify these Items.
            // Every remaining unavailable yield still refuses the complete table.
            for key in [
                "oteryn:item.tibia.i3061",
                "oteryn:item.tibia.i3072",
                "oteryn:item.tibia.i2903",
                "oteryn:item.tibia.i3002",
                "oteryn:item.tibia.i3069",
                "oteryn:item.tibia.i3070",
                "oteryn:item.tibia.i7416",
            ] {
                let facts = loot
                    .items
                    .iter()
                    .find(|facts| facts.item.key == key)
                    .ok_or("qualified source loot identity")?;
                assert!(facts.materializable);
                assert_eq!(facts.item.family, "Item");
                assert_eq!(facts.item.revision, "definition-r1");
            }
            let (body, refused): (&str, &[&str]) = match policy.definition_key.as_str() {
                "oteryn:creature.the_old_widow" => {
                    ("oteryn:item.tibia.i5977", &["oteryn:item.tibia.i12320"])
                }
                _ => (
                    "oteryn:item.tibia.i4097",
                    &["oteryn:item.tibia.i3116", "oteryn:item.tibia.i3309"],
                ),
            };
            assert_eq!(corpse.key, body);
            let binding = loot
                .creature_loot
                .iter()
                .find(|binding| binding.creature.key == policy.definition_key)
                .ok_or("source refused boss loot binding")?;
            let reference = binding.loot.as_ref().ok_or("source refused boss loot")?;
            let table = loot
                .tables
                .iter()
                .find(|table| table.identity == *reference)
                .ok_or("source refused boss table")?;
            assert_eq!(
                source_loot_refusals::items(&table.entries, &loot.items),
                refused
            );
            assert_eq!(
                source.reward_table().row(&policy.definition_key).err(),
                Some(NoSettlementReason::LootItemInadmissible)
            );
            inadmissible_loot += 1;
            refreshed_body_loot_refusals.push(policy.definition_key.as_str());
            continue;
        }
        if materializable_corpse.is_some()
            && matches!(
                policy.definition_key.as_str(),
                "oteryn:creature.an_observer_eye"
                    | "oteryn:creature.apprentice_sheng"
                    | "oteryn:creature.armenius_creature"
                    | "oteryn:creature.bakragore"
                    | "oteryn:creature.dharalion"
                    | "oteryn:creature.grand_mother_foulscale"
                    | "oteryn:creature.orshabaal"
                    | "oteryn:creature.the_weakened_count"
            )
        {
            shared_body.push(policy.definition_key.as_str());
        }
        if materializable_corpse.is_some()
            && matches!(
                policy.definition_key.as_str(),
                "oteryn:creature.an_observer_eye"
                    | "oteryn:creature.apprentice_sheng"
                    | "oteryn:creature.bazir"
                    | "oteryn:creature.dharalion"
                    | "oteryn:creature.earth_overlord"
                    | "oteryn:creature.energy_overlord"
                    | "oteryn:creature.fire_overlord"
                    | "oteryn:creature.gelidrazah_the_frozen"
                    | "oteryn:creature.gravelord_oshuran"
                    | "oteryn:creature.infernatil"
                    | "oteryn:creature.irgix_the_flimsy"
                    | "oteryn:creature.orshabaal"
                    | "oteryn:creature.soul_of_dragonking_zyrtarch"
                    | "oteryn:creature.spirit_overlord"
                    | "oteryn:creature.the_enraged_thorn_knight"
                    | "oteryn:creature.the_souldespoiler"
                    | "oteryn:creature.the_weakened_count"
                    | "oteryn:creature.unaz_the_mean"
                    | "oteryn:creature.vok_the_freakish"
            )
        {
            inadmissible_loot += 1;
            assert_eq!(
                source.reward_table().row(&policy.definition_key).err(),
                Some(NoSettlementReason::LootItemInadmissible)
            );
            continue;
        }
        let expected = if let Some(corpse) = materializable_corpse {
            admitted.push(policy.definition_key.as_str());
            let row = source
                .reward_table()
                .row(&policy.definition_key)
                .map_err(|reason| format!("{}: {reason:?}", policy.definition_key))?;
            if policy.definition_key == "oteryn:creature.demodras" {
                assert_eq!(corpse.key, "oteryn:item.tibia.i5984");
                let tooth = loot
                    .items
                    .iter()
                    .find(|facts| facts.item.key == "oteryn:item.tibia.i5919")
                    .ok_or("source-qualified Demodras yield")?;
                assert_eq!(tooth.item.family, "Item");
                assert_eq!(tooth.item.revision, "definition-r1");
                assert!(tooth.materializable);
                assert!(tooth.stackable);
            }
            if matches!(
                policy.definition_key.as_str(),
                "oteryn:creature.armenius_creature" | "oteryn:creature.grand_mother_foulscale"
            ) {
                assert!(!row.loot_table.entries.is_empty());
            }
            assert_eq!(
                row.corpse_item,
                Some(LootDefinitionRef::new(
                    "Item",
                    &corpse.key,
                    &corpse.revision
                ))
            );
            assert_eq!(
                row.xp_amount,
                oteryn_simulation_determinism::ExactI64::new(
                    i64::try_from(profile.experience.ok_or("source boss XP")?)
                        .map_err(|error| error.to_string())?
                )
            );
            let binding = loot
                .creature_loot
                .iter()
                .find(|binding| binding.creature.key == policy.definition_key)
                .ok_or("source reward-boss loot binding")?;
            if let Some(reference) = &binding.loot {
                assert_eq!(
                    row.loot_table_ref,
                    Some(LootDefinitionRef::new(
                        &reference.family,
                        &reference.key,
                        &reference.revision
                    ))
                );
                let table = loot
                    .tables
                    .iter()
                    .find(|table| table.identity == *reference)
                    .ok_or("original complete source loot table")?;
                assert_eq!(row.loot_table.entries.len(), table.entries.len());
                if newly_admitted.contains(&policy.definition_key.as_str()) {
                    assert!(!table.entries.is_empty());
                }
                assert_eq!(table.algorithm, "IndependentBernoulliPpm");
                source_loot_assertions::entries(
                    &row.loot_table.entries,
                    &table.entries,
                    &loot.items,
                );
                if policy.definition_key == "oteryn:creature.sugar_daddy" {
                    // W02 admits the unchanged source table and all its 22 Items.
                    assert_eq!(table.entries.len(), 22);
                } else if policy.definition_key == "oteryn:creature.demodras" {
                    assert_eq!(table.entries.len(), 17);
                } else if policy.definition_key == "oteryn:creature.the_primal_menace" {
                    // Crystal the_primal_menace.lua:118 explicitly authors {}.
                    assert_eq!(reference.family, "Loot");
                    assert_eq!(reference.key, "oteryn:loot.creature.the_primal_menace");
                    assert_eq!(reference.revision, "definition-r1");
                    assert!(table.entries.is_empty());
                    assert!(row.loot_table.entries.is_empty());
                }
                for (actual, original) in row.loot_table.entries.iter().zip(&table.entries) {
                    assert_eq!(
                        actual.item,
                        LootDefinitionRef::new(
                            &original.item.family,
                            &original.item.key,
                            &original.item.revision
                        )
                    );
                    assert_eq!(
                        (actual.min_count, actual.max_count, actual.probability_ppm),
                        (
                            original.min_count,
                            original.max_count,
                            original.probability_ppm
                        )
                    );
                    if policy.definition_key == "oteryn:creature.sugar_daddy" {
                        assert!(actual.details.children.is_empty());
                        assert!(original.children.is_empty());
                    }
                }
            } else {
                assert!(!newly_admitted.contains(&policy.definition_key.as_str()));
                assert_ne!(
                    policy.definition_key, "oteryn:creature.sugar_daddy",
                    "Sugar keeps its actual source Loot binding"
                );
                assert_ne!(
                    policy.definition_key, "oteryn:creature.the_primal_menace",
                    "Primal keeps its explicit empty Loot binding"
                );
                assert_eq!(row.loot_table_ref, row.corpse_item);
                assert!(row.loot_table.entries.is_empty());
            }
            assert_eq!(
                row.loot_table.algorithm,
                LootSelectionAlgorithm::IndependentBernoulliPpm
            );
            None
        } else if corpse.is_none() {
            missing += 1;
            Some(NoSettlementReason::CorpseItemMissing)
        } else {
            inadmissible += 1;
            Some(NoSettlementReason::CorpseItemInadmissible)
        };
        assert_eq!(
            source.reward_table().row(&policy.definition_key).err(),
            expected,
            "{}",
            policy.definition_key
        );
    }
    admitted.sort_unstable();
    assert_eq!(
        missing + inadmissible + inadmissible_loot + admitted.len(),
        bosses.len()
    );
    assert_eq!(
        (missing, inadmissible, inadmissible_loot, admitted.len()),
        (52, 283, 20, 33)
    );
    assert_eq!(
        admitted,
        [
            "oteryn:creature.ahau",
            "oteryn:creature.anomaly",
            "oteryn:creature.armenius_creature",
            "oteryn:creature.bakragore",
            "oteryn:creature.chagorz",
            "oteryn:creature.chayenne",
            "oteryn:creature.demodras",
            "oteryn:creature.dragonking_zyrtarch",
            "oteryn:creature.dreadful_disruptor",
            "oteryn:creature.eradicator",
            "oteryn:creature.eradicator2",
            "oteryn:creature.goshnar_s_cruelty",
            "oteryn:creature.goshnar_s_greed",
            "oteryn:creature.goshnar_s_hatred",
            "oteryn:creature.goshnar_s_malice",
            "oteryn:creature.goshnar_s_megalomania_blue",
            "oteryn:creature.goshnar_s_megalomania_green",
            "oteryn:creature.goshnar_s_megalomania_purple",
            "oteryn:creature.goshnar_s_spite",
            "oteryn:creature.grand_mother_foulscale",
            "oteryn:creature.ice_overlord",
            "oteryn:creature.ichgahal",
            "oteryn:creature.mitmah_vanguard",
            "oteryn:creature.murcion",
            "oteryn:creature.outburst",
            "oteryn:creature.rupture",
            "oteryn:creature.sugar_daddy",
            "oteryn:creature.tazhadur",
            "oteryn:creature.the_first_dragon",
            "oteryn:creature.the_percht_queen",
            "oteryn:creature.the_primal_menace",
            "oteryn:creature.the_scion_of_havoc",
            "oteryn:creature.vemiath",
        ]
    );
    shared_body.sort_unstable();
    refreshed_body_loot_refusals.sort_unstable();
    assert_eq!(
        refreshed_body_loot_refusals,
        [
            "oteryn:creature.bazir",
            "oteryn:creature.infernatil",
            "oteryn:creature.the_old_widow"
        ]
    );
    assert_eq!(
        shared_body,
        [
            "oteryn:creature.an_observer_eye",
            "oteryn:creature.apprentice_sheng",
            "oteryn:creature.armenius_creature",
            "oteryn:creature.bakragore",
            "oteryn:creature.dharalion",
            "oteryn:creature.grand_mother_foulscale",
            "oteryn:creature.orshabaal",
            "oteryn:creature.the_weakened_count",
        ]
    );
    assert_eq!(bosses.len(), 388);
    let before = source.reward_table().row("oteryn:creature.rat").unwrap();
    let before_policy = source
        .policies()
        .policies()
        .iter()
        .find(|policy| policy.definition_key == "oteryn:creature.rat")
        .unwrap();
    assert_eq!(before_policy.reward_boss, Some(false));
    let (fixture, _) = activated_reward_boss_fixture(world, None);
    let fixture = fixture.active().unwrap().native_gameplay().unwrap();
    let after = fixture.reward_table().row("oteryn:creature.rat").unwrap();
    let after_policy = fixture
        .policies()
        .policies()
        .iter()
        .find(|policy| policy.definition_key == "oteryn:creature.rat")
        .unwrap();
    assert_eq!(after_policy.reward_boss, Some(true));
    assert_eq!(after_policy.maximum_health, before_policy.maximum_health);
    assert_eq!(
        after, before,
        "source XP, corpse, loot and metadata are unchanged"
    );
    assert_ne!(
        fixture.source_digest(),
        source.source_digest(),
        "the authored test classification is included in its real generated pin"
    );
    Ok(())
}

pub fn activated_with_custom_loot_tables(
    world: oteryn_protocol_oteryn::WorldId,
    source_world: Option<&[u8]>,
    loot_tables: Option<&[u8]>,
) -> (
    ContentActivationController,
    crate::content::NativeEntryContentPin,
) {
    let mut supplied = manifest_input();
    assert!(
        supplied.loot_tables.is_some(),
        "the manifest pins loot_tables"
    );
    if let Some(bytes) = loot_tables {
        supplied.loot_tables = Some(pinned(bytes));
    }
    supplied.source_world = source_world.map(pinned);
    activate(world, &supplied)
}

pub fn activated_loop_monster(
    world: oteryn_protocol_oteryn::WorldId,
    summons: bool,
) -> (
    ContentActivationController,
    crate::content::NativeEntryContentPin,
) {
    activate(world, &loop_monster_input(summons))
}
fn loop_monster_input(summons: bool) -> NativeGameplayInput {
    let mut supplied = manifest_input();
    supplied.source_world = Some(pinned(include_bytes!(
        "../../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
    )));
    if summons {
        let mut creatures: CreatureProfilesDocument =
            serde_json::from_slice(&supplied.creature_profiles.bytes).unwrap();
        // Pin the actual ranged Item spell; source catalogue order also contains
        // untargeted range-zero removal callbacks such as Aftershock's AnomalyBreak.
        let source = creatures
            .records
            .iter()
            .find(|r| r.profile.target.key == "oteryn:creature.angry_demon")
            .expect("source Item spell closure");
        let ability = source
            .source_item_abilities
            .iter()
            .find(|ability| ability.target.key == "oteryn:ability.creature.angry_demon.attack-4")
            .expect("source ranged Item spell")
            .clone();
        // Keep the reused inline Item body and its exact reference in the fixture closure.
        let ability_reference = source
            .attack_definitions
            .as_ref()
            .expect("source Item attack closure")
            .records
            .iter()
            .find(|reference| {
                matches!(reference,
                    crate::content::ProjectReferenceRecord::Ability { identity, effects, .. }
                        if identity.family == "Ability"
                            && identity.key == ability.target.key
                            && identity.revision == ability.target.revision
                            && effects.is_empty()
                )
            })
            .expect("source inline Item Ability reference")
            .clone();
        let Some(ProjectV2AuthoringProfile {
            data: ProjectV2AuthoringProfileData::Behavior(source_behavior),
            ..
        }) = &source.behavior
        else {
            panic!("source Behavior");
        };
        let mut attack = source_behavior
            .attacks
            .iter()
            .find(|entry| entry.ability == ability.target)
            .expect("source attack")
            .clone();
        attack.interval_ms = 1000;
        attack.chance_ppm = 1_000_000;
        // The donor body may be uncapped (0); this synthetic schedule qualifies
        // the explicit seven-tile creature override, independent of record order.
        attack.range_tiles = Some(7);
        let rat = creatures
            .records
            .iter_mut()
            .find(|r| r.profile.target.key == "oteryn:creature.rat")
            .expect("admitted rat");
        let child = rat.profile.target.clone();
        if let ProjectV2AuthoringProfileData::Creature(profile) = &mut rat.profile.data {
            profile.abilities = vec![ability.target.clone()];
        }
        let definitions = rat.attack_definitions.as_mut().expect("rat attack closure");
        assert!(
            !definitions
                .profiles
                .iter()
                .any(|profile| profile.target == ability.target)
        );
        definitions.records.push(ability_reference);
        definitions.profiles.push(ability.clone());
        rat.source_item_abilities = vec![ability];
        rat.monster_melee = None;
        let Some(ProjectV2AuthoringProfile {
            data: ProjectV2AuthoringProfileData::Behavior(behavior),
            ..
        }) = &mut rat.behavior
        else {
            panic!("rat Behavior closure");
        };
        behavior.attacks = vec![attack];
        behavior.defenses.clear();
        behavior.summons = Some(serde_json::from_value(serde_json::json!({"max_summons":2,"entries":[{"creature":child,"interval_ms":1000,"chance_ppm":1000000,"count":2}]})).unwrap());
        refresh_fixture_attack_roots(rat);
        supplied.creature_profiles = pinned(&serde_json::to_vec(&creatures).unwrap());
    }
    supplied
}

// Explicit fixture overlays must retain the exact admitted Creature/Behavior roots.
// This is called only by these synthetic scenarios, never by activation or validation.
fn refresh_fixture_attack_roots(record: &mut CreatureProfileRecord) {
    let roots = [
        record.profile.clone(),
        record.behavior.as_ref().expect("fixture Behavior").clone(),
    ];
    let definitions = record
        .attack_definitions
        .as_mut()
        .expect("fixture attack closure");
    for root in roots {
        let bound = definitions
            .profiles
            .iter_mut()
            .find(|profile| profile.target == root.target)
            .expect("exact fixture root membership");
        *bound = root;
    }
}

pub fn loop_monster_inner_digest(pin: &crate::content::NativeEntryContentPin) -> [u8; 32] {
    let compiled = pin.qualified_room().compiled();
    let decoded = crate::content::native_source_world_carrier::decode(
        &compiled.server_artifact,
        &compiled.client_artifact,
    )
    .expect("source outer envelope");
    sha256(decoded.baseline_server)
}

pub fn activated_loop_monster_missing_child(
    world: oteryn_protocol_oteryn::WorldId,
) -> (
    ContentActivationController,
    crate::content::NativeEntryContentPin,
) {
    let mut supplied = loop_monster_input(true);
    let mut creatures: CreatureProfilesDocument =
        serde_json::from_slice(&supplied.creature_profiles.bytes).unwrap();
    let child = creatures
        .records
        .iter_mut()
        .find(|r| r.profile.target.key == "oteryn:creature.snake")
        .expect("admitted child");
    child.behavior = None;
    child.monster_melee = None;
    child.source_item_abilities.clear();
    // This scenario intentionally has no child Behavior or executable attack closure;
    // its failure belongs to runtime summon registration, after valid Content activation.
    child.attack_definitions = None;
    let child = child.profile.target.clone();
    let rat = creatures
        .records
        .iter_mut()
        .find(|r| r.profile.target.key == "oteryn:creature.rat")
        .expect("admitted parent");
    let Some(ProjectV2AuthoringProfile {
        data: ProjectV2AuthoringProfileData::Behavior(behavior),
        ..
    }) = &mut rat.behavior
    else {
        panic!("parent Behavior");
    };
    behavior.summons.as_mut().expect("summons").entries[0].creature = child;
    refresh_fixture_attack_roots(rat);
    supplied.creature_profiles = pinned(&serde_json::to_vec(&creatures).unwrap());
    activate(world, &supplied)
}

#[cfg(test)]
#[test]
fn srx3_actual_grand_mother_reward_boss_has_source_corpse_and_complete_material_loot()
-> Result<(), String> {
    // Crystal201189f data-global/monster/raids/grand_mother_foulscale.lua:
    // corpse5973(:24), actual rewardBoss=true(:46), guaranteed5920/5877(:93/95).
    let world = test_source(1)
        .map_err(|error| format!("source world: {error:?}"))?
        .world_id;
    let (controller, _) = activated_with_loot_tables(world, None);
    let native = controller
        .active()
        .and_then(|generation| generation.native_gameplay())
        .ok_or("active native gameplay")?;
    let policy = native
        .policies()
        .policies()
        .iter()
        .find(|policy| policy.definition_key == "oteryn:creature.grand_mother_foulscale")
        .ok_or("actual source Grand Mother Foulscale profile")?;
    assert_eq!(policy.reward_boss, Some(true));
    let row = native
        .reward_table()
        .row(&policy.definition_key)
        .map_err(|error| format!("actual source boss reward qualification: {error:?}"))?;
    assert_eq!(
        row.corpse_item
            .as_ref()
            .ok_or("physical source corpse")?
            .production_key,
        "oteryn:item.tibia.i5973"
    );
    for id in [5973, 4025, 4026, 4027] {
        let key = format!("oteryn:item.tibia.i{id}");
        let profile = native
            .item_policy(&key, "definition-r1")
            .unwrap_or_else(|| panic!("actual Grand Mother corpse descendant {key}"));
        assert_eq!(
            profile.record().admission.legal_destinations,
            [super::super::NativeItemDestination::Ground]
        );
    }
    assert_eq!(row.loot_table.entries.len(), 19);
    for expected in ["oteryn:item.tibia.i5920", "oteryn:item.tibia.i5877"] {
        assert!(
            row.loot_table
                .entries
                .iter()
                .any(|entry| entry.item.production_key == expected
                    && entry.probability_ppm == Some(1_000_000)),
            "source guaranteed material reward {expected}"
        );
    }
    Ok(())
}

#[cfg(test)]
#[test]
fn srx3_actual_w11_bosses_have_source_ground_corpses_and_complete_reward_rows() -> Result<(), String>
{
    let world = test_source(1)
        .map_err(|error| format!("source world: {error:?}"))?
        .world_id;
    let (controller, _) = activated_with_loot_tables(world, None);
    let native = controller
        .active()
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native gameplay")?;
    let roots = [
        6028, 33859, 33863, 33867, 33871, 33875, 33889, 44012, 44015, 44018, 44021, 44024,
    ];
    for id in roots {
        let key = format!("oteryn:item.tibia.i{id}");
        let policy = native
            .item_policy(&key, "definition-r1")
            .ok_or_else(|| format!("missing actual W11 native corpse profile {key}"))?;
        assert!(policy.record().admission.materializable);
        assert_eq!(
            policy.record().admission.legal_destinations,
            vec![crate::content::native_gameplay::NativeItemDestination::Ground]
        );
    }
    for key in [
        "goshnar_s_greed",
        "goshnar_s_malice",
        "goshnar_s_spite",
        "goshnar_s_hatred",
        "goshnar_s_cruelty",
        "goshnar_s_megalomania_blue",
        "goshnar_s_megalomania_green",
        "goshnar_s_megalomania_purple",
        "ichgahal",
        "murcion",
        "vemiath",
        "chagorz",
        "bakragore",
    ] {
        let key = format!("oteryn:creature.{key}");
        let policy = native
            .policies()
            .policies()
            .iter()
            .find(|policy| policy.definition_key == key)
            .ok_or("actual W11 Creature")?;
        assert_eq!(policy.reward_boss, Some(true));
        native
            .reward_table()
            .row(&key)
            .map_err(|reason| format!("actual W11 source reward closure {key}: {reason:?}"))?;
    }
    Ok(())
}

#[cfg(test)]
#[test]
fn srx3_actual_rotten_reward_yields_have_source_inventory_profiles() -> Result<(), String> {
    use crate::content::ReferenceItemField as F;
    use crate::content::native_gameplay::NativeItemDestination;
    let world = test_source(1)
        .map_err(|error| format!("source world: {error:?}"))?
        .world_id;
    let (controller, _) = activated_with_loot_tables(world, None);
    let native = controller
        .active()
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native gameplay")?;
    for id in [
        33778, 39037, 43501, 43502, 43503, 43504, 43946, 43947, 43961, 43962, 43966, 43967, 43968,
        44048,
    ] {
        let key = format!("oteryn:item.tibia.i{id}");
        let record = native
            .item_policy(&key, "definition-r1")
            .ok_or_else(|| format!("missing actual Rotten loot yield native profile {key}"))?
            .record();
        assert!(record.admission.materializable);
        assert_eq!(
            record.admission.legal_destinations,
            vec![
                NativeItemDestination::CharacterInventory,
                NativeItemDestination::Ground
            ]
        );
        assert!(matches!(&record.semantics.physical, F::Known(physical)
            if physical.movable==F::Known(true) && physical.pickupable==F::Known(true)));
    }
    Ok(())
}
