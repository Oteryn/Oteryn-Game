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
    threaded_scenario(admin, true, StakeCase::Legacy)
}

#[test]
fn guild_war_postgres_real_dispatch_outside_kill_and_end_time() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    threaded_scenario(admin, false, StakeCase::Legacy)
}

#[test]
fn guild_war_postgres_stake_escrow_settlement_and_guild_room() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    threaded_scenario(admin, false, StakeCase::Outcomes)
}

#[test]
fn guild_war_postgres_closing_account_refunds_and_refuses_new_funding() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    threaded_scenario(admin, false, StakeCase::Closing)
}

#[derive(Clone, Copy)]
enum StakeCase {
    Legacy,
    Outcomes,
    Closing,
}

fn threaded_scenario(admin: String, warring_killer: bool, staked: StakeCase) -> TestResult {
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
    let result = war_reply(owner, session, intent).await?;
    assert_eq!(result.code, 1);
    Ok(result)
}

async fn war_reply(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: &mut AdmittedSession,
    intent: GuildWarIntent,
) -> TestResult<GuildWarResult> {
    let id = session.continuity.next_command_id;
    let (status, bytes) = super::guild_wire_fixtures::exchange(
        owner,
        session,
        id,
        COMMAND_TYPE_GUILD_WAR_INTENT,
        &encode_guild_war_intent(&intent).map_err(debug)?,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    Ok(decode_guild_war_result(&bytes).map_err(debug)?)
}

async fn score(h: &Harness, war: [u8; 16]) -> TestResult<(i16, i32, i32, Option<i16>)> {
    Ok(sqlx::query_as("SELECT state,declaring_frags,opposing_frags,ended_reason FROM game_guild_wars WHERE war_id=encode($1,'hex')::uuid")
        .bind(war.as_slice()).fetch_one(&h.pool).await?)
}

fn scenario(admin: String, warring_killer: bool, staked: StakeCase) -> TestResult {
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
        if !matches!(staked, StakeCase::Legacy) {
            match staked {
                StakeCase::Outcomes => {
                    settlement_scenarios(&h, &owner, &mut leader, &mut opponent, opposing).await?
                }
                _ => closing_scenario(&h, &owner, &mut leader, &mut opponent, opposing).await?,
            }
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
                owner.chat_next_delivery(opponent, &mut 1).await,
                Some((ChatLine::Guild(line), Some(_))) if line.speaker_name == "[Guild War]"
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

    let GuildReply::Payload(bytes) = owner
        .guild_command(
            opponent,
            opponent.continuity.next_command_id,
            COMMAND_TYPE_GUILD_WAR_INTENT,
            &encode_guild_war_intent(&acceptance).map_err(debug)?,
        )
        .await
    else {
        return Err("a predecessor's declaration must return a funding refusal".into());
    };
    assert_eq!(decode_guild_war_result(&bytes).map_err(debug)?.code, 2);
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

fn select_stake_capabilities(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    leader: &mut AdmittedSession,
    opponent: &mut AdmittedSession,
) -> TestResult {
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
    Ok(())
}

async fn closing_scenario(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    leader: &mut AdmittedSession,
    opponent: &mut AdmittedSession,
    opposing: GuildId,
) -> TestResult {
    select_stake_capabilities(owner, leader, opponent)?;
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
        stake: 5000,
    };
    let declared = command(owner, leader, declaration).await?;
    command(
        owner,
        opponent,
        GuildWarIntent::AcceptWithStake {
            war: declared.war,
            stake: 5000,
        },
    )
    .await?;
    assert_eq!(balances(h).await?, [94_000, 94_000]);

    // Seed the committed CloseAccount owner state. All subsequent bank guards run normally.
    sqlx::query("INSERT INTO game_account_closures(account_id,termination_id,close_operation_id,
        lifecycle_generation,close_generation,state,recovery_generation,closed_at)
        VALUES($1::text::uuid,$2::text::uuid,$3::text::uuid,1,1,'CLOSING',1,game_account_closure_now())")
        .bind(seed::uuid(id(40))).bind(seed::uuid(id(180))).bind(seed::uuid(id(181)))
        .execute(&h.pool).await?;
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
        Some(3),
        "the World pass closes the funded war"
    );
    assert_eq!(
        balances(h).await?,
        [99_000, 99_000],
        "both stakes return across the closure fence"
    );

    // A proposal already on disk can still reach the open opponent's acceptance route.
    let pending = seed::sid(191, 3);
    sqlx::query("INSERT INTO game_guild_wars(war_id,world_id,declaring_guild,opposing_guild,state,
         frag_limit,duration_days,declared_at,stake_gold,stake_declarer)
         VALUES($1::text::uuid,$2::text::uuid,$3::text::uuid,$4::text::uuid,1,10,7,1,5000,$5::text::uuid)")
        .bind(seed::uuid(pending)).bind(seed::uuid(id(42))).bind(seed::uuid(seed::sid(80,3)))
        .bind(seed::uuid(*opposing.as_bytes())).bind(seed::uuid(id(41))).execute(&h.pool).await?;
    let refused = war_reply(
        owner,
        opponent,
        GuildWarIntent::AcceptWithStake {
            war: pending,
            stake: 5000,
        },
    )
    .await?;
    assert_eq!(
        refused,
        GuildWarResult {
            code: 2,
            war: pending
        }
    );
    assert_eq!(balances(h).await?, [99_000, 99_000]);
    assert_eq!(score(h, pending).await?.0, 1);
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
    const HIGH_BALANCE: i64 = 999_999_998_999;
    let installed: bool =
        sqlx::query_scalar("SELECT success FROM _sqlx_migrations WHERE version=3353")
            .fetch_one(&h.pool)
            .await?;
    assert!(installed);
    super::guild_bank_fixtures::bank_seed::balance(h, id(40), id(41), HIGH_BALANCE, 120).await?;
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
    let staked_wire = encode_guild_war_intent(&declaration).map_err(debug)?;
    assert!(
        staked_wire.ends_with(&[40, 0x88, 0x27]),
        "capability probe carries field 5"
    );
    let GuildReply::Rejected = owner
        .guild_command(
            *leader,
            leader.continuity.next_command_id,
            COMMAND_TYPE_GUILD_WAR_INTENT,
            &staked_wire,
        )
        .await
    else {
        return Err("stake writable without capability 628".into());
    };
    select_stake_capabilities(owner, leader, opponent)?;
    prepare_bazaar(h).await?;
    let mut expected = [HIGH_BALANCE, 100_000];
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
            let held = u64::try_from(HIGH_BALANCE - 5999)?;
            let decision = bazaar_gold(owner, 210, BazaarGold::Hold(held)).await?;
            let crate::native_admission_source::character_commands::Decision::Publish {
                receipt,
                result_code,
            } = decision
            else {
                return Err(format!("expected live Bazaar hold, got {decision:?}").into());
            };
            assert_eq!(result_code, "CHAR_CMD_OK");
            let payload: serde_json::Value = serde_json::from_slice(&receipt)?;
            let hold = payload["result_data"]["hold_id"]
                .as_str()
                .ok_or("hold ID missing")?;
            assert_eq!(
                war_reply(owner, opponent, accept).await?,
                GuildWarResult {
                    code: 2,
                    war: declared.war
                }
            );
            assert_eq!(
                balances(h).await?,
                expected,
                "Bazaar funds cannot fund a war"
            );
            assert!(
                matches!(bazaar_gold(owner,211,BazaarGold::Release { hold, amount:held }).await?,
                crate::native_admission_source::character_commands::Decision::Publish { result_code, .. }
                if result_code == "CHAR_CMD_OK")
            );
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
            assert!(
                matches!(bazaar_gold(owner,212,BazaarGold::Hold(10_000)).await?,
                crate::native_admission_source::character_commands::Decision::Publish { result_code, .. }
                if result_code == "CHAR_CMD_BAZAAR_HOLD_CONFLICT"),
                "a new Bazaar hold cannot consume the active war's winner headroom"
            );
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
        if frags[0] == 10 {
            let omitted = encode_guild_war_intent(&legacy_accept).map_err(debug)?;
            let changed = [omitted.as_slice(), &[40, 0x89, 0x27]].concat();
            let zero = [omitted.as_slice(), &[40, 0]].concat();
            for wire in [omitted, changed, zero] {
                assert!(
                    matches!(
                        owner
                            .guild_command(
                                *opponent,
                                command_id,
                                COMMAND_TYPE_GUILD_WAR_INTENT,
                                &wire
                            )
                            .await,
                        GuildReply::Rejected
                    ),
                    "a replay with different stake terms cannot recover the accepted receipt"
                );
            }
        }
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
            use oteryn_protocol_oteryn::guild_bank as bank;
            let command_id = opponent.continuity.next_command_id;
            let (status, bytes) = super::guild_wire_fixtures::exchange(
                owner,
                opponent,
                command_id,
                bank::COMMAND_TYPE_GUILD_BANK_INTENT,
                &bank::encode_guild_bank_intent(&bank::GuildBankIntent {
                    guild: *opposing.as_bytes(),
                    action: bank::GuildBankAction::Disband("Opposing Guild".into()),
                })
                .map_err(debug)?,
            )
            .await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert_eq!(
                bank::decode_guild_bank_reply(&bytes).map_err(debug)?.code,
                1
            );
        }
        if frags[0] == 10 {
            let healthy_war = seed_healthy_war(h).await?;
            sqlx::raw_sql(
                "CREATE FUNCTION deny_war_notice() RETURNS trigger LANGUAGE plpgsql AS $$
                 BEGIN
                   IF NEW.message LIKE '%Declaring Guild%' OR NEW.message LIKE '%Opposing Guild%'
                   THEN RAISE EXCEPTION 'guild war notification fault'; END IF;
                   RETURN NEW;
                 END $$;
                 CREATE CONSTRAINT TRIGGER deny_war_notice AFTER INSERT ON game_guild_war_notices
                 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION deny_war_notice()",
            )
            .execute(&h.pool)
            .await?;
            oteryn_gameplay_social::guild::run_guild_world_pass(
                &h.root,
                owner.character,
                &h.node,
                leader.world_id,
                &mut None,
            )
            .await?;
            assert_eq!(
                score(h, healthy_war).await?.3,
                Some(2),
                "another due war commits despite the failed one"
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
        if cancelled {
            let member: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=$1::text::uuid)",
            )
            .bind(seed::uuid(*opposing.as_bytes()))
            .fetch_one(&h.pool)
            .await?;
            assert!(
                !member,
                "the offline recipient's roster was actually cleaned up"
            );
        }
        let releases: i64 = sqlx::query_scalar("SELECT sum(settled_gold)::bigint FROM game_guild_war_escrow WHERE war_id=$1::text::uuid")
            .bind(seed::uuid(declared.war)).fetch_one(&h.pool).await?;
        assert_eq!(releases, 2 * i64::from(STAKE));
        for (session, message) in [(*leader, outcome[0]), (*opponent, outcome[1])] {
            owner.chat_guild_admit(session).await;
            let Some((ChatLine::Guild(line), Some(notice))) =
                owner.chat_next_delivery(session, &mut 1).await
            else {
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
                owner.chat_next_delivery(session, &mut 1).await,
                Some((ChatLine::Guild(line), Some(notice))),
                "an unwritten notice is eligible on reconnect"
            );
            if frags[0] == 10 && session.game_session_id == leader.game_session_id {
                assert!(
                    !notice_socket(owner, session, true).await?,
                    "first notice frame must fail to write"
                );
                owner.chat_guild_admit(session).await;
                assert!(
                    matches!(owner.chat_next_delivery(session, &mut 1).await,
                    Some((ChatLine::Guild(_), Some(replayed))) if replayed == notice),
                    "failed socket write keeps exactly the same notice available"
                );
                assert!(
                    notice_socket(owner, session, false).await?,
                    "replacement connection receives the notice"
                );
            } else {
                owner.guild_war_notice_written(session, notice).await;
            }
            owner.chat_guild_admit(session).await;
            assert_eq!(
                owner.chat_next_delivery(session, &mut 1).await,
                None,
                "notice is consumed once"
            );
        }
        let declaring_bank =
            super::guild_bank_fixtures::query(owner, leader, seed::sid(80, 3)).await?;
        let opposing_bank =
            super::guild_bank_fixtures::query(owner, opponent, *opposing.as_bytes()).await?;
        assert_eq!(
            [declaring_bank.bank_balance, opposing_bank.bank_balance],
            [u64::try_from(expected[0])?, u64::try_from(expected[1])?]
        );
    }
    Ok(())
}

