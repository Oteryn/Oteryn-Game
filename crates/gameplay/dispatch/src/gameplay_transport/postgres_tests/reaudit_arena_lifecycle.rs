//! SRVGAP-B22: the admitted arena route must also accept a distinct Drome lifecycle.
use super::TestResult;
use super::arena_match::fixture;
use super::fixture::{Harness, debug, id};

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

#[derive(Clone, Copy)]
enum Physical {
    Standard,
    Exploding,
    Grace,
    Source,
    DelayedObservation,
    PendingCorpseEffects,
    ThreeWaveCommandCatchup,
    SharedLowerWave,
    SomersaultFx,
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

pub(super) fn physical() -> TestResult {
    physical_at(0)
}

pub(super) fn physical_at(starting_wave: u32) -> TestResult {
    physical_context(starting_wave, true, false, Physical::Standard)
}

pub(super) fn physical_unqualified() -> TestResult {
    physical_context(0, false, false, Physical::Standard)
}

pub(super) fn physical_hazards() -> TestResult {
    physical_context(0, true, true, Physical::Standard)
}

async fn persisted_drome_record(
    harness: &Harness,
) -> TestResult<crate::durability::tibiadrome::Record> {
    let body: serde_json::Value = sqlx::query_scalar(
        "SELECT body FROM game_character_tibiadrome WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&harness.pool)
    .await?;
    Ok(serde_json::from_value(body)?)
}

async fn replay_observed_drome_update(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    session: crate::foundation::GameSessionId,
    observed: &crate::durability::tibiadrome::Record,
) -> TestResult {
    let mut fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|_| "current Drome replay fence")?
        .ok_or("current Drome replay fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    fence.expected_character_revision = slot
        .cursor(owner.root, owner.character)
        .await
        .map_err(debug)?;
    owner
        .root
        .settle_tibiadrome(
            owner.character,
            owner.holder,
            fence,
            crate::durability::tibiadrome::RunUpdate {
                run: observed.run,
                completed: observed.completed,
                level: observed.progress.level,
                ended: !observed.active,
                wave: observed.wave,
            },
        )
        .await
        .map_err(debug)?;
    Ok(())
}

fn physical_context(
    starting_wave: u32,
    mechanics_test_context: bool,
    large_hazards: bool,
    case: Physical,
) -> TestResult {
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
            let full = (facts.max_health, facts.max_mana);
            *state = state
                .after_level_up(facts)
                .ok_or("level-up vitals successor")?;
            if large_hazards {
                // The receipt case advances owner time with full vitals from the real build.
                state.restore_saved_vitals(full.0, full.1);
            } else {
                state.restore_saved_vitals(200, 100);
            }
        }
        let xp_before = owner
            .root
            .read_character_progression(&authority, fence.character_id)
            .await?
            .ok_or("progression")?
            .total_experience;
        if starting_wave != 0 {
            let now: i64 =
                sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                    .fetch_one(&harness.pool)
                    .await?;
            let seeded = crate::durability::tibiadrome::Record {
                progress: crate::durability::tibiadrome::Progress {
                    rotation: u32::try_from(now)? / crate::durability::tibiadrome::ROTATION_SECONDS,
                    start: starting_wave,
                    ..Default::default()
                },
                ..Default::default()
            };
            sqlx::query("INSERT INTO game_character_tibiadrome(character_id,body) VALUES(encode($1,'hex')::uuid,$2) ON CONFLICT(character_id) DO UPDATE SET body=EXCLUDED.body")
                .bind(id(41).as_slice()).bind(serde_json::to_value(seeded)?).execute(&harness.pool).await?;
        }
        let mechanics_test_context = mechanics_test_context && !matches!(case, Physical::Source);
        owner
            .states
            .tibiadrome
            .mechanics_test_context
            .store(mechanics_test_context, std::sync::atomic::Ordering::Release);
        owner
            .states
            .tibiadrome
            .mechanics_test_hazards
            .store(large_hazards, std::sync::atomic::Ordering::Release);
        let refusal_before = if !mechanics_test_context {
            let physical = {
                let runtime = deps.runtime.lock().await;
                (
                    runtime.read_actor_position(deps.actor).map_err(debug)?,
                    runtime
                        .visible_entities()
                        .creatures
                        .iter()
                        .map(|creature| creature.actor)
                        .collect::<Vec<_>>(),
                )
            };
            let backpack = owner
                .root
                .read_character_backpack(&authority, fence.character_id)
                .await?;
            let fees: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_fee_burns")
                .fetch_one(&harness.pool)
                .await?;
            Some((physical, backpack, fees))
        } else {
            None
        };
        let entered = drome_exchange(
            &owner,
            &mut admitted,
            oteryn_protocol_oteryn::arena::DromeAction::Enter,
        )
        .await?;
        if let Some(((position, creatures), backpack, fees)) = refusal_before {
            assert_eq!(
                (entered.disposition, entered.phase, entered.wave),
                (6, 0, 0),
                "a source hall or Svargrond profiles alone cannot qualify a native Drome pool"
            );
            {
                let runtime = deps.runtime.lock().await;
                assert!(!runtime.is_tibiadrome(deps.actor));
                assert_eq!(
                    runtime.read_actor_position(deps.actor).map_err(debug)?,
                    position
                );
                assert_eq!(
                    runtime
                        .visible_entities()
                        .creatures
                        .iter()
                        .map(|creature| creature.actor)
                        .collect::<Vec<_>>(),
                    creatures,
                    "refused entry cannot admit replacement native opponents"
                );
            }
            assert_eq!(
                owner
                    .root
                    .read_character_backpack(&authority, fence.character_id)
                    .await?,
                backpack,
                "refused entry preserves every carried item and quantity"
            );
            let after_fees: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_fee_burns")
                .fetch_one(&harness.pool)
                .await?;
            assert_eq!(after_fees, fees, "refused entry cannot charge an arena fee");
            return Ok(());
        }
        assert_eq!(
            (entered.disposition, entered.phase, entered.wave),
            (1, 1, starting_wave)
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
        let before_clears = if matches!(case, Physical::ThreeWaveCommandCatchup) {
            let observed = persisted_drome_record(&harness).await?;
            assert!(observed.active);
            assert_eq!((observed.completed, observed.progress.level), (0, 0));
            Some(observed)
        } else {
            None
        };
        if matches!(case, Physical::SharedLowerWave) {
            // A higher-start participant can join an existing wave-zero run.
            // Seed only that personal predecessor; all clears below still
            // require the actual admitted native opponents and lethal receipts.
            let mut joined = persisted_drome_record(&harness).await?;
            assert_eq!(
                (joined.wave, joined.completed, joined.progress.start),
                (0, 0, 0)
            );
            joined.progress.start = 10;
            joined.progress.lifetime_levels = Some(20);
            let updated = sqlx::query(
                "UPDATE game_character_tibiadrome SET body=$2 WHERE character_id=encode($1,'hex')::uuid",
            )
            .bind(id(41).as_slice())
            .bind(serde_json::to_value(joined)?)
            .execute(&harness.pool)
            .await?;
            assert_eq!(updated.rows_affected(), 1);
        }
        {
            let mut runtime = deps.runtime.lock().await;
            crate::gameplay_transport::tibiadrome::tick_at(
                &owner,
                &mut runtime,
                std::time::Instant::now() + std::time::Duration::from_secs(9),
            );
        }
        if large_hazards {
            return super::srx3_drome_hazards::large_hazards_receipt(
                &mut owner,
                &mut admitted,
                &harness,
            )
            .await;
        }
        let mut panel = oteryn_dev_client::client_ui::arena::ArenaPanel {
            selected: true,
            ..Default::default()
        };
        let request = oteryn_dev_client::client_ui::arena::request(
            &mut panel,
            oteryn_dev_client::client_ui::arena::Intent::Action(
                oteryn_protocol_oteryn::arena::ArenaAction::Drome(
                    oteryn_protocol_oteryn::arena::DromeAction::Status,
                ),
            ),
        )
        .ok_or("native phase refresh")?;
        let command_id = admitted.continuity.next_command_id;
        let (status, payload) = super::guild_wire::exchange(
            &owner,
            &mut admitted,
            command_id,
            request.kind,
            &request.payload,
        )
        .await?;
        assert!(oteryn_dev_client::client_ui::arena::apply_result(
            &mut panel,
            &oteryn_dev_client::client_runtime::raw_feature::GatedCommandResult {
                command_id,
                status,
                server_sequence: 1,
                payload,
            },
        ));
        assert_eq!(panel.drome.ok_or("decoded native combat phase")?.phase, 2);
        assert!(oteryn_dev_client::client_ui::arena::panel_text(&panel).contains("Fighting: wave"));
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
        if matches!(case, Physical::SomersaultFx) {
            // The existing admitted physical fixture has a sealed ModifierOwner.
            // This exercises its real draw and pending-effect consumer, not a raw
            // Teleport effect or a second modifier engine.
            {
                let mut runtime = deps.runtime.lock().await;
                owner
                    .states
                    .tibiadrome
                    .test_somersault(&mut runtime, session)?;
            }
            let actor = *targets.first().ok_or("actual admitted Drome opponent")?;
            let baseline = world
                .facts()
                .movement_effects_since(0)
                .last()
                .map_or(0, |entry| entry.0);
            let mut observed = None;
            for _ in 0..256 {
                let mut runtime = deps.runtime.lock().await;
                let before = runtime.read_actor_position(actor).map_err(debug)?;
                crate::gameplay_transport::tibiadrome::modifiers::melee(
                    &owner,
                    &mut runtime,
                    actor,
                );
                crate::gameplay_transport::tibiadrome::tick(&owner, &mut runtime);
                let after = runtime.read_actor_position(actor).map_err(debug)?;
                if before.position() != after.position() {
                    observed = Some((
                        owner
                            .wire_observation(&runtime, before.position())
                            .ok_or("Somersault source frame")?
                            .actor_position,
                        owner
                            .wire_observation(&runtime, after.position())
                            .ok_or("Somersault destination frame")?
                            .actor_position,
                    ));
                    break;
                }
            }
            let (from, to) = observed.ok_or("real sealed Somersault draw must commit movement")?;
            assert_ne!(
                from, to,
                "cue assertion requires actual committed translation"
            );
            let effects: Vec<_> = world
                .facts()
                .movement_effects_since(baseline)
                .into_iter()
                .filter(|(_, _, effect)| *effect == 11)
                .map(|(_, at, _)| (i32::from(at.x), i32::from(at.y), i16::from(at.floor)))
                .collect();
            assert_eq!(
                effects,
                vec![(from.x, from.y, from.floor), (to.x, to.y, to.floor)],
                "one committed sealed Somersault must publish effect11 at actual origin and destination"
            );
        }

        if matches!(case, Physical::ThreeWaveCommandCatchup) {
            let mut runtime = deps.runtime.lock().await;
            owner
                .states
                .tibiadrome
                .test_somersault(&mut runtime, session)?;
        }
        if exploding {
            let mut runtime = deps.runtime.lock().await;
            owner
                .states
                .tibiadrome
                .test_exploding_corpses(&mut runtime, session)?;
            assert!(runtime.permits_tibiadrome_health_loss(deps.actor, 25));
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
            for target in &targets {
                // ARENA1 A4 / ARENA1-RL-20: round(base_hit * 1.039^wave).
                // These actors were spawned and scaled by the production Drome tick.
                assert_eq!(
                    runtime
                        .encounter_source_damage(*target, 1_000, 0)
                        .map_err(debug)?,
                    (
                        if starting_wave == 0 {
                            1_000
                        } else {
                            92_907_061
                        },
                        false
                    ),
                    "spawned wave must use its accepted source scaling"
                );
                let before = runtime.companion_snapshot(*target).map_err(debug)?;
                let base = runtime
                    .companion_policy(&before.state.policy.definition_key)
                    .map_err(debug)?;
                if starting_wave == 0 {
                    assert_eq!(before.maximum_health, base.maximum_health);
                }
                assert!(runtime.scale_tibiadrome_opponent(*target, 301).is_err());
                assert_eq!(
                    runtime.companion_snapshot(*target).map_err(debug)?,
                    before,
                    "out-of-range waves cannot mutate a live opponent"
                );
                for (wave, expected) in [
                    (0, 1_000),
                    (1, 1_039),
                    (10, 1_466),
                    (100, 45_873),
                    (300, 96_530_436),
                ] {
                    runtime
                        .scale_tibiadrome_opponent(*target, wave)
                        .map_err(debug)?;
                    assert_eq!(
                        runtime
                            .encounter_source_damage(*target, 1_000, 0)
                            .map_err(debug)?,
                        (expected, false)
                    );
                    if wave == 1 {
                        assert_eq!(
                            runtime
                                .companion_snapshot(*target)
                                .map_err(debug)?
                                .maximum_health,
                            (base.maximum_health * 1_039 + 500) / 1_000
                        );
                    }
                }
                runtime
                    .scale_tibiadrome_opponent(*target, starting_wave)
                    .map_err(debug)?;
                assert_eq!(
                    runtime.companion_snapshot(*target).map_err(debug)?,
                    before,
                    "scaling is based on the admitted profile, never the previous scaled HP"
                );
            }
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
            if starting_wave == 0 {
                fixture::fight(&mut owner, &mut admitted, deps.actor, target).await?;
            } else {
                use crate::foundation::runtime_actor_spell_types::{
                    OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
                };
                let mut runtime = deps.runtime.lock().await;
                let now = owner.owner_now();
                let batch = OwnerCombatBatch {
                    caster: deps.actor,
                    attacker: crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    current_lease_generation: fence.character_lease_generation,
                    command: crate::foundation::CommandRef::new(
                        session,
                        crate::foundation::CommandId::new(900 + u64::from(target.actor_local_id()))
                            .map_err(debug)?,
                    ),
                    occurrence: SpellOccurrenceBinding {
                        id: "srx3:drome-ceiling-lethal".into(),
                        revisions: ["r1", "r1", "r1", "r1", "r1"].map(str::to_owned),
                    },
                    binding: b"srx3-drome-ceiling-lethal".to_vec(),
                    anchor: None,
                    now_us: Some(now),
                    now_ms: now.get() / 1_000,
                    effects: vec![OwnerCombatEffect {
                        target,
                        sub_ordinal: 0,
                        change: OwnerCombatChange::Damage {
                            target_atom: runtime
                                .creature_spell_target_atom(target)
                                .map_err(debug)?,
                            magnitude: runtime.companion_snapshot(target).map_err(debug)?.health,
                        },
                    }],
                    deferred: None,
                };
                let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
                let receipt = runtime.commit_spell_batch(staged).map_err(debug)?;
                assert!(receipt.applied);
                assert_eq!(
                    receipt.effects[0]
                        .health
                        .ok_or("actual canonical lethal HP receipt")?
                        .health_after,
                    0,
                    "the real canonical batch must actually kill the scaled opponent"
                );
                let mut death = runtime.borrow_combat_death();
                let lethal = death.committed_lethal_receipt(target).map_err(debug)?;
                death.project_committed_lethal(lethal).map_err(debug)?;
            }
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
                {
                    let mut runtime = deps.runtime.lock().await;
                    owner
                        .states
                        .tibiadrome
                        .test_somersault(&mut runtime, session)?;
                }
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
            let latest = persisted_drome_record(&harness).await?;
            assert_eq!(latest.progress.lifetime_levels, Some(3));
            let earlier = before_clears
                .as_ref()
                .ok_or("actual pre-clear Drome snapshot")?;
            assert!(latest.active);
            assert_eq!(earlier.run, latest.run);
            assert!(earlier.completed < latest.completed);
            assert!(earlier.wave < latest.wave);
            replay_observed_drome_update(&owner, session, earlier).await?;
            assert_eq!(
                persisted_drome_record(&harness).await?,
                latest,
                "an older cadence observation cannot rewind the wave or duplicate credit"
            );
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
        if matches!(case, Physical::SharedLowerWave) {
            let settled = persisted_drome_record(&harness).await?;
            assert_eq!(
                (
                    settled.progress.start,
                    settled.progress.level,
                    settled.completed
                ),
                (11, 1, 1),
                "cadence awards the native clear on top of the higher personal start"
            );
            assert_eq!(
                (settled.progress.waves, settled.progress.lifetime_levels),
                (1, Some(21))
            );
            assert!(owner.drome_sync(session, false).await);
            assert_eq!(persisted_drome_record(&harness).await?, settled);
        }
        let status = drome_exchange(
            &owner,
            &mut admitted,
            oteryn_protocol_oteryn::arena::DromeAction::Status,
        )
        .await?;
        if starting_wave == 299 {
            assert_eq!(
                (status.level, status.start, status.wave, status.phase),
                (300, 295, 300, 0)
            );
            assert!(status.cooldown > 0);
            let terminal = persisted_drome_record(&harness).await?;
            assert!(!terminal.active);
            assert_eq!(
                (terminal.wave, terminal.progress.level),
                (status.wave, status.level)
            );
            replay_observed_drome_update(&owner, session, &terminal).await?;
            assert_eq!(
                persisted_drome_record(&harness).await?,
                terminal,
                "terminal replay cannot extend cooldown or award credit again"
            );
            let runtime = deps.runtime.lock().await;
            assert!(
                !runtime
                    .visible_entities()
                    .creatures
                    .iter()
                    .any(|c| runtime.is_tibiadrome(c.actor))
            );
            return Ok(());
        }
        let expected = if matches!(case, Physical::ThreeWaveCommandCatchup) {
            3
        } else {
            1
        };
        if matches!(case, Physical::SharedLowerWave) {
            assert_eq!((status.level, status.start), (1, 11));
        } else {
            assert_eq!((status.level, status.start), (expected, expected));
        }
        if expected == 1 && !matches!(case, Physical::SharedLowerWave) {
            super::srx3_drome::lifetime_highscore_job(
                std::sync::Arc::new(harness.root.clone()),
                &harness.pool,
                deps.admitted.world_id,
                fence.character_id,
            )
            .await?;
        }
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
