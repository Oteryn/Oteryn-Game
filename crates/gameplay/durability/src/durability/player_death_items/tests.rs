use super::*;
#[test]
fn player_death_planner_preserves_container_and_burns_only_necklace() {
    assert!(matches!(plan(false, 9, 1, 1), Ok((1, 1))));
    assert!(matches!(plan(true, 7, 1, 0), Ok((0, 2))));
    for args in [
        (true, 2, 1, 0),
        (true, 7, 2, 0),
        (false, 9, 1, 2),
        (false, 0, 101, 0),
    ] {
        assert!(plan(args.0, args.1, args.2, args.3).is_err());
    }
}
#[test]
fn player_death_resource_rows_max_and_max_plus_one() {
    for burn in [false, true] {
        let limits = if burn {
            audit::AMULET_LIMITS
        } else {
            audit::DROP_LIMITS
        };
        assert!(audit::check_usage(burn, limits).is_ok());
        for index in 0..limits.len() {
            let mut over = limits;
            over[index] += 1;
            assert_eq!(
                audit::check_usage(burn, over),
                Err(mint::AuditError::CapacityExceeded)
            );
        }
    }
}
fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}
fn request() -> PlayerDeathItemRequest {
    PlayerDeathItemRequest {
        transaction_id: id(20),
        event_id: id(21),
        occurred_at_unix_ms: 1_790_000_000_000,
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "simulation-1".into(),
        server_build_id: "death-test".into(),
    }
}
fn event(burn: bool) -> audit::OneItemPlayerDeathItemV1 {
    let before = mint::OneItemStateV1 {
        item_instance_id: id(10).to_vec(),
        world_id: id(42).to_vec(),
        definition: Some(definition_message(&TypedDefinitionRef {
            family: "Item".into(),
            production_key: "fixture:amulet".into(),
            revision_ref: "revision-1".into(),
        })),
        quantity: 1,
        lifecycle: 1,
        key_number: 0,
    };
    let after = mint::OneItemStateV1 {
        quantity: if burn { 0 } else { 1 },
        lifecycle: if burn { 2 } else { 1 },
        ..before.clone()
    };
    audit::OneItemPlayerDeathItemV1 {
        death_occurrence: id(11).to_vec(),
        item_instance: id(10).to_vec(),
        before: Some(before),
        after: Some(after),
        character_id: id(41).to_vec(),
        source_slot: 7,
        source_entry: None,
        ground: (!burn).then(|| mint::OneItemGroundV1 {
            world_id: id(42).to_vec(),
            channel_id: id(43).to_vec(),
            spatial_position: vec![1, 2, 3],
            corpse_ref: [
                super::super::item_loop_move::ITEM_LOOP_DROP_REF_PREFIX,
                id(20).as_slice(),
            ]
            .concat(),
            map_revision: "map-1".into(),
            content_revision: "content-1".into(),
            native_room_placement_context: id(11).to_vec(),
            runtime_scope_ownership_generation: 1,
            instance_id: None,
        }),
        typed_sink: if burn {
            audit::AMULET_SINK.into()
        } else {
            String::new()
        },
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "simulation-1".into(),
        session_generation: 1,
        game_session_id: id(50).to_vec(),
        lease_generation: 1,
        instance_scope: None,
    }
}
#[test]
fn player_death_audit_encoders_roundtrip_and_reject_silent_delete()
-> std::result::Result<(), mint::AuditError> {
    for burn in [false, true] {
        let value = event(burn);
        let wire = audit::encode(&request(), burn, value.clone(), mint::Type2EventTuple::V1)?;
        let (_, decoded_burn, decoded) = audit::decode(&wire)?;
        assert_eq!(decoded, value);
        assert_eq!(decoded_burn, burn);
        let mut wrong = value.clone();
        wrong.after = None;
        assert!(audit::encode(&request(), burn, wrong, mint::Type2EventTuple::V1).is_err());
        let mut wrong = value;
        wrong.typed_sink = "silent-delete".into();
        assert!(audit::encode(&request(), burn, wrong, mint::Type2EventTuple::V1).is_err());
    }
    Ok(())
}
#[test]
fn player_death_drop_audit_binds_loop_reference_to_transaction()
-> std::result::Result<(), mint::AuditError> {
    let value = event(false);
    let mut wrong = value.clone();
    wrong
        .ground
        .as_mut()
        .ok_or(mint::AuditError::InvalidInput)?
        .corpse_ref = id(11).to_vec();
    assert!(audit::encode(&request(), false, wrong, mint::Type2EventTuple::V1).is_err());
    let mut changed = request();
    changed.transaction_id = id(22);
    assert!(audit::encode(&changed, false, value, mint::Type2EventTuple::V1).is_err());
    let wire = audit::encode(&request(), false, event(false), mint::Type2EventTuple::V1)?;
    let (mut envelope, _, _) = audit::decode(&wire)?;
    envelope
        .transaction_event
        .as_mut()
        .ok_or(mint::AuditError::InvalidInput)?
        .transaction_id = id(22).to_vec();
    use prost::Message;
    assert!(audit::decode(&envelope.encode_to_vec()).is_err());
    Ok(())
}
#[test]
fn player_death_request_binding_is_exact_and_unambiguous() {
    let first = request();
    let mut changed = first.clone();
    changed.sim_revision.push('x');
    assert_ne!(
        first.binding(false, &id(1), &id(2), &id(3)),
        changed.binding(false, &id(1), &id(2), &id(3))
    );
    assert_ne!(
        first.binding(false, &id(1), &id(2), &id(3)),
        first.binding(true, &id(1), &id(2), &id(3))
    );
}

