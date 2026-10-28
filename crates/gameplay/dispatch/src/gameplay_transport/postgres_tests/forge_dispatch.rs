//! Native DustLimit and history through the actual admitted connection router.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::domain::CharacterRevision;
use crate::durability::character_forge_dust::{
    ForgeDustGainCause, ForgeDustWriteIds, gain_forge_dust_in_transaction,
};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};
use oteryn_protocol_oteryn::{self as protocol, CommandStatus, item_forge::Request};
use tokio::io::AsyncWriteExt;

#[test]
fn forge_dispatch_native_limit_history_replay_and_stale_generation() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("forge-dispatch", scenario)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "forge_dispatch", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let generation = controller.active().ok_or("active generation")?;
        let room = pin.qualified_room().clone();
        let frame_digest = pin.frame_binding().digest();
        let deps = Dependencies::with_content(
            pin.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                deps.actor,
                crate::foundation::MovementLocalPosition {
                    x: 7,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        deps.initialize(&owner).await;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(frame_digest.as_slice()).execute(&h.pool).await?;
        let revision: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        let gameplay = CurrentCharacterGameplayFence {
            character_id: fence.character_id,
            game_session_id: fence.game_session_id,
            connection_generation: fence.connection_generation,
            character_lease_generation: fence.character_lease_generation,
            runtime_scope: fence.runtime_scope,
            scope_ownership_generation: fence.scope_ownership_generation,
            expected_character_revision: CharacterRevision::new(revision.parse()?)
                .map_err(debug)?,
        };
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        gain_forge_dust_in_transaction(
            &mut tx,
            &gameplay,
            ForgeDustGainCause::CreatureKill { occurrence: id(91) },
            100,
            ForgeDustWriteIds {
                entry_id: id(92),
                transaction_id: id(93),
            },
        )
        .await
        .map_err(debug)?;
        tx.commit().await?;

        let mut admitted = deps.admitted;
        admitted.item_fence = Some(fence);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        // First prove the wire gate rejects without a selected capability.
        for enabled in [false, true] {
            let ids: &[u32] = if enabled {
                &[4, 6, 12, 14, 152]
            } else {
                &[4, 6, 12, 14]
            };
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
                ids,
            )
            .ok_or("capabilities")?;
            let (mut server, mut client) = tokio::io::duplex(1 << 20);
            let serve = serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS);
            let first_command = admitted.continuity.next_command_id;
            let exercise = async move {
                let mut dust_limit = [0; 51];
                dust_limit[0] = 7;
                dust_limit[1..17].copy_from_slice(&id(94));
                let (status, result) = send(&mut client, first_command, &dust_limit).await?;
                if !enabled {
                    assert_eq!((status, result), (CommandStatus::Rejected, Vec::new()));
                } else {
                    assert_eq!(status, CommandStatus::Accepted);
                    let first: serde_json::Value = serde_json::from_slice(&result)?;
                    assert_eq!(first["succeeded"], true);
                    assert_eq!(first["result"]["dust_limit"], 120);
                    assert_eq!(first["result"]["dust"], 75);
                    assert_eq!(
                        send(&mut client, first_command + 1, &dust_limit).await?,
                        (CommandStatus::Accepted, result),
                        "new CommandId returns retained receipt"
                    );
                    let mut history = [0; 51];
                    history[50] = 50;
                    let (status, result) = send(&mut client, first_command + 2, &history).await?;
                    assert_eq!(status, CommandStatus::Accepted);
                    let rows: serde_json::Value = serde_json::from_slice(&result)?;
                    assert_eq!(rows.as_array().ok_or("history array")?.len(), 1);
                    assert_eq!(rows[0]["receipt"], first);
                    // Staged item conversions refuse through real dispatch, without new stores.
                    let mut conversion = dust_limit;
                    conversion[0] = 5;
                    assert_eq!(
                        send(&mut client, first_command + 3, &conversion).await?,
                        (CommandStatus::Rejected, Vec::new())
                    );
                    // A malformed payload is rejected by the canonical decoder.
                    conversion[18] = 1;
                    assert_eq!(
                        send(&mut client, first_command + 4, &conversion).await?,
                        (CommandStatus::Rejected, Vec::new())
                    );
                }
                client.shutdown().await?;
                Ok::<_, Box<dyn std::error::Error>>(())
            };
            let end =
                tokio::time::timeout(std::time::Duration::from_secs(45), drive(serve, exercise))
                    .await??;
            let ConnectionEnd::AdmittedThenDisconnected(next) = end else {
                return Err("forge connection ended unexpectedly".into());
            };
            admitted = next;
        }
        let before: (i64,i16) = sqlx::query_as("SELECT balance,dust_limit FROM game_character_forge_dust WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(before, (75, 120));
        let mut request = [0; 51];
        request[0] = 7;
        request[1..17].copy_from_slice(&id(95));
        sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=current_generation+1 WHERE game_session_id=encode($1,'hex')::uuid")
            .bind(admitted.game_session_id.as_bytes().as_slice()).execute(&h.pool).await?;
        assert_eq!(
            owner
                .item_forge(
                    admitted,
                    admitted.continuity.next_command_id,
                    Request::decode(&request).ok_or("request")?
                )
                .await,
            Some(Vec::new())
        );
        assert_eq!(before, sqlx::query_as("SELECT balance,dust_limit FROM game_character_forge_dust WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?);
        let staged: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_forge_wallets")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(staged, 0, "dispatch never creates a parallel wallet");
        let receipts: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_forge_dust_limit_receipts")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(receipts, 1);
        let denied: bool = sqlx::query_scalar("SELECT has_function_privilege('oteryn_game_runtime','game_commit_character_forge(uuid,uuid,jsonb)','EXECUTE')")
            .fetch_one(&h.pool).await?;
        assert!(!denied, "staged prototype is unavailable to runtime");
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn send(
    client: &mut tokio::io::DuplexStream,
    id: u64,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let frame = protocol::encode_client_command(
        1,
        &protocol::ClientCommandValue {
            command_id: id,
            command_type: protocol::item_forge::COMMAND_TYPE_ITEM_FORGE,
            payload,
        },
    )
    .map_err(debug)?;
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    loop {
        let frame = super::item_look::read_frame(client).await?;
        let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == protocol::MessageType::CommandResult {
            let result = protocol::decode_command_result(envelope.payload()).map_err(debug)?;
            return Ok((result.status, result.payload.to_vec()));
        }
    }
}

async fn drive<A, B>(serve: A, exercise: B) -> TestResult<ConnectionEnd>
where
    A: std::future::Future<Output = ConnectionEnd>,
    B: std::future::Future<Output = TestResult>,
{
    use std::task::Poll;
    let mut serve = std::pin::pin!(serve);
    let mut exercise = std::pin::pin!(exercise);
    let mut finished = false;
    std::future::poll_fn(|cx| {
        if !finished && let Poll::Ready(result) = exercise.as_mut().poll(cx) {
            if let Err(error) = result {
                return Poll::Ready(Err(error));
            }
            finished = true;
        }
        if let Poll::Ready(end) = serve.as_mut().poll(cx) {
            return Poll::Ready(if finished {
                Ok(end)
            } else {
                Err(format!("forge connection ended before exercise: {end:?}").into())
            });
        }
        Poll::Pending
    })
    .await
}
