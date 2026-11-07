use super::*;

/// #1703 P2 4175400422 and #1711 P2: a selected set naming 4 without 6, or 12 without 4, is
/// refused by the encoder and by ingress decode of both acceptance messages (ITEM-USE-WIRE-1:
/// and 15 without 4).
#[test]
fn acceptance_refuses_a_selected_capability_without_its_requirement()
-> Result<(), FoundationProtocolError> {
    let (session, world, channel) = (test_uuid_v7(1), test_uuid_v7(2), test_uuid_v7(3));
    let accepted = ServerAcceptedValue {
        game_session_id: GameSessionId::decode(&session)?,
        world_id: WorldId::decode(&world)?,
        channel_id: ChannelId::decode(&channel)?,
        connection_generation: 1,
        current_server_sequence: 0,
        next_command_id: 1,
        schema_revision: 1,
        selected_capabilities: &[],
    };
    let resumed = ServerResumeAcceptedValue {
        game_session_id: accepted.game_session_id,
        connection_generation: 2,
        current_server_sequence: 0,
        next_command_id: 1,
        schema_revision: 1,
        selected_capabilities: &[],
    };
    for (selected, valid) in [
        (&[4_u32][..], false),
        (
            &[
                4,  // ITEM_VIEW_MOVE_V1
                12, // ITEM_EQUIP_DROP_V1
            ][..],
            false,
        ),
        (
            &[
                6,  // WORLD_SPATIAL_ENTITIES
                12, // ITEM_EQUIP_DROP_V1
            ][..],
            false,
        ),
        (&[12][..], false),
        (
            &[
                4, // ITEM_VIEW_MOVE_V1
                6, // WORLD_SPATIAL_ENTITIES
            ][..],
            true,
        ),
        (
            &[
                4,  // ITEM_VIEW_MOVE_V1
                6,  // WORLD_SPATIAL_ENTITIES
                12, // ITEM_EQUIP_DROP_V1
            ][..],
            true,
        ),
        (&[14][..], false),
        (
            &[
                4,  // ITEM_VIEW_MOVE_V1
                6,  // WORLD_SPATIAL_ENTITIES
                14, // CONTAINER_TREE_V1
            ][..],
            false,
        ),
        (
            &[
                6,  // WORLD_SPATIAL_ENTITIES
                12, // ITEM_EQUIP_DROP_V1
                14, // CONTAINER_TREE_V1
            ][..],
            false,
        ),
        (
            &[
                4,  // ITEM_VIEW_MOVE_V1
                6,  // WORLD_SPATIAL_ENTITIES
                12, // ITEM_EQUIP_DROP_V1
                14, // CONTAINER_TREE_V1
            ][..],
            true,
        ),
        (&[15][..], false),
        (
            &[
                6,  // WORLD_SPATIAL_ENTITIES
                15, // ITEM_USE_V1
            ][..],
            false,
        ),
        (
            &[
                4,  // ITEM_VIEW_MOVE_V1
                6,  // WORLD_SPATIAL_ENTITIES
                15, // ITEM_USE_V1
            ][..],
            true,
        ),
        (&[17][..], false),
        (
            &[
                4,  // ITEM_VIEW_MOVE_V1
                17, // ATTACK_V1
            ][..],
            false,
        ),
        (
            &[
                6,  // WORLD_SPATIAL_ENTITIES
                17, // ATTACK_V1
            ][..],
            true,
        ),
        (
            &[
                4,  // ITEM_VIEW_MOVE_V1
                6,  // WORLD_SPATIAL_ENTITIES
                17, // ATTACK_V1
            ][..],
            true,
        ),
        (&[19][..], false),
        (
            &[
                4,  // ITEM_VIEW_MOVE_V1
                19, // WORLD_LIGHT_V1
            ][..],
            false,
        ),
        (
            &[
                6,  // WORLD_SPATIAL_ENTITIES
                19, // WORLD_LIGHT_V1
            ][..],
            true,
        ),
        (
            &[
                4,  // ITEM_VIEW_MOVE_V1
                6,  // WORLD_SPATIAL_ENTITIES
                19, // WORLD_LIGHT_V1
            ][..],
            true,
        ),
        (&[11][..], false),
        (
            &[
                4,  // ITEM_VIEW_MOVE_V1
                6,  // WORLD_SPATIAL_ENTITIES
                11, // WORLD_INTERACTION_V1
                15, // ITEM_USE_V1
            ][..],
            false,
        ),
        (
            &[
                4,  // ITEM_VIEW_MOVE_V1
                6,  // WORLD_SPATIAL_ENTITIES
                11, // WORLD_INTERACTION_V1
                15, // ITEM_USE_V1
                18, // WORLD_MAP_VIEW_V1
            ][..],
            true,
        ),
        (&[22][..], false),
        (
            &[
                4,  // ITEM_VIEW_MOVE_V1
                6,  // WORLD_SPATIAL_ENTITIES
                22, // RUNE_USE_V1
            ][..],
            false,
        ),
        (
            &[
                4,  // ITEM_VIEW_MOVE_V1
                6,  // WORLD_SPATIAL_ENTITIES
                15, // ITEM_USE_V1
                22, // RUNE_USE_V1
            ][..],
            true,
        ),
    ] {
        let expected = |result: Result<(), FoundationProtocolError>| match valid {
            true => assert!(result.is_ok(), "{selected:?}"),
            false => assert_eq!(
                result,
                Err(FoundationProtocolError::InvalidCapabilitySet),
                "{selected:?}"
            ),
        };
        expected(
            encode_server_accepted(&ServerAcceptedValue {
                selected_capabilities: selected,
                ..accepted
            })
            .map(drop),
        );
        expected(
            encode_server_resume_accepted(&ServerResumeAcceptedValue {
                selected_capabilities: selected,
                ..resumed
            })
            .map(drop),
        );
        let wide: Vec<usize> = selected.iter().map(|&c| c as usize).collect();
        expected(
            decode_wire_envelope(&test_envelope(
                2,
                &test_server_accepted_payload(1, 1, 1, [&session, &world, &channel], &wide),
            ))
            .map(drop),
        );
        expected(
            decode_wire_envelope(&test_envelope(
                4,
                &test_server_resume_accepted_payload(2, 1, 1, &session, &wide),
            ))
            .map(drop),
        );
        expected(
            decode_server_accepted(&test_server_accepted_payload(
                1,
                1,
                1,
                [&session, &world, &channel],
                &wide,
            ))
            .map(drop),
        );
    }
    Ok(())
}

