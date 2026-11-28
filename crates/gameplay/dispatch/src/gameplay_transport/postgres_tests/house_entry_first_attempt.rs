//! First house entry on a cold Channel command stream.
use super::TestResult;
use super::fixture::{Harness, debug, id};
use super::house_transfer_fixtures::{
    RecoverySource, TRANSFER_TEST_HOUSE as HOUSE, create_other_house_owner,
    initialize_transfer_source_for_owner,
    prepare_owned_session, prepare_transfer_scene,
};
use crate::foundation::{GameSessionId, MessageType, decode_wire_envelope};
use crate::gameplay_transport::connection::{IDLE_LIVENESS, serve_admitted};
use crate::gameplay_transport::house_transfer::{HOUSE_CHECKOUT_BARRIER, HouseCheckoutBarrier};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use oteryn_protocol_oteryn::house_transfer::{self as wire, COMMAND_TYPE_HOUSE_TRANSFER};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use std::{sync::Arc, time::Duration};
use tokio::io::{AsyncWriteExt, DuplexStream};

#[test]
fn house_entry_first_attempt_admits_owner_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, true)
}

#[test]
fn house_entry_rejection_carries_reason_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, false)
}

fn scenario(admin: String, is_owner: bool) -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("house-entry-first-attempt", move || {
        let runtime = crate::bestiary_postgres_harness::runtime()?;
        runtime.block_on(async {
            let h = Harness::create_with_admission_seed(
                admin,
                "house_entry_first_attempt",
                true,
                false,
            )
            .await?;
            let result = exercise(&h, is_owner).await;
            h.cleanup().await?;
            result
        })
    })
}

async fn exercise(h: &Harness, is_owner: bool) -> TestResult {
    let evidence = RecoverySource::start(h).await?;
    prepare_owned_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let house_owner = if is_owner {
        id(41)
    } else {
        create_other_house_owner(h, &authority, &evidence.evidence).await?
    };
    let (controller, cells, deps) = prepare_transfer_scene(h).await?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = controller.active();
    owner.movement_cells = (&cells).into();
    owner.evidence = &evidence.evidence;
    // An owned house with no ACL grant for this character exercises the real access read.
    let mut admitted = initialize_transfer_source_for_owner(
        h,
        &owner,
        &deps,
        cells.scope().generation_digest,
        house_owner,
    )
    .await?;
    admitted.continuity.house_input_pending = Some(false);
    let role: Option<String> = sqlx::query_scalar(
        "SELECT role FROM game_house_access(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid)",
    )
    .bind(admitted.world_id.as_bytes().as_slice())
    .bind(HOUSE)
    .bind(id(41).as_slice())
    .fetch_optional(&h.pool)
    .await?;
    assert_eq!(role.as_deref(), is_owner.then_some("OWNER"));

    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let barrier = Arc::new(HouseCheckoutBarrier::default());
    let serving = async {
        let serve = serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS);
        let end = if is_owner {
            HOUSE_CHECKOUT_BARRIER
                .scope(Arc::clone(&barrier), serve)
                .await
        } else {
            serve.await
        };
        drop(server);
        Ok(end)
    };
    let commands = async {
        read_message(&mut client, MessageType::SnapshotCommit).await?;
        let command = entry_status(&mut client, admitted.continuity.connection_generation, 1);
        let (status, payload) = if is_owner {
            // Force a single real checkout failure after the account publication,
            // checkpoints and the exact entrance check have all completed.
            let contention = async {
                barrier.ready.notified().await;
                let held = h.root.try_issue_semantic_pass()?;
                barrier.attempt.notify_one();
                barrier.busy.notified().await;
                held.run(|_, _| Box::pin(async { Ok(()) })).await?;
                Ok::<_, Box<dyn std::error::Error>>(())
            };
            tokio::try_join!(command, contention)?.0
        } else {
            command.await?
        };
        if is_owner {
            assert_eq!(
                status,
                CommandStatus::Accepted,
                "refusal payload {payload:?}"
            );
            read_message(&mut client, MessageType::SnapshotCommit).await?;
            let recovery = h
                .root
                .house_transfer_recovery(admitted.game_session_id)
                .await?
                .ok_or("committed house entry")?;
            let destination = GameSessionId::decode(&recovery.ack.session).map_err(debug)?;
            let inside = owner
                .runtime
                .lock()
                .await
                .house_actor_state(deps.actor, destination)
                .map_err(debug)?;
            assert_eq!(inside.house, HOUSE);
            let entry =
                crate::gameplay_transport::house_query::entrance(HOUSE).ok_or("house entrance")?;
            assert_eq!(
                inside.position,
                crate::foundation::LocalPosition {
                    x: entry.x,
                    y: entry.y - 1,
                    floor: entry.floor,
                }
            );
        } else {
            assert_eq!(status, CommandStatus::Rejected);
            assert_eq!(
                wire::decode_house_transfer_refusal(&payload),
                Some(wire::HouseTransferRefusal::NoAccess)
            );
            assert!(
                h.root
                    .house_transfer_recovery(admitted.game_session_id)
                    .await?
                    .is_none()
            );
            let position = owner
                .runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position();
            let entrance =
                crate::gameplay_transport::house_query::entrance(HOUSE).ok_or("house entrance")?;
            assert_eq!(
                (position.x, position.y, position.floor),
                (entrance.x, entrance.y, entrance.floor)
            );
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    tokio::time::timeout(Duration::from_secs(20), async {
        tokio::try_join!(serving, commands)
    })
    .await??;
    eprintln!("HOUSE_FIRST_ENTRY_POSTGRES_COMPLETED owner={is_owner}");
    Ok(())
}

async fn read_message(client: &mut DuplexStream, kind: MessageType) -> TestResult<Vec<u8>> {
    loop {
        let frame = read_frame(client).await?;
        if decode_wire_envelope(&frame)?.message_type() == kind {
            return Ok(frame);
        }
    }
}

async fn entry_status(
    client: &mut DuplexStream,
    generation: u64,
    id: u64,
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let mut payload = vec![8, 1, 18, HOUSE.len() as u8];
    payload.extend_from_slice(HOUSE.as_bytes());
    let frame = encode_client_command(
        generation,
        &ClientCommandValue {
            command_id: id,
            command_type: COMMAND_TYPE_HOUSE_TRANSFER,
            payload: &payload,
        },
    )?;
    write_frame(client, &frame).await?;
    let reply = read_message(client, MessageType::CommandResult)
        .await
        .map_err(|error| format!("missing house-entry reply for command {id}: {error}"))?;
    let result = decode_command_result(decode_wire_envelope(&reply)?.payload())?;
    assert_eq!(result.command_id, id);
    Ok((result.status, result.payload.to_vec()))
}
