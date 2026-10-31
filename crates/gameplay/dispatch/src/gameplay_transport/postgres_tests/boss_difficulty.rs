//! W21: actual command router -> fenced durable draft/receipt -> client reply and row read-back.
use super::fixture::{self, Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use oteryn_protocol_oteryn::boss_difficulty::{self as wire, Refusal, Reply, Request};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use tokio::io::AsyncWriteExt;

const BOSS: &str = "oteryn:boss.difficulty_fixture";

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let before = *admitted;
    let (mut server, mut client) = tokio::io::duplex(4096);
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
                    command_type: wire::COMMAND_TYPE_BOSS_DIFFICULTY,
                    payload,
                },
            )
            .map_err(debug)?,
        )
        .await?;
        let reply = loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::CommandResult {
                let result = decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(result.command_id, id);
                assert!(result.payload.len() <= wire::MAX_RESULT_BYTES);
                break (result.status, result.payload.to_vec());
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
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = &ended
    {
        *admitted = *current;
    }
    result.map_err(|error| format!("difficulty command {id} ended {ended:?}: {error}").into())
}

async fn send(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    request: &Request,
) -> TestResult<(u64, Vec<u8>)> {
    let id = admitted.continuity.next_command_id;
    let (status, payload) = exchange(
        owner,
        admitted,
        id,
        &wire::encode_request(request).map_err(debug)?,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    Ok((id, payload))
}

fn window(context: [u8; 16], revision: &str, selected_difficulty: u16) -> TestResult<Vec<u8>> {
    wire::encode_reply(&Reply::Window {
        context,
        boss_key: BOSS.into(),
        content_revision: revision.into(),
        selected_difficulty,
    })
    .map_err(debug)
}

#[test]
fn boss_difficulty_postgres_real_dispatch_context_preview_cancel_and_refusal() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("BOSS_DIFFICULTY POSTGRES_CASE_SKIPPED: no admin URL");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "boss difficulty E2E panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "boss_difficulty_wire", true, false).await?;
        prepare_current_session(&h).await?;
        let deps = Dependencies::new()?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(admitted.game_session_id)
                .await?
                .0;
        admitted.item_fence = item_fence_of(current);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        // Exercise the real bundle offer. The fixture bypasses admission negotiation only;
        // the command still crosses serve_admitted, capability routing and the durable owner.
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[
                oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1,
                wire::CAPABILITY_BOSS_DIFFICULTY_V1,
            ],
        )
        .ok_or("difficulty bundle capability not offered")?;
        let fence = admitted.item_fence.ok_or("difficulty fence missing")?;
        let open = Request::Open {
            boss_key: BOSS.into(),
        };
        let (_, absent) = send(&owner, &mut admitted, &open).await?;
        assert_eq!(
            absent,
            wire::encode_reply(&Reply::Refused {
                context: None,
                reason: Refusal::BossUnavailable
            })
            .map_err(debug)?
        );
        // Catalogue values are isolated fixture data, never game defaults or a difficulty profile.
        sqlx::query("INSERT INTO game_boss_room_catalogue(boss_key,content_revision,boss_creature_key,minimum_players,maximum_players,minimum_level,cooldown_seconds,time_limit_seconds,lever_positions,room_template_key,reward_key,enabled) VALUES($1,'fixture-v1','oteryn:creature.fixture',1,1,1,1,1,'[]','oteryn:room.fixture','oteryn:reward.fixture',true)")
            .bind(BOSS).execute(&h.pool).await?;
        let (open_id, opened) = send(&owner, &mut admitted, &open).await?;
        let context: Vec<u8> = sqlx::query_scalar("SELECT decode(replace(context_id::text,'-',''),'hex') FROM game_boss_difficulty_contexts WHERE character_id=encode($1,'hex')::uuid AND NOT closed")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        let context: [u8; 16] = context.try_into().map_err(|_| "invalid context")?;
        assert_eq!(opened, window(context, "fixture-v1", 0)?);
        let (status, replayed) = exchange(
            &owner,
            &mut admitted,
            open_id,
            &wire::encode_request(&open).map_err(debug)?,
        )
        .await?;
        assert_eq!(status, CommandStatus::DuplicateReplay);
        assert_eq!(replayed, opened);
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_boss_difficulty_contexts WHERE character_id=encode($1,'hex')::uuid")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(count, 1);
        eprintln!("BOSS_DIFFICULTY_OPEN POSTGRES_CASE_COMPLETED");

        let preview = Request::Preview {
            context,
            difficulty: 5,
        };
        let (preview_id, previewed) = send(&owner, &mut admitted, &preview).await?;
        assert_eq!(previewed, window(context, "fixture-v1", 5)?);
        let (_, clamped) = send(
            &owner,
            &mut admitted,
            &Request::Preview {
                context,
                difficulty: u16::MAX,
            },
        )
        .await?;
        assert_eq!(clamped, window(context, "fixture-v1", 25)?);
        // Re-read the durable receipt after a later mutation: exact replay must not restore 5.
        assert_eq!(
            h.root
                .boss_difficulty_command(
                    &authority,
                    &h.node,
                    fence,
                    preview_id,
                    &wire::encode_request(&preview).map_err(debug)?
                )
                .await?,
            Some(previewed)
        );
        let selected: i16 = sqlx::query_scalar("SELECT selected_difficulty FROM game_boss_difficulty_contexts WHERE context_id=encode($1,'hex')::uuid")
            .bind(context.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(selected, 25);
        let conflict = h
            .root
            .boss_difficulty_command(
                &authority,
                &h.node,
                fence,
                preview_id,
                &wire::encode_request(&Request::Preview {
                    context,
                    difficulty: 6,
                })
                .map_err(debug)?,
            )
            .await?;
        assert_eq!(
            conflict,
            Some(
                wire::encode_reply(&Reply::Refused {
                    context: Some(context),
                    reason: Refusal::ReplayConflict
                })
                .map_err(debug)?
            )
        );
        eprintln!("BOSS_DIFFICULTY_PREVIEW POSTGRES_CASE_COMPLETED");

        for difficulty in [0, 25] {
            let (_, refused) = send(
                &owner,
                &mut admitted,
                &Request::Start {
                    context,
                    difficulty,
                },
            )
            .await?;
            assert_eq!(
                refused,
                wire::encode_reply(&Reply::Refused {
                    context: Some(context),
                    reason: Refusal::NoAcceptedProfile
                })
                .map_err(debug)?
            );
        }
        let state: (i16, bool) = sqlx::query_as("SELECT selected_difficulty,closed FROM game_boss_difficulty_contexts WHERE context_id=encode($1,'hex')::uuid")
            .bind(context.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(state, (25, false));
        let activity_count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_boss_room_activities WHERE boss_key=$1")
                .bind(BOSS)
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(activity_count, 0);
        eprintln!("BOSS_DIFFICULTY_START_REFUSAL POSTGRES_CASE_COMPLETED");

        let mut stale_connection = fence;
        stale_connection.connection_generation =
            crate::foundation::ConnectionGeneration::new(fence.connection_generation.get() + 1)
                .map_err(debug)?;
        let mut stale_lease = fence;
        stale_lease.character_lease_generation += 1;
        let mut stale_scope = fence;
        stale_scope.scope_ownership_generation = crate::foundation::ScopeOwnershipGeneration::new(
            fence.scope_ownership_generation.get() + 1,
        )
        .map_err(debug)?;
        for invalid in [stale_connection, stale_lease, stale_scope] {
            assert!(
                h.root
                    .boss_difficulty_command(
                        &authority,
                        &h.node,
                        invalid,
                        preview_id,
                        &wire::encode_request(&preview).map_err(debug)?
                    )
                    .await?
                    .is_none()
            );
        }
        let (_, foreign) = send(
            &owner,
            &mut admitted,
            &Request::Preview {
                context: [9; 16],
                difficulty: 7,
            },
        )
        .await?;
        assert_eq!(
            foreign,
            wire::encode_reply(&Reply::Refused {
                context: Some([9; 16]),
                reason: Refusal::ContextMissing
            })
            .map_err(debug)?
        );
        sqlx::query(
            "UPDATE game_boss_room_catalogue SET content_revision='fixture-v2' WHERE boss_key=$1",
        )
        .bind(BOSS)
        .execute(&h.pool)
        .await?;
        let (_, changed) = send(
            &owner,
            &mut admitted,
            &Request::Preview {
                context,
                difficulty: 7,
            },
        )
        .await?;
        assert_eq!(
            changed,
            wire::encode_reply(&Reply::Refused {
                context: Some(context),
                reason: Refusal::ContentChanged
            })
            .map_err(debug)?
        );
        eprintln!("BOSS_DIFFICULTY_FENCING POSTGRES_CASE_COMPLETED");

        let cancel = Request::Cancel { context };
        let (cancel_id, closed) = send(&owner, &mut admitted, &cancel).await?;
        assert_eq!(
            closed,
            wire::encode_reply(&Reply::Closed { context }).map_err(debug)?
        );
        assert_eq!(
            h.root
                .boss_difficulty_command(
                    &authority,
                    &h.node,
                    fence,
                    cancel_id,
                    &wire::encode_request(&cancel).map_err(debug)?
                )
                .await?,
            Some(closed)
        );
        let (_, terminal) = send(&owner, &mut admitted, &preview).await?;
        assert_eq!(
            terminal,
            wire::encode_reply(&Reply::Refused {
                context: Some(context),
                reason: Refusal::ContextClosed
            })
            .map_err(debug)?
        );
        let state: (i16, bool) = sqlx::query_as("SELECT selected_difficulty,closed FROM game_boss_difficulty_contexts WHERE context_id=encode($1,'hex')::uuid")
            .bind(context.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(state, (25, true));
        eprintln!("BOSS_DIFFICULTY_CANCEL POSTGRES_CASE_COMPLETED");
        eprintln!("BOSS_DIFFICULTY POSTGRES_CASE_COMPLETED");
        Ok(())
    })
}
