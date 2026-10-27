//! Command 133 through the real admitted connection, fenced PostgreSQL read and wire result.
use super::fixture::{self, Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
    serve_admitted,
};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use oteryn_protocol_oteryn::boss_encounter::*;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use tokio::io::AsyncWriteExt;

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
                    command_type: COMMAND_TYPE_BOSS_COOLDOWNS,
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
                assert!(r.payload.len() <= MAX_RESULT_BYTES);
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
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = &ended
    {
        *admitted = *current;
    }
    result.map_err(|error| format!("cooldown command {id} ended {ended:?}: {error}").into())
}

// Keep one connection alive across the changes. Its initial valid command
// reconciles carried commands before the participant/interpretation changes;
// later commands exercise the cooldown handler rather than setup/reconciliation.
async fn qualify_fence_changes(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let pool = &h.pool;
    let activity = prepare_unadmitted_activity(h).await?;
    let before = *admitted;
    let fence = before.item_fence.ok_or("missing cooldown fence")?;
    let (mut server, mut client) = tokio::io::duplex(4096);
    let serving = async {
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
        let generation = before.continuity.connection_generation;
        let mut command_id = before.continuity.next_command_id;
        let (status, payload) = query(&mut client, generation, command_id).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let (_, expected_entries, expected_next) = page(&payload)?;
        assert_eq!(expected_entries.len(), 64);
        command_id += 1;
        // Refused admission must leave this already admitted connection usable.
        // These are administrative failure probes, not a handoff implementation.
        for attempt in 0..3 {
            refuse_unproven_activity_handoff(h, before, activity, attempt).await?;
            let (status, payload) = query(&mut client, generation, command_id).await?;
            assert_eq!(status, CommandStatus::Accepted, "handoff probe {attempt}");
            let (_, entries, next) = page(&payload)?;
            assert_eq!(entries, expected_entries);
            assert_eq!(next, expected_next);
            command_id += 1;
        }
        // Administrative fixture mutations isolate each live participant fact.
        // They grant no session or handoff authority and change no immutable
        // publication history. Each refusal must produce a CommandResult on
        // this same connection; a disconnect is a failure, not qualification.
        for (label, table, changed, restored) in [
            (
                "Character eligibility",
                "game_durability_admission_character_guards",
                "eligible=false",
                "eligible=true",
            ),
            (
                "Character lease generation",
                "game_durability_admission_character_guards",
                "lease_generation=lease_generation+1",
                "lease_generation=lease_generation-1",
            ),
            (
                "Character lease holder",
                "game_durability_admission_character_guards",
                "holder_game_session_id=NULL",
                "holder_game_session_id=encode($2,'hex')::uuid",
            ),
            (
                "Character account",
                "game_durability_admission_character_guards",
                "account_id=encode($3,'hex')::uuid",
                "account_id=(SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid)",
            ),
            (
                "Character World",
                "game_durability_admission_character_guards",
                "world_id=encode($3,'hex')::uuid",
                "world_id=(SELECT world_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid)",
            ),
            (
                "account presence Character",
                "game_durability_admission_account_guards",
                "presence_character_id=encode($3,'hex')::uuid",
                "presence_character_id=encode($1,'hex')::uuid",
            ),
            (
                "account presence release",
                "game_durability_admission_account_guards",
                "presence_character_id=NULL,holder_game_session_id=NULL",
                "presence_character_id=encode($1,'hex')::uuid,holder_game_session_id=encode($2,'hex')::uuid",
            ),
        ] {
            let predicate = if table == "game_durability_admission_character_guards" {
                "character_id=encode($1,'hex')::uuid"
            } else {
                "account_id=(SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid)"
            };
            for (set, expected) in [
                (changed, CommandStatus::Rejected),
                (restored, CommandStatus::Accepted),
            ] {
                // All SQL fragments above are fixed test literals. The three
                // typed UUID parameters have fixed casts even when a particular
                // case uses only one of them.
                let updated = sqlx::query(sqlx::AssertSqlSafe(format!(
                    "UPDATE {table} SET {set} WHERE {predicate} AND encode($2,'hex')::uuid IS NOT NULL AND encode($3,'hex')::uuid IS NOT NULL"
                )))
                .bind(fence.character_id.as_bytes().as_slice())
                .bind(fence.game_session_id.as_bytes().as_slice())
                .bind(fixture::id(91).as_slice())
                .execute(pool)
                .await?;
                assert_eq!(updated.rows_affected(), 1, "{label}");
                let durable = owner
                    .root
                    .read_boss_cooldown_list(owner.character, owner.holder, fence, &[])
                    .await?;
                assert_eq!(
                    durable.is_some(),
                    expected == CommandStatus::Accepted,
                    "{label}"
                );
                let (status, payload) = query(&mut client, generation, command_id).await?;
                assert_eq!(status, expected, "{label}");
                if expected == CommandStatus::Accepted {
                    let (_, entries, next) = page(&payload)?;
                    assert_eq!(entries, expected_entries, "{label}");
                    assert_eq!(next, expected_next, "{label}");
                } else {
                    assert!(payload.is_empty(), "{label}");
                }
                command_id += 1;
            }
        }
        for index in 0..4 {
            let mut revisions = ["profile-1", "ruleset-1", "content-1", "starter-1"];
            revisions[index] = "boss-incompatible-2";
            sqlx::query("SELECT game_character_configure_interpretation($1,$2,$3,$4)")
                .bind(revisions[0])
                .bind(revisions[1])
                .bind(revisions[2])
                .bind(revisions[3])
                .execute(pool)
                .await?;
            assert!(
                owner
                    .root
                    .read_boss_cooldown_list(owner.character, owner.holder, fence, &[])
                    .await?
                    .is_none(),
                "interpretation column {index}"
            );
            let (status, payload) = query(&mut client, generation, command_id).await?;
            assert_eq!(
                status,
                CommandStatus::Rejected,
                "interpretation column {index}"
            );
            assert!(payload.is_empty());
            command_id += 1;
            // Restore by publishing a new interpretation, preserving history.
            sqlx::query("SELECT game_character_configure_interpretation('profile-1','ruleset-1','content-1','starter-1')")
                .execute(pool).await?;
            let (status, payload) = query(&mut client, generation, command_id).await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert_eq!(page(&payload)?.1.len(), 64);
            command_id += 1;
        }
        client.shutdown().await?;
        Ok(())
    };
    let (ended, result): (_, TestResult) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        fixture::join_two(serving, driving),
    )
    .await?;
    result?;
    let ConnectionEnd::AdmittedThenDisconnected(current) = ended else {
        return Err(format!("participant/interpretation connection ended {ended:?}").into());
    };
    *admitted = current;
    Ok(())
}

