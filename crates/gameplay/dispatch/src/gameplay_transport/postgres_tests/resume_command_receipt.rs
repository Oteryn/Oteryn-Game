use super::house_transfer_fixtures::{RecoverySource, prepare_owned_session};
use super::*;
use crate::foundation::AuthenticatedTransportRefV1;
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{
    AdmissionRefusal, ConnectionEnd, ControlLossResult, FreshAdmissionAuthority, IDLE_LIVENESS,
    ResumeAttempt, serve_admitted,
};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use oteryn_protocol_oteryn::world_event::{
    CAPABILITY_WORLD_EVENT_V1, COMMAND_TYPE_WORLD_EVENT_QUERY,
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, MessageType, decode_command_result, decode_wire_envelope,
    encode_client_command,
};
use oteryn_protocol_oteryn::{quick_loot, vip};
use tokio::io::{AsyncWriteExt, DuplexStream};

#[test]
fn resume_replays_committed_world_event_after_lost_commit_reply() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, Case::WorldEvent)
}

#[test]
fn resume_command_receipt_storage_failure_is_retryable() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, Case::StorageFailure)
}

#[test]
fn resume_replays_vip_account_receipt_after_lost_commit_reply() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, Case::VipAccount)
}

#[test]
fn resume_replays_vip_query_and_friends_query_after_lost_commit_reply() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin.clone(), Case::VipQuery)?;
    run_case(admin, Case::FriendsQuery)
}

#[test]
fn resume_replays_quick_loot_review_destinations_after_lost_commit_reply() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, Case::QuickLootReview)
}

#[derive(Clone, Copy)]
enum Case {
    WorldEvent,
    StorageFailure,
    VipAccount,
    VipQuery,
    FriendsQuery,
    QuickLootReview,
}

fn run_case(admin: String, case: Case) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?
                .block_on(scenario(admin, case))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "command receipt resume thread panicked")??;
    Ok(())
}

