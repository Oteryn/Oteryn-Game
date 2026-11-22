//! Replay M04's canonical entity projection from the actual pre-push snapshot.
//! Names, handles and native icon/object fields are retained; light and party-icon sidecars
//! and noncore frames are excluded. Native TLS capture tests cover transport sequencing.
//! This replay admits the baseline, then feeds the already-validated domain stream through
//! Session::apply_state_delta with its original revisions and possibly noncontiguous server
//! sequences. It never renumbers frames or changes Session's transport sequence counter.

use super::*;
use oteryn_protocol_oteryn::world_spatial_entities::{EntityFields, EntityLights};

struct Transition {
    sequence: u64,
    base: u64,
    new: u64,
    payload: Vec<u8>,
    icons_only: bool,
}

struct Fixture {
    snapshot: Vec<u8>,
    snapshot_revision: u64,
    snapshot_sequence: u64,
    transitions: Vec<Transition>,
    selected: Vec<u32>,
    target: EntityRef,
    landing: ActorPosition,
}

#[test]
fn captured_push_delta_updates_visible_player_and_invalid_update_is_atomic() -> Result<(), BoxError>
{
    run_fixture(fixture()?)
}

#[test]
fn core_capture_regression_m04_replay_preserves_complete_baseline_and_revision_chain()
-> Result<(), BoxError> {
    run_fixture(mixed_fixture(false)?)
}

#[test]
fn core_capture_regression_m04_replay_keeps_entity_fields_and_observed_revisions()
-> Result<(), BoxError> {
    run_fixture(mixed_fixture(true)?)
}

fn run_fixture(fixture: Fixture) -> Result<(), BoxError> {
    std::thread::Builder::new()
        .name("m04-domain-replay".into())
        .stack_size(64 << 20)
        .spawn(move || {
            block_on(async {
                tokio::time::timeout(Duration::from_secs(5), replay_fixture(fixture))
                    .await
                    .map_err(|_| "M04 domain replay deadline")?
            })?
        })?
        .join()
        .map_err(|_| "M04 domain replay panicked")?
}

