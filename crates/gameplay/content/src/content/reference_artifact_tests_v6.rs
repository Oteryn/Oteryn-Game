const V6: ReferenceArtifactProfile = ReferenceArtifactProfile::TypedItemV6;

fn potion(
    restores: &[(ReferenceRestoreResource, u16, u16)],
    empty_flask: ReferenceItemField<ReferenceItemTarget>,
) -> ReferenceItemConsumption {
    ReferenceItemConsumption::Potion(ReferenceItemPotion {
        restores: restores
            .iter()
            .map(|&(resource, min, max)| ReferencePotionRestore { resource, min, max })
            .collect(),
        empty_flask,
    })
}

fn food(regeneration_seconds: u16) -> ReferenceItemConsumption {
    ReferenceItemConsumption::Food(ReferenceItemFood {
        regeneration_seconds,
    })
}

fn worst_item_v6() -> Result<ReferenceItemDefinition, ContentError> {
    let mut item = worst_item_v5()?;
    item.semantics.consumption = ReferenceItemField::Known(potion(
        &[
            (ReferenceRestoreResource::Health, 10_000, 10_000),
            (ReferenceRestoreResource::Mana, 10_000, 10_000),
        ],
        ReferenceItemField::Known(target()?),
    ));
    Ok(item)
}

fn consumption_item(consumption: ReferenceItemConsumption) -> ReferenceItemDefinition {
    plain_item(ReferenceItemSemantics {
        consumption: ReferenceItemField::Known(consumption),
        ..Default::default()
    })
}

fn server_round_trip(
    item: &ReferenceItemDefinition,
) -> Result<(Vec<u8>, ReferenceItemSemantics), ContentError> {
    let definitions = [source_definition(item.clone())?];
    let record = encode_typed_item(
        item,
        ReferenceArtifactProjection::ServerAuthoritative,
        &definitions,
        V6,
    )?;
    let decoded = parse_typed_server_item(V6, &record, &one_index(&record)?)?.semantics;
    Ok((record, decoded))
}

#[test]
fn typed_v6_body_matches_registered_record_maxima_and_round_trips() -> Result<(), ContentError> {
    let item = worst_item_v6()?;
    let definitions = worst_definitions(item.clone())?;
    let server = encode_typed_item(
        &item,
        ReferenceArtifactProjection::ServerAuthoritative,
        &definitions,
        V6,
    )?;
    let client = encode_typed_item(
        &item,
        ReferenceArtifactProjection::ClientSafe,
        &definitions,
        V6,
    )?;
    assert_eq!(
        server.len(),
        TYPED_V6_REFERENCE_ITEM_MAX_SERVER_RECORD_BYTES
    );
    assert_eq!(
        client.len(),
        TYPED_V6_REFERENCE_ITEM_MAX_CLIENT_RECORD_BYTES
    );
    // The same worst shape as the resource-profile tool's v6 evidence.
    assert_eq!(
        sha256(&server).as_slice(),
        hex_bytes("2181e6e62d70b2a01069a696c40c4324ad06c7a5c771362f47c599072b0cfe35")?
    );
    assert_eq!(
        sha256(&client).as_slice(),
        hex_bytes("ad6bef56fd078582975ed1d5818cca8b8ae3cace0adff696eb960502991b899f")?
    );
    // 18 server groups and 12 client groups: the registered exact maxima.
    assert_eq!(server[6..8], 18_u16.to_be_bytes());
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
        parse_typed_server_item(V6, &server, &index)?.semantics,
        item.semantics
    );
    assert_eq!(
        parse_typed_client_item(V6, &client, &index)?.semantics,
        item.semantics.client_projection()
    );
    let mut over = server.clone();
    over.push(0);
    assert!(matches!(
        parse_typed_server_item(V6, &over, &index),
        Err(ContentError::LimitExceeded {
            actual: 3_599,
            limit: 3_598,
            ..
        })
    ));
    let mut over = client.clone();
    over.push(0);
    assert!(matches!(
        parse_typed_client_item(V6, &over, &index),
        Err(ContentError::LimitExceeded {
            actual: 3_455,
            limit: 3_454,
            ..
        })
    ));
    let mut groups = server;
    groups[6..8].copy_from_slice(&19_u16.to_be_bytes());
    assert!(matches!(
        parse_typed_server_item(V6, &groups, &index),
        Err(ContentError::LimitExceeded {
            resource: "Reference Item groups",
            actual: 19,
            limit: 18,
        })
    ));
    let mut groups = client;
    groups[3..5].copy_from_slice(&13_u16.to_be_bytes());
    assert!(matches!(
        parse_typed_client_item(V6, &groups, &index),
        Err(ContentError::LimitExceeded {
            resource: "Reference Item groups",
            actual: 13,
            limit: 12,
        })
    ));
    Ok(())
}

