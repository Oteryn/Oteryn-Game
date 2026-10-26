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
                    command_id: 6,
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
