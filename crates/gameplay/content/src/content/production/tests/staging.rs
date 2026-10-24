#[test]
fn staged_pair_revalidates_expected_provenance_and_projection() -> Result<(), ContentError> {
    let source = test_source(3)?;
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

    assert_eq!(staged.runtime_state().server_record_count(), 23);
    assert_eq!(staged.runtime_state().client_record_count(), 6);
    assert_eq!(
        staged
            .runtime_state()
            .server_definition_fields("oteryn:prod.creature")
            .and_then(|fields| fields.get(3))
            .map(String::as_str),
        Some("creature-policy-r1")
    );
    assert!(matches!(
        stage_artifact(
            &compiled.client_artifact,
            ProductionProjection::ServerAuthoritative
        ),
        Err(ContentError::PairMismatch(_))
    ));
    Ok(())
}

#[test]
fn staging_rejects_self_consistent_artifact_below_cell_minimum() -> Result<(), ContentError> {
    let source = test_source(2)?;
    let server_metadata = ProductionArtifactMetadata::from_source(
        &source,
        ProductionProjection::ServerAuthoritative,
    )?;
    let client_metadata =
        ProductionArtifactMetadata::from_source(&source, ProductionProjection::ClientSafe)?;
    let server = encode_artifact(&server_metadata, &server_records(&source)?)?;
    let client = encode_artifact(&client_metadata, &client_records(&source))?;
    let expected =
        FirstProductionExpectation::from_source(&source, server.digest, client.digest)?;

    assert!(matches!(
        StagedGeneration::stage(&server.bytes, &client.bytes, &expected),
        Err(ContentError::InvalidArtifact(
            "first-production cells must contain at least 3 entries"
        ))
    ));
    Ok(())
}

#[test]
fn staging_expectation_binds_exact_artifact_digest_pair() -> Result<(), ContentError> {
    let source = test_source(3)?;
    let compiled =
        compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;
    let mut records = server_records(&source)?;
    let cell = records
        .iter_mut()
        .find(|record| record.kind == RECORD_CELL)
        .ok_or(ContentError::InvalidArtifact("cell record missing in test"))?;
    cell.fields[7] = if cell.fields[7] == "walkable" {
        "blocked".to_owned()
    } else {
        "walkable".to_owned()
    };
    let metadata = ProductionArtifactMetadata::from_source(
        &source,
        ProductionProjection::ServerAuthoritative,
    )?;
    let crafted = encode_artifact(&metadata, &records)?;

    assert!(matches!(
        StagedGeneration::stage(
            &crafted.bytes,
            &compiled.client_artifact,
            compiled.expectation(),
        ),
        Err(ContentError::RevisionMismatch(
            "first-production expected artifact digest pair"
        ))
    ));
    Ok(())
}

#[test]
fn world_id_is_typed_uuidv7_and_round_trips_through_artifacts() -> Result<(), ContentError> {
    let source = test_source(3)?;
    let compiled =
        compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;
    let staged = StagedGeneration::stage(
        &compiled.server_artifact,
        &compiled.client_artifact,
        compiled.expectation(),
    )?;
    assert_eq!(staged.identity().world_id(), source.world_id);
    assert!(parse_world_id("world-prod-v1").is_err());
    assert!(parse_world_id("00000000000000000000000000000000").is_err());
    Ok(())
}

#[test]
fn presentation_references_are_one_to_one_and_cover_all_presentations()
-> Result<(), ContentError> {
    let mut source = test_source(3)?;
    source.abilities[0].presentation_key = source.creatures[0].presentation_key.clone();
    assert!(matches!(
        compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease),
        Err(ContentError::InvalidArtifact(
            "first-production presentation references must be distinct and cover all presentations"
        ))
    ));

    let server_metadata = ProductionArtifactMetadata::from_source(
        &source,
        ProductionProjection::ServerAuthoritative,
    )?;
    let client_metadata =
        ProductionArtifactMetadata::from_source(&source, ProductionProjection::ClientSafe)?;
    let server = encode_artifact(&server_metadata, &server_records(&source)?)?;
    let client = encode_artifact(&client_metadata, &client_records(&source))?;
    let expected =
        FirstProductionExpectation::from_source(&source, server.digest, client.digest)?;
    assert!(matches!(
        StagedGeneration::stage(&server.bytes, &client.bytes, &expected),
        Err(ContentError::InvalidArtifact(
            "first-production presentation references must be distinct and cover all presentations"
        ))
    ));
    Ok(())
}