async fn replay_fixture(fixture: Fixture) -> Result<(), BoxError> {
    validate_profile(&fixture.selected)?;
    let fields = EntityFields::selected(&fixture.selected);
    let (captured_baseline, lights) =
        world_spatial_entities::decode_world_spatial_entities_snapshot_with_fields(
            &fixture.snapshot,
            fields,
        )
        .map_err(|error| format!("M04 baseline decode: {error:?}"))?;
    assert!(lights.is_empty());
    let decoded = fixture
        .transitions
        .iter()
        .map(|transition| decode_delta(&transition.payload, &fixture.selected))
        .collect::<Result<Vec<_>, _>>()?;
    let last = decoded.last().ok_or("M04 capture has no core history")?;
    let moved = *last
        .update
        .iter()
        .find(|entity| entity.entity == fixture.target)
        .ok_or("M04 final spatial update omits pushed player")?;
    assert_eq!(moved.position, fixture.landing);
    assert!(
        !fixture
            .transitions
            .last()
            .ok_or("M04 core history")?
            .icons_only
    );
    assert_ne!(
        captured_baseline
            .entities
            .iter()
            .find(|entity| entity.entity == fixture.target)
            .ok_or("M04 baseline omits pushed player")?
            .position,
        fixture.landing
    );
    let baseline = captured_baseline.clone();
    let snapshot = world_spatial_entities::encode_world_spatial_entities_snapshot_with_fields(
        &baseline,
        fields,
        &EntityLights::new(),
    )
    .map_err(|error| format!("M04 replay baseline encode: {error:?}"))?;
    let selected = fixture.selected.clone();
    let snapshot_revision = fixture.snapshot_revision;
    let snapshot_sequence = fixture.snapshot_sequence;
    let (client, mut server) = tokio::io::duplex(256 * 1024);
    let peer = tokio::spawn(async move {
        let bootstrap = read_frame(&mut server).await?;
        assert_eq!(
            decode_wire_envelope(&bootstrap)?
                .client_bootstrap()?
                .supported_capabilities,
            CLIENT_SUPPORTED_CAPABILITIES
        );
        write_frame(
            &mut server,
            &encode_server_accepted(&ServerAcceptedValue {
                game_session_id: GameSessionId::decode(&uuid_v7(1))?,
                world_id: WorldId::decode(&uuid_v7(2))?,
                channel_id: ChannelId::decode(&uuid_v7(3))?,
                connection_generation: 1,
                current_server_sequence: snapshot_sequence,
                next_command_id: 7,
                schema_revision: 1,
                selected_capabilities: &selected,
            })?,
        )
        .await?;
        let overlay = encode_world_object_overlay_snapshot(&[])
            .map_err(|error| format!("M04 replay overlay: {error:?}"))?;
        for frame in encode_single_chunk_snapshot(
            1,
            1,
            snapshot_sequence,
            &[
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: snapshot_revision,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                    payload: &snapshot,
                },
                DomainSnapshot {
                    domain_id:
                        oteryn_protocol_oteryn::world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                    revision: 0,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                    payload: &overlay,
                },
            ],
        )? {
            write_frame(&mut server, &frame).await?;
        }
        let mut rest = Vec::new();
        let _ = server.read_to_end(&mut rest).await;
        Ok::<(), BoxError>(())
    });
    let mut session = Session::admit(client, entity_admission()?).await?;
    let visible = session
        .world_entities()
        .ok_or("M04 admitted entity store")?;
    assert_eq!(visible.own_identity(), &captured_baseline.own_identity);
    assert_eq!(
        visible.iter().copied().collect::<Vec<_>>(),
        captured_baseline.entities
    );
    assert_eq!(session.spatial_revision, fixture.snapshot_revision);
    let mut previous_sequence = fixture.snapshot_sequence;
    for (transition, value) in fixture.transitions.iter().zip(&decoded) {
        assert!(transition.sequence > previous_sequence);
        assert_eq!(transition.base, session.spatial_revision);
        if transition.icons_only {
            assert!(value.enter.is_empty() && value.update.is_empty() && value.leave.is_empty());
            assert_eq!(value.actor_position, session.world_spatial().actor_position);
        }
        let before = session
            .world_entities()
            .cloned()
            .ok_or("M04 before entity store")?;
        let frame = encode_state_delta(
            1,
            transition.sequence,
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            transition.base,
            transition.new,
            DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
            &transition.payload,
        )?;
        let envelope = decode_wire_envelope(&frame)?;
        envelope.validate(oteryn_protocol_oteryn::Direction::ServerToClient, true)?;
        assert_eq!(envelope.server_sequence(), transition.sequence);
        let event = session.apply_state_delta(transition.sequence, envelope.payload())?;
        assert_eq!(
            event,
            SessionEvent::WorldSpatialEntities(AppliedDelta {
                server_sequence: transition.sequence,
                base_revision: transition.base,
                new_revision: transition.new,
                value: value.clone(),
            })
        );
        assert_eq!(session.spatial_revision, transition.new);
        let after = session.world_entities().ok_or("M04 after entity store")?;
        assert_eq!(after.own_identity(), &captured_baseline.own_identity);
        assert_eq!(
            after.len(),
            before.len() - value.leave.len() + value.enter.len()
        );
        for reference in &value.leave {
            assert!(after.get(reference).is_none());
        }
        for entity in value.enter.iter().chain(&value.update) {
            assert_eq!(after.get(&entity.entity), Some(entity));
        }
        for entity in before.iter().filter(|entity| {
            !value.leave.contains(&entity.entity)
                && !value
                    .update
                    .iter()
                    .any(|updated| updated.entity == entity.entity)
        }) {
            assert_eq!(after.get(&entity.entity), Some(entity));
        }
        previous_sequence = transition.sequence;
    }
    assert_eq!(
        session
            .world_entities()
            .and_then(|view| view.get(&fixture.target)),
        Some(&moved)
    );
    // Direct domain replay preserves, rather than fabricates, the omitted transport traffic.
    assert_eq!(session.last_server_sequence(), fixture.snapshot_sequence);
    let before = session
        .world_entities()
        .cloned()
        .ok_or("M04 final entity store")?;
    let before_revision = session.spatial_revision;
    let before_position = session.world_spatial().actor_position;
    let mut unknown = moved;
    unknown.entity.generation = unknown
        .entity
        .generation
        .checked_add(1)
        .ok_or("M04 unknown generation")?;
    assert!(before.get(&unknown.entity).is_none());
    let invalid = WorldSpatialEntitiesDelta {
        content_generation: captured_baseline.content_generation,
        actor_position: before_position,
        enter: vec![],
        update: vec![unknown],
        leave: vec![],
    };
    // Use the same negotiated codec and no inherited enter/leave operators: only this exact
    // unknown entity may cause the rejection, after native decoding succeeds.
    let invalid_payload = world_spatial_entities::encode_world_spatial_entities_delta_with_fields(
        &invalid,
        fields,
        &EntityLights::new(),
    )
    .map_err(|error| format!("M04 invalid fixture encode: {error:?}"))?;
    assert_eq!(
        world_spatial_entities::decode_world_spatial_entities_delta_with_fields(
            &invalid_payload,
            fields,
        )
        .map_err(|error| format!("M04 invalid fixture decode: {error:?}"))?
        .0,
        invalid
    );
    let invalid_frame = encode_state_delta(
        1,
        previous_sequence + 1,
        STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
        before_revision,
        before_revision + 1,
        DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
        &invalid_payload,
    )?;
    let error = session
        .apply_state_delta(
            previous_sequence + 1,
            decode_wire_envelope(&invalid_frame)?.payload(),
        )
        .err()
        .ok_or("M04 unknown entity update must be rejected")?;
    assert!(
        matches!(
            error,
            SessionError::EntityStoreInconsistent {
                reason: "update of an entity that is not stored"
            }
        ),
        "{error:?}"
    );
    assert_eq!(session.world_entities(), Some(&before));
    assert_eq!(session.spatial_revision, before_revision);
    assert_eq!(session.world_spatial().actor_position, before_position);
    drop(session);
    peer.await??;
    Ok(())
}

