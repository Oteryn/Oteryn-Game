//! C07 replay covers the session state consumed by `tools/dev-client`: map payloads are applied
//! by `crates/session/src/map_transport.rs::apply`, and field-contact health is applied by
//! `crates/session/src/domain_dispatch.rs::Session::apply_state_delta` before callers observe
//! `Session::world_map_snapshot()` / `Session::actor_vitals()`.
use super::*;
use oteryn_protocol_oteryn::world_map::*;

#[test]
fn e2e_c07_replays_field_placement_contact_and_expiry() -> Result<(), BoxError> {
    block_on(async {
        let header = MapViewHeader {
            content_generation: CONTENT_GENERATION,
            bundle_digest: [0x22; 32],
            reset_epoch: 0,
            origin: ActorPosition {
                x: 100,
                y: 100,
                floor: 0,
            },
            first_visible_floor: 0,
        };
        let (placed, expired, vitals_payload, expected) = fixture(&header)?;
        let fields = expected.split_whitespace().collect::<Vec<_>>();
        let health_before = value(&fields, "health_before")?.parse::<u32>()?;
        let health_after = value(&fields, "health_after")?.parse::<u32>()?;
        assert!(health_after < health_before);

        let snapshot = encode_world_map_snapshot(&WorldMapViewSnapshot {
            header,
            tiles: vec![],
        })
        .map_err(|error| format!("map snapshot: {error:?}"))?;
        let (client, mut server) = tokio::io::duplex(256 * 1024);
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                &[4, 6, 18],
                &[4, 6, 18],
                &[],
                (2, item_snapshot(vec![])?.1),
                &[(17, 1, 1, snapshot)],
            )
            .await?;
            for frame in [
                encode_state_delta(1, 41, 17, 1, 2, 1, &placed)?,
                encode_state_delta(
                    1,
                    42,
                    actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                    0,
                    1,
                    actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
                    &vitals_payload,
                )?,
                encode_state_delta(1, 43, 17, 2, 3, 1, &expired)?,
            ] {
                write_frame(&mut server, &frame).await?;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
            // Replaying the old placement is the refusal case: base revision 1 is stale.
            write_frame(
                &mut server,
                &encode_state_delta(1, 44, 17, 1, 2, 1, &placed)?,
            )
            .await?;
            Ok::<(), BoxError>(())
        });
        let admission = Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: CharacterId::decode(&uuid_v7(4))?,
            admission_material: b"grant",
            client_build_id: "c07-replay",
            supported_capabilities: &[4, 6, 18],
            deadline: DEADLINE,
        };
        let mut session = Session::admit(client, admission).await?;
        session.service_liveness(Duration::from_millis(100)).await?;
        let events = session.take_events();
        assert!(events.iter().any(|event| matches!(event, SessionEvent::WorldMap(delta) if delta.new_revision == 2 && delta.value.tiles.iter().any(|tile| !tile.items.is_empty()))));
        assert!(events.iter().any(|event| matches!(event, SessionEvent::WorldMap(delta) if delta.new_revision == 3 && delta.value.tiles.iter().all(|tile| tile.items.iter().all(|item| item.definition != MapDefinition::Item(std::num::NonZeroU32::new(21465).unwrap_or(std::num::NonZeroU32::MIN)))))));
        assert_eq!(
            session.actor_vitals().map(|vitals| vitals.health),
            Some(health_after)
        );
        let before_invalid = session.actor_vitals().copied();
        assert!(
            session
                .service_liveness(Duration::from_millis(100))
                .await
                .is_err()
        );
        assert_eq!(session.actor_vitals().copied(), before_invalid);
        drop(session);
        peer.await??;
        Ok(())
    })?
}

type Fixture = (Vec<u8>, Vec<u8>, Vec<u8>, String);

fn fixture(header: &MapViewHeader) -> Result<Fixture, BoxError> {
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        return Ok((
            std::fs::read(path.join("c07-map-placed.bin"))?,
            std::fs::read(path.join("c07-map-expired.bin"))?,
            std::fs::read(path.join("c07-vitals.bin"))?,
            std::fs::read_to_string(path.join("c07-expected.txt"))?,
        ));
    }
    let at = ActorPosition {
        x: 101,
        y: 100,
        floor: 0,
    };
    let field = std::num::NonZeroU32::new(21465).ok_or("field definition must be nonzero")?;
    let tile = |items| MapTile {
        position: at,
        items,
        more: false,
        ground_speed: 100,
    };
    let ground = MapItem {
        definition: MapDefinition::Terrain(
            std::num::NonZeroU32::new(1).ok_or("terrain definition must be nonzero")?,
        ),
        count: 1,
        sub_type: 0,
        origin: MapOrigin::BaseOrdinal {
            ordinal: 1,
            object_revision: 0,
        },
        appearance_id: 0,
    };
    let item = MapItem {
        definition: MapDefinition::Item(field),
        count: 1,
        sub_type: 0,
        origin: MapOrigin::DisplayOnly,
        appearance_id: 0,
    };
    let placed = encode_world_map_delta(
        header,
        &WorldMapViewDelta {
            header: *header,
            tiles: vec![tile(vec![ground, item])],
            cleared: vec![],
        },
    )
    .map_err(|error| format!("placed: {error:?}"))?;
    let expired = encode_world_map_delta(
        header,
        &WorldMapViewDelta {
            header: *header,
            tiles: vec![tile(vec![ground])],
            cleared: vec![],
        },
    )
    .map_err(|error| format!("expired: {error:?}"))?;
    let vitals_payload = actor_spell::encode_actor_vitals(&vitals(90, 55))
        .map_err(|error| format!("vitals: {error:?}"))?;
    Ok((
        placed,
        expired,
        vitals_payload,
        "field_ref=21465 target=101 100 0 health_before=100 health_after=90 field_present=false\n"
            .into(),
    ))
}

fn value<'a>(fields: &'a [&str], name: &str) -> Result<&'a str, BoxError> {
    fields
        .iter()
        .find_map(|field| field.strip_prefix(&format!("{name}=")))
        .ok_or_else(|| format!("missing {name} in C07 expected values").into())
}
