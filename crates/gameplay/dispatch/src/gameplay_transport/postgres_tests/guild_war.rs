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
    assert!(writer.contains("guild_war_end("));
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
use crate::gameplay_transport::{ComposedFreshAdmission, guild_intent::GuildReply, item_fence_of};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::chat::{
    CAPABILITY_CHAT_V1, ChatLine, guild_room::CAPABILITY_CHAT_GUILD_ROOM_V1,
};
use oteryn_protocol_oteryn::guild_war::*;

#[test]
fn guild_war_postgres_real_dispatch_declare_accept_replay_and_frag_limit() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    threaded_scenario(admin, true, false)
}

#[test]
fn guild_war_postgres_real_dispatch_outside_kill_and_end_time() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    threaded_scenario(admin, false, false)
}

#[test]
fn guild_war_postgres_stake_escrow_settlement_and_guild_room() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    threaded_scenario(admin, false, true)
}

fn threaded_scenario(admin: String, warring_killer: bool, staked: bool) -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(admin, warring_killer, staked).map_err(|error| error.to_string()))?
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
    let (status, bytes) = super::guild_wire_fixtures::exchange(
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

fn scenario(admin: String, warring_killer: bool, staked: bool) -> TestResult {
    run(|_| async move {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "guild_war_dispatch", true, false).await?;
        prepare_current_session(&h).await?;
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
        historical_guild(&h, declaring, "Declaring Guild", id(41), id(40)).await?;
        historical_guild(
            &h,
            opposing,
            "Opposing Guild",
            *opponent_character.as_bytes(),
            seed::sid(90, 2),
        )
        .await?;
        let mut guest_base = fence(1)?.item_fence();
        guest_base.character_lease_generation = 1;
        let guest = seed::historical_session(&h.pool, 90, guest_base).await?;
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
        if staked {
            settlement_scenarios(&h, &owner, &mut leader, &mut opponent, opposing).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        // A legacy connection cannot submit this command even though it is registered.
        let selected = leader.continuity.selected_capabilities;
        leader.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[])
                .ok_or("empty selection")?;
        let denied_id = leader.continuity.next_command_id;
        let (status, _) = super::guild_wire_fixtures::exchange(
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
        let (status, bytes) = super::guild_wire_fixtures::exchange(
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
        // This uses the legacy wire and the existing guild-room delivery route.
        let guild_room = SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[CAPABILITY_CHAT_V1, CAPABILITY_CHAT_GUILD_ROOM_V1],
        )
        .ok_or("guild room not offered")?;
        opponent.continuity.selected_capabilities = guild_room;
        owner.chat_guild_admit(opponent).await;
        assert!(
            matches!(
                owner.chat_next_bound(opponent, &mut 1).await,
                Some(ChatLine::Guild(line)) if line.speaker_name == "[Guild War]"
            ),
            "completed legacy war must produce a guild-channel notice"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

/// Retained guild/roster prefix. All runtime writes exercise the installed, unmodified schema.
async fn historical_guild(
    h: &Harness,
    guild: GuildId,
    name: &str,
    leader: [u8; 16],
    account: [u8; 16],
) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_guilds(guild_id,world_id,name,state,formation_deadline,revision,created_at) VALUES($1::text::uuid,$2::text::uuid,$3,2,9999999999999,1,1)")
        .bind(seed::uuid(*guild.as_bytes())).bind(seed::uuid(id(42))).bind(name).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_guild_ranks(guild_id,level,name) SELECT $1::text::uuid,n::smallint,rank FROM unnest(ARRAY['Leader','Vice Leader','Member','Visitor']) WITH ORDINALITY AS r(rank,n)")
        .bind(seed::uuid(*guild.as_bytes())).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_guild_members(character_id,guild_id,rank_level,title,joined_at) VALUES($1::text::uuid,$2::text::uuid,1,'',1)")
        .bind(seed::uuid(leader)).bind(seed::uuid(*guild.as_bytes())).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_account_guild_leadership VALUES($1::text::uuid,$2::text::uuid,$3::text::uuid)")
        .bind(seed::uuid(account)).bind(seed::uuid(*guild.as_bytes())).bind(seed::uuid(leader)).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn balances(h: &Harness) -> TestResult<[i64; 2]> {
    let mut amounts = [0; 2];
    for (target, account) in amounts.iter_mut().zip([id(40), seed::sid(90, 2)]) {
        *target = sqlx::query_scalar("SELECT balance FROM game_account_bank_balances WHERE account_id=$1::text::uuid AND world_id=$2::text::uuid")
            .bind(seed::uuid(account)).bind(seed::uuid(id(42))).fetch_one(&h.pool).await?;
    }
    Ok(amounts)
}

async fn cannot_credit_unsettled_escrow(h: &Harness, war: [u8; 16]) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query(
        "WITH forged AS (
           INSERT INTO game_account_bank_entries(entry_id,war_escrow_id,war_escrow_guild,war_escrow_phase,
            account_id,world_id,amount,balance_before,balance_after,previous_entry_id,acting_character_id)
           SELECT game_character_uuid_v7(),e.war_id,e.guild_id,'SETTLE',
            b.account_id,b.world_id,1,b.balance,b.balance+1,b.last_entry_id,e.leader
           FROM game_guild_war_escrow e JOIN game_account_bank_balances b USING(account_id,world_id)
           WHERE e.war_id=$1::text::uuid AND e.account_id=$2::text::uuid
           RETURNING account_id,world_id,balance_after,entry_id
         ) UPDATE game_account_bank_balances b SET balance=f.balance_after,last_entry_id=f.entry_id
           FROM forged f WHERE b.account_id=f.account_id AND b.world_id=f.world_id"
    ).bind(seed::uuid(war)).bind(seed::uuid(id(40))).execute(&mut *tx).await?;
    let error = tx
        .commit()
        .await
        .err()
        .ok_or("unsettled escrow created bank gold")?;
    assert!(
        error
            .to_string()
            .contains("guild war bank entry lacks its exact escrow cause")
    );
    Ok(())
}

async fn refuse_successor_debit(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    opponent: AdmittedSession,
    acceptance: GuildWarIntent,
) -> TestResult {
    let successor = CharacterId::from_bytes(seed::sid(91, 1)).map_err(debug)?;
    let account = seed::sid(91, 2);
    seed::historical_character(&h.pool, 91, successor, account, id(42), "Successor Leader").await?;
    super::guild_bank_fixtures::bank_seed::balance(h, account, *successor.as_bytes(), 100_000, 140)
        .await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_guild_members SET rank_level=3 WHERE character_id=$1::text::uuid")
        .bind(seed::uuid(id(41)))
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO game_guild_members(character_id,guild_id,rank_level,joined_at)
                 VALUES($1::text::uuid,$2::text::uuid,1,1)",
    )
    .bind(seed::uuid(*successor.as_bytes()))
    .bind(seed::uuid(seed::sid(80, 3)))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    assert!(
        matches!(
            owner
                .guild_command(
                    opponent,
                    opponent.continuity.next_command_id,
                    COMMAND_TYPE_GUILD_WAR_INTENT,
                    &encode_guild_war_intent(&acceptance).map_err(debug)?
                )
                .await,
            GuildReply::Rejected
        ),
        "a predecessor's declaration cannot debit a successor"
    );
    let liquid: i64 = sqlx::query_scalar(
        "SELECT balance FROM game_account_bank_balances WHERE account_id=$1::text::uuid",
    )
    .bind(seed::uuid(account))
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(liquid, 100_000);

    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM game_guild_members WHERE character_id=$1::text::uuid")
        .bind(seed::uuid(*successor.as_bytes()))
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_guild_members SET rank_level=1 WHERE character_id=$1::text::uuid")
        .bind(seed::uuid(id(41)))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

async fn settlement_scenarios(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    leader: &mut AdmittedSession,
    opponent: &mut AdmittedSession,
    opposing: GuildId,
) -> TestResult {
    const STAKE: u32 = 5000;
    let installed: bool =
        sqlx::query_scalar("SELECT success FROM _sqlx_migrations WHERE version=3332")
            .fetch_one(&h.pool)
            .await?;
    assert!(installed);
    super::guild_bank_fixtures::bank_seed::balance(h, id(40), id(41), 100_000, 120).await?;
    super::guild_bank_fixtures::bank_seed::balance(
        h,
        seed::sid(90, 2),
        seed::sid(90, 1),
        100_000,
        130,
    )
    .await?;
    let declaration = GuildWarIntent::DeclareWithStake {
        opponent: *opposing.as_bytes(),
        frag_limit: 10,
        duration_days: 7,
        stake: STAKE,
    };
    let GuildReply::Rejected = owner
        .guild_command(
            *leader,
            leader.continuity.next_command_id,
            COMMAND_TYPE_GUILD_WAR_INTENT,
            &encode_guild_war_intent(&declaration).map_err(debug)?,
        )
        .await
    else {
        return Err("stake writable without capability 628".into());
    };
    let selected = SelectedCapabilities::select(
        owner.offered_capabilities(),
        &[
            CAPABILITY_GUILD_WAR_V1,
            CAPABILITY_GUILD_WAR_STAKE_V1,
            CAPABILITY_CHAT_V1,
            CAPABILITY_CHAT_GUILD_ROOM_V1,
            oteryn_protocol_oteryn::guild_bank::CAPABILITY_GUILD_BANK_V1,
        ],
    )
    .ok_or("guild war stake/guild room selection unavailable")?;
    leader.continuity.selected_capabilities = selected;
    opponent.continuity.selected_capabilities = selected;
    let mut expected = [100_000, 100_000];
    for (frags, timed_out, cancelled, credit, outcome) in [
        ([10, 2], false, false, [10_000, 0], ["was won", "was lost"]),
        ([2, 6], true, false, [3_000, 7_000], ["was lost", "was won"]),
        ([3, 3], true, false, [5_000, 5_000], ["draw", "draw"]),
        (
            [3, 1],
            false,
            true,
            [5_000, 5_000],
            ["cancelled", "cancelled"],
        ),
    ] {
        let declared = command(owner, leader, declaration).await?;
        assert_eq!(balances(h).await?, expected, "declaration must not debit");
        let mut bypass = h.pool.begin().await?;
        sqlx::query(
            "UPDATE game_guild_wars SET state=2,started_at=7,ends_at=8 WHERE war_id=$1::text::uuid",
        )
        .bind(seed::uuid(declared.war))
        .execute(&mut *bypass)
        .await?;
        assert!(
            bypass.commit().await.is_err(),
            "direct activation has no paired ledger debits"
        );
        let legacy_accept = GuildWarIntent::Accept { war: declared.war };
        let GuildReply::Rejected = owner
            .guild_command(
                *opponent,
                opponent.continuity.next_command_id,
                COMMAND_TYPE_GUILD_WAR_INTENT,
                &encode_guild_war_intent(&legacy_accept).map_err(debug)?,
            )
            .await
        else {
            return Err("accepted staked terms without consent".into());
        };
        let accept = GuildWarIntent::AcceptWithStake {
            war: declared.war,
            stake: STAKE,
        };
        if frags[0] == 10 {
            refuse_successor_debit(h, owner, *opponent, accept).await?;
        }
        let command_id = opponent.continuity.next_command_id;
        if frags[0] == 10 {
            let bytes = encode_guild_war_intent(&accept).map_err(debug)?;
            let (first, second) = tokio::join!(
                owner.guild_command(*opponent, command_id, COMMAND_TYPE_GUILD_WAR_INTENT, &bytes),
                owner.guild_command(*opponent, command_id, COMMAND_TYPE_GUILD_WAR_INTENT, &bytes),
            );
            assert!(
                matches!((first, second), (GuildReply::Payload(a), GuildReply::Payload(b)) if a == b),
                "concurrent acceptance observes one durable receipt"
            );
        }
        assert_eq!(command(owner, opponent, accept).await?, declared);
        expected = expected.map(|amount| amount - 6000);
        assert_eq!(
            balances(h).await?,
            expected,
            "accepted war debits both stakes and fees"
        );
        let held: Vec<i64> = sqlx::query_scalar("SELECT stake_gold FROM game_guild_war_escrow WHERE war_id=$1::text::uuid AND settled_xact IS NULL")
            .bind(seed::uuid(declared.war)).fetch_all(&h.pool).await?;
        assert_eq!(held, vec![5000, 5000]);
        let reserved: i64 = sqlx::query_scalar("SELECT game_account_bank_credit_headroom_before_war($1::text::uuid,$2::text::uuid)-game_account_bank_credit_headroom($1::text::uuid,$2::text::uuid)")
            .bind(seed::uuid(id(40))).bind(seed::uuid(id(42))).fetch_one(&h.pool).await?;
        assert_eq!(reserved, 10_000);
        if frags[0] == 10 {
            cannot_credit_unsettled_escrow(h, declared.war).await?;
        }
        assert_eq!(score(h, declared.war).await?.0, 2);
        assert!(matches!(
            owner
                .guild_command(
                    *opponent,
                    command_id,
                    COMMAND_TYPE_GUILD_WAR_INTENT,
                    &encode_guild_war_intent(&accept).map_err(debug)?
                )
                .await,
            GuildReply::Payload(_)
        ));
        assert_eq!(
            balances(h).await?,
            expected,
            "native replay cannot debit again"
        );

        sqlx::query(
            "UPDATE game_guild_wars SET declaring_frags=$2,opposing_frags=$3,
          ends_at=CASE WHEN $4 THEN 1 ELSE ends_at END WHERE war_id=$1::text::uuid",
        )
        .bind(seed::uuid(declared.war))
        .bind(frags[0])
        .bind(frags[1])
        .bind(timed_out)
        .execute(&h.pool)
        .await?;
        if cancelled {
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE game_guilds SET state=4 WHERE guild_id=$1::text::uuid")
                .bind(seed::uuid(*opposing.as_bytes()))
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
        }
        if frags[0] == 10 {
            sqlx::raw_sql(
                "CREATE FUNCTION deny_war_notice() RETURNS trigger LANGUAGE plpgsql AS $$
                 BEGIN RAISE EXCEPTION 'guild war notification fault'; END $$;
                 CREATE TRIGGER deny_war_notice BEFORE INSERT ON game_guild_war_notices
                 FOR EACH ROW EXECUTE FUNCTION deny_war_notice()",
            )
            .execute(&h.pool)
            .await?;
            assert!(
                oteryn_gameplay_social::guild::run_guild_world_pass(
                    &h.root,
                    owner.character,
                    &h.node,
                    leader.world_id,
                    &mut None,
                )
                .await
                .is_err()
            );
            assert_eq!(
                score(h, declared.war).await?.0,
                2,
                "notice failure rolls back war end"
            );
            assert_eq!(
                balances(h).await?,
                expected,
                "notice failure rolls back the payout"
            );
            sqlx::raw_sql("DROP TRIGGER deny_war_notice ON game_guild_war_notices; DROP FUNCTION deny_war_notice()")
                .execute(&h.pool).await?;
        }
        for _ in 0..2 {
            oteryn_gameplay_social::guild::run_guild_world_pass(
                &h.root,
                owner.character,
                &h.node,
                leader.world_id,
                &mut None,
            )
            .await?;
            assert_eq!(
                score(h, declared.war).await?.3,
                Some(if cancelled {
                    3
                } else if timed_out {
                    2
                } else {
                    1
                })
            );
            assert_eq!(
                balances(h).await?,
                [expected[0] + credit[0], expected[1] + credit[1]],
                "settlement pays only once"
            );
        }
        expected = [expected[0] + credit[0], expected[1] + credit[1]];
        let declaring_bank =
            super::guild_bank_fixtures::query(owner, leader, seed::sid(80, 3)).await?;
        let opposing_bank =
            super::guild_bank_fixtures::query(owner, opponent, *opposing.as_bytes()).await?;
        assert_eq!(
            [declaring_bank.bank_balance, opposing_bank.bank_balance],
            [u64::try_from(expected[0])?, u64::try_from(expected[1])?]
        );
        let releases: i64 = sqlx::query_scalar("SELECT sum(settled_gold)::bigint FROM game_guild_war_escrow WHERE war_id=$1::text::uuid")
            .bind(seed::uuid(declared.war)).fetch_one(&h.pool).await?;
        assert_eq!(releases, 2 * i64::from(STAKE));
        for (session, message) in [(*leader, outcome[0]), (*opponent, outcome[1])] {
            owner.chat_guild_admit(session).await;
            let Some(ChatLine::Guild(line)) = owner.chat_next_bound(session, &mut 1).await else {
                return Err("settled war has no guild channel notice".into());
            };
            assert_eq!(
                line.guild_id,
                if session.game_session_id == leader.game_session_id {
                    seed::sid(80, 3)
                } else {
                    *opposing.as_bytes()
                }
            );
            assert!(
                line.text.contains(message),
                "wrong guild result: {}",
                line.text
            );
            owner.chat_guild_admit(session).await;
            assert_eq!(
                owner.chat_next_bound(session, &mut 1).await,
                None,
                "notice is consumed once"
            );
        }
    }
    Ok(())
}
