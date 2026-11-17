// --- ENTITY-CLIENT-1: capability 6 (domain 1 type 2 snapshot and delta).

const OWN: [u8; 16] = [0xa0; 16];

#[test]
fn clxa_entity_light_survives_join_update_expiry_and_leave() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(clxa_entity_light_scenario)?
        .join()
        .map_err(|_| "CLX-A light session panicked")?
}

fn clxa_entity_light_scenario() -> Result<(), BoxError> {
    block_on(async {
        use oteryn_protocol_oteryn::world_spatial_entities::{
            EntityFields, EntityLight, EntityLights,
            encode_world_spatial_entities_delta_with_fields,
            encode_world_spatial_entities_snapshot_with_fields,
        };
        let fields = EntityFields::selected(&[6, 19]);
        let player = own(at(0, 0));
        let rat = actor(EntityKind::Creature, 8, 1, at(1, 0));
        let light = EntityLight {
            level: 8,
            color: 215,
        };
        let lights = EntityLights::from([(OWN, light), (rat.entity.identity, light)]);
        let snapshot = WorldSpatialEntitiesSnapshot {
            content_generation: CONTENT_GENERATION,
            actor_position: at(0, 0),
            own_identity: OWN,
            entities: vec![player, rat],
        };
        let bytes = encode_world_spatial_entities_snapshot_with_fields(&snapshot, fields, &lights)
            .map_err(|e| format!("{e:?}"))?;
        let (client, mut server) = tokio::io::duplex(256 * 1024);
        let peer = tokio::spawn(async move {
            let clock = oteryn_protocol_oteryn::world_clock::encode_world_clock(
                oteryn_protocol_oteryn::world_clock::WorldClock {
                    tibian_minute: 0,
                    light_level: 40,
                    light_color: 215,
                },
            )
            .map_err(|e| format!("{e:?}"))?;
            join_peer_with(
                &mut server,
                CLIENT_SUPPORTED_CAPABILITIES,
                &[6, 19],
                &[],
                (2, bytes),
                &[(18, 0, 1, clock)],
            )
            .await?;
            let mut rest = Vec::new();
            let _ = server.read_to_end(&mut rest).await;
            Ok::<_, BoxError>(())
        });
        let mut session = Session::admit(client, entity_admission()?).await?;
        assert_eq!(
            session.visible_appearances()[0].light,
            Some((light.level, light.color))
        );
        for (sequence, update, lights) in [
            (
                41,
                entities_delta(at(0, 0), vec![], vec![player], vec![]),
                EntityLights::new(),
            ),
            (
                42,
                entities_delta(at(0, 0), vec![], vec![], vec![rat.entity]),
                EntityLights::new(),
            ),
        ] {
            let bytes = encode_world_spatial_entities_delta_with_fields(&update, fields, &lights)
                .map_err(|e| format!("{e:?}"))?;
            let frame = oteryn_protocol_oteryn::encode_state_delta(
                1,
                sequence,
                1,
                sequence - 36,
                sequence - 35,
                2,
                &bytes,
            )?;
            session.apply_state_delta(
                sequence,
                oteryn_protocol_oteryn::decode_wire_envelope(&frame)?.payload(),
            )?;
            let entries = session.visible_appearances();
            assert!(
                entries
                    .iter()
                    .find(|e| e.entity.entity.identity == OWN)
                    .ok_or("own")?
                    .light
                    .is_none()
            );
            if sequence == 41 {
                assert_eq!(
                    entries
                        .iter()
                        .find(|e| e.entity.entity == rat.entity)
                        .ok_or("rat")?
                        .light,
                    Some((light.level, light.color))
                );
            } else {
                assert_eq!(entries.len(), 1);
            }
        }
        drop(session);
        peer.await??;
        Ok(())
    })?
}

fn at(x: i32, y: i32) -> ActorPosition {
    ActorPosition { x, y, floor: 0 }
}

fn actor(
    kind: EntityKind,
    marker: u8,
    generation: u64,
    position: ActorPosition,
) -> WorldSpatialEntity {
    WorldSpatialEntity {
        display_name: None,
        kind,
        entity: EntityRef {
            identity: [marker; 16],
            generation,
        },
        position,
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    }
}

fn own(position: ActorPosition) -> WorldSpatialEntity {
    let mut entity = actor(EntityKind::Player, 0, 1, position);
    entity.entity.identity = OWN;
    entity
}

