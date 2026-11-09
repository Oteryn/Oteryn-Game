//! A real command 178 moves the unique native player and its live spell state,
//! commits the durable Instance session, then crosses domain 99 before the new
//! owner's initial snapshot. All room/boss values here are isolated test data.
use super::fixture::{self, Harness, debug, run};
use super::{Dependencies, TestResult, reset};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{
    ChannelRuntimeV1, CharacterLease, CommandStatus, MessageType, MovementLocalPosition,
    RuntimeScopeRefV1, decode_wire_envelope,
};
use crate::gameplay_transport::activity_runtime::{ActivityRuntimeAssets, ActivityTemplateKey};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use crate::gameplay_transport::{ComposedFreshAdmission, SeamRooms, item_fence_of};
use oteryn_protocol_oteryn::activity_scope::{
    ActivityScope, ActivityScopeSnapshot, STATE_DOMAIN_ACTIVITY_SCOPE, decode_activity_scope,
};
use oteryn_protocol_oteryn::boss_encounter::{
    self as wire, BossActivityRefusal, BossActivityReply,
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, decode_command_result, decode_snapshot_chunk, encode_client_command,
};
use tokio::io::AsyncWriteExt;

// This isolated catalogue exercises the real cooldown writer, whose
// ProductionKey validator rejects names containing a fixture marker.
const BOSS: &str = "oteryn:boss.native_activity";
const CREATURE: &str = "fixture:activity_boss";
const TEMPLATE: &str = "oteryn:room.native_activity_fixture";

#[path = "boss_activity/endgame.rs"]
mod endgame;
#[path = "boss_activity/endgame_receipts.rs"]
mod endgame_receipts;
#[path = "boss_activity/instance_corpse.rs"]
mod instance_corpse;
#[path = "boss_activity/nonreward.rs"]
mod nonreward;
#[path = "boss_activity/offline_death.rs"]
mod offline_death;
#[path = "boss_activity/recovery.rs"]
mod recovery;
#[path = "boss_activity/rewards.rs"]
mod rewards;
#[path = "boss_activity/state_continuity.rs"]
mod state_continuity;
#[path = "boss_activity/state_effects.rs"]
mod state_effects;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    Entry,
    AwardContinuity,
    EndgameReceipts,
    Rewards,
    InstanceCorpse,
    ProductionRewards,
    LateRewards,
    NonRewardDefeat,
    Recovery,
    RecoveryCancelled,
    RecoveryProtection,
    CapabilityMismatch,
    OfflineDeathMismatch,
    OfflineDeathDeadline,
    SoulWarEntry,
    SoulWarFinalEntry,
    RottenBloodEntry,
    SoulWarDefeat,
    SoulWarLateDefeat,
    SoulWarFinalDefeat,
    RottenBloodDefeat,
    PrimalDefeat,
}

#[test]
fn boss_activity_postgres_tagged_instance_primal_receipt_replay_and_scope_fences() -> TestResult {
    run_scenario(Case::EndgameReceipts)
}

#[test]
fn boss_activity_postgres_real_wire_native_entry_and_scope_barrier() -> TestResult {
    run_scenario(Case::Entry)
}

#[test]
fn boss_activity_postgres_native_award_effects_follow_return_and_apply_once() -> TestResult {
    run_scenario(Case::AwardContinuity)
}

#[test]
fn boss_activity_postgres_real_instance_native_boss_death_and_reward_custody() -> TestResult {
    run_scenario(Case::Rewards)
}

#[test]
fn instance_corpse_postgres_native_scope_mint_audit_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = run_scenario(Case::InstanceCorpse);
    eprintln!(
        "INSTANCE_CORPSE POSTGRES_CASE_DURATION elapsed_ms={} success={}",
        started.elapsed().as_millis(),
        result.is_ok()
    );
    result
}

#[test]
fn boss_activity_postgres_production_reward_boss_instance_corpse_and_personal_custody() -> TestResult
{
    run_scenario(Case::ProductionRewards)
}

#[test]
fn boss_activity_postgres_late_native_death_drains_and_returns_without_rewards() -> TestResult {
    run_scenario(Case::LateRewards)
}

#[test]
fn boss_activity_postgres_nonreward_native_death_replays_and_returns_after_kill() -> TestResult {
    run_scenario(Case::NonRewardDefeat)
}

#[test]
fn boss_activity_postgres_authenticated_recovery_keeps_exact_native_state() -> TestResult {
    run_scenario(Case::Recovery)
}

#[test]
fn boss_activity_postgres_cancelled_committed_recovery_returns_and_releases() -> TestResult {
    run_scenario(Case::RecoveryCancelled)
}

#[test]
fn boss_activity_postgres_recovery_protection_survives_entry_loss_and_return() -> TestResult {
    run_scenario(Case::RecoveryProtection)
}

#[test]
fn boss_activity_postgres_capability_mismatch_returns_before_terminal_release() -> TestResult {
    run_scenario(Case::CapabilityMismatch)
}

#[test]
fn boss_activity_postgres_offline_native_death_returns_before_mismatch_release() -> TestResult {
    run_scenario(Case::OfflineDeathMismatch)
}

