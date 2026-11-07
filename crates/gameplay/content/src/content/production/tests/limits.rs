#[test]
fn protected_registry_constants_and_text_boundaries_match() -> Result<(), ContentError> {
    assert_eq!(FIRST_PRODUCTION_MAX_MANIFEST_FIELDS, 20);
    assert_eq!(FIRST_PRODUCTION_MAX_MANIFEST_BYTES, 9_384);
    assert_eq!(FIRST_PRODUCTION_MAX_SERVER_ARTIFACT_BYTES, 4_312_850);
    assert_eq!(FIRST_PRODUCTION_MAX_CLIENT_ARTIFACT_BYTES, 34_248);
    assert_eq!(FIRST_PRODUCTION_MAX_GENERATION_PAIR_BYTES, 4_347_098);
    assert_eq!(FIRST_PRODUCTION_MAX_DECODED_FIELDS, 8_448);
    assert_eq!(FIRST_PRODUCTION_MAX_SECTION_BYTES, 4_303_314);
    assert_eq!(FIRST_PRODUCTION_MAX_RECORD_BYTES, 4_114);
    assert_eq!(FIRST_PRODUCTION_MAX_DEFINITIONS, 1_042);
    assert_eq!(FIRST_PRODUCTION_MAX_REFERENCES, 3_090);
    assert_eq!(FIRST_PRODUCTION_MAX_SERVER_RECORDS, 1_045);
    assert_eq!(FIRST_PRODUCTION_MAX_SPAWN_POPULATION, 2);
    assert_eq!(FIRST_PRODUCTION_MAX_SCOPE_POPULATION, 2);
    // Amendment 04 §4: every record may reach the record maximum plus its 4-byte length.
    assert_eq!(
        FIRST_PRODUCTION_MAX_SECTION_BYTES,
        FIRST_PRODUCTION_MAX_SERVER_RECORDS * (4 + FIRST_PRODUCTION_MAX_RECORD_BYTES) + 4
    );
    assert_eq!(FIRST_PRODUCTION_MAX_CLIENT_RECORDS, 6);
    assert_eq!(
        FIRST_PRODUCTION_MAX_SERVER_ARTIFACT_BYTES + FIRST_PRODUCTION_MAX_CLIENT_ARTIFACT_BYTES,
        FIRST_PRODUCTION_MAX_GENERATION_PAIR_BYTES
    );

    let key_512 = format!("o:{}", "a".repeat(510));
    let key_513 = format!("o:{}", "a".repeat(511));
    assert_eq!(key_512.len(), 512);
    assert_eq!(key_513.len(), 513);
    ProductionKey::new(&key_512)?;
    assert!(matches!(
        ProductionKey::new(&key_513),
        Err(ContentError::LimitExceeded { .. })
    ));

    let atom_512 = "a".repeat(512);
    let atom_513 = "a".repeat(513);
    ProductionAtom::new("boundary atom", &atom_512)?;
    assert!(matches!(
        ProductionAtom::new("boundary atom", &atom_513),
        Err(ContentError::LimitExceeded { .. })
    ));
    assert!(Sha256HexDigest::new(&"a".repeat(63)).is_err());
    assert!(matches!(
        Sha256HexDigest::new(&"a".repeat(65)),
        Err(ContentError::LimitExceeded {
            resource: "first-production sha256 digest bytes",
            actual: 65,
            limit: 64,
        })
    ));
    assert!(Sha256HexDigest::new(&format!("{}g", "a".repeat(63))).is_err());
    Ok(())
}

#[test]
fn every_runtime_representable_family_rejects_max_plus_one() -> Result<(), ContentError> {
    macro_rules! reject_extra {
        ($field:ident) => {{
            let mut source = test_source(3)?;
            let extra = source.$field[0].clone();
            source.$field.push(extra);
            assert!(
                matches!(
                    compile_first_production(
                        &source,
                        FirstProductionCompileTarget::OrdinaryRelease
                    ),
                    Err(ContentError::LimitExceeded { .. })
                ),
                "{} did not classify max+1 as capacity exceeded",
                stringify!($field)
            );
        }};
    }

    reject_extra!(regions);
    reject_extra!(areas);
    reject_extra!(terrains);
    reject_extra!(relocations);
    reject_extra!(behaviors);
    reject_extra!(presentations);
    reject_extra!(creatures);
    reject_extra!(spawns);
    reject_extra!(formula_profiles);
    reject_extra!(effects);
    reject_extra!(abilities);
    reject_extra!(items);
    reject_extra!(loot_tables);
    reject_extra!(xp_definitions);

    let mut loot_entry = test_source(3)?;
    let extra = loot_entry.loot_tables[0].entries[0].clone();
    loot_entry.loot_tables[0].entries.push(extra);
    assert!(matches!(
        compile_first_production(&loot_entry, FirstProductionCompileTarget::OrdinaryRelease),
        Err(ContentError::LimitExceeded { .. })
    ));

    let mut rng_purpose = test_source(3)?;
    rng_purpose
        .rng
        .purpose_keys
        .push(rng_purpose.rng.purpose_keys[0].clone());
    assert!(matches!(
        compile_first_production(&rng_purpose, FirstProductionCompileTarget::OrdinaryRelease),
        Err(ContentError::LimitExceeded { .. })
    ));

    let mut second_lock = test_source(3)?;
    second_lock
        .content_lock
        .entries
        .push(second_lock.content_lock.entries[0].clone());
    assert!(matches!(
        compile_first_production(&second_lock, FirstProductionCompileTarget::OrdinaryRelease),
        Err(ContentError::LimitExceeded { .. })
    ));
    Ok(())
}

#[test]
fn duplicate_unresolved_capability_and_migration_escape_fail_closed() -> Result<(), ContentError>
{
    let mut duplicate = test_source(3)?;
    duplicate.items[0].key = duplicate.abilities[0].key.clone();
    assert!(matches!(
        compile_first_production(&duplicate, FirstProductionCompileTarget::OrdinaryRelease),
        Err(ContentError::DuplicateKey(_))
    ));

    let mut unresolved = test_source(3)?;
    unresolved.abilities[0].effect_key = ProductionKey::new("oteryn:prod.effect.missing")?;
    assert!(matches!(
        compile_first_production(&unresolved, FirstProductionCompileTarget::OrdinaryRelease),
        Err(ContentError::MissingReference { .. })
    ));

    let mut capability = test_source(3)?;
    capability.capability_profile =
        ProductionAtom::new("capability profile", "content:first-production-v2")?;
    assert!(
        compile_first_production(&capability, FirstProductionCompileTarget::OrdinaryRelease)
            .is_err()
    );

    let mut migration = test_source(3)?;
    migration.migration_class = DurableMigrationClass::MigrationRequired;
    assert!(
        compile_first_production(&migration, FirstProductionCompileTarget::OrdinaryRelease)
            .is_err()
    );
    Ok(())
}
