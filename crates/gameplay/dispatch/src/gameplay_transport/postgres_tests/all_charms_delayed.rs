//! Current assignments, qualified Leech and joined children through real timer callbacks.
use super::death_ml_loss_composed::{current_fence, production_generation_with_source};
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};
use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceRewardOccurrence,
};
use crate::foundation::{ChannelContentPin, CharacterLease, GameSessionId};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::gameplay_transport::spell_character_facts::{CastFactsLoad, load_character_cast_facts};
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_protocol_oteryn::actor_spell;

#[test]
fn all_charms_delayed_native_callback_cripple() -> TestResult {
    scenario(true, "cripple")
}
#[test]
fn all_charms_delayed_chain_callback_cripple() -> TestResult {
    scenario(false, "cripple")
}
#[test]
fn all_charms_delayed_native_callback_leech() -> TestResult {
    scenario(true, "leech")
}
#[test]
fn all_charms_delayed_chain_callback_leech() -> TestResult {
    scenario(false, "leech")
}
#[test]
fn all_charms_delayed_native_callback_damage_procs() -> TestResult {
    scenario(true, "batch")
}
#[test]
fn all_charms_delayed_chain_callback_damage_procs() -> TestResult {
    scenario(false, "batch")
}
#[test]
fn all_charms_delayed_native_callback_carnage() -> TestResult {
    scenario(true, "carnage")
}
#[test]
fn all_charms_delayed_chain_callback_carnage() -> TestResult {
    scenario(false, "carnage")
}
#[test]
fn all_charms_delayed_chain_callback_carnage_after_retired_holder() -> TestResult {
    retired_scenario(false, "carnage", RECEIPT_WRITE)
}
#[test]
fn all_charms_delayed_chain_callback_zero_after_cancelled_tile_read() -> TestResult {
    retired_scenario(false, "zero", TILE_READ)
}
#[test]
fn all_charms_delayed_native_callback_fatal_hold() -> TestResult {
    scenario(true, "fatal_hold")
}
#[test]
fn all_charms_delayed_chain_callback_fatal_hold() -> TestResult {
    scenario(false, "fatal_hold")
}
#[test]
fn all_charms_delayed_native_callback_wrong_species() -> TestResult {
    scenario(true, "wrong_species")
}
#[test]
fn all_charms_delayed_chain_callback_wrong_species() -> TestResult {
    scenario(false, "wrong_species")
}
#[test]
fn all_charms_delayed_native_callback_immunity() -> TestResult {
    scenario(true, "immune")
}
#[test]
fn all_charms_delayed_chain_callback_immunity() -> TestResult {
    scenario(false, "immune")
}
#[test]
fn all_charms_delayed_native_callback_lethal_proc() -> TestResult {
    scenario(true, "overpower_lethal")
}
#[test]
fn all_charms_delayed_native_callback_assigned_critical() -> TestResult {
    scenario(true, "critical")
}
#[test]
fn all_charms_delayed_chain_callback_assigned_critical() -> TestResult {
    scenario(false, "critical")
}
#[test]
fn all_charms_delayed_native_callback_cripple_immunity() -> TestResult {
    scenario(true, "cripple_immune")
}
#[test]
fn all_charms_delayed_chain_callback_cripple_immunity() -> TestResult {
    scenario(false, "cripple_immune")
}
#[test]
fn all_charms_delayed_native_callback_zero() -> TestResult {
    scenario(true, "zero")
}
#[test]
fn all_charms_delayed_chain_callback_zero() -> TestResult {
    scenario(false, "zero")
}
#[test]
fn all_charms_delayed_native_callback_leech_minors() -> TestResult {
    scenario(true, "leech_minors")
}
#[test]
fn all_charms_delayed_chain_callback_leech_minors() -> TestResult {
    scenario(false, "leech_minors")
}
fn scenario(delayed: bool, charm: &'static str) -> TestResult {
    scenario_retiring(delayed, charm, None)
}

/// How the forced retire reaches the cast pass: the relation the blocker locks, and whether
/// the waiting holder's statement is cancelled (as `transaction_timeout` does on a slow runner)
/// instead of outliving the deadline.
#[derive(Clone, Copy)]
struct Retire {
    lock: &'static str,
    cancel_waiter_on: Option<&'static str>,
}
/// Only the cast pass writes a spell item receipt; reads stay open to earlier passes.
const RECEIPT_WRITE: Retire = Retire {
    lock: "LOCK TABLE game_spell_item_receipts IN EXCLUSIVE MODE",
    cancel_waiter_on: None,
};
/// The cast prepare reads its tiles from the ground locations; that read fails mid-prepare.
const TILE_READ: Retire = Retire {
    lock: "LOCK TABLE game_item_ground_locations IN ACCESS EXCLUSIVE MODE",
    cancel_waiter_on: Some("game_item_ground_locations"),
};

fn retired_scenario(delayed: bool, charm: &'static str, retire: Retire) -> TestResult {
    scenario_retiring(delayed, charm, Some(retire))
}

