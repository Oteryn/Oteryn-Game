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
    use oteryn_gameplay_domain::domain::loot_data::{
        GAMEITEM01_CORPSE_CONTAINER_ENTRIES_MAX, LootDefinitionRef, LootSelectionAlgorithm,
    };
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
    // Source-qualified quest corpses also expose these ten named boss bodies.
    // Their independent private-loot closure remains mandatory: five new aliases
    // (Monk, Demon, Elf and Rotten Blood bodies) do not acquire private loot.
    // Keep each exact admission reason:
    // a missing corpse is different from an authored but inadmissible corpse.
    // Change these source-qualification expectations only with admitted content.
    let newly_admitted = ["oteryn:creature.the_percht_queen"];
    let mut missing = 0;
    let mut inadmissible = 0;
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
                    && facts.container_capacity.is_some_and(|capacity| {
                        (1..=GAMEITEM01_CORPSE_CONTAINER_ENTRIES_MAX as u32).contains(&capacity)
                    })
            })
        });
        let expected = if let Some(corpse) = materializable_corpse {
            if matches!(
                policy.definition_key.as_str(),
                "oteryn:creature.bazir" | "oteryn:creature.infernatil"
            ) {
                // Loot regeneration refreshes their already qualified source body;
                // none of these eight unsupported loot identities becomes a grant.
                assert_eq!(corpse.key, "oteryn:item.tibia.i4097");
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
                    [
                        "oteryn:item.tibia.i3002",
                        "oteryn:item.tibia.i3061",
                        "oteryn:item.tibia.i3072",
                        "oteryn:item.tibia.i3070",
                        "oteryn:item.tibia.i3069",
                        "oteryn:item.tibia.i2903",
                        "oteryn:item.tibia.i3116",
                        "oteryn:item.tibia.i3309",
                    ]
                );
                assert_eq!(
                    source.reward_table().row(&policy.definition_key).err(),
                    Some(NoSettlementReason::LootItemInadmissible)
                );
                inadmissible += 1;
                refreshed_body_loot_refusals.push(policy.definition_key.as_str());
                continue;
            }
            if matches!(
                policy.definition_key.as_str(),
                "oteryn:creature.an_observer_eye"
                    | "oteryn:creature.apprentice_sheng"
                    | "oteryn:creature.armenius_creature"
                    | "oteryn:creature.bakragore"
                    | "oteryn:creature.bazir"
                    | "oteryn:creature.dharalion"
                    | "oteryn:creature.grand_mother_foulscale"
                    | "oteryn:creature.infernatil"
                    | "oteryn:creature.orshabaal"
                    | "oteryn:creature.the_weakened_count"
            ) {
                shared_body.push(policy.definition_key.as_str());
                match policy.definition_key.as_str() {
                    "oteryn:creature.armenius_creature"
                    | "oteryn:creature.grand_mother_foulscale" => {
                        let row = source
                            .reward_table()
                            .row(&policy.definition_key)
                            .map_err(|reason| format!("{}: {reason:?}", policy.definition_key))?;
                        assert_eq!(
                            row.corpse_item,
                            LootDefinitionRef::new("Item", &corpse.key, &corpse.revision)
                        );
                        assert!(!row.loot_table.entries.is_empty());
                    }
                    _ => assert_eq!(
                        source.reward_table().row(&policy.definition_key).err(),
                        Some(NoSettlementReason::LootItemInadmissible),
                        "corpse admission cannot bypass independent source loot admission"
                    ),
                }
                continue;
            }
            admitted.push(policy.definition_key.as_str());
            let row = source
                .reward_table()
                .row(&policy.definition_key)
                .map_err(|reason| format!("{}: {reason:?}", policy.definition_key))?;
            assert_eq!(
                row.corpse_item,
                LootDefinitionRef::new("Item", &corpse.key, &corpse.revision)
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
            if newly_admitted.contains(&policy.definition_key.as_str()) {
                let reference = binding.loot.as_ref().ok_or("source Queen loot binding")?;
                let table = loot
                    .tables
                    .iter()
                    .find(|t| t.identity == *reference)
                    .ok_or("source Queen loot table")?;
                assert_eq!(
                    row.loot_table_ref,
                    LootDefinitionRef::new(&reference.family, &reference.key, &reference.revision)
                );
                assert_eq!(table.algorithm, "IndependentBernoulliPpm");
                assert!(!table.entries.is_empty());
                source_loot_assertions::entries(
                    &row.loot_table.entries,
                    &table.entries,
                    &loot.items,
                );
            } else if policy.definition_key == "oteryn:creature.sugar_daddy" {
                // W02's exact source capture admits this body and every Item
                // of the unchanged canonical loot table, never a fixture table.
                let bound = binding.loot.as_ref().ok_or("Sugar source loot binding")?;
                assert_eq!(
                    row.loot_table_ref,
                    LootDefinitionRef::new(&bound.family, &bound.key, &bound.revision)
                );
                let original = loot
                    .tables
                    .iter()
                    .find(|table| table.identity == *bound)
                    .ok_or("Sugar original source loot table")?;
                assert_eq!(row.loot_table.entries.len(), 22);
                assert_eq!(row.loot_table.entries.len(), original.entries.len());
                for (actual, expected) in row.loot_table.entries.iter().zip(&original.entries) {
                    assert_eq!(
                        actual.item,
                        LootDefinitionRef::new(
                            &expected.item.family,
                            &expected.item.key,
                            &expected.item.revision
                        )
                    );
                    assert_eq!(
                        (actual.min_count, actual.max_count, actual.probability_ppm),
                        (
                            expected.min_count,
                            expected.max_count,
                            expected.probability_ppm
                        )
                    );
                    assert!(actual.details.children.is_empty());
                    assert!(expected.children.is_empty());
                }
            } else if policy.definition_key == "oteryn:creature.the_primal_menace" {
                // Crystal the_primal_menace.lua:118 is a source-proven empty
                // Loot table, never the legacy no-binding/corpse-ref fallback.
                let bound = binding
                    .loot
                    .as_ref()
                    .ok_or("Primal explicit empty source loot binding")?;
                assert_eq!(bound.family, "Loot");
                assert_eq!(bound.key, "oteryn:loot.creature.the_primal_menace");
                assert_eq!(bound.revision, "definition-r1");
                assert_eq!(
                    row.loot_table_ref,
                    LootDefinitionRef::new(&bound.family, &bound.key, &bound.revision)
                );
                let original = loot
                    .tables
                    .iter()
                    .find(|table| table.identity == *bound)
                    .ok_or("Primal original explicit empty source loot table")?;
                assert!(original.entries.is_empty());
                assert!(row.loot_table.entries.is_empty());
            } else {
                assert!(binding.loot.is_none(), "source boss has no authored loot");
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
    assert_eq!(
        (missing, inadmissible, admitted.len()),
        (52, 328 - newly_admitted.len(), 2 + newly_admitted.len())
    );
    admitted.sort_unstable();
    assert_eq!(
        admitted,
        [
            "oteryn:creature.sugar_daddy",
            "oteryn:creature.the_percht_queen",
            "oteryn:creature.the_primal_menace"
        ]
    );
    shared_body.sort_unstable();
    refreshed_body_loot_refusals.sort_unstable();
    assert_eq!(
        refreshed_body_loot_refusals,
        ["oteryn:creature.bazir", "oteryn:creature.infernatil"]
    );
    assert_eq!(
        shared_body,
        [
            "oteryn:creature.an_observer_eye",
            "oteryn:creature.apprentice_sheng",
            "oteryn:creature.armenius_creature",
            "oteryn:creature.bakragore",
            "oteryn:creature.bazir",
            "oteryn:creature.dharalion",
            "oteryn:creature.grand_mother_foulscale",
            "oteryn:creature.infernatil",
            "oteryn:creature.orshabaal",
            "oteryn:creature.the_weakened_count",
        ]
    );
    assert_eq!(bosses.len(), 387);
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
        // Reuse one real admitted ranged Spell body rather than inventing executable effects.
        let source = creatures
            .records
            .iter()
            .find(|r| !r.source_item_abilities.is_empty())
            .expect("source Item spell closure");
        let ability = source.source_item_abilities[0].clone();
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
