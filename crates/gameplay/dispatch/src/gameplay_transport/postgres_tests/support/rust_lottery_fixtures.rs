use std::cell::RefCell;
use std::num::NonZeroU32;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use super::fixture::debug;

use super::TestResult;

use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseTarget, encode_item_use_intent,
};

use crate::gameplay_transport::ComposedFreshAdmission;

use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, IDLE_LIVENESS, serve_admitted,
};

pub(super) fn bundle_action_room(
    room: &crate::content::QualifiedNativeEntryRoom,
    map: &crate::map::boot::BundleWorld,
    world: crate::foundation::WorldId,
    cells: &crate::content::NativeEntryMovementCells,
) -> TestResult<crate::content::QualifiedNativeEntryRoom> {
    let start = map.start();
    let floor = start
        .floor
        .checked_neg()
        .ok_or("bundle fixture start floor")?;
    Ok(room
        .bundle_view(
            cells.clone(),
            map.frame_binding_digest(world),
            map.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: i32::from(start.x),
                y: i32::from(start.y),
                floor: i16::from(floor),
            },
        )
        .ok_or("qualified bundle fixture room")?)
}

pub(super) async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    payload: &[u8],
    current_source: Option<std::num::NonZeroU32>,
    current_target: Option<std::num::NonZeroU32>,
) -> TestResult<(AdmittedSession, UseDisposition, Vec<u8>)> {
    exchange_selected_source(
        owner,
        admitted,
        command_id,
        payload,
        current_source,
        current_target,
        false,
        None,
    )
    .await
}

/// Select the last matching entry from the real current inventory snapshot.
/// Dice-streak fixtures keep earlier dice in their valid carried custody.
pub(super) async fn exchange_latest_source(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    payload: &[u8],
    current_source: Option<std::num::NonZeroU32>,
) -> TestResult<(AdmittedSession, UseDisposition, Vec<u8>)> {
    exchange_selected_source(
        owner,
        admitted,
        command_id,
        payload,
        current_source,
        None,
        true,
        None,
    )
    .await
}

struct WireEffects {
    visuals: RefCell<Vec<oteryn_protocol_oteryn::presentation::PresentationEvent>>,
    /// For an idle carried transform, the played definition followed by the restored definition.
    expiry: Option<(NonZeroU32, NonZeroU32)>,
}

/// Drain deltas after the result, through the real production publisher.
pub(super) async fn exchange_with_visuals(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    payload: &[u8],
    current_source: Option<std::num::NonZeroU32>,
    expiry: Option<(NonZeroU32, NonZeroU32)>,
) -> TestResult<(
    AdmittedSession,
    UseDisposition,
    Vec<u8>,
    Vec<oteryn_protocol_oteryn::presentation::PresentationEvent>,
)> {
    let effects = WireEffects {
        visuals: RefCell::default(),
        expiry,
    };
    let (end, result, actual) = exchange_selected_source(
        owner,
        admitted,
        command_id,
        payload,
        current_source,
        None,
        false,
        Some(&effects),
    )
    .await?;
    Ok((end, result, actual, effects.visuals.into_inner()))
}

