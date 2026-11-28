use super::fixture::{Harness, debug, fence, id, pvp_source_death_fixture};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::gameplay_transport::death_ml_loss_postgres::death as death_request;
use oteryn_simulation_determinism::ExactI64;

#[test]
fn unfair_fight_reduces_the_committed_death_loss_and_replays_it() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, false)
}

#[test]
fn unfair_fight_excludes_active_guild_war_opponents() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, true)
}

fn scenario(admin: String, guild_war: bool) -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |_| {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "unfair_fight_death", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&h, &authority);

        sqlx::query("INSERT INTO game_control_field_policy_grants VALUES(session_user,encode($1,'hex')::uuid,'policy-1')")
            .bind(id(42).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_spell_field_world_policies(world_id,world_policy_revision,world_type,protection_level,in_fight_ms,source_pin,control_role,decision_identity) VALUES(encode($1,'hex')::uuid,'policy-1',2,8,60000,'99902524e052f37574194466c2949c576e4ab269',session_user,'unfair-fight-fixture')")
            .bind(id(42).as_slice()).execute(&h.pool).await?;
        pvp_source_death_fixture::seed_root(&h.pool, 41, 60, 42).await?;
        sqlx::query("INSERT INTO game_character_pvp_state (character_id, world_id) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).execute(&h.pool).await?;
        pvp_source_death_fixture::seed_ledger(&h.pool, 41, 60, true, Some(1), "0 seconds").await?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_character_progression_state SET level=21,total_experience=2000000 WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_progression_state \
            (character_id,character_revision,level,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) \
            SELECT encode($2,'hex')::uuid,1,120,1000000,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision \
            FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).bind(id(60).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_pvp_harm(game_session_id,command_id,ordinal,victim,attacker,binding,damage,unjustified) \
            VALUES(encode($3,'hex')::uuid,1,0,encode($1,'hex')::uuid,encode($2,'hex')::uuid,$4,100,true)")
            .bind(id(41).as_slice()).bind(id(60).as_slice()).bind(id(77).as_slice())
            .bind([7_u8;32].as_slice()).execute(&mut *tx).await?;
        if guild_war {
            seed_war(&mut tx).await?;
        }
        tx.commit().await?;
        eprintln!("unfair_fight_death PostgreSQL fixture ready, guild_war={guild_war}");

        let mut current = fence(1)?;
        current.character_lease_generation = 2;
        let mut request = death_request(90)?;
        for threshold in &mut request.policy.thresholds {
            threshold.minimum_experience = ExactI64::new(i64::from(threshold.level - 1) * 100_000);
        }
        request.policy.terminal_exclusive_experience = ExactI64::new(10_000_000);
        let ordinary_loss =
            crate::domain::death::death_experience_loss(21, 0, false).map_err(debug)?;
        let expected_loss = i64::try_from(if guild_war {
            ordinary_loss
        } else {
            ordinary_loss / 5
        })?;
        let expected_experience = 2_000_000 - expected_loss;
        for attempt in 0..2 {
            let death = owner
                .settle_source_player_death(current, request.clone(), None, None)
                .await
                .map_err(debug)?;
            assert_eq!(
                death.experience_lost.get(),
                expected_loss,
                "attempt {attempt}: apply the 20% floor only to an eligible opponent"
            );
            assert_eq!(
                (death.level_after, death.experience_after.get()),
                (20, expected_experience)
            );
        }
        let state: (i64, i64) = sqlx::query_as(
            "SELECT level,total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid",
        ).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(state, (20, expected_experience));
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn seed_war(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) -> TestResult {
    for (guild, leader, name) in [(65, 41, "Victim Guild"), (66, 60, "Attacker Guild")] {
        sqlx::query("INSERT INTO game_guilds(guild_id,world_id,name,state,formation_deadline,revision,created_at) \
            VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,2,0,1,0)")
            .bind(id(guild).as_slice()).bind(id(42).as_slice()).bind(name).execute(&mut **tx).await?;
        sqlx::query(
            "INSERT INTO game_guild_members(character_id,guild_id,rank_level,joined_at) \
            VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,0)",
        )
        .bind(id(leader).as_slice())
        .bind(id(guild).as_slice())
        .execute(&mut **tx)
        .await?;
    }
    sqlx::query("INSERT INTO game_guild_wars(war_id,world_id,declaring_guild,opposing_guild,state,frag_limit,duration_days,declared_at,started_at,ends_at) \
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,2,10,7,0,0, \
               floor(extract(epoch FROM clock_timestamp()+interval '1 day')*1000)::bigint)")
        .bind(id(67).as_slice()).bind(id(42).as_slice()).bind(id(65).as_slice()).bind(id(66).as_slice())
        .execute(&mut **tx).await?;
    Ok(())
}
