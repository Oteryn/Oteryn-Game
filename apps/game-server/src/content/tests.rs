use super::artifact::{test_reseal_body_offset, test_reseal_unknown_critical_section};
use super::digest::sha256;
use super::*;

fn limits() -> Result<EvidenceLimits, ContentError> {
    EvidenceLimits::new(
        "evidence:test-v1",
        262_144,
        8,
        131_072,
        256,
        4_096,
        128,
        256,
        256,
        64,
        1_024,
    )
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

#[test]
fn sha256_matches_known_vector() {
    assert_eq!(
        sha256(b"abc"),
        [
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
            0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
            0xf2, 0x00, 0x15, 0xad,
        ]
    );
}

#[test]
fn compile_is_byte_identical_after_source_enumeration_shuffle() -> Result<(), ContentError> {
    let limits = limits()?;
    let source = synthetic_vsl_fixture(&limits)?;
    let mut shuffled = source.clone();
    shuffled.reverse_enumeration_for_test();

    let first = compile(&source, &limits, CompileTarget::Evidence)?;
    let second = compile(&shuffled, &limits, CompileTarget::Evidence)?;

    assert_eq!(first.server_artifact, second.server_artifact);
    assert_eq!(first.client_artifact, second.client_artifact);
    assert_eq!(first.server_digest(), second.server_digest());
    assert_eq!(first.client_digest(), second.client_digest());
    Ok(())
}

#[test]
fn duplicate_semantic_key_fails_compilation() -> Result<(), ContentError> {
    let limits = limits()?;
    let mut source = synthetic_vsl_fixture(&limits)?;
    let duplicate = source.items[0].clone();
    source.items.push(duplicate);

    let result = compile(&source, &limits, CompileTarget::Evidence);
    assert!(matches!(result, Err(ContentError::DuplicateKey(_))));
    Ok(())
}

#[test]
fn missing_reference_fails_compilation() -> Result<(), ContentError> {
    let limits = limits()?;
    let mut source = synthetic_vsl_fixture(&limits)?;
    source.abilities[0].effect_key = ContentKey::new("oteryn:vsl.effect.missing", &limits)?;

    let result = compile(&source, &limits, CompileTarget::Evidence);
    assert!(matches!(result, Err(ContentError::MissingReference { .. })));
    Ok(())
}

#[test]
fn value_producing_spawn_requires_channel_classification() -> Result<(), ContentError> {
    let limits = limits()?;
    let mut source = synthetic_vsl_fixture(&limits)?;
    source.spawns[0].multiplicity = None;

    let missing_multiplicity = compile(&source, &limits, CompileTarget::Evidence);
    assert!(matches!(
        missing_multiplicity,
        Err(ContentError::MissingSourceClassification(_))
    ));

    let mut source = synthetic_vsl_fixture(&limits)?;
    source.spawns[0].eligibility_scope = None;
    let missing_eligibility = compile(&source, &limits, CompileTarget::Evidence);
    assert!(matches!(
        missing_eligibility,
        Err(ContentError::MissingSourceClassification(_))
    ));
    Ok(())
}

#[test]
fn client_artifact_excludes_server_only_fixture_fields() -> Result<(), ContentError> {
    let limits = limits()?;
    let source = synthetic_vsl_fixture(&limits)?;
    let compiled = compile(&source, &limits, CompileTarget::Evidence)?;

    let rng_marker = b"SYNTHETIC_TEST_ROOT_NOT_SECRET";
    let loot_marker = b"oteryn:vsl.loot.table.fixture";
    assert!(contains_bytes(&compiled.server_artifact, rng_marker));
    assert!(contains_bytes(&compiled.server_artifact, loot_marker));
    assert!(!contains_bytes(&compiled.client_artifact, rng_marker));
    assert!(!contains_bytes(&compiled.client_artifact, loot_marker));
    Ok(())
}

#[test]
fn fixture_profile_cannot_compile_for_ordinary_release() -> Result<(), ContentError> {
    let limits = limits()?;
    let source = synthetic_vsl_fixture(&limits)?;

    let result = compile(&source, &limits, CompileTarget::OrdinaryRelease);
    assert!(matches!(
        result,
        Err(ContentError::FixtureOnlyReleaseRejected)
    ));
    Ok(())
}

#[test]
fn valid_pair_stages_and_activates_as_one_revision() -> Result<(), ContentError> {
    let limits = limits()?;
    let source = synthetic_vsl_fixture(&limits)?;
    let compiled = compile(&source, &limits, CompileTarget::Evidence)?;
    let expectation = compiled.expectation();

    let pair = StagedContentPair::stage(
        &compiled.server_artifact,
        &compiled.client_artifact,
        &limits,
    )?;
    pair.verify_expected(&expectation)?;

    let mut slot = ActivationSlot::new();
    slot.stage_and_activate(
        &compiled.server_artifact,
        &compiled.client_artifact,
        &expectation,
        &limits,
    )?;

    let active = slot.active();
    assert!(active.is_some());
    assert_eq!(
        active.map(ActiveContent::content_revision),
        Some(expectation.content_revision.as_str())
    );
    Ok(())
}

#[test]
fn corrupt_artifact_fails_without_replacing_active_revision() -> Result<(), ContentError> {
    let limits = limits()?;
    let source = synthetic_vsl_fixture(&limits)?;
    let compiled = compile(&source, &limits, CompileTarget::Evidence)?;
    let expectation = compiled.expectation();
    let mut slot = ActivationSlot::new();

    slot.stage_and_activate(
        &compiled.server_artifact,
        &compiled.client_artifact,
        &expectation,
        &limits,
    )?;
    let before = slot.active().cloned();

    let mut corrupt = compiled.server_artifact.clone();
    let index = corrupt.len() - 33;
    corrupt[index] ^= 0x5a;

    let result =
        slot.stage_and_activate(&corrupt, &compiled.client_artifact, &expectation, &limits);
    assert!(matches!(result, Err(ContentError::IntegrityMismatch(_))));
    assert_eq!(slot.active().cloned(), before);
    Ok(())
}

#[test]
fn truncated_and_oversized_artifacts_fail_before_staging() -> Result<(), ContentError> {
    let limits = limits()?;
    let source = synthetic_vsl_fixture(&limits)?;
    let compiled = compile(&source, &limits, CompileTarget::Evidence)?;

    let mut truncated = compiled.server_artifact.clone();
    truncated.truncate(20);
    let truncated_result = StagedArtifact::stage(&truncated, &limits);
    assert!(matches!(truncated_result, Err(ContentError::Truncated)));

    let smaller_limit = limits.with_max_artifact_bytes(compiled.server_artifact.len() - 1)?;
    let oversized_result = StagedArtifact::stage(&compiled.server_artifact, &smaller_limit);
    assert!(matches!(
        oversized_result,
        Err(ContentError::LimitExceeded {
            resource: "artifact bytes",
            ..
        })
    ));
    Ok(())
}

#[test]
fn incompatible_server_client_revision_pair_fails_closed() -> Result<(), ContentError> {
    let limits = limits()?;
    let source_a = synthetic_vsl_fixture(&limits)?;
    let mut source_b = source_a.clone();
    source_b.revisions.content = RevisionToken::new("vsl-content-r2", &limits)?;

    let compiled_a = compile(&source_a, &limits, CompileTarget::Evidence)?;
    let compiled_b = compile(&source_b, &limits, CompileTarget::Evidence)?;

    let result = StagedContentPair::stage(
        &compiled_a.server_artifact,
        &compiled_b.client_artifact,
        &limits,
    );
    assert!(matches!(result, Err(ContentError::PairMismatch(_))));
    Ok(())
}

#[test]
fn unknown_critical_section_is_rejected_even_with_valid_digest() -> Result<(), ContentError> {
    let limits = limits()?;
    let source = synthetic_vsl_fixture(&limits)?;
    let compiled = compile(&source, &limits, CompileTarget::Evidence)?;
    let tampered = test_reseal_unknown_critical_section(&compiled.server_artifact)?;

    let result = StagedArtifact::stage(&tampered, &limits);
    assert!(matches!(
        result,
        Err(ContentError::UnknownCriticalSection(_))
    ));
    Ok(())
}

#[test]
fn overflowing_section_bounds_are_rejected_even_with_valid_digest() -> Result<(), ContentError> {
    let limits = limits()?;
    let source = synthetic_vsl_fixture(&limits)?;
    let compiled = compile(&source, &limits, CompileTarget::Evidence)?;
    let tampered = test_reseal_body_offset(&compiled.server_artifact, u32::MAX)?;

    let result = StagedArtifact::stage(&tampered, &limits);
    assert!(matches!(result, Err(ContentError::InvalidSectionBounds)));
    Ok(())
}

#[test]
fn evidence_limits_have_no_unbounded_or_production_default() {
    let bad_profile = EvidenceLimits::new("production", 1, 1, 1, 1, 1, 1, 1, 1, 1, 1);
    assert!(matches!(
        bad_profile,
        Err(ContentError::InvalidLimitProfile)
    ));

    let zero_limit = EvidenceLimits::new("evidence:test", 0, 1, 1, 1, 1, 1, 1, 1, 1, 1);
    assert!(matches!(zero_limit, Err(ContentError::InvalidLimit(_))));
}