// Allocate and attest only the runtime. A ready assignment conveys no
// participant provenance and must never make any of the probes below succeed.
async fn prepare_unadmitted_activity(
    h: &Harness,
) -> TestResult<crate::foundation::RuntimeScopeRefV1> {
    use super::fresh_fixture::fresh::Source;
    use crate::durability::admission_authority_guards::GuardPublicationDisposition;
    use crate::durability::runtime_scope_assignment::{
        AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
        RuntimeScopeAssignmentWriter,
    };
    use crate::foundation::admission_authority_publication::{
        AdmissionAuthorityGuardKeyV1, AdmissionAuthorityPublicationV1,
    };
    let scope = crate::foundation::RuntimeScopeRefV1::instance(
        fixture::fence(1)?.runtime_scope.world_id(),
        fixture::id(88),
    )
    .map_err(debug)?;
    sqlx::query("INSERT INTO game_control_activity_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)")
        .bind(scope.world_id().as_bytes().as_slice()).bind(fixture::id(88).as_slice())
        .execute(&h.pool).await?;
    let writer = RuntimeScopeAssignmentWriter::open(h.root.clone(), "sequencer-writer")
        .await
        .map_err(debug)?;
    let outcome = writer
        .submit(&AssignmentRequest {
            operation_key: OperationKey::from_bytes([88; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            command: AssignmentCommand::Assign {
                scope,
                target: h.node.fact(),
            },
        })
        .await
        .map_err(debug)?;
    let AssignmentOutcome::Committed(receipt) = outcome else {
        return Err(format!("activity allocation refused: {outcome:?}").into());
    };
    assert_eq!(receipt.assignment.scope, scope);
    assert_eq!(receipt.assignment.ownership_generation, 1);
    let now = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
        .fetch_one(&h.pool)
        .await?;
    let mut source = Source::new(now)?;
    source
        .rows
        .retain(|row| matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
    assert_eq!(source.rows.len(), 1);
    source.rows[0].key = AdmissionAuthorityGuardKeyV1::Runtime(scope);
    let publication = AdmissionAuthorityPublicationV1::prepare(&source, now).map_err(debug)?;
    assert_eq!(
        h.root
            .publish_runtime_readiness(&h.node, &publication)
            .await
            .map_err(debug)?,
        GuardPublicationDisposition::Applied
    );
    Ok(scope)
}

// Hash the exact bounded participant rows rather than print account/session
// data in assertion failures. No query scans other Characters or their history.
async fn participant_binding(h: &Harness, admitted: AdmittedSession) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT md5(jsonb_build_array(to_jsonb(s),to_jsonb(c),to_jsonb(a),to_jsonb(r))::text) \
         FROM game_durability_reconnect_sessions s \
         JOIN game_durability_admission_character_guards c ON c.character_id=s.character_id \
         JOIN game_durability_admission_account_guards a ON a.account_id=s.account_id \
         JOIN game_character_roots r ON r.character_id=s.character_id \
         WHERE s.game_session_id=encode($1,'hex')::uuid",
    )
    .bind(admitted.game_session_id.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?)
}

async fn refuse_unproven_activity_handoff(
    h: &Harness,
    admitted: AdmittedSession,
    activity: crate::foundation::RuntimeScopeRefV1,
    attempt: u8,
) -> TestResult {
    let crate::foundation::RuntimeScopeRefV1::Instance { instance_id, .. } = activity else {
        return Err("activity probe requires Instance".into());
    };
    let original = participant_binding(h, admitted).await?;
    let candidate = fixture::id(90 + attempt);
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL statement_timeout='5s'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT 1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(admitted.game_session_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
    let error = if attempt == 0 {
        // Updating the existing source cannot stand in for committed provenance.
        sqlx::query("UPDATE game_durability_reconnect_sessions SET runtime_scope_kind=2,runtime_scope_channel_id=NULL,runtime_scope_instance_id=encode($2,'hex')::uuid WHERE game_session_id=encode($1,'hex')::uuid")
            .bind(admitted.game_session_id.as_bytes().as_slice()).bind(instance_id.as_slice())
            .execute(&mut *tx).await.expect_err("unproven scope update must refuse")
    } else {
        // Simulate the dangerous half of a handoff in one transaction: retire
        // source, advance/release its lease and clear presence, then try an
        // unproven destination.
        // Rollback must restore every one of those facts, not merely the scope.
        let retired = sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3,current_transport_ref=NULL,prepared_attempt_ref=NULL WHERE game_session_id=encode($1,'hex')::uuid AND session_state IN (1,2)")
            .bind(admitted.game_session_id.as_bytes().as_slice()).execute(&mut *tx).await?;
        assert_eq!(retired.rows_affected(), 1);
        let leased = sqlx::query("UPDATE game_durability_admission_character_guards SET lease_generation=lease_generation+1,holder_game_session_id=NULL WHERE character_id=(SELECT character_id FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid) AND holder_game_session_id=encode($1,'hex')::uuid")
            .bind(admitted.game_session_id.as_bytes().as_slice())
            .execute(&mut *tx).await?;
        assert_eq!(leased.rows_affected(), 1);
        let present = sqlx::query("UPDATE game_durability_admission_account_guards SET presence_character_id=NULL,holder_game_session_id=NULL WHERE account_id=(SELECT account_id FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid) AND holder_game_session_id=encode($1,'hex')::uuid")
            .bind(admitted.game_session_id.as_bytes().as_slice())
            .execute(&mut *tx).await?;
        assert_eq!(present.rows_affected(), 1);
        // A terminal administrative row is allowed as history, but activating
        // it must prove the same handoff as a fresh INSERT. Neither is admission.
        let inserted = sqlx::query(
            "INSERT INTO game_durability_reconnect_sessions \
             (game_session_id,account_id,character_id,world_id,runtime_scope_kind, \
              runtime_scope_world_id,runtime_scope_channel_id,runtime_scope_instance_id, \
              control_loss_epoch,original_grace_deadline,predecessor_generation, \
              character_lease_generation,scope_ownership_generation,current_generation,session_state) \
             SELECT encode($2,'hex')::uuid,account_id,character_id,world_id,2, \
                    runtime_scope_world_id,NULL,encode($3,'hex')::uuid, \
                    1,COALESCE(original_grace_deadline,floor(extract(epoch FROM clock_timestamp()))::bigint),1, \
                    character_lease_generation+1,1,1,$4 \
             FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid")
            .bind(admitted.game_session_id.as_bytes().as_slice()).bind(candidate.as_slice())
            .bind(instance_id.as_slice()).bind(if attempt == 1 { 1_i16 } else { 3_i16 })
            .execute(&mut *tx).await;
        if attempt == 1 {
            inserted.expect_err("unproven destination INSERT must refuse")
        } else {
            assert_eq!(inserted?.rows_affected(), 1);
            sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=1 WHERE game_session_id=encode($1,'hex')::uuid")
                .bind(candidate.as_slice()).execute(&mut *tx).await
                .expect_err("unproven terminal destination activation must refuse")
        }
    };
    // Check the precise provenance guard, not an unrelated UNIQUE/CHECK error.
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("23514")
    );
    assert!(
        error
            .to_string()
            .contains("activity session requires a committed activity handoff")
    );
    tx.rollback().await?;
    assert_eq!(participant_binding(h, admitted).await?, original);
    let retained: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid)")
        .bind(candidate.as_slice()).fetch_one(&h.pool).await?;
    assert!(!retained, "failed handoff retained its destination");
    Ok(())
}

