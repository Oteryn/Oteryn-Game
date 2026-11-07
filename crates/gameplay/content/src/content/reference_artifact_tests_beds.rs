const V7: ReferenceArtifactProfile = ReferenceArtifactProfile::TypedItemV7;

fn bed(
    part: ReferenceBedPart,
    partner_direction: ReferenceBedDirection,
    occupied_male: ReferenceItemTarget,
    occupied_female: ReferenceItemTarget,
) -> ReferenceItemBed {
    ReferenceItemBed {
        part,
        partner_direction,
        occupied_male,
        occupied_female,
    }
}

fn bed_item(value: ReferenceItemBed) -> ReferenceItemDefinition {
    plain_item(ReferenceItemSemantics {
        bed: ReferenceItemField::Known(value),
        ..Default::default()
    })
}

fn worst_item_v7() -> Result<ReferenceItemDefinition, ContentError> {
    let mut item = worst_item_v6()?;
    item.semantics.bed = ReferenceItemField::Known(bed(
        ReferenceBedPart::Head,
        ReferenceBedDirection::North,
        target()?,
        target()?,
    ));
    Ok(item)
}

fn look(ordinal: usize) -> Result<ReferenceItemTarget, ContentError> {
    ReferenceItemTarget::new(
        &format!("oteryn:item.schema.boundary-{ordinal:05}"),
        "definition-r1",
    )
}

/// Two other looks sorted before the record's own Item.
fn bed_definitions(
    item: ReferenceItemDefinition,
) -> Result<Vec<ReferenceDefinition>, ContentError> {
    let mut definitions = (0..2)
        .map(|ordinal| {
            let look = look(ordinal)?;
            Ok(ReferenceDefinition {
                definition: look.typed_ref()?,
                kind: ReferenceDefinitionKind::Item(plain_item(Default::default())),
                client_projection: ClientProjectionClass::ClientSafe,
            })
        })
        .collect::<Result<Vec<_>, ContentError>>()?;
    definitions.push(source_definition(item)?);
    Ok(definitions)
}

fn v7_server_round_trip(
    item: &ReferenceItemDefinition,
    definitions: &[ReferenceDefinition],
) -> Result<(Vec<u8>, ReferenceItemSemantics), ContentError> {
    let record = encode_typed_item(
        item,
        ReferenceArtifactProjection::ServerAuthoritative,
        definitions,
        V7,
    )?;
    let index = definitions
        .iter()
        .map(|definition| IndexEntry {
            identity: definition.definition.clone(),
            body_offset: 0,
            body_length: record.len(),
            body_digest: sha256(&record),
        })
        .collect::<Vec<_>>();
    let decoded = parse_typed_server_item(V7, &record, &index)?.semantics;
    Ok((record, decoded))
}

/// A self-targeted Head facing North, alone in its family.
fn self_bed_record() -> Result<Vec<u8>, ContentError> {
    let item = bed_item(bed(
        ReferenceBedPart::Head,
        ReferenceBedDirection::North,
        target()?,
        target()?,
    ));
    Ok(v7_server_round_trip(&item, &[source_definition(item.clone())?])?.0)
}

