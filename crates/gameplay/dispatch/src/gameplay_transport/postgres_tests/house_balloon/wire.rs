use super::*;

pub(in super::super) struct HouseExchange<'a> {
    pub(in super::super) current: Option<std::num::NonZeroU32>,
    pub(in super::super) after: Option<std::num::NonZeroU32>,
    pub(in super::super) nested: Option<std::num::NonZeroU32>,
    pub(in super::super) foreign_after_snapshot: Option<&'a Harness>,
}

pub(in super::super) async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command: u64,
    payload: &[u8],
    view: HouseExchange<'_>,
) -> TestResult<(AdmittedSession, UseDisposition, Vec<u8>, usize)> {
    let HouseExchange {
        current,
        after,
        nested,
        foreign_after_snapshot,
    } = view;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let client_task = async {
        let mut source = None;
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
                    if domain.domain_id
                        == oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY
                    {
                        let inventory = oteryn_protocol_oteryn::item_view::decode_character_inventory_with_equip_drop(domain.payload,
                            admitted.continuity.selected_capabilities.contains(oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_EQUIP_DROP_V1)).map_err(debug)?;
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
        let actual = if current.is_some() {
            let mut intent = oteryn_protocol_oteryn::world_object::decode_use_intent_target(
                admitted.continuity.selected_capabilities.as_slice(),
                payload,
            )
            .map_err(debug)?;
            intent.target = UseTarget::Item(source.ok_or("House source handle")?);
            encode_item_use_intent(&intent).map_err(debug)?
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
                command_type: 2,
                payload: &actual,
            },
        )
        .map_err(debug)?;
        client.write_u32(u32::try_from(frame.len())?).await?;
        client.write_all(&frame).await?;
        client.shutdown().await?;
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
                assert_eq!(result.command_id, command);
                assert_eq!(result.status, CommandStatus::Accepted);
                response = Some(
                    oteryn_protocol_oteryn::world_object::decode_use_result(result.payload)
                        .map_err(debug)?,
                );
            }
            if envelope.message_type() == MessageType::StateDelta {
                let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                    .map_err(debug)?;
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
