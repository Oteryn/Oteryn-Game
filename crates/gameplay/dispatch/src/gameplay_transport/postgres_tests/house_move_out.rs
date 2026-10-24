//! HOUSE-1b: admitted connection bytes, native mutation, World job and durable reload.
use super::fixture::{self, Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use oteryn_protocol_oteryn::house_move_out::*;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use tokio::io::AsyncWriteExt;
const HOUSE: &str = "oteryn:content.house.move_out_fixture";
const WORLD: u8 = 42;

#[path = "house_move_out/fixture.rs"]
mod seed;

#[test]
fn house_move_out_postgres_real_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "house move-out E2E panicked")??;
    Ok(())
}

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    kind: u32,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let before = *admitted;
    let (mut server, mut client) = tokio::io::duplex(1024);
    let serving = async move {
        let ended = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
        drop(server);
        ended
    };
    let driving = async {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        write_frame(
            &mut client,
            &encode_client_command(
                before.continuity.connection_generation,
                &ClientCommandValue {
                    command_id: id,
                    command_type: kind,
                    payload,
                },
            )
            .map_err(debug)?,
        )
        .await?;
        let reply = loop {
            let frame = read_frame(&mut client).await?;
            let env = decode_wire_envelope(&frame).map_err(debug)?;
            if env.message_type() == MessageType::CommandResult {
                let r = decode_command_result(env.payload()).map_err(debug)?;
                assert_eq!(r.command_id, id);
                break (r.status, r.payload.to_vec());
            }
        };
        client.shutdown().await?;
        Ok(reply)
    };
    let (ended, result): (_, TestResult<(CommandStatus, Vec<u8>)>) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        fixture::join_two(serving, driving),
    )
    .await?;
    match &ended {
        crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(current)
        | crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenClosed(current, _) => {
            *admitted = *current
        }
        _ => {}
    }
    result.map_err(|error| format!("house command {id}/{kind} ended {ended:?}: {error}").into())
}
async fn command(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    action: u8,
    due: i64,
) -> TestResult<HouseMoveOutReply> {
    reply(owner, admitted, id, action, due, CommandStatus::Accepted).await
}
/// SESSION-INGRESS-1: a reserved ID on this GameSession's command stream replays the
/// retained result instead of reaching the House family again.
async fn replayed(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    action: u8,
    due: i64,
) -> TestResult<HouseMoveOutReply> {
    reply(owner, admitted, id, action, due, CommandStatus::DuplicateReplay).await
}
async fn reply(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    action: u8,
    due: i64,
    expected: CommandStatus,
) -> TestResult<HouseMoveOutReply> {
    let request = HouseMoveOutRequest {
        house: HOUSE.into(),
        action,
        due_at_unix_ms: due,
    };
    let (status, bytes) = exchange(
        owner,
        admitted,
        id,
        if action == 0 {
            COMMAND_TYPE_HOUSE_MOVE_OUT_QUERY
        } else {
            COMMAND_TYPE_HOUSE_MOVE_OUT
        },
        &encode_house_move_out_request(&request).map_err(debug)?,
    )
    .await?;
    assert_eq!(status, expected, "command {id}");
    decode_house_move_out_reply(&bytes).map_err(debug)
}
fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let mut h =
            Harness::create_with_admission_seed(admin.clone(), "house_move_out_wire", true, false)
                .await
                .map_err(|e| format!("harness: {e:?}"))?;
        prepare_current_session(&h)
            .await
            .map_err(|e| format!("prepare session: {e}"))?;
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&h.pool)
                .await?;
        let recipient = seed::reclaim_character(&h, 60, 70, now).await?;
        seed::house_and_items(&h, *recipient.as_bytes(), 101).await?;
        seed::assert_unproven_removal_closed(&h).await?;
        h.root = seed::runtime_root(&h, &admin).await?;
        let deps = Dependencies::new()?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(admitted.game_session_id)
                .await?
                .0;
        admitted.item_fence = item_fence_of(current);
        let original = admitted;
        let mut commands = {
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            deps.initialize(&owner).await;
            owner
                .runtime
                .lock()
                .await
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                owner.offered_capabilities(),
                &[CAPABILITY_HOUSE_MOVE_OUT_V1],
            )
            .ok_or("selection")?;
            assert!(
                admitted
                    .continuity
                    .selected_capabilities
                    .contains(CAPABILITY_HOUSE_MOVE_OUT_V1)
            );
            let query = command(&owner, &mut admitted, 1, 0, 0).await?;
            assert_eq!(query.state, 0);
            let before_notice = admitted;
            let due = now * 1000 + 2 * 86_400_000;
            let noticed = command(&owner, &mut admitted, 2, 1, due).await?;
            assert_eq!(noticed.state, 1);
            assert_eq!(noticed.due_at_unix_ms, due);
            assert_eq!(replayed(&owner, &mut admitted, 2, 1, due).await?, noticed);
            let bytes = encode_house_move_out_request(&HouseMoveOutRequest {
                house: HOUSE.into(),
                action: 1,
                due_at_unix_ms: due + 1,
            })
            .map_err(debug)?;
            // FND-02 §13.4: replacement bytes under a reserved ID are not a new House intent.
            let (status, replay) =
                exchange(&owner, &mut admitted, 2, COMMAND_TYPE_HOUSE_MOVE_OUT, &bytes).await?;
            assert_eq!(status, CommandStatus::DuplicateReplay);
            assert_eq!(decode_house_move_out_reply(&replay).map_err(debug)?, noticed);
            // The native House receipt still refuses changed input under the committed ID.
            assert!(matches!(
                owner
                    .house_move_out_command(before_notice, 2, COMMAND_TYPE_HOUSE_MOVE_OUT, &bytes)
                    .await,
                crate::gameplay_transport::house_move_out::HouseMoveOutReply::Rejected
            ));
            let cancel_before = admitted;
            let cancelled = command(&owner, &mut admitted, 3, 2, 0).await?;
            assert_eq!(cancelled.state, 4);
            assert_eq!(cancelled.operation, noticed.operation);
            assert_eq!(replayed(&owner, &mut admitted, 3, 2, 0).await?, cancelled);
            let second = command(&owner, &mut admitted, 4, 1, due).await?;
            assert_ne!(second.operation, noticed.operation);
            // Once a newer command retires the stream's retained result, the native receipt
            // still answers the earlier cancel.
            let cancel = encode_house_move_out_request(&HouseMoveOutRequest {
                house: HOUSE.into(),
                action: 2,
                due_at_unix_ms: 0,
            })
            .map_err(debug)?;
            let crate::gameplay_transport::house_move_out::HouseMoveOutReply::Payload(receipt) =
                owner
                    .house_move_out_command(cancel_before, 3, COMMAND_TYPE_HOUSE_MOVE_OUT, &cancel)
                    .await
            else {
                return Err("cancel receipt".into());
            };
            assert_eq!(decode_house_move_out_reply(&receipt).map_err(debug)?, cancelled);
            assert_eq!(
                command(&owner, &mut admitted, 5, 0, 0).await?.operation,
                second.operation
            );
            command(&owner, &mut admitted, 6, 2, 0).await?;
            // Check the mechanic's stale lease rejection directly: connection bootstrap also
            // independently disconnects a stale lease during Bank reconciliation.
            let mut stale = admitted;
            stale
                .item_fence
                .as_mut()
                .ok_or("fence")?
                .character_lease_generation += 1;
            let bytes = encode_house_move_out_request(&HouseMoveOutRequest {
                house: HOUSE.into(),
                action: 1,
                due_at_unix_ms: due,
            })
            .map_err(debug)?;
            assert!(matches!(
                owner
                    .house_move_out_command(stale, 7, COMMAND_TYPE_HOUSE_MOVE_OUT, &bytes)
                    .await,
                crate::gameplay_transport::house_move_out::HouseMoveOutReply::Rejected
            ));
            // No capability, malformed payload, invalid deadlines and wrong owner.
            for (mut session, kind, bytes) in seed::invalid_commands(original, admitted, due)? {
                // Every connection resumes this GameSession's command stream; keep the
                // case's capability selection.
                let selected = session.continuity.selected_capabilities;
                session.continuity = admitted.continuity;
                session.continuity.selected_capabilities = selected;
                let next = session.continuity.next_command_id;
                assert_eq!(
                    exchange(&owner, &mut session, next, kind, &bytes).await?.0,
                    CommandStatus::Rejected
                );
                let selected = admitted.continuity.selected_capabilities;
                admitted.continuity = session.continuity;
                admitted.continuity.selected_capabilities = selected;
            }
            std::mem::take(&mut owner.states.command_ingress)
        };
        // An earlier-day NOTICE is historical DATA only; no current authority is derived from it.
        seed::overdue(&h).await?;
        seed::prepare_entrant(&h).await?;
        seed::assert_unproven_checkpoint_closed(&h).await?;
        for (phase, count, state) in [(0, 0, 2), (1, 100, 2), (2, 101, 3), (3, 101, 3)] {
            // Fresh Root and composed owner after every committed boundary, including DRAINED.
            h.root = seed::runtime_root(&h, &admin).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            // The Channel keeps each GameSession's command stream (SESSION-INGRESS-1).
            let mut owner = deps.owner(&h, &authority);
            owner.states.command_ingress = std::mem::take(&mut commands);
            owner
                .drain_source_house_dispositions()
                .await
                .map_err(debug)?;
            let next = admitted.continuity.next_command_id;
            let result = command(&owner, &mut admitted, next, 0, 0).await?;
            assert_eq!(
                (result.state, result.census_count, result.delivered),
                (state, 101, count)
            );
            if phase == 0 {
                seed::assert_frozen(&h).await?;
                seed::blocked_steps_rollback(&h, &owner).await?;
            }
            commands = std::mem::take(&mut owner.states.command_ingress);
        }
        seed::assert_final(&h, *recipient.as_bytes()).await?;
        seed::assert_runtime_privileges(&h).await?;
        h.cleanup().await
    })
}