async fn query(
    client: &mut tokio::io::DuplexStream,
    generation: u64,
    command_id: u64,
) -> TestResult<(CommandStatus, Vec<u8>)> {
    write_frame(
        client,
        &encode_client_command(
            generation,
            &ClientCommandValue {
                command_id,
                command_type: COMMAND_TYPE_BOSS_COOLDOWNS,
                payload: &[],
            },
        )
        .map_err(debug)?,
    )
    .await?;
    loop {
        let frame = read_frame(client).await?;
        let envelope = decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(result.command_id, command_id);
            assert!(result.payload.len() <= MAX_RESULT_BYTES);
            return Ok((result.status, result.payload.to_vec()));
        }
    }
}

// Change durable authority only after the real Channel snapshot committed.
// This is a bare FND-04 Instance session, not a fabricated activity admission.
// The old Channel actor and its selected capability cannot keep reading for it.
async fn refuse_live_instance_scope(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    pool: &sqlx::PgPool,
) -> TestResult {
    let (mut server, mut client) = tokio::io::duplex(4096);
    let serving = async {
        let ended = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
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
        let updated = sqlx::query("UPDATE game_durability_reconnect_sessions SET runtime_scope_kind=2,runtime_scope_channel_id=NULL,runtime_scope_instance_id=encode($1,'hex')::uuid WHERE game_session_id=encode($2,'hex')::uuid")
            .bind(fixture::id(89).as_slice()).bind(admitted.game_session_id.as_bytes().as_slice())
            .execute(pool).await?;
        assert_eq!(updated.rows_affected(), 1);
        let sent = write_frame(
            &mut client,
            &encode_client_command(
                admitted.continuity.connection_generation,
                &ClientCommandValue {
                    command_id: admitted.continuity.next_command_id,
                    command_type: COMMAND_TYPE_BOSS_COOLDOWNS,
                    payload: &[],
                },
            )
            .map_err(debug)?,
        )
        .await;
        // The live session refresh may detect the changed scope before this
        // write. A closed peer is the only permitted send error; either order
        // must end in EOF without a command result below.
        if let Err(error) = sent {
            assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
        }
        // A stale current-session fence terminates the connection before the
        // command can disclose rows. Only clean EOF is a successful refusal.
        let error = read_frame(&mut client)
            .await
            .err()
            .ok_or("stale scope emitted a frame")?;
        assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof);
        Ok(())
    };
    let (ended, result): (_, TestResult) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        fixture::join_two(serving, driving),
    )
    .await?;
    result.map_err(|error| format!("live Instance scope ended {ended:?}: {error}"))?;
    assert!(
        matches!(ended, ConnectionEnd::AdmittedThenDisconnected(_)),
        "{ended:?}"
    );
    Ok(())
}

