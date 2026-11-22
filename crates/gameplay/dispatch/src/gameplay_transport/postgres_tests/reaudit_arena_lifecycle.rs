//! SRVGAP-B22: the admitted arena route must also accept a distinct Drome lifecycle.
use super::reaudit_arena_lifecycle_fixtures::{Physical, physical_context};

use super::TestResult;

#[test]
fn reaudit_arena_lifecycle() -> TestResult {
    // Existing command 436, additive action 6, Drome operation 1 (status).
    let decoded = oteryn_protocol_oteryn::arena::decode_match_request(&[8, 6, 16, 1]);
    assert!(
        decoded.is_ok(),
        "W17 has no Tibiadrome lifecycle request: {decoded:?}"
    );
    Ok(())
}

#[test]
fn reaudit_arena_lifecycle_postgres_rotation_rank_rewards_and_replay() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Drome regression thread panicked")??;
    eprintln!(
        "B22 POSTGRES_CASE_COMPLETED rotation_rank_rewards_replay duration={:?}",
        started.elapsed()
    );
    Ok(())
}
#[test]
fn srvenc_tibiadrome_postgres_raffle_awards_one_fifth_of_large_rotation() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(true).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Drome raffle thread panicked")??;
    eprintln!(
        "SRVENC POSTGRES_CASE_COMPLETED exact_large_rotation_raffle duration={:?}",
        started.elapsed()
    );
    Ok(())
}

