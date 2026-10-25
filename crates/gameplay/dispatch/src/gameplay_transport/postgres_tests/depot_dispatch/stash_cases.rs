use super::super::TestResult;
use super::super::fixture::{debug, id};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, FreshAdmissionAuthority, IDLE_LIVENESS, UseCommand, serve_admitted,
};
use oteryn_protocol_oteryn::{CommandStatus, MessageType, item_view::ItemHandle, stash::*};
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

async fn send(client: &mut DuplexStream, command_id: u64, intent: StashIntent) -> TestResult {
    let payload = encode_stash_intent(intent).map_err(debug)?;
    let frame = oteryn_protocol_oteryn::encode_client_command(
        1,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id,
            command_type: COMMAND_TYPE_STASH_INTENT,
            payload: &payload,
        },
    )
    .map_err(debug)?;
    client
        .write_all(&u32::try_from(frame.len())?.to_be_bytes())
        .await?;
    client.write_all(&frame).await?;
    Ok(())
}

async fn result(client: &mut DuplexStream, id: u64, deltas: &mut usize) -> TestResult<StashReply> {
    loop {
        let len = client.read_u32().await? as usize;
        assert!(len <= 1 << 20);
        let mut frame = vec![0; len];
        client.read_exact(&mut frame).await?;
        let wire = oteryn_protocol_oteryn::decode_wire_envelope(&frame).map_err(debug)?;
        if wire.message_type() == MessageType::StateDelta {
            let delta =
                oteryn_protocol_oteryn::decode_state_delta(wire.payload()).map_err(debug)?;
            if delta.domain_id == DOMAIN_STASH {
                *deltas += 1;
                assert_eq!(delta.delta_type, 1);
                decode_stash_reply(delta.payload).map_err(debug)?;
            }
        }
        if wire.message_type() == MessageType::CommandResult {
            let result =
                oteryn_protocol_oteryn::decode_command_result(wire.payload()).map_err(debug)?;
            assert_eq!(result.command_id, id);
            assert_eq!(result.status, CommandStatus::Accepted);
            return decode_stash_reply(result.payload).map_err(debug);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    handle: ItemHandle,
    other_handles: &[ItemHandle],
    pool: &sqlx::PgPool,
    source: [u8; 16],
    actor: ExactActorRef,
    fence: CurrentCharacterItemFence,
) -> TestResult {
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 18, 37, 62])
            .ok_or("stash capability")?;
    assert!(
        admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_STASH_V1)
    );
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let server_run = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS);
    let client_run = async {
        let mut deltas = 0;
        send(
            &mut client,
            1,
            StashIntent {
                page: 0,
                action: StashAction::Browse,
            },
        )
        .await?;
        let opened = result(&mut client, 1, &mut deltas).await?;
        assert_eq!(opened.outcome, StashOutcome::Opened);
        assert!(opened.rows.is_empty());
        let stow = StashIntent {
            page: 0,
            action: StashAction::Stow(handle),
        };
        send(&mut client, 2, stow).await?;
        let stowed = result(&mut client, 2, &mut deltas).await?;
        assert_eq!(stowed.outcome, StashOutcome::Committed);
        assert!(stowed.transaction.is_some());
        assert_eq!(stowed.stowed_quantity, 100);
        assert_eq!(stowed.message(), "Stowed 100 objects.");
        assert_eq!(stowed.rows.len(), 1);
        assert_eq!(stowed.rows[0].quantity, 100);
        assert_eq!(stowed.rows[0].definition_key, "oteryn:item.tibia.i3028");
        assert!(!stowed.rows[0].migration_required);
        let old_row = stowed.rows[0].handle;
        let intent = |row, amount| StashIntent {
            page: 0,
            action: StashAction::Withdraw { row, amount },
        };
        send(&mut client, 3, intent(old_row, 2001)).await?;
        let limit = result(&mut client, 3, &mut deltas).await?;
        assert_eq!(limit.outcome, StashOutcome::AmountLimit);
        assert_eq!(limit.rows[0].quantity, 100);
        send(&mut client, 4, intent(old_row, 101)).await?;
        assert_eq!(
            result(&mut client, 4, &mut deltas).await?.outcome,
            StashOutcome::Insufficient
        );
        send(&mut client, 5, intent(old_row, 37)).await?;
        let withdrawn = result(&mut client, 5, &mut deltas).await?;
        assert_eq!(withdrawn.outcome, StashOutcome::Committed);
        assert_eq!(withdrawn.rows[0].quantity, 63);
        assert_ne!(withdrawn.rows[0].handle, old_row);
        send(&mut client, 6, intent(old_row, 1)).await?;
        assert_eq!(
            result(&mut client, 6, &mut deltas).await?.outcome,
            StashOutcome::Stale
        );
        let mut latest = withdrawn.rows[0].handle;
        for (i, handle) in other_handles.iter().enumerate() {
            let command_id = 7 + u64::try_from(i)?;
            send(
                &mut client,
                command_id,
                StashIntent {
                    page: 0,
                    action: StashAction::Stow(*handle),
                },
            )
            .await?;
            let added = result(&mut client, command_id, &mut deltas).await?;
            assert_eq!(added.outcome, StashOutcome::Committed);
            assert_eq!(added.rows[0].quantity, 63 + 100 * (u32::try_from(i)? + 1));
            latest = added.rows[0].handle;
        }
        send(&mut client, 9, intent(latest, 201)).await?;
        let multiple = result(&mut client, 9, &mut deltas).await?;
        assert_eq!(multiple.outcome, StashOutcome::Committed);
        assert_eq!(multiple.rows[0].quantity, 62);
        send(&mut client, 10, intent(multiple.rows[0].handle, 62)).await?;
        let empty = result(&mut client, 10, &mut deltas).await?;
        assert_eq!(empty.outcome, StashOutcome::Committed);
        assert!(empty.rows.is_empty());
        assert!(deltas >= 9);
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>((stowed, withdrawn, limit, stow, intent(old_row, 37)))
    };
    let mut server_run = std::pin::pin!(server_run);
    let mut client_run = std::pin::pin!(client_run);
    let mut server_end = None;
    let client_result = std::future::poll_fn(|cx| {
        use std::future::Future;
        if server_end.is_none()
            && let std::task::Poll::Ready(end) = server_run.as_mut().poll(cx)
        {
            server_end = Some(end);
        }
        match client_run.as_mut().poll(cx) {
            std::task::Poll::Pending if server_end.is_some() => {
                std::task::Poll::Ready(Err("server ended before stash client completed".into()))
            }
            state => state,
        }
    })
    .await;
    if client_result.is_ok() && server_end.is_none() {
        server_end = Some(server_run.await);
    }

    let (stowed, withdrawn, limit, stow, withdraw) = client_result?;
    let command = |n| UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: n,
        item_fence: Some(fence),
    };
    let mut old = match server_end.ok_or("stash server outcome")? {
        crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(value) => {
            value
        }
        other => return Err(format!("stash connection: {other:?}").into()),
    };
    old.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 18, 37])
            .ok_or("old capability")?;
    let payload = encode_stash_intent(StashIntent {
        page: 0,
        action: StashAction::Browse,
    })
    .map_err(debug)?;
    let frame = oteryn_protocol_oteryn::encode_client_command(
        1,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id: old.continuity.next_command_id,
            command_type: COMMAND_TYPE_STASH_INTENT,
            payload: &payload,
        },
    )
    .map_err(debug)?;
    let mut input = u32::try_from(frame.len())?.to_be_bytes().to_vec();
    input.extend(frame);
    let (_, output) =
        super::super::weather_wire::fixture_access::serve_input(owner, old, &input).await?;
    let mut at = output.as_slice();
    let mut refused = 0;
    while !at.is_empty() {
        let len = u32::from_be_bytes(at[..4].try_into()?) as usize;
        let wire = oteryn_protocol_oteryn::decode_wire_envelope(&at[4..4 + len]).map_err(debug)?;
        if wire.message_type() == MessageType::CommandResult {
            assert_eq!(
                oteryn_protocol_oteryn::decode_command_result(wire.payload())
                    .map_err(debug)?
                    .status,
                CommandStatus::Rejected
            );
            refused += 1;
        }
        if wire.message_type() == MessageType::StateDelta {
            assert_ne!(
                oteryn_protocol_oteryn::decode_state_delta(wire.payload())
                    .map_err(debug)?
                    .domain_id,
                DOMAIN_STASH
            );
        }
        at = &at[4 + len..];
    }
    assert_eq!(refused, 1, "unselected stash command must be refused");
    // Lost reply/restart: the native owner probes durable history before expired view handles.
    let replay = owner
        .stash(actor, command(2), stow, None, false)
        .await
        .ok_or("stow replay")?;
    assert_eq!(
        (replay.transaction, replay.outcome),
        (stowed.transaction, stowed.outcome)
    );
    assert_eq!(replay.stowed_quantity, 100);
    assert_eq!(replay.message(), "Stowed 100 objects.");
    let replay = owner
        .stash(actor, command(5), withdraw, None, false)
        .await
        .ok_or("withdraw replay")?;
    assert_eq!(
        (replay.transaction, replay.outcome),
        (withdrawn.transaction, withdrawn.outcome)
    );
    let replay = owner
        .stash(
            actor,
            command(3),
            StashIntent {
                page: 0,
                action: StashAction::Withdraw {
                    row: stowed.rows[0].handle,
                    amount: 2001,
                },
            },
            None,
            false,
        )
        .await
        .ok_or("refusal replay")?;
    assert_eq!(
        (replay.transaction, replay.outcome),
        (limit.transaction, limit.outcome)
    );
    let changed = owner
        .stash(
            actor,
            command(5),
            StashIntent {
                page: 1,
                ..withdraw
            },
            None,
            false,
        )
        .await
        .ok_or("conflict")?;
    assert_eq!(changed.outcome, StashOutcome::Rejected);
    let balance:(i64,i64)=sqlx::query_as("SELECT quantity,(SELECT count(*) FROM game_character_stash_entries) FROM game_character_stash_balances").fetch_one(pool).await?;
    assert_eq!(balance, (0, 6));
    let source_state:(i64,i16)=sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(pool).await?;
    assert_eq!(source_state, (0, 2));
    let (carried,outputs):(i64,i64)=sqlx::query_as("SELECT COALESCE(sum(i.quantity),0)::bigint,count(*) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(pool).await?;
    assert_eq!((carried, outputs), (300, 5));
    let events:(i64,i64)=sqlx::query_as("SELECT count(*),min(expires_at-occurred_at) FROM game_character_stash_audit_outbox WHERE event_type_id=9 AND retention_profile_id='ECONOMY_LEDGER_RETENTION_V1'").fetch_one(pool).await?;
    assert_eq!(events, (6, 2_592_000_000));
    let wrong_row = owner
        .stash(
            actor,
            command(20),
            StashIntent {
                page: 0,
                action: StashAction::Withdraw {
                    row: id(99),
                    amount: 1,
                },
            },
            None,
            true,
        )
        .await
        .ok_or("foreign row")?;
    assert_eq!(wrong_row.outcome, StashOutcome::Stale);
    let wrong_source = owner
        .stash(
            actor,
            command(21),
            stow,
            Some(crate::gameplay_transport::item_view::ItemKey::Instance(id(
                99,
            ))),
            true,
        )
        .await
        .ok_or("foreign source")?;
    assert_eq!(wrong_source.outcome, StashOutcome::Stale);
    let mut stale = fence;
    stale.connection_generation = crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
    let stale_reply = owner
        .stash(
            actor,
            UseCommand {
                item_fence: Some(stale),
                ..command(22)
            },
            stow,
            None,
            false,
        )
        .await
        .ok_or("stale generation")?;
    assert_eq!(stale_reply.outcome, StashOutcome::Rejected);
    let no_receipt: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_stash_operations WHERE command_id=22",
    )
    .fetch_one(pool)
    .await?;
    assert_eq!(no_receipt, 0, "no authority to persist even a refusal");
    // The database rejects raw balance writes independently of the runtime planner.
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE game_character_stash_balances SET quantity=1")
        .execute(&mut *tx)
        .await
        .expect_err("raw credit cannot bypass ledger");
    tx.rollback().await?;
    // A zero balance row is retained and credits the next stow again. Altered
    // instances cannot lose their extra state through the conversion.
    let item:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(i.item_instance_id) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.quantity=37 LIMIT 1").bind(id(41).as_slice()).fetch_one(pool).await?;
    let item: [u8; 16] = item.try_into().map_err(|_| "item identity")?;
    sqlx::query(
        "INSERT INTO game_item_tiers(item_instance_id,tier) VALUES(encode($1,'hex')::uuid,1)",
    )
    .bind(item.as_slice())
    .execute(pool)
    .await?;
    let selected = Some(crate::gameplay_transport::item_view::ItemKey::Instance(
        item,
    ));
    let altered = owner
        .stash(actor, command(30), stow, selected, true)
        .await
        .ok_or("tier refusal")?;
    assert_eq!(altered.outcome, StashOutcome::Unsupported);
    sqlx::query("DELETE FROM game_item_tiers WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice())
        .execute(pool)
        .await?;
    let restowed = owner
        .stash(actor, command(31), stow, selected, true)
        .await
        .ok_or("restow")?;
    assert_eq!(restowed.outcome, StashOutcome::Committed);
    assert_eq!(restowed.rows[0].quantity, 37);
    let take = StashIntent {
        page: 0,
        action: StashAction::Withdraw {
            row: restowed.rows[0].handle,
            amount: 20,
        },
    };
    let mut left = std::pin::pin!(owner.stash(actor, command(32), take, None, true));
    let mut right = std::pin::pin!(owner.stash(actor, command(33), take, None, true));
    let (mut a, mut b) = (None, None);
    let (a, b) = std::future::poll_fn(|cx| {
        use std::future::Future;
        if a.is_none()
            && let std::task::Poll::Ready(result) = left.as_mut().poll(cx)
        {
            a = Some(result);
        }
        if b.is_none()
            && let std::task::Poll::Ready(result) = right.as_mut().poll(cx)
        {
            b = Some(result);
        }
        if a.is_some() && b.is_some() {
            std::task::Poll::Ready((a.take(), b.take()))
        } else {
            std::task::Poll::Pending
        }
    })
    .await;
    let (a, b) = (a.flatten(), b.flatten());
    let successes = [a.as_ref(), b.as_ref()]
        .into_iter()
        .flatten()
        .filter(|r| r.outcome == StashOutcome::Committed)
        .count();
    assert_eq!(
        successes, 1,
        "the Channel lane and root fence admit only one debit"
    );
    for reply in [a.as_ref(), b.as_ref()].into_iter().flatten() {
        assert!(matches!(
            reply.outcome,
            StashOutcome::Committed | StashOutcome::Stale
        ));
        assert_eq!(reply.rows[0].quantity, 17);
    }
    // The single native Channel lane can refuse the concurrent turn before it
    // acquires a permit. Its later retry must see the row's new version.
    if a.is_none() || b.is_none() {
        let retry = owner
            .stash(
                actor,
                command(if a.is_none() { 32 } else { 33 }),
                take,
                None,
                true,
            )
            .await
            .ok_or("concurrent retry")?;
        assert_eq!(retry.outcome, StashOutcome::Stale);
    }
    let committed = a.or(b).ok_or("concurrent receipt")?;
    owner.release_premium(id(40), admitted.game_session_id);
    assert!(
        !owner
            .session_premium_gate(admitted.game_session_id)
            .await
            .premium_current()
    );
    let free = owner
        .stash(
            actor,
            command(34),
            StashIntent {
                page: 0,
                action: StashAction::Withdraw {
                    row: committed.rows[0].handle,
                    amount: 17,
                },
            },
            None,
            true,
        )
        .await
        .ok_or("free withdrawal")?;
    assert_eq!(free.outcome, StashOutcome::Committed);
    assert!(free.rows.is_empty());
    let conserved:(i64,i64)=sqlx::query_as("SELECT COALESCE(sum(i.quantity),0)::bigint,(SELECT quantity FROM game_character_stash_balances) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(pool).await?;
    assert_eq!(conserved, (300, 0));
    {
        let mut runtime = owner.runtime.lock().await;
        let mut movement = runtime.borrow_movement_position();
        for x in (2..7).rev() {
            let snapshot = movement.read(actor).map_err(debug)?;
            movement
                .commit_cardinal(
                    snapshot,
                    crate::foundation::MovementLocalPosition { x, y: 0, floor: 7 },
                )
                .map_err(debug)?;
        }
    }
    let far = owner
        .stash(actor, command(35), stow, selected, true)
        .await
        .ok_or("locker reach")?;
    assert_eq!(far.outcome, StashOutcome::NoLocker);
    assert_eq!(far.outcome.message(), "You can't use stash right now.");
    assert!(far.rows.is_empty());
    let replay = owner
        .stash(actor, command(2), stow, None, false)
        .await
        .ok_or("distant replay")?;
    assert_eq!(
        (replay.transaction, replay.outcome),
        (stowed.transaction, StashOutcome::Committed)
    );
    assert!(
        replay.rows.is_empty(),
        "a historical receipt grants no locker access"
    );
    Ok(())
}