#[allow(
    clippy::too_many_arguments,
    reason = "test helper preserves existing exchange inputs and adds an optional wire recorder"
)]
async fn exchange_selected_source(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    payload: &[u8],
    current_source: Option<std::num::NonZeroU32>,
    current_target: Option<std::num::NonZeroU32>,
    latest_source: bool,
    effects: Option<&WireEffects>,
) -> TestResult<(AdmittedSession, UseDisposition, Vec<u8>)> {
    if owner.root.has_ready_demand() {
        owner.root.maintain_ready_once().await.map_err(debug)?;
    }
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let expiry = effects.and_then(|effects| effects.expiry);
    let client_task = async move {
        let mut source = None;
        let mut target = None;
        let mut container = None;
        loop {
            let raw = super::item_look_fixtures::read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::SnapshotChunk {
                let (_, domains) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())?;
                for domain in domains {
                    if domain.domain_id
                        == oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY
                    {
                        let inventory =
                            oteryn_protocol_oteryn::item_view::decode_character_inventory_with_equip_drop(
                                domain.payload,
                                admitted.continuity.selected_capabilities.contains(12),
                            )
                            .map_err(debug)?;
                        // A picked-up target may precede the real bag in the
                        // inventory. OPEN must name a published, native-qualified
                        // container rather than merely any non-source item.
                        container = inventory
                            .entries
                            .iter()
                            .find_map(|entry| {
                                let native = owner.active_generation?.native_gameplay()?;
                                let index = native.item_index()?;
                                let definition = crate::gameplay_transport::item_look::typed_definition(
                                    index, entry.item_definition_ref,
                                )?;
                                let facts = crate::gameplay_transport::item_ref_admission::generation_item_facts(
                                    native, &definition,
                                ).ok()?;
                                facts.container_capacity.filter(|capacity| *capacity > 0)
                                    .map(|_| entry.handle)
                            })
                            .or(container);
                        target = inventory
                            .entries
                            .iter()
                            .find(|entry| Some(entry.item_definition_ref) == current_target)
                            .map(|entry| entry.handle);
                        let mut candidates = inventory
                            .entries
                            .iter()
                            .chain(inventory.equipment.iter().map(|entry| &entry.item))
                            .filter(|entry| Some(entry.item_definition_ref) == current_source);
                        source = (if latest_source {
                            candidates.next_back()
                        } else {
                            candidates.next()
                        })
                        .map(|entry| entry.handle)
                        .or(source);
                    }
                    if domain.domain_id
                        == oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW
                    {
                        use oteryn_protocol_oteryn::world_map::{MapDefinition, MapOrigin};
                        let map = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(
                            domain.payload,
                        )
                        .map_err(debug)?;
                        target = map
                            .tiles
                            .iter()
                            .flat_map(|tile| tile.items.iter())
                            .find_map(|item| {
                                if let (MapDefinition::Item(reference), MapOrigin::Handle(handle)) =
                                    (item.definition, item.origin)
                                    && Some(reference) == current_target
                                {
                                    Some(handle)
                                } else {
                                    None
                                }
                            })
                            .or(target);
                        source = map
                            .tiles
                            .iter()
                            .flat_map(|tile| tile.items.iter())
                            .find_map(|item| {
                                if let (MapDefinition::Item(reference), MapOrigin::Handle(handle)) =
                                    (item.definition, item.origin)
                                    && Some(reference) == current_source
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
            if envelope.message_type() == crate::foundation::MessageType::SnapshotCommit {
                break;
            }
        }
        if current_source.is_some()
            && source.is_none()
            && admitted.continuity.selected_capabilities.contains(14)
            && command_id > 1
        {
            use oteryn_protocol_oteryn::container_tree as containers;
            let open =
                containers::encode_container_view_intent(&containers::ContainerViewIntent::Open {
                    handle: container.ok_or("nested source container handle")?,
                    replace_view: None,
                })
                .map_err(debug)?;
            let frame = oteryn_protocol_oteryn::encode_client_command(
                1,
                &oteryn_protocol_oteryn::ClientCommandValue {
                    command_id: command_id - 1,
                    command_type: containers::COMMAND_TYPE_CONTAINER_VIEW_INTENT,
                    payload: &open,
                },
            )?;
            client.write_u32(u32::try_from(frame.len())?).await?;
            client.write_all(&frame).await?;
            let mut opened = false;
            while source.is_none() || !opened {
                let raw = super::item_look_fixtures::read_frame(&mut client).await?;
                let envelope = crate::foundation::decode_wire_envelope(&raw)?;
                if envelope.message_type() == crate::foundation::MessageType::CommandResult {
                    let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                    assert_eq!(result.command_id, command_id - 1);
                    assert_eq!(
                        containers::decode_container_view_result(result.payload).map_err(debug)?,
                        containers::ContainerViewOutcome::Opened
                    );
                    opened = true;
                }
                if envelope.message_type() == crate::foundation::MessageType::StateDelta {
                    let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?;
                    if delta.domain_id == containers::STATE_DOMAIN_CONTAINER_VIEWS {
                        let views =
                            containers::decode_container_views(delta.payload).map_err(debug)?;
                        source = views
                            .views
                            .iter()
                            .flat_map(|view| view.entries.iter())
                            .find(|entry| Some(entry.item_definition_ref) == current_source)
                            .map(|entry| entry.handle);
                    }
                }
            }
        }
        let actual = if current_source.is_some() {
            let mut intent = oteryn_protocol_oteryn::world_object::decode_use_intent_target(
                admitted.continuity.selected_capabilities.as_slice(),
                payload,
            )
            .map_err(debug)?;
            intent.target = UseTarget::Item(source.ok_or("current source handle")?);
            if current_target.is_some() {
                intent.use_with_map_item =
                    Some(MapUseTarget::Item(target.ok_or("current target handle")?));
            }
            encode_item_use_intent(&intent).map_err(debug)?
        } else {
            payload.to_vec()
        };
        let frame = oteryn_protocol_oteryn::encode_client_command(
            1,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id,
                command_type: 2,
                payload: &actual,
            },
        )?;
        client.write_u32(u32::try_from(frame.len())?).await?;
        client.write_all(&frame).await?;
        if expiry.is_none() {
            client.shutdown().await?;
        }
        let mut result = None;
        let mut played = None;
        let mut applied_sequence = 0;
        let expiry_timeout = tokio::time::Instant::now() + Duration::from_secs(12);
        loop {
            let read = super::item_look_fixtures::read_frame(&mut client);
            let frame = if expiry.is_some() {
                tokio::time::timeout_at(expiry_timeout, read)
                    .await
                    .map_err(|_| "idle session received no restored trumpet inventory delta")?
            } else {
                read.await
            };
            let raw = match frame {
                Ok(raw) => raw,
                Err(error)
                    if effects.is_some()
                        && expiry.is_none()
                        && result.is_some()
                        && error.downcast_ref::<std::io::Error>().is_some_and(|error| {
                            error.kind() == std::io::ErrorKind::UnexpectedEof
                        }) =>
                {
                    return Ok::<_, Box<dyn std::error::Error>>((
                        result.ok_or("missing result")?,
                        actual,
                    ));
                }
                Err(error) => return Err(error),
            };
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if expiry.is_some()
                && envelope.message_type() == crate::foundation::MessageType::LivenessProbe
            {
                let probe = oteryn_protocol_oteryn::decode_liveness_probe(envelope.payload())?;
                let ack = oteryn_protocol_oteryn::encode_liveness_ack(
                    envelope.connection_generation(), probe, applied_sequence,
                )?;
                client.write_u32(u32::try_from(ack.len())?).await?;
                client.write_all(&ack).await?;
            }
            applied_sequence = applied_sequence.max(envelope.server_sequence());
            if envelope.message_type() == crate::foundation::MessageType::StateDelta
                && let Some(effects) = effects
            {
                let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?;
                if delta.domain_id
                    == oteryn_protocol_oteryn::presentation::STATE_DOMAIN_ITEM_EFFECTS
                {
                    effects.visuals.borrow_mut().extend(
                        oteryn_protocol_oteryn::presentation::decode_item_effect_batch(
                            delta.payload,
                        )
                        .map_err(debug)?,
                    );
                }
                if let Some((played_ref, restored_ref)) = expiry
                    && delta.domain_id == oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY
                {
                    let inventory = oteryn_protocol_oteryn::item_view::decode_character_inventory_with_equip_drop(
                        delta.payload, admitted.continuity.selected_capabilities.contains(12),
                    ).map_err(debug)?;
                    if let Some(item) = inventory.entries.iter().find(|item| item.item_definition_ref == played_ref) {
                        played = Some(item.handle);
                    } else if let Some(handle) = played
                        && inventory.entries.iter().any(|item| item.handle == handle && item.item_definition_ref == restored_ref)
                    {
                        client.shutdown().await?;
                        return Ok((result.ok_or("USE result before restored trumpet")?, actual));
                    }
                }
            }
            if envelope.message_type() == crate::foundation::MessageType::CommandResult {
                let reply = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                if reply.command_id == command_id {
                    let disposition =
                        oteryn_protocol_oteryn::world_object::decode_use_result(reply.payload)
                            .map_err(debug)?;
                    if effects.is_none() {
                        return Ok::<_, Box<dyn std::error::Error>>((disposition, actual));
                    }
                    result = Some(disposition);
                }
            }
        }
    };
    use std::future::Future;
    use std::task::Poll;
    let serving = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let mut serving = std::pin::pin!(serving);
    let mut client_task = std::pin::pin!(client_task);
    let mut end = None;
    let mut result = None;
    let mut maintenance = std::pin::pin!(super::item_craft_fixtures::maintain_ready(owner));
    // The production node has its own deadline sweep. The connection fixture drives the
    // same owner while the client remains idle and reads server-pushed frames.
    let mut deadline_sweep = std::pin::pin!(async {
        if expiry.is_some() {
            let mut tick = tokio::time::interval(Duration::from_millis(250));
            loop {
                tick.tick().await;
                owner.drain_source_item_deadlines().await;
            }
        }
    });
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        std::future::poll_fn(|context| {
            if let Poll::Ready(result) = maintenance.as_mut().poll(context) {
                return Poll::Ready(result);
            }
            if expiry.is_some() && end.is_none() {
                let _ = deadline_sweep.as_mut().poll(context);
            }
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
                Poll::Ready(Ok(()))
            } else {
                Poll::Pending
            }
        }),
    )
    .await??;
    let end = end.ok_or("server did not finish")?;
    let ConnectionEnd::AdmittedThenDisconnected(admitted) = end else {
        return Err(format!("{end:?}; client_result={result:?}").into());
    };
    let (result, actual) = result.ok_or("client did not finish")??;
    Ok((admitted, result, actual))
}