fn scenario(large_rotation: bool) -> TestResult {
    use super::fixture::{Harness, debug, id, run};
    use super::{Dependencies, prepare_current_session};
    use crate::durability::tibiadrome::{Progress, ROTATION_SECONDS, Record};
    use crate::gameplay_transport::capabilities::SelectedCapabilities;
    use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
    use oteryn_protocol_oteryn::arena::{self as wire, ArenaAction, DromeAction};
    run(async |admin| {
        let harness = Harness::create_with_admission_seed(admin, "b22_drome", false, false).await?;
        super::reset();
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = super::fixture::fence(1)?.runtime_scope.world_id();
        let (controller, pin) = super::death_ml_loss_composed_fixtures::production_generation_with_pin(
            world,
            "content-1",
        )?;
        let active = controller.active().ok_or("production generation")?;
        let native = active.native_gameplay().ok_or("native generation")?;
        let room = pin.qualified_room().clone();
        let deps = Dependencies::with_content(
            pin.into_channel_pin(),
            room,
            native.spell_book().clone(),
        )?;
        {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    deps.admitted.game_session_id,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                        2,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;

            let at = runtime.read_actor_position(deps.actor).map_err(debug)?;
            runtime
                .relocate_arena_player(
                    deps.actor,
                    deps.admitted.game_session_id,
                    at,
                    crate::foundation::MovementLocalPosition {
                        x: 32251,
                        y: 31098,
                        floor: 6,
                    },
                )
                .map_err(debug)?;
        }
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);

        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(deps.admitted.game_session_id)
        .await?
        .0;
        let mut admitted = deps.admitted;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[170],
        )
        .ok_or("capabilities")?;
        owner.admit_progression_session(&mut admitted).await;
        let status =
            wire::encode_match_request(ArenaAction::Drome(DromeAction::Status)).map_err(debug)?;
        let first_id = admitted.continuity.next_command_id;
        let (accepted, bytes) = super::guild_wire_fixtures::exchange(
            &owner,
            &mut admitted,
            first_id,
            wire::COMMAND_TYPE_ARENA_MATCH,
            &status,
        )
        .await?;
        assert_eq!(accepted, oteryn_protocol_oteryn::CommandStatus::Accepted);
        let initial = wire::DromeResult::decode(&bytes).map_err(debug)?;
        assert_eq!((initial.level, initial.points, initial.rank), (0, 0, 0));
        let replay = owner
            .arena_match_command(admitted, first_id, &status)
            .await
            .ok_or("status replay")?;
        assert_eq!(replay, bytes);
        let entry =
            wire::encode_match_request(ArenaAction::Drome(DromeAction::Enter)).map_err(debug)?;
        let entry_id = admitted.continuity.next_command_id;
        let (_, denied) = super::guild_wire_fixtures::exchange(
            &owner,
            &mut admitted,
            entry_id,
            wire::COMMAND_TYPE_ARENA_MATCH,
            &entry,
        )
        .await?;
        assert_eq!(
            wire::DromeResult::decode(&denied)
                .map_err(debug)?
                .disposition,
            3,
            "non-premium/below-50 admission is refused"
        );
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&harness.pool)
                .await?;
        let rotation = u32::try_from(now)? / ROTATION_SECONDS;
        let state = Record {
            progress: Progress {
                lifetime_levels: None,
                rotation: rotation - 1,
                level: 25,
                start: 25,
                waves: 1,
                ..Progress::default()
            },
            ..Record::default()
        };
        sqlx::query("UPDATE game_character_tibiadrome SET body=$2 WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).bind(serde_json::to_value(state)?).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_tibiadrome_scores(world_id,rotation,character_id,level,experience) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,25,1234)").bind(id(42).as_slice()).bind(i64::from(rotation-1)).bind(id(41).as_slice()).execute(&harness.pool).await?;
        // A concurrent participant may hold its current root while the previous
        // rotation is published. Award FKs must lock its committed score, not that root.
        let mut seed = harness.pool.begin().await?;

        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *seed)
            .await?;
        sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) SELECT encode($2,'hex')::uuid,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,'Drome Other' FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).bind(id(92).as_slice()).execute(&mut *seed).await?;

        sqlx::query("INSERT INTO game_tibiadrome_scores(world_id,rotation,character_id,level,experience) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,25,2000)")
            .bind(id(42).as_slice()).bind(i64::from(rotation-1)).bind(id(92).as_slice()).execute(&mut *seed).await?;
        if large_rotation {
            // Manual combat.md:284: 320 participants leave 300 beyond the
            // ranked twenty; exactly 60 receive weighted raffle potions.
            // Give each additional participant a real UUIDv7 account guard.
            sqlx::query("INSERT INTO game_character_account_guards(account_id) SELECT ('10000000-0000-7000-8000-'||lpad(n::text,12,'0'))::uuid FROM generate_series(1,318) AS sample(n)")
                .execute(&mut *seed).await?;
            sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) SELECT ('20000000-0000-7000-8000-'||lpad(n::text,12,'0'))::uuid,('10000000-0000-7000-8000-'||lpad(n::text,12,'0'))::uuid,r.world_id,r.lifecycle,r.character_revision,r.profile_revision,r.ruleset_revision,r.content_revision,r.starter_template_revision,'Drome Raffle '||chr(65+(n-1)/26)||chr(65+(n-1)%26) FROM game_character_roots r CROSS JOIN generate_series(1,318) AS sample(n) WHERE r.character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).execute(&mut *seed).await?;
            sqlx::query("INSERT INTO game_tibiadrome_scores(world_id,rotation,character_id,level,experience) SELECT encode($1,'hex')::uuid,$2,('20000000-0000-7000-8000-'||lpad(n::text,12,'0'))::uuid,25,10000+n FROM generate_series(1,318) AS sample(n)")
                .bind(id(42).as_slice()).bind(i64::from(rotation-1)).execute(&mut *seed).await?;
        }

        seed.commit().await?;
        let mut concurrent = harness.pool.begin().await?;
        sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE").bind(id(92).as_slice()).execute(&mut *concurrent).await?;
        let next_id = admitted.continuity.next_command_id;
        let (_, rotated) = super::guild_wire_fixtures::exchange(
            &owner,
            &mut admitted,
            next_id,
            wire::COMMAND_TYPE_ARENA_MATCH,
            &status,
        )
        .await?;
        concurrent.rollback().await?;
        let result = wire::DromeResult::decode(&rotated).map_err(debug)?;

        assert_eq!(
            (result.level, result.start, result.points, result.potions),
            (0, 20, 250, 3)
        );
        let award:(i64,i32)=sqlx::query_as("SELECT rank,potions FROM game_tibiadrome_awards WHERE character_id=encode($1,'hex')::uuid AND rotation=$2").bind(id(41).as_slice()).bind(i64::from(rotation-1)).fetch_one(&harness.pool).await?;
        assert_eq!(award, (1, 3));
        if large_rotation {
            let counts: (i64,i64) = sqlx::query_as("SELECT count(*),count(*) FILTER(WHERE rank>20) FROM game_tibiadrome_awards WHERE world_id=encode($1,'hex')::uuid AND rotation=$2")
                .bind(id(42).as_slice()).bind(i64::from(rotation-1)).fetch_one(&harness.pool).await?;
            assert_eq!(
                counts,
                (80, 60),
                "20 ranked winners plus exactly 20% of the other 300 participants"
            );
        }
        super::item_look_fixtures::seed_main_backpack(&harness, &[]).await?;
        let claim = wire::encode_match_request(ArenaAction::Drome(DromeAction::ClaimPotion))
            .map_err(debug)?;
        let claim_id = admitted.continuity.next_command_id;
        let (_, claimed) = super::guild_wire_fixtures::exchange(
            &owner,
            &mut admitted,
            claim_id,
            wire::COMMAND_TYPE_ARENA_MATCH,
            &claim,
        )
        .await?;
        let result = wire::DromeResult::decode(&claimed).map_err(debug)?;
        assert_eq!((result.disposition, result.potions), (1, 2));
        assert_eq!(
            owner
                .arena_match_command(admitted, claim_id, &claim)
                .await
                .ok_or("potion replay")?,
            claimed
        );
        let minted:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key BETWEEN 'oteryn:item.tibia.i36723' AND 'oteryn:item.tibia.i36742'").fetch_one(&harness.pool).await?;
        assert_eq!(minted, 1, "lost ack/replay does not mint a second potion");
        let next_id = admitted.continuity.next_command_id;
        let (_, again) = super::guild_wire_fixtures::exchange(
            &owner,
            &mut admitted,
            next_id,
            wire::COMMAND_TYPE_ARENA_MATCH,
            &status,
        )
        .await?;
        assert_eq!(
            wire::DromeResult::decode(&again).map_err(debug)?.points,
            250,
            "rotation points are paid exactly once"
        );
        if large_rotation {
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_tibiadrome_awards WHERE world_id=encode($1,'hex')::uuid AND rotation=$2")
                .bind(id(42).as_slice()).bind(i64::from(rotation-1)).fetch_one(&harness.pool).await?;
            assert_eq!(
                count, 80,
                "repeated native reads cannot allocate a second raffle"
            );
        }
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "current fence")?
            .ok_or("current fence")?;
        let attempt = crate::foundation::CommandRef::new(
            admitted.game_session_id,
            crate::foundation::CommandId::new(999).map_err(debug)?,
        );
        let read = owner
            .root
            .read_tibiadrome(&authority, owner.holder, fence, attempt, &status)
            .await
            .map_err(debug)?;
        let mut forged = read.state.clone();
        forged.progress.points += 1;
        assert!(
            owner
                .root
                .commit_tibiadrome(
                    &authority,
                    owner.holder,
                    fence,
                    attempt,
                    status.clone(),
                    read.state,
                    forged,
                    1,
                    None,
                    None,
                )
                .await
                .is_err(),
            "the durable owner cannot mint caller-supplied Drome points"
        );
        let mut stale = admitted;
        stale
            .item_fence
            .as_mut()
            .ok_or("fence")?
            .character_lease_generation += 1;
        assert!(
            owner
                .arena_match_command(stale, admitted.continuity.next_command_id, &status)
                .await
                .is_none()
        );
        let changed = wire::encode_match_request(ArenaAction::Drome(DromeAction::LowerStart))
            .map_err(debug)?;
        assert!(
            owner
                .arena_match_command(admitted, first_id, &changed)
                .await
                .is_none(),
            "same command id cannot change its intent"
        );
        if !large_rotation {
            let buy = wire::encode_match_request(ArenaAction::Drome(DromeAction::Buy(17)))
                .map_err(debug)?;
            let refused = owner
                .arena_match_command(admitted, admitted.continuity.next_command_id, &buy)
                .await
                .ok_or("unaffordable Phant reply")?;
            assert_eq!(wire::DromeResult::decode(&refused).map_err(debug)?.disposition, 7);
            assert_eq!(phantastic_grants(&harness).await?, 0);
            sqlx::query(
                "UPDATE game_character_tibiadrome
                 SET body=jsonb_set(body,'{progress,points}','8000')
                 WHERE character_id=encode($1,'hex')::uuid",
            )
            .bind(id(41).as_slice())
            .execute(&harness.pool)
            .await?;
            let command = admitted.continuity.next_command_id + 1;
            let bought = owner
                .arena_match_command(admitted, command, &buy)
                .await
                .ok_or("Phant purchase reply")?;
            let result = wire::DromeResult::decode(&bought).map_err(debug)?;
            assert_eq!((result.disposition, result.points), (1, 0));
            assert_eq!(phantastic_grants(&harness).await?, 1);
            assert_eq!(
                owner.arena_match_command(admitted, command, &buy).await,
                Some(bought)
            );
            assert_eq!(phantastic_grants(&harness).await?, 1);
        }
        Ok(())
    })
}

