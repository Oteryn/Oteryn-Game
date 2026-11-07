const V5: ReferenceArtifactProfile = ReferenceArtifactProfile::TypedItemV5;

fn all_vocations() -> Result<Vec<ReferenceBaseVocation>, ContentError> {
    (1..=6).map(ReferenceBaseVocation::from_wire).collect()
}

fn worst_item_v5() -> Result<ReferenceItemDefinition, ContentError> {
    use ReferenceItemField::Known;
    let mut item = worst_item()?;
    let Known(equipment) = &mut item.semantics.equipment else {
        unreachable!()
    };
    let Known(patterns) = &mut equipment.patterns else {
        unreachable!()
    };
    for pattern in patterns.iter_mut() {
        pattern.vocations = Known(all_vocations()?);
    }
    let Known(trade) = &mut item.semantics.trade_restrictions else {
        unreachable!()
    };
    trade.vocations = Known(all_vocations()?);
    item.semantics.use_requirements = Known(ReferenceItemUseRequirements {
        min_level: Known(0),
        min_magic_level: Known(0),
        vocations: Known(all_vocations()?),
        enforcement_mode: ReferenceUseEnforcementMode::OnUse,
    });
    Ok(item)
}

fn plain_item(semantics: ReferenceItemSemantics) -> ReferenceItemDefinition {
    ReferenceItemDefinition {
        physical_class: ReferenceItemPhysicalClass::Physical,
        materializable: true,
        stack_class: ReferenceItemStackClass::NonStackable,
        legal_destinations: vec![ReferenceItemDestination::CharacterInventory],
        semantics,
    }
}

fn one_index(record: &[u8]) -> Result<[IndexEntry; 1], ContentError> {
    Ok([IndexEntry {
        identity: identity()?,
        body_offset: 0,
        body_length: record.len(),
        body_digest: sha256(record),
    }])
}