#[test]
fn typed_v6_resource_limits_accept_exact_max_and_reject_max_plus_one() {
    for (projection, record, body, artifact) in [
        (
            ReferenceArtifactProjection::ServerAuthoritative,
            TYPED_V6_REFERENCE_ITEM_MAX_SERVER_RECORD_BYTES,
            TYPED_V6_REFERENCE_ITEM_MAX_SERVER_BODY_BYTES,
            TYPED_V6_REFERENCE_ITEM_MAX_SERVER_ARTIFACT_BYTES,
        ),
        (
            ReferenceArtifactProjection::ClientSafe,
            TYPED_V6_REFERENCE_ITEM_MAX_CLIENT_RECORD_BYTES,
            TYPED_V6_REFERENCE_ITEM_MAX_CLIENT_BODY_BYTES,
            TYPED_V6_REFERENCE_ITEM_MAX_CLIENT_ARTIFACT_BYTES,
        ),
    ] {
        assert_eq!(V6.body_record_limit(projection), record);
        assert!(check_body_record_length(V6, projection, record).is_ok());
        assert!(check_body_record_length(V6, projection, record + 1).is_err());
        assert!(check_body_section_length(V6, projection, body).is_ok());
        assert!(check_body_section_length(V6, projection, body + 1).is_err());
        assert!(check_artifact_length(V6, artifact, projection).is_ok());
        assert!(check_artifact_length(V6, artifact + 1, projection).is_err());
    }
    assert_eq!(
        TYPED_V6_REFERENCE_ITEM_MAX_SERVER_BODY_BYTES,
        FAMILY_MAX_INDEX_ENTRIES * TYPED_V6_REFERENCE_ITEM_MAX_SERVER_RECORD_BYTES
    );
    assert_eq!(
        TYPED_V6_REFERENCE_ITEM_MAX_CLIENT_BODY_BYTES,
        FAMILY_MAX_INDEX_ENTRIES * TYPED_V6_REFERENCE_ITEM_MAX_CLIENT_RECORD_BYTES
    );
    assert_eq!(
        TYPED_V6_REFERENCE_ITEM_MAX_GENERATION_PAIR_BYTES,
        TYPED_V6_REFERENCE_ITEM_MAX_SERVER_ARTIFACT_BYTES
            + TYPED_V6_REFERENCE_ITEM_MAX_CLIENT_ARTIFACT_BYTES
    );
    assert!(
        check_pair_lengths(
            V6,
            TYPED_V6_REFERENCE_ITEM_MAX_SERVER_ARTIFACT_BYTES,
            TYPED_V6_REFERENCE_ITEM_MAX_CLIENT_ARTIFACT_BYTES,
        )
        .is_ok()
    );
    assert!(
        check_pair_lengths(
            V6,
            TYPED_V6_REFERENCE_ITEM_MAX_SERVER_ARTIFACT_BYTES,
            TYPED_V6_REFERENCE_ITEM_MAX_CLIENT_ARTIFACT_BYTES + 1,
        )
        .is_err()
    );
    assert_eq!(
        V6.max_groups(ReferenceArtifactProjection::ServerAuthoritative),
        18
    );
    assert_eq!(V6.max_groups(ReferenceArtifactProjection::ClientSafe), 12);
    assert_eq!(
        V5.max_groups(ReferenceArtifactProjection::ServerAuthoritative),
        17
    );
    assert_eq!(V6.max_vocations(), 6);
    // v5 keeps its own ceilings for decoding.
    assert_eq!(
        V5.body_record_limit(ReferenceArtifactProjection::ServerAuthoritative),
        TYPED_V5_REFERENCE_ITEM_MAX_SERVER_RECORD_BYTES
    );
}