async fn seed_healthy_war(h: &Harness) -> TestResult<[u8; 16]> {
    // A disconnected legacy pair keeps each notice in this test scoped to its own guild.
    let war = seed::sid(199, 4);
    let left = seed::sid(199, 1);
    let right = seed::sid(199, 2);
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_guilds(guild_id,world_id,name,state,formation_deadline,revision,created_at)
        VALUES($1::text::uuid,$3::text::uuid,'Healthy East',2,9999999999999,1,1),
              ($2::text::uuid,$3::text::uuid,'Healthy West',2,9999999999999,1,1)")
        .bind(seed::uuid(left)).bind(seed::uuid(right)).bind(seed::uuid(id(42))).execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO game_guild_wars(war_id,world_id,declaring_guild,opposing_guild,state,
        frag_limit,duration_days,declared_at,started_at,ends_at)
        VALUES($1::text::uuid,$2::text::uuid,$3::text::uuid,$4::text::uuid,2,10,7,1,1,2)",
    )
    .bind(seed::uuid(war))
    .bind(seed::uuid(id(42)))
    .bind(seed::uuid(left))
    .bind(seed::uuid(right))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(war)
}

async fn prepare_bazaar(h: &Harness) -> TestResult {
    use crate::durability::native_admission_source::{
        DescriptorRegistration, FreshStoreProvenance,
    };
    let provenance = FreshStoreProvenance {
        namespace: "gwar-bazaar-dispatch".into(),
        authorization: "test-owner".into(),
        source_authority: "platform".into(),
        initialized_at: 10,
    };
    let descriptor = DescriptorRegistration {
        revision: 1,
        facts: vec![1],
        installed_at: 10,
    };
    assert!(
        h.root
            .record_native_source_descriptor_issuance(
                "platform",
                descriptor.clone(),
                Some(provenance.clone())
            )
            .await
            .map_err(debug)?
    );
    h.root
        .initialize_native_admission_source(&h.node, provenance, descriptor)
        .await
        .map_err(debug)?;
    sqlx::query(
        r#"UPDATE game_bazaar_configuration SET enabled=true,policy='{
      "common_restore_scope":true,"proof_retention_seconds":7776000,"v2_receipt_bytes":270336,
      "gold_fee_bps":250,"reserve_incoming_headroom":true}'::jsonb WHERE singleton"#,
    )
    .execute(&h.pool)
    .await?;
    Ok(())
}