#[test]
fn selected_capabilities_are_registry_closed_but_supported_unknowns_are_additive() {
    let session = test_uuid_v7(1);
    let world = test_uuid_v7(2);
    let channel = test_uuid_v7(3);
    assert!(
        decode_wire_envelope(&test_envelope(
            1,
            &test_client_bootstrap_payload(1, 1, 1, &session, &[777])
        ))
        .is_ok()
    );
    assert!(
        decode_wire_envelope(&test_envelope(
            3,
            &test_client_resume_payload(1, 1, 1, &session, &[777])
        ))
        .is_ok()
    );
    // 1 BESTIARY_CHARMS_V1, 4 ITEM_VIEW_MOVE_V1, 6 WORLD_SPATIAL_ENTITIES, 7 CHAT_V1, 8
    // ACHIEVEMENT_NOTICES_V1 and 10 ANALYSER_V1 are registered; 2 is reserved for PROF-WIRE-1
    // and 1000 is unallocated: a selected capability this build does not know fails.
    for selected in [
        &[6_usize][..],
        &[
            1, // BESTIARY_CHARMS_V1
            6, // WORLD_SPATIAL_ENTITIES
        ][..],
        &[
            4, // ITEM_VIEW_MOVE_V1
            6, // WORLD_SPATIAL_ENTITIES
        ][..],
        &[7][..],
        &[8][..],
        &[10][..],
        &[
            1,  // BESTIARY_CHARMS_V1
            4,  // ITEM_VIEW_MOVE_V1
            6,  // WORLD_SPATIAL_ENTITIES
            7,  // CHAT_V1
            8,  // ACHIEVEMENT_NOTICES_V1
            10, // ANALYSER_V1
        ][..],
    ] {
        assert!(
            decode_wire_envelope(&test_envelope(
                2,
                &test_server_accepted_payload(1, 1, 1, [&session, &world, &channel], selected)
            ))
            .is_ok()
        );
    }
    assert_eq!(
        decode_wire_envelope(&test_envelope(
            2,
            &test_server_accepted_payload(1, 1, 1, [&session, &world, &channel], &[1000])
        )),
        Err(FoundationProtocolError::CapabilityMismatch)
    );
    assert!(
        decode_wire_envelope(&test_envelope(
            2,
            &test_server_accepted_payload(1, 1, 1, [&session, &world, &channel], &[1])
        ))
        .is_ok()
    );
    assert_eq!(
        decode_wire_envelope(&test_envelope(
            2,
            &test_server_accepted_payload(1, 1, 1, [&session, &world, &channel], &[2])
        )),
        Err(FoundationProtocolError::CapabilityMismatch)
    );
    assert_eq!(
        decode_wire_envelope(&test_envelope(
            4,
            &test_server_resume_accepted_payload(2, 1, 1, &session, &[2])
        )),
        Err(FoundationProtocolError::CapabilityMismatch)
    );
}

#[test]
fn seam_typed_bootstrap_borrows_validated_material() -> Result<(), FoundationProtocolError> {
    let id = test_uuid_v7(1);
    let wire = test_envelope(
        1,
        &test_client_bootstrap_payload(
            1,
            1,
            2,
            &id,
            &[
                7, // CHAT_V1
                9, // HIGHSCORES_V1
            ],
        ),
    );
    let envelope = decode_wire_envelope(&wire)?;
    let view = envelope.client_bootstrap()?;
    assert_eq!(view.protocol_major, 1);
    assert_eq!(view.transport_profile, 1);
    assert_eq!(view.schema_revision, 2);
    assert_eq!(view.character_id.as_bytes(), &id);
    assert_eq!(view.admission_material, &[0xaa]);
    assert_eq!(view.client_build_id, "test-client");
    assert_eq!(
        view.supported_capabilities,
        &[
            7, // CHAT_V1
            9, // HIGHSCORES_V1
        ]
    );
    let start = wire.as_ptr() as usize;
    assert!((start..start + wire.len()).contains(&(view.admission_material.as_ptr() as usize)));
    assert!(envelope.client_resume().is_err());
    Ok(())
}
