//! W18 explicit730 protocol -> selected capability -> composed owner -> PostgreSQL.
use super::*;
use oteryn_protocol_oteryn::house_rotation as rotation;

pub(super) async fn scenario(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    mut admitted: AdmittedSession,
    case: Case,
    before: &Value,
) -> TestResult {
    use super::scenario::{custody, item, receipts};
    let after_id = match case {
        Case::Decoration(_, after) => after,
        Case::NativeNoop(source) => source,
        _ => return Err("not rotation case".into()),
    };
    let old_custody = custody(h).await?;
    let (next, query, _) = exchange(owner, admitted, 1, rotation::Request::Query(0)).await?;
    admitted = next;
    let rotation::Reply::Catalogue(catalogue) = query else {
        return Err("native rotation catalogue".into());
    };
    let target = catalogue
        .entries
        .iter()
        .find(|entry| matches!(&entry.target,rotation::Target::Item{item,..} if *item==id(71)))
        .ok_or("actual nearby House furniture")?;
    assert!(!target.label.contains("oteryn:"));
    assert_eq!(
        target.target,
        rotation::Target::Item {
            item: id(71),
            expected_revision: 1
        }
    );
    let request = rotation::Request::Rotate(target.target.clone());
    let (next, reply, status) = exchange(owner, admitted, 2, request.clone()).await?;
    admitted = next;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(reply, rotation::Reply::Applied(UseDisposition::Committed));
    let after = item(h).await?;
    assert_eq!(
        after["definition_production_key"],
        format!("oteryn:item.tibia.i{after_id}")
    );
    let unchanged = matches!(case, Case::NativeNoop(_));
    assert_eq!(after["state_revision"], if unchanged { 1 } else { 2 });
    assert_eq!(
        after["minted_transaction_id"],
        before["minted_transaction_id"]
    );
    assert_eq!(custody(h).await?, old_custody);
    assert_eq!(receipts(h).await?, if unchanged { 0 } else { 1 });
    if unchanged {
        assert_eq!(after, *before);
    }
    let (next, replay, status) = exchange(owner, admitted, 2, request.clone()).await?;
    admitted = next;
    assert_eq!(status, CommandStatus::DuplicateReplay);
    assert_eq!(replay, reply);
    assert_eq!(item(h).await?, after);
    assert_eq!(receipts(h).await?, if unchanged { 0 } else { 1 });
    let (next, query, _) = exchange(owner, admitted, 3, rotation::Request::Query(0)).await?;
    admitted = next;
    let rotation::Reply::Catalogue(catalogue) = query else {
        return Err("refreshed rotation catalogue".into());
    };
    let target = catalogue
        .entries
        .iter()
        .find(|entry| matches!(&entry.target,rotation::Target::Item{item,..} if *item==id(71)))
        .ok_or("refreshed House furniture")?;
    assert_eq!(
        target.target,
        rotation::Target::Item {
            item: id(71),
            expected_revision: if unchanged { 1 } else { 2 }
        }
    );
    let stale_request = rotation::Request::Rotate(rotation::Target::Item {
        item: id(71),
        expected_revision: if unchanged { 2 } else { 1 },
    });
    let (next, stale, status) = exchange(owner, admitted, 4, stale_request).await?;
    admitted = next;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(stale, rotation::Reply::Applied(UseDisposition::StaleState));
    assert_eq!(item(h).await?, after);
    assert_eq!(receipts(h).await?, if unchanged { 0 } else { 1 });
    // The real durable owner independently replays history even if ingress has
    // lost its transient reply directory, and still checks current authority.
    let context = {
        let runtime = owner.runtime.lock().await;
        crate::durability::consume_transform::HouseActionContext::from_native_owner(
            &runtime,
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            admitted.item_fence.ok_or("fence")?,
        )
        .map_err(debug)?
    }
    .with_rotation(1);
    let mut intent = rotation::COMMAND.to_be_bytes().to_vec();
    intent.extend(request.encode().ok_or("rotation encode")?);
    let command = crate::durability::consume_transform::ConsumeTransformRequest {
        command: crate::foundation::CommandRef::new(
            admitted.game_session_id,
            crate::foundation::CommandId::new(2).map_err(debug)?,
        ),
        source: id(71),
        target: None,
        intent,
    };
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let replay = owner
        .root
        .consume_transform_in_house(owner.holder, context, command, native)
        .await?;
    assert_eq!(replay.disposition, UseDisposition::Committed);
    assert!(!replay.fresh);
    assert_eq!(item(h).await?, after);
    eprintln!(
        "W18 POSTGRES_CASE_COMPLETED explicit730 real_catalogue_route_mutation_duplicate_and_durable_replay source={after_id}"
    );
    Ok(())
}

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    id: u64,
    request: rotation::Request,
) -> TestResult<(AdmittedSession, rotation::Reply, CommandStatus)> {
    let payload = request.encode().ok_or("request encode")?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let generation = admitted.continuity.connection_generation;
    let client_task = async {
        loop {
            let bytes = super::super::item_look::read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
            if envelope.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        let bytes = encode_client_command(
            generation,
            &ClientCommandValue {
                command_id: id,
                command_type: rotation::COMMAND,
                payload: &payload,
            },
        )
        .map_err(debug)?;
        client.write_u32(u32::try_from(bytes.len())?).await?;
        client.write_all(&bytes).await?;
        client.shutdown().await?;
        let mut reply = None;
        loop {
            let length = match client.read_u32().await {
                Ok(length) => usize::try_from(length)?,
                Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(error) => return Err(error.into()),
            };
            let mut bytes = vec![0; length];
            client.read_exact(&mut bytes).await?;
            let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
            if envelope.message_type() == MessageType::CommandResult {
                let response = decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(response.command_id, id);
                reply = Some((
                    rotation::Reply::decode(response.payload)
                        .ok_or("decoded native rotation reply")?,
                    response.status,
                ));
            }
        }
        reply.ok_or_else(|| Box::<dyn std::error::Error>::from("missing rotation reply"))
    };
    let serving = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    use std::future::Future;
    use std::task::Poll;
    let mut serving = std::pin::pin!(serving);
    let mut client_task = std::pin::pin!(client_task);
    let (mut end, mut result) = (None, None);
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
    let ConnectionEnd::AdmittedThenDisconnected(admitted) = end.ok_or("server ended")? else {
        return Err("rotation server admission lost".into());
    };
    let (reply, status) = result.ok_or("rotation client ended")??;
    Ok((admitted, reply, status))
}