async fn allow_bazaar_buyer(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    revision: u8,
) -> TestResult {
    use crate::durability::native_admission_source::{
        NativeSourceOperation, NativeSourceSubject, SourceObservation,
    };
    let now = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs(),
    )?;
    let account = seed::uuid(id(40));
    let facts = serde_json::json!({
      "version":1,"operation":"ReadAccountSecurityV1","result":"observed","source_authority":"platform",
      "source_revision":revision.to_string(),"decision_identity":revision.to_string(),"source_observed_at":now.to_string(),"clock_uncertainty_seconds":"0",
      "account_id":account,"purpose":"platform_security","scope":"fresh_admission","allowed":true,"minimum_valid_generation":"1"
    });
    owner
        .root
        .accept_native_source_observation(
            owner.holder,
            SourceObservation {
                source_authority: "platform".into(),
                operation: NativeSourceOperation::ReadAccountSecurityV1,
                subject: NativeSourceSubject::account_security(&account).map_err(debug)?,
                source_revision: u64::from(revision),
                decision_identity: revision.to_string(),
                observed_at: now,
                semantic_facts: serde_json::to_vec(&facts)?,
            },
        )
        .await
        .map_err(debug)?;
    Ok(())
}

enum BazaarGold<'a> {
    Hold(u64),
    Release { hold: &'a str, amount: u64 },
}

