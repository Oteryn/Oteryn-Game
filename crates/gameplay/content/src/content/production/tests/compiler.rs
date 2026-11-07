#[test]
fn production_provenance_grammar_has_stable_golden_digest() -> Result<(), ContentError> {
    let source = test_source(1)?;
    let binding = &source.package_manifest;
    let digest = binding.package_provenance_digest()?;
    assert_eq!(
        digest.as_str(),
        "b1028ffc978a4b96cea3d43283c010242a3c2f46679521f930a902ee8f71370a"
    );
    let preimage = binding.provenance_preimage()?;
    assert_eq!(&preimage[..4], &(31_u32.to_be_bytes()));
    Ok(())
}

#[test]
fn ordinary_release_is_deterministic_after_enumeration_shuffle() -> Result<(), ContentError> {
    let source = test_source(4)?;
    let mut shuffled = source.clone();
    shuffled.reverse_enumeration_for_test();
    let first =
        compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;
    let second =
        compile_first_production(&shuffled, FirstProductionCompileTarget::OrdinaryRelease)?;
    assert_eq!(first.server_artifact, second.server_artifact);
    assert_eq!(first.client_artifact, second.client_artifact);
    Ok(())
}

#[test]
fn exact_profile_maxima_and_footprint_are_enforced() -> Result<(), ContentError> {
    let source = test_source(FIRST_PRODUCTION_MAX_CELLS)?;
    compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;

    let minimum = test_source(3)?;
    compile_first_production(&minimum, FirstProductionCompileTarget::OrdinaryRelease)?;
    let below_minimum = test_source(2)?;
    assert!(matches!(
        compile_first_production(
            &below_minimum,
            FirstProductionCompileTarget::OrdinaryRelease
        ),
        Err(ContentError::InvalidArtifact(
            "first-production cells must contain at least 3 entries"
        ))
    ));

    let too_many = test_source(FIRST_PRODUCTION_MAX_CELLS + 1)?;
    assert!(matches!(
        compile_first_production(&too_many, FirstProductionCompileTarget::OrdinaryRelease),
        Err(ContentError::LimitExceeded {
            resource: "first-production cells",
            ..
        })
    ));

    let mut too_wide = test_source(3)?;
    too_wide.cells[2].x = 32;
    assert!(matches!(
        compile_first_production(&too_wide, FirstProductionCompileTarget::OrdinaryRelease),
        Err(ContentError::LimitExceeded {
            resource: "first-production x span",
            ..
        })
    ));

    let mut too_tall = test_source(3)?;
    too_tall.cells[2].y = 32;
    assert!(matches!(
        compile_first_production(&too_tall, FirstProductionCompileTarget::OrdinaryRelease),
        Err(ContentError::LimitExceeded {
            resource: "first-production y span",
            ..
        })
    ));

    let mut second_floor = test_source(3)?;
    second_floor.cells[1].z = 8;
    assert!(matches!(
        compile_first_production(&second_floor, FirstProductionCompileTarget::OrdinaryRelease),
        Err(ContentError::LimitExceeded {
            resource: "first-production floors must equal 1",
            ..
        })
    ));
    Ok(())
}

#[test]
fn population_and_content_lock_fail_closed() -> Result<(), ContentError> {
    let mut zero = test_source(3)?;
    zero.spawns[0].population_limit = 0;
    assert!(
        compile_first_production(&zero, FirstProductionCompileTarget::OrdinaryRelease).is_err()
    );

    // Amendment 04: population 2 with two distinct placement cells is the accepted maximum.
    let two = test_source_with_population(3, 2)?;
    let compiled =
        compile_first_production(&two, FirstProductionCompileTarget::OrdinaryRelease)?;
    StagedGeneration::stage(
        &compiled.server_artifact,
        &compiled.client_artifact,
        compiled.expectation(),
    )?;

    let mut three = test_source_with_population(3, 3)?;
    assert!(matches!(
        compile_first_production(&three, FirstProductionCompileTarget::OrdinaryRelease),
        Err(ContentError::LimitExceeded {
            resource: "first-production spawn population per spawn",
            actual: 3,
            limit: 2,
        })
    ));

    let server_metadata = ProductionArtifactMetadata::from_source(
        &three,
        ProductionProjection::ServerAuthoritative,
    )?;
    let client_metadata =
        ProductionArtifactMetadata::from_source(&three, ProductionProjection::ClientSafe)?;
    let server = encode_artifact(&server_metadata, &server_records(&three)?)?;
    let client = encode_artifact(&client_metadata, &client_records(&three))?;
    let expected =
        FirstProductionExpectation::from_source(&three, server.digest, client.digest)?;
    assert!(matches!(
        StagedGeneration::stage(&server.bytes, &client.bytes, &expected),
        Err(ContentError::LimitExceeded {
            resource: "first-production spawn population per spawn",
            actual: 3,
            limit: 2,
        })
    ));

    // The placement list must match the population exactly and be duplicate-free.
    three.spawns[0].population_limit = 2;
    assert!(matches!(
        compile_first_production(&three, FirstProductionCompileTarget::OrdinaryRelease),
        Err(ContentError::InvalidArtifact(
            "first-production spawn cell count must equal its population"
        ))
    ));
    let mut duplicate = test_source_with_population(3, 2)?;
    duplicate.spawns[0].cell_keys[1] = duplicate.spawns[0].cell_keys[0].clone();
    assert!(matches!(
        compile_first_production(&duplicate, FirstProductionCompileTarget::OrdinaryRelease),
        Err(ContentError::InvalidArtifact(
            "first-production spawn cells must be distinct"
        ))
    ));
    let mut dangling = test_source_with_population(3, 2)?;
    dangling.spawns[0].cell_keys[1] = ProductionKey::new("oteryn:prod.cell.missing")?;
    assert!(matches!(
        compile_first_production(&dangling, FirstProductionCompileTarget::OrdinaryRelease),
        Err(ContentError::MissingReference { .. })
    ));

    let mut empty_lock = test_source(3)?;
    empty_lock.content_lock.entries.clear();
    assert!(
        compile_first_production(&empty_lock, FirstProductionCompileTarget::OrdinaryRelease)
            .is_err()
    );

    let mut dependency_lock = test_source(3)?;
    dependency_lock.content_lock.entries[0].dependency = true;
    assert!(
        compile_first_production(
            &dependency_lock,
            FirstProductionCompileTarget::OrdinaryRelease
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn nonproduction_markers_and_digest_shape_are_rejected() {
    assert!(ProductionKey::new("oteryn:fixture.item").is_err());
    assert!(ProductionKey::new("oteryn:test.formula").is_err());
    assert!(ProductionKey::new("oteryn:prod.test.formula").is_err());
    assert!(ProductionAtom::new("presentation", "synthetic://asset").is_err());
    assert!(ProductionAtom::new("profile", "evidence:test-v1").is_err());
    assert!(
        Sha256HexDigest::new(
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        )
        .is_err()
    );
    assert!(Sha256HexDigest::new("abc").is_err());
}