#[test]
fn house_move_out_postgres_oversized_census() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            run(async |admin| {
                reset();
                let h = Harness::create_with_admission_seed(
                    admin,
                    "house_move_out_oversized",
                    true,
                    false,
                )
                .await?;
                prepare_current_session(&h).await?;
                seed::house_and_items(&h, id(41), 2001).await?;
                seed::overdue(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let deps = Dependencies::new()?;
                let owner = deps.owner(&h, &authority);
                assert!(owner.drain_source_house_dispositions().await.is_err());
                assert_eq!(
                    sqlx::query_scalar::<_, i16>("SELECT state FROM game_house_dispositions")
                        .fetch_one(&h.pool)
                        .await?,
                    1
                );
                assert_eq!(
                    sqlx::query_scalar::<_, i64>(
                        "SELECT count(*) FROM game_house_disposition_census"
                    )
                    .fetch_one(&h.pool)
                    .await?,
                    0
                );
                assert_eq!(
                    sqlx::query_scalar::<_, i64>(
                        "SELECT count(*) FROM game_item_house_interior_locations"
                    )
                    .fetch_one(&h.pool)
                    .await?,
                    2001
                );
                drop(owner);
                drop(authority);
                drop(seal);
                h.cleanup().await
            })
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "oversized census panicked")??;
    Ok(())
}
