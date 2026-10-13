use super::*;
use crate::map_view::{
    MapDefinition, MapItem, MapOrigin, MapTile, MapViewHeader, WorldMapViewDelta,
    WorldMapViewSnapshot,
};
use std::num::NonZeroU32;

const SELECTED: &[u32] = &[4, 6, 18];

fn header() -> MapViewHeader {
    MapViewHeader {
        content_generation: CONTENT_GENERATION,
        bundle_digest: [2; 32],
        reset_epoch: 1,
        origin: at(0, 0),
        first_visible_floor: 0,
    }
}

fn map_snapshot() -> Result<Vec<u8>, BoxError> {
    world_map::encode_world_map_snapshot(&WorldMapViewSnapshot {
        header: header(),
        tiles: vec![MapTile {
            position: at(0, 0),
            items: vec![MapItem {
                definition: MapDefinition::Item(NonZeroU32::MIN),
                count: 1,
                sub_type: 0,
                origin: MapOrigin::DisplayOnly,
                appearance_id: 0,
            }],
            more: false,
            ground_speed: 150,
        }],
    })
    .map_err(|e| format!("{e:?}").into())
}

fn map_peer(
    selected: &'static [u32],
    map: Option<Vec<u8>>,
    overlay: bool,
    tail: Option<Vec<u8>>,
    resync: bool,
) -> (DuplexStream, tokio::task::JoinHandle<Result<(), BoxError>>) {
    map_peer_script(selected, map, overlay, tail, resync, None)
}

fn map_peer_script(
    selected: &'static [u32],
    map: Option<Vec<u8>>,
    overlay: bool,
    tail: Option<Vec<u8>>,
    resync: bool,
    expected_use: Option<Vec<u8>>,
) -> (DuplexStream, tokio::task::JoinHandle<Result<(), BoxError>>) {
    let (client, mut server) = tokio::io::duplex(256 * 1024);
    let peer = tokio::spawn(async move {
        let result_sequence = if tail.is_some() { 42 } else { 41 };
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
                current_server_sequence: 0,
                next_command_id: 7,
                schema_revision: 1,
                selected_capabilities: selected,
            })?,
        )
        .await?;
        let (spatial_type, spatial) = entity_snapshot(vec![]);
        let objects = encode_world_object_overlay_snapshot(&[]).map_err(|e| format!("{e:?}"))?;
        let mut domains = vec![DomainSnapshot {
            domain_id: 1,
            revision: 5,
            snapshot_type: spatial_type,
            payload: &spatial,
        }];
        if let Some(payload) = &map {
            domains.push(DomainSnapshot {
                domain_id: 17,
                revision: 9,
                snapshot_type: 1,
                payload,
            });
        }
        if overlay {
            domains.push(DomainSnapshot {
                domain_id: 2,
                revision: 2,
                snapshot_type: 1,
                payload: &objects,
            });
        }
        for frame in encode_single_chunk_snapshot(1, 1, 40, &domains)? {
            write_frame(&mut server, &frame).await?;
        }
        if let Some(frame) = tail {
            write_frame(&mut server, &frame).await?;
        }
        if resync {
            let request = read_frame(&mut server).await?;
            assert_eq!(
                request,
                oteryn_protocol_oteryn::encode_resync_request(1, 40, &[(17, 9)])?
            );
            decode_wire_envelope(&request)?.validate(Direction::ClientToServer, true)?;
        }
        if let Some(expected) = expected_use {
            let frame = read_frame(&mut server).await?;
            let command = decode_wire_envelope(&frame)?.client_command(1)?;
            assert_eq!((command.command_id, command.command_type), (7, 2));
            assert_eq!(command.payload, expected);
            write_frame(
                &mut server,
                &encode_command_result(
                    1,
                    result_sequence,
                    7,
                    CommandStatus::Accepted,
                    &world_object::encode_use_result(UseDisposition::Committed),
                )?,
            )
            .await?;
        }
        let mut rest = Vec::new();
        let _ = server.read_to_end(&mut rest).await;
        Ok(())
    });
    (client, peer)
}

fn map_admission() -> Result<Admission<'static>, BoxError> {
    entity_admission()
}