async fn bazaar_gold(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    key: u8,
    intent: BazaarGold<'_>,
) -> TestResult<crate::native_admission_source::character_commands::Decision> {
    if matches!(intent, BazaarGold::Hold(_)) {
        allow_bazaar_buyer(owner, key).await?;
    }
    let (command, details) = match intent {
        BazaarGold::Hold(amount) => (
            "HoldBazaarGold",
            serde_json::json!({
                "buyer_character_id":seed::uuid(id(41)), "hold_purpose":"OFFER","offer_id":seed::uuid(seed::sid(u32::from(key),7)),
                "amount":amount.to_string(), "expected_hold_revision":"0"
            }),
        ),
        BazaarGold::Release { hold, amount } => (
            "ReleaseBazaarGold",
            serde_json::json!({
                "hold_id":hold,"expected_hold_revision":"1","release_reason":"EXCESS","amount":amount.to_string()
            }),
        ),
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let operation = seed::uuid(id(key));
    let mut wire = serde_json::json!({
        "contract_version":2,"command":command,"variant":"PLATFORM_BAZAAR",
        "issuer_authority":"OTERYN_PLATFORM_BAZAAR_AUTHORITY","issuer_decision_id":seed::uuid(id(209)),
        "source_revision":key.to_string(),"operation_id":operation,"listing_id":seed::uuid(id(215)),
        "seller_account_id":seed::uuid(seed::sid(90,2)),"buyer_account_id":seed::uuid(id(40)),
        "world_id":seed::uuid(id(42)),
        "issued_at_source":(now-1).to_string(),"expires_at_source":(now+120).to_string(),
        "audience":"OTERYN_GAME_BAZAAR_AUTHORITY"
    });
    let details = details.as_object().ok_or("Bazaar operation fields")?;
    wire.as_object_mut()
        .ok_or("Bazaar command fields")?
        .extend(details.clone());
    Ok(owner
        .root
        .decide_bazaar_command(
            owner.character,
            owner.holder,
            "oteryn:character-authority:primary",
            command,
            &operation,
            &serde_json::to_vec(&wire)?,
        )
        .await)
}

const WAR_SPEAKER: &[u8] = b"[Guild War]";

struct NoticeSocket {
    inner: tokio::io::DuplexStream,
    fail_notice: bool,
    failed: bool,
}

impl tokio::io::AsyncRead for NoticeSocket {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        output: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.get_mut().inner).poll_read(cx, output)
    }
}