#[test]
fn boss_activity_postgres_offline_native_death_deadline_retry_keeps_retained_trigger() -> TestResult
{
    run_scenario(Case::OfflineDeathDeadline)
}

#[test]
fn l3_endgame_quest_entry_soul_war_child_requires_level() -> TestResult {
    run_scenario(Case::SoulWarEntry)
}

#[test]
fn l3_endgame_quest_entry_soul_war_final_requires_level() -> TestResult {
    run_scenario(Case::SoulWarFinalEntry)
}

#[test]
fn l3_endgame_quest_entry_rotten_blood_requires_level() -> TestResult {
    run_scenario(Case::RottenBloodEntry)
}

#[test]
fn l3_instance_endgame_soul_war_native_death_progresses_once() -> TestResult {
    run_scenario(Case::SoulWarDefeat)
}

#[test]
fn l3_instance_endgame_soul_war_final_native_death_progresses_once() -> TestResult {
    run_scenario(Case::SoulWarFinalDefeat)
}

#[test]
fn l3_instance_endgame_rotten_blood_native_death_progresses_once() -> TestResult {
    run_scenario(Case::RottenBloodDefeat)
}

#[test]
fn l3_instance_endgame_late_soul_war_death_cannot_earn_taint() -> TestResult {
    run_scenario(Case::SoulWarLateDefeat)
}

#[test]
fn l3_instance_endgame_primal_native_death_unlocks_hazard_once() -> TestResult {
    run_scenario(Case::PrimalDefeat)
}

fn run_scenario(case: Case) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("BOSS_ACTIVITY POSTGRES_CASE_SKIPPED: no admin URL");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "boss activity fixture panicked")??;
    Ok(())
}

#[test]
fn endgame_progression_fixture_keeps_corpse_without_personal_loot() -> TestResult {
    // Pure content activation: this regression test never opens PostgreSQL.
    super::kill_reward_e2e::scenario_thread::run("endgame-progression-content", || {
        let world = crate::foundation::WorldId::decode(&fixture::id(42)).map_err(debug)?;
        let (controller, _) = endgame_progression_generation(world)?;
        let native = controller
            .active()
            .and_then(|active| active.native_gameplay())
            .ok_or("native fixture")?;
        let creature = "oteryn:creature.goshnar_s_spite";
        assert!(matches!(
            native.reward_table().row(creature),
            Err(crate::content::creature_reward::NoSettlementReason::LootTableMissing)
        ));
        assert!(native.reward_table().reward_boss_corpse(creature).is_some());
        assert!(
            native
                .reward_table()
                .row("oteryn:creature.the_primal_menace")
                .is_ok()
        );
        Ok(())
    })
}