fn decode_delta(payload: &[u8], selected: &[u32]) -> Result<WorldSpatialEntitiesDelta, BoxError> {
    let (value, lights) = world_spatial_entities::decode_world_spatial_entities_delta_with_fields(
        payload,
        EntityFields::selected(selected),
    )
    .map_err(|error| format!("M04 decode: {error:?}"))?;
    assert!(lights.is_empty());
    Ok(value)
}

fn fixture() -> Result<Fixture, BoxError> {
    let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return mixed_fixture(false);
    };
    let directory = std::path::Path::new(&directory);
    let baseline = std::fs::read_to_string(directory.join("m04-baseline.txt"))?;
    let numbers = baseline
        .split_whitespace()
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()?;
    let [snapshot_revision, snapshot_sequence] = numbers.as_slice() else {
        return Err("invalid M04 baseline metadata".into());
    };
    let expected = std::fs::read_to_string(directory.join("m04-expected.txt"))?;
    let fields = expected.split_whitespace().collect::<Vec<_>>();
    let [identity, generation, x, y, floor] = fields.as_slice() else {
        return Err("invalid M04 expected capture".into());
    };
    let selected = std::fs::read_to_string(directory.join("m04-capabilities.txt"))?
        .split_whitespace()
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>()?;
    let mut transitions = Vec::new();
    for (index, line) in std::fs::read_to_string(directory.join("m04-history.txt"))?
        .lines()
        .enumerate()
    {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        let [sequence, base, new, kind] = fields.as_slice() else {
            return Err("invalid M04 spatial history metadata".into());
        };
        let icons_only = match *kind {
            "spatial" => false,
            "icons-projection" => true,
            _ => return Err("invalid M04 canonical transition kind".into()),
        };
        transitions.push(Transition {
            sequence: sequence.parse()?,
            base: base.parse()?,
            new: new.parse()?,
            icons_only,
            payload: std::fs::read(directory.join(format!("m04-spatial-{index}.bin")))?,
        });
    }
    Ok(Fixture {
        snapshot: std::fs::read(directory.join("m04-baseline.bin"))?,
        snapshot_revision: *snapshot_revision,
        snapshot_sequence: *snapshot_sequence,
        transitions,
        selected,
        target: EntityRef {
            identity: decode_identity(identity)?,
            generation: generation.parse()?,
        },
        landing: ActorPosition {
            x: x.parse()?,
            y: y.parse()?,
            floor: floor.parse()?,
        },
    })
}

