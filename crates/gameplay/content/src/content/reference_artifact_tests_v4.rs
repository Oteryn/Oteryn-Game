#[test]
fn typed_body_matches_accepted_exact_record_maxima_and_round_trips() -> Result<(), ContentError> {
    let item = worst_item()?;
    let definitions = worst_definitions(item.clone())?;
    let server = encode_typed_item(
        &item,
        ReferenceArtifactProjection::ServerAuthoritative,
        &definitions,
        V4,
    )?;
    let client = encode_typed_item(
        &item,
        ReferenceArtifactProjection::ClientSafe,
        &definitions,
        V4,
    )?;
    assert_eq!(server.len(), TYPED_REFERENCE_ITEM_MAX_SERVER_RECORD_BYTES);
    assert_eq!(client.len(), TYPED_REFERENCE_ITEM_MAX_CLIENT_RECORD_BYTES);
    assert_eq!(
        sha256(&server).as_slice(),
        hex_bytes("19c38b3cd18c7e2765aefb9ec2c7b0da9a31be66eaa30b84c63a7cf9ee132fc5")?
    );
    assert_eq!(
        sha256(&client).as_slice(),
        hex_bytes("0f9f25815a61fa292d6a74f89974f8b4bbefcca3d70a19ee023d730e88261e21")?
    );
    assert_eq!(
        encode_typed_item(
            &item,
            ReferenceArtifactProjection::ServerAuthoritative,
            &definitions,
            V4
        )?,
        server
    );
    let index = definitions
        .iter()
        .map(|definition| IndexEntry {
            identity: definition.definition.clone(),
            body_offset: 0,
            body_length: server.len(),
            body_digest: sha256(&server),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        parse_typed_server_item(V4, &server, &index)?.semantics,
        item.semantics
    );
    assert_eq!(
        parse_typed_client_item(V4, &client, &index)?.semantics,
        item.semantics.client_projection()
    );
    let mut over = server;
    over.push(0);
    assert!(matches!(
        parse_typed_server_item(V4, &over, &index),
        Err(ContentError::LimitExceeded {
            actual: 3_556,
            limit: 3_555,
            ..
        })
    ));
    let mut over = client;
    over.push(0);
    assert!(matches!(
        parse_typed_client_item(V4, &over, &index),
        Err(ContentError::LimitExceeded {
            actual: 3_434,
            limit: 3_433,
            ..
        })
    ));
    Ok(())
}

#[test]
fn typed_resource_limits_accept_exact_max_and_reject_max_plus_one() {
    let profile = ReferenceArtifactProfile::TypedItemV4;
    for (projection, body, artifact) in [
        (
            ReferenceArtifactProjection::ServerAuthoritative,
            TYPED_REFERENCE_ITEM_MAX_SERVER_BODY_BYTES,
            TYPED_REFERENCE_ITEM_MAX_SERVER_ARTIFACT_BYTES,
        ),
        (
            ReferenceArtifactProjection::ClientSafe,
            TYPED_REFERENCE_ITEM_MAX_CLIENT_BODY_BYTES,
            TYPED_REFERENCE_ITEM_MAX_CLIENT_ARTIFACT_BYTES,
        ),
    ] {
        assert!(check_body_section_length(profile, projection, body).is_ok());
        assert!(check_body_section_length(profile, projection, body + 1).is_err());
        assert!(check_artifact_length(profile, artifact, projection).is_ok());
        assert!(check_artifact_length(profile, artifact + 1, projection).is_err());
    }
    assert!(
        check_pair_lengths(
            profile,
            TYPED_REFERENCE_ITEM_MAX_SERVER_ARTIFACT_BYTES,
            TYPED_REFERENCE_ITEM_MAX_CLIENT_ARTIFACT_BYTES,
        )
        .is_ok()
    );
    assert!(
        check_pair_lengths(
            profile,
            TYPED_REFERENCE_ITEM_MAX_SERVER_ARTIFACT_BYTES + 1,
            TYPED_REFERENCE_ITEM_MAX_CLIENT_ARTIFACT_BYTES,
        )
        .is_err()
    );
}

#[test]
fn retained_core_wire_goldens_and_partial_unknown_guard_hold() -> Result<(), ContentError> {
    let definitions = [source_definition(ReferenceItemDefinition {
        physical_class: ReferenceItemPhysicalClass::Physical,
        materializable: true,
        stack_class: ReferenceItemStackClass::NonStackable,
        legal_destinations: vec![ReferenceItemDestination::CharacterInventory],
        semantics: Default::default(),
    })?];
    let ReferenceDefinitionKind::Item(materializable) = &definitions[0].kind else {
        unreachable!()
    };
    assert_eq!(
        encode_typed_item(
            materializable,
            ReferenceArtifactProjection::ServerAuthoritative,
            &definitions,
            V4
        )?,
        [2, 1, 1, 1, 1, 1, 0, 0],
    );
    assert_eq!(
        encode_typed_item(
            materializable,
            ReferenceArtifactProjection::ClientSafe,
            &definitions,
            V4
        )?,
        [2, 1, 1, 0, 0],
    );
    let identity_only = ReferenceItemDefinition {
        physical_class: ReferenceItemPhysicalClass::Unknown,
        materializable: false,
        stack_class: ReferenceItemStackClass::Unknown,
        legal_destinations: Vec::new(),
        semantics: Default::default(),
    };
    let identity_definitions = [source_definition(identity_only.clone())?];
    assert_eq!(
        encode_typed_item(
            &identity_only,
            ReferenceArtifactProjection::ServerAuthoritative,
            &identity_definitions,
            V4
        )?,
        hex_bytes("02020003000000")?,
    );
    assert_eq!(
        encode_typed_item(
            &identity_only,
            ReferenceArtifactProjection::ClientSafe,
            &identity_definitions,
            V4
        )?,
        hex_bytes("0202030000")?,
    );
    let partial = ReferenceItemDefinition {
        physical_class: ReferenceItemPhysicalClass::Unknown,
        materializable: false,
        stack_class: ReferenceItemStackClass::Unknown,
        legal_destinations: Vec::new(),
        semantics: ReferenceItemSemantics {
            physical: ReferenceItemField::Known(ReferenceItemPhysical {
                weight: ReferenceItemField::Known(123),
                movable: ReferenceItemField::Unknown,
                pickupable: ReferenceItemField::Unknown,
            }),
            ..Default::default()
        },
    };
    let partial_definitions = [source_definition(partial.clone())?];
    let server = encode_typed_item(
        &partial,
        ReferenceArtifactProjection::ServerAuthoritative,
        &partial_definitions,
        V4,
    )?;
    assert_eq!(server.len(), 18);
    let index = [IndexEntry {
        identity: identity()?,
        body_offset: 0,
        body_length: server.len(),
        body_digest: sha256(&server),
    }];
    let decoded = parse_typed_server_item(V4, &server, &index)?;
    assert!(!decoded.materializable);
    assert_eq!(decoded.semantics.physical, partial.semantics.physical);
    Ok(())
}

#[test]
fn typed_weapon_bytes_match_independent_python_oracle() -> Result<(), ContentError> {
    let fixtures = [
        (
            false,
            "0201010201010001060058030308030000002a0300000000030000000003000103000000000000000000000000000000010300000000000000000000000000000001010305010300000000020300000000030300000000040300000000050300000000",
            "0201020001060058030308030000002a0300000000030000000003000103000000000000000000000000000000010300000000000000000000000000000001010305010300000000020300000000030300000000040300000000050300000000",
            "c7ea4af176b3eebe80390f9a9ee26deb4cdc9f67695ef5e77390f2bc5c6c68b6",
            "f4396b9af4322ac0841fb9783aa51efb0a333d2d83113c943a4dc28526249114",
        ),
        (
            true,
            "0201010201010001060059030304030000002303000000000300000000030007030000000000000009000000000000000a030000000000000001000000000000000103010305010300000000020300000000030300000000040300000000050300000000",
            "0201020001060059030304030000002303000000000300000000030007030000000000000009000000000000000a030000000000000001000000000000000103010305010300000000020300000000030300000000040300000000050300000000",
            "d4649e3a70e117c485975174cc6c1f0977618a26a9d18bf01539d87be3d7c6a9",
            "8c1c2aaea9349e75be386299afc350a0d8ac0acb27bb44cf205a3f972f27ee11",
        ),
    ];
    for (distance, server_hex, client_hex, server_sha, client_sha) in fixtures {
        let item = oracle_weapon(distance)?;
        let definitions = [source_definition(item.clone())?];
        let server = encode_typed_item(
            &item,
            ReferenceArtifactProjection::ServerAuthoritative,
            &definitions,
            V4,
        )?;
        let client = encode_typed_item(
            &item,
            ReferenceArtifactProjection::ClientSafe,
            &definitions,
            V4,
        )?;
        assert_eq!(server, hex_bytes(server_hex)?);
        assert_eq!(client, hex_bytes(client_hex)?);
        assert_eq!(sha256(&server).as_slice(), hex_bytes(server_sha)?);
        assert_eq!(sha256(&client).as_slice(), hex_bytes(client_sha)?);
    }
    Ok(())
}

#[test]
fn typed_codec_rejects_noncanonical_and_projection_unsafe_shapes() -> Result<(), ContentError> {
    use ReferenceItemField::Known;
    assert!(ReferenceRationalPercent::new(2, 4).is_err());
    assert!(ReferenceRationalPercent::new(0, 2).is_err());
    assert!(ReferenceRationalPercent::new(1, 0).is_err());
    let mut noncanonical_rational = Vec::new();
    noncanonical_rational.extend_from_slice(&2_i64.to_be_bytes());
    noncanonical_rational.extend_from_slice(&4_u64.to_be_bytes());
    assert!(read_rational(&mut SliceReader::new(&noncanonical_rational)).is_err());
    assert!(ReferenceItemType::from_wire(u8::MAX).is_err());

    let mut item = worst_item()?;
    let ReferenceItemField::Known(stack) = &mut item.semantics.stack else {
        unreachable!()
    };
    stack.stackable = Known(false);
    assert!(crate::content::reference_playable::validate_item_definition(&item).is_err());

    let mut item = worst_item()?;
    let ReferenceItemField::Known(equipment) = &mut item.semantics.equipment else {
        unreachable!()
    };
    let ReferenceItemField::Known(patterns) = &mut equipment.patterns else {
        unreachable!()
    };
    let mut third = patterns[1].clone();
    third.pattern_id = 3;
    patterns.push(third);
    assert!(crate::content::reference_playable::validate_item_definition(&item).is_err());

    let mut item = worst_item()?;
    let ReferenceItemField::Known(imbuement) = &mut item.semantics.imbuement else {
        unreachable!()
    };
    imbuement.slot_count = Known(REFERENCE_ITEM_MAX_IMBUEMENT_SLOTS + 1);
    assert!(crate::content::reference_playable::validate_item_definition(&item).is_err());

    let identity = identity()?;
    let index = [IndexEntry {
        identity,
        body_offset: 0,
        body_length: 0,
        body_digest: [0; 32],
    }];
    for group in [
        ITEM_GROUP_TEMPORAL,
        ITEM_GROUP_USE_TRANSFORM,
        ITEM_GROUP_TRADE_RESTRICTIONS,
        ITEM_GROUP_FLUID,
        ITEM_GROUP_READABLE_WRITEABLE,
    ] {
        let client_with_server_group = [2, 1, 2, 0, 1, group, 0, 1, 0];
        assert!(parse_typed_client_item(V4, &client_with_server_group, &index).is_err());
    }
    let duplicate_group = [2, 1, 2, 0, 2, 1, 0, 1, 0, 1, 0, 1, 0];
    assert!(parse_typed_client_item(V4, &duplicate_group, &index).is_err());
    let stack_contradiction = [2, 1, 2, 0, 1, ITEM_GROUP_STACK, 0, 4, 3, 3, 0, 0];
    assert!(parse_typed_client_item(V4, &stack_contradiction, &index).is_err());
    Ok(())
}