fn corpse(marker: u8, position: ActorPosition) -> WorldSpatialEntity {
    WorldSpatialEntity {
        display_name: None,
        kind: EntityKind::Corpse,
        entity: EntityRef {
            identity: [marker; 16],
            generation: 0,
        },
        position,
        detail: EntityDetail::Object {
            item_definition_ref: 9,
            quantity: 1,
            item_handle: None,
        },
    }
}

/// A snapshot of the own actor at (0, 0) and `others`.
fn entity_snapshot(others: Vec<WorldSpatialEntity>) -> (u32, Vec<u8>) {
    let mut entities = vec![own(at(0, 0))];
    entities.extend(others);
    let payload = encode_world_spatial_entities_snapshot(&WorldSpatialEntitiesSnapshot {
        content_generation: CONTENT_GENERATION,
        actor_position: at(0, 0),
        own_identity: OWN,
        entities,
    })
    .map_err(|error| format!("{error:?}"))
    .unwrap_or_default();
    (SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2, payload)
}

fn entities_delta(
    actor_position: ActorPosition,
    enter: Vec<WorldSpatialEntity>,
    update: Vec<WorldSpatialEntity>,
    leave: Vec<EntityRef>,
) -> WorldSpatialEntitiesDelta {
    WorldSpatialEntitiesDelta {
        content_generation: CONTENT_GENERATION,
        actor_position,
        enter,
        update,
        leave,
    }
}

fn entities_push(
    sequence: u64,
    base: u64,
    new: u64,
    delta: &WorldSpatialEntitiesDelta,
) -> Result<Step, BoxError> {
    Ok(Step::Send(encode_state_delta(
        1,
        sequence,
        STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
        base,
        new,
        DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
        &encode_world_spatial_entities_delta(delta).map_err(|error| format!("{error:?}"))?,
    )?))
}

fn entity_admission() -> Result<Admission<'static>, BoxError> {
    Ok(Admission {
        appearance_items: None,
        supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
        ..admission()?
    })
}

/// Joins with capability 6 selected and the given domain-1 snapshot, then plays `script`.
fn entity_peer(
    snapshot: (u32, Vec<u8>),
    selected: &'static [u32],
    script: Vec<Step>,
) -> (DuplexStream, tokio::task::JoinHandle<Result<(), BoxError>>) {
    let (client, mut server) = tokio::io::duplex(256 * 1024);
    let peer = tokio::spawn(async move {
        join_peer_with(
            &mut server,
            CLIENT_SUPPORTED_CAPABILITIES,
            selected,
            &[],
            snapshot,
            &[],
        )
        .await?;
        for step in script {
            play_step(&mut server, step).await?;
        }
        let mut rest = Vec::new();
        let _ = server.read_to_end(&mut rest).await;
        Ok::<(), BoxError>(())
    });
    (client, peer)
}

#[test]
fn the_advertised_set_is_closed_under_the_registry_requires() {
    // PROTOCOL_OTERYN_V1_REGISTRY.json `requires`: 4 needs 6, 12 needs 4, 14 needs 4 and 12.
    // CL25J extends the closure check for every newly advertised dependent mechanic.
    let requires: &[(u32, &[u32])] = &[
        (4, &[6]),
        (12, &[4]),
        (14, &[4, 12]),
        (36, &[7]),
        (152, &[14]),
        (153, &[37, 62]),
        (179, &[12, 14]),
        (230, &[14, 18]),
        (240, &[7, 42]),
        (258, &[7, 43]),
    ];
    for (capability, needed) in requires {
        if CLIENT_SUPPORTED_CAPABILITIES.contains(capability) {
            for need in *needed {
                assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(need));
            }
        }
    }
    assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&CAPABILITY_WORLD_SPATIAL_ENTITIES));
}

#[test]
fn a_type_2_snapshot_of_0_1_and_256_entities_is_stored_by_entity_ref() -> Result<(), BoxError> {
    block_on(async {
        for others in [0_usize, 1, MAX_SNAPSHOT_ENTITIES - 1] {
            let extra = (0..others)
                .map(|index| {
                    corpse(
                        u8::try_from(index % 200 + 1).unwrap_or(1),
                        at(i32::try_from(index).unwrap_or(0) + 1, 0),
                    )
                })
                .collect::<Vec<_>>();
            // Identities must be unique: spread the markers over two bytes.
            let extra = extra
                .into_iter()
                .enumerate()
                .map(|(index, mut entity)| {
                    entity.entity.identity[1] = u8::try_from(index / 200).unwrap_or(0);
                    entity
                })
                .collect::<Vec<_>>();
            let (client, peer) = entity_peer(entity_snapshot(extra), &[6], vec![]);
            let session = Session::admit(client, entity_admission()?).await?;
            let entities = session.world_entities().ok_or("capability 6 selected")?;
            assert_eq!(entities.len(), others + 1);
            assert_eq!(entities.own_identity(), &OWN);
            assert!(
                entities
                    .get(&EntityRef {
                        identity: OWN,
                        generation: 1
                    })
                    .is_some()
            );
            assert_eq!(session.world_spatial().actor_position, at(0, 0));
            drop(session);
            peer.await??;
        }
        Ok(())
    })?
}

