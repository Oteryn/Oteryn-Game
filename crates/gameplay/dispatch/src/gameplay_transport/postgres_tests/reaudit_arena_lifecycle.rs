//! SRVGAP-B22: the admitted arena route must also accept a distinct Drome lifecycle.
use super::TestResult;
use super::arena_match::fixture;
use super::fixture::Harness;

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
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Drome regression thread panicked")??;
    eprintln!(
        "B22 POSTGRES_CASE_COMPLETED rotation_rank_rewards_replay duration={:?}",
        started.elapsed()
    );
    Ok(())
}
fn scenario() -> TestResult {
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
        .spawn(|| physical().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Drome physical thread panicked")??;
    eprintln!(
        "B22 POSTGRES_CASE_COMPLETED real_wave_exempt_death duration={:?}",
        start.elapsed()
    );
    Ok(())
}
pub(super) fn physical() -> TestResult {
    physical_at(0)
}

pub(super) fn physical_at(starting_wave: u32) -> TestResult {
    physical_context(starting_wave, true, false)
}

pub(super) fn physical_unqualified() -> TestResult {
    physical_context(0, false, false)
}

pub(super) fn physical_hazards() -> TestResult {
    physical_context(0, true, true)
}

fn physical_context(
    starting_wave: u32,
    mechanics_test_context: bool,
    large_hazards: bool,
) -> TestResult {
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
        let (bytes, pins) = fixture::bundle(native)?;
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let world = crate::map::boot::boot(
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
        .map_err(debug)?;
        fixture::runtime(&mut deps, active, &pin, &world)?;
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
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
        let entered = drome_exchange(
            &owner,
            &mut admitted,
            oteryn_protocol_oteryn::arena::DromeAction::Enter,
        )
        .await?;
        if !mechanics_test_context {
            assert_eq!(
                (entered.disposition, entered.phase),
                (6, 0),
                "unqualified Svargrond profiles must not admit a Drome battle"
            );
            assert!(!owner.runtime.lock().await.is_tibiadrome(deps.actor));
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
            let room = owner.rooms.gate_monster_ai();
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
        for target in targets {
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
        }
        assert!(owner.drome_sync(session, false).await);
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
        assert_eq!((status.level, status.start), (1, 1));
        super::srx3_drome::lifetime_highscore_job(
            std::sync::Arc::new(harness.root.clone()),
            &harness.pool,
            deps.admitted.world_id,
            fence.character_id,
        )
        .await?;
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
