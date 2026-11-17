mod appearance_fixture;
mod item_dictionary_tests;
mod native_icons;
use super::*;
use oteryn_protocol_oteryn::appearance;
const GOLDEN: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../protocol-oteryn/tests/fixtures/appearance_snapshot_v1.bin"
));

#[test]
fn appearance_result_first_and_observation_first_converge_through_session_dispatch()
-> Result<(), BoxError> {
    block_on(async {
        for (result_first, with_forge) in
            [(true, false), (false, false), (true, true), (false, true)]
        {
            let (mut snapshot, entity, _) = appearance_fixture::snapshot()?;
            snapshot.bindings[0].forge_stacks = Some(5);
            // Forge markers describe creatures and require independent capability 104.
            // Keep the own-player outfit ordering checks and exercise both projections.
            let creature = WorldSpatialEntity {
                kind: EntityKind::Creature,
                entity: EntityRef {
                    identity: [0xb0; 16],
                    ..entity.entity
                },
                detail: EntityDetail::Actor {
                    direction: StepDirection::South,
                    appearance_ref: 2,
                    health_percent: 100,
                },
                ..entity
            };
            let mut creature_binding = snapshot.bindings[0].clone();
            creature_binding.entity = creature.entity;
            creature_binding.appearance_ref = 2;
            snapshot.bindings.push(creature_binding.clone());
            snapshot.high_water = 2;
            let spatial = (
                SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                encode_world_spatial_entities_snapshot(&WorldSpatialEntitiesSnapshot {
                    content_generation: CONTENT_GENERATION,
                    actor_position: at(0, 0),
                    own_identity: OWN,
                    entities: vec![entity, creature],
                })
                .map_err(|e| format!("{e:?}"))?,
            );
            let domain = (21, 1, 1, snapshot.encode().map_err(|e| format!("{e:?}"))?);
            let original = snapshot.bindings[0].look.clone();
            let mut binding = snapshot.bindings[0].clone();
            binding.root_revision = 2;
            binding.forge_stacks = Some(15);
            creature_binding.root_revision = 2;
            creature_binding.forge_stacks = Some(15);
            binding.look = appearance::Look::Outfit {
                look_type: 129,
                colours: [2, 1, 0, 2],
                addons: 0,
            };
            let expected = binding.look.clone();
            let mapping = appearance::Mappings {
                context: snapshot.catalogue.context,
                upserts: vec![binding, creature_binding],
                forgets: Vec::new(),
            }
            .encode()
            .map_err(|e| format!("{e:?}"))?;
            let (client, mut server) = tokio::io::duplex(256 * 1024);
            let peer = tokio::spawn(async move {
                join_peer_with(
                    &mut server,
                    CLIENT_SUPPORTED_CAPABILITIES,
                    if with_forge { &[6, 25, 104] } else { &[6, 25] },
                    &[],
                    spatial,
                    &[domain],
                )
                .await?;
                read_frame(&mut server).await?;
                let start = if result_first { 42 } else { 41 };
                let result = encode_command_result(
                    1,
                    if result_first { 41 } else { 43 },
                    7,
                    CommandStatus::Accepted,
                    &appearance::ChangeResult::Committed(2)
                        .encode()
                        .map_err(|e| format!("{e:?}"))?,
                )?;
                if result_first {
                    write_frame(&mut server, &result).await?;
                }
                write_frame(
                    &mut server,
                    &encode_state_delta(1, start, 21, 1, 2, 1, &mapping)?,
                )
                .await?;
                let spatial = entities_delta(at(0, 0), Vec::new(), vec![entity], Vec::new());
                play_step(&mut server, entities_push(start + 1, 5, 6, &spatial)?).await?;
                if !result_first {
                    write_frame(&mut server, &result).await?;
                }
                let mut rest = Vec::new();
                server.read_to_end(&mut rest).await?;
                Ok::<_, BoxError>(())
            });
            let mut session = Session::admit(client, entity_admission()?).await?;
            assert_eq!(session.visible_appearances()[0].forge_stacks, None);
            assert_eq!(
                session
                    .visible_appearances()
                    .iter()
                    .find(|entry| entry.entity.entity == creature.entity)
                    .ok_or("missing visible creature")?
                    .forge_stacks,
                with_forge.then_some(5)
            );
            assert_eq!(
                session.visible_appearances()[0].look.as_ref(),
                Some(&original)
            );
            let change = appearance::Change {
                context: snapshot.catalogue.context,
                expected_root_revision: 1,
                prior: None,
                selection: appearance::Selection {
                    outfit_key: "oteryn:outfit.citizen".into(),
                    colours: [2, 1, 0, 2],
                    addons: 0,
                    mount_key: None,
                },
                content_revision: "r1".into(),
            };
            let outcome = session
                .gated_command(25, 25, &change.encode().map_err(|e| format!("{e:?}"))?)
                .await?;
            assert_eq!(
                appearance::ChangeResult::decode(&outcome.payload).map_err(|e| format!("{e:?}"))?,
                appearance::ChangeResult::Committed(2)
            );
            if result_first {
                assert_eq!(
                    *session
                        .appearance
                        .as_ref()
                        .ok_or("state")?
                        .resolve(entity.entity, 1)
                        .ok_or("binding")?,
                    original
                );
            }
            session.service_liveness(Duration::from_millis(50)).await?;
            assert_eq!(session.visible_appearances()[0].forge_stacks, None);
            assert_eq!(
                session
                    .visible_appearances()
                    .iter()
                    .find(|entry| entry.entity.entity == creature.entity)
                    .ok_or("missing visible creature")?
                    .forge_stacks,
                with_forge.then_some(15)
            );
            assert_eq!(
                *session
                    .appearance
                    .as_ref()
                    .ok_or("state")?
                    .resolve(entity.entity, 1)
                    .ok_or("binding")?,
                expected
            );
            assert_eq!(
                session.visible_appearances()[0].look.as_ref(),
                Some(&expected)
            );
            assert!(matches!(
                session.take_events().as_slice(),
                [
                    SessionEvent::Appearance(_),
                    SessionEvent::WorldSpatialEntities(_)
                ]
            ));
            drop(session);
            peer.await??;
        }
        Ok(())
    })?
}
