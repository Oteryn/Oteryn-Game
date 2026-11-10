//! O040: C10's genuine native House fixture drives USE -> modal -> fenced sleep.
use super::TestResult;
use super::fixture::{Harness, debug};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, ConnectionEnd, IDLE_LIVENESS, serve_admitted},
    tcp_tls::{read_frame, write_frame},
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, MessageType, decode_command_result, decode_snapshot_chunk,
    decode_wire_envelope, encode_client_command,
    server_modal::{
        Answer, COMMAND_TYPE_SERVER_MODAL_ANSWER, COMMAND_TYPE_SERVER_MODAL_QUERY, Dialog,
        QUERY_WITH_TERMINAL_REPLY, Reply,
    },
    world_map::{MapOrigin, STATE_DOMAIN_WORLD_MAP_VIEW, decode_world_map_snapshot},
    world_object::{
        COMMAND_TYPE_USE_INTENT, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
        decode_use_result, encode_item_use_intent,
    },
};
use tokio::io::{AsyncReadExt, DuplexStream};

async fn command(
    client: &mut DuplexStream,
    admitted: AdmittedSession,
    next: &mut u64,
    kind: u32,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let id = *next;
    *next += 1;
    write_frame(
        client,
        &encode_client_command(
            admitted.continuity.connection_generation,
            &ClientCommandValue {
                command_id: id,
                command_type: kind,
                payload,
            },
        )
        .map_err(debug)?,
    )
    .await?;
    loop {
        let bytes = read_frame(client).await?;
        let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
        if envelope.message_type() == MessageType::CommandResult {
            let reply = decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(reply.command_id, id);
            return Ok((reply.status, reply.payload.to_vec()));
        }
    }
}

async fn query(
    client: &mut DuplexStream,
    admitted: AdmittedSession,
    next: &mut u64,
) -> TestResult<Dialog> {
    let (status, bytes) =
        command(client, admitted, next, COMMAND_TYPE_SERVER_MODAL_QUERY, &[]).await?;
    assert_eq!(status, CommandStatus::Accepted);
    let Reply::Dialog(dialog) = Reply::decode(&bytes).map_err(debug)? else {
        return Err("bed rule dialog".into());
    };
    assert_eq!(dialog.title, "Choose a Skill");
    assert_eq!(dialog.choices.len(), 6);
    Ok(dialog)
}