#[test]
fn production_bootstrap_selects_map_or_retains_the_declined_legacy_domain() -> Result<(), BoxError>
{
    block_on(async {
        for selected_map in [true, false] {
            let (client, peer) = map_peer(
                if selected_map { SELECTED } else { &[4, 6] },
                if selected_map {
                    Some(map_snapshot()?)
                } else {
                    None
                },
                !selected_map,
                None,
                false,
            );
            let session = Session::admit(client, entity_admission()?).await?;
            assert_eq!(session.selected_capabilities().contains(&18), selected_map);
            assert_eq!(session.world_map_view().is_some(), selected_map);
            assert_eq!(session.join_snapshot().world_map.is_some(), selected_map);
            assert_eq!(
                session.join_snapshot().world_map_revision,
                if selected_map { 9 } else { 0 }
            );
            drop(session);
            peer.await??;
        }
        Ok(())
    })?
}

#[test]
fn live_map_origin_selects_exact_use_payload_and_stale_target_sends_nothing() -> Result<(), BoxError>
{
    block_on(async {
        use crate::map_view::MapTarget;
        let handle = std::num::NonZeroU64::MIN;
        for origin in [
            MapOrigin::BaseOrdinal {
                ordinal: 7,
                object_revision: 23,
            },
            MapOrigin::Handle(handle),
        ] {
            let mut snapshot = world_map::decode_world_map_snapshot(&map_snapshot()?)
                .map_err(|e| format!("{e:?}"))?;
            snapshot.tiles[0].items[0].origin = origin;
            let mut model =
                crate::map_view::MapView::new(CONTENT_GENERATION).map_err(|e| format!("{e:?}"))?;
            model
                .replace_decoded(snapshot.clone())
                .map_err(|e| format!("{e:?}"))?;
            let target = model.target(at(0, 0), 0).ok_or("no map target")?;
            let expected = match target {
                MapTarget::Base {
                    placement,
                    expected_revision,
                } => world_object::encode_use_intent(&WorldObjectTarget {
                    placement: placement.to_vec(),
                    expected_revision,
                })
                .map_err(|e| format!("{e:?}"))?,
                MapTarget::Item(handle) => world_object::encode_use_item_intent(handle),
            };
            let bytes =
                world_map::encode_world_map_snapshot(&snapshot).map_err(|e| format!("{e:?}"))?;
            let (client, peer) =
                map_peer_script(SELECTED, Some(bytes), false, None, false, Some(expected));
            let mut session = Session::admit(client, map_admission()?).await?;
            let stale = match target {
                MapTarget::Base {
                    placement,
                    expected_revision,
                } => MapTarget::Base {
                    placement,
                    expected_revision: expected_revision + 1,
                },
                MapTarget::Item(_) => {
                    MapTarget::Item(std::num::NonZeroU64::new(2).ok_or("zero handle")?)
                }
            };
            assert!(session.use_map_target(stale).await?.is_none());
            let outcome = session
                .use_map_target(target)
                .await?
                .ok_or("use disappeared")?;
            assert_eq!(outcome.disposition, UseDisposition::Committed);
            assert_eq!(outcome.command_id, 7); // stale input reserved no ID and emitted no frame
            drop(session);
            peer.await??;
        }
        Ok(())
    })?
}