#[test]
fn food_round_trips_at_bounds_and_rejects_outside_them() -> Result<(), ContentError> {
    for seconds in [1, 600, REFERENCE_ITEM_MAX_FOOD_REGENERATION_SECONDS] {
        let item = consumption_item(food(seconds));
        let (record, decoded) = server_round_trip(&item)?;
        assert_eq!(record[0], TYPED_V6_BODY_RECORD_VERSION);
        assert_eq!(decoded, item.semantics);
        // The client projection carries no consumption group.
        let definitions = [source_definition(item.clone())?];
        let client = encode_typed_item(
            &item,
            ReferenceArtifactProjection::ClientSafe,
            &definitions,
            V6,
        )?;
        let decoded = parse_typed_client_item(V6, &client, &one_index(&client)?)?.semantics;
        assert_eq!(decoded.consumption, ReferenceItemField::Unknown);
    }
    for seconds in [0, 1_200, u16::MAX] {
        let item = consumption_item(food(seconds));
        assert!(crate::content::reference_playable::validate_item_definition(&item).is_err());
    }
    // The decoder validates: a Food payload of 0 or 1,200 seconds fails the record.
    let (record, _) = server_round_trip(&consumption_item(food(1)))?;
    let end = record.len();
    assert_eq!(record[end - 4..end - 2], [3, 1]);
    for seconds in [0_u16, 1_200] {
        let mut patched = record.clone();
        patched[end - 2..].copy_from_slice(&seconds.to_be_bytes());
        assert!(parse_typed_server_item(V6, &patched, &one_index(&patched)?).is_err());
    }
    // An unknown variant fails closed.
    for variant in [0, 3, u8::MAX] {
        let mut patched = record.clone();
        patched[end - 3] = variant;
        assert!(matches!(
            parse_typed_server_item(V6, &patched, &one_index(&patched)?),
            Err(ContentError::InvalidArtifact(
                "unknown ReferenceItemConsumption"
            ))
        ));
    }
    Ok(())
}