#[test]
fn typed_v5_body_matches_registered_record_maxima_and_round_trips() -> Result<(), ContentError> {
    let item = worst_item_v5()?;
    let definitions = worst_definitions(item.clone())?;
    let server = encode_typed_item(
        &item,
        ReferenceArtifactProjection::ServerAuthoritative,
        &definitions,
        V5,
    )?;
    let client = encode_typed_item(
        &item,
        ReferenceArtifactProjection::ClientSafe,
        &definitions,
        V5,
    )?;
    assert_eq!(
        server.len(),
        TYPED_V5_REFERENCE_ITEM_MAX_SERVER_RECORD_BYTES
    );
    assert_eq!(
        client.len(),
        TYPED_V5_REFERENCE_ITEM_MAX_CLIENT_RECORD_BYTES
    );
    // The same worst shape as the resource-profile tool's v5 evidence.
    assert_eq!(
        sha256(&server).as_slice(),
        hex_bytes("779f3a027f8ccaa9da8bd0e0236f9b0f76da8322985904122daa18e53c67b47e")?
    );
    assert_eq!(
        sha256(&client).as_slice(),
        hex_bytes("c99b352169d7d5c88dd6215d971640f9aedd2cea2b0b19d3048bcbc56718383e")?
    );
    // 17 server groups and 12 client groups: the registered exact maxima.
    assert_eq!(server[6..8], 17_u16.to_be_bytes());
    assert_eq!(client[3..5], 12_u16.to_be_bytes());
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
        parse_typed_server_item(V5, &server, &index)?.semantics,
        item.semantics
    );
    assert_eq!(
        parse_typed_client_item(V5, &client, &index)?.semantics,
        item.semantics.client_projection()
    );
    let mut over = server.clone();
    over.push(0);
    assert!(matches!(
        parse_typed_server_item(V5, &over, &index),
        Err(ContentError::LimitExceeded {
            actual: 3_578,
            limit: 3_577,
            ..
        })
    ));
    let mut over = client.clone();
    over.push(0);
    assert!(matches!(
        parse_typed_client_item(V5, &over, &index),
        Err(ContentError::LimitExceeded {
            actual: 3_455,
            limit: 3_454,
            ..
        })
    ));
    let mut groups = server.clone();
    groups[6..8].copy_from_slice(&18_u16.to_be_bytes());
    assert!(matches!(
        parse_typed_server_item(V5, &groups, &index),
        Err(ContentError::LimitExceeded {
            resource: "Reference Item groups",
            actual: 18,
            limit: 17,
        })
    ));
    let mut groups = client;
    groups[3..5].copy_from_slice(&13_u16.to_be_bytes());
    assert!(matches!(
        parse_typed_client_item(V5, &groups, &index),
        Err(ContentError::LimitExceeded {
            resource: "Reference Item groups",
            actual: 13,
            limit: 12,
        })
    ));
    // Seven vocations exceed the six-value domain on encode.
    let mut too_many = item;
    let ReferenceItemField::Known(requirements) = &mut too_many.semantics.use_requirements else {
        unreachable!()
    };
    let mut seven = all_vocations()?;
    seven.push(ReferenceBaseVocation::Druid);
    requirements.vocations = ReferenceItemField::Known(seven);
    assert!(
        encode_typed_item(
            &too_many,
            ReferenceArtifactProjection::ServerAuthoritative,
            &definitions,
            V5
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn typed_v5_resource_limits_accept_exact_max_and_reject_max_plus_one() {
    for (projection, body, artifact) in [
        (
            ReferenceArtifactProjection::ServerAuthoritative,
            TYPED_V5_REFERENCE_ITEM_MAX_SERVER_BODY_BYTES,
            TYPED_V5_REFERENCE_ITEM_MAX_SERVER_ARTIFACT_BYTES,
        ),
        (
            ReferenceArtifactProjection::ClientSafe,
            TYPED_V5_REFERENCE_ITEM_MAX_CLIENT_BODY_BYTES,
            TYPED_V5_REFERENCE_ITEM_MAX_CLIENT_ARTIFACT_BYTES,
        ),
    ] {
        assert!(check_body_section_length(V5, projection, body).is_ok());
        assert!(check_body_section_length(V5, projection, body + 1).is_err());
        assert!(check_artifact_length(V5, artifact, projection).is_ok());
        assert!(check_artifact_length(V5, artifact + 1, projection).is_err());
        assert!(check_body_record_length(V5, projection, V5.body_record_limit(projection)).is_ok());
        assert!(
            check_body_record_length(V5, projection, V5.body_record_limit(projection) + 1).is_err()
        );
    }
    assert_eq!(
        TYPED_V5_REFERENCE_ITEM_MAX_SERVER_BODY_BYTES,
        FAMILY_MAX_INDEX_ENTRIES * TYPED_V5_REFERENCE_ITEM_MAX_SERVER_RECORD_BYTES
    );
    assert_eq!(
        TYPED_V5_REFERENCE_ITEM_MAX_CLIENT_BODY_BYTES,
        FAMILY_MAX_INDEX_ENTRIES * TYPED_V5_REFERENCE_ITEM_MAX_CLIENT_RECORD_BYTES
    );
    assert_eq!(
        TYPED_V5_REFERENCE_ITEM_MAX_GENERATION_PAIR_BYTES,
        TYPED_V5_REFERENCE_ITEM_MAX_SERVER_ARTIFACT_BYTES
            + TYPED_V5_REFERENCE_ITEM_MAX_CLIENT_ARTIFACT_BYTES
    );
    assert!(
        check_pair_lengths(
            V5,
            TYPED_V5_REFERENCE_ITEM_MAX_SERVER_ARTIFACT_BYTES,
            TYPED_V5_REFERENCE_ITEM_MAX_CLIENT_ARTIFACT_BYTES,
        )
        .is_ok()
    );
    assert!(
        check_pair_lengths(
            V5,
            TYPED_V5_REFERENCE_ITEM_MAX_SERVER_ARTIFACT_BYTES + 1,
            TYPED_V5_REFERENCE_ITEM_MAX_CLIENT_ARTIFACT_BYTES,
        )
        .is_err()
    );
    // v4 keeps its own ceilings for decoding.
    assert_eq!(
        V4.body_record_limit(ReferenceArtifactProjection::ServerAuthoritative),
        TYPED_REFERENCE_ITEM_MAX_SERVER_RECORD_BYTES
    );
    assert_eq!(
        V4.max_groups(ReferenceArtifactProjection::ServerAuthoritative),
        16
    );
    assert_eq!(V4.max_groups(ReferenceArtifactProjection::ClientSafe), 11);
    assert_eq!(V4.max_vocations(), 5);
    assert_eq!(V5.max_vocations(), 6);
}

#[test]
fn use_requirements_round_trip_with_each_field_present_and_absent() -> Result<(), ContentError> {
    use ReferenceItemField::{Known, Unknown};
    let mut cases = Vec::new();
    for mask in 0..8_u8 {
        cases.push(ReferenceItemUseRequirements {
            min_level: if mask & 1 == 0 { Unknown } else { Known(55) },
            min_magic_level: if mask & 2 == 0 { Unknown } else { Known(4) },
            vocations: if mask & 4 == 0 {
                Unknown
            } else {
                Known(vec![
                    ReferenceBaseVocation::Paladin,
                    ReferenceBaseVocation::None,
                ])
            },
            enforcement_mode: ReferenceUseEnforcementMode::OnUse,
        });
    }
    for requirements in cases {
        let item = plain_item(ReferenceItemSemantics {
            use_requirements: Known(requirements),
            ..Default::default()
        });
        let definitions = [source_definition(item.clone())?];
        for projection in [
            ReferenceArtifactProjection::ServerAuthoritative,
            ReferenceArtifactProjection::ClientSafe,
        ] {
            let record = encode_typed_item(&item, projection, &definitions, V5)?;
            assert_eq!(record[0], TYPED_V5_BODY_RECORD_VERSION);
            let index = one_index(&record)?;
            let decoded = match projection {
                ReferenceArtifactProjection::ServerAuthoritative => {
                    parse_typed_server_item(V5, &record, &index)?.semantics
                }
                ReferenceArtifactProjection::ClientSafe => {
                    parse_typed_client_item(V5, &record, &index)?.semantics
                }
            };
            assert_eq!(decoded, item.semantics);
        }
    }
    // Client record: version 3, Physical, NonStackable, one group 17 with every field
    // Unknown, then the enforcement mode. Only ON_USE (1) is admitted.
    let index = one_index(&[])?;
    let record = |group: u8, mode: u8| [3, 1, 1, 0, 1, group, 0, 5, 3, 0, 0, 0, mode];
    assert!(parse_typed_client_item(V5, &record(17, 1), &index).is_ok());
    for mode in [0, 2, u8::MAX] {
        assert!(matches!(
            parse_typed_client_item(V5, &record(17, mode), &index),
            Err(ContentError::InvalidArtifact(
                "unknown ReferenceUseEnforcementMode"
            ))
        ));
    }
    // An unknown group id still fails closed.
    assert!(parse_typed_client_item(V5, &record(18, 1), &index).is_err());
    assert!(parse_typed_server_item(V5, &[3, 1, 1, 1, 1, 1, 0, 1, 18, 0, 1, 0], &index).is_err());
    Ok(())
}

#[test]
fn vocation_none_and_use_requirements_exist_only_in_v5() -> Result<(), ContentError> {
    use ReferenceItemField::{Known, Unknown};
    let index = one_index(&[])?;
    // Client record with one Equipment pattern whose vocation set is [wire value].
    let equipment = |version: u8, vocation: u8| {
        [
            version, 1, 1, 0, 1, 5, 0, 12, 3, 3, 1, 1, 0, 0, 0, 3, 1, vocation, 0, 0,
        ]
    };
    let decoded = parse_typed_client_item(V5, &equipment(3, 6), &index)?;
    let Known(value) = decoded.semantics.equipment else {
        unreachable!()
    };
    let Known(patterns) = value.patterns else {
        unreachable!()
    };
    assert_eq!(
        patterns[0].vocations,
        Known(vec![ReferenceBaseVocation::None])
    );
    assert!(parse_typed_client_item(V5, &equipment(3, 7), &index).is_err());
    assert!(parse_typed_client_item(V4, &equipment(2, 5), &index).is_ok());
    assert!(matches!(
        parse_typed_client_item(V4, &equipment(2, 6), &index),
        Err(ContentError::InvalidArtifact(
            "unknown ReferenceBaseVocation"
        ))
    ));
    // A v5 body version is not a v4 body and the reverse.
    assert!(parse_typed_client_item(V4, &equipment(3, 5), &index).is_err());
    assert!(parse_typed_client_item(V5, &equipment(2, 5), &index).is_err());
    // v4 refuses group 17 by group id.
    let group_17 = [2, 1, 1, 0, 1, 17, 0, 1, 0];
    assert!(matches!(
        parse_typed_client_item(V4, &group_17, &index),
        Err(ContentError::InvalidArtifact(
            "Reference Item group is unknown, duplicated, unordered or excluded from projection"
        ))
    ));
    // The v4 encoder refuses both additions; v5 writes them.
    let none_vocation = plain_item(ReferenceItemSemantics {
        trade_restrictions: Known(ReferenceItemTradeRestrictions {
            tradeable: Unknown,
            marketable: Unknown,
            vocations: Known(vec![ReferenceBaseVocation::None]),
            account_binding_policy: Unknown,
            character_binding_policy: Unknown,
        }),
        ..Default::default()
    });
    let requirements = plain_item(ReferenceItemSemantics {
        use_requirements: Known(ReferenceItemUseRequirements {
            min_level: Known(20),
            min_magic_level: Unknown,
            vocations: Unknown,
            enforcement_mode: ReferenceUseEnforcementMode::OnUse,
        }),
        ..Default::default()
    });
    for item in [none_vocation, requirements] {
        let definitions = [source_definition(item.clone())?];
        assert!(
            encode_typed_item(
                &item,
                ReferenceArtifactProjection::ServerAuthoritative,
                &definitions,
                V4
            )
            .is_err()
        );
        assert!(
            encode_typed_item(
                &item,
                ReferenceArtifactProjection::ServerAuthoritative,
                &definitions,
                V5
            )
            .is_ok()
        );
    }
    // v5 admits six vocations; seven fail validation before encoding.
    let mut seven = all_vocations()?;
    seven.push(ReferenceBaseVocation::Knight);
    let invalid = plain_item(ReferenceItemSemantics {
        use_requirements: Known(ReferenceItemUseRequirements {
            min_level: Unknown,
            min_magic_level: Unknown,
            vocations: Known(seven),
            enforcement_mode: ReferenceUseEnforcementMode::OnUse,
        }),
        ..Default::default()
    });
    assert!(crate::content::reference_playable::validate_item_definition(&invalid).is_err());
    Ok(())
}
