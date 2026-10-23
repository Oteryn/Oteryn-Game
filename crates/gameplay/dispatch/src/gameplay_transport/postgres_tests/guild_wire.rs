//! GUILD-WIRE-1: real admitted connection bytes and native Guild persistence.
#[path = "guild_wire/paging.rs"]
mod paging;

use super::fixture::{self, Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::domain::CharacterId;
use crate::durability::guild_chat_fixture as seed;
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use oteryn_protocol_oteryn::guild::*;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use tokio::io::AsyncWriteExt;

#[test]
fn guild_wire_postgres_real_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "guild wire e2e panicked")??;
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
        | crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenClosed(current, _) => *admitted = *current,
        _ => {}
    }
    result.map_err(|error| format!("guild command {id}/{kind} ended {ended:?}: {error}").into())
}
async fn intent(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    action: GuildIntent,
    code: u8,
) -> TestResult<GuildIntentResult> {
    let (status, bytes) = exchange(
        owner,
        admitted,
        id,
        COMMAND_TYPE_GUILD_INTENT,
        &encode_guild_intent(&action).map_err(debug)?,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    let r = decode_guild_intent_result(&bytes).map_err(debug)?;
    assert_eq!(r.code, code);
    Ok(r)
}
async fn query(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    q: GuildQuery,
) -> TestResult<GuildQueryResult> {
    let (status, bytes) = exchange(
        owner,
        admitted,
        id,
        COMMAND_TYPE_GUILD_QUERY,
        &encode_guild_query(&q).map_err(debug)?,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    decode_guild_query_result(&bytes).map_err(debug)
}
fn scenario() -> TestResult {
    run(|admin| async move {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "guild_wire_dispatch", true, false).await?;
        prepare_current_session(&h).await?;
        seed::prepare(&h.pool).await?;
        sqlx::raw_sql(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../apps/game-server/migrations/0161_guild_roster_keyset.sql"
        )))
        .execute(&h.pool)
        .await?;
        let base = super::fixture::fence(1)?;
        let base = crate::durability::item_transfer::CurrentCharacterItemFence {
            character_id: base.character_id,
            game_session_id: base.game_session_id,
            connection_generation: base.connection_generation,
            character_lease_generation: 1,
            runtime_scope: base.runtime_scope,
            scope_ownership_generation: base.scope_ownership_generation,
        };
        let mut guest_fences = Vec::new();
        for (n, name) in [(90, "Guest One"), (91, "Guest Two")] {
            seed::historical_character(
                &h.pool,
                n,
                CharacterId::from_bytes(seed::sid(n, 1)).map_err(debug)?,
                seed::sid(n, 2),
                id(42),
                name,
            )
            .await?;
            guest_fences.push(seed::historical_session(&h.pool, n, base).await?);
        }
        seed::guard(&h.pool).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        // This scenario has three simultaneous native sessions; the shared fixture defaults to two.
        let mut runtime = crate::foundation::ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1, 1, 1, "runtime-scope-assignment:1", 3,
            crate::foundation::ChannelContentPin::test(deps.admitted.world_id),
        ).map_err(debug)?;
        let reservation=runtime.reserve_fresh_session(deps.admitted.game_session_id).map_err(debug)?;
        deps.actor=runtime.commit_fresh_session(reservation).map_err(debug)?;
        deps.admitted.runtime_actor=Some(deps.actor);
        deps.runtime=tokio::sync::Mutex::new(runtime);
        let coordinator =
            crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
                Default::default(),
                None,
                None,
                Some(h.root.clone()),
            );
        let mut owner = deps.owner(&h, &authority);
        owner.premium_coordinator = Some(&coordinator);
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let (current, _) =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?;
        let mut leader = deps.admitted;
        leader.first_entry = FirstEntryOutcome::Positioned;
        leader.item_fence = item_fence_of(current);
        leader.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[40])
                .ok_or("guild selection")?;
        let mut guests = Vec::new();
        for fence in guest_fences {
            let actor = {
                let mut runtime = owner.runtime.lock().await;
                let reservation = runtime
                    .reserve_fresh_session(fence.game_session_id)
                    .map_err(debug)?;
                let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
                runtime
                    .initialize_first_entry_position(actor)
                    .map_err(debug)?;
                actor
            };
            deps.initialize_player(&owner, actor, fence.game_session_id)
                .await;
            guests.push(AdmittedSession {
                game_session_id: fence.game_session_id,
                runtime_actor: Some(actor),
                item_fence: Some(fence),
                controller: None,
                ..leader
            });
        }
        let mut one = guests[0];
        let mut two = guests[1];
        let character = *one.item_fence.ok_or("guest fence")?.character_id.as_bytes();
        let second = *two.item_fence.ok_or("guest fence")?.character_id.as_bytes();
        // Prove the Guild mutation fence on a fresh command, before reconnect's independent
        // Bank reconciliation prelude can reject the same stale lease first.
        let mut stale = leader;
        stale.item_fence.as_mut().ok_or("fence")?.character_lease_generation += 1;
        assert_eq!(exchange(&owner,&mut stale,1,COMMAND_TYPE_GUILD_INTENT,
            &encode_guild_intent(&GuildIntent::Found{name:"Wire Guild".into()}).map_err(debug)?
        ).await?.0,CommandStatus::Rejected);
        let before_ack = leader;
        let found = GuildIntent::Found {
            name: "Wire Guild".into(),
        };
        let r = intent(&owner, &mut leader, 1, found.clone(), 1).await?;
        let guild = r.guild;
        assert_eq!(r.revision, Some(1));
        // Reconnect from before acknowledgement: byte-identical receipt, despite a fresh clock read.
        let mut replay = before_ack;
        assert_eq!(intent(&owner, &mut replay, 1, found, 1).await?, r);
        let mut changed = before_ack;
        let (status, _) = exchange(
            &owner,
            &mut changed,
            1,
            COMMAND_TYPE_GUILD_INTENT,
            &encode_guild_intent(&GuildIntent::Found {
                name: "Other Guild".into(),
            })
            .map_err(debug)?,
        )
        .await?;
        assert_eq!(status, CommandStatus::Rejected);
        intent(&owner, &mut leader, 2, GuildIntent::Leave { guild }, 2).await?; // leader cannot leave
        intent(&owner, &mut one, 1, GuildIntent::Join { guild }, 5).await?; // not invited
        intent(
            &owner,
            &mut leader,
            3,
            GuildIntent::Invite {
                guild,
                name: "Guest One".into(),
            },
            1,
        )
        .await?;
        let invitations = query(&owner, &mut one, 2, GuildQuery::Invitations).await?;
        assert!(
            matches!(invitations,GuildQueryResult::Invitations(ref list) if list.len()==1 && list[0].guild==guild)
        );
        intent(&owner, &mut one, 3, GuildIntent::Join { guild }, 1).await?;
        assert_eq!(
            query(&owner, &mut one, 4, GuildQuery::Invitations).await?,
            GuildQueryResult::Invitations(vec![])
        );
        let GuildQueryResult::Own(Some(summary)) =
            query(&owner, &mut leader, 4, GuildQuery::Own).await?
        else {
            return Err("missing own guild".into());
        };
        assert_eq!(summary.name, "Wire Guild");
        assert_eq!(summary.own_rank, 1);
        assert_eq!(summary.ranks, vec!["Leader", "Vice Leader", "Member"]);
        intent(
            &owner,
            &mut leader,
            5,
            GuildIntent::SetRank {
                guild,
                character,
                level: 2,
                expected_revision: summary.revision - 1,
            },
            13,
        )
        .await?;
        intent(
            &owner,
            &mut leader,
            6,
            GuildIntent::SetRank {
                guild,
                character,
                level: 2,
                expected_revision: summary.revision,
            },
            1,
        )
        .await?;
        intent(
            &owner,
            &mut one,
            5,
            GuildIntent::Kick {
                guild,
                character: *leader.item_fence.ok_or("leader")?.character_id.as_bytes(),
            },
            2,
        )
        .await?;
        intent(
            &owner,
            &mut leader,
            7,
            GuildIntent::Invite {
                guild,
                name: "Guest Two".into(),
            },
            1,
        )
        .await?;
        intent(
            &owner,
            &mut leader,
            8,
            GuildIntent::RevokeInvite {
                guild,
                character: second,
            },
            1,
        )
        .await?;
        intent(&owner, &mut two, 1, GuildIntent::Join { guild }, 5).await?;
        intent(
            &owner,
            &mut leader,
            9,
            GuildIntent::Invite {
                guild,
                name: "Guest Two".into(),
            },
            1,
        )
        .await?;
        intent(&owner, &mut two, 2, GuildIntent::Join { guild }, 1).await?;
        intent(
            &owner,
            &mut leader,
            10,
            GuildIntent::Kick {
                guild,
                character: second,
            },
            1,
        )
        .await?;
        assert_eq!(
            query(&owner, &mut two, 3, GuildQuery::Own).await?,
            GuildQueryResult::Own(None)
        );
        let GuildQueryResult::Roster { members, next, .. } =
            query(&owner, &mut one, 6, GuildQuery::Roster { guild, after: None }).await?
        else {
            return Err("missing roster".into());
        };
        assert_eq!(members.len(), 2);
        assert_eq!(members[1].character, character);
        assert_eq!(members[1].level, 2);
        assert!(next.is_none());
        intent(&owner, &mut one, 7, GuildIntent::Leave { guild }, 1).await?;
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_guild_members WHERE guild_id=encode($1,'hex')::uuid",
        )
        .bind(guild.as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(count, 1);
        let revision:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(revision, "1");
        let mut old = leader;
        old.continuity.selected_capabilities = SelectedCapabilities::NONE;
        assert_eq!(
            exchange(
                &owner,
                &mut old,
                11,
                COMMAND_TYPE_GUILD_QUERY,
                &encode_guild_query(&GuildQuery::Own).map_err(debug)?
            )
            .await?
            .0,
            CommandStatus::Rejected
        );
        assert_eq!(
            exchange(
                &owner,
                &mut old,
                12,
                COMMAND_TYPE_GUILD_INTENT,
                &encode_guild_intent(&GuildIntent::Found {
                    name: "No Capability".into()
                })
                .map_err(debug)?
            )
            .await?
            .0,
            CommandStatus::Rejected
        );
        assert_eq!(
            exchange(&owner, &mut leader, 11, COMMAND_TYPE_GUILD_INTENT, &[0xff])
                .await?
                .0,
            CommandStatus::Rejected
        );
        // No in-memory Guild cache: new composed owner and reopened authority read committed data.
        drop(owner);
        drop(authority);
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut restarted = deps.owner(&h, &authority);
        restarted.premium_coordinator = Some(&coordinator);
        deps.initialize(&restarted).await;
        let GuildQueryResult::Own(Some(summary)) =
            query(&restarted, &mut leader, 12, GuildQuery::Own).await?
        else {
            return Err("guild lost on reload".into());
        };
        assert_eq!(summary.name, "Wire Guild");
        assert_eq!(summary.revision, 10);
        let mut replay = before_ack;
        assert_eq!(
            intent(
                &restarted,
                &mut replay,
                1,
                GuildIntent::Found {
                    name: "Wire Guild".into()
                },
                1
            )
            .await?,
            r
        );
        let rows: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_guild_operations WHERE result=0")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(rows, 10);
        let events: i64 = sqlx::query_scalar("SELECT count(*) FROM game_guild_event_outbox")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(events, 10);
        drop(restarted);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