async fn phantastic_grants(harness: &super::fixture::Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievement_grant_requests
         WHERE achievement_key='oteryn:achievement/phantastic'",
    )
    .fetch_one(&harness.pool)
    .await?)
}

#[test]
fn reaudit_arena_lifecycle_postgres_real_wave_and_exempt_death() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let start = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| physical_case(Physical::Standard).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Drome physical thread panicked")??;
    eprintln!(
        "B22 POSTGRES_CASE_COMPLETED real_wave_exempt_death duration={:?}",
        start.elapsed()
    );
    Ok(())
}
#[test]
fn srvenc_tibiadrome_postgres_exploding_corpse_hits_current_health_once() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| physical_case(Physical::Exploding).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Drome modifier thread panicked")??;
    eprintln!(
        "SRVENC POSTGRES_CASE_COMPLETED exploding_corpse_once duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn srvenc_tibiadrome_postgres_grace_expiry_clears_temporary_status_before_save() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| physical_case(Physical::Grace).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Drome grace thread panicked")??;
    eprintln!(
        "SRVENC POSTGRES_CASE_COMPLETED drome_grace_cleanup duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn srvenc_tibiadrome_postgres_source_hall_refuses_unqualified_native_pool() -> TestResult {
    run_physical_regression(Physical::Source)
}

