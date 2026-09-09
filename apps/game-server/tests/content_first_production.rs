use oteryn_game_server::content::*;
use oteryn_game_server::domain::WorldId;

fn world_id() -> Result<WorldId, ContentError> {
    let mut bytes = [0_u8; 16];
    bytes[0] = 1;
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = 1;
    WorldId::from_bytes(bytes)
        .map_err(|_| ContentError::InvalidArtifact("integration WorldId invalid"))
}

fn source(cell_count: usize) -> Result<FirstProductionContentSource, ContentError> {
    let package_key = ProductionKey::new("oteryn:content.first-production")?;
    let package_revision = ProductionAtom::new("first-production package revision", "package-r1")?;
    let package_manifest = PackageManifestBinding::new(
        package_key.clone(),
        package_revision.clone(),
        ProductionAtom::new("semantic schema", "schema-v1")?,
        ProductionAtom::new("licensing metadata", "license:project-owned-v1")?,
        Sha256HexDigest::new("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")?,
    );
    let provenance = package_manifest.package_provenance_digest()?;
    let content_lock = ContentLockBinding {
        revision_digest_token: ProductionAtom::new("content lock token", "lock:content-r1")?,
        entries: vec![ContentLockEntry::exact(
            package_key,
            package_revision,
            provenance,
        )],
    };
    let region = ProductionKey::new("oteryn:prod.region")?;
    let area = ProductionKey::new("oteryn:prod.area")?;
    let terrain = ProductionKey::new("oteryn:prod.terrain")?;
    let mut cells = Vec::with_capacity(cell_count);
    for index in 0..cell_count {
        let x = i32::try_from(index % 32).map_err(|_| ContentError::InvalidSectionBounds)?;
        let y = i32::try_from(index / 32).map_err(|_| ContentError::InvalidSectionBounds)?;
        cells.push(FirstProductionCell {
            key: ProductionKey::new(&format!("oteryn:prod.cell.{index:04}"))?,
            region_key: region.clone(),
            area_key: area.clone(),
            terrain_key: terrain.clone(),
            x,
            y,
            z: 7,
            collision: if index == 1 {
                CollisionClass::Blocked
            } else {
                CollisionClass::Walkable
            },
        });
    }
    if cells.is_empty() {
        return Err(ContentError::InvalidArtifact(
            "test source needs at least one cell",
        ));
    }
    let first_cell = cells[0].key.clone();
    let last_cell = cells[cells.len() - 1].key.clone();
    let behavior = ProductionKey::new("oteryn:prod.behavior")?;
    let creature_presentation = ProductionKey::new("oteryn:prod.presentation.creature")?;
    let item_presentation = ProductionKey::new("oteryn:prod.presentation.item")?;
    let ability_presentation = ProductionKey::new("oteryn:prod.presentation.ability")?;
    let creature = ProductionKey::new("oteryn:prod.creature")?;
    let formula = ProductionKey::new("oteryn:prod.formula")?;
    let effect = ProductionKey::new("oteryn:prod.effect")?;
    let item = ProductionKey::new("oteryn:prod.item")?;
    let rng_purpose = ProductionKey::new("oteryn:prod.rng.loot")?;

    Ok(FirstProductionContentSource {
        package_manifest,
        content_lock,
        world_id: world_id()?,
        revisions: FirstProductionRevisionSet {
            content: ProductionAtom::new("content revision", "content-r1")?,
            map: ProductionAtom::new("map revision", "map-r1")?,
            ruleset: ProductionAtom::new("ruleset revision", "ruleset-r1")?,
            world_policy: ProductionAtom::new("world policy revision", "world-policy-r1")?,
            compiler: ProductionAtom::new("compiler revision", "compiler-prod-r1")?,
            canonicalization: ProductionAtom::new(
                "canonicalization revision",
                "canonicalization-r1",
            )?,
            sim_profile: ProductionAtom::new("sim profile", "sim-v1")?,
            profile_revision: ProductionAtom::new(
                "first-production profile revision",
                FIRST_PRODUCTION_PROFILE_ID,
            )?,
        },
        capability_profile: ProductionAtom::new(
            "capability profile",
            FIRST_PRODUCTION_CAPABILITY_PROFILE,
        )?,
        migration_class: DurableMigrationClass::CompatibleNoMigration,
        regions: vec![FirstProductionRegion { key: region }],
        areas: vec![FirstProductionArea { key: area }],
        terrains: vec![FirstProductionTerrain { key: terrain }],
        cells,
        relocations: vec![FirstProductionRelocation {
            key: ProductionKey::new("oteryn:prod.relocation")?,
            from_cell: first_cell,
            to_cell: last_cell.clone(),
        }],
        behaviors: vec![FirstProductionBehavior {
            key: behavior.clone(),
            policy_revision: ProductionAtom::new("behavior policy", "behavior-policy-r1")?,
        }],
        presentations: vec![
            FirstProductionPresentation {
                key: creature_presentation.clone(),
                metadata_token: ProductionAtom::new(
                    "presentation metadata",
                    "appearance:creature-v1",
                )?,
            },
            FirstProductionPresentation {
                key: item_presentation.clone(),
                metadata_token: ProductionAtom::new("presentation metadata", "appearance:item-v1")?,
            },
            FirstProductionPresentation {
                key: ability_presentation.clone(),
                metadata_token: ProductionAtom::new(
                    "presentation metadata",
                    "appearance:ability-v1",
                )?,
            },
        ],
        creatures: vec![FirstProductionCreature {
            key: creature.clone(),
            behavior_key: behavior.clone(),
            presentation_key: creature_presentation,
            policy_revision: ProductionAtom::new("creature policy", "creature-policy-r1")?,
        }],
        spawns: vec![FirstProductionSpawn {
            key: ProductionKey::new("oteryn:prod.spawn")?,
            creature_key: creature,
            behavior_key: behavior,
            cell_key: last_cell,
            population_limit: 1,
            recovery: SpawnRecoveryClass::CheckpointedRuntimeContinuity,
            multiplicity: MultiplicityClass::ChannelLocalSharedEligibility,
            eligibility_scope: EligibilityScope::CharacterWorld,
        }],
        formula_profiles: vec![FirstProductionFormulaProfile {
            key: formula.clone(),
        }],
        effects: vec![FirstProductionEffect {
            key: effect.clone(),
            family: EffectFamily::Damage,
            formula_profile_key: formula.clone(),
        }],
        abilities: vec![FirstProductionAbility {
            key: ProductionKey::new("oteryn:prod.ability")?,
            effect_key: effect,
            presentation_key: ability_presentation,
        }],
        items: vec![FirstProductionItem {
            key: item.clone(),
            presentation_key: item_presentation,
            materializable: true,
        }],
        loot_tables: vec![FirstProductionLootTable {
            key: ProductionKey::new("oteryn:prod.loot.table")?,
            entries: vec![FirstProductionLootEntry {
                key: ProductionKey::new("oteryn:prod.loot.entry")?,
                item_key: item,
                rng_purpose_key: rng_purpose.clone(),
            }],
        }],
        xp_definitions: vec![FirstProductionXpDefinition {
            key: ProductionKey::new("oteryn:prod.xp")?,
            formula_profile_key: formula,
        }],
        rng: FirstProductionRngContext {
            profile_revision: ProductionAtom::new("rng profile", "rng-metadata-r1")?,
            purpose_keys: vec![rng_purpose],
        },
    })
}