fn varint(bytes: &[u8], at: &mut usize) -> TestResult<u64> {
    let mut value = 0;
    for shift in (0..70).step_by(7) {
        let b = *bytes.get(*at).ok_or("short varint")?;
        *at += 1;
        if shift == 63 && b > 1 {
            return Err("varint overflow".into());
        }
        value |= u64::from(b & 127) << shift;
        if b < 128 {
            return Ok(value);
        }
    }
    Err("varint overflow".into())
}
fn bytes<'a>(input: &'a [u8], at: &mut usize) -> TestResult<&'a [u8]> {
    let len = usize::try_from(varint(input, at)?)?;
    let end = at.checked_add(len).ok_or("length overflow")?;
    let value = input.get(*at..end).ok_or("short bytes")?;
    *at = end;
    Ok(value)
}
type CooldownPage = (u64, Vec<(String, u64)>, String);

fn page(payload: &[u8]) -> TestResult<CooldownPage> {
    let mut at = 0;
    assert_eq!(varint(payload, &mut at)?, 8);
    let now = varint(payload, &mut at)?;
    let mut entries = Vec::new();
    let mut next = String::new();
    while at < payload.len() {
        match varint(payload, &mut at)? {
            18 => {
                let entry = bytes(payload, &mut at)?;
                let mut ea = 0;
                assert_eq!(varint(entry, &mut ea)?, 10);
                let key = std::str::from_utf8(bytes(entry, &mut ea)?)?.to_owned();
                assert_eq!(varint(entry, &mut ea)?, 16);
                let available = varint(entry, &mut ea)?;
                assert_eq!(ea, entry.len());
                entries.push((key, available));
            }
            26 => {
                next = std::str::from_utf8(bytes(payload, &mut at)?)?.to_owned();
            }
            _ => return Err("unknown result field".into()),
        }
    }
    Ok((now, entries, next))
}
fn cursor(key: &str) -> Vec<u8> {
    // Test keys fit the one-byte protobuf length. The production decoder handles 512 B.
    let mut out = vec![10, key.len() as u8];
    out.extend_from_slice(key.as_bytes());
    out
}