// Keep this negative progression fixture without personal-loot admission even
// after the production source independently admits Goshnar's Spite. Retain the
// exact native policy, XP and corpse; only its private-loot table is absent.
fn endgame_progression_generation(
    world: crate::foundation::WorldId,
) -> TestResult<super::quest_catalogue_chests::CatalogueActivation> {
    use sha2::{Digest, Sha256};
    let mut input = super::death_ml_loss_composed::production_input_with_source("content-1", None)?;
    let section = input
        .loot_tables
        .as_mut()
        .ok_or("production loot section")?;
    let mut loot: serde_json::Value = serde_json::from_slice(&section.bytes)?;
    let bindings = loot["creature_loot"].as_array().ok_or("creature loot")?;
    let excluded = bindings
        .iter()
        .find(|row| row["creature"]["key"] == "oteryn:creature.goshnar_s_spite")
        .ok_or("Spite source binding")?["loot"]
        .clone();
    assert!(
        excluded.is_object(),
        "production personal loot is independently admitted"
    );
    assert_eq!(
        bindings
            .iter()
            .filter(|row| row["loot"] == excluded)
            .count(),
        1,
        "exclude only Spite's private loot"
    );
    let tables = loot["tables"].as_array_mut().ok_or("loot tables")?;
    let before = tables.len();
    tables.retain(|table| table["identity"] != excluded);
    assert_eq!(tables.len() + 1, before, "one exact private table removed");
    section.bytes = serde_json::to_vec(&loot)?;
    section.sha256 = Sha256::digest(&section.bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    crate::content::native_gameplay::test_support::try_activate_input(world, &input).map_err(debug)
}

fn scenario(case: Case) -> TestResult {
    run(async |admin| {
        let started = std::time::Instant::now();
        let with_live_progression = matches!(case, Case::AwardContinuity | Case::EndgameReceipts);
        let with_rewards = matches!(
            case,
            Case::Rewards
                | Case::ProductionRewards
                | Case::InstanceCorpse
                | Case::LateRewards
                | Case::AwardContinuity
                | Case::EndgameReceipts
        );
        let endgame_death = matches!(
            case,
            Case::SoulWarDefeat
                | Case::SoulWarLateDefeat
                | Case::SoulWarFinalDefeat
                | Case::RottenBloodDefeat
                | Case::PrimalDefeat
        );
        let late = matches!(case, Case::LateRewards | Case::SoulWarLateDefeat);
        let with_death = matches!(
            case,
            Case::OfflineDeathMismatch | Case::OfflineDeathDeadline
        );
        reset();
        // Real progression cases let the current Character admission owner
        // initialize its pinned policy instead of seeding the harness XP table.
        let initialized = !with_death && !with_live_progression && !endgame_death;
        let h =
            Harness::create_with_admission_seed(admin, "native_activity_wire", initialized, false)
                .await?;
        let evidence = super::house_transfer::RecoverySource::start(&h).await?;
        super::house_transfer::prepare_owned_session(&h).await?;
        let world = fixture::fence(1)?.runtime_scope.world_id();
        let (controller, source_pin) = if with_live_progression {
            state_effects::generation(world, "content-1")?
        } else if case == Case::ProductionRewards {
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None)
        } else if with_rewards {
            crate::content::native_gameplay::test_support::activated_reward_boss_fixture(
                world, None,
            )
        } else if endgame_death {
            // Like the existing composed-death fixture, bind unchanged native
            // training bytes to the fixture Character's content-1 interpretation
            // before activation. Production pairs both under s3b-content-1.
            endgame_progression_generation(world)?
        } else if with_death {
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?
        } else {
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None)
        };
        let source_room = source_pin.qualified_room().clone();
        let pin = source_pin.into_channel_pin();
        let content_pin = pin.clone();
        let deps = Dependencies::with_content(
            pin,
            source_room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.evidence = &evidence.evidence;
        owner.active_generation = controller.active();
        owner.rooms = SeamRooms::new(Some(&deps.room), Some(&deps.room));
        let endgame_creature = match case {
            Case::SoulWarEntry | Case::SoulWarDefeat | Case::SoulWarLateDefeat => {
                Some("oteryn:creature.goshnar_s_spite")
            }
            Case::SoulWarFinalEntry => Some("oteryn:creature.goshnar_s_megalomania_purple"),
            Case::PrimalDefeat => Some("oteryn:creature.the_primal_menace"),
            Case::SoulWarFinalDefeat => Some("oteryn:creature.goshnar_s_megalomania_green"),
            Case::RottenBloodEntry | Case::RottenBloodDefeat => Some("oteryn:creature.bakragore"),
            _ => None,
        };
        let (mut boss_key, policy) = if let Some(key) = endgame_creature {
            let policy = owner
                .active_generation
                .ok_or("generation")?
                .native_gameplay()
                .ok_or("native gameplay")?
                .policies()
                .policies()
                .iter()
                .find(|policy| policy.definition_key == key)
                .ok_or("source endgame creature")?;
            // The catalogue key is an alias: the durable owner must gate by the
            // native creature identity as well as the public command's key.
            (BOSS.to_owned(), policy.as_ref().clone())
        } else if with_rewards {
            rewards::source_boss(&owner, case == Case::ProductionRewards)?
        } else if case == Case::NonRewardDefeat {
            nonreward::source_creature(&owner)?
        } else {
            (BOSS.to_owned(), fixture_policy())
        };
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = item_fence_of(
            FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(admitted.game_session_id)
                .await?
                .0,
        );
        let requested = if case == Case::CapabilityMismatch || with_death {
            vec![
                wire::CAPABILITY_BOSS_ENCOUNTER_V1,
                oteryn_protocol_oteryn::boss_difficulty::CAPABILITY_BOSS_DIFFICULTY_V1,
            ]
        } else {
            vec![wire::CAPABILITY_BOSS_ENCOUNTER_V1]
        };
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &requested)
                .ok_or("boss capability missing")?;
        let fence = admitted.item_fence.ok_or("source fence")?;
        if with_rewards || endgame_death {
            super::item_look::seed_main_backpack(&h, &[]).await?;
            sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT(character_id) DO NOTHING")
                .bind(fence.character_id.as_bytes().as_slice()).execute(&h.pool).await?;
        }
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    admitted.game_session_id,
                    CharacterLease::new(
                        crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                            .map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
        }
        if with_death {
            offline_death::prepare_source(&h, &mut owner, &mut admitted).await?;
        }
        if endgame_death {
            owner.admit_progression_session(&mut admitted).await;
            assert!(
                owner
                    .progression_binding(admitted.game_session_id)
                    .is_some()
            );
            deps.initialize(&owner).await;
        } else if with_live_progression {
            state_effects::prepare_source(&h, &mut owner, &mut admitted).await?;
        } else {
            deps.initialize(&owner).await;
        }
        owner
            .register_current_runtime()
            .await
            .map_err(|_| "register source owner")?;
        owner
            .activity_runtimes
            .remember(admitted)
            .map_err(|_| "remember source")?;
        let inherited_protection = if case == Case::RecoveryProtection {
            Some(recovery::prepare_source_reentry(&h, &owner, &mut admitted).await?)
        } else {
            None
        };
        // The protection case performs a real Channel recovery first. Every
        // subsequent request must use its actual fresh connection generation.
        let fence = admitted.item_fence.ok_or("current source fence")?;
        let before_state = {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let state = states
                .get(&runtime, deps.actor, admitted.game_session_id)
                .ok_or("source live state")?;
            (state.revision(), state.vitals(), state.saved_vitals())
        };
        let source_cell = owner
            .runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position();

        issue_content_and_reset(&h, &content_pin).await?;
        let instance = super::activity_runtime::allocate_ready_instance(&h, 88).await?;
        let assignment = h
            .root
            .read_runtime_scope_assignment(instance)
            .await
            .map_err(debug)?
            .ok_or("allocated Instance")?;
        // Two independent test activation invocations produce equal qualified
        // content. Production uses the consumed activity-template activation API.
        let (_instance_controller, instance_pin) = if with_live_progression {
            state_effects::generation(world, "content-1")?
        } else if case == Case::ProductionRewards {
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None)
        } else if with_rewards {
            crate::content::native_gameplay::test_support::activated_reward_boss_fixture(
                world, None,
            )
        } else if endgame_death {
            endgame_progression_generation(world)?
        } else if with_death {
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?
        } else {
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None)
        };
        let instance_room = instance_pin.qualified_room().clone();
        let pin = instance_pin.into_channel_pin();
        assert_eq!(
            pin.server_artifact_digest(),
            content_pin.server_artifact_digest()
        );
        let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
            instance,
            h.node.fact().node_id(),
            h.node.fact().registration_revision(),
            assignment.ownership_generation,
            assignment.source_revision,
            &assignment.decision_identity,
            4,
            pin.clone(),
        )
        .map_err(debug)?;
        let reference =
            crate::world_runtime::ReferenceContentGeneration::from_content(instance_room.door())
                .map_err(debug)?;
        let generation =
            crate::foundation::ScopeOwnershipGeneration::new(assignment.ownership_generation)
                .map_err(debug)?;
        let door_fence = crate::world_runtime::ScopeContentGenerationFence::for_test(
            instance, generation, reference,
        );
        let door = crate::world_runtime::bind_native_entry_door(
            instance_room.door(),
            &door_fence,
            instance,
            generation,
        )
        .map_err(debug)?;
        let start = instance_room.entry_start();
        let entry = MovementLocalPosition {
            x: start.x,
            y: start.y,
            floor: start.floor,
        };
        let cells = instance_room.movement_cells();
        let boss_cell = (-8_i32..=8)
            .flat_map(|dy| (-8_i32..=8).map(move |dx| (dx, dy)))
            .map(|(dx, dy)| MovementLocalPosition {
                x: start.x + dx,
                y: start.y + dy,
                floor: start.floor,
            })
            .find(|cell| {
                *cell != entry
                    && crate::gameplay_transport::respawn_cell_admissible(
                        cells.index(),
                        cells.scope(),
                        world,
                        pin.server_artifact_digest(),
                        door.blocking_cells(),
                        *cell,
                    )
            })
            .ok_or("a second actual qualified walkable cell")?;
        runtime
            .install_companion_policies(
                crate::foundation::CompiledCreaturePolicies::from_active_artifact(
                    pin.server_artifact_digest(),
                    vec![policy.clone()],
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        runtime
            .realize_native_qualification_spawn(pin, &policy.definition_key, boss_cell)
            .map_err(debug)?;
        sqlx::query("INSERT INTO game_boss_room_catalogue(boss_key,content_revision,boss_creature_key,minimum_players,maximum_players,minimum_level,cooldown_seconds,time_limit_seconds,lever_positions,room_template_key,reward_key,enabled,template_revision,entry_positions,exit_position,boss_position,after_kill_seconds) VALUES($1,'content-1',$2,1,1,1,20,$8,$3,$4,'fixture:reward',true,'fixture-r1',$5,$6,$7,1)")
            .bind(&boss_key).bind(&policy.definition_key).bind(serde_json::json!([cell_json(source_cell)]))
            .bind(TEMPLATE).bind(serde_json::json!([cell_json(entry)]))
            .bind(cell_json(source_cell)).bind(cell_json(boss_cell)).bind(if late || matches!(case, Case::Entry | Case::OfflineDeathDeadline) { 1_i32 } else if case == Case::NonRewardDefeat { 5 } else { 900 }).execute(&h.pool).await?;

        let (_, refused, switch) = exchange(&owner, &mut admitted, &boss_key, false).await?;
        assert_eq!(
            refused,
            BossActivityReply::Refused(BossActivityRefusal::ContentUnavailable)
        );
        assert!(switch.is_none());
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            source_cell
        );
        owner
            .register_activity_runtime(
                ActivityTemplateKey {
                    key: TEMPLATE.into(),
                    revision: "fixture-r1".into(),
                },
                &assignment,
                runtime,
                door,
                ActivityRuntimeAssets {
                    movement_cells: cells.into(),
                    rooms: SeamRooms::new(Some(&instance_room), Some(&instance_room)),
                },
            )
            .await
            .map_err(|_| "register genuine destination owner")?;

        // Prove fixture readiness independently of the typed wire result. An
        // unavailable catalogue or unready allocation must not be mistaken for
        // the expected pre-registration ContentUnavailable negative case above.
        let handoffs =
            crate::durability::activity_handoff::ActivityHandoffOwner::new(h.root.clone());
        let definition = handoffs
            .read_boss_room(
                &authority,
                &h.node,
                admitted.item_fence.ok_or("current source fence")?,
                &boss_key,
            )
            .await?
            .ok_or("authored fixture catalogue is visible to the current source fence")?;
        let allocations = handoffs
            .ready_allocations(&authority, &h.node, world, None)
            .await?;
        assert!(
            allocations
                .iter()
                .any(|allocation| allocation.scope == instance
                    && allocation.generation.get() == assignment.ownership_generation
                    && allocation.source_revision == assignment.source_revision),
            "allocated Instance remains independently ready: {allocations:?}"
        );
        let registered = owner
            .activity_runtimes
            .get(instance, assignment.ownership_generation)
            .ok_or("registered Instance")?;
        {
            let runtime = registered.runtime.lock().await;
            assert!(
                crate::gameplay_transport::native_activity::boss_room_native_ready(
                    &runtime,
                    &definition
                ),
                "the fresh room contains its exact authored living boss before entry"
            );
        }
        if let Some(key) = endgame_creature {
            let (_, reply, barrier) = exchange(&owner, &mut admitted, &boss_key, false).await?;
            assert_eq!(
                reply,
                BossActivityReply::Refused(BossActivityRefusal::Level)
            );
            assert!(barrier.is_none());
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_activity_handoff_groups")
                    .fetch_one(&h.pool)
                    .await?,
                0
            );
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_boss_cooldowns")
                    .fetch_one(&h.pool)
                    .await?,
                0
            );
            assert_eq!(
                owner
                    .runtime
                    .lock()
                    .await
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                source_cell
            );
            if endgame_death {
                endgame::prepare_level(
                    &owner,
                    admitted,
                    if case == Case::PrimalDefeat { 500 } else { 250 },
                )
                .await?;
            } else {
                // Imported snapshots exercise admission only, not quest completion.
                let mut seed = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica")
                    .execute(&mut *seed)
                    .await?;
                sqlx::query("UPDATE game_character_roots SET character_revision=character_revision+1 WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).execute(&mut *seed).await?;
                sqlx::query("UPDATE game_character_progression_state p SET level=250,total_experience=254237300,character_revision=r.character_revision FROM game_character_roots r WHERE p.character_id=r.character_id AND p.character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).execute(&mut *seed).await?;
                seed.commit().await?;
            }
            if case != Case::PrimalDefeat {
                let (_, reply, barrier) = exchange(&owner, &mut admitted, &boss_key, false).await?;
                assert_eq!(
                    reply,
                    BossActivityReply::Refused(BossActivityRefusal::AuthorityRejected)
                );
                assert!(barrier.is_none());

                let mut seed = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica")
                    .execute(&mut *seed)
                    .await?;
                if matches!(case, Case::RottenBloodEntry | Case::RottenBloodDefeat) {
                    sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,'oteryn:quest/rotten-blood',4,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(oteryn_gameplay_quests::quest::rotten_blood::ACCESS_TRACK)
                    .execute(&mut *seed).await?;
                } else {
                    sqlx::query("INSERT INTO game_character_soul_war(character_id,teleport_access) VALUES(encode($1,'hex')::uuid,true)")
                    .bind(fence.character_id.as_bytes().as_slice()).execute(&mut *seed).await?;
                }
                seed.commit().await?;
                if !matches!(
                    case,
                    Case::SoulWarEntry | Case::SoulWarDefeat | Case::SoulWarLateDefeat
                ) {
                    let (_, reply, barrier) =
                        exchange(&owner, &mut admitted, &boss_key, false).await?;
                    assert_eq!(
                        reply,
                        BossActivityReply::Refused(BossActivityRefusal::AuthorityRejected)
                    );
                    assert!(barrier.is_none());
                    let mut seed = h.pool.begin().await?;
                    sqlx::query("SET LOCAL session_replication_role=replica")
                        .execute(&mut *seed)
                        .await?;
                    if matches!(case, Case::RottenBloodEntry | Case::RottenBloodDefeat) {
                        sqlx::query("INSERT INTO game_character_rotten_blood(character_id,child_flags) VALUES(encode($1,'hex')::uuid,15)")
                        .bind(fence.character_id.as_bytes().as_slice()).execute(&mut *seed).await?;
                    } else {
                        sqlx::query("UPDATE game_character_soul_war SET defeated_bosses=31,first_taint_at=floor(extract(epoch FROM transaction_timestamp()))::bigint-1209601 WHERE character_id=encode($1,'hex')::uuid")
                        .bind(fence.character_id.as_bytes().as_slice()).execute(&mut *seed).await?;
                    }
                    seed.commit().await?;
                    if matches!(case, Case::SoulWarFinalEntry | Case::SoulWarFinalDefeat) {
                        let (_, reply, barrier) =
                            exchange(&owner, &mut admitted, &boss_key, false).await?;
                        assert_eq!(
                            reply,
                            BossActivityReply::Refused(BossActivityRefusal::AuthorityRejected)
                        );
                        assert!(
                            barrier.is_none(),
                            "expired taints cannot admit the final boss"
                        );
                        let mut seed = h.pool.begin().await?;
                        sqlx::query("SET LOCAL session_replication_role=replica")
                            .execute(&mut *seed)
                            .await?;
                        sqlx::query("UPDATE game_character_soul_war SET first_taint_at=floor(extract(epoch FROM transaction_timestamp()))::bigint WHERE character_id=encode($1,'hex')::uuid")
                        .bind(fence.character_id.as_bytes().as_slice()).execute(&mut *seed).await?;
                        seed.commit().await?;
                    }
                }
            }
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_activity_handoff_groups")
                    .fetch_one(&h.pool)
                    .await?,
                0
            );
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_boss_cooldowns")
                    .fetch_one(&h.pool)
                    .await?,
                0
            );
            // Valid source predicates must cross the real W02 scope barrier
            // through the canonical command key, not a preview-only success.
            let canonical = crate::gameplay_transport::endgame_entry::canonical_boss_key(key)
                .ok_or("captured endgame boss identity")?;
            sqlx::query("UPDATE game_boss_room_catalogue SET boss_key=$2 WHERE boss_key=$1")
                .bind(&boss_key)
                .bind(&canonical)
                .execute(&h.pool)
                .await?;
            boss_key = canonical;
        }
        let attack_continuity = if case == Case::Entry {
            Some(state_continuity::prepare_attack(&owner, admitted).await?)
        } else {
            None
        };
        let old = admitted;
        let (command, reply, barrier) = exchange(&owner, &mut admitted, &boss_key, true).await?;
        let BossActivityReply::Accepted {
            handoff,
            deadline_unix_ms: Some(_),
        } = reply
        else {
            return Err(format!("expected real entry: {reply:?}").into());
        };
        let barrier = barrier.ok_or("scope barrier missing")?;
        assert_eq!(barrier.handoff, handoff);
        assert_eq!(barrier.source_session, *old.game_session_id.as_bytes());
        assert_eq!(barrier.session, *admitted.game_session_id.as_bytes());
        assert_ne!(old.game_session_id, admitted.game_session_id);
        let RuntimeScopeRefV1::Instance {
            world_id,
            instance_id,
        } = instance
        else {
            return Err("Instance expected".into());
        };
        assert_eq!(
            barrier.scope,
            ActivityScope::Instance {
                world: *world_id.as_bytes(),
                instance: instance_id
            }
        );
        assert_eq!(barrier.connection_generation, 1);
        assert_eq!(barrier.scope_generation, assignment.ownership_generation);
        assert_eq!(
            barrier.lease_generation,
            fence.character_lease_generation + 1
        );
        assert_eq!(
            barrier.selection.as_slice(),
            old.continuity.selected_capabilities.as_slice()
        );
        assert_ne!(
            admitted
                .controller
                .ok_or("destination controller")?
                .transport,
            old.controller.ok_or("source controller")?.transport
        );
        assert!(
            owner
                .runtime
                .lock()
                .await
                .player_control_facts(deps.actor, old.game_session_id)
                .is_err()
        );
        let destination = owner
            .resolve_runtime_view(admitted)
            .await
            .map_err(|_| "resolve Instance owner")?
            .ok_or("owner actually changed")?;
        let next_actor = admitted.runtime_actor.ok_or("destination actor")?;
        assert!(
            destination
                .refresh_pvp_locks(next_actor, admitted.game_session_id)
                .await,
            "activity Instance retains the ordinary PvP reader, without House custody"
        );
        {
            let runtime = destination.runtime.lock().await;
            assert_eq!(runtime.binding().runtime_scope(), instance);
            assert_eq!(
                runtime
                    .read_actor_position(next_actor)
                    .map_err(debug)?
                    .position(),
                entry
            );
            let states = destination.states.spell_states.lock().await;
            let state = states
                .get(&runtime, next_actor, admitted.game_session_id)
                .ok_or("moved live state")?;
            assert_eq!(
                (state.revision(), state.vitals(), state.saved_vitals()),
                before_state
            );
        }
        let store = FreshAdmissionStore::from_root(h.root.clone());
        assert_eq!(
            store
                .current_session_at(old.game_session_id)
                .await?
                .0
                .session_state(),
            crate::foundation::GameSessionState::Terminal
        );
        assert_eq!(
            item_fence_of(store.current_session_at(admitted.game_session_id).await?.0),
            admitted.item_fence
        );
        let replay = owner
            .retry_native_activity(old, command, &boss_key)
            .await
            .map_err(|_| "same attempt replay")?
            .ok_or("retained attempt")?;
        let crate::gameplay_transport::native_activity::NativeActivityOutcome::Committed(replayed) =
            replay
        else {
            return Err("replay refused".into());
        };
        assert_eq!(replayed.handoff_id, handoff);
        owner
            .publish_activity_commit(&replayed)
            .map_err(|_| "idempotent publication")?;
        assert!(
            owner
                .activity_runtimes
                .pending_switch(old)
                .map_err(|_| "pending read")?
                .is_none(),
            "replay must not resurrect a delivered barrier"
        );
        let counts:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_activity_handoff_groups),(SELECT count(*) FROM game_boss_room_activities)").fetch_one(&h.pool).await?;
        assert_eq!(counts, (1, 1));
        let mut forged = admitted;
        forged
            .item_fence
            .as_mut()
            .ok_or("fence")?
            .scope_ownership_generation =
            crate::foundation::ScopeOwnershipGeneration::new(assignment.ownership_generation + 1)
                .map_err(debug)?;
        assert!(owner.resolve_runtime_view(forged).await.is_err());
        if matches!(
            case,
            Case::Rewards | Case::ProductionRewards | Case::InstanceCorpse | Case::LateRewards
        ) {
            rewards::settle_and_collect(
                &h,
                &owner,
                &destination,
                admitted,
                &policy,
                late,
                case == Case::InstanceCorpse,
            )
            .await?;
            if late {
                eprintln!("BOSS_ACTIVITY_LATE_REWARD_TIMEOUT POSTGRES_CASE_COMPLETED");
            } else {
                eprintln!("BOSS_ACTIVITY_INSTANCE_REWARD_CUSTODY POSTGRES_CASE_COMPLETED");
            }
        }
        match case {
            Case::EndgameReceipts => {
                endgame_receipts::assert_receipts(&h, &destination, admitted).await?
            }
            Case::Entry => {
                state_continuity::assert_entry_and_timeout_return(
                    &h,
                    &owner,
                    &destination,
                    &mut admitted,
                    &boss_key,
                    attack_continuity.ok_or("source attack continuity")?,
                )
                .await?
            }
            Case::AwardContinuity => {
                state_effects::assert_award_follows_return(
                    &h,
                    &owner,
                    &destination,
                    admitted,
                    &policy,
                )
                .await?
            }
            Case::NonRewardDefeat => {
                nonreward::assert_defeat_and_return(&h, &owner, &destination, admitted, &policy)
                    .await?
            }
            Case::Recovery => recovery::assert_recovery(&h, &owner, old, admitted).await?,
            Case::RecoveryCancelled => {
                recovery::assert_cancelled_recovery(&h, &owner, old, admitted).await?
            }
            Case::RecoveryProtection => {
                recovery::assert_inherited_protection(
                    &h,
                    &owner,
                    admitted,
                    inherited_protection.ok_or("source protection")?,
                )
                .await?
            }
            Case::CapabilityMismatch => {
                recovery::assert_capability_mismatch(&h, &owner, admitted).await?
            }
            Case::OfflineDeathMismatch | Case::OfflineDeathDeadline => {
                offline_death::assert_offline_death(
                    &h,
                    &owner,
                    &destination,
                    admitted,
                    case == Case::OfflineDeathDeadline,
                )
                .await?
            }
            _ => {}
        }
        if endgame_death {
            endgame::settle(&h, &destination, admitted, &policy, case).await?;
        }
        if let Some(key) = endgame_creature {
            eprintln!(
                "L3_ENDGAME_QUEST_ENTRY POSTGRES_CASE_COMPLETED {key} elapsed_ms={}",
                started.elapsed().as_millis()
            );
        }
        eprintln!("BOSS_ACTIVITY_NATIVE_ENTRY POSTGRES_CASE_COMPLETED");
        eprintln!("BOSS_ACTIVITY_SCOPE_BARRIER POSTGRES_CASE_COMPLETED");
        eprintln!("BOSS_ACTIVITY_STATE_AND_REPLAY POSTGRES_CASE_COMPLETED");
        Ok(())
    })
}

