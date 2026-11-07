use super::model::*;

pub fn synthetic_vsl_fixture(limits: &EvidenceLimits) -> Result<FixtureSource, ContentError> {
    let key = |value: &str| ContentKey::new(value, limits);
    let package = |value: &str| PackageKey::new(value, limits);
    let world = |value: &str| WorldId::new(value, limits);
    let revision = |value: &str| RevisionToken::new(value, limits);

    let terrain_key = key("oteryn:vsl.terrain.walkable")?;
    let region_key = key("oteryn:vsl.region.slice")?;
    let area_key = key("oteryn:vsl.area.slice")?;
    let cell_start = key("oteryn:vsl.cell.start")?;
    let cell_wall = key("oteryn:vsl.cell.wall")?;
    let cell_target = key("oteryn:vsl.cell.target")?;
    let behavior_key = key("oteryn:vsl.ai.fixture-policy")?;
    let creature_presentation = key("oteryn:vsl.appearance.creature")?;
    let item_presentation = key("oteryn:vsl.appearance.item")?;
    let ability_presentation = key("oteryn:vsl.appearance.ability")?;
    let creature_key = key("oteryn:vsl.creature.target")?;
    let formula_key = key("oteryn:vsl.formula.fixture-damage")?;
    let effect_key = key("oteryn:vsl.effect.fixture-damage")?;
    let item_key = key("oteryn:vsl.item.fixture-drop")?;
    let rng_purpose = key("oteryn:vsl.rng.loot.fixture-drop")?;

    Ok(FixtureSource {
        schema_version: FIXTURE_SCHEMA_VERSION,
        package_key: package("oteryn:vsl.package")?,
        package_revision: revision("vsl-package-r1")?,
        world_id: world("world-fixture-vsl")?,
        revisions: RevisionSet {
            content: revision("vsl-content-r1")?,
            map: revision("vsl-map-r1")?,
            ruleset: revision("vsl-ruleset-fixture-r1")?,
            world_policy: revision("vsl-world-policy-r1")?,
            compiler: revision("vsl-compiler-evidence-r1")?,
            canonicalization: revision("vsl-canonicalization-r1")?,
            content_lock: revision("vsl-content-lock-r1")?,
            provenance: revision("vsl-provenance-synthetic-r1")?,
            sim_profile: revision("sim-v1")?,
            fixture_profile: revision("vsl-movement-combat-fixture-r1")?,
        },
        regions: vec![RegionDefinition {
            key: region_key.clone(),
        }],
        areas: vec![AreaDefinition {
            key: area_key.clone(),
        }],
        terrains: vec![TerrainDefinition {
            key: terrain_key.clone(),
        }],
        cells: vec![
            CellDefinition {
                key: cell_start.clone(),
                region_key: region_key.clone(),
                area_key: area_key.clone(),
                terrain_key: terrain_key.clone(),
                x: 10,
                y: 10,
                z: 7,
                collision: CollisionClass::Walkable,
            },
            CellDefinition {
                key: cell_wall,
                region_key: region_key.clone(),
                area_key: area_key.clone(),
                terrain_key: terrain_key.clone(),
                x: 11,
                y: 10,
                z: 7,
                collision: CollisionClass::Blocked,
            },
            CellDefinition {
                key: cell_target.clone(),
                region_key,
                area_key,
                terrain_key,
                x: 12,
                y: 10,
                z: 7,
                collision: CollisionClass::Walkable,
            },
        ],
        relocations: vec![RelocationDefinition {
            key: key("oteryn:vsl.relocation.local")?,
            from_cell: cell_start,
            to_cell: cell_target.clone(),
        }],
        behaviors: vec![BehaviorDefinition {
            key: behavior_key.clone(),
            policy_revision: revision("vsl-ai-policy-r1")?,
        }],
        presentations: vec![
            PresentationDefinition {
                key: creature_presentation.clone(),
                synthetic_asset_token: "synthetic://vsl/creature-square".to_owned(),
            },
            PresentationDefinition {
                key: item_presentation.clone(),
                synthetic_asset_token: "synthetic://vsl/item-square".to_owned(),
            },
            PresentationDefinition {
                key: ability_presentation.clone(),
                synthetic_asset_token: "synthetic://vsl/ability-flash".to_owned(),
            },
        ],
        creatures: vec![CreatureDefinition {
            key: creature_key.clone(),
            behavior_key: behavior_key.clone(),
            presentation_key: creature_presentation,
            fixture_max_hp: 7,
        }],
        spawns: vec![SpawnDefinition {
            key: key("oteryn:vsl.spawn.target")?,
            creature_key,
            behavior_key,
            cell_key: cell_target,
            fixture_population_limit: 1,
            recovery: SpawnRecoveryClass::CheckpointedRuntimeContinuity,
            multiplicity: Some(MultiplicityClass::ChannelLocalSharedEligibility),
            eligibility_scope: Some(EligibilityScope::CharacterWorld),
        }],
        formula_profiles: vec![FormulaProfileDefinition {
            key: formula_key.clone(),
            fixture_only: true,
        }],
        effects: vec![EffectDefinition {
            key: effect_key.clone(),
            family: EffectFamily::Damage,
            formula_profile_key: formula_key.clone(),
        }],
        abilities: vec![AbilityDefinition {
            key: key("oteryn:vsl.ability.fixture-strike")?,
            effect_key,
            presentation_key: ability_presentation,
        }],
        items: vec![ItemDefinition {
            key: item_key.clone(),
            presentation_key: item_presentation,
            materializable: true,
        }],
        loot_tables: vec![LootTableDefinition {
            key: key("oteryn:vsl.loot.table.fixture")?,
            entries: vec![LootEntryDefinition {
                key: key("oteryn:vsl.loot.entry.fixture-drop")?,
                item_key,
                rng_purpose_key: rng_purpose.clone(),
                fixture_weight: 1,
            }],
        }],
        xp_definitions: vec![XpDefinition {
            key: key("oteryn:vsl.xp.fixture")?,
            formula_profile_key: formula_key,
            fixture_amount: 3,
        }],
        rng: RngFixtureContext {
            profile_revision: revision("vsl-rng-fixture-r1")?,
            synthetic_root_label: "SYNTHETIC_TEST_ROOT_NOT_SECRET".to_owned(),
            purpose_keys: vec![rng_purpose],
        },
    })
}
