#[test]
fn production_tampering_pair_mismatch_and_unknown_critical_fail_closed()
-> Result<(), ContentError> {
    let source = test_source(3)?;
    let compiled =
        compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;

    let mut truncated = compiled.server_artifact.clone();
    truncated.truncate(20);
    assert!(matches!(
        StagedGeneration::stage(
            &truncated,
            &compiled.client_artifact,
            compiled.expectation(),
        ),
        Err(ContentError::Truncated)
    ));

    let mut other_source = test_source(3)?;
    other_source.revisions.content = ProductionAtom::new("content revision", "content-r2")?;
    let other =
        compile_first_production(&other_source, FirstProductionCompileTarget::OrdinaryRelease)?;
    assert!(matches!(
        StagedGeneration::stage(
            &compiled.server_artifact,
            &other.client_artifact,
            compiled.expectation(),
        ),
        Err(ContentError::PairMismatch(_))
    ));
    assert!(matches!(
        StagedGeneration::stage(
            &compiled.server_artifact,
            &compiled.client_artifact,
            other.expectation(),
        ),
        Err(ContentError::RevisionMismatch(_))
    ));

    let mut unknown = compiled.server_artifact.clone();
    let second_entry = HEADER_LEN + SECTION_ENTRY_LEN;
    unknown[second_entry..second_entry + 2].copy_from_slice(&0x7ffe_u16.to_be_bytes());
    let payload_end = unknown.len() - TRAILER_LEN;
    let digest = sha256(&unknown[..payload_end]);
    unknown[payload_end..].copy_from_slice(&digest);
    assert!(matches!(
        StagedGeneration::stage(&unknown, &compiled.client_artifact, compiled.expectation(),),
        Err(ContentError::UnknownCriticalSection(0x7ffe))
    ));
    Ok(())
}

#[test]
fn every_record_kind_is_distinct() {
    let kinds = [
        RECORD_REGION,
        RECORD_AREA,
        RECORD_TERRAIN,
        RECORD_CELL,
        RECORD_RELOCATION,
        RECORD_BEHAVIOR,
        RECORD_PRESENTATION,
        RECORD_CREATURE,
        RECORD_SPAWN,
        RECORD_FORMULA,
        RECORD_EFFECT,
        RECORD_ABILITY,
        RECORD_ITEM,
        RECORD_LOOT_TABLE,
        RECORD_LOOT_ENTRY,
        RECORD_XP,
        RECORD_RNG_CONTEXT,
        RECORD_RNG_PURPOSE,
        RECORD_SPAWN_CELL,
        RECORD_CLIENT_CREATURE,
        RECORD_CLIENT_ABILITY,
        RECORD_CLIENT_ITEM,
    ];
    let distinct: BTreeSet<u8> = kinds.into_iter().collect();
    assert_eq!(distinct.len(), kinds.len());
    assert_eq!(RECORD_SPAWN_CELL, 19);
}

/// Amendment 04 §4: the maximum-cell graph with the two-placement spawn reaches exactly the
/// server-record and reference maxima; one more of each is refused.
#[test]
fn amendment_04_record_and_reference_maxima_are_exact() -> Result<(), ContentError> {
    let source = test_source_with_population(FIRST_PRODUCTION_MAX_CELLS, 2)?;
    let compiled =
        compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;
    StagedGeneration::stage(
        &compiled.server_artifact,
        &compiled.client_artifact,
        compiled.expectation(),
    )?;
    let canonical = canonicalize_first_production(&source)?;
    let records = server_records(&canonical.source)?;
    assert_eq!(records.len(), FIRST_PRODUCTION_MAX_SERVER_RECORDS);
    assert_eq!(
        first_production_reference_count(FIRST_PRODUCTION_MAX_CELLS, 2)?,
        FIRST_PRODUCTION_MAX_REFERENCES
    );
    assert!(matches!(
        FirstProductionLimits::v1().check(
            "first-production references",
            first_production_reference_count(FIRST_PRODUCTION_MAX_CELLS, 2)? + 1,
            FIRST_PRODUCTION_MAX_REFERENCES,
        ),
        Err(ContentError::LimitExceeded { .. })
    ));
    let mut extra = records;
    extra.push(extra[extra.len() - 1].clone());
    let metadata = ProductionArtifactMetadata::from_source(
        &canonical.source,
        ProductionProjection::ServerAuthoritative,
    )?;
    assert!(matches!(
        encode_artifact(&metadata, &extra),
        Err(ContentError::LimitExceeded {
            resource: "first-production records",
            ..
        })
    ));
    assert!(
        FirstProductionLimits::v1()
            .check(
                "first-production section bytes",
                FIRST_PRODUCTION_MAX_SECTION_BYTES,
                FIRST_PRODUCTION_MAX_SECTION_BYTES
            )
            .is_ok()
    );
    assert!(
        FirstProductionLimits::v1()
            .check(
                "first-production section bytes",
                FIRST_PRODUCTION_MAX_SECTION_BYTES + 1,
                FIRST_PRODUCTION_MAX_SECTION_BYTES
            )
            .is_err()
    );
    Ok(())
}

