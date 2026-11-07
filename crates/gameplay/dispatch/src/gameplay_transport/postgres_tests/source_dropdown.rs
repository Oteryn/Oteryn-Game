//! Crystal dropdown435 is an explicit USE relocation, never a passive floor change.
use super::TestResult;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[test]
fn source_sewer_grate_use_descends_from_object_position() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256 << 20).spawn(|| -> Result<(), String> {
        let started = std::time::Instant::now();
        for name in ["source-dropdown", "source-dropdown-added"] { super::quest_world_callbacks::scenario(name, Position {x:32482,y:32170,floor:-14}, false).map_err(|e| e.to_string())?; }
        println!("POSTGRES_CASE_COMPLETED source_sewer_grate_use_descends_from_object_position duration={:?}", started.elapsed());
        Ok(())
    })?.join().map_err(|_| "dropdown scenario panicked")??;
    Ok(())
}

pub(super) async fn exercise(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &crate::gameplay_transport::connection::AdmittedSession,
    h: &super::fixture::Harness,
    actor: crate::foundation::ExactActorRef,
    trigger: Position,
    added: bool,
) -> TestResult {
    let map = owner.bundle_map().ok_or("map")?;
    let root = oteryn_world_bundle::bundle::placement_key(trigger.floor, trigger.x, trigger.y, 1)
        .ok_or("root")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&root.to_be_bytes());
    let results = if added {
        use crate::map::view::EntrySource;
        let pos = crate::map::overlay::TilePos {
            x: trigger.x,
            y: trigger.y,
            floor: trigger.floor,
        };
        let current = crate::map::view::compose_tile(&map.overlay, &map.facts, pos)
            .map_err(super::fixture::debug)?
            .ok_or("tile")?;
        let root_facts = current.entries.iter().find(|e| matches!(e.source,EntrySource::Base {placement_key,..} if placement_key == root)).ok_or("grate")?.facts;
        assert!(map.facts.apply_quest_objects(
            root,
            0,
            root_facts,
            &[crate::map::facts::QuestObjectMutation {
                position: pos,
                remove: Some(EntrySource::Base {
                    ordinal: 1,
                    placement_key: root,
                    movable: false
                }),
                create: Some(map.facts.quest_object_facts(435).ok_or("435 facts")?)
            }]
        ));
        wire_created_grate(owner, *admitted).await?
    } else {
        super::quest_callbacks::wire_uses(
            owner,
            *admitted,
            &[1, 2],
            &WorldObjectTarget {
                placement,
                expected_revision: 0,
            },
        )
        .await?
    };
    assert_eq!(results[0], UseDisposition::Committed);
    assert_eq!(
        results[1],
        UseDisposition::TooFar,
        "replay cannot descend from the old floor"
    );
    let position = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(super::fixture::debug)?
        .position();
    assert_eq!(
        position,
        super::quest_world_callbacks::local(Position {
            floor: trigger.floor - 1,
            ..trigger
        })
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    Ok(())
}

/// Read the actual live handle from the serving connection, as the client does.
pub(super) async fn wire_created_grate(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult<Vec<UseDisposition>> {
    use super::fixture::debug;
    use crate::foundation::{MessageType, decode_wire_envelope};
    use crate::gameplay_transport::connection::{IDLE_LIVENESS, serve_admitted};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("map use capabilities")?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let peer = async {
        let handle = loop {
            let length = client.read_u32().await?;
            let mut bytes = vec![0; usize::try_from(length)?];
            client.read_exact(&mut bytes).await?;
            let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
            if envelope.message_type() == MessageType::SnapshotChunk {
                let mut found = None;
                for domain in oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())
                    .map_err(debug)?
                    .1
                {
                    if domain.domain_id
                        == oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW
                    {
                        let snapshot =
                            oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(
                                domain.payload,
                            )
                            .map_err(debug)?;
                        found = snapshot
                            .tiles
                            .iter()
                            .flat_map(|tile| tile.items.iter())
                            .find(|item| item.appearance_id == 435)
                            .and_then(|item| match item.origin {
                                oteryn_protocol_oteryn::world_map::MapOrigin::Handle(handle) => {
                                    Some(handle)
                                }
                                _ => None,
                            });
                    }
                }
                if let Some(handle) = found {
                    break handle;
                }
            }
        };
        let payload = oteryn_protocol_oteryn::world_object::encode_item_use_intent(
            &oteryn_protocol_oteryn::world_object::UseIntent {
                target: oteryn_protocol_oteryn::world_object::UseTarget::Item(handle),
                use_with: None,
                use_with_map_item: None,
                use_with_position: None,
            },
        )
        .map_err(debug)?;
        let mut results = Vec::new();
        let first = admitted.continuity.next_command_id;
        for command_id in [first, first + 1] {
            let frame = oteryn_protocol_oteryn::encode_client_command(
                admitted.continuity.connection_generation,
                &oteryn_protocol_oteryn::ClientCommandValue {
                    command_id,
                    command_type: 2,
                    payload: &payload,
                },
            )
            .map_err(debug)?;
            client
                .write_all(&u32::try_from(frame.len())?.to_be_bytes())
                .await?;
            client.write_all(&frame).await?;
            loop {
                let length = client.read_u32().await?;
                let mut bytes = vec![0; usize::try_from(length)?];
                client.read_exact(&mut bytes).await?;
                let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())
                        .map_err(debug)?;
                    assert_eq!(result.command_id, command_id);
                    results.push(
                        oteryn_protocol_oteryn::world_object::decode_use_result(result.payload)
                            .map_err(debug)?,
                    );
                    break;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(210)).await;
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(results)
    };
    let (end, results) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::fixture::join_two(
            async {
                let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
                drop(server);
                end
            },
            peer,
        ),
    )
    .await?;
    results.map_err(|error| format!("{end:?}: {error}").into())
}