fn scenario_retiring(delayed: bool, charm: &'static str, retire: Option<Retire>) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(move || {
            run(async move |admin| {
                let h =
                    Harness::create_with_admission_seed(admin, "all_charms_delayed", false, false)
                        .await?;
                let result = {
                    let mut scenario = std::pin::pin!(scenario_with_harness(&h, delayed, charm, 3, retire));
                    std::future::poll_fn(|cx| {
                        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            std::future::Future::poll(scenario.as_mut(), cx)
                        })) {
                            Ok(std::task::Poll::Pending) => std::task::Poll::Pending,
                            Ok(std::task::Poll::Ready(result)) => {
                                std::task::Poll::Ready(Ok(result))
                            }
                            Err(payload) => std::task::Poll::Ready(Err(payload)),
                        }
                    })
                    .await
                };
                let cleanup = h.cleanup().await;
                match result {
                    Ok(result) => result.and(cleanup),
                    Err(payload) => std::panic::resume_unwind(payload),
                }
            })
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "delayed charm thread panicked")?
        .map_err(Into::into)
}
async fn scenario_with_harness(
    h: &Harness,
    delayed: bool,
    charm: &str,
    stage: u8,
    retire: Option<Retire>,
) -> TestResult {
    let native_path = false;
    let race = match charm {
        "immune" | "cripple_immune" => "oteryn:creature.dragon",
        "zero" if delayed => "oteryn:creature.ghost",
        "zero" => "oteryn:creature.energy_elemental",
        "fatal_hold" => "oteryn:creature.orc_spearman",
        _ => "oteryn:creature.cyclops",
    };
    prepare_current_session(h).await?;
    h.seed_pvp_target().await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) = if charm == "critical" {
        equipment_activation(world, true, 100)?
    } else if charm == "leech_minors" {
        equipment_activation(world, false, 16)?
    } else if matches!(
        charm,
        "leech" | "batch" | "overpower_lethal" | "critical" | "zero" | "leech_minors"
    ) {
        super::item_spell_augments::activate(world, true)?
    } else {
        production_generation_with_source(
            world,
            "content-1",
            Some(include_bytes!(
                "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
            )),
        )
        .map_err(debug)?
    };
    let active = controller.active().ok_or("active")?;
    let native = active.native_gameplay().ok_or("native")?;
    let arena = !delayed || charm == "carnage";
    let room = if !arena {
        pin.qualified_room().clone()
    } else {
        let entry = pin.qualified_room().clone();
        let map = crate::map::spell_tiles::tests::movement_arena::world();
        let cells = map.movement_cells(entry.movement_cells()).map_err(debug)?;
        entry
            .bundle_view(
                cells,
                map.frame_binding_digest(world),
                map.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: 10,
                    y: 10,
                    floor: 7,
                },
            )
            .ok_or("combat arena")?
    };
    let frame_digest = room.frame_binding().digest();
    let start = if arena { (10, 10, 7) } else { (2, 11, -7) };
    let cp = ChannelContentPin::from_activation(
        world,
        1,
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        frame_digest,
        room.map_revision_digest(),
        start,
    );
    let mut deps =
        Dependencies::with_content_and_capacity(cp, room.clone(), native.spell_book().clone(), 16)?;
    deps.combat_room = Some(room);
    let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
    h.root
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
        Some(h.root.clone()),
    );
    let mut owner = deps.owner(h, &authority);
    owner.premium_coordinator = Some(&premium);
    owner.active_generation = Some(active);
    let mut admitted = deps.admitted;
    let actor = deps.actor;
    let session = admitted.game_session_id;
    owner.admit_premium(id(40), session);
    owner.admit_progression_session(&mut admitted).await;
    let progression = owner.progression_binding(session).ok_or("progression")?;
    let formula = native.training_formula().ok_or("training")?;
    let sorcerer = convert_vocation(
        formula,
        &DurableBuildState::default(),
        if native_path { "druid" } else { "sorcerer" },
    )
    .map_err(debug)?;
    h.root
        .commit_character_build(
            &authority,
            &h.node,
            current_fence(&owner, session).await?,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(90)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                before: DurableBuildState::default(),
                after: sorcerer.clone(),
                pruned_stance: None,
            },
            formula,
        )
        .await
        .map_err(debug)?;
    h.root
        .commit_character_experience(
            &authority,
            &h.node,
            current_fence(&owner, session).await?,
            ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(id(91)).map_err(debug)?,
                amount: oteryn_simulation_determinism::ExactI64::new(30_000_000),
                context: progression.context.clone(),
                policy_revision: progression.policy_revision.clone(),
                reward_revision: progression.reward_revision.clone(),
                policy: progression.policy.clone(),
            },
        )
        .await
        .map_err(debug)?;
    let CastFactsLoad::Ready { facts, .. } = load_character_cast_facts(
        &h.root,
        &authority,
        current_fence(&owner, session).await?.character_id,
    )
    .await
    else {
        return Err("cast facts unavailable".into());
    };
    let augment_target;
    {
        let mut runtime = deps.runtime.lock().await;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                session,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        {
            // Keep the source spell nonlethal so Cripple has a living target.
            let p = runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position();
            let position = crate::foundation::MovementLocalPosition {
                x: p.x + 1,
                y: p.y,
                floor: p.floor,
            };
            runtime
                .install_companion_policies(native.policies().clone())
                .map_err(debug)?;
            let health = runtime
                .companion_policy(race)
                .map_err(debug)?
                .maximum_health;
            runtime
                .realize_activation_spawns(vec![(
                    if charm == "carnage" {
                        vec![
                            (position.x, position.y, position.floor),
                            (position.x, position.y + 1, position.floor),
                            (position.x, position.y + 2, position.floor),
                            (position.x - 1, position.y + 1, position.floor),
                        ]
                    } else if native_path {
                        vec![
                            (position.x, position.y, position.floor),
                            (position.x + 1, position.y + 1, position.floor),
                        ]
                    } else {
                        vec![(position.x, position.y, position.floor)]
                    },
                    60_000,
                    5_000,
                    race.into(),
                    u64::try_from(health)?,
                )])
                .map_err(debug)?;
            let target = runtime
                .visible_entities()
                .creatures
                .iter()
                .find(|creature| {
                    runtime
                        .read_actor_position(creature.actor)
                        .is_ok_and(|snapshot| snapshot.position() == position)
                })
                .ok_or("charm target")?
                .actor;
            augment_target = Some((target, position));
        }
        let mut states = owner.states.spell_states.lock().await;
        assert!(
            states
                .initialize(&runtime, actor, session, facts, (0, 0), owner.owner_now())
                .is_some()
        );
        assert!(states.load_owned_build(&runtime, actor, session, &sorcerer));
        states
            .get_mut(&runtime, actor, session)
            .ok_or("caster")?
            .enable_owned_training(&sorcerer, formula)
            .map_err(debug)?;
    }

    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&h.pool).await?;
    if matches!(
        charm,
        "leech" | "batch" | "overpower_lethal" | "critical" | "zero" | "leech_minors"
    ) {
        let mut tx = h.pool.begin().await?;
        // Fixture custody only; all reads and installation use production fences.
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i6529','definition-r1',1,1,encode($1,'hex')::uuid)")
            .bind(id(112).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,4,encode($3,'hex')::uuid,encode($3,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(112).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let mut states = owner.states.spell_states.lock().await;
        crate::gameplay_transport::actor_spell::tests::wound(&mut states, actor, session, 1);
    }
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(frame_digest.as_slice()).execute(&h.pool).await?;
    h.root
        .initialize_familiar_group(&authority, &h.node, current_fence(&owner, session).await?)
        .await
        .map_err(debug)?;
    if matches!(
        charm,
        "leech"
            | "batch"
            | "overpower_lethal"
            | "critical"
            | "zero"
            | "leech_minors"
            | "cripple_refusal"
    ) {
        let _permit = owner.spell_lane_permit().await.ok_or("spell lane")?;
        let effects = owner
            .read_admission_effects(session)
            .await
            .map_err(|_| "equipment effects")?;
        let runtime = deps.runtime.lock().await;
        owner
            .install_admission_suppressions(&runtime, actor, session, effects)
            .await
            .ok_or("install equipment effects")?;
    }
    let (target, position) = augment_target.ok_or("target")?;
    let cases = if charm == "batch" {
        vec![
            "wound",
            "poison",
            "freeze",
            "zap",
            "curse",
            "enflame",
            "divine_wrath",
            "overpower",
            "overflux",
        ]
    } else if charm == "leech_minors" {
        vec![
            "vampiric_embrace",
            "voids_call",
            "voids_wrong_species",
            "leech_overkill",
        ]
    } else if charm == "critical" {
        vec!["savage_blow", "savage_wrong_species"]
    } else {
        vec![charm]
    };
    for (case_index, charm) in cases.into_iter().enumerate() {
        let charm_key = match charm {
            "wrong_species" | "leech" | "zero" => "wound",
            "cripple_immune" => "cripple",
            "immune" => "enflame",
            "savage_wrong_species" => "savage_blow",
            "voids_wrong_species" | "leech_overkill" => "voids_call",
            "overpower_lethal" => "overpower",
            other => other,
        };
        let key = crate::domain::charm::CharmKey::new(format!("oteryn:charm.{charm_key}"))
            .map_err(debug)?;
        let state = crate::durability::charm_state::CombatCharmState::new(
            crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?,
            crate::durability::charm_state::CharacterCharmState {
                unlocks: [(
                    key.clone(),
                    crate::domain::charm::CharmStage::new(stage).map_err(debug)?,
                )]
                .into(),
                assignments: [(
                    key,
                    crate::domain::charm::BestiaryRaceKey::new(race).map_err(debug)?,
                )]
                .into(),
            },
        );
        let root =
            oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(native.source_digest());
        let command = (1 + case_index as u64 * 4096..4096 + case_index as u64 * 4096).find(|command| {
        if matches!(charm, "savage_blow" | "savage_wrong_species" | "vampiric_embrace" | "voids_call" | "voids_wrong_species" | "leech_overkill") { return true; }
        let triggered = |target, ordinal| {
        crate::gameplay_transport::attack::run_charm_hook(
            owner.imported_charms, &state, id(41), race, &root,
            hit_occurrence(actor, session, *command, target, ordinal),
            oteryn_gameplay_progression::charm_effects::CharmHookEvent::CommittedHit {
                source: oteryn_gameplay_progression::charm_effects::CharmHitSource::CharacterAttack,
                attacker: oteryn_gameplay_progression::charm_effects::CharmAttackerFacts {level:facts.level,max_health:u64::from(facts.max_health),max_mana:u64::from(facts.max_mana)},
                creature_max_health:260, health_before:260, health_after:if charm == "carnage" {0} else {259},
            },
        ).is_ok_and(|outcomes| outcomes.iter().any(|outcome| matches!(
            crate::gameplay_transport::attack::charm_combat::effect(outcome),
            Some(oteryn_gameplay_progression::charm_effects::CharmEffectOutcome::ParalyseCreature { .. }
                | oteryn_gameplay_progression::charm_effects::CharmEffectOutcome::ProcDamage { .. }
                | oteryn_gameplay_progression::charm_effects::CharmEffectOutcome::PreventCreatureFlee { .. }
                | oteryn_gameplay_progression::charm_effects::CharmEffectOutcome::AreaDamageAroundKill { .. })
        )))
        };
        triggered(target, if delayed { 1 } else { 0 })
    }).ok_or("triggering cast")?;
        let spell = (1..=deps.spells.source_len())
            .filter_map(|i| u32::try_from(i).ok().and_then(std::num::NonZeroU32::new))
            .find(|i| {
                deps.spells
                    .indexed(*i)
                    .is_some_and(|s| s.name == if delayed { "Death Echo" } else { "Lightning" })
            })
            .ok_or("spell")?;
        let intent = actor_spell::SpellCastIntent {
            spell,
            target: actor_spell::SpellTarget::Position(actor_spell::SpellTargetPosition {
                x: position.x,
                y: if charm == "carnage" && delayed {
                    position.y - 2
                } else {
                    position.y
                },
                floor: position.floor,
            }),
            aim_at_target: !delayed,
        };
        // Arm the assignment only after the source cast: the due turn reads current assignments.
        {
            let runtime = deps.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            assert!(states.install_charm_state(
                &runtime,
                actor,
                session,
                crate::durability::charm_state::CombatCharmState::new(
                    crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?,
                    Default::default()
                )
            ));
        }
        {
            let mut runtime = deps.runtime.lock().await;
            let maximum = runtime
                .companion_snapshot_including_dead(target)
                .map_err(debug)?
                .maximum_health;
            runtime
                .set_test_creature_health(target, maximum)
                .map_err(debug)?;
        }
        if let Some(retire) = retire {
            cast_through_retired_holder(h, &owner, retire, actor, session, command, intent).await?;
        }
        let outcome = cast_ready(&owner, actor, session, command, intent).await?;
        assert_eq!(outcome.disposition, actor_spell::SpellCastDisposition::Cast);
        {
            let runtime = deps.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let selected = if matches!(
                charm,
                "wrong_species" | "savage_wrong_species" | "voids_wrong_species"
            ) {
                let key = crate::domain::charm::CharmKey::new(format!("oteryn:charm.{charm_key}"))
                    .map_err(debug)?;
                crate::durability::charm_state::CombatCharmState::new(
                    crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?,
                    crate::durability::charm_state::CharacterCharmState {
                        unlocks: [(
                            key.clone(),
                            crate::domain::charm::CharmStage::new(stage).map_err(debug)?,
                        )]
                        .into(),
                        assignments: [(
                            key,
                            crate::domain::charm::BestiaryRaceKey::new("oteryn:creature.rat")
                                .map_err(debug)?,
                        )]
                        .into(),
                    },
                )
            } else {
                state
            };
            assert!(states.install_charm_state(&runtime, actor, session, selected));
            assert!(
                runtime
                    .actor_conditions(target, None)
                    .map_err(debug)?
                    .instances()
                    .is_empty()
            );
        }
        {
            let mut runtime = deps.runtime.lock().await;
            let maximum = runtime
                .companion_snapshot_including_dead(target)
                .map_err(debug)?
                .maximum_health;
            runtime
                .set_test_creature_health(
                    target,
                    match charm {
                        "carnage" | "leech_overkill" => 1,
                        _ => maximum,
                    },
                )
                .map_err(debug)?;
            if charm == "carnage" {
                let child = runtime
                    .visible_entities()
                    .creatures
                    .into_iter()
                    .find(|c| {
                        runtime.read_actor_position(c.actor).is_ok_and(|p| {
                            p.position().x == position.x && p.position().y == position.y + 1
                        })
                    })
                    .ok_or("cardinal Carnage victim")?
                    .actor;
                runtime.set_test_creature_health(child, 1).map_err(debug)?;
            }
        }
        if matches!(
            charm,
            "leech"
                | "overpower_lethal"
                | "savage_blow"
                | "savage_wrong_species"
                | "vampiric_embrace"
                | "voids_call"
                | "voids_wrong_species"
                | "leech_overkill"
        ) {
            let mut states = owner.states.spell_states.lock().await;
            crate::gameplay_transport::actor_spell::tests::wound(&mut states, actor, session, 1);
        }
        let expected = if matches!(
            charm,
            "savage_blow" | "savage_wrong_species" | "overpower_lethal" | "leech_overkill"
        ) {
            let runtime = deps.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            Some(expected_due_damage(
                &runtime,
                &states,
                owner.imported_charms,
                native.source_digest(),
                actor,
                session,
                target,
                charm == "savage_blow",
                matches!(charm, "savage_blow" | "savage_wrong_species"),
            )?)
        } else {
            None
        };
        if charm == "overpower_lethal" {
            let amount = expected.ok_or("retained source damage")?;
            let mut runtime = deps.runtime.lock().await;
            assert!(
                amount > 0
                    && amount
                        < runtime
                            .companion_snapshot(target)
                            .map_err(debug)?
                            .maximum_health
            );
            runtime
                .set_test_creature_health(target, amount + 1)
                .map_err(debug)?;
        }
        let before_due = owner.observe_vitals(actor, session).await.ok_or("vitals")?;
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_millis(1100))
            .ok_or("clock")?;
        let report = owner.drain_native_ai_timers().await.map_err(debug)?;
        assert!(
            report.blocked.is_none(),
            "due blocked: {:?}",
            report.blocked
        );
        assert_eq!(report.receipts.len(), 1);
        if charm == "cripple_immune" {
            let runtime = deps.runtime.lock().await;
            assert!(
                runtime
                    .actor_conditions(target, None)
                    .map_err(debug)?
                    .instances()
                    .is_empty(),
                "native condition immunity leaves no false Cripple"
            );
        }
        if charm == "cripple" {
            let runtime = deps.runtime.lock().await;
            assert!(
                runtime
                    .actor_conditions(target, None)
                    .map_err(debug)?
                    .instances()
                    .iter()
                    .any(|i| i.provenance().definition_key == "oteryn:charm.paralysis"),
                "due callback must apply current assigned Cripple"
            );
        }
        let effects = &report.receipts[0].batch.effects;
        let leaves: Vec<_> = effects.iter().filter(|e| e.sub_ordinal >= 256).collect();
        let primary = effects
            .iter()
            .filter(|e| e.sub_ordinal < 256)
            .filter_map(|e| e.health)
            .map(|h| h.health_before - h.health_after)
            .sum::<i64>();
        if matches!(
            charm,
            "wrong_species"
                | "immune"
                | "fatal_hold"
                | "savage_blow"
                | "savage_wrong_species"
                | "zero"
                | "vampiric_embrace"
                | "voids_call"
                | "voids_wrong_species"
                | "leech_overkill"
        ) {
            assert!(
                leaves.is_empty(),
                "{charm} grants no damage or condition child"
            );
        } else if !matches!(charm, "cripple" | "cripple_immune") {
            assert_eq!(leaves.len(), 1, "{charm} uses the real due child owner");
            let child = leaves[0].health.ok_or("child HP")?;
            assert!(child.health_after < child.health_before);
            if charm == "overpower_lethal" {
                assert!(
                    effects
                        .iter()
                        .filter(|e| e.sub_ordinal < 256)
                        .filter_map(|e| e.health)
                        .all(|h| h.health_after > 0)
                );
                assert_eq!(
                    child.health_after, 0,
                    "generated proc can kill without becoming another source"
                );
            }
            if charm == "carnage" {
                assert_eq!(
                    child.health_after, 0,
                    "Carnage leaf kills its cardinal victim"
                );
                let runtime = deps.runtime.lock().await;
                for c in runtime.visible_entities().creatures {
                    let p = runtime
                        .read_actor_position(c.actor)
                        .map_err(debug)?
                        .position();
                    if p.y == position.y + 2 || p.x == position.x - 1 {
                        let snapshot = runtime.companion_snapshot(c.actor).map_err(debug)?;
                        assert_eq!(
                            snapshot.health, snapshot.maximum_health,
                            "leaf kills never recurse"
                        );
                    }
                }
            }
        }
        if charm == "zero" {
            assert_eq!(primary, 0, "immune source has zero damage");
            assert_eq!(
                owner.observe_vitals(actor, session).await,
                Some(before_due),
                "zero source grants neither life nor mana"
            );
        }
        if let Some(expected) = expected
            && charm != "leech_overkill"
        {
            assert_eq!(
                primary, expected,
                "due critical reads the current assigned race"
            );
        }
        if matches!(
            charm,
            "leech"
                | "overpower_lethal"
                | "savage_blow"
                | "savage_wrong_species"
                | "vampiric_embrace"
                | "voids_call"
                | "voids_wrong_species"
                | "leech_overkill"
        ) {
            let after = owner
                .observe_vitals(actor, session)
                .await
                .ok_or("after due")?;
            let (life_rate, mana_rate) = match charm {
                "vampiric_embrace" => (1920, 1600),
                "voids_call" | "leech_overkill" => (1600, 1760),
                "voids_wrong_species" => (1600, 1600),
                _ => (10000, 10000),
            };
            let basis = if charm == "leech_overkill" {
                assert_eq!(primary, 1, "physical overkill is clipped");
                let basis = expected.ok_or("pre-overkill damage")?;
                assert!(basis > primary);
                basis
            } else {
                primary
            };
            let basis = u32::try_from(basis)?;
            let life=oteryn_gameplay_combat_rules::resolution::equipped_offence::leech_amount_for_targets(basis,life_rate,1);
            let mana=oteryn_gameplay_combat_rules::resolution::equipped_offence::leech_amount_for_targets(basis,mana_rate,1);
            assert_eq!(
                after.1.health,
                before_due.1.health + life,
                "per-target life ceiling excludes generated damage"
            );
            assert_eq!(
                after.1.mana,
                (before_due.1.mana + mana).min(facts.max_mana),
                "one qualified mana gain includes only the assigned minor"
            );
        }
        if charm == "carnage" {
            // A source chain may have retained another real hit. Finish that admitted
            // callback before checking an empty duplicate drain.
            for _ in 0..4 {
                let next = owner.drain_native_ai_timers().await.map_err(debug)?;
                assert!(
                    next.blocked.is_none(),
                    "remaining source chain: {:?}",
                    next.blocked
                );
                if owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .spell_timers
                    .as_ref()
                    .is_none_or(|timer| timer.pending_len() == 0)
                {
                    break;
                }
            }
        }
        let after = owner.observe_vitals(actor, session).await.ok_or("after")?;
        assert!(
            owner
                .drain_native_ai_timers()
                .await
                .map_err(debug)?
                .receipts
                .is_empty()
        );
        assert_eq!(
            owner.observe_vitals(actor, session).await,
            Some(after),
            "duplicate callback grants no gain"
        );
        if charm == "fatal_hold" {
            {
                let mut runtime = deps.runtime.lock().await;
                let health = runtime.companion_snapshot(target).map_err(debug)?.health;
                runtime
                    .commit_monster_lab_damage(target, b"delayed-flee-threshold", health - 1)
                    .map_err(debug)?;
            }
            owner.clock_origin -= std::time::Duration::from_secs(1);
            owner.drain_monster_melee().await;
            assert!(
                !owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .monster_melee
                    .creature_ai
                    .state(target)
                    .ok_or("flee owner")?
                    .is_fleeing(),
                "due Fatal Hold suppresses fleeing"
            );
            owner.clock_origin -= std::time::Duration::from_secs(31);
            owner.drain_monster_melee().await;
            assert!(
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .monster_melee
                    .creature_ai
                    .state(target)
                    .ok_or("flee owner")?
                    .is_fleeing(),
                "due Fatal Hold expires"
            );
        }
        let costs: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(
            costs,
            i64::try_from(case_index + 1)?,
            "each source pays once across retries and due phases"
        );
        owner.clock_origin -= std::time::Duration::from_secs(20);
    }
    Ok(())
}
fn hit_occurrence(
    actor: crate::foundation::ExactActorRef,
    session: GameSessionId,
    command: u64,
    target: crate::foundation::ExactActorRef,
    ordinal: u16,
) -> oteryn_simulation_determinism::DecisionOccurrenceId {
    use sha2::{Digest, Sha256};
    let hash = Sha256::new()
        .chain_update(b"oteryn:spell-charm-hit:v1")
        .chain_update(actor.placement_identity())
        .chain_update(session.as_bytes())
        .chain_update(command.to_be_bytes())
        .chain_update(target.placement_identity())
        .chain_update(ordinal.to_be_bytes())
        .finalize();
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hash[..16]);
    oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(bytes)
}