#[test]
fn potion_round_trips_and_rejects_invalid_restores_and_flasks() -> Result<(), ContentError> {
    use ReferenceItemField::{Conflict, Known, NotApplicable, Unknown};
    use ReferenceRestoreResource::{Health, Mana};
    for restores in [
        vec![(Health, 1, 1)],
        vec![(Mana, 10_000, 10_000)],
        vec![(Health, 125, 175), (Mana, 75, 125)],
    ] {
        for flask in [Known(target()?), NotApplicable] {
            let item = consumption_item(potion(&restores, flask));
            let (_, decoded) = server_round_trip(&item)?;
            assert_eq!(decoded, item.semantics);
        }
    }
    let invalid = [
        potion(&[(Health, 100, 200)], Unknown),
        potion(&[(Health, 100, 200)], Conflict),
        potion(&[], NotApplicable),
        potion(&[(Health, 1, 1), (Mana, 1, 1), (Mana, 2, 2)], NotApplicable),
        potion(&[(Health, 1, 1), (Health, 2, 2)], NotApplicable),
        potion(&[(Mana, 1, 1), (Health, 1, 1)], NotApplicable),
        potion(&[(Health, 0, 10)], NotApplicable),
        potion(&[(Health, 11, 10)], NotApplicable),
        potion(&[(Health, 1, 10_001)], NotApplicable),
    ];
    for consumption in invalid {
        let item = consumption_item(consumption);
        assert!(crate::content::reference_playable::validate_item_definition(&item).is_err());
    }
    // Three restores also exceed the encoder's count bound.
    let three = consumption_item(potion(
        &[(Health, 1, 1), (Mana, 1, 1), (Mana, 2, 2)],
        NotApplicable,
    ));
    assert!(
        encode_typed_item(
            &three,
            ReferenceArtifactProjection::ServerAuthoritative,
            &[source_definition(three.clone())?],
            V6,
        )
        .is_err()
    );

    // Wire tail of one restore with an NA flask: Known, Potion, 1, resource, min, max, NA.
    let (record, _) = server_round_trip(&consumption_item(potion(
        &[(Health, 100, 200)],
        NotApplicable,
    )))?;
    let end = record.len();
    assert_eq!(record[end - 9..end - 6], [3, 2, 1]);
    assert_eq!(record[end - 6], Health.wire());
    assert_eq!(record[end - 1], 1);
    let reject = |patch: &dyn Fn(&mut Vec<u8>)| -> Result<bool, ContentError> {
        let mut patched = record.clone();
        patch(&mut patched);
        Ok(parse_typed_server_item(V6, &patched, &one_index(&patched)?).is_err())
    };
    // Unknown or Conflict flask state.
    assert!(reject(&|bytes| bytes[end - 1] = 0)?);
    assert!(reject(&|bytes| bytes[end - 1] = 2)?);
    // Unknown resource.
    assert!(reject(&|bytes| bytes[end - 6] = 3)?);
    assert!(reject(&|bytes| bytes[end - 6] = 0)?);
    // Zero or three restores.
    assert!(reject(&|bytes| bytes[end - 7] = 0)?);
    assert!(reject(&|bytes| bytes[end - 7] = 3)?);
    // min 0, min > max, max 10,001.
    assert!(reject(
        &|bytes| bytes[end - 5..end - 3].copy_from_slice(&[0, 0])
    )?);
    assert!(reject(
        &|bytes| bytes[end - 5..end - 3].copy_from_slice(&201_u16.to_be_bytes())
    )?);
    assert!(reject(
        &|bytes| bytes[end - 3..end - 1].copy_from_slice(&10_001_u16.to_be_bytes())
    )?);

    // A Known flask that does not resolve fails encoding and decoding.
    let dangling = consumption_item(potion(
        &[(Health, 1, 1)],
        Known(ReferenceItemTarget::new(
            "oteryn:item.schema.not-in-family",
            "definition-r1",
        )?),
    ));
    assert!(
        encode_typed_item(
            &dangling,
            ReferenceArtifactProjection::ServerAuthoritative,
            &[source_definition(dangling.clone())?],
            V6,
        )
        .is_err()
    );
    let (record, _) = server_round_trip(&consumption_item(potion(
        &[(Health, 1, 1)],
        Known(target()?),
    )))?;
    let end = record.len();
    let mut patched = record.clone();
    patched[end - 4..].copy_from_slice(&1_u32.to_be_bytes());
    assert!(matches!(
        parse_typed_server_item(V6, &patched, &one_index(&patched)?),
        Err(ContentError::InvalidArtifact(
            "dangling Reference Item target ordinal"
        ))
    ));
    Ok(())
}

#[test]
fn consumption_group_exists_only_in_v6_server_records() -> Result<(), ContentError> {
    let item = consumption_item(food(30));
    let definitions = [source_definition(item.clone())?];
    for profile in [V4, V5] {
        assert!(
            encode_typed_item(
                &item,
                ReferenceArtifactProjection::ServerAuthoritative,
                &definitions,
                profile,
            )
            .is_err()
        );
    }
    let (record, _) = server_round_trip(&item)?;
    // The v6 server record re-labelled with the v5 body version: v5 refuses group 18.
    let mut as_v5 = record.clone();
    as_v5[0] = TYPED_V5_BODY_RECORD_VERSION;
    assert!(matches!(
        parse_typed_server_item(V5, &as_v5, &one_index(&as_v5)?),
        Err(ContentError::InvalidArtifact(
            "Reference Item group is unknown, duplicated, unordered or excluded from projection"
        ))
    ));
    // A client record never admits group 18, and group 19 is unknown on the server.
    let index = one_index(&[])?;
    let client = [4, 1, 1, 0, 1, 18, 0, 4, 3, 1, 0, 30];
    assert!(parse_typed_client_item(V6, &client, &index).is_err());
    let mut unknown = record;
    let group_at = unknown.len() - 7;
    assert_eq!(unknown[group_at], 18);
    unknown[group_at] = 19;
    assert!(parse_typed_server_item(V6, &unknown, &one_index(&unknown)?).is_err());
    Ok(())
}