impl tokio::io::AsyncWrite for NoticeSocket {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bytes: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        if this.fail_notice
            && bytes
                .windows(WAR_SPEAKER.len())
                .any(|part| part == WAR_SPEAKER)
        {
            this.failed = true;
            return std::task::Poll::Ready(Err(std::io::ErrorKind::BrokenPipe.into()));
        }
        std::pin::Pin::new(&mut this.inner).poll_write(cx, bytes)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}

async fn notice_socket(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: AdmittedSession,
    fail_notice: bool,
) -> TestResult<bool> {
    use crate::gameplay_transport::{
        connection::{IDLE_LIVENESS, serve_admitted},
        tcp_tls::read_frame,
    };
    let (server, mut client) = tokio::io::duplex(1024);
    let serving = async move {
        let mut server = NoticeSocket {
            inner: server,
            fail_notice,
            failed: false,
        };
        serve_admitted(&mut server, session, owner, IDLE_LIVENESS).await;
        server.failed
    };
    let driving = async move {
        while let Ok(frame) = read_frame(&mut client).await {
            if frame
                .windows(WAR_SPEAKER.len())
                .any(|part| part == WAR_SPEAKER)
            {
                return true;
            }
        }
        false
    };
    let (failed, written) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::fixture::join_two(serving, driving),
    )
    .await?;
    assert_eq!(
        failed, fail_notice,
        "the fault must reach the war notice socket write"
    );
    Ok(written)
}