async fn scenario(admin: String, case: Case) -> TestResult {
    let (tag, capability, command_type, payload) = match case {
        Case::VipAccount | Case::VipQuery | Case::FriendsQuery => {
            let (tag, operation) = match case {
                Case::VipQuery => ("resume_vip_query", vip::Operation::Query),
                Case::FriendsQuery => ("resume_friends_query", vip::Operation::FriendsQuery),
                _ => ("resume_vip_account", vip::Operation::MainCharacter),
            };
            let (command_type, payload) = vip::encode_request(&vip::Request {
                operation,
                number: 0,
                text: String::new(),
                icon: 0,
                notify: false,
                groups: Vec::new(),
            })
            .map_err(debug)?;
            (tag, vip::CAPABILITY_VIP_V1, command_type, payload)
        }
        Case::QuickLootReview => (
            "resume_quick_loot_review",
            quick_loot::CAPABILITY_QUICK_LOOT_DESTINATIONS_V1,
            quick_loot::COMMAND_TYPE_QUICK_LOOT_DESTINATIONS,
            quick_loot::encode_quick_loot_request(&quick_loot::QuickLootRequest {
                action: quick_loot::QuickLootAction::ReviewDestinations,
                corpse: None,
                category: None,
                recipient: None,
                recipient_id: None,
                fallback_to_main: false,
                filter_mode: None,
                filter_definition_ref: None,
            })
            .map_err(debug)?,
        ),
        Case::WorldEvent | Case::StorageFailure => {
            let tag = if matches!(case, Case::StorageFailure) {
                "resume_receipt_unavailable"
            } else {
                "resume_command_receipt"
            };
            (
                tag,
                CAPABILITY_WORLD_EVENT_V1,
                COMMAND_TYPE_WORLD_EVENT_QUERY,
                Vec::new(),
            )
        }
    };
    let quick_loot = matches!(case, Case::QuickLootReview);
    let h = Harness::create_with_admission_seed(admin, tag, quick_loot, false).await?;
    let recovery = RecoverySource::start(&h).await?;
    prepare_owned_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let (deps, content) = if quick_loot {
        let world = fence(1)?.runtime_scope.world_id();
        let (controller, pin) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                world,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let room = pin.qualified_room().clone();
        let deps = Dependencies::with_content(
            pin.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        (deps, Some(controller))
    } else {
        (Dependencies::new()?, None)
    };
    let mut owner = deps.owner(&h, &authority);
    owner.evidence = &recovery.evidence;
    owner.active_generation = content.as_ref().and_then(|controller| controller.active());
    deps.initialize(&owner).await;
    owner
        .runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let current = FreshAdmissionStore::from_root(h.root.clone())
        .current_session(admitted.game_session_id)
        .await?;
    admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
    if quick_loot {
        deps.runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                admitted.game_session_id,
                current.current_character_lease(),
            )
            .map_err(debug)?;
    }
    let supported = if quick_loot {
        vec![4, 6, 98, capability]
    } else {
        vec![capability]
    };
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &supported)
            .ok_or("receipt command capability")?;
    assert!(
        admitted
            .continuity
            .selected_capabilities
            .command_selected(command_type)
    );
    owner.states.command_ingress.admit(admitted.game_session_id);

    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let fault = match case {
            Case::VipAccount | Case::VipQuery | Case::FriendsQuery => {
                &crate::gameplay_transport::vip::STALL_WRITE_COMMIT
            }
            Case::QuickLootReview => {
                &crate::gameplay_transport::quick_loot::STALL_QUICK_LOOT_COMMIT
            }
            _ => &crate::gameplay_transport::world_event::STALL_QUERY_COMMIT,
        };
        let (end, stalled) = fault
            .scope(Cell::new(false), async {
                let end = serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS).await;
                (end, fault.with(Cell::get))
            })
            .await;
        drop(server);
        (end, stalled)
    };
    let driving = async {
        send_after_snapshot(&mut client, 1, command_type, &payload).await?;
        loop {
            match read_frame(&mut client).await {
                Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
                    return Ok::<(), Box<dyn std::error::Error>>(());
                }
                Err(error) => return Err(error.into()),
                Ok(frame) => assert_ne!(
                    decode_wire_envelope(&frame)?.message_type(),
                    MessageType::CommandResult
                ),
            }
        }
    };
    let ((end, stalled), driven) =
        tokio::time::timeout(Duration::from_secs(30), fixture::join_two(serving, driving)).await?;
    driven?;
    assert!(stalled, "fault reached the command transaction COMMIT");
    let ConnectionEnd::AdmittedThenDisconnected(lost) = end else {
        return Err(format!("unexpected connection end: {end:?}").into());
    };
    let (stored, count_query) = match case {
        Case::VipAccount | Case::VipQuery | Case::FriendsQuery => {
            let stored: serde_json::Value = sqlx::query_scalar(
                "SELECT result FROM game_account_friend_receipts WHERE command_id=1",
            )
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(stored["code"], 1);
            let payload = crate::durability::vip::encode_receipt(&stored).ok_or("VIP reply")?;
            (payload, "SELECT count(*) FROM game_account_friend_receipts")
        }
        Case::QuickLootReview => {
            let stored: Vec<u8> = sqlx::query_scalar(
                "SELECT result FROM game_quick_loot_receipts WHERE command_id=1",
            )
            .fetch_one(&h.pool)
            .await?;
            assert!(
                !quick_loot::decode_quick_loot_destinations(&stored)
                    .map_err(debug)?
                    .loot
                    .is_empty()
            );
            (stored, "SELECT count(*) FROM game_quick_loot_receipts")
        }
        Case::WorldEvent | Case::StorageFailure => {
            let stored: Vec<u8> = sqlx::query_scalar(
                "SELECT result FROM game_world_event_query_receipts WHERE command_id=1",
            )
            .fetch_one(&h.pool)
            .await?;
            (
                stored,
                "SELECT count(*) FROM game_world_event_query_receipts",
            )
        }
    };
    assert!(!owner.states.command_ingress.resumable(&lost));
    assert_eq!(
        owner.lose_control(lost, Duration::ZERO, None).await,
        ControlLossResult::Recorded
    );

    let token = RecoverySource::token(2)?;
    let attempt = || -> TestResult<ResumeAttempt<'_>> {
        Ok(ResumeAttempt {
            game_session_id: lost.game_session_id,
            recovery_material: token.as_bytes(),
            transport: AuthenticatedTransportRefV1::decode(&[2; 16]).map_err(debug)?,
            last_applied_server_sequence: lost.continuity.server_sequence,
            supported_capabilities: &supported,
        })
    };
    let mut stale = lost;
    stale.continuity.connection_generation += 1;
    owner
        .states
        .lost
        .lock()
        .map_err(debug)?
        .insert(lost.game_session_id, stale);
    assert!(matches!(
        owner.resume(attempt()?).await,
        Err(AdmissionRefusal::Rejected)
    ));
    assert!(!owner.states.command_ingress.resumable(&lost));
    owner
        .states
        .lost
        .lock()
        .map_err(debug)?
        .insert(lost.game_session_id, lost);

    if matches!(case, Case::StorageFailure) {
        // Fault only the receipt lookup. The durable session and receipt remain present.
        sqlx::query(
            "ALTER TABLE game_world_event_query_receipts RENAME COLUMN result TO offline_result",
        )
        .execute(&h.pool)
        .await?;
        let unavailable = owner.resume(attempt()?).await;
        sqlx::query(
            "ALTER TABLE game_world_event_query_receipts RENAME COLUMN offline_result TO result",
        )
        .execute(&h.pool)
        .await?;
        assert!(matches!(unavailable, Err(AdmissionRefusal::Unavailable)));
        assert!(!owner.states.command_ingress.resumable(&lost));
    }

    let resumed = owner
        .resume(attempt()?)
        .await
        .map_err(|refusal| format!("receipt resume refused: {refusal:?}"))?;
    assert_eq!(resumed.continuity.connection_generation, 2);
    assert_eq!(resumed.continuity.next_command_id, 2);
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, resumed, &owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let replaying = async {
        // Changed bytes prove the retry uses the retained outcome without re-dispatch.
        send_after_snapshot(&mut client, 2, command_type, &[0]).await?;
        let mut results = 0;
        loop {
            let frame = match read_frame(&mut client).await {
                Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(error) => return Err(error.into()),
                Ok(frame) => frame,
            };
            let envelope = decode_wire_envelope(&frame)?;
            if envelope.message_type() == MessageType::CommandResult {
                let reply = decode_command_result(envelope.payload())?;
                assert_eq!(envelope.connection_generation(), 2);
                assert_eq!(reply.command_id, 1);
                assert_eq!(reply.status, CommandStatus::DuplicateReplay);
                assert_eq!(reply.payload, stored);
                results += 1;
                client.shutdown().await?;
            }
        }
        assert_eq!(results, 1);
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let (end, replayed) = tokio::time::timeout(
        Duration::from_secs(30),
        fixture::join_two(serving, replaying),
    )
    .await?;
    replayed?;
    assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
    let count: i64 = sqlx::query_scalar(count_query).fetch_one(&h.pool).await?;
    assert_eq!(count, 1);
    eprintln!("RESUME_COMMAND_RECEIPT POSTGRES_CASE_COMPLETED");
    drop(owner);
    drop(seal);
    h.cleanup().await
}

async fn send_after_snapshot(
    client: &mut DuplexStream,
    generation: u64,
    command_type: u32,
    payload: &[u8],
) -> TestResult {
    loop {
        let frame = read_frame(client).await?;
        if decode_wire_envelope(&frame)?.message_type() == MessageType::SnapshotCommit {
            break;
        }
    }
    write_frame(
        client,
        &encode_client_command(
            generation,
            &ClientCommandValue {
                command_id: 1,
                command_type,
                payload,
            },
        )?,
    )
    .await?;
    Ok(())
}
