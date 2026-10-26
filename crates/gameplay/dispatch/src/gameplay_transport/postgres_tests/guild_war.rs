//! GUILD-WAR-1 migration qualification mounted with the dispatch PostgreSQL suite.
#[test]
fn guild_war_schema_is_embedded_for_dispatch() {
    let migration =
        include_str!("../../../../../../apps/game-server/migrations/0232_guild_wars.sql");
    assert!(migration.contains("CREATE TABLE game_guild_wars"));
    assert!(migration.contains("game_guild_war_commands"));
    assert!(migration.contains("CREATE TABLE game_guild_war_frags"));
    assert!(migration.contains("death_occurrence_id UUID PRIMARY KEY"));
}

#[test]
fn guild_war_frag_and_end_writes_are_mounted_with_dispatch() {
    let writer = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../durability/src/durability/guild_war.rs"
    ));
    assert!(writer.contains("attribute_guild_war_frag_in_transaction"));
    assert!(writer.contains("ON CONFLICT(death_occurrence_id) DO NOTHING"));
    assert!(writer.contains("guild_war_end(limit"));
}

use super::fixture::{Harness, debug, fence, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::domain::{CharacterId, guild::GuildId};
use crate::durability::guild_chat_fixture as seed;
use crate::durability::pvp_death_consequences::{PvpContributorKind, PvpDeathContributor};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority,
};
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::guild_war::*;

#[test]
fn guild_war_postgres_real_dispatch_declare_accept_replay_and_frag_limit() -> TestResult {
    threaded_scenario(true)
}

#[test]
fn guild_war_postgres_real_dispatch_outside_kill_and_end_time() -> TestResult {
    threaded_scenario(false)
}

fn threaded_scenario(warring_killer: bool) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(warring_killer).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "guild war dispatch panicked")??;
    Ok(())
}