#[test]
fn typed_v7_body_matches_registered_record_maxima_and_round_trips() -> Result<(), ContentError> {
    let item = worst_item_v7()?;
    let definitions = worst_definitions(item.clone())?;
    let server = encode_typed_item(
        &item,
        ReferenceArtifactProjection::ServerAuthoritative,
        &definitions,
        V7,
    )?;
    let client = encode_typed_item(
        &item,
        ReferenceArtifactProjection::ClientSafe,
        &definitions,
        V7,
    )?;
    assert_eq!(
        server.len(),
        TYPED_V7_REFERENCE_ITEM_MAX_SERVER_RECORD_BYTES
    );
    assert_eq!(
        client.len(),
        TYPED_V7_REFERENCE_ITEM_MAX_CLIENT_RECORD_BYTES
    );
    // The same worst shape as the resource-profile tool's v7 evidence.
    assert_eq!(
        sha256(&server).as_slice(),
        hex_bytes("60719d884c6008a4472b70c30a3b925fd0aaeea28ab86e6ea9c6c6869d3e8521")?
    );
    assert_eq!(
        sha256(&client).as_slice(),
        hex_bytes("ec9391cb9289ee626185363cfe2b6dd2d55de35b4f4a87997591053d2b4af599")?
    );
    // 19 server groups and 12 client groups: the registered exact maxima.
    assert_eq!(server[6..8], 19_u16.to_be_bytes());
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
        parse_typed_server_item(V7, &server, &index)?.semantics,
        item.semantics
    );
    assert_eq!(
        parse_typed_client_item(V7, &client, &index)?.semantics,
        item.semantics.client_projection()
    );
    let mut over = server.clone();
    over.push(0);
    assert!(matches!(
        parse_typed_server_item(V7, &over, &index),
        Err(ContentError::LimitExceeded {
            actual: 3_613,
            limit: 3_612,
            ..
        })
    ));
    let mut over = client.clone();
    over.push(0);
    assert!(matches!(
        parse_typed_client_item(V7, &over, &index),
        Err(ContentError::LimitExceeded {
            actual: 3_455,
            limit: 3_454,
            ..
        })
    ));
    let mut groups = server;
    groups[6..8].copy_from_slice(&20_u16.to_be_bytes());
    assert!(matches!(
        parse_typed_server_item(V7, &groups, &index),
        Err(ContentError::LimitExceeded {
            resource: "Reference Item groups",
            actual: 20,
            limit: 19,
        })
    ));
    let mut groups = client;
    groups[3..5].copy_from_slice(&13_u16.to_be_bytes());
    assert!(matches!(
        parse_typed_client_item(V7, &groups, &index),
        Err(ContentError::LimitExceeded {
            resource: "Reference Item groups",
            actual: 13,
            limit: 12,
        })
    ));
    Ok(())
}

#[test]
fn typed_v7_resource_limits_accept_exact_max_and_reject_max_plus_one() {
    for (projection, record, body, artifact) in [
        (
            ReferenceArtifactProjection::ServerAuthoritative,
            TYPED_V7_REFERENCE_ITEM_MAX_SERVER_RECORD_BYTES,
            TYPED_V7_REFERENCE_ITEM_MAX_SERVER_BODY_BYTES,
            TYPED_V7_REFERENCE_ITEM_MAX_SERVER_ARTIFACT_BYTES,
        ),
        (
            ReferenceArtifactProjection::ClientSafe,
            TYPED_V7_REFERENCE_ITEM_MAX_CLIENT_RECORD_BYTES,
            TYPED_V7_REFERENCE_ITEM_MAX_CLIENT_BODY_BYTES,
            TYPED_V7_REFERENCE_ITEM_MAX_CLIENT_ARTIFACT_BYTES,
        ),
    ] {
        assert_eq!(V7.body_record_limit(projection), record);
        assert!(check_body_record_length(V7, projection, record).is_ok());
        assert!(check_body_record_length(V7, projection, record + 1).is_err());
        assert!(check_body_section_length(V7, projection, body).is_ok());
        assert!(check_body_section_length(V7, projection, body + 1).is_err());
        assert!(check_artifact_length(V7, artifact, projection).is_ok());
        assert!(check_artifact_length(V7, artifact + 1, projection).is_err());
    }
    assert_eq!(
        TYPED_V7_REFERENCE_ITEM_MAX_SERVER_BODY_BYTES,
        FAMILY_MAX_INDEX_ENTRIES * TYPED_V7_REFERENCE_ITEM_MAX_SERVER_RECORD_BYTES
    );
    assert_eq!(
        TYPED_V7_REFERENCE_ITEM_MAX_CLIENT_BODY_BYTES,
        FAMILY_MAX_INDEX_ENTRIES * TYPED_V7_REFERENCE_ITEM_MAX_CLIENT_RECORD_BYTES
    );
    assert_eq!(
        TYPED_V7_REFERENCE_ITEM_MAX_GENERATION_PAIR_BYTES,
        TYPED_V7_REFERENCE_ITEM_MAX_SERVER_ARTIFACT_BYTES
            + TYPED_V7_REFERENCE_ITEM_MAX_CLIENT_ARTIFACT_BYTES
    );
    assert!(
        check_pair_lengths(
            V7,
            TYPED_V7_REFERENCE_ITEM_MAX_SERVER_ARTIFACT_BYTES,
            TYPED_V7_REFERENCE_ITEM_MAX_CLIENT_ARTIFACT_BYTES,
        )
        .is_ok()
    );
    assert!(
        check_pair_lengths(
            V7,
            TYPED_V7_REFERENCE_ITEM_MAX_SERVER_ARTIFACT_BYTES,
            TYPED_V7_REFERENCE_ITEM_MAX_CLIENT_ARTIFACT_BYTES + 1,
        )
        .is_err()
    );
    assert_eq!(
        V7.max_groups(ReferenceArtifactProjection::ServerAuthoritative),
        19
    );
    assert_eq!(V7.max_groups(ReferenceArtifactProjection::ClientSafe), 12);
    assert_eq!(
        V6.max_groups(ReferenceArtifactProjection::ServerAuthoritative),
        18
    );
    // v6 keeps its own ceilings for decoding.
    assert_eq!(
        V6.body_record_limit(ReferenceArtifactProjection::ServerAuthoritative),
        TYPED_V6_REFERENCE_ITEM_MAX_SERVER_RECORD_BYTES
    );
}