#[test]
fn a_type_2_snapshot_is_refused_without_capability_6_and_type_1_with_it() -> Result<(), BoxError> {
    block_on(async {
        let (client, peer) = entity_peer(entity_snapshot(vec![]), &[13], vec![]);
        let refused = Session::admit(client, entity_admission()?).await;
        assert!(matches!(
            refused,
            Err(SessionError::UnregisteredSnapshotType {
                domain_id: 1,
                snapshot_type: 2
            })
        ));
        drop(peer);
        let (client, peer) =
            entity_peer((SNAPSHOT_TYPE_WORLD_SPATIAL_V1, spatial(0)), &[6], vec![]);
        let refused = Session::admit(client, entity_admission()?).await;
        assert!(matches!(
            refused,
            Err(SessionError::UnregisteredSnapshotType {
                domain_id: 1,
                snapshot_type: 1
            })
        ));
        drop(peer);
        Ok(())
    })?
}

#[test]
fn a_pushed_delta_enters_updates_and_removes_entities_and_is_queued() -> Result<(), BoxError> {
    block_on(async {
        let walker = actor(EntityKind::Creature, 1, 0, at(2, 0));
        let enter = entities_delta(at(0, 0), vec![walker, corpse(2, at(3, 0))], vec![], vec![]);
        let moved = actor(EntityKind::Creature, 1, 0, at(2, 1));
        let update = entities_delta(at(0, 0), vec![], vec![moved], vec![]);
        let leave = entities_delta(
            at(0, 0),
            vec![],
            vec![],
            vec![moved.entity, corpse(2, at(3, 0)).entity],
        );
        let (client, peer) = entity_peer(
            entity_snapshot(vec![]),
            &[6],
            vec![
                entities_push(41, 5, 6, &enter)?,
                entities_push(42, 6, 7, &update)?,
                entities_push(43, 7, 8, &leave)?,
            ],
        );
        let mut session = Session::admit(client, entity_admission()?).await?;
        session.service_liveness(Duration::from_millis(300)).await?;
        let entities = session.world_entities().ok_or("capability 6 selected")?;
        assert_eq!(entities.len(), 1);
        let events = session.take_events();
        assert_eq!(events.len(), 3);
        assert!(matches!(
            &events[1],
            SessionEvent::WorldSpatialEntities(delta)
                if delta.base_revision == 6 && delta.value == update
        ));
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn a_step_claims_the_entity_delta_and_moves_the_own_actor_in_both_views() -> Result<(), BoxError> {
    block_on(async {
        let moved = entities_delta(at(1, 0), vec![], vec![own(at(1, 0))], vec![]);
        let (client, peer) = entity_peer(
            entity_snapshot(vec![]),
            &[6, 13],
            vec![
                Step::ReadCommand,
                result_frame(41, 7, StepDisposition::Moved)?,
                entities_push(42, 5, 6, &moved)?,
            ],
        );
        let mut session = Session::admit(client, entity_admission()?).await?;
        let outcome = session.step(StepDirection::East).await?;
        let delta = outcome
            .world_spatial_delta
            .ok_or("Moved carries its delta")?;
        assert_eq!(
            (delta.new_revision, delta.value.actor_position),
            (6, at(1, 0))
        );
        assert_eq!(session.world_spatial().actor_position, at(1, 0));
        let entities = session.world_entities().ok_or("capability 6 selected")?;
        assert_eq!(
            entities
                .iter()
                .map(|entity| entity.position)
                .collect::<Vec<_>>(),
            vec![at(1, 0)]
        );
        // The claimed delta also reaches the queue so consumers see its entity lists.
        let events = session.take_events();
        assert!(matches!(
            events.as_slice(),
            [SessionEvent::WorldSpatialEntities(queued)]
                if queued.new_revision == 6 && queued.value == moved
        ));
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn a_step_delta_makes_entities_appear_move_and_leave_in_the_store_and_queue() -> Result<(), BoxError>
{
    block_on(async {
        let walker = actor(EntityKind::Creature, 7, 0, at(4, 0));
        let walker_moved = actor(EntityKind::Creature, 7, 0, at(5, 0));
        let stale = corpse(2, at(3, 0));
        let arrived = corpse(5, at(2, 0));
        let step_delta = entities_delta(
            at(1, 0),
            vec![arrived],
            vec![own(at(1, 0)), walker_moved],
            vec![stale.entity],
        );
        let (client, peer) = entity_peer(
            entity_snapshot(vec![walker, stale]),
            &[6, 13],
            vec![
                Step::ReadCommand,
                result_frame(41, 7, StepDisposition::Moved)?,
                entities_push(42, 5, 6, &step_delta)?,
            ],
        );
        let mut session = Session::admit(client, entity_admission()?).await?;
        session.step(StepDirection::East).await?;
        let stored = session.world_entities().ok_or("capability 6 selected")?;
        assert!(stored.get(&arrived.entity).is_some());
        assert!(stored.get(&stale.entity).is_none());
        assert_eq!(
            stored.get(&walker.entity).map(|entity| entity.position),
            Some(at(5, 0))
        );
        assert!(matches!(
            session.take_events().as_slice(),
            [SessionEvent::WorldSpatialEntities(queued)] if queued.value == step_delta
        ));
        drop(session);
        peer.await??;
        Ok(())
    })?
}

async fn assert_entity_push_poisons(
    selected: &'static [u32],
    push: Step,
    check: impl FnOnce(&SessionError) -> bool,
) -> Result<(), BoxError> {
    let (client, peer) = entity_peer(
        entity_snapshot(vec![corpse(2, at(3, 0))]),
        selected,
        vec![push],
    );
    let mut session = Session::admit(client, entity_admission()?).await?;
    let error = session
        .service_liveness(Duration::from_millis(200))
        .await
        .err()
        .ok_or("the push must fail closed")?;
    assert!(check(&error), "{error:?}");
    assert!(matches!(
        session.step(StepDirection::East).await,
        Err(SessionError::SessionUnusable)
    ));
    drop(session);
    peer.await??;
    Ok(())
}

#[test]
fn a_contradicting_entity_delta_poisons_the_session() -> Result<(), BoxError> {
    block_on(async {
        let inconsistent =
            |error: &SessionError| matches!(error, SessionError::EntityStoreInconsistent { .. });
        let stranger = actor(EntityKind::Creature, 7, 0, at(4, 0)).entity;
        // Base revision mismatch.
        let fine = entities_delta(at(0, 0), vec![], vec![], vec![]);
        assert_entity_push_poisons(&[6], entities_push(41, 4, 6, &fine)?, |error| {
            matches!(
                error,
                SessionError::StateRevisionMismatch { domain_id: 1, .. }
            )
        })
        .await?;
        // Leave and update of an entity that is not stored, enter of one that is.
        for delta in [
            entities_delta(at(0, 0), vec![], vec![], vec![stranger]),
            entities_delta(
                at(0, 0),
                vec![],
                vec![actor(EntityKind::Creature, 7, 0, at(4, 0))],
                vec![],
            ),
            entities_delta(at(0, 0), vec![corpse(2, at(5, 0))], vec![], vec![]),
        ] {
            assert_entity_push_poisons(&[6], entities_push(41, 5, 6, &delta)?, inconsistent)
                .await?;
        }
        // The own actor must agree with the header, and must not leave.
        for delta in [
            entities_delta(at(1, 0), vec![], vec![], vec![]),
            entities_delta(at(0, 0), vec![], vec![], vec![own(at(0, 0)).entity]),
        ] {
            assert_entity_push_poisons(&[6], entities_push(41, 5, 6, &delta)?, inconsistent)
                .await?;
        }
        // A v2 delta without capability 6 selected is an unregistered type.
        let (client, peer) = script_peer(vec![entities_push(41, 5, 6, &fine)?]);
        let mut session = Session::admit(client, admission()?).await?;
        let error = session
            .service_liveness(Duration::from_millis(200))
            .await
            .err()
            .ok_or("a v2 delta without capability 6 must fail closed")?;
        assert!(matches!(
            error,
            SessionError::UnregisteredDeltaType {
                domain_id: 1,
                delta_type: 2
            }
        ));
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn the_store_holds_exactly_256_entities_and_fails_closed_at_257() -> Result<(), BoxError> {
    block_on(async {
        let batch = |range: std::ops::Range<usize>| {
            range
                .map(|index| {
                    let mut entity = corpse(3, at(10, 0));
                    entity.entity.identity[1] = u8::try_from(index / 200).unwrap_or(0);
                    entity.entity.identity[2] = u8::try_from(index % 200).unwrap_or(0);
                    entity
                })
                .collect::<Vec<_>>()
        };
        // The own actor plus 255 enter exactly fills the store; one more is the 257th.
        let fill = entities_delta(at(0, 0), batch(0..255), vec![], vec![]);
        let over = entities_delta(at(0, 0), batch(255..256), vec![], vec![]);
        let (client, peer) = entity_peer(
            entity_snapshot(vec![]),
            &[6],
            vec![
                entities_push(41, 5, 6, &fill)?,
                entities_push(42, 6, 7, &over)?,
            ],
        );
        let mut session = Session::admit(client, entity_admission()?).await?;
        let error = session
            .service_liveness(Duration::from_millis(300))
            .await
            .err()
            .ok_or("the 257th entity must fail closed")?;
        assert!(matches!(
            error,
            SessionError::EntityStoreInconsistent { .. }
        ));
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn negotiated_names_survive_join_delta_and_visibility_leave() -> Result<(), BoxError> {
    block_on(async {
        use oteryn_protocol_oteryn::world_spatial_entities::{
            EntityFields, EntityLights, encode_world_spatial_entities_delta_with_fields,
            encode_world_spatial_entities_snapshot_with_fields,
        };
        let fields = EntityFields::selected(&[6, 286]);
        let mut player = own(at(0, 0));
        player.display_name = Some(EntityDisplayName::new("Player").map_err(|e| format!("{e:?}"))?);
        let mut rat = actor(EntityKind::Creature, 8, 1, at(1, 0));
        rat.display_name = Some(EntityDisplayName::new("Rat").map_err(|e| format!("{e:?}"))?);
        let snapshot = WorldSpatialEntitiesSnapshot {
            content_generation: CONTENT_GENERATION,
            actor_position: at(0, 0),
            own_identity: OWN,
            entities: vec![player, rat],
        };
        let bytes = encode_world_spatial_entities_snapshot_with_fields(
            &snapshot,
            fields,
            &EntityLights::new(),
        )
        .map_err(|e| format!("{e:?}"))?;
        rat.position.x = 2;
        rat.display_name = Some(EntityDisplayName::new("Cave Rat").map_err(|e| format!("{e:?}"))?);
        let delta = entities_delta(at(0, 0), vec![], vec![rat], vec![]);
        let delta_bytes =
            encode_world_spatial_entities_delta_with_fields(&delta, fields, &EntityLights::new())
                .map_err(|e| format!("{e:?}"))?;
        let (client, peer) = entity_peer((2, bytes), &[6, 286], vec![]);
        let mut session = Session::admit(client, entity_admission()?).await?;
        assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&286));
        assert_eq!(session.visible_appearances().len(), 2);
        let delta = oteryn_protocol_oteryn::encode_state_delta(1, 41, 1, 5, 6, 2, &delta_bytes)?;
        session.apply_state_delta(
            41,
            oteryn_protocol_oteryn::decode_wire_envelope(&delta)?.payload(),
        )?;
        let appearances = session.visible_appearances();
        let updated = appearances
            .iter()
            .find(|entry| entry.entity.entity == rat.entity)
            .ok_or("rat missing")?;
        assert_eq!(
            updated
                .entity
                .display_name
                .ok_or("name missing")?
                .as_str()
                .map_err(|e| format!("{e:?}"))?,
            "Cave Rat"
        );
        assert_eq!(updated.entity.position.x, 2);
        let leave = entities_delta(at(0, 0), vec![], vec![], vec![rat.entity]);
        let leave_bytes =
            encode_world_spatial_entities_delta_with_fields(&leave, fields, &EntityLights::new())
                .map_err(|e| format!("{e:?}"))?;
        let leave = oteryn_protocol_oteryn::encode_state_delta(1, 42, 1, 6, 7, 2, &leave_bytes)?;
        session.apply_state_delta(
            42,
            oteryn_protocol_oteryn::decode_wire_envelope(&leave)?.payload(),
        )?;
        assert_eq!(session.visible_appearances().len(), 1);
        drop(session);
        peer.await??;
        Ok(())
    })?
}