#[test]
fn public_production_compile_and_restart_staging_use_immutable_bytes() -> Result<(), ContentError> {
    let source = source(3)?;
    let compiled =
        compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;
    let staged = StagedGeneration::stage(
        &compiled.server_artifact,
        &compiled.client_artifact,
        compiled.expectation(),
    )?;
    assert_eq!(
        staged.identity().package_provenance_digest(),
        compiled.expectation().package_provenance_digest()
    );
    let reconstructed = FirstProductionExpectation::from_generation_identity(staged.identity());
    let restaged = StagedGeneration::stage(
        &compiled.server_artifact,
        &compiled.client_artifact,
        &reconstructed,
    )?;
    assert_eq!(restaged.identity(), staged.identity());
    assert_eq!(staged.identity().world_id(), source.world_id);

    let mut corrupt = compiled.server_artifact.clone();
    let index = corrupt.len() - 33;
    corrupt[index] ^= 0x5a;
    assert!(matches!(
        StagedGeneration::stage(&corrupt, &compiled.client_artifact, compiled.expectation()),
        Err(ContentError::IntegrityMismatch(_))
    ));
    Ok(())
}

#[test]
fn public_profile_enforces_minimum_and_rejects_nonproduction_target() -> Result<(), ContentError> {
    let below_minimum = source(2)?;
    assert!(matches!(
        compile_first_production(
            &below_minimum,
            FirstProductionCompileTarget::OrdinaryRelease,
        ),
        Err(ContentError::InvalidArtifact(
            "first-production cells must contain at least 3 entries"
        ))
    ));

    let valid = source(3)?;
    assert!(matches!(
        compile_first_production(&valid, FirstProductionCompileTarget::NonProductionEvidence,),
        Err(ContentError::FixtureOnlyReleaseRejected)
    ));
    Ok(())
}

