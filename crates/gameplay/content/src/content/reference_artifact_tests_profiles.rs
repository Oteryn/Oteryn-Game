fn typed_family(
    semantics: ReferenceItemSemantics,
) -> Result<CanonicalReferencePlayableContent, Box<dyn std::error::Error>> {
    const B1_EVIDENCE: &[u8] = include_bytes!(
        "../../../../../docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json"
    );
    let limits = ProjectEvidenceLimits {
        max_documents: 8,
        max_document_bytes: 128_000_000,
        max_total_bytes: 220_000_000,
        max_json_depth: 24,
        max_decoded_fields: 3_000_000,
        max_string_bytes: 128_000_000,
        max_locator_bytes: 160,
        max_locator_segments: 8,
        max_reference_records: CW2_B1_FULL_ITEM_FAMILY_COUNT,
        max_import_records: CW2_B1_FULL_ITEM_FAMILY_COUNT,
        max_reimport_states: CW2_B1_FULL_ITEM_FAMILY_COUNT,
    };
    let mut imported = protected_cw2_b1_full_item_family_import(B1_EVIDENCE)?;
    let record = imported
        .records
        .iter_mut()
        .find_map(|record| match record {
            ProjectReferenceRecord::Item {
                semantics: target, ..
            } => Some(target),
            _ => None,
        })
        .ok_or(ContentError::InvalidArtifact("full family has an Item"))?;
    *record = semantics;
    let canonical = CanonicalProjectDocuments::from_draft(
        ProjectDraft {
            project_revision: "project-r1".to_owned(),
            package_key: "oteryn:content.world-project".to_owned(),
            semantic_schema_version: "reference-schema-v1".to_owned(),
            licensing_metadata: "PENDING".to_owned(),
            world_id: "0123456789ab70cd8ef0123456789abc".to_owned(),
            coordinate_frame: "global-target-2026-09-27".to_owned(),
            records: imported.records,
            imports: vec![imported.batch],
            metadata: Vec::new(),
        },
        limits,
    )?;
    Ok(canonical.into_snapshot(limits)?.parse(limits)?.link()?)
}

fn reprofile(bytes: &[u8], magic: [u8; 8], version: u16) -> Vec<u8> {
    let mut patched = bytes.to_vec();
    patched[..8].copy_from_slice(&magic);
    patched[8..10].copy_from_slice(&version.to_be_bytes());
    let payload_end = patched.len() - TRAILER_LEN;
    let digest = sha256(&patched[..payload_end]);
    patched[payload_end..].copy_from_slice(&digest);
    patched
}

