//! I01 replay coverage for the session state used by `tools/dev-client/src/lib.rs`.
//! The consuming code is `container_tree::initial` and `container_tree::apply` in
//! `crates/session/src/container_tree.rs`; the latter stores updates in `Session::container_views`.

use super::*;
use crate::tests::{BoxError, admission_with, block_on, join_peer_with};
use oteryn_protocol_oteryn::world_spatial_entities::{
    SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2, WorldSpatialEntitiesSnapshot,
    encode_world_spatial_entities_snapshot_with_item_handles,
};
use oteryn_protocol_oteryn::{decode_wire_envelope, encode_state_delta};

fn entry(handle: u64, definition: u32, count: u32) -> Result<ItemEntry, BoxError> {
    Ok(ItemEntry {
        handle: ItemHandle::new(handle).ok_or("zero fixture handle")?,
        item_definition_ref: definition.try_into()?,
        count: count.try_into()?,
        sub_type: 0,
    })
}

fn visible(views: &ContainerViews) -> String {
    let mut output = String::new();
    for view in &views.views {
        output.push_str(&format!(
            "view {} {} {} {}\n",
            view.view_id,
            view.container_handle,
            view.parent_handle.map_or(0, ItemHandle::get),
            view.capacity
        ));
        for item in &view.entries {
            output.push_str(&format!(
                "item {} {} {}\n",
                item.handle, item.item_definition_ref, item.count
            ));
        }
    }
    output
}