fn cell_json(cell: MovementLocalPosition) -> serde_json::Value {
    serde_json::json!({"x":cell.x,"y":cell.y,"floor":cell.floor})
}

fn fixture_policy() -> crate::foundation::CompiledCreaturePolicy {
    crate::foundation::CompiledCreaturePolicy {
        definition_key: CREATURE.into(),
        definition_revision: "fixture-r1".into(),
        display_name: "Activity fixture".into(),
        maximum_health: 37,
        forge_difficulty: None,
        base_speed: 110,
        outfit_look_type: 21,
        object_look_type: None,
        summonable: false,
        convinceable: false,
        mana_cost: None,
        is_familiar: false,
        condition_immunities: vec![],
        defense: Some(1),
        armor: Some(1),
        mitigation: None,
        resistances: vec![],
        damage_immunities: vec![],
        healing_from_damage: vec![],
        preferred_distance: Some(1),
        reward_boss: Some(true),
        blocked_by_nearby_players: false,
        flags: crate::foundation::CreatureFlags {
            attackable: true,
            illusionable: false,
            health_hidden: false,
        },
    }
}

pub(super) async fn issue_content_and_reset(
    h: &Harness,
    pin: &oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
) -> TestResult {
    issue_content_and_reset_with_runtime_pin(h, pin, pin, "activity-fixture").await
}