#[test]
fn compiler_writes_only_v7_and_v4_to_v6_artifacts_still_decode_under_their_profile()
-> Result<(), Box<dyn std::error::Error>> {
    use ReferenceItemField::Known;
    let presentation = ReferenceItemSemantics {
        presentation: Known(ReferenceItemPresentation {
            name: Known("power bolt".to_owned()),
            description: ReferenceItemField::Unknown,
        }),
        ..Default::default()
    };
    let linked = typed_family(presentation.clone())?;
    let compiled = super::compile(&linked)?;
    let server = ReferencePlayableArtifactView::load(
        &compiled.server_artifact,
        ReferenceArtifactProjection::ServerAuthoritative,
    )?;
    assert_eq!(server.artifact_profile_id(), TYPED_V7_ARTIFACT_PROFILE_ID);
    let identity = linked.definitions[0].definition.clone();

    // A v6 pair written before ITEM-SEM-BED-1 loads, stages and looks up under v6.
    let v6 = compile_with_profile(&linked, V6)?;
    let staged = NonAuthoritativeReferenceStage::stage(
        &v6.server_artifact,
        &v6.client_artifact,
        v6.expectation(),
    )?;
    assert_eq!(
        staged.server_artifact().artifact_profile_id(),
        TYPED_V6_ARTIFACT_PROFILE_ID
    );
    assert_eq!(
        staged
            .server_artifact()
            .lookup_server_item(&identity)?
            .map(|item| item.semantics),
        Some(linked_semantics(&linked, 0))
    );
    // A v6 reader refuses v7 bytes by profile id, and a v7 reader refuses v6 bytes.
    for (bytes, magic, version) in [
        (
            &compiled.server_artifact,
            TYPED_V6_MAGIC,
            TYPED_V6_PROFILE_VERSION,
        ),
        (
            &v6.server_artifact,
            TYPED_V7_MAGIC,
            TYPED_V7_PROFILE_VERSION,
        ),
    ] {
        assert!(matches!(
            ReferencePlayableArtifactView::load(
                &reprofile(bytes, magic, version),
                ReferenceArtifactProjection::ServerAuthoritative
            ),
            Err(ContentError::InvalidArtifact(
                "unexpected Reference artifact profile id"
            ))
        ));
    }

    // A v5 pair written before ITEM-SEM-USE-1 loads, stages and looks up under v5.
    let v5 = compile_with_profile(&linked, V5)?;
    let staged = NonAuthoritativeReferenceStage::stage(
        &v5.server_artifact,
        &v5.client_artifact,
        v5.expectation(),
    )?;
    assert_eq!(
        staged.server_artifact().artifact_profile_id(),
        TYPED_V5_ARTIFACT_PROFILE_ID
    );
    assert_eq!(
        staged
            .server_artifact()
            .lookup_server_item(&identity)?
            .map(|item| item.semantics),
        Some(linked_semantics(&linked, 0))
    );
    // A v5 reader refuses v6 bytes by profile id, and a v6 reader refuses v5 bytes.
    for (bytes, magic, version) in [
        (
            &v6.server_artifact,
            TYPED_V5_MAGIC,
            TYPED_V5_PROFILE_VERSION,
        ),
        (
            &v5.server_artifact,
            TYPED_V6_MAGIC,
            TYPED_V6_PROFILE_VERSION,
        ),
    ] {
        assert!(matches!(
            ReferencePlayableArtifactView::load(
                &reprofile(bytes, magic, version),
                ReferenceArtifactProjection::ServerAuthoritative
            ),
            Err(ContentError::InvalidArtifact(
                "unexpected Reference artifact profile id"
            ))
        ));
    }

    // A v4 pair written before ITEM-SEM-2b-3 loads, stages and looks up under v4.
    let v4 = compile_with_profile(&linked, V4)?;
    let v4_server = ReferencePlayableArtifactView::load(
        &v4.server_artifact,
        ReferenceArtifactProjection::ServerAuthoritative,
    )?;
    assert_eq!(v4_server.artifact_profile_id(), TYPED_ARTIFACT_PROFILE_ID);
    let staged = NonAuthoritativeReferenceStage::stage(
        &v4.server_artifact,
        &v4.client_artifact,
        v4.expectation(),
    )?;
    let found = staged
        .server_artifact()
        .lookup_server_item(&identity)?
        .ok_or(ContentError::InvalidArtifact("v4 Item"))?;
    assert_eq!(found.semantics, linked_semantics(&linked, 0));
    assert_eq!(
        server
            .lookup_server_item(&identity)?
            .map(|item| item.semantics),
        Some(linked_semantics(&linked, 0))
    );

    // A reader taking the v7 bytes as v4 refuses them by profile id, not by a parse error.
    let as_v4 = reprofile(
        &compiled.server_artifact,
        TYPED_MAGIC,
        TYPED_PROFILE_VERSION,
    );
    assert!(matches!(
        ReferencePlayableArtifactView::load(
            &as_v4,
            ReferenceArtifactProjection::ServerAuthoritative
        ),
        Err(ContentError::InvalidArtifact(
            "unexpected Reference artifact profile id"
        ))
    ));
    let as_v5 = reprofile(
        &v4.server_artifact,
        TYPED_V5_MAGIC,
        TYPED_V5_PROFILE_VERSION,
    );
    assert!(matches!(
        ReferencePlayableArtifactView::load(
            &as_v5,
            ReferenceArtifactProjection::ServerAuthoritative
        ),
        Err(ContentError::InvalidArtifact(
            "unexpected Reference artifact profile id"
        ))
    ));

    // v5 grammar cannot be written as v4.
    let with_requirements = typed_family(ReferenceItemSemantics {
        use_requirements: Known(ReferenceItemUseRequirements {
            min_level: Known(55),
            min_magic_level: ReferenceItemField::Unknown,
            vocations: Known(vec![ReferenceBaseVocation::None]),
            enforcement_mode: ReferenceUseEnforcementMode::OnUse,
        }),
        ..presentation
    })?;
    assert!(compile_with_profile(&with_requirements, V4).is_err());
    let compiled = super::compile(&with_requirements)?;
    let staged = NonAuthoritativeReferenceStage::stage(
        &compiled.server_artifact,
        &compiled.client_artifact,
        compiled.expectation(),
    )?;
    let identity = with_requirements.definitions[0].definition.clone();
    let client = staged
        .client_artifact()
        .lookup_client_item(&identity)?
        .ok_or(ContentError::InvalidArtifact("v5 Item"))?;
    assert_eq!(
        client.semantics.use_requirements,
        linked_semantics(&with_requirements, 0).use_requirements
    );

    // Consumption is v6-only and server-only.
    let with_potion = typed_family(ReferenceItemSemantics {
        consumption: Known(potion(
            &[(ReferenceRestoreResource::Health, 125, 175)],
            ReferenceItemField::NotApplicable,
        )),
        ..Default::default()
    })?;
    assert!(compile_with_profile(&with_potion, V5).is_err());
    let compiled = super::compile(&with_potion)?;
    let staged = NonAuthoritativeReferenceStage::stage(
        &compiled.server_artifact,
        &compiled.client_artifact,
        compiled.expectation(),
    )?;
    let identity = with_potion.definitions[0].definition.clone();
    let server = staged
        .server_artifact()
        .lookup_server_item(&identity)?
        .ok_or(ContentError::InvalidArtifact("v6 Item"))?;
    assert_eq!(
        server.semantics.consumption,
        linked_semantics(&with_potion, 0).consumption
    );
    let client = staged
        .client_artifact()
        .lookup_client_item(&identity)?
        .ok_or(ContentError::InvalidArtifact("v6 Item"))?;
    assert_eq!(client.semantics.consumption, ReferenceItemField::Unknown);

    // A Known empty flask that does not resolve fails the load.
    let dangling = typed_family(ReferenceItemSemantics {
        consumption: Known(potion(
            &[(ReferenceRestoreResource::Mana, 75, 125)],
            Known(ReferenceItemTarget::new(
                "oteryn:item.schema.not-in-family",
                "definition-r1",
            )?),
        )),
        ..Default::default()
    })
    .and_then(|linked| Ok(super::compile(&linked)?));
    assert!(dangling.is_err());
    Ok(())
}
