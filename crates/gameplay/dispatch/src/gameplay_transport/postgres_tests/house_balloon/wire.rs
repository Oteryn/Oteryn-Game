use super::*;
include!("wire_lamp_prelude.rs");

#[derive(Default)]
pub(super) struct HouseWireCapture {
    pub(super) move_destination: Option<oteryn_protocol_oteryn::item_view::ItemMoveDestination>,
    pub(super) move_outcome: Option<oteryn_protocol_oteryn::item_view::ItemMoveOutcome>,
    pub(super) map_deltas: Vec<Vec<u8>>,
    pub(super) timer_before_death: Option<serde_json::Value>,
    pub(super) wait_for_death: bool,
    pub(super) exit_after_death: Option<Vec<u8>>,
    pub(super) exit_after_commit: bool,
    pub(super) exit_status: Option<CommandStatus>,
    pub(super) exit_ack: Option<Vec<u8>>,
    pub(super) effects: Vec<Vec<u8>>,
    pub(super) vitals: Vec<Vec<u8>>,
    pub(super) maps: Vec<Vec<u8>>,
    pub(super) chat_snapshots: Vec<Vec<u8>>,
    pub(super) chat_lines: Vec<Vec<u8>>,
}

pub(super) struct HouseExchange<'a> {
    pub(super) capture: Option<std::sync::Arc<std::sync::Mutex<HouseWireCapture>>>,
    pub(super) rotate: bool,
    pub(super) current: Option<std::num::NonZeroU32>,
    pub(super) after: Option<std::num::NonZeroU32>,
    pub(super) nested: Option<std::num::NonZeroU32>,
    pub(super) foreign_after_snapshot: Option<&'a Harness>,
}

pub(super) async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command: u64,
    payload: &[u8],
    view: HouseExchange<'_>,
) -> TestResult<(AdmittedSession, UseDisposition, Vec<u8>, usize)> {
    exchange_inner(owner, admitted, command, payload, view, None).await
}

pub(super) async fn exchange_with_lamp(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command: u64,
    payload: &[u8],
    view: HouseExchange<'_>,
    prelude: HouseLampPrelude<'_>,
) -> TestResult<(AdmittedSession, UseDisposition, Vec<u8>, usize)> {
    exchange_inner(owner, admitted, command, payload, view, Some(prelude)).await
}

