fn canonical_b1_vase_content()
-> Result<CanonicalReferencePlayableContent, Box<dyn std::error::Error>> {
    let imported = protected_cw2_b1_vase_import(B1_EVIDENCE)?;
    let limits = reference_project_limits();
    Ok(CanonicalProjectDocuments::from_draft(
        ProjectDraft {
            project_revision: "project-r1".to_owned(),
            package_key: "oteryn:content.world-project".to_owned(),
            semantic_schema_version: "reference-schema-v1".to_owned(),
            licensing_metadata: "PENDING".to_owned(),
            world_id: "0123456789ab70cd8ef0123456789abc".to_owned(),
            coordinate_frame: "global-target-2026-09-27".to_owned(),
            records: vec![imported.record],
            imports: vec![imported.batch],
            metadata: Vec::new(),
        },
        limits,
    )?
    .into_snapshot(limits)?
    .parse(limits)?
    .link()?)
}

#[test]
fn protected_b1_vase_stage_binds_exact_runtime_item_view_without_controller_authority()
-> Result<(), Box<dyn std::error::Error>> {
    let linked = canonical_b1_vase_content()?;
    let compiled = compile_reference_playable(&linked)?;
    let staged = stage_reference_playable(
        &compiled.server_artifact,
        &compiled.client_artifact,
        compiled.expectation(),
    )?;
    let exact_generation = staged.identity().clone();
    let exact_item = exact_generation.item_identity().clone();
    let channel = ChannelId::decode(&uuid_v7(91))?;
    let scope = RuntimeScopeRefV1::channel(linked.world_id, channel);
    let controller = ContentActivationController::new();

    let view = NonAuthoritativeReferenceItemRuntimeView::bind(staged, scope)?;

    assert_eq!(view.scope(), scope);
    assert_eq!(view.generation_identity(), &exact_generation);
    assert_eq!(view.item_identity(), &exact_item);
    assert_eq!(view.item_identity().family(), DefinitionFamily::Item);
    assert_eq!(view.item_identity().key().as_str(), CW2_B1_VASE_KEY);
    assert_eq!(
        view.item_identity().revision().as_str(),
        CW2_B1_VASE_REVISION
    );
    assert_eq!(view.server_artifact_digest(), compiled.server_digest());
    assert_eq!(view.client_artifact_digest(), compiled.client_digest());

    let item = view
        .lookup_item(&exact_item)
        .ok_or(fixture_error("runtime Item lookup"))?;
    assert_eq!(item.physical_class, ReferenceItemPhysicalClass::Physical);
    assert!(item.materializable);
    assert_eq!(item.stack_class, ReferenceItemStackClass::NonStackable);
    assert_eq!(
        item.legal_destinations,
        [ReferenceItemDestination::CharacterInventory]
    );

    let unknown = TypedDefinitionRef::new(
        DefinitionFamily::Item,
        ProductionKey::new("oteryn:item.decor.unknown")?,
        DefinitionRevisionRef::new("definition-r1")?,
    );
    assert_eq!(view.lookup_item(&unknown), None);
    assert!(controller.staged_identity().is_none());
    assert!(controller.active().is_none());
    assert!(!controller.is_ready());
    Ok(())
}

#[test]
fn reference_item_runtime_view_rejects_a_different_world() -> Result<(), Box<dyn std::error::Error>>
{
    let linked = canonical_b1_vase_content()?;
    let compiled = compile_reference_playable(&linked)?;
    let staged = stage_reference_playable(
        &compiled.server_artifact,
        &compiled.client_artifact,
        compiled.expectation(),
    )?;
    let other_world = decode_world(99)?;
    assert_ne!(other_world, linked.world_id);
    let channel = ChannelId::decode(&uuid_v7(92))?;

    assert!(matches!(
        NonAuthoritativeReferenceItemRuntimeView::bind(
            staged,
            RuntimeScopeRefV1::channel(other_world, channel),
        ),
        Err(WorldRuntimeError::InvalidBinding(
            "Reference Item generation world does not match runtime scope"
        ))
    ));
    Ok(())
}