#[test]
fn boss_cooldown_list_postgres_real_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "boss cooldown E2E panicked")??;
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "boss_cooldowns_wire", true, false).await?;
        prepare_current_session(&h).await?;
        eprintln!("Canonical PostgreSQL 18.6: boss cooldown real dispatch qualification");
        use crate::durability::guild_chat_fixture as characters;
        let other =
            crate::domain::CharacterId::from_bytes(characters::sid(90, 1)).map_err(debug)?;
        characters::historical_character(
            &h.pool,
            90,
            other,
            characters::sid(90, 2),
            fixture::id(42),
            "Alice",
        )
        .await?;
        sqlx::query("INSERT INTO game_character_boss_cooldowns(character_id,boss_key,available_at,admission,reset_epoch) VALUES(encode($1,'hex')::uuid,'oteryn:boss.private',0,encode($2,'hex')::uuid,0)")
            .bind(other.as_bytes().as_slice()).bind(characters::sid(90, 3).as_slice()).execute(&h.pool).await?;
        let deps = Dependencies::new()?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(admitted.game_session_id)
                .await?
                .0;
        admitted.item_fence = item_fence_of(current);
        let unselected = admitted;
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
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[CAPABILITY_BOSS_ENCOUNTER_V1],
        )
        .ok_or("boss capability not offered")?;
        // A typed Instance handle cannot borrow the current Channel player's slot, even
        // when World, UUID bytes and every actor/session generation are identical.
        let scope = deps.actor.runtime_scope();
        assert_eq!(
            scope,
            crate::foundation::RuntimeScopeRefV1::channel(admitted.world_id, admitted.channel_id)
        );
        assert_eq!(deps.actor.channel_id(), Some(admitted.channel_id));
        let instance_actor =
            crate::foundation::ExactActorRef(crate::foundation::ActorRef::from_scoped_owner_parts(
                crate::foundation::RuntimeScopeRefV1::instance(
                    admitted.world_id,
                    *admitted.channel_id.as_bytes(),
                )
                .map_err(debug)?,
                deps.actor.scope_generation(),
                deps.actor.0.actor_local_id,
                deps.actor.0.actor_local_generation,
            ));
        assert_eq!(instance_actor.channel_id(), None);
        {
            let mut runtime = owner.runtime.lock().await;
            let before = runtime.read_actor_position(deps.actor).map_err(debug)?;
            assert_eq!(
                runtime.read_actor_position(instance_actor),
                Err(crate::foundation::CarrierError::WrongScope)
            );
            assert_eq!(
                runtime.borrow_movement_position().read(instance_actor),
                Err(crate::foundation::CarrierError::WrongScope)
            );
            assert_eq!(
                runtime.read_actor_position(deps.actor).map_err(debug)?,
                before
            );
            assert_eq!(before.context(), runtime.pinned_movement_context());
        }
        let mut wrong_actor = admitted;
        wrong_actor.runtime_actor = Some(instance_actor);
        assert!(matches!(
            owner
                .boss_cooldowns_command(wrong_actor, 1, COMMAND_TYPE_BOSS_COOLDOWNS, &[])
                .await,
            crate::gameplay_transport::guild_intent::GuildReply::Rejected
        ));
        assert!(
            owner
                .runtime
                .lock()
                .await
                .player_control_facts(deps.actor, admitted.game_session_id)
                .is_ok()
        );
        // Empty is a real accepted read, not a missing-table or unsupported fallback.
        let (status, payload) = exchange(&owner, &mut admitted, 1, &[]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let (now, entries, next) = page(&payload)?;
        assert!(now > 0);
        assert!(entries.is_empty() && next.is_empty());
        let fence = admitted.item_fence.ok_or("fence")?;
        // Seed admission outcomes through the admin connection. This test qualifies the
        // dispatched read, not a boss-admission writer. Deadlines include expired rows.
        for i in 0..66 {
            sqlx::query("INSERT INTO game_character_boss_cooldowns(character_id,boss_key,available_at,admission,reset_epoch) VALUES(encode($1,'hex')::uuid,$2,$3,encode($4,'hex')::uuid,0)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(format!("oteryn:boss.{i:03}"))
                .bind(i64::from(i)).bind(fence.game_session_id.as_bytes().as_slice())
                .execute(&h.pool).await?;
        }
        let (status, payload) = exchange(&owner, &mut admitted, 2, &[]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let (now, entries, next) = page(&payload)?;
        assert!(now > 0);
        assert_eq!(entries.len(), 64);
        assert_eq!(entries.first(), Some(&("oteryn:boss.000".into(), 0)));
        assert_eq!(entries.last(), Some(&("oteryn:boss.063".into(), 63)));
        assert_eq!(next, "oteryn:boss.063");
        let (status, second) = exchange(&owner, &mut admitted, 3, &cursor(&next)).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let (_, entries, next) = page(&second)?;
        assert_eq!(
            entries,
            vec![
                ("oteryn:boss.064".into(), 64),
                ("oteryn:boss.065".into(), 65)
            ]
        );
        assert!(next.is_empty());
        let (status, replayed) =
            exchange(&owner, &mut admitted, 3, &cursor("oteryn:boss.063")).await?;
        assert_eq!(status, CommandStatus::DuplicateReplay);
        assert_eq!(replayed, second);
        let (status, payload) = exchange(&owner, &mut admitted, 4, &[10, 0, 10, 0]).await?;
        assert_eq!(status, CommandStatus::Rejected);
        assert!(payload.is_empty());
        let mut without_capability = admitted;
        without_capability.continuity.selected_capabilities =
            unselected.continuity.selected_capabilities;
        let (status, payload) = exchange(&owner, &mut without_capability, 5, &[]).await?;
        assert_eq!(status, CommandStatus::Rejected);
        assert!(payload.is_empty());
        // The command stream is shared by both capability selections. Carry
        // the returned sequence/command continuity into the selected connection.
        let selected = admitted.continuity.selected_capabilities;
        admitted = without_capability;
        admitted.continuity.selected_capabilities = selected;
        use crate::gameplay_transport::guild_intent::GuildReply;
        assert!(matches!(
            owner
                .boss_cooldowns_command(unselected, 1, COMMAND_TYPE_BOSS_COOLDOWNS, &[])
                .await,
            GuildReply::Rejected
        ));
        let mut stale = admitted;
        stale.continuity.connection_generation += 1;
        assert!(matches!(
            owner
                .boss_cooldowns_command(stale, 5, COMMAND_TYPE_BOSS_COOLDOWNS, &[])
                .await,
            GuildReply::Rejected
        ));
        let mut stale = fence;
        stale.character_lease_generation += 1;
        assert!(
            h.root
                .read_boss_cooldown_list(&authority, &h.node, stale, &[])
                .await?
                .is_none()
        );
        let mut wrong_character = fence;
        wrong_character.character_id = other;
        assert!(
            h.root
                .read_boss_cooldown_list(&authority, &h.node, wrong_character, &[])
                .await?
                .is_none()
        );
        let mut stale_connection = fence;
        stale_connection.connection_generation =
            crate::foundation::ConnectionGeneration::new(fence.connection_generation.get() + 1)
                .map_err(debug)?;
        assert!(
            h.root
                .read_boss_cooldown_list(&authority, &h.node, stale_connection, &[])
                .await?
                .is_none()
        );
        let mut stale_scope = fence;
        stale_scope.scope_ownership_generation = crate::foundation::ScopeOwnershipGeneration::new(
            fence.scope_ownership_generation.get() + 1,
        )
        .map_err(debug)?;
        assert!(
            h.root
                .read_boss_cooldown_list(&authority, &h.node, stale_scope, &[])
                .await?
                .is_none()
        );
        // An Instance scope cannot borrow the admitted Channel actor, even if
        // its raw UUID bytes equal the Channel's. Exercise the composed and
        // durable boundaries independently, keeping every other fact unchanged.
        // The ordinary Channel traffic above exercises the transport; direct
        // calls here isolate the scope check from the other connection gates.
        let mut forged_instance = admitted;
        let mut forged_fence = fence;
        forged_fence.runtime_scope = crate::foundation::RuntimeScopeRefV1::instance(
            admitted.world_id,
            *admitted.channel_id.as_bytes(),
        )
        .map_err(debug)?;
        forged_instance.item_fence = Some(forged_fence);
        assert!(matches!(
            owner
                .boss_cooldowns_command(forged_instance, 5, COMMAND_TYPE_BOSS_COOLDOWNS, &[])
                .await,
            GuildReply::Rejected
        ));
        assert!(
            h.root
                .read_boss_cooldown_list(&authority, &h.node, forged_fence, &[])
                .await?
                .is_none()
        );
        qualify_fence_changes(&owner, &mut admitted, &h).await?;
        refuse_live_instance_scope(&owner, admitted, &h.pool).await?;
        // No query changes the Character revision or its cooldown rows.
        let revision: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(revision, "1");
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_boss_cooldowns")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(count, 67);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