pub(super) async fn issue_content_and_reset_with_runtime_pin(
    h: &Harness,
    pin: &oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
    runtime_pin: &oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
    runtime_map_revision: &str,
) -> TestResult {
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fixture::fence(1)?.runtime_scope
    else {
        return Err("fixture Channel".into());
    };
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .bind(pin.server_artifact_digest().as_slice()).bind(pin.client_artifact_digest().as_slice())
        .bind(pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
    // Initial reset fixture follows the existing kill-reward PG harness. Runtime
    // entry never writes a control grant, activation or world-reset row.
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,$5,$2,$3,$4,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp(),'2026-10')")
        .bind(world_id.as_bytes().as_slice()).bind(runtime_pin.server_artifact_digest().as_slice())
        .bind(runtime_pin.client_artifact_digest().as_slice()).bind(runtime_pin.frame_binding_digest().as_slice()).bind(runtime_map_revision).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
        .bind(world_id.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    Ok(())
}

pub(super) async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    boss: &str,
    expect_switch: bool,
) -> TestResult<(u64, BossActivityReply, Option<ActivityScopeSnapshot>)> {
    exchange_command(
        owner,
        admitted,
        boss,
        expect_switch,
        wire::COMMAND_TYPE_BOSS_LEVER,
    )
    .await
}