// Synthetic skill amounts are read through the real fenced equipment owner.
// The critical fixture guarantees a visible extra-damage decision.
fn equipment_activation(
    world: crate::foundation::WorldId,
    critical: bool,
    leech_percent: u32,
) -> TestResult<(
    crate::content::ContentActivationController,
    crate::content::NativeEntryContentPin,
)> {
    use crate::content::*;
    use sha2::{Digest, Sha256};
    let mut input = native_gameplay::NativeGameplayInput::from_manifest(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../content/spells.manifest.json"),
    )?;
    let pinned = |bytes: Vec<u8>| native_gameplay::PinnedGameplayBytes {
        sha256: Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
        bytes,
    };
    input.source_world = Some(pinned(
        include_bytes!(
            "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
        )
        .to_vec(),
    ));
    input
        .build_training
        .as_mut()
        .ok_or("training")?
        .content_revision = "content-1".into();
    let pin = input.item_profiles.as_mut().ok_or("items")?;
    let mut document: serde_json::Value = serde_json::from_slice(&pin.bytes)?;
    let item = document["records"]
        .as_array_mut()
        .ok_or("records")?
        .iter_mut()
        .find(|r| r["production_definition"]["production_key"] == "oteryn:item.tibia.i6529")
        .ok_or("equipped item")?;
    item["attributes"]["spell_augments"] = serde_json::json!([]);
    let modifier = |kind, value| {
        serde_json::json!({
            "kind":kind,"parameter":{"state":"KNOWN","value":{"kind":"RATIONAL_PERCENT","value":{"numerator":value,"denominator":1}}},
            "evaluation_phase":{"state":"UNKNOWN"},"priority":{"state":"UNKNOWN"},"target_domain":{"state":"UNKNOWN"}
        })
    };
    let mut modifiers: Vec<_> = ["LIFE_LEECH_AMOUNT", "MANA_LEECH_AMOUNT"]
        .map(|kind| modifier(kind, leech_percent))
        .into();
    if critical {
        modifiers.insert(0, modifier("CRITICAL_HIT_CHANCE", 95));
    }
    item["semantics"]["skill_modifiers"] = serde_json::json!({"state":"KNOWN","value":{"modifiers":{"state":"KNOWN","value":modifiers}}});
    *pin = pinned(serde_json::to_vec(&document)?);
    let room = qualify_selected_native_gameplay_room(world, &input).map_err(debug)?;
    let issuance = NativeEntryActivationIssuance {
        world_id: world,
        activation_sequence: 1,
        server_artifact_digest: room.compiled().server_digest(),
        client_artifact_digest: room.compiled().client_digest(),
        frame_binding_digest: room.frame_binding().digest(),
    };
    let mut controller = ContentActivationController::new();
    let pin = activate_native_entry_room_with_gameplay(
        &mut controller,
        &NodeBootQuiescence::before_channel_runtime(),
        world,
        &issuance,
        &input,
    )
    .map_err(debug)?;
    Ok((controller, pin))
}

