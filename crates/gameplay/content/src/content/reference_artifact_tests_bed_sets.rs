/// A family whose first Item is a Head facing North with both occupied looks on the second
/// Item. The second Item is a bed part when `partner` names one, with its own occupied looks
/// on itself, so only the first Item's rule applies.
fn bed_pair_family(
    partner: Option<(ReferenceBedPart, ReferenceBedDirection)>,
) -> Result<CanonicalReferencePlayableContent, Box<dyn std::error::Error>> {
    let mut linked = typed_family(ReferenceItemSemantics::default())?;
    let partner_identity = linked.definitions[1].definition.clone();
    let occupied = ReferenceItemTarget::new(
        partner_identity.key().as_str(),
        partner_identity.revision().as_str(),
    )?;
    let first = bed(
        ReferenceBedPart::Head,
        ReferenceBedDirection::North,
        occupied.clone(),
        occupied.clone(),
    );
    let partner = match partner {
        Some((part, direction)) => {
            ReferenceItemField::Known(bed(part, direction, occupied.clone(), occupied))
        }
        None => ReferenceItemField::Unknown,
    };
    for (position, value) in [(0, ReferenceItemField::Known(first)), (1, partner)] {
        let ReferenceDefinitionKind::Item(item) = &mut linked.definitions[position].kind else {
            return Err(ContentError::InvalidArtifact("full family Item").into());
        };
        item.semantics.bed = value;
    }
    Ok(linked)
}

/// The server artifact of `source` without the compile-time bed set rule.
fn server_artifact_without_set_rule(
    source: &CanonicalReferencePlayableContent,
) -> Result<Vec<u8>, ContentError> {
    let records = source
        .definitions
        .iter()
        .map(|definition| match &definition.kind {
            ReferenceDefinitionKind::Item(item) => Ok(EncodedRecord {
                identity: &definition.definition,
                body: encode_server_item(item, V7, &source.definitions)?,
            }),
            _ => Err(ContentError::InvalidArtifact("typed Item")),
        })
        .collect::<Result<Vec<_>, ContentError>>()?;
    let metadata = ReferenceArtifactMetadata::from_source(
        source,
        ReferenceArtifactProjection::ServerAuthoritative,
        V7,
    )?;
    Ok(encode_artifact(&metadata, V7, &records)?.bytes)
}

fn assert_bed_set_refused(
    partner: Option<(ReferenceBedPart, ReferenceBedDirection)>,
    expected: &'static str,
) -> Result<(), Box<dyn std::error::Error>> {
    let linked = bed_pair_family(partner)?;
    // At compile.
    assert!(matches!(
        super::compile(&linked),
        Err(ContentError::InvalidArtifact(message)) if message == expected
    ));
    // At load: the bytes bypass the compile check, and the lookup refuses them.
    let bytes = server_artifact_without_set_rule(&linked)?;
    let server = ReferencePlayableArtifactView::load(
        &bytes,
        ReferenceArtifactProjection::ServerAuthoritative,
    )?;
    assert!(matches!(
        server.lookup_server_item(&linked.definitions[0].definition),
        Err(ContentError::InvalidArtifact(message)) if message == expected
    ));
    Ok(())
}

#[test]
fn bed_set_refuses_an_occupied_target_without_the_bed_group()
-> Result<(), Box<dyn std::error::Error>> {
    assert_bed_set_refused(None, "Reference Item bed occupied target is not a bed part")
}

#[test]
fn bed_set_refuses_an_occupied_target_with_another_part() -> Result<(), Box<dyn std::error::Error>>
{
    assert_bed_set_refused(
        Some((ReferenceBedPart::Foot, ReferenceBedDirection::North)),
        "Reference Item bed occupied target has another part",
    )
}

#[test]
fn bed_set_refuses_an_occupied_target_with_another_direction()
-> Result<(), Box<dyn std::error::Error>> {
    assert_bed_set_refused(
        Some((ReferenceBedPart::Head, ReferenceBedDirection::East)),
        "Reference Item bed occupied target has another partner direction",
    )
}

#[test]
fn bed_set_admits_a_matching_occupied_target_at_compile_and_load()
-> Result<(), Box<dyn std::error::Error>> {
    let linked = bed_pair_family(Some((ReferenceBedPart::Head, ReferenceBedDirection::North)))?;
    let compiled = super::compile(&linked)?;
    let staged = NonAuthoritativeReferenceStage::stage(
        &compiled.server_artifact,
        &compiled.client_artifact,
        compiled.expectation(),
    )?;
    for position in [0, 1] {
        let identity = &linked.definitions[position].definition;
        assert_eq!(
            staged
                .server_artifact()
                .lookup_server_item(identity)?
                .map(|item| item.semantics.bed),
            Some(linked_semantics(&linked, position).bed)
        );
        let client = staged
            .client_artifact()
            .lookup_client_item(identity)?
            .ok_or(ContentError::InvalidArtifact("v7 Item"))?;
        assert_eq!(client.semantics.bed, ReferenceItemField::Unknown);
    }
    Ok(())
}

fn linked_semantics(
    linked: &CanonicalReferencePlayableContent,
    position: usize,
) -> ReferenceItemSemantics {
    match &linked.definitions[position].kind {
        ReferenceDefinitionKind::Item(item) => item.semantics.clone(),
        _ => ReferenceItemSemantics::default(),
    }
}