#[test]
fn player_death_audit_maximum_fields_and_oversize_rejection()
-> std::result::Result<(), mint::AuditError> {
    use prost::Message;
    for burn in [false, true] {
        let mut request = request();
        request.content_revision = "c".repeat(512);
        request.ruleset_revision = "r".repeat(512);
        request.sim_revision = "s".repeat(512);
        request.server_build_id = "b".repeat(128);
        let mut value = event(burn);
        value.content_revision = request.content_revision.clone();
        value.ruleset_revision = request.ruleset_revision.clone();
        value.sim_revision = request.sim_revision.clone();
        for state in [&mut value.before, &mut value.after] {
            let definition = state
                .as_mut()
                .and_then(|s| s.definition.as_mut())
                .ok_or(mint::AuditError::InvalidInput)?;
            definition.family = "I".repeat(128);
            definition.production_key = "k".repeat(512);
            definition.revision_ref = "d".repeat(512);
        }
        if let Some(ground) = value.ground.as_mut() {
            ground.map_revision = "m".repeat(512);
            ground.content_revision = request.content_revision.clone();
            ground.spatial_position = vec![1; 128];
        }
        for tuple in [mint::Type2EventTuple::V1, mint::Type2EventTuple::V2] {
            let wire = audit::encode(&request, burn, value.clone(), tuple)?;
            let (mut envelope, _, _) = audit::decode(&wire)?;
            assert!(envelope.payload.len() <= mint::RL07_PAYLOAD_BYTES_MAX);
            assert!(wire.len() <= mint::RL07_ENVELOPE_BYTES_MAX);
            envelope.payload.push(0); // noncanonical/unknown payload bytes fail closed
            envelope.payload_sha256 = Sha256::digest(&envelope.payload).to_vec();
            assert!(audit::decode(&envelope.encode_to_vec()).is_err());
        }
        value.sim_revision.push('x');
        assert!(audit::encode(&request, burn, value, mint::Type2EventTuple::V1).is_err());
    }
    assert!(matches!(
        audit::decode(&vec![0; mint::RL07_ENVELOPE_BYTES_MAX + 1]),
        Err(mint::AuditError::CapacityExceeded)
    ));
    Ok(())
}

// W02 prerequisite: the new Instance source is explicit; legacy Channel fixtures above
// keep an absent additive field and retain their original encoded representation.
#[test]
fn player_death_instance_drop_and_amulet_audit_bind_the_exact_scope()
-> std::result::Result<(), mint::AuditError> {
    use prost::Message;
    let world =
        crate::foundation::WorldId::decode(&id(42)).map_err(|_| mint::AuditError::InvalidInput)?;
    let scope =
        RuntimeScopeRefV1::instance(world, id(43)).map_err(|_| mint::AuditError::InvalidInput)?;
    for burn in [false, true] {
        let legacy = event(burn);
        let legacy_wire =
            audit::encode(&request(), burn, legacy.clone(), mint::Type2EventTuple::V1)?;
        let mut value = legacy;
        value.instance_scope = Some(audit::OneItemPlayerDeathInstanceV1 {
            instance_id: id(43).to_vec(),
            ownership_generation: 1,
        });
        if let Some(ground) = value.ground.take() {
            value.ground = Some(ground.with_runtime_scope(scope));
        }
        let wire = audit::encode(&request(), burn, value.clone(), mint::Type2EventTuple::V1)?;
        assert_ne!(wire, legacy_wire);
        let (mut envelope, decoded_burn, decoded) = audit::decode(&wire)?;
        assert_eq!(decoded_burn, burn);
        assert_eq!(decoded, value);
        assert_eq!(envelope.instance_id, Some(id(43).to_vec()));
        assert_eq!(envelope.channel_id, None);
        envelope.channel_id = Some(id(43).to_vec());
        assert!(audit::decode(&envelope.encode_to_vec()).is_err());
        let mut wrong = value.clone();
        wrong
            .instance_scope
            .as_mut()
            .ok_or(mint::AuditError::InvalidInput)?
            .ownership_generation = 0;
        assert!(audit::encode(&request(), burn, wrong, mint::Type2EventTuple::V1).is_err());
        if !burn {
            let mut mixed = value.clone();
            mixed
                .ground
                .as_mut()
                .ok_or(mint::AuditError::InvalidInput)?
                .channel_id = id(43).to_vec();
            assert!(audit::encode(&request(), burn, mixed, mint::Type2EventTuple::V1).is_err());
            let mut other = value;
            other
                .instance_scope
                .as_mut()
                .ok_or(mint::AuditError::InvalidInput)?
                .instance_id = id(44).to_vec();
            assert!(audit::encode(&request(), burn, other, mint::Type2EventTuple::V1).is_err());
        }
    }
    Ok(())
}

#[test]
fn ground_scope_projection_rejects_missing_mixed_and_malformed_identity()
-> std::result::Result<(), mint::AuditError> {
    let ground = event(false).ground.ok_or(mint::AuditError::InvalidInput)?;
    assert!(matches!(
        ground.runtime_scope(),
        Ok(RuntimeScopeRefV1::Channel { .. })
    ));
    let mut missing = ground.clone();
    missing.channel_id.clear();
    assert!(missing.runtime_scope().is_err());
    let mut mixed = ground.clone();
    mixed.instance_id = Some(id(43).to_vec());
    assert!(mixed.runtime_scope().is_err());
    let mut malformed = missing.clone();
    malformed.instance_id = Some(vec![0; 16]);
    assert!(malformed.runtime_scope().is_err());
    let scope = RuntimeScopeRefV1::instance(
        crate::foundation::WorldId::decode(&id(42)).map_err(|_| mint::AuditError::InvalidInput)?,
        id(43),
    )
    .map_err(|_| mint::AuditError::InvalidInput)?;
    assert_eq!(ground.with_runtime_scope(scope).runtime_scope()?, scope);
    Ok(())
}
