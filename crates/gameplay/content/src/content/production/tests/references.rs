#[test]
fn explicit_event_policy_required_fails_closed_without_event_owner_policy()
-> Result<(), ContentError> {
    let mut invalid_source = test_source(3)?;
    invalid_source.spawns[0].multiplicity = MultiplicityClass::ExplicitEventPolicyRequired;
    assert!(matches!(
        compile_first_production(
            &invalid_source,
            FirstProductionCompileTarget::OrdinaryRelease
        ),
        Err(ContentError::InvalidArtifact(
            "first-production explicit event policy is unresolved"
        ))
    ));

    let source = test_source(3)?;
    let compiled =
        compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;
    let mut records = server_records(&source)?;
    let spawn = records
        .iter_mut()
        .find(|record| record.kind == RECORD_SPAWN)
        .ok_or(ContentError::InvalidArtifact(
            "spawn record missing in explicit-event policy test",
        ))?;
    spawn.fields[5] = "EXPLICIT_EVENT_POLICY_REQUIRED".to_owned();
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
        Err(ContentError::InvalidArtifact(
            "first-production explicit event policy is unresolved"
        ))
    ));
    Ok(())
}
#[test]
fn cross_family_references_fail_in_compiler_and_staging() -> Result<(), ContentError> {
    let mut invalid_source = test_source(3)?;
    invalid_source.spawns[0].creature_key = invalid_source.cells[0].key.clone();
    assert!(matches!(
        compile_first_production(
            &invalid_source,
            FirstProductionCompileTarget::OrdinaryRelease
        ),
        Err(ContentError::MissingReference { .. })
    ));

    let source = test_source(3)?;
    let compiled =
        compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;
    let mut records = server_records(&source)?;
    let spawn = records
        .iter_mut()
        .find(|record| record.kind == RECORD_SPAWN)
        .ok_or(ContentError::InvalidArtifact(
            "spawn record missing in test",
        ))?;
    spawn.fields[1] = source.cells[0].key.as_str().to_owned();

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
        Err(ContentError::MissingReference { .. })
    ));
    Ok(())
}

#[test]
fn projection_pair_requires_shared_semantic_equivalence() -> Result<(), ContentError> {
    let source = test_source(3)?;
    let compiled =
        compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;
    let mut records = client_records(&source);

    let presentation = records
        .iter_mut()
        .find(|record| record.kind == RECORD_PRESENTATION)
        .ok_or(ContentError::InvalidArtifact(
            "presentation record missing in test",
        ))?;
    presentation.fields[1] = "appearance:tampered-v2".to_owned();
    let metadata =
        ProductionArtifactMetadata::from_source(&source, ProductionProjection::ClientSafe)?;
    let crafted = encode_artifact(&metadata, &records)?;
    assert!(matches!(
        StagedGeneration::stage(
            &compiled.server_artifact,
            &crafted.bytes,
            compiled.expectation(),
        ),
        Err(ContentError::PairMismatch(_))
    ));
    Ok(())
}

#[test]
fn staging_revalidates_behavior_policy_and_rejects_payload_gaps() -> Result<(), ContentError> {
    let source = test_source(3)?;
    let compiled =
        compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;

    let mut records = server_records(&source)?;
    let behavior = records
        .iter_mut()
        .find(|record| record.kind == RECORD_BEHAVIOR)
        .ok_or(ContentError::InvalidArtifact(
            "behavior record missing in test",
        ))?;
    behavior.fields[1] = "evidence:behavior-policy-r1".to_owned();
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
        Err(ContentError::InvalidString(
            "first-production behavior policy revision"
        ))
    ));

    let mut gap = compiled.server_artifact.clone();
    let old_payload_end = usize::try_from(read_u32_at(&gap, 20)?)
        .map_err(|_| ContentError::InvalidSectionBounds)?;
    gap.insert(old_payload_end, b'X');

    let new_payload_end = old_payload_end
        .checked_add(1)
        .ok_or(ContentError::InvalidSectionBounds)?;
    gap[20..24].copy_from_slice(&to_u32(new_payload_end)?.to_be_bytes());
    let digest = sha256(&gap[..new_payload_end]);
    gap[new_payload_end..new_payload_end + TRAILER_LEN].copy_from_slice(&digest);
    assert!(matches!(
        StagedGeneration::stage(&gap, &compiled.client_artifact, compiled.expectation(),),
        Err(ContentError::InvalidSectionBounds)
    ));
    Ok(())
}

#[test]
fn activated_runtime_indexes_loot_entry_by_entry_key() -> Result<(), ContentError> {
    let source = test_source(3)?;
    let compiled =
        compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;
    let staged = StagedGeneration::stage(
        &compiled.server_artifact,
        &compiled.client_artifact,
        compiled.expectation(),
    )?;

    let fields = staged
        .runtime_state()
        .server_definition_fields("oteryn:prod.loot.entry")
        .ok_or(ContentError::InvalidArtifact(
            "loot entry not indexed by entry key",
        ))?;
    assert_eq!(
        fields.first().map(String::as_str),
        Some("oteryn:prod.loot.entry")
    );
    assert_eq!(
        fields.get(1).map(String::as_str),
        Some("oteryn:prod.loot.table")
    );
    Ok(())
}