/// Amendment 04 §3: spawn-cell records follow their spawn record directly, in ordinal order,
/// and a missing, extra, out-of-order or dangling record refuses at staging.
#[test]
fn staging_refuses_malformed_spawn_cell_records() -> Result<(), ContentError> {
    let source = test_source_with_population(3, 2)?;
    let compiled =
        compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;
    let canonical = canonicalize_first_production(&source)?;
    let records = server_records(&canonical.source)?;
    let metadata = ProductionArtifactMetadata::from_source(
        &canonical.source,
        ProductionProjection::ServerAuthoritative,
    )?;
    let spawn_index = records
        .iter()
        .position(|record| record.kind == RECORD_SPAWN)
        .ok_or(ContentError::InvalidArtifact(
            "spawn record missing in test",
        ))?;
    let stage = |records: &[ProductionRecord]| -> Result<(), ContentError> {
        let crafted = encode_artifact(&metadata, records)?;
        StagedGeneration::stage(
            &crafted.bytes,
            &compiled.client_artifact,
            compiled.expectation(),
        )
        .map(|_| ())
    };

    let mut missing = records.clone();
    missing.remove(spawn_index + 2);
    assert!(stage(&missing).is_err());

    let mut swapped = records.clone();
    swapped.swap(spawn_index + 1, spawn_index + 2);
    assert!(matches!(
        stage(&swapped),
        Err(ContentError::InvalidArtifact(
            "first-production spawn cell record is out of order"
        ))
    ));

    let mut duplicate = records.clone();
    duplicate[spawn_index + 2].fields[2] = duplicate[spawn_index + 1].fields[2].clone();
    assert!(matches!(
        stage(&duplicate),
        Err(ContentError::InvalidArtifact(
            "first-production spawn cells must be distinct"
        ))
    ));

    let mut detached = records.clone();
    let moved = detached.remove(spawn_index + 2);
    detached.push(moved);
    assert!(stage(&detached).is_err());

    let mut wrong_owner = records;
    wrong_owner[spawn_index + 2].fields[0] = "oteryn:prod.other-spawn".to_owned();
    assert!(stage(&wrong_owner).is_err());
    Ok(())
}

#[test]
fn byte_and_decoded_field_boundaries_fail_closed_before_parse() {
    assert!(
        check_pair_lengths(
            FIRST_PRODUCTION_MAX_SERVER_ARTIFACT_BYTES,
            FIRST_PRODUCTION_MAX_CLIENT_ARTIFACT_BYTES
        )
        .is_ok()
    );
    assert!(matches!(
        check_pair_lengths(
            FIRST_PRODUCTION_MAX_SERVER_ARTIFACT_BYTES,
            FIRST_PRODUCTION_MAX_CLIENT_ARTIFACT_BYTES + 1
        ),
        Err(ContentError::LimitExceeded {
            resource: "first-production generation pair bytes",
            ..
        })
    ));
    assert!(check_decoded_fields(FIRST_PRODUCTION_MAX_DECODED_FIELDS).is_ok());
    assert!(check_decoded_fields(FIRST_PRODUCTION_MAX_DECODED_FIELDS + 1).is_err());
}

#[test]
fn corruption_and_oversize_fail_before_staging() -> Result<(), ContentError> {
    let source = test_source(3)?;
    let compiled =
        compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;
    let mut corrupt = compiled.server_artifact.clone();
    let index = corrupt.len() - 33;
    corrupt[index] ^= 0x5a;
    assert!(matches!(
        StagedGeneration::stage(&corrupt, &compiled.client_artifact, compiled.expectation(),),
        Err(ContentError::IntegrityMismatch(_))
    ));

    let oversized = vec![0u8; FIRST_PRODUCTION_MAX_SERVER_ARTIFACT_BYTES + 1];
    assert!(matches!(
        stage_artifact(&oversized, ProductionProjection::ServerAuthoritative),
        Err(ContentError::LimitExceeded {
            resource: "first-production server artifact bytes",
            ..
        })
    ));
    Ok(())
}