pub(super) async fn sleep(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    assert!(
        !admitted
            .continuity
            .selected_capabilities
            .contains(oteryn_protocol_oteryn::hireling::CAPABILITY_HIRELING_V1),
        "bed modal is independent of hirelings"
    );
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let playing = async {
        let mut target = None;
        loop {
            let bytes = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
            if envelope.message_type() == MessageType::SnapshotChunk {
                for domain in decode_snapshot_chunk(envelope.payload()).map_err(debug)?.1 {
                    if domain.domain_id != STATE_DOMAIN_WORLD_MAP_VIEW {
                        continue;
                    }
                    let map = decode_world_map_snapshot(domain.payload).map_err(debug)?;
                    let Some(tile) = map.tiles.iter().find(|tile| {
                        (tile.position.x, tile.position.y, tile.position.floor)
                            == (32311, 32804, -7)
                    }) else {
                        continue;
                    };
                    let item = tile.items.get(1).ok_or("visible bed foot")?;
                    target = Some(match item.origin {
                        MapOrigin::Handle(handle) => UseTarget::Item(handle),
                        MapOrigin::BaseOrdinal {
                            ordinal,
                            object_revision,
                        } => {
                            let key = crate::map::view::placement_key(
                                crate::map::overlay::TilePos {
                                    x: 32311,
                                    y: 32804,
                                    floor: -7,
                                },
                                ordinal,
                            )
                            .ok_or("bed placement")?;
                            let mut placement = map.header.bundle_digest.to_vec();
                            placement.extend_from_slice(&key.to_be_bytes());
                            UseTarget::WorldObject(WorldObjectTarget {
                                placement,
                                expected_revision: object_revision,
                            })
                        }
                        MapOrigin::DisplayOnly => return Err("bed must be usable".into()),
                    });
                }
            }
            if envelope.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        let target = target.ok_or("bed from the real House map")?;
        let UseTarget::WorldObject(object) = &target else {
            return Err("the native fixed bed must have a base ordinal".into());
        };
        let mut stale = object.clone();
        stale.expected_revision += 1;
        let stale = encode_item_use_intent(&UseIntent {
            target: UseTarget::WorldObject(stale),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(debug)?;
        let mut next = admitted.continuity.next_command_id;
        let payload = encode_item_use_intent(&UseIntent {
            target,
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(debug)?;
        let (status, bytes) = command(
            &mut client,
            admitted,
            &mut next,
            COMMAND_TYPE_SERVER_MODAL_QUERY,
            &[],
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(Reply::decode(&bytes).map_err(debug)?, Reply::None);
        assert_eq!(
            command(
                &mut client,
                admitted,
                &mut next,
                COMMAND_TYPE_USE_INTENT,
                &payload
            )
            .await?
            .0,
            CommandStatus::Rejected,
            "legacy client cannot enter terminal bed modal"
        );
        let (status, bytes) = command(
            &mut client,
            admitted,
            &mut next,
            COMMAND_TYPE_SERVER_MODAL_QUERY,
            QUERY_WITH_TERMINAL_REPLY,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(Reply::decode(&bytes).map_err(debug)?, Reply::None);
        assert_eq!(
            command(
                &mut client,
                admitted,
                &mut next,
                COMMAND_TYPE_USE_INTENT,
                &stale
            )
            .await?
            .0,
            CommandStatus::Rejected
        );
        let (status, bytes) = command(
            &mut client,
            admitted,
            &mut next,
            COMMAND_TYPE_USE_INTENT,
            &payload,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            decode_use_result(&bytes).map_err(debug)?,
            UseDisposition::BedChooseSkill
        );
        let first = query(&mut client, admitted, &mut next).await?;
        assert_eq!(query(&mut client, admitted, &mut next).await?, first);
        let invalid = Answer {
            dialog_id: first.id,
            button_id: 1,
            choice_id: Some(99),
        };
        assert_eq!(
            command(
                &mut client,
                admitted,
                &mut next,
                COMMAND_TYPE_SERVER_MODAL_ANSWER,
                &invalid.encode().map_err(debug)?
            )
            .await?
            .0,
            CommandStatus::Rejected
        );
        let cancel = Answer {
            button_id: 2,
            choice_id: None,
            ..invalid
        };
        for _ in 0..2 {
            let (status, bytes) = command(
                &mut client,
                admitted,
                &mut next,
                COMMAND_TYPE_SERVER_MODAL_ANSWER,
                &cancel.encode().map_err(debug)?,
            )
            .await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert_eq!(
                Reply::decode(&bytes).map_err(debug)?,
                Reply::Answered(cancel)
            );
        }
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_house_bed_sleep_receipts")
                .fetch_one(&h.pool)
                .await?,
            0
        );
        let (status, _) = command(
            &mut client,
            admitted,
            &mut next,
            COMMAND_TYPE_USE_INTENT,
            &payload,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        let dialog = query(&mut client, admitted, &mut next).await?;
        assert_ne!(first.id, dialog.id);
        assert_eq!(
            command(
                &mut client,
                admitted,
                &mut next,
                COMMAND_TYPE_SERVER_MODAL_ANSWER,
                &cancel.encode().map_err(debug)?
            )
            .await?
            .0,
            CommandStatus::Rejected
        );
        let answer = Answer {
            dialog_id: dialog.id,
            button_id: 1,
            choice_id: Some(6),
        };
        let (status, bytes) = command(
            &mut client,
            admitted,
            &mut next,
            COMMAND_TYPE_SERVER_MODAL_ANSWER,
            &answer.encode().map_err(debug)?,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            Reply::decode(&bytes).map_err(debug)?,
            Reply::AnsweredAndLoggedOut(answer)
        );
        let mut after = [0];
        assert_eq!(
            client.read(&mut after).await?,
            0,
            "committed modal ACK precedes clean logout"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_house_bed_sleep_receipts")
                .fetch_one(&h.pool)
                .await?,
            1
        );
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let (end, result) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::fixture::join_two(serving, playing),
    )
    .await?;
    result?;
    assert!(
        matches!(end, ConnectionEnd::AdmittedThenLoggedOut(_)),
        "{end:?}"
    );
    Ok(())
}