#[test]
fn cross_item_target_slots_are_13_in_v6_and_15_in_v7() -> Result<(), Box<dyn std::error::Error>> {
    // Every target slot of the worst shape points at the boundary witness.
    let slots = |item: &ReferenceItemDefinition| -> Result<usize, serde_json::Error> {
        Ok(serde_json::to_string(&item.semantics)?
            .matches("\"oteryn:item.schema.boundary-witness\"")
            .count())
    };
    // The last group of each worst record: consumption (18 payload bytes) in v6 and the bed
    // (11) in v7.
    for (profile, item, slot_count, last_group, payload) in [
        (V6, worst_item_v6()?, 13, ITEM_GROUP_CONSUMPTION, 18),
        (V7, worst_item_v7()?, 15, ITEM_GROUP_BED, 11),
    ] {
        assert_eq!(slots(&item)?, slot_count);
        let definitions = worst_definitions(item.clone())?;
        let record = encode_typed_item(
            &item,
            ReferenceArtifactProjection::ServerAuthoritative,
            &definitions,
            profile,
        )?;
        let index = definitions
            .iter()
            .map(|definition| IndexEntry {
                identity: definition.definition.clone(),
                body_offset: 0,
                body_length: record.len(),
                body_digest: sha256(&record),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            parse_typed_server_item(profile, &record, &index)?.semantics,
            item.semantics
        );
        // A target past the last slot of the last group is refused.
        let mut extra = record.clone();
        let group_length_at = extra.len() - payload - 2;
        assert_eq!(extra[group_length_at - 1], last_group);
        assert_eq!(
            extra[group_length_at..group_length_at + 2],
            u16::try_from(payload)?.to_be_bytes()
        );
        extra[group_length_at..group_length_at + 2]
            .copy_from_slice(&u16::try_from(payload + 4)?.to_be_bytes());
        extra.extend_from_slice(&0_u32.to_be_bytes());
        assert!(parse_typed_server_item(profile, &extra, &index).is_err());
        // A dangling last target is refused.
        let mut dangling = record;
        let end = dangling.len();
        dangling[end - 4..].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(matches!(
            parse_typed_server_item(profile, &dangling, &index),
            Err(ContentError::InvalidArtifact(
                "dangling Reference Item target ordinal"
            ))
        ));
    }
    Ok(())
}

#[test]
fn bed_round_trips_for_each_part_direction_and_look() -> Result<(), ContentError> {
    use ReferenceBedDirection::{East, North, South, West};
    use ReferenceBedPart::{Foot, Head};
    let looks = [
        // Distinct occupied looks.
        (look(0)?, look(1)?),
        // One look for both sexes.
        (look(1)?, look(1)?),
        // "No change": the record's own Item.
        (target()?, target()?),
    ];
    for part in [Head, Foot] {
        for direction in [North, East, South, West] {
            for (male, female) in &looks {
                let item = bed_item(bed(part, direction, male.clone(), female.clone()));
                let (record, decoded) =
                    v7_server_round_trip(&item, &bed_definitions(item.clone())?)?;
                assert_eq!(record[0], TYPED_V7_BODY_RECORD_VERSION);
                assert_eq!(decoded, item.semantics);
                let end = record.len();
                assert_eq!(
                    record[end - 14..end - 8],
                    [ITEM_GROUP_BED, 0, 11, 3, part.wire(), direction.wire()]
                );
            }
        }
    }
    // The client projection carries no bed group.
    let item = bed_item(bed(Head, North, target()?, target()?));
    let client = encode_typed_item(
        &item,
        ReferenceArtifactProjection::ClientSafe,
        &[source_definition(item.clone())?],
        V7,
    )?;
    let decoded = parse_typed_client_item(V7, &client, &one_index(&client)?)?.semantics;
    assert_eq!(decoded.bed, ReferenceItemField::Unknown);
    Ok(())
}

#[test]
fn bed_decode_rejects_unknown_part_direction_and_dangling_targets() -> Result<(), ContentError> {
    let record = self_bed_record()?;
    let end = record.len();
    let decode = |patched: &[u8]| parse_typed_server_item(V7, patched, &one_index(patched)?);
    for part in [0, 3, u8::MAX] {
        let mut patched = record.clone();
        patched[end - 10] = part;
        assert!(matches!(
            decode(&patched),
            Err(ContentError::InvalidArtifact("unknown ReferenceBedPart"))
        ));
    }
    for direction in [0, 5, u8::MAX] {
        let mut patched = record.clone();
        patched[end - 9] = direction;
        assert!(matches!(
            decode(&patched),
            Err(ContentError::InvalidArtifact(
                "unknown ReferenceBedDirection"
            ))
        ));
    }
    for at in [end - 8, end - 4] {
        let mut patched = record.clone();
        patched[at..at + 4].copy_from_slice(&1_u32.to_be_bytes());
        assert!(matches!(
            decode(&patched),
            Err(ContentError::InvalidArtifact(
                "dangling Reference Item target ordinal"
            ))
        ));
    }
    // A group id past 19 is unknown in v7.
    let mut unknown = record;
    assert_eq!(unknown[end - 14], ITEM_GROUP_BED);
    unknown[end - 14] = 20;
    assert!(decode(&unknown).is_err());
    Ok(())
}

#[test]
fn bed_encode_rejects_a_dangling_target() -> Result<(), ContentError> {
    let missing = ReferenceItemTarget::new("oteryn:item.schema.not-in-family", "definition-r1")?;
    for (male, female) in [(missing.clone(), target()?), (target()?, missing)] {
        let item = bed_item(bed(
            ReferenceBedPart::Foot,
            ReferenceBedDirection::South,
            male,
            female,
        ));
        assert!(
            encode_typed_item(
                &item,
                ReferenceArtifactProjection::ServerAuthoritative,
                &[source_definition(item.clone())?],
                V7,
            )
            .is_err()
        );
    }
    Ok(())
}

#[test]
fn bed_group_exists_only_in_v7_server_records() -> Result<(), ContentError> {
    let item = bed_item(bed(
        ReferenceBedPart::Head,
        ReferenceBedDirection::West,
        target()?,
        target()?,
    ));
    let definitions = [source_definition(item.clone())?];
    for profile in [V4, V5, V6] {
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
    let record = self_bed_record()?;
    // The v7 record re-labelled with the v6 body version: v6 refuses group 19.
    let mut as_v6 = record.clone();
    as_v6[0] = TYPED_V6_BODY_RECORD_VERSION;
    assert!(matches!(
        parse_typed_server_item(V6, &as_v6, &one_index(&as_v6)?),
        Err(ContentError::InvalidArtifact(
            "Reference Item group is unknown, duplicated, unordered or excluded from projection"
        ))
    ));
    // A v6 reader refuses the v7 body version.
    assert!(parse_typed_server_item(V6, &record, &one_index(&record)?).is_err());
    // A client record never admits group 19.
    let index = one_index(&[])?;
    let client = [
        TYPED_V7_BODY_RECORD_VERSION,
        1,
        1,
        0,
        1,
        ITEM_GROUP_BED,
        0,
        11,
        3,
        1,
        1,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ];
    assert!(parse_typed_client_item(V7, &client, &index).is_err());
    Ok(())
}