#[allow(
    clippy::too_many_arguments,
    reason = "test oracle keeps the exact owner provenance visible"
)]
fn expected_due_damage(
    runtime: &crate::foundation::ChannelRuntimeV1,
    states: &crate::gameplay_transport::actor_spell::ChannelSpellStates,
    catalogue: &crate::content::charm_source::CanonicalCharmCatalogue,
    digest: [u8; 32],
    actor: crate::foundation::ExactActorRef,
    session: GameSessionId,
    target: crate::foundation::ExactActorRef,
    assigned: bool,
    guaranteed: bool,
) -> TestResult<i64> {
    use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
    use sha2::{Digest, Sha256};
    let player = states.get(runtime, actor, session).ok_or("player")?;
    let timer = states.spell_timers.as_ref().ok_or("timer")?;
    let payload = &timer
        .payloads
        .iter()
        .find(|p| p.payload.binding().command.game_session_id() == session)
        .ok_or("pending")?
        .payload;
    let root = GameplayDecisionRoot::from_bytes(digest);
    let (hash, label) = match payload {
        crate::spell::delayed_execution::TimerPayload::Strike(saved) => (
            Sha256::new()
                .chain_update(b"oteryn:delayed-strike-due:v1")
                .chain_update(&saved.binding.parent_binding),
            "spell.strike.due.draw",
        ),
        crate::spell::delayed_execution::TimerPayload::OrdinaryCombat(saved) => (
            Sha256::new()
                .chain_update(b"oteryn:ordinary-chain-due:v1")
                .chain_update(&saved.binding.parent_binding)
                .chain_update(saved.phase.to_be_bytes()),
            "spell.ordinary.due.draw",
        ),
        _ => return Err("numeric callback".into()),
    };
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hash.finalize()[..16]);
    let decision = DecisionOccurrenceId::from_bytes(bytes);
    let mut ordinal = 0;
    let mut draw = |min, max| {
        let value = oteryn_simulation_determinism::deterministic_decision_u64(
            &root, decision, label, ordinal,
        )
        .map_err(debug)?;
        ordinal += 1;
        Ok::<_, Box<dyn std::error::Error>>(crate::spell::uniform_draw(value, min, max))
    };
    let (raw, element) = match payload {
        crate::spell::delayed_execution::TimerPayload::Strike(saved) => {
            let inputs = saved
                .binding
                .cast_snapshot
                .as_ref()
                .ok_or("cast snapshot")?
                .inputs;
            let p = &saved.binding.spell.spell()["execution"]["native_behavior"]["parameters"]["formula"];
            let evaluate = |key| -> TestResult<i64> {
                let expression =
                    oteryn_gameplay_content::content::spell_data::authoring::expression(&p[key], 1)
                        .map_err(debug)?;
                Ok((expression.evaluate(&inputs).map_err(debug)? * 0.5).trunc() as i64)
            };
            (draw(evaluate("minimum")?, evaluate("maximum")?)?, "death")
        }
        crate::spell::delayed_execution::TimerPayload::OrdinaryCombat(saved) => {
            let facts = player.character_facts();
            let vitals = player.vitals();
            let caster = crate::spell::CasterState {
                vocation: facts.vocation,
                level: facts.level,
                magic_level: facts.magic_level,
                premium: crate::spell::CasterPremium::Current,
                mana: vitals.mana,
                max_mana: vitals.max_mana,
                soul: vitals.soul,
                learned: Default::default(),
                attack_skill: 0,
                attack_value: 0,
                attack_factor: 0.0,
                shielding_skill: 0,
                melee_weapon: false,
                shield_defense: None,
                harmony_multiplier: crate::spell::harmony::HarmonyMultiplier::ONE,
            };
            let mut invalid = false;
            let effects = crate::spell::cast::resolve_ordinary_due_effects_with_wheel(
                &saved.binding.source,
                &caster,
                saved.step,
                &mut |min, max| match draw(min, max) {
                    Ok(value) => value,
                    Err(_) => {
                        invalid = true;
                        min
                    }
                },
                Default::default(),
            )
            .map_err(debug)?;
            assert!(!invalid);
            let [crate::spell::ResolvedEffect::Damage { magnitude, .. }] = effects.as_slice()
            else {
                return Err("source damage".into());
            };
            (*magnitude, "energy")
        }
        _ => return Err("numeric callback".into()),
    };
    let creature = runtime.companion_snapshot(target).map_err(debug)?;
    let offence = player.equipped_offence();
    let base_extra = offence.player_critical().1;
    let charm_state = states
        .charm_state(runtime, actor, session)
        .ok_or("charms")?;
    let outcomes = crate::gameplay_transport::attack::run_charm_hook(
        catalogue,
        charm_state,
        id(41),
        &creature.state.policy.definition_key,
        &root,
        DecisionOccurrenceId::from_bytes(id(133)),
        oteryn_gameplay_progression::charm_effects::CharmHookEvent::AttackDamageCalculation {
            source: oteryn_gameplay_progression::charm_effects::CharmHitSource::CharacterAttack,
        },
    )
    .map_err(debug)?;
    let (chance, _) = offence.player_critical();
    let bonus: u32 = outcomes.iter().filter_map(|outcome| match crate::gameplay_transport::attack::charm_combat::effect(outcome) {
        Some(oteryn_gameplay_progression::charm_effects::CharmEffectOutcome::CriticalExtraDamageBonus { percent }) => Some(percent.hundredths()),
        _ => None,
    }).sum();
    let extra = u32::from(base_extra) + bonus;
    if guaranteed {
        assert_eq!(
            chance, 10_000,
            "equipment plus the intrinsic chance guarantees a critical"
        );
    }
    assert_eq!(
        extra > u32::from(base_extra),
        assigned,
        "only the assigned race receives Savage Blow"
    );
    let mut raw = raw as f64;
    if draw(1, 10_000)? <= i64::from(chance) {
        raw = (raw * (1.0 + f64::from(extra) / 10_000.0)).trunc();
    }
    let mitigation = creature.state.policy.mitigation.ok_or("mitigation")?;
    let mitigation = (mitigation.numerator as f64 / mitigation.denominator as f64).min(30.0);
    let resistance = creature
        .state
        .policy
        .resistances
        .iter()
        .find(|r| r.damage_type == element)
        .map_or(0.0, |r| {
            r.percent.numerator as f64 / r.percent.denominator as f64
        });
    Ok(
        ((raw - raw * mitigation / 100.0).trunc() * (100.0 - resistance) / 100.0)
            .round()
            .max(0.0) as i64,
    )
}