#[test]
fn srvenc_tibiadrome_postgres_native_kills_before_deadline_survive_delayed_observation()
-> TestResult {
    run_physical_regression(Physical::DelayedObservation)
}

#[test]
fn srvenc_tibiadrome_postgres_native_victory_credits_before_reserved_corpse_effects() -> TestResult
{
    run_physical_regression(Physical::PendingCorpseEffects)
}

#[test]
fn srvenc_tibiadrome_postgres_three_native_waves_command_catchup_counts_each_once() -> TestResult {
    run_physical_regression(Physical::ThreeWaveCommandCatchup)
}

#[test]
fn srvenc_tibiadrome_postgres_cadence_credits_higher_personal_start_once() -> TestResult {
    run_physical_regression(Physical::SharedLowerWave)
}

#[test]
fn srx3_drome_sealed_melee_somersault_publishes_both_native_teleport_cues_postgres() -> TestResult {
    run_physical_regression(Physical::SomersaultFx)
}

fn run_physical_regression(case: Physical) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || physical_case(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Drome source/deadline thread panicked")??;
    eprintln!(
        "SRVENC POSTGRES_CASE_COMPLETED native_source_or_deadline duration={:?}",
        started.elapsed()
    );
    Ok(())
}

fn physical_case(case: Physical) -> TestResult {
    physical_context(0, true, false, case)
}