pub(super) async fn exchange_command(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    boss: &str,
    expect_switch: bool,
    command_type: u32,
) -> TestResult<(u64, BossActivityReply, Option<ActivityScopeSnapshot>)> {
    let before = *admitted;
    let id = before.continuity.next_command_id;
    let body = wire::encode_boss_key(boss).map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(16384);
    let serving = async {
        let end = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let mut stage = "source initial snapshot";
    let mut frames = std::collections::VecDeque::with_capacity(16);
    let driving = async {
        loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if frames.len() == 16 {
                frames.pop_front();
            }
            frames.push_back(format!(
                "{stage}: {:?}, sequence {}",
                envelope.message_type(),
                envelope.server_sequence()
            ));
            if envelope.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        stage = "write boss command";
        write_frame(
            &mut client,
            &encode_client_command(
                before.continuity.connection_generation,
                &ClientCommandValue {
                    command_id: id,
                    command_type,
                    payload: &body,
                },
            )
            .map_err(debug)?,
        )
        .await?;
        stage = "command result";
        let mut reply = None;
        let mut barrier = None;
        let mut commits_after_barrier = 0;
        loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if frames.len() == 16 {
                frames.pop_front();
            }
            frames.push_back(format!(
                "{stage}: {:?}, sequence {}",
                envelope.message_type(),
                envelope.server_sequence()
            ));
            match envelope.message_type() {
                MessageType::CommandResult => {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(result.command_id, id);
                    let decoded = wire::decode_boss_activity_reply(result.payload);
                    assert_eq!(
                        result.status,
                        if expect_switch {
                            CommandStatus::Accepted
                        } else {
                            CommandStatus::Rejected
                        },
                        "boss command {id}, session {:?}, generation {}: {decoded:?}; raw payload {:?}",
                        before.game_session_id,
                        before.continuity.connection_generation,
                        result.payload
                    );
                    reply = Some(decoded.map_err(debug)?);
                    if !expect_switch {
                        break;
                    }
                    stage = "source scope barrier";
                }
                MessageType::SnapshotChunk => {
                    let (_, domains) = decode_snapshot_chunk(envelope.payload()).map_err(debug)?;
                    for domain in domains {
                        if domain.domain_id == STATE_DOMAIN_ACTIVITY_SCOPE {
                            assert!(reply.is_some(), "initiator result precedes scope barrier");
                            let projected = decode_activity_scope(domain.payload).map_err(debug)?;
                            if let Some(prior) = barrier.as_ref() {
                                // The new owner's full initial snapshot includes
                                // its binding too. It is the same projection, not
                                // another source-stream switch or reset.
                                assert!(commits_after_barrier >= 1, "one source-stream barrier");
                                assert_eq!(
                                    prior, &projected,
                                    "initial scope projection agrees with committed barrier"
                                );
                            } else {
                                barrier = Some(projected);
                                stage = "source scope barrier commit";
                            }
                        }
                    }
                }
                MessageType::SnapshotCommit if barrier.is_some() => {
                    commits_after_barrier += 1;
                    if commits_after_barrier == 2 {
                        break;
                    }
                    stage = "destination initial snapshot";
                }
                _ => {}
            }
        }
        stage = "client shutdown";
        client.shutdown().await?;
        stage = "complete";
        Ok((id, reply.ok_or("command result")?, barrier))
    };
    let (ended, result): (_, TestResult<_>) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        fixture::join_two(serving, driving),
    )
    .await?;
    if let ConnectionEnd::AdmittedThenDisconnected(next)
    | ConnectionEnd::AdmittedThenClosed(next, _) = ended
    {
        *admitted = next;
    }
    result.map_err(|error| {
        format!(
            "boss wire command {id}, expect_switch={expect_switch}, stage={stage}, source={before:?}, ended={ended:?}, frames={frames:?}: {error}"
        )
        .into()
    })
}