#[test]
fn e2e_i01_replays_container_capacity_and_stack_quantities() -> Result<(), BoxError> {
    let (snapshot_payload, update_payload, revisions, expected) =
        if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let directory = std::path::Path::new(&directory);
            (
                std::fs::read(directory.join("i01-snapshot.bin"))?,
                std::fs::read(directory.join("i01-update.bin"))?,
                std::fs::read_to_string(directory.join("i01-revisions.txt"))?,
                std::fs::read_to_string(directory.join("i01-expected.txt"))?,
            )
        } else {
            let nested = entry(12, 2_854, 1)?;
            let stack = entry(13, 3_031, 37)?;
            let views = ContainerViews {
                views: vec![ContainerView {
                    view_id: 1,
                    container_handle: ItemHandle::new(10).ok_or("fixture handle")?,
                    parent_handle: None,
                    capacity: 20,
                    entries: vec![nested, stack],
                    browse: None,
                }],
            };
            (
                encode_container_views(&ContainerViews::default())
                    .map_err(|error| format!("{error:?}"))?,
                encode_container_views(&views).map_err(|error| format!("{error:?}"))?,
                "1 1 2 41".into(),
                visible(&views),
            )
        };
    let numbers = revisions
        .split_whitespace()
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()?;
    let [
        snapshot_revision,
        base_revision,
        new_revision,
        server_sequence,
    ] = numbers.as_slice()
    else {
        return Err("invalid I01 capture revision metadata".into());
    };
    if snapshot_revision != base_revision || new_revision <= base_revision {
        return Err("inconsistent I01 capture revisions".into());
    }
    let (snapshot_revision, base_revision, new_revision, server_sequence) = (
        *snapshot_revision,
        *base_revision,
        *new_revision,
        *server_sequence,
    );
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            block_on(async move {
                const SELECTED: &[u32] = &[4, 6, 12, CAPABILITY_CONTAINER_TREE_V1];
                let initial_views = decode_container_views(&snapshot_payload)
                    .map_err(|error| format!("{error:?}"))?;
                let (client, mut server) = tokio::io::duplex(256 * 1024);
                let peer = tokio::spawn(async move {
                    let position = ActorPosition {
                        x: 0,
                        y: 0,
                        floor: 0,
                    };
                    let own_identity = [0xa0; 16];
                    let entities = encode_world_spatial_entities_snapshot_with_item_handles(
                        &WorldSpatialEntitiesSnapshot {
                            content_generation: [0x11; 32],
                            actor_position: position,
                            own_identity,
                            entities: vec![WorldSpatialEntity {
                                display_name: None,
                                kind: EntityKind::Player,
                                entity: EntityRef {
                                    identity: own_identity,
                                    generation: 1,
                                },
                                position,
                                detail: EntityDetail::Actor {
                                    direction: StepDirection::South,
                                    appearance_ref: 1,
                                    health_percent: 100,
                                },
                            }],
                        },
                    )
                    .map_err(|error| format!("I01 spatial fixture: {error:?}"))?;
                    join_peer_with(
                        &mut server,
                        SELECTED,
                        SELECTED,
                        &[],
                        (SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2, entities),
                        &[(
                            STATE_DOMAIN_CONTAINER_VIEWS,
                            snapshot_revision,
                            SNAPSHOT_TYPE_CONTAINER_VIEWS_V1,
                            snapshot_payload,
                        )],
                    )
                    .await?;
                    Ok::<_, BoxError>(server)
                });
                let admitted = Session::admit(client, admission_with(SELECTED)?).await;
                let _server = peer.await??;
                let mut session = admitted?;
                assert_eq!(session.container_views(), Some(&initial_views));

                let frame = |base, new, payload: &[u8]| {
                    encode_state_delta(
                        1,
                        server_sequence,
                        STATE_DOMAIN_CONTAINER_VIEWS,
                        base,
                        new,
                        DELTA_TYPE_CONTAINER_VIEWS_V1,
                        payload,
                    )
                };
                let delta = frame(base_revision, new_revision, &update_payload)?;
                let event = session
                    .apply_state_delta(server_sequence, decode_wire_envelope(&delta)?.payload())?;
                let SessionEvent::ContainerViews(applied) = event else {
                    return Err("I01 update did not produce container views event".into());
                };
                assert_eq!(applied.server_sequence, server_sequence);
                assert_eq!(applied.base_revision, base_revision);
                assert_eq!(applied.new_revision, new_revision);
                let state = session.container_views().ok_or("I01 update state absent")?;
                assert_eq!(visible(state), expected);
                assert_eq!(&applied.value, state);
                let retained = state.clone();
                let snapshot = session
                    .gated_snapshot(STATE_DOMAIN_CONTAINER_VIEWS)
                    .ok_or("I01 retained snapshot absent")?
                    .clone();
                assert_eq!(snapshot.revision, new_revision);
                assert_eq!(snapshot.payload, update_payload);
                let revisions = session.gated_revisions.clone();
                assert!(revisions.contains(&(STATE_DOMAIN_CONTAINER_VIEWS, new_revision)));

                // Replaying the old base must fail, even with a valid payload.
                assert!(matches!(
                    session.apply_state_delta(
                        server_sequence + 1,
                        decode_wire_envelope(&delta)?.payload(),
                    ),
                    Err(SessionError::StateRevisionMismatch { .. })
                ));
                assert_eq!(session.container_views(), Some(&retained));
                assert_eq!(session.gated_revisions, revisions);
                assert_eq!(
                    session.gated_snapshot(STATE_DOMAIN_CONTAINER_VIEWS),
                    Some(&snapshot)
                );

                let malformed = frame(new_revision, new_revision + 1, &[0xff])?;
                assert!(matches!(
                    session.apply_state_delta(
                        server_sequence + 1,
                        decode_wire_envelope(&malformed)?.payload(),
                    ),
                    Err(SessionError::ItemView(_))
                ));
                assert_eq!(session.container_views(), Some(&retained));
                assert_eq!(session.gated_revisions, revisions);
                assert_eq!(
                    session.gated_snapshot(STATE_DOMAIN_CONTAINER_VIEWS),
                    Some(&snapshot)
                );
                Ok(())
            })?
        })?
        .join()
        .map_err(|_| "I01 replay thread panicked")?
}
