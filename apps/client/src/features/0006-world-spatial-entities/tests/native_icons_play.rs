//! Real Session pushes stationary native creature icon changes into the native play view.
use super::*;
use crate::play::{PlayLink, PlayView, play_channel, run_session};
use oteryn_session::test_support::wire;
type Error = Box<dyn std::error::Error + Send + Sync>;
const CAPS: &[u32] = &[
    6,
    wire::world_spatial_entities::CAPABILITY_NATIVE_CREATURE_ICONS_V1,
];
fn id(value: u8) -> [u8; 16] {
    let mut id = [value; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}
fn own() -> wire::world_spatial_entities::WorldSpatialEntity {
    use wire::world_spatial_entities::*;
    WorldSpatialEntity {
        display_name: None,
        kind: EntityKind::Player,
        entity: EntityRef {
            identity: id(1),
            generation: 1,
        },
        position: wire::world_spatial::ActorPosition {
            x: 100,
            y: 200,
            floor: 0,
        },
        detail: EntityDetail::Actor {
            direction: wire::world_spatial::StepDirection::South,
            appearance_ref: 0,
            health_percent: 100,
        },
    }
}
fn creature(stage: u8) -> Result<wire::world_spatial_entities::WorldSpatialEntity, Error> {
    use wire::world_spatial_entities::*;
    let mut entity = own();
    entity.kind = EntityKind::Creature;
    entity.entity.identity = id(8);
    entity.position.x = 101;
    if stage < 2 {
        let (icon, count) = if stage == 0 { (6, 15) } else { (7, 0) };
        entity.detail = EntityDetail::ActorWithIcons {
            direction: wire::world_spatial::StepDirection::South,
            appearance_ref: 0,
            health_percent: 100,
            icons: NativeCreatureIcons::new(&[NativeCreatureIcon {
                category: 1,
                id: icon,
                count,
            }])
            .map_err(|error| format!("{error:?}"))?,
        };
    }
    Ok(entity)
}
async fn peer(
    mut peer: tokio::io::DuplexStream,
    mut control: tokio::sync::mpsc::Receiver<()>,
) -> Result<(), Error> {
    use wire::world_spatial_entities::*;
    let bootstrap = oteryn_session::read_frame(&mut peer).await?;
    assert_eq!(
        wire::decode_wire_envelope(&bootstrap)?
            .client_bootstrap()?
            .supported_capabilities,
        CAPS
    );
    oteryn_session::write_frame(
        &mut peer,
        &wire::encode_server_accepted(&wire::ServerAcceptedValue {
            game_session_id: wire::GameSessionId::decode(&id(2))?,
            world_id: wire::WorldId::decode(&id(3))?,
            channel_id: wire::ChannelId::decode(&id(4))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 1,
            schema_revision: 1,
            selected_capabilities: CAPS,
        })?,
    )
    .await?;
    let fields = EntityFields::selected(CAPS);
    let snapshot = WorldSpatialEntitiesSnapshot {
        content_generation: [7; 32],
        actor_position: own().position,
        own_identity: own().entity.identity,
        entities: vec![own(), creature(0)?],
    };
    let payload =
        encode_world_spatial_entities_snapshot_with_fields(&snapshot, fields, &EntityLights::new())
            .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        decode_world_spatial_entities_snapshot_with_fields(&payload, fields)
            .map_err(|error| format!("{error:?}"))?
            .0,
        snapshot
    );
    let overlay = wire::world_object::encode_world_object_overlay_snapshot(&[])
        .map_err(|error| format!("{error:?}"))?;
    for frame in wire::encode_single_chunk_snapshot(
        1,
        1,
        1,
        &[
            wire::DomainSnapshot {
                domain_id: 1,
                revision: 1,
                snapshot_type: 2,
                payload: &payload,
            },
            wire::DomainSnapshot {
                domain_id: 2,
                revision: 1,
                snapshot_type: 1,
                payload: &overlay,
            },
        ],
    )? {
        oteryn_session::write_frame(&mut peer, &frame).await?;
    }
    for stage in 1..=2 {
        control
            .recv()
            .await
            .ok_or("native icon test control closed")?;
        let entity = creature(stage)?;
        assert_eq!(
            entity.position,
            creature(0)?.position,
            "source badge updates must be stationary"
        );
        assert!(entity.display_name.is_none());
        let delta = WorldSpatialEntitiesDelta {
            content_generation: [7; 32],
            actor_position: own().position,
            enter: vec![],
            update: vec![entity],
            leave: vec![],
        };
        let bytes =
            encode_world_spatial_entities_delta_with_fields(&delta, fields, &EntityLights::new())
                .map_err(|error| format!("{error:?}"))?;
        oteryn_session::write_frame(
            &mut peer,
            &wire::encode_state_delta(
                1,
                u64::from(stage) + 1,
                1,
                u64::from(stage),
                u64::from(stage) + 1,
                2,
                &bytes,
            )?,
        )
        .await?;
    }
    control
        .recv()
        .await
        .ok_or("native icon final control closed")?;
    Ok(())
}
async fn rendered(view: &mut PlayView, link: &PlayLink, label: &str) -> Result<(), Error> {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            view.tick(link)
                .map_err(|error| format!("native icon play: {error:?}"))?;
            let scene = view.scene();
            let matching = if label.is_empty() {
                scene.names.is_empty()
            } else {
                let mut cache = scene.cache.clone();
                scene.names.len() == 1 && scene.names[0].1 == cache.name(label)
            };
            if matching {
                return Ok::<(), Error>(());
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .map_err(|_| format!("native icon label timed out: {label}"))??;
    Ok(())
}
#[test]
fn srx3_native_creature_modification_icons_cross_real_session_stationary_delta_and_clear()
-> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            oteryn_client_runtime::ClientRuntime::new()?.block_on(async {
                let (client, server) = tokio::io::duplex(256 * 1024);
                let (control, receiver) = tokio::sync::mpsc::channel(1);
                let peer = tokio::spawn(peer(server, receiver));
                let session = oteryn_session::Session::admit(
                    client,
                    oteryn_session::Admission {
                        appearance_items: None,
                        schema_revision: 1,
                        character_id: oteryn_session::CharacterId::decode(&id(5))?,
                        admission_material: b"native-icons",
                        client_build_id: "srx3-native-icons",
                        supported_capabilities: CAPS,
                        deadline: std::time::Duration::from_secs(3),
                    },
                )
                .await;
                let session = match session {
                    Ok(session) => session,
                    Err(error) => {
                        if peer.is_finished() {
                            let result = peer.await?;
                            return Err(
                                format!("native icon admit: {error:?}; peer: {result:?}").into()
                            );
                        }
                        peer.abort();
                        return Err(format!("native icon admit: {error:?}").into());
                    }
                };
                let mut view = PlayView::from_join(session.join_snapshot())?;
                let (link, commands, outbox) = play_channel();
                let runner = tokio::spawn(run_session(session, commands, outbox));
                rendered(&mut view, &link, "[ReducedHealth 15]").await?;
                assert!(view.scene().sprites().len() > 1);
                assert_eq!(view.scene().names[0].0, TileCoord::new(101, 200));
                control.send(()).await?;
                rendered(&mut view, &link, "[ReducedHealth! 0]").await?;
                assert_eq!(view.scene().names[0].0, TileCoord::new(101, 200));
                control.send(()).await?;
                rendered(&mut view, &link, "").await?;
                assert!(
                    view.scene().actor_draws.iter().any(|(tile,draws)| *tile == TileCoord::new(101,200) && !draws.is_empty()),
                    "clearing icon metadata must keep the stationary creature body"
                );
                control.send(()).await?;
                drop(link);
                runner.await?;
                peer.await??;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "native creature icon play thread panicked")?
}
