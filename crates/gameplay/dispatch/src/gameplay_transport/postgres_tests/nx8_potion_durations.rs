//! NX-8 W09: real framed query -> current duration owners -> exact remaining time.
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::CharacterLease;
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use oteryn_protocol_oteryn::potion_durations::{
    CAPABILITY_POTION_DURATIONS_V1, COMMAND_TYPE_POTION_DURATIONS_QUERY, PotionDurations,
};

#[test]
fn nx8_potion_durations_postgres_online_offline_xp_sources_and_expiry() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let started = std::time::Instant::now();
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "nx8_potion_durations", true, false).await?;
        prepare_current_session(&h).await?;
        {
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            let owner = deps.owner(&h, &authority);
            {
                let mut runtime = owner.runtime.lock().await;
                runtime
                    .bind_attacker_lease(
                        deps.actor,
                        deps.admitted.game_session_id,
                        CharacterLease::new(
                            crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                            2,
                        )
                        .map_err(debug)?,
                    )
                    .map_err(debug)?;
                runtime
                    .initialize_first_entry_position(deps.actor)
                    .map_err(debug)?;
            }
            deps.initialize(&owner).await;
            let mut fence = crate::bestiary_postgres_harness::item_fence()?;
            fence.character_lease_generation = 2;
            let mut admitted = deps.admitted;
            admitted.item_fence = Some(fence);
            admitted.first_entry =
                crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
                &[CAPABILITY_POTION_DURATIONS_V1],
            )
            .ok_or("selection")?;
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::raw_sql("CREATE TABLE nx8_potion_clock(at timestamptz NOT NULL); GRANT SELECT ON nx8_potion_clock TO oteryn_game_runtime,oteryn_game_control; INSERT INTO nx8_potion_clock VALUES(date_trunc('milliseconds',clock_timestamp())); CREATE OR REPLACE FUNCTION game_daily_effect_now() RETURNS timestamptz LANGUAGE sql VOLATILE AS $$ SELECT at FROM public.nx8_potion_clock $$")
            .execute(&mut *tx).await?;
            sqlx::query("UPDATE game_durability_reconnect_sessions SET training_created_at_us=(extract(epoch FROM game_daily_effect_now())*1000000)::bigint-600000000 WHERE game_session_id=encode($1,'hex')::uuid")
            .bind(id(50).as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_character_concoction_effects(transaction_id,character_id,concoction,started_at,expires_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'wealth_duplex',game_daily_effect_now()-interval '2 hours',game_daily_effect_now()-interval '1 hour')")
            .bind(id(90).as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
            // Active, queued and expired Store grants. Only the active current interval projects.
            for (seed, start, end) in [(91, -1_i32, 2_i32), (92, 5, 7), (93, -5, -3)] {
                sqlx::query("INSERT INTO game_character_store_xp_boosts(operation_id,character_id,world_id,starts_at,expires_at,percent) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,game_daily_effect_now()+make_interval(mins=>$4),game_daily_effect_now()+make_interval(mins=>$5),50)")
                .bind(id(seed).as_slice()).bind(id(41).as_slice()).bind(id(42).as_slice()).bind(start).bind(end).execute(&mut *tx).await?;
            }
            // Winter Update 2026: an older claim's longer daily line is replaced by the
            // newest claim's line; only that one projects, beside the Store line.
            for (claim, epoch, lane_day, percent, start, end) in [
                (96_u8, 122_i64, 6_i16, 25_i16, -27_i32, 30_i32),
                (94, 123, 7, 50, -1, 4),
            ] {
                sqlx::query("INSERT INTO game_character_daily_reward_claims(character_id,world_id,reset_epoch,reset_day,transaction_id,request_binding,lane_day,streak_before,streak_after,jokers_spent,premium) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$5,$5-122,encode($3,'hex')::uuid,$4,$6,$6-1,$6,0,false)")
                .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(claim).as_slice()).bind([claim;32].as_slice()).bind(epoch).bind(lane_day).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_character_daily_reward_lines(claim_id,line,kind,quantity,status,delivering_transaction_id,effect_started_at,effect_expires_at,xp_boost_percent) VALUES(encode($1,'hex')::uuid,0,'XP_BOOST',30,'DELIVERED',encode($1,'hex')::uuid,game_daily_effect_now()+make_interval(mins=>$2),game_daily_effect_now()+make_interval(mins=>$3),$4)")
                .bind(id(claim).as_slice()).bind(start).bind(end).bind(percent).execute(&mut *tx).await?;
            }
            tx.commit().await?;
            let (status, payload) = super::cyclopedia::support::exchange(
                &owner,
                &mut admitted,
                1,
                COMMAND_TYPE_POTION_DURATIONS_QUERY,
                &[],
            )
            .await?;
            assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
            let first = PotionDurations::decode(&payload).ok_or("duration reply")?;
            assert_eq!(
                first
                    .active
                    .iter()
                    .map(|entry| (entry.source, entry.percent, entry.remaining_ms))
                    .collect::<Vec<_>>(),
                vec![(1, 0, 3_000_000), (3, 50, 120_000), (4, 50, 240_000)]
            );
            // Wall time advances while this session is online. Its current Drome timer spends
            // exactly one minute; expired XP entries disappear and queued ones remain absent.
            sqlx::query("UPDATE nx8_potion_clock SET at=at+interval '1 minute'")
                .execute(&h.pool)
                .await?;
            let (_, payload) = super::cyclopedia::support::exchange(
                &owner,
                &mut admitted,
                2,
                COMMAND_TYPE_POTION_DURATIONS_QUERY,
                &[],
            )
            .await?;
            let next = PotionDurations::decode(&payload).ok_or("next reply")?;
            assert_eq!(
                next.active
                    .iter()
                    .map(|entry| entry.remaining_ms)
                    .collect::<Vec<_>>(),
                vec![2_940_000, 60_000, 180_000]
            );
            // Two hours of online time exhaust Wealth; every XP interval is expired too.
            sqlx::query("UPDATE nx8_potion_clock SET at=at+interval '2 hours'")
                .execute(&h.pool)
                .await?;
            let (_, payload) = super::cyclopedia::support::exchange(
                &owner,
                &mut admitted,
                3,
                COMMAND_TYPE_POTION_DURATIONS_QUERY,
                &[],
            )
            .await?;
            assert!(
                PotionDurations::decode(&payload)
                    .ok_or("expired reply")?
                    .active
                    .is_empty()
            );
            let mut stale = admitted;
            stale
                .item_fence
                .as_mut()
                .ok_or("fence")?
                .character_lease_generation = 1;
            assert!(owner.potion_durations_page(stale).await.is_none());
            eprintln!(
                "POSTGRES_EXECUTED NX-8 W09 durations elapsed={:.3}s wealth_offline=3000000 wealth_online=2940000 xp_sources=2 expired=0",
                started.elapsed().as_secs_f64()
            );
        }
        h.cleanup().await?;
        Ok(())
    })
}
