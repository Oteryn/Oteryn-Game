use super::*;
use crate::content::{
    CW2_B1_NATIVE_ITEM_BATCH_COUNT, CanonicalProjectDocuments, ProjectDraft, ProjectEvidenceLimits,
    protected_cw2_b1_native_item_batch_import,
};

const B1_EVIDENCE: &[u8] = include_bytes!(
    "../../../../../docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json"
);

fn batch_limits() -> ProjectEvidenceLimits {
    ProjectEvidenceLimits {
        max_documents: 8,
        max_document_bytes: 2_097_152,
        max_total_bytes: 4_194_304,
        max_json_depth: 24,
        max_decoded_fields: 32_768,
        max_string_bytes: 2_097_152,
        max_locator_bytes: 160,
        max_locator_segments: 8,
        max_reference_records: CW2_B1_NATIVE_ITEM_BATCH_COUNT,
        max_import_records: CW2_B1_NATIVE_ITEM_BATCH_COUNT,
        max_reimport_states: CW2_B1_NATIVE_ITEM_BATCH_COUNT,
    }
}

fn linked_batch() -> Result<CanonicalReferencePlayableContent, Box<dyn std::error::Error>> {
    let imported = protected_cw2_b1_native_item_batch_import(B1_EVIDENCE)?;
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
        batch_limits(),
    )?;
    Ok(canonical
        .into_snapshot(batch_limits())?
        .parse(batch_limits())?
        .link()?)
}

#[test]
fn frozen_native_item_batch_compiles_loads_and_stages_as_v2()
-> Result<(), Box<dyn std::error::Error>> {
    let linked = linked_batch()?;
    assert_eq!(linked.definitions.len(), BATCH_MAX_INDEX_ENTRIES);

    let first = compile(&linked)?;
    let repeated = compile(&linked)?;
    assert_eq!(first.server_artifact, repeated.server_artifact);
    assert_eq!(first.client_artifact, repeated.client_artifact);

    let server = ReferencePlayableArtifactView::load(
        &first.server_artifact,
        ReferenceArtifactProjection::ServerAuthoritative,
    )?;
    let client = ReferencePlayableArtifactView::load(
        &first.client_artifact,
        ReferenceArtifactProjection::ClientSafe,
    )?;
    assert_eq!(server.artifact_profile_id(), BATCH_ARTIFACT_PROFILE_ID);
    assert_eq!(client.artifact_profile_id(), BATCH_ARTIFACT_PROFILE_ID);
    assert_eq!(
        server.indexed_identity_count(),
        CW2_B1_NATIVE_ITEM_BATCH_COUNT
    );
    assert_eq!(
        client.indexed_identity_count(),
        CW2_B1_NATIVE_ITEM_BATCH_COUNT
    );
    assert_eq!(
        server.indexed_identities().cloned().collect::<Vec<_>>(),
        linked
            .definitions
            .iter()
            .map(|definition| definition.definition.clone())
            .collect::<Vec<_>>()
    );

    for definition in &linked.definitions {
        let authoritative = server
            .lookup_server_item(&definition.definition)?
            .ok_or(ContentError::InvalidArtifact("authoritative batch Item"))?;
        let projected = client
            .lookup_client_item(&definition.definition)?
            .ok_or(ContentError::InvalidArtifact("client batch Item"))?;
        assert_eq!(authoritative.physical_class, projected.physical_class);
        assert_eq!(authoritative.stack_class, projected.stack_class);
    }

    let staged = NonAuthoritativeReferenceStage::stage(
        &first.server_artifact,
        &first.client_artifact,
        first.expectation(),
    )?;
    assert_eq!(
        staged.identity().item_identities().len(),
        CW2_B1_NATIVE_ITEM_BATCH_COUNT
    );
    assert_eq!(
        staged.identity().item_identity(),
        &linked.definitions[0].definition
    );
    Ok(())
}

#[test]
fn successor_profiles_preserve_v2_bound_and_select_v3_for_the_full_family() {
    assert!(
        !ReferenceArtifactProfile::NativeItemBatchV2
            .accepts_item_count(BATCH_MAX_INDEX_ENTRIES + 1)
    );
    assert!(matches!(
        ReferenceArtifactProfile::for_definition_count(BATCH_MAX_INDEX_ENTRIES + 1),
        Ok(ReferenceArtifactProfile::NativeItemFamilyV3)
    ));
    assert!(
        ReferenceArtifactProfile::NativeItemFamilyV3.accepts_item_count(FAMILY_MAX_INDEX_ENTRIES)
    );
    assert!(matches!(
        ReferenceArtifactProfile::for_definition_count(FAMILY_MAX_INDEX_ENTRIES + 1),
        Err(ContentError::LimitExceeded {
            resource: "Reference playable definitions",
            actual,
            limit: FAMILY_MAX_INDEX_ENTRIES,
        }) if actual == FAMILY_MAX_INDEX_ENTRIES + 1
    ));
}

#[test]
fn compiler_rejects_duplicate_item_key_across_revisions() -> Result<(), Box<dyn std::error::Error>>
{
    let mut linked = linked_batch()?;
    let duplicate_key = linked.definitions[0].definition.key().clone();
    linked.definitions[1].definition = TypedDefinitionRef::new(
        DefinitionFamily::Item,
        duplicate_key.clone(),
        DefinitionRevisionRef::new("definition-r2")?,
    );

    assert!(matches!(
        compile(&linked),
        Err(ContentError::DuplicateKey(key)) if key == duplicate_key.as_str()
    ));
    Ok(())
}

// Include topic files in this module to preserve test paths and private fixture access.