#[test]
fn header_only_roof_delta_invalidates_queued_use_without_reserving_a_command_id()
-> Result<(), BoxError> {
    block_on(async {
        let lower = std::num::NonZeroU64::MIN;
        let upper = std::num::NonZeroU64::new(2).ok_or("zero handle")?;
        let mut snapshot =
            world_map::decode_world_map_snapshot(&map_snapshot()?).map_err(|e| format!("{e:?}"))?;
        snapshot.header.origin.floor = -1;
        snapshot.header.first_visible_floor = -1;
        snapshot.tiles[0].position.floor = -1;
        snapshot.tiles[0].items[0].origin = MapOrigin::Handle(lower);
        let mut roof = snapshot.tiles[0].clone();
        roof.position = ActorPosition {
            x: 1,
            y: 1,
            floor: 0,
        };
        roof.items[0].origin = MapOrigin::Handle(upper);
        snapshot.tiles.push(roof);
        let mut next = snapshot.header;
        next.first_visible_floor = 0;
        let delta = world_map::encode_world_map_delta(
            &snapshot.header,
            &WorldMapViewDelta {
                header: next,
                cleared: vec![],
                tiles: vec![],
            },
        )
        .map_err(|e| format!("{e:?}"))?;
        let bytes =
            world_map::encode_world_map_snapshot(&snapshot).map_err(|e| format!("{e:?}"))?;
        let (client, peer) = map_peer_script(
            SELECTED,
            Some(bytes),
            false,
            Some(encode_state_delta(1, 41, 17, 9, 10, 1, &delta)?),
            false,
            Some(world_object::encode_use_item_intent(upper)),
        );
        let mut session = Session::admit(client, map_admission()?).await?;
        session.service_liveness(Duration::from_millis(50)).await?;
        assert!(
            session
                .use_map_target(crate::map_view::MapTarget::Item(lower))
                .await?
                .is_none()
        );
        let outcome = session
            .use_map_target(crate::map_view::MapTarget::Item(upper))
            .await?
            .ok_or("visible roof target disappeared")?;
        assert_eq!(outcome.command_id, 7);
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn selected_map_replaces_object_domain_and_routes_typed_pushed_delta() -> Result<(), BoxError> {
    block_on(async {
        let mut next = header();
        next.origin.x = 1;
        let value = WorldMapViewDelta {
            header: next,
            tiles: vec![MapTile {
                position: at(1, 0),
                items: vec![MapItem {
                    definition: MapDefinition::Item(NonZeroU32::MIN),
                    count: 1,
                    sub_type: 0,
                    origin: MapOrigin::DisplayOnly,
                    appearance_id: 0,
                }],
                more: false,
                ground_speed: 137,
            }],
            cleared: vec![at(0, 0)],
        };
        let payload =
            world_map::encode_world_map_delta(&header(), &value).map_err(|e| format!("{e:?}"))?;
        let frame = encode_state_delta(1, 41, 17, 9, 10, 1, &payload)?;
        let (client, peer) = map_peer(SELECTED, Some(map_snapshot()?), false, Some(frame), false);
        let mut session = Session::admit(client, map_admission()?).await?;
        assert_eq!(
            session.join_snapshot().world_map.as_ref().map(|s| s.header),
            Some(header())
        );
        assert_eq!(session.join_snapshot().world_map_revision, 9);
        assert!(session.world_object_overlay().is_empty());
        session.service_liveness(Duration::from_millis(50)).await?;
        assert_eq!(
            session.world_map_view().and_then(|map| map.header()),
            Some(next)
        );
        assert_eq!(session.map_revision, 10);
        let map = session.world_map_view().ok_or("map disappeared")?;
        assert!(map.tile(at(0, 0)).is_none());
        assert_eq!(map.ground_speed(at(1, 0)), Some(137));
        assert_eq!(map.snapshot().ok_or("no map snapshot")?.tiles, value.tiles);
        assert!(
            matches!(session.take_events().as_slice(), [SessionEvent::WorldMapView(delta)]
            if delta.value == value && delta.base_revision == 9 && delta.new_revision == 10 && delta.server_sequence == 41)
        );
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn map_admission_rejects_missing_map_mixed_domains_and_missing_dependencies() -> Result<(), BoxError>
{
    block_on(async {
        for (selected, missing, overlay) in [
            (SELECTED, true, false),
            (SELECTED, false, true),
            (&[6, 18][..], false, false),
        ] {
            let (client, peer) = map_peer(
                selected,
                if missing { None } else { Some(map_snapshot()?) },
                overlay,
                None,
                false,
            );
            let result = Session::admit(client, map_admission()?).await;
            if missing {
                assert!(matches!(result, Err(SessionError::MissingDomain(17))));
            } else if overlay {
                assert!(matches!(
                    result,
                    Err(SessionError::UnselectedDomain { domain_id: 2 })
                ));
            } else {
                assert!(matches!(
                    result,
                    Err(SessionError::CapabilityNotSelected { capability: 4 })
                ));
            }
            drop(result);
            let _ = peer.await?;
        }
        Ok(())
    })?
}

#[test]
fn unselected_or_wrong_generation_map_snapshot_is_refused() -> Result<(), BoxError> {
    block_on(async {
        for selected in [&[4, 6][..], SELECTED] {
            let mut wrong = header();
            wrong.content_generation = [3; 32];
            let payload = world_map::encode_world_map_snapshot(&WorldMapViewSnapshot {
                header: wrong,
                tiles: vec![],
            })
            .map_err(|e| format!("{e:?}"))?;
            let (client, peer) = map_peer(selected, Some(payload), false, None, false);
            let result = Session::admit(client, map_admission()?).await;
            if selected == SELECTED {
                assert!(matches!(
                    result,
                    Err(SessionError::ContentGenerationMismatch { domain_id: 17 })
                ));
            } else {
                assert!(matches!(
                    result,
                    Err(SessionError::UnselectedDomain { domain_id: 17 })
                ));
            }
            drop(result);
            let _ = peer.await?;
        }
        Ok(())
    })?
}

#[test]
fn refused_map_revision_invalidates_all_map_targets() -> Result<(), BoxError> {
    block_on(async {
        let mut next = header();
        next.origin.x = 1;
        let payload = world_map::encode_world_map_delta(
            &header(),
            &WorldMapViewDelta {
                header: next,
                tiles: vec![],
                cleared: vec![],
            },
        )
        .map_err(|e| format!("{e:?}"))?;
        let frame = encode_state_delta(1, 41, 17, 8, 9, 1, &payload)?;
        let (client, peer) = map_peer(SELECTED, Some(map_snapshot()?), false, Some(frame), false);
        let mut session = Session::admit(client, map_admission()?).await?;
        assert!(matches!(
            session.service_liveness(Duration::from_millis(50)).await,
            Err(SessionError::StateRevisionMismatch { domain_id: 17, .. })
        ));
        assert!(session.world_map_view().is_none());
        assert!(session.join_snapshot().world_map.is_none());
        assert!(session.take_events().is_empty());
        assert_eq!(session.world_map_revision(), 9);
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn malformed_map_origin_requests_resync_and_cannot_reuse_old_targets() -> Result<(), BoxError> {
    block_on(async {
        let mut value =
            world_map::decode_world_map_snapshot(&map_snapshot()?).map_err(|e| format!("{e:?}"))?;
        value.tiles[0].position = at(1, 0);
        let mut payload = world_map::encode_world_map_delta(
            &header(),
            &WorldMapViewDelta {
                header: header(),
                tiles: value.tiles,
                cleared: vec![],
            },
        )
        .map_err(|e| format!("{e:?}"))?;
        // The encoded display_only oneof is 30 01; zero is not a valid origin.
        let flag = payload
            .windows(2)
            .rposition(|bytes| bytes == [0x30, 1])
            .ok_or("no display-only origin")?;
        payload[flag + 1] = 0;
        let frame = encode_state_delta(1, 41, 17, 9, 10, 1, &payload)?;
        let (client, peer) = map_peer(SELECTED, Some(map_snapshot()?), false, Some(frame), true);
        let mut session = Session::admit(client, map_admission()?).await?;
        assert!(matches!(
            session.service_liveness(Duration::from_millis(50)).await,
            Err(SessionError::WorldMap(
                world_map::WorldMapWireError::Malformed
            ))
        ));
        assert_eq!(session.map_revision, 9);
        assert!(session.world_map_view().is_none());
        assert!(session.join_snapshot().world_map.is_none());
        assert!(session.take_events().is_empty());
        assert!(matches!(
            session.step(StepDirection::East).await,
            Err(SessionError::SessionUnusable)
        ));
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn map_binding_mismatch_sends_resync_without_advancing_map_revision_and_poisoning_prevents_reuse()
-> Result<(), BoxError> {
    block_on(async {
        let mut other = header();
        other.reset_epoch += 1;
        let mut next = other;
        next.origin.x = 1;
        let payload = world_map::encode_world_map_delta(
            &other,
            &WorldMapViewDelta {
                header: next,
                tiles: vec![],
                cleared: vec![],
            },
        )
        .map_err(|e| format!("{e:?}"))?;
        let frame = encode_state_delta(1, 41, 17, 9, 10, 1, &payload)?;
        let (client, peer) = map_peer(SELECTED, Some(map_snapshot()?), false, Some(frame), true);
        let mut session = Session::admit(client, map_admission()?).await?;
        assert!(matches!(
            session.service_liveness(Duration::from_millis(50)).await,
            Err(SessionError::WorldMap(
                world_map::WorldMapWireError::BindingMismatch
            ))
        ));
        assert_eq!(session.map_revision, 9);
        assert!(session.world_map_view().is_none());
        assert!(session.join_snapshot().world_map.is_none());
        assert!(session.take_events().is_empty());
        assert!(matches!(
            session.step(StepDirection::East).await,
            Err(SessionError::SessionUnusable)
        ));
        drop(session);
        peer.await??;
        Ok(())
    })?
}