async fn exchange_inner(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command: u64,
    payload: &[u8],
    view: HouseExchange<'_>,
    prelude: Option<HouseLampPrelude<'_>>,
) -> TestResult<(AdmittedSession, UseDisposition, Vec<u8>, usize)> {
    let HouseExchange {
        rotate,
        current,
        after,
        nested,
        foreign_after_snapshot,
        capture,
    } = view;
    let move_destination = capture
        .as_ref()
        .and_then(|capture| capture.lock().ok().and_then(|value| value.move_destination));
    let move_selection = oteryn_protocol_oteryn::item_view::ItemMoveSelection {
        equip_drop: admitted.continuity.selected_capabilities.contains(12),
        container_tree: admitted.continuity.selected_capabilities.contains(14),
        world_interaction: admitted.continuity.selected_capabilities.contains(11),
    };
    let wait_for_death = match &capture {
        Some(capture) => {
            capture
                .lock()
                .map_err(|_| "House capture control")?
                .wait_for_death
        }
        None => false,
    };
    let exit_after_commit = capture
        .as_ref()
        .and_then(|capture| capture.lock().ok().map(|value| value.exit_after_commit))
        .unwrap_or(false);
    let exit_after_death = capture
        .as_ref()
        .and_then(|capture| capture.lock().ok()?.exit_after_death.clone());
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let client_task = async {
        let mut source = None;
        let mut lamp = None;
        let mut inner = None;
        let mut binding = None;
        loop {
            let raw = super::super::item_look::read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&raw).map_err(debug)?;
            if envelope.message_type() == MessageType::SnapshotChunk {
                let (_, domains) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())
                        .map_err(debug)?;
                for domain in domains {
                    if domain.domain_id == oteryn_protocol_oteryn::chat::STATE_DOMAIN_CHAT
                        && let Some(capture) = &capture
                    {
                        capture
                            .lock()
                            .map_err(|_| "House chat snapshot capture")?
                            .chat_snapshots
                            .push(domain.payload.to_vec());
                    }
                    if domain.domain_id
                        == oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY
                    {
                        let inventory = oteryn_protocol_oteryn::item_view::decode_character_inventory_with_equip_drop(domain.payload,
                            admitted.continuity.selected_capabilities.contains(oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_EQUIP_DROP_V1)).map_err(debug)?;
                        lamp = prelude
                            .as_ref()
                            .and_then(|prelude| {
                                inventory
                                    .entries
                                    .iter()
                                    .find(|entry| entry.item_definition_ref == prelude.definition)
                                    .map(|entry| entry.handle)
                            })
                            .or(lamp);
                        source = inventory
                            .entries
                            .iter()
                            .find(|entry| Some(entry.item_definition_ref) == current)
                            .map(|entry| entry.handle)
                            .or(source);
                        inner = inventory
                            .entries
                            .iter()
                            .find(|entry| Some(entry.item_definition_ref) == nested)
                            .map(|entry| entry.handle)
                            .or(inner);
                    }
                    if domain.domain_id
                        == oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW
                    {
                        if let Some(capture) = &capture {
                            capture
                                .lock()
                                .map_err(|_| "House map capture")?
                                .maps
                                .push(domain.payload.to_vec());
                        }
                        let map = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(
                            domain.payload,
                        )
                        .map_err(debug)?;
                        binding = Some(map.header);
                        source = map
                            .tiles
                            .iter()
                            .flat_map(|tile| tile.items.iter())
                            .find_map(|item| {
                                if let (MapDefinition::Item(reference), MapOrigin::Handle(handle)) =
                                    (item.definition, item.origin)
                                    && Some(reference) == current
                                {
                                    Some(handle)
                                } else {
                                    None
                                }
                            })
                            .or(source);
                    }
                }
            }
            if envelope.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        assert_eq!(
            binding.is_some(),
            admitted
                .continuity
                .selected_capabilities
                .contains(oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1),
            "only selected House map capability produces domain17"
        );
        if nested.is_some() {
            use oteryn_protocol_oteryn::container_tree::{
                ContainerViewIntent, ContainerViewOutcome, decode_container_view_result,
                decode_container_views, encode_container_view_intent,
            };
            let open = encode_container_view_intent(&ContainerViewIntent::Open {
                handle: inner.ok_or("actual inner-bag handle")?,
                replace_view: None,
            })
            .map_err(debug)?;
            let open_command = command.checked_sub(1).ok_or("OPEN command")?;
            let frame = encode_client_command(
                admitted.continuity.connection_generation,
                &ClientCommandValue {
                    command_id: open_command,
                    command_type:
                        oteryn_protocol_oteryn::container_tree::COMMAND_TYPE_CONTAINER_VIEW_INTENT,
                    payload: &open,
                },
            )
            .map_err(debug)?;
            client.write_u32(u32::try_from(frame.len())?).await?;
            client.write_all(&frame).await?;
            let (mut opened, mut observed) = (false, false);
            while !opened || !observed {
                let raw = super::super::item_look::read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&raw).map_err(debug)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(result.command_id, open_command);
                    assert_eq!(result.status, CommandStatus::Accepted);
                    assert_eq!(
                        decode_container_view_result(result.payload).map_err(debug)?,
                        ContainerViewOutcome::Opened
                    );
                    opened = true;
                }
                if envelope.message_type() == MessageType::StateDelta {
                    let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                        .map_err(debug)?;
                    if delta.domain_id
                        == oteryn_protocol_oteryn::container_tree::STATE_DOMAIN_CONTAINER_VIEWS
                    {
                        let views = decode_container_views(delta.payload).map_err(debug)?;
                        source = views
                            .views
                            .iter()
                            .flat_map(|view| view.entries.iter())
                            .find(|entry| Some(entry.item_definition_ref) == current)
                            .map(|entry| entry.handle)
                            .or(source);
                        observed = true;
                    }
                }
            }
        }
        if let Some(prelude) = &prelude {
            use_lamp_before_death(
                &mut client,
                admitted,
                command,
                lamp.ok_or("actual carried lamp handle")?,
                payload,
            )
            .await?;
            let timer:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid").bind(prelude.item.as_slice()).fetch_one(&prelude.harness.pool).await?;
            assert_eq!(
                timer["remaining_ms"], 180000,
                "genuine source lamp USE starts its three minute timer"
            );
            if let Some(capture) = &capture {
                capture
                    .lock()
                    .map_err(|_| "actual lamp timer capture")?
                    .timer_before_death = Some(timer);
            }
        }
        let actual = if current.is_some() {
            let target = UseTarget::Item(source.ok_or("House source handle")?);
            if let Some(destination) = move_destination {
                oteryn_protocol_oteryn::item_view::encode_item_move_intent_for(
                    &oteryn_protocol_oteryn::item_view::ItemMoveIntent {
                        source: source.ok_or("actualHousecmd9sourcehandle")?,
                        destination,
                    },
                    move_selection,
                )
                .map_err(debug)?
            } else if rotate {
                // Rotate has a distinct observed-definition field and cannot be
                // decoded by the ordinary USE grammar.
                oteryn_protocol_oteryn::world_rotate::encode_rotate_intent(&target, current)
                    .map_err(debug)?
            } else {
                let mut intent = oteryn_protocol_oteryn::world_object::decode_use_intent_target(
                    admitted.continuity.selected_capabilities.as_slice(),
                    payload,
                )
                .map_err(debug)?;
                intent.target = target;
                encode_item_use_intent(&intent).map_err(debug)?
            }
        } else {
            payload.to_vec()
        };
        if let Some(harness) = foreign_after_snapshot {
            rehome_to_foreign_character(harness).await?;
        }
        let frame = encode_client_command(
            admitted.continuity.connection_generation,
            &ClientCommandValue {
                command_id: command,
                command_type: if move_destination.is_some() {
                    9
                } else if rotate {
                    oteryn_protocol_oteryn::world_rotate::COMMAND_TYPE_ROTATE_WORLD_ITEM
                } else {
                    2
                },
                payload: &actual,
            },
        )
        .map_err(debug)?;
        client.write_u32(u32::try_from(frame.len())?).await?;
        client.write_all(&frame).await?;
        let mut write_closed = !wait_for_death && exit_after_death.is_none();
        if write_closed {
            client.shutdown().await?;
        }
        let mut exit_sent = false;
        let mut exit_accepted = false;
        let mut dead_observed = false;
        let mut response = None;
        let mut deltas = 0;
        loop {
            let length = match client.read_u32().await {
                Ok(length) => usize::try_from(length)?,
                Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(error) => return Err(error.into()),
            };
            let mut frame = vec![0; length];
            client.read_exact(&mut frame).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::CommandResult {
                let result = decode_command_result(envelope.payload()).map_err(debug)?;
                if exit_sent && result.command_id == command + 1 {
                    if let Some(capture) = &capture {
                        let mut capture = capture.lock().map_err(|_| "House live exit capture")?;
                        capture.exit_status = Some(result.status);
                        capture.exit_ack = Some(result.payload.to_vec());
                    }
                    exit_accepted = result.status == CommandStatus::Accepted;
                    if !exit_accepted {
                        client.shutdown().await?;
                        write_closed = true;
                    }
                    continue;
                }
                assert_eq!(result.command_id, command);
                assert!(
                    result.status == CommandStatus::Accepted
                        || (current.is_none() && result.status == CommandStatus::DuplicateReplay),
                    "fresh request is Accepted; retained exact replay may report DuplicateReplay: actual={:?}",
                    result.status
                );
                response = Some(if move_destination.is_some() {
                    let moved = oteryn_protocol_oteryn::item_view::decode_item_move_result_for(
                        result.payload,
                        move_selection,
                    )
                    .map_err(debug)?;
                    if let Some(capture) = &capture {
                        capture
                            .lock()
                            .map_err(|_| "nativeHouseMOVEcapture")?
                            .move_outcome = Some(moved);
                    }
                    if moved == oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved {
                        UseDisposition::Committed
                    } else {
                        UseDisposition::Rejected
                    }
                } else {
                    oteryn_protocol_oteryn::world_object::decode_use_result(result.payload)
                        .map_err(debug)?
                });
            }
            if envelope.message_type() == MessageType::StateDelta {
                let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                    .map_err(debug)?;
                if delta.domain_id == oteryn_protocol_oteryn::chat::STATE_DOMAIN_CHAT
                    && let Some(capture) = &capture
                {
                    capture
                        .lock()
                        .map_err(|_| "House speech capture")?
                        .chat_lines
                        .push(delta.payload.to_vec());
                }
                if delta.domain_id == oteryn_protocol_oteryn::actor_spell::STATE_DOMAIN_ACTOR_VITALS
                {
                    let vitals =
                        oteryn_protocol_oteryn::actor_spell::decode_actor_vitals(delta.payload)
                            .map_err(debug)?;
                    dead_observed |= vitals.max_health > 0 && vitals.health == 0;
                    if let Some(capture) = &capture {
                        capture
                            .lock()
                            .map_err(|_| "House vitals capture")?
                            .vitals
                            .push(delta.payload.to_vec());
                    }
                }
                if delta.domain_id
                    == oteryn_protocol_oteryn::presentation::STATE_DOMAIN_ITEM_EFFECTS
                    && let Some(capture) = &capture
                {
                    capture
                        .lock()
                        .map_err(|_| "House cue capture")?
                        .effects
                        .push(delta.payload.to_vec());
                }
                if delta.domain_id
                    == oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY
                {
                    let inventory = oteryn_protocol_oteryn::item_view::decode_character_inventory_with_equip_drop(delta.payload,
                        admitted.continuity.selected_capabilities.contains(oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_EQUIP_DROP_V1)).map_err(debug)?;
                    if let Some(after) = after {
                        assert!(
                            inventory
                                .entries
                                .iter()
                                .any(|entry| entry.item_definition_ref == after
                                    && Some(entry.handle) == source)
                        );
                    }
                    deltas += 1;
                }
                if delta.domain_id
                    == oteryn_protocol_oteryn::container_tree::STATE_DOMAIN_CONTAINER_VIEWS
                {
                    let views = oteryn_protocol_oteryn::container_tree::decode_container_views(
                        delta.payload,
                    )
                    .map_err(debug)?;
                    if let Some(after) = after {
                        assert!(
                            views
                                .views
                                .iter()
                                .flat_map(|view| view.entries.iter())
                                .any(|entry| entry.item_definition_ref == after
                                    && Some(entry.handle) == source)
                        );
                    }
                    deltas += 1;
                }
                if delta.domain_id == oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW
                {
                    if let Some(capture) = &capture {
                        capture
                            .lock()
                            .map_err(|_| "nativeHousemapdelta")?
                            .map_deltas
                            .push(delta.payload.to_vec());
                    }
                    let map = oteryn_protocol_oteryn::world_map::decode_world_map_delta(
                        &binding.ok_or("map binding")?,
                        delta.payload,
                    )
                    .map_err(debug)?;
                    if let Some(after) = after {
                        let original_handle = source.ok_or("original House handle")?;
                        assert!(
                            map.tiles
                                .iter()
                                .flat_map(|tile| tile.items.iter())
                                .any(|item| item.definition == MapDefinition::Item(after)
                                    && item.origin == MapOrigin::Handle(original_handle))
                        );
                    }
                    binding = Some(map.header);
                    deltas += 1;
                }
            }
            if exit_accepted
                && envelope.message_type() == MessageType::SnapshotCommit
                && !write_closed
            {
                client.shutdown().await?;
                write_closed = true;
            }
            if !exit_sent
                && (dead_observed || exit_after_commit)
                && response == Some(UseDisposition::Committed)
                && let Some(exit_payload) = &exit_after_death
            {
                let frame = encode_client_command(
                    admitted.continuity.connection_generation,
                    &ClientCommandValue {
                        command_id: command + 1,
                        command_type: 102,
                        payload: exit_payload,
                    },
                )
                .map_err(debug)?;
                client.write_u32(u32::try_from(frame.len())?).await?;
                client.write_all(&frame).await?;
                exit_sent = true;
            }
            if !write_closed
                && !exit_sent
                && response
                    .is_some_and(|outcome| outcome != UseDisposition::Committed || dead_observed)
            {
                client.shutdown().await?;
                write_closed = true;
            }
        }
        Ok::<_, Box<dyn std::error::Error>>((response.ok_or("House USE result")?, actual, deltas))
    };
    let serving = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    // Use the repository's existing manual join: Tokio's macros feature is not enabled.
    use std::future::Future;
    use std::task::Poll;
    let mut serving = std::pin::pin!(serving);
    let mut client_task = std::pin::pin!(client_task);
    let mut end = None;
    let mut result = None;
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        std::future::poll_fn(|context| {
            if end.is_none()
                && let Poll::Ready(value) = serving.as_mut().poll(context)
            {
                end = Some(value);
            }
            if result.is_none()
                && let Poll::Ready(value) = client_task.as_mut().poll(context)
            {
                result = Some(value);
            }
            if end.is_some() && result.is_some() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }),
    )
    .await?;
    let end = end.ok_or("House server did not finish")?;
    let result = result.ok_or("House client did not finish")?;
    let ConnectionEnd::AdmittedThenDisconnected(admitted) = end else {
        return Err(format!("House USE end: {end:?}").into());
    };
    let (outcome, actual, deltas) = result?;
    Ok((admitted, outcome, actual, deltas))
}

async fn rehome_to_foreign_character(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($4,'hex')::uuid)")
        .bind(id(104).as_slice()).bind(id(42).as_slice()).bind(INNER).bind(id(105).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(id(101).as_slice()).bind(id(104).as_slice()).bind(id(42).as_slice()).bind(id(105).as_slice()).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_item_container_entries SET character_id=encode($1,'hex')::uuid,parent_item_instance_id=encode($2,'hex')::uuid WHERE item_instance_id=encode($3,'hex')::uuid")
        .bind(id(101).as_slice()).bind(id(104).as_slice()).bind(id(71).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