#[test]
fn public_identifier_and_digest_boundaries_are_fail_closed() {
    let key_512 = format!("oteryn:{}", "a".repeat(505));
    let key_513 = format!("oteryn:{}", "a".repeat(506));
    assert_eq!(key_512.len(), 512);
    assert_eq!(key_513.len(), 513);
    assert!(ProductionKey::new(&key_512).is_ok());
    assert!(ProductionKey::new(&key_513).is_err());

    assert!(ProductionAtom::new("boundary atom", &"a".repeat(512)).is_ok());
    assert!(ProductionAtom::new("boundary atom", &"a".repeat(513)).is_err());
    assert!(Sha256HexDigest::new(&"a".repeat(64)).is_ok());
    assert!(Sha256HexDigest::new(&"a".repeat(63)).is_err());
    assert!(Sha256HexDigest::new(&"a".repeat(65)).is_err());
    assert!(Sha256HexDigest::new(&"A".repeat(64)).is_err());
}

#[test]
fn public_constants_match_protected_first_production_envelope() {
    assert_eq!(FIRST_PRODUCTION_MAX_MANIFEST_FIELDS, 20);
    assert_eq!(FIRST_PRODUCTION_MAX_MANIFEST_BYTES, 9_384);
    assert_eq!(FIRST_PRODUCTION_MAX_SERVER_ARTIFACT_BYTES, 4_304_614);
    assert_eq!(FIRST_PRODUCTION_MAX_CLIENT_ARTIFACT_BYTES, 34_248);
    assert_eq!(FIRST_PRODUCTION_MAX_GENERATION_PAIR_BYTES, 4_338_862);
    assert_eq!(FIRST_PRODUCTION_MAX_DECODED_FIELDS, 8_432);
    assert_eq!(FIRST_PRODUCTION_MAX_CELLS, 1_024);
    assert_eq!(FIRST_PRODUCTION_MAX_DEFINITIONS, 1_042);
    assert_eq!(FIRST_PRODUCTION_MAX_REFERENCES, 3_087);
    assert_eq!(FIRST_PRODUCTION_MAX_SERVER_RECORDS, 1_043);
    assert_eq!(FIRST_PRODUCTION_MAX_CLIENT_RECORDS, 6);
    assert_eq!(FIRST_PRODUCTION_MAX_SPAWN_POPULATION, 1);
    assert_eq!(FIRST_PRODUCTION_MAX_SCOPE_POPULATION, 1);
}
