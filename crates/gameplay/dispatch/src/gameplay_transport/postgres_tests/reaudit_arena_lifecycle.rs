//! SRVGAP-B22: the admitted arena route must also accept a distinct Drome lifecycle.
use super::TestResult;
use super::arena_match::fixture;
use super::fixture::Harness;

#[path = "reaudit_arena_lifecycle/source.rs"]
mod source_map;

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
        let deps = Dependencies::new()?;
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
        let (controller, _) = super::death_ml_loss_composed::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?;
        owner.active_generation = controller.active();

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
        let (accepted, bytes) = super::guild_wire::exchange(
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
        let (_, denied) = super::guild_wire::exchange(
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
        let (_, rotated) = super::guild_wire::exchange(
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
        super::item_look::seed_main_backpack(&harness, &[]).await?;
        let claim = wire::encode_match_request(ArenaAction::Drome(DromeAction::ClaimPotion))
            .map_err(debug)?;
        let claim_id = admitted.continuity.next_command_id;
        let (_, claimed) = super::guild_wire::exchange(
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
        let (_, again) = super::guild_wire::exchange(
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
                    None
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
        Ok(())
    })
}

#[test]
fn reaudit_arena_lifecycle_postgres_real_wave_and_exempt_death() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let start = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| physical(Physical::Standard).map_err(|e| e.to_string()))?
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
        .spawn(|| physical(Physical::Exploding).map_err(|error| error.to_string()))?
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
        .spawn(|| physical(Physical::Grace).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Drome grace thread panicked")??;
    eprintln!(
        "SRVENC POSTGRES_CASE_COMPLETED drome_grace_cleanup duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[derive(Clone, Copy)]
enum Physical {
    Standard,
    Exploding,
    Grace,
    Source,
    DelayedObservation,
    PendingCorpseEffects,
    ThreeWaveCommandCatchup,
}

#[test]
fn srvenc_tibiadrome_postgres_real_source_hall_runs_an_irregular_native_wave() -> TestResult {
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

fn run_physical_regression(case: Physical) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || physical(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Drome source/deadline thread panicked")??;
    eprintln!(
        "SRVENC POSTGRES_CASE_COMPLETED native_source_or_deadline duration={:?}",
        started.elapsed()
    );
    Ok(())
}

fn physical(case: Physical) -> TestResult {
    let exploding = matches!(case, Physical::Exploding | Physical::PendingCorpseEffects);
    let grace = matches!(case, Physical::Grace);
    use super::fixture::{debug, id, run};
    use super::{Dependencies, prepare_current_session};
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceRewardOccurrence,
    };
    use crate::gameplay_transport::capabilities::SelectedCapabilities;
    use crate::gameplay_transport::connection::FirstEntryOutcome;
    use crate::gameplay_transport::spell_premium_coordinator::{
        SpellPremiumCoordinator, purchase_fixture,
    };
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "b22_drome_physical", false, false).await?;
        super::reset();
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let funded = true;
        let mut deps = Dependencies::new()?;
        let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?;
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let world = if matches!(case, Physical::Source) {
            source_map::world(deps.admitted.world_id, deps.admitted.channel_id, native)?
        } else {
            let (bytes, pins) = fixture::bundle(native)?;
            let base = crate::map::load(&bytes, &pins).map_err(debug)?;
            crate::map::boot::boot(
                &bytes,
                &crate::map::boot::BootPins {
                    bundle: pins,
                    map_revision: crate::map::overlay::map_revision(&base),
                    start: crate::map::overlay::TilePos {
                        x: 32251,
                        y: 31098,
                        floor: -6,
                    },
                },
                deps.admitted.world_id,
                deps.admitted.channel_id,
                |key| {
                    let index = native.item_index()?;
                    Some(crate::map::facts::ItemDefinition {
                        reference: index.definition_ref("Item", key, "definition-r1")?,
                        solid: Some(false),
                        blocks_projectile: false,
                        pickupable: false,
                        door: false,
                    })
                },
            )
            .map_err(debug)?
        };
        if matches!(case, Physical::Source) {
            fixture::runtime_at(&mut deps, active, &pin, &world, source_map::ENTRY)?;
        } else {
            fixture::runtime(&mut deps, active, &pin, &world)?;
        }
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        {
            let runtime = deps.runtime.lock().await;
            owner
                .states
                .spell_states
                .lock()
                .await
                .initialize(
                    &runtime,
                    deps.actor,
                    deps.admitted.game_session_id,
                    crate::spell::cast::CharacterCastFacts {
                        vocation: crate::spell::Vocation::Knight,
                        magic_level: 0,
                        ..crate::gameplay_transport::actor_spell::tests::FACTS
                    },
                    (0, 0),
                    owner.owner_now(),
                )
                .ok_or("player spell initialization")?;
        }
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
            &[4, 6, 17, 170],
        )
        .ok_or("capability selection")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(active.identity().server_artifact_digest().as_slice()).bind(active.identity().client_artifact_digest().as_slice()).bind(world.frame_binding_digest(deps.admitted.world_id).as_slice()).execute(&harness.pool).await?;
        let coins = if funded {
            "oteryn:item.tibia.i3043"
        } else {
            "oteryn:item.tibia.i3035"
        };
        super::item_look::seed_main_backpack(&harness, &[(coins, 2)]).await?;
        fixture::equip_sword(&harness).await?;
        owner.admit_progression_session(&mut admitted).await;
        assert!(
            owner
                .progression_binding(admitted.game_session_id)
                .is_some(),
            "production progression binding"
        );
        fixture::train_character(&owner, admitted.game_session_id).await?;
        let build = harness
            .root
            .read_character_build_state(
                &authority,
                admitted.item_fence.ok_or("fence")?.character_id,
            )
            .await?;
        {
            let runtime = deps.runtime.lock().await;
            assert!(owner.states.spell_states.lock().await.load_owned_build(
                &runtime,
                deps.actor,
                admitted.game_session_id,
                &build
            ));
        }

        let session = admitted.game_session_id;
        let binding = owner
            .progression_binding(session)
            .ok_or("progression binding")?;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?;
        owner
            .root
            .commit_character_experience(
                &authority,
                owner.holder,
                fence,
                ExperienceAwardRequest {
                    occurrence: ExperienceRewardOccurrence::from_bytes(id(91)).map_err(debug)?,
                    amount: oteryn_simulation_determinism::ExactI64::new(27_393_800),
                    context: binding.context.clone(),
                    policy_revision: binding.policy_revision.clone(),
                    reward_revision: binding.reward_revision.clone(),
                    policy: binding.policy.clone(),
                },
            )
            .await
            .map_err(debug)?;
        let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
        harness
            .root
            .latch_premium_activation(
                &activation.activation_id,
                activation.switch_over_us,
                activation.switch_over_us + 60_000_000,
            )
            .await
            .map_err(debug)?;
        let premium = SpellPremiumCoordinator::new(
            consumer,
            Some(clock),
            Some(activation),
            Some(harness.root.clone()),
        );
        owner.premium_coordinator = Some(&premium);
        owner.admit_premium(id(40), session);
        assert!(owner.session_premium_gate(session).await.premium_current());
        let crate::gameplay_transport::spell_character_facts::CastFactsLoad::Ready {
            facts, ..
        } = crate::gameplay_transport::spell_character_facts::load_character_cast_facts(
            &harness.root,
            &authority,
            fence.character_id,
        )
        .await
        else {
            return Err("cast facts".into());
        };
        {
            let runtime = deps.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let state = states
                .get_mut(&runtime, deps.actor, session)
                .ok_or("player")?;
            *state = state
                .after_level_up(facts)
                .ok_or("level-up vitals successor")?;
            state.restore_saved_vitals(200, 100);
        }
        let xp_before = owner
            .root
            .read_character_progression(&authority, fence.character_id)
            .await?
            .ok_or("progression")?
            .total_experience;
        let entered = drome_exchange(
            &owner,
            &mut admitted,
            oteryn_protocol_oteryn::arena::DromeAction::Enter,
        )
        .await?;
        assert_eq!(
            (entered.disposition, entered.phase, entered.wave),
            (1, 1, 1)
        );
        assert_eq!(
            drome_exchange(
                &owner,
                &mut admitted,
                oteryn_protocol_oteryn::arena::DromeAction::Status
            )
            .await?
            .level,
            0,
            "no client-authored kills"
        );
        {
            let mut runtime = deps.runtime.lock().await;
            crate::gameplay_transport::tibiadrome::tick_at(
                &owner,
                &mut runtime,
                std::time::Instant::now() + std::time::Duration::from_secs(9),
            );
        }
        let targets: Vec<_> = {
            let runtime = deps.runtime.lock().await;
            runtime
                .visible_entities()
                .creatures
                .iter()
                .filter(|c| runtime.is_tibiadrome(c.actor))
                .map(|c| c.actor)
                .collect()
        };
        assert_eq!(targets.len(), 4);
        if matches!(case, Physical::ThreeWaveCommandCatchup) {
            owner.states.tibiadrome.test_somersault(session)?;
        }
        if exploding {
            owner.states.tibiadrome.test_exploding_corpses(session)?;
            let mut runtime = deps.runtime.lock().await;
            let before = runtime.read_actor_position(deps.actor).map_err(debug)?;
            let origin = before.position();
            let other = crate::foundation::GameSessionId::decode(&id(94)).map_err(debug)?;
            assert!(
                runtime
                    .return_tibiadrome_player(deps.actor, other, before, origin)
                    .is_err(),
                "a terminal arena return cannot borrow another session"
            );
            assert_eq!(
                runtime
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                origin
            );
            runtime
                .relocate_arena_player(deps.actor, session, before, origin)
                .map_err(debug)?;
            assert!(
                runtime
                    .return_tibiadrome_player(deps.actor, session, before, origin)
                    .is_err(),
                "a terminal arena return compares the current position revision"
            );
            assert!(runtime.is_tibiadrome(deps.actor));
        }
        {
            let mut runtime = deps.runtime.lock().await;
            let outsider_session =
                crate::foundation::GameSessionId::decode(&id(93)).map_err(debug)?;
            let reservation = runtime
                .reserve_fresh_session(outsider_session)
                .map_err(debug)?;
            let outsider = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_first_entry_position(outsider)
                .map_err(debug)?;
            let visible = runtime.visible_entities();
            let target = *targets.first().ok_or("Drome opponent")?;
            let generation = visible
                .creatures
                .iter()
                .find(|c| c.actor == target)
                .ok_or("visible opponent")?
                .generation;
            let reference = oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                identity: target.placement_identity(),
                generation,
            };
            let mut attack = owner.states.attack.lock().await;
            let room = owner.rooms().gate_monster_ai();
            assert_ne!(
                attack.set_target(
                    &runtime,
                    room,
                    outsider,
                    outsider_session,
                    owner.owner_now(),
                    Some(reference),
                    77
                ),
                oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok,
                "outsiders cannot clear another group's waves"
            );
            runtime.set_tibiadrome(outsider, true).map_err(debug)?;
            assert_eq!(
                attack.set_target(
                    &runtime,
                    room,
                    outsider,
                    outsider_session,
                    owner.owner_now(),
                    Some(reference),
                    78
                ),
                oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok,
                "the same physical target is legal for a participant"
            );
            runtime.remove_test_actor(outsider).map_err(debug)?;
        }
        let defeated_targets = targets.clone();
        for (index, target) in targets.into_iter().enumerate() {
            fixture::fight(&mut owner, &mut admitted, deps.actor, target).await?;
            if exploding && index == 0 {
                let mut runtime = deps.runtime.lock().await;
                let (_, corpse) = runtime
                    .borrow_combat_death()
                    .projected_death(target)
                    .map_err(debug)?;
                let before = runtime.read_actor_position(deps.actor).map_err(debug)?;
                runtime
                    .relocate_arena_player(deps.actor, session, before, corpse)
                    .map_err(debug)?;
                let health = owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .get(&runtime, deps.actor, session)
                    .ok_or("Drome participant")?
                    .vitals()
                    .health;
                crate::gameplay_transport::tibiadrome::tick(&owner, &mut runtime);
                let after = owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .get(&runtime, deps.actor, session)
                    .ok_or("Drome participant")?
                    .vitals()
                    .health;
                assert_eq!(
                    after,
                    health - health / 4,
                    "Exploding Corpses takes 25% of CURRENT health"
                );
                crate::gameplay_transport::tibiadrome::tick(&owner, &mut runtime);
                assert_eq!(
                    owner
                        .states
                        .spell_states
                        .lock()
                        .await
                        .get(&runtime, deps.actor, session)
                        .ok_or("Drome participant")?
                        .vitals()
                        .health,
                    after,
                    "a retained native death cannot explode twice"
                );
            }
        }
        if matches!(case, Physical::ThreeWaveCommandCatchup) {
            // Manual combat.md:264: each of three actual waves increases the
            // starting level once even when command settlement catches up later.
            for wave in 2_u32..=3 {
                let observed = std::time::Instant::now()
                    + std::time::Duration::from_secs(u64::from(wave) * 20);
                let targets: Vec<_> = {
                    let mut runtime = deps.runtime.lock().await;
                    crate::gameplay_transport::tibiadrome::tick_at(&owner, &mut runtime, observed);
                    crate::gameplay_transport::tibiadrome::tick_at(
                        &owner,
                        &mut runtime,
                        observed + std::time::Duration::from_secs(9),
                    );
                    runtime
                        .visible_entities()
                        .creatures
                        .iter()
                        .filter(|creature| runtime.is_tibiadrome(creature.actor))
                        .map(|creature| creature.actor)
                        .collect()
                };
                assert_eq!(targets.len(), 4, "native next-wave spawning");
                owner.states.tibiadrome.test_somersault(session)?;
                for target in targets {
                    fixture::fight(&mut owner, &mut admitted, deps.actor, target).await?;
                }
            }
            {
                let mut runtime = deps.runtime.lock().await;
                crate::gameplay_transport::tibiadrome::tick_at(
                    &owner,
                    &mut runtime,
                    std::time::Instant::now() + std::time::Duration::from_secs(80),
                );
            }
            // Deliberately use the real native command before drome_sync, so
            // its separate catch-up owner must observe all three native clears.
            let caught_up = drome_exchange(
                &owner,
                &mut admitted,
                oteryn_protocol_oteryn::arena::DromeAction::Status,
            )
            .await?;
            assert_eq!(
                (caught_up.level, caught_up.start),
                (3, 3),
                "three native victories grant three starting levels, not five"
            );
            let waves:i64=sqlx::query_scalar("SELECT (body->'progress'->>'waves')::bigint FROM game_character_tibiadrome WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(waves, 3);
        }
        if matches!(case, Physical::PendingCorpseEffects) {
            // Manual combat.md:255,264: native defeat earns the wave even
            // while the canonical HP owner temporarily reserves corpse effects.
            let reserved = owner.states.spell_states.lock().await;
            for _ in 0..2 {
                assert!(owner.drome_sync(session, false).await);
                let progress: (i64,i64) = sqlx::query_as("SELECT (body->'progress'->>'level')::bigint,(body->'progress'->>'waves')::bigint FROM game_character_tibiadrome WHERE character_id=encode($1,'hex')::uuid")
                    .bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
                assert_eq!(
                    progress,
                    (1, 1),
                    "one native victory must settle once before corpse effects can acquire their owner"
                );
                let runtime = deps.runtime.lock().await;
                assert!(
                    defeated_targets
                        .iter()
                        .all(|target| runtime.read_actor_position(*target).is_ok()),
                    "retain corpse actors until their effects resolve; do not advance physical spawning"
                );
            }
            drop(reserved);
        }
        if matches!(case, Physical::DelayedObservation) {
            // Native lethal receipts retain their real owner timestamps; only
            // the observer is delayed past this wave's 120-second deadline.
            owner.clock_origin -= std::time::Duration::from_secs(121);
            let mut runtime = deps.runtime.lock().await;
            crate::gameplay_transport::tibiadrome::tick_at(
                &owner,
                &mut runtime,
                std::time::Instant::now() + std::time::Duration::from_secs(131),
            );
        }
        assert!(owner.drome_sync(session, false).await);
        let status = drome_exchange(
            &owner,
            &mut admitted,
            oteryn_protocol_oteryn::arena::DromeAction::Status,
        )
        .await?;
        let expected = if matches!(case, Physical::ThreeWaveCommandCatchup) {
            3
        } else {
            1
        };
        assert_eq!((status.level, status.start), (expected, expected));
        assert_eq!(
            owner
                .root
                .read_character_progression(&authority, fence.character_id)
                .await?
                .ok_or("progression")?
                .total_experience,
            xp_before,
            "Drome kills award no experience"
        );
        if grace {
            // Expire the actual combat-owner in-fight clock before expiry; the
            // newly applied modifier must still be live at the condition save.
            owner.clock_origin -= std::time::Duration::from_secs(61);
            {
                let runtime = deps.runtime.lock().await;
                let mut states = owner.states.spell_states.lock().await;
                let next = states
                    .get(&runtime, deps.actor, session)
                    .ok_or("Drome player")?
                    .try_after_drome_status(
                        crate::spell::cast::DromeStatus::Root,
                        owner.owner_now().get(),
                    )
                    .map_err(debug)?;
                assert!(states.commit(&runtime, deps.actor, session, next));
            }
            let deadline: i64 =
                sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint-1")
                    .fetch_one(&harness.pool)
                    .await?;
            sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=1,current_transport_ref=NULL,control_loss_epoch=1,predecessor_generation=current_generation,original_grace_deadline=$1 WHERE game_session_id=encode($2,'hex')::uuid")
                .bind(deadline).bind(session.as_bytes().as_slice()).execute(&harness.pool).await?;
            deps.runtime
                .lock()
                .await
                .record_control_loss(
                    deps.actor,
                    session,
                    crate::foundation::ControlLossMark {
                        epoch: 1,
                        grace_deadline: deadline,
                    },
                )
                .map_err(debug)?;
            assert_eq!(
                tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    owner.release_after_grace(admitted)
                )
                .await?,
                crate::gameplay_transport::connection::GraceExpiryResult::Released
            );
            let snapshot: Option<serde_json::Value> = sqlx::query_scalar("SELECT snapshot FROM game_character_condition_snapshots WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&harness.pool).await?;
            assert!(
                !snapshot
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.to_string().contains("drome.")),
                "temporary Drome effects must not survive grace expiry"
            );
            let stored: serde_json::Value = sqlx::query_scalar("SELECT body FROM game_character_tibiadrome WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(stored.get("active"), Some(&serde_json::Value::Bool(false)));
            return Ok(());
        }
        let death = {
            let runtime = deps.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            states
                .apply_creature_damage_native(
                    &runtime,
                    deps.actor,
                    session,
                    u32::MAX,
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                        owner.owner_now().get(),
                    ),
                )
                .ok_or("lethal Drome hit")?;
            states
                .player_death(&runtime, deps.actor, session)
                .ok_or("Drome death")?
        };
        let (_, vitals) = owner
            .respawn_after_death(deps.actor, session, death)
            .await
            .ok_or("arena return")?;
        assert_eq!((vitals.health, vitals.mana), (200, 100));
        assert!(owner.drome_sync(session, true).await);
        let deaths:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_death_receipts WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(
            deaths, 0,
            "Drome death never enters XP/skill/item/blessing loss"
        );
        let next = drome_exchange(
            &owner,
            &mut admitted,
            oteryn_protocol_oteryn::arena::DromeAction::Enter,
        )
        .await?;
        assert_eq!(next.disposition, 4);
        assert!(next.cooldown > 0);
        Ok(())
    })
}
async fn drome_exchange(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    action: oteryn_protocol_oteryn::arena::DromeAction,
) -> TestResult<oteryn_protocol_oteryn::arena::DromeResult> {
    use oteryn_protocol_oteryn::arena as wire;
    let id = admitted.continuity.next_command_id;
    let (_, bytes) = super::guild_wire::exchange(
        owner,
        admitted,
        id,
        wire::COMMAND_TYPE_ARENA_MATCH,
        &wire::encode_match_request(wire::ArenaAction::Drome(action))
            .map_err(super::fixture::debug)?,
    )
    .await?;
    wire::DromeResult::decode(&bytes).map_err(super::fixture::debug)
}
