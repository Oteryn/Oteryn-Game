//! Durable daily streak -> existing vitals cadence on a qualified bundle PZ tile.
use super::*;
use crate::foundation::ChannelContentPin;
use crate::gameplay_transport::actor_spell;
use crate::map::boot::tests::{floor_items, floor_pins, teleport_bundle};

#[test]
fn daily_resting_persisted_streak_feeds_existing_vitals_owner() -> TestResult {
    scenario(false)
}

#[test]
fn daily_resting_premium_streak_feeds_vitals_and_soul_owner() -> TestResult {
    scenario(true)
}

/// Winter Update 2026: the resting bonus does not depend on the reward streak.
#[test]
fn daily_resting_zero_streak_premium_regenerates_soul() -> TestResult {
    scenario_with(true, true)
}

fn scenario(premium: bool) -> TestResult {
    scenario_with(premium, false)
}

fn scenario_with(premium: bool, zero_streak: bool) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || resting_scenario(premium, zero_streak).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "daily resting PostgreSQL thread panicked")??;
    Ok(())
}

fn resting_scenario(premium: bool, zero_streak: bool) -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "daily_resting", false, false).await?;
        prepare_current_session(&h).await?;
        super::daily_reward::epoch(&h, 1).await?;
        // Seed a valid final projection/receipt pair, without disabling its deferred guards.
        let mut tx = h.pool.begin().await?;
        sqlx::query("INSERT INTO game_character_daily_rewards(character_id,world_id,lane_day,streak,last_claim_epoch,last_claim_day) SELECT encode($1,'hex')::uuid,world_id,2,7,reset_epoch,reset_day FROM game_world_reset_history WHERE world_id=encode($2,'hex')::uuid AND reset_epoch=1")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_daily_reward_claims(character_id,world_id,reset_epoch,reset_day,transaction_id,request_binding,lane_day,streak_before,streak_after,jokers_spent,premium) SELECT character_id,world_id,last_claim_epoch,last_claim_day,encode($1,'hex')::uuid,$2,1,6,7,0,false FROM game_character_daily_rewards WHERE character_id=encode($3,'hex')::uuid")
            .bind(id(91).as_slice()).bind([3_u8;32].as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let channel = crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?;
        let room = crate::content::qualify_native_entry_room(world).map_err(debug)?;
        let (bytes, pins) = teleport_bundle();
        let map = crate::map::boot::boot(&bytes, &floor_pins(pins), world, channel, floor_items)
            .map_err(debug)?;
        let cells = map.movement_cells(room.movement_cells()).map_err(debug)?;
        // The fixture's real tile at (5, 1, 8) has the PZ flag; (6, 1, 8) is outside.
        for (start, inside) in [((5, 1, 8), true), ((6, 1, 8), false)] {
            let pin = ChannelContentPin::from_activation(
                world,
                1,
                room.compiled().server_digest(),
                room.compiled().client_digest(),
                map.frame_binding_digest(world),
                map.map_revision_digest(),
                start,
            );
            let deps = Dependencies::with_content(
                pin,
                room.clone(),
                crate::spell::cast::v1_spell_book().map_err(debug)?,
            )?;
            let mut owner = deps.owner(&h, &authority);
            owner.movement_cells = (&cells).into();
            deps.initialize(&owner).await;
            let (actor, session) = (deps.actor, deps.admitted.game_session_id);
            deps.runtime
                .lock()
                .await
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
            // Use the durable loader used by first-entry admission, never an in-memory claim.
            let streak = h
                .root
                .read_daily_reward_streak(
                    &authority,
                    crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?,
                    world,
                )
                .await
                .map_err(debug)?;
            assert_eq!(streak, 7);
            let projected = if zero_streak { 0 } else { streak };
            owner.states.resting_areas.install(session, actor, projected);
            {
                let mut runtime = deps.runtime.lock().await;
                assert_eq!(
                    actor_spell::qualified_protection_zone(&mut runtime, &cells, actor),
                    Some(inside)
                );
                let mut states = owner.states.spell_states.lock().await;
                let state = states.get_mut(&runtime, actor, session).ok_or("actor")?;
                state.restore_saved_vitals(100, 10);
                // Start below the unpromoted soul cap so the cadence can credit a point.
                state
                    .restore_bed_rest([5, 0, 0, 0], owner.owner_now().get())
                    .ok_or("resting soul seed")?;
                if premium {
                    state
                        .apply_owner_premium_transition(true, Some(u64::MAX))
                        .map_err(debug)?;
                }
            }
            // Initialization keeps microseconds; condition ticks truncate to milliseconds.
            // Advance a full second from the last observation, rounding up so even the
            // first tick is due without depending on setup/database latency.
            for _ in 0..120 {
                let next_tick = Duration::from_millis(owner.owner_now().get().div_ceil(1_000))
                    + Duration::from_secs(1);
                owner.clock_origin = std::time::Instant::now()
                    .checked_sub(next_tick)
                    .ok_or("clock")?;
                Box::pin(owner.tick_actor_vitals(actor, session)).await;
            }
            let runtime = deps.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let vitals = states
                .get(&runtime, actor, session)
                .ok_or("actor")?
                .vitals();
            // Free accounts retain the day-2/day-3 rates, with or without a streak.
            // Crystal 201189f8 src/creatures/combat/condition.cpp:1348-1356,1385-1391
            // unlocks resting credits and doubles them, rather than adding a flat bonus
            // (data/libs/systems/daily_reward.lua:1-6). This unfed level-8 Monk has
            // max HP/MP 185/90. Monk regenerates like Paladin (owner 2026-10-29: 1 HP per
            // 8 s, 2 MP per 4 s). REGEN-PCT-4 credits max(185*8/10000, 1*3/8)=0.375 HP
            // per 4 s and max(90*10/10000, 2*3/4)=1.5 MP per 6 s: 120 s gains 11/30,
            // or 22/60 with Premium inside; outside receives the base gains.
            assert_eq!(
                (vitals.health, vitals.mana),
                if premium && inside {
                    (122, 70)
                } else {
                    (111, 40)
                }
            );
            // Free soul stays at the seeded 5: resting_area.rs gates the soul point on
            // Premium, matching docs/reference/tibia-manual/characters.md:58.
            assert_eq!(vitals.soul, if premium && inside { 6 } else { 5 });
            eprintln!(
                "PG_MARKER daily-resting-real-tick-soul premium={premium} zero_streak={zero_streak} inside={inside} soul={}",
                vitals.soul
            );
        }
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