async fn command(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: &mut AdmittedSession,
    intent: GuildWarIntent,
) -> TestResult<GuildWarResult> {
    let command_id = session.continuity.next_command_id;
    let (status, bytes) = super::guild_wire::exchange(
        owner,
        session,
        command_id,
        COMMAND_TYPE_GUILD_WAR_INTENT,
        &encode_guild_war_intent(&intent).map_err(debug)?,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    let result = decode_guild_war_result(&bytes).map_err(debug)?;
    assert_eq!(result.code, 1);
    Ok(result)
}

async fn score(h: &Harness, war: [u8; 16]) -> TestResult<(i16, i32, i32, Option<i16>)> {
    Ok(sqlx::query_as("SELECT state,declaring_frags,opposing_frags,ended_reason FROM game_guild_wars WHERE war_id=encode($1,'hex')::uuid")
        .bind(war.as_slice()).fetch_one(&h.pool).await?)
}

fn scenario(warring_killer: bool) -> TestResult {
    run(|admin| async move {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "guild_war_dispatch", true, false).await?;
        prepare_current_session(&h).await?;
        // The shared historical Guild fixture installs guards after seeding the two rosters.
        // Reinstall 0232 after the historical Guild fixture rebuilds its parent tables:
        // its broad function cleanup would otherwise remove the war receipt triggers too.
        sqlx::raw_sql("DROP TABLE game_guild_war_frags,game_guild_war_commands,game_guild_wars; DROP FUNCTION game_guild_war_rows_immutable()")
            .execute(&h.pool).await?;
        seed::prepare(&h.pool).await?;
        let opponent_character = CharacterId::from_bytes(seed::sid(90, 1)).map_err(debug)?;
        seed::historical_character(
            &h.pool,
            90,
            opponent_character,
            seed::sid(90, 2),
            id(42),
            "War Opponent",
        )
        .await?;
        let declaring = GuildId::from_bytes(seed::sid(80, 3)).map_err(debug)?;
        let opposing = GuildId::from_bytes(seed::sid(90, 3)).map_err(debug)?;
        seed::initial_guild(
            &h.pool,
            declaring,
            id(42),
            "Declaring Guild",
            CharacterId::from_bytes(id(41)).map_err(debug)?,
            id(40),
        )
        .await?;
        seed::initial_guild(
            &h.pool,
            opposing,
            id(42),
            "Opposing Guild",
            opponent_character,
            seed::sid(90, 2),
        )
        .await?;
        let mut guest_base = fence(1)?.item_fence();
        guest_base.character_lease_generation = 1;
        let guest = seed::historical_session(&h.pool, 90, guest_base).await?;
        seed::guard(&h.pool).await?;
        sqlx::raw_sql(include_str!(
            "../../../../../../apps/game-server/migrations/0232_guild_wars.sql"
        ))
        .execute(&h.pool)
        .await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(|error| format!("open authority: {error:?}"))?;
        super::fixture::pvp_source_death_fixture::seed_root(&h.pool, 41, 60, 42).await?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
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
            SelectedCapabilities::select(owner.offered_capabilities(), &[CAPABILITY_GUILD_WAR_V1])
                .ok_or("guild war capability was not offered")?;
        let actor = {
            let mut runtime = owner.runtime.lock().await;
            let reservation = runtime
                .reserve_fresh_session(guest.game_session_id)
                .map_err(debug)?;
            let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
            actor
        };
        deps.initialize_player(&owner, actor, guest.game_session_id)
            .await;
        let mut opponent = AdmittedSession {
            game_session_id: guest.game_session_id,
            runtime_actor: Some(actor),
            item_fence: Some(guest),
            controller: None,
            ..leader
        };
        // A legacy connection cannot submit this command even though it is registered.
        let selected = leader.continuity.selected_capabilities;
        leader.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[])
                .ok_or("empty selection")?;
        let denied_id = leader.continuity.next_command_id;
        let (status, _) = super::guild_wire::exchange(
            &owner,
            &mut leader,
            denied_id,
            COMMAND_TYPE_GUILD_WAR_INTENT,
            &encode_guild_war_intent(&GuildWarIntent::Declare {
                opponent: *opposing.as_bytes(),
                frag_limit: 10,
                duration_days: 7,
            })
            .map_err(debug)?,
        )
        .await?;
        assert_eq!(status, CommandStatus::Rejected);
        leader.continuity.selected_capabilities = selected;
        // Global: same-World declaration, agreed 10..1000 kills and 7..180 days.
        // https://www.tibia.com/gameguides/?section=guilds&subtopic=manual (5.8.4)
        let declared = command(
            &owner,
            &mut leader,
            GuildWarIntent::Declare {
                opponent: *opposing.as_bytes(),
                frag_limit: 10,
                duration_days: 7,
            },
        )
        .await?;
        assert_eq!(score(&h, declared.war).await?, (1, 0, 0, None));
        let timing: (Option<i64>, Option<i64>) = sqlx::query_as(
            "SELECT started_at,ends_at FROM game_guild_wars WHERE war_id=encode($1,'hex')::uuid",
        )
        .bind(declared.war.as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(timing, (None, None));
        let accept = GuildWarIntent::Accept { war: declared.war };
        let accepted_id = opponent.continuity.next_command_id;
        assert_eq!(command(&owner, &mut opponent, accept).await?, declared);
        assert_eq!(score(&h, declared.war).await?, (2, 0, 0, None));
        let duration: i64 = sqlx::query_scalar(
            "SELECT ends_at-started_at FROM game_guild_wars WHERE war_id=encode($1,'hex')::uuid",
        )
        .bind(declared.war.as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(duration, 7 * 86_400_000);
        let (status, bytes) = super::guild_wire::exchange(
            &owner,
            &mut opponent,
            accepted_id,
            COMMAND_TYPE_GUILD_WAR_INTENT,
            &encode_guild_war_intent(&accept).map_err(debug)?,
        )
        .await?;
        assert_eq!(status, CommandStatus::DuplicateReplay);
        assert_eq!(decode_guild_war_result(&bytes).map_err(debug)?, declared);
        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_guild_war_commands")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(receipts, 2);
        // Neither side has won and the deadline is still in the future.
        assert_eq!(
            h.root
                .finish_due_guild_wars(&authority, &h.node, leader.world_id)
                .await
                .map_err(|error| format!("finish guild wars: {error:?}"))?,
            0
        );

        // The native receipt also survives loss of the Channel's retained command reply.
        let crate::gameplay_transport::guild_intent::GuildReply::Payload(native) = owner
            .guild_command(
                opponent,
                accepted_id,
                COMMAND_TYPE_GUILD_WAR_INTENT,
                &encode_guild_war_intent(&accept).map_err(debug)?,
            )
            .await
        else {
            return Err("native war replay failed".into());
        };
        assert_eq!(native, bytes);

        assert!(matches!(
            owner
                .guild_command(
                    opponent,
                    accepted_id,
                    COMMAND_TYPE_GUILD_WAR_INTENT,
                    &encode_guild_war_intent(&GuildWarIntent::Declare {
                        opponent: *declaring.as_bytes(),
                        frag_limit: 20,
                        duration_days: 7,
                    })
                    .map_err(debug)?,
                )
                .await,
            crate::gameplay_transport::guild_intent::GuildReply::Rejected
        ));

        // Exercise the production death seam with ledger evidence, as the sibling PvP suite does.
        // The native death receipt prevents double scoring.
        // Global does not specify last-hit versus most-damage attribution. Follow pinned Canary
        // 47dfd51f45280a59a1d3e50ba7edd573d7234446, data/scripts/creaturescripts/player/death.lua,
        // handleGuildWar: the killing player and victim must belong to opposing warring guilds.
        let (tag, killer, expected) = if warring_killer {
            // Prior kills are fixture data; this real death reaches the agreed frag limit.
            sqlx::query(
                "UPDATE game_guild_wars SET opposing_frags=9 WHERE war_id=encode($1,'hex')::uuid",
            )
            .bind(declared.war.as_slice())
            .execute(&h.pool)
            .await?;
            (211, seed::sid(90, 1), 10)
        } else {
            (210, id(60), 0)
        };
        {
            sqlx::query("INSERT INTO game_character_pvp_state(character_id,world_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid) ON CONFLICT DO NOTHING")
                .bind(id(41).as_slice()).bind(id(42).as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_character_pvp_ledger(victim_character_id,contributor_character_id,buckets,last_at,compact,compact_kind) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'[]',clock_timestamp(),true,1)")
                .bind(id(41).as_slice()).bind(killer.as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_character_pvp_harm(game_session_id,command_id,ordinal,victim,attacker,binding,damage,unjustified) VALUES(encode($1,'hex')::uuid,$2,0,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,100,false)")
                .bind(id(50).as_slice()).bind(i64::from(tag)).bind(id(41).as_slice()).bind(killer.as_slice()).bind([tag;32].as_slice()).execute(&h.pool).await?;
            let mut request = crate::gameplay_transport::death_ml_loss_postgres::death(tag)?;
            request.pvp_contributors = vec![PvpDeathContributor {
                character_id: CharacterId::from_bytes(killer).map_err(debug)?,
                kind: PvpContributorKind::Damage,
                unjustified: false,
            }];
            let mut current = fence(1)?;
            current.character_lease_generation = 2;
            owner
                .settle_source_player_death(current, request.clone(), None, None)
                .await
                .map_err(|error| format!("settle guild war death: {error:?}"))?;
            owner
                .settle_source_player_death(current, request, None, None)
                .await
                .map_err(|error| format!("replay guild war death: {error:?}"))?;
            assert_eq!(score(&h, declared.war).await?, (2, 0, expected, None));
            let frags: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_guild_war_frags WHERE war_id=encode($1,'hex')::uuid",
            )
            .bind(declared.war.as_slice())
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(frags, i64::from(warring_killer));
        }
        if !warring_killer {
            sqlx::query("UPDATE game_guild_wars SET started_at=floor(extract(epoch FROM clock_timestamp())*1000)::bigint-8*86400000::bigint,ends_at=floor(extract(epoch FROM clock_timestamp())*1000)::bigint-86400000 WHERE war_id=encode($1,'hex')::uuid")
                .bind(declared.war.as_slice()).execute(&h.pool).await?;
        }
        assert_eq!(
            h.root
                .finish_due_guild_wars(&authority, &h.node, leader.world_id)
                .await
                .map_err(|error| format!("finish guild wars: {error:?}"))?,
            1
        );
        assert_eq!(
            score(&h, declared.war).await?,
            (3, 0, expected, Some(if warring_killer { 1 } else { 2 }))
        );
        assert_eq!(
            h.root
                .finish_due_guild_wars(&authority, &h.node, leader.world_id)
                .await
                .map_err(|error| format!("finish guild wars: {error:?}"))?,
            0
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