fn mixed_fixture(rich: bool) -> Result<Fixture, BoxError> {
    let selected = if rich {
        vec![4, 6, 286, 372, 386]
    } else {
        vec![6]
    };
    let fields = EntityFields::selected(&selected);
    let make_actor = |kind, marker, position| -> Result<WorldSpatialEntity, BoxError> {
        let mut entity = actor(kind, marker, 1, position);
        if rich {
            entity.display_name = Some(
                oteryn_protocol_oteryn::entity_names::EntityDisplayName::new("Captured Actor")
                    .map_err(|error| format!("M04 fixture name: {error:?}"))?,
            );
        }
        Ok(entity)
    };
    // This actual own identity differs from the generic tests' OWN constant.
    let player = make_actor(EntityKind::Player, 0x5a, at(0, 0))?;
    let target = make_actor(EntityKind::Player, 1, at(1, 0))?;
    let bystander = make_actor(EntityKind::Player, 2, at(2, 0))?;
    let mut creature = make_actor(EntityKind::Creature, 3, at(2, 1))?;
    if rich {
        creature.detail = EntityDetail::ActorWithIcons {
            direction: StepDirection::South,
            appearance_ref: 2,
            health_percent: 88,
            icons: world_spatial_entities::NativeCreatureIcons::new(&[
                world_spatial_entities::NativeCreatureIcon {
                    category: 1,
                    id: 6,
                    count: 1,
                },
            ])
            .map_err(|error| format!("M04 fixture icons: {error:?}"))?,
        };
    }
    let ground = |marker, handle| -> Result<WorldSpatialEntity, BoxError> {
        let detail = if rich {
            EntityDetail::ObjectWithAppearance {
                item_definition_ref: 3031,
                quantity: 7,
                item_handle: Some(ItemHandle::new(handle).ok_or("M04 fixture handle")?),
                appearance_id: std::num::NonZeroU16::new(3031).ok_or("M04 fixture appearance")?,
            }
        } else {
            EntityDetail::Object {
                item_definition_ref: 3031,
                quantity: 7,
                item_handle: None,
            }
        };
        Ok(WorldSpatialEntity {
            display_name: None,
            kind: EntityKind::GroundItem,
            entity: EntityRef {
                identity: [marker; 16],
                generation: 0,
            },
            position: at(2, 2),
            detail,
        })
    };
    let removed = ground(4, 9)?;
    let entered = ground(5, 11)?;
    let mut entities = vec![player, target, bystander, creature, removed];
    entities.sort_by_key(|entity| entity.entity);
    let baseline = WorldSpatialEntitiesSnapshot {
        content_generation: CONTENT_GENERATION,
        actor_position: player.position,
        own_identity: player.entity.identity,
        entities,
    };
    let mut moved_own = player;
    moved_own.position = at(0, 1);
    let mut moved_bystander = bystander;
    moved_bystander.position = at(2, 3);
    let mut moved_target = target;
    moved_target.position = at(1, 1);
    let mut changed_creature = creature;
    if rich {
        changed_creature.detail = EntityDetail::ActorWithIcons {
            direction: StepDirection::South,
            appearance_ref: 2,
            health_percent: 88,
            icons: world_spatial_entities::NativeCreatureIcons::new(&[
                world_spatial_entities::NativeCreatureIcon {
                    category: 1,
                    id: 6,
                    count: 5,
                },
            ])
            .map_err(|error| format!("M04 fixture icons: {error:?}"))?,
        };
    } else {
        changed_creature.position = at(2, 4);
    }
    let values = [
        WorldSpatialEntitiesDelta {
            content_generation: CONTENT_GENERATION,
            actor_position: player.position,
            enter: vec![],
            update: vec![],
            leave: vec![],
        },
        WorldSpatialEntitiesDelta {
            content_generation: CONTENT_GENERATION,
            actor_position: moved_own.position,
            enter: vec![],
            update: vec![moved_own, moved_bystander],
            leave: vec![removed.entity],
        },
        WorldSpatialEntitiesDelta {
            content_generation: CONTENT_GENERATION,
            actor_position: moved_own.position,
            enter: vec![entered],
            update: vec![moved_target, changed_creature],
            leave: vec![],
        },
    ];
    let mut transitions = Vec::new();
    // Gaps deliberately represent omitted, already-validated noncore traffic, not lost core revisions.
    for ((index, value), sequence) in values.into_iter().enumerate().zip([81, 84, 87]) {
        transitions.push(Transition {
            sequence,
            base: 27 + u64::try_from(index)?,
            new: 28 + u64::try_from(index)?,
            icons_only: index == 0,
            payload: world_spatial_entities::encode_world_spatial_entities_delta_with_fields(
                &value,
                fields,
                &EntityLights::new(),
            )
            .map_err(|error| format!("M04 fixture delta encode: {error:?}"))?,
        });
    }
    Ok(Fixture {
        snapshot: world_spatial_entities::encode_world_spatial_entities_snapshot_with_fields(
            &baseline,
            fields,
            &EntityLights::new(),
        )
        .map_err(|error| format!("M04 fixture baseline encode: {error:?}"))?,
        snapshot_revision: 27,
        snapshot_sequence: 80,
        transitions,
        selected,
        target: target.entity,
        landing: moved_target.position,
    })
}

fn validate_profile(selected: &[u32]) -> Result<(), BoxError> {
    if !selected.contains(&6)
        || selected
            .iter()
            .any(|capability| ![4, 6, 286, 372, 386].contains(capability))
        || selected.windows(2).any(|pair| pair[0] >= pair[1])
        || (selected.contains(&386) && !selected.contains(&4))
    {
        return Err("M04 capture has an invalid entity projection profile".into());
    }
    Ok(())
}

fn decode_identity(value: &str) -> Result<[u8; 16], BoxError> {
    if value.len() != 32 {
        return Err("M04 identity length".into());
    }
    let mut identity = [0; 16];
    for (index, byte) in identity.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)?;
    }
    Ok(identity)
}