/// A slow CI pass outlived the deadline and retired the sole holder mid-cast. Reproduce it:
/// hold a relation the cast pass needs past the deadline, so its first pass retires.
async fn cast_through_retired_holder(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    retire: Retire,
    actor: crate::foundation::ExactActorRef,
    session: GameSessionId,
    command: u64,
    intent: actor_spell::SpellCastIntent,
) -> TestResult {
    let mut blocker = h.pool.begin().await?;
    sqlx::query(retire.lock).execute(&mut *blocker).await?;
    let release = async {
        let held_until = tokio::time::Instant::now() + crate::durability::DB_PASS_DEADLINE * 2;
        if let Some(relation) = retire.cancel_waiter_on {
            // Wait for the holder to queue on the lock, then end its backend mid-statement.
            while tokio::time::Instant::now() < held_until {
                let waiter: Option<i32> = sqlx::query_scalar(
                    "SELECT pid FROM pg_locks WHERE relation = $1::regclass AND NOT granted LIMIT 1",
                )
                .bind(relation)
                .fetch_optional(&h.pool)
                .await?;
                if let Some(pid) = waiter {
                    sqlx::query("SELECT pg_terminate_backend($1)")
                        .bind(pid)
                        .execute(&h.pool)
                        .await?;
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        }
        tokio::time::sleep_until(held_until).await;
        blocker.rollback().await
    };
    let (outcome, released) = tokio::join!(
        owner.cast_spell_dispatch(actor, session, command, intent),
        release
    );
    released?;
    eprintln!("[DBG-charmzero] first={:?}", outcome.as_ref().map(|o| o.disposition));
    assert!(outcome.is_none(), "the blocked cast pass retires and stays pending");
    Ok(())
}

async fn cast_ready(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: GameSessionId,
    command: u64,
    intent: actor_spell::SpellCastIntent,
) -> TestResult<crate::gameplay_transport::actor_spell::SpellCastOutcome> {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        loop {
            if let Some(outcome) = owner
                .cast_spell_dispatch(actor, session, command, intent)
                .await
            {
                return TestResult::Ok(outcome);
            }
            // A pending cast may mean the durability holder retired; wait for the
            // recovered holder instead of spinning against a retired one.
            if !owner.root.is_ready() {
                owner.root.await_ready().await?;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .map_err(|_| "source cast remained pending across bounded same-command retries")?
}
