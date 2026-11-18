//! Real ordinary/native player casts must consume assigned outgoing charms.
use super::death_ml_loss_composed_fixtures::{current_fence, production_generation_with_source};
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
fn all_charms_outgoing_ordinary_cripple_real_cast() -> TestResult {
    scenario(false, "cripple", 3)
}
#[test]
fn all_charms_outgoing_native_cripple_real_cast() -> TestResult {
    scenario(true, "cripple", 3)
}
#[test]
fn all_charms_outgoing_ordinary_cripple_native_immunity() -> TestResult {
    scenario(false, "cripple_immune", 3)
}
#[test]
fn all_charms_outgoing_native_cripple_native_immunity() -> TestResult {
    scenario(true, "cripple_immune", 3)
}
#[test]
fn all_charms_outgoing_ordinary_cripple_condition_refusal() -> TestResult {
    scenario(false, "cripple_refusal", 3)
}
#[test]
fn all_charms_outgoing_native_cripple_condition_refusal() -> TestResult {
    scenario(true, "cripple_refusal", 3)
}
#[test]
fn all_charms_outgoing_ordinary_cripple_primary_kill() -> TestResult {
    scenario(false, "cripple_primary_kill", 3)
}
#[test]
fn all_charms_outgoing_native_cripple_primary_kill() -> TestResult {
    scenario(true, "cripple_primary_kill", 3)
}
#[test]
fn all_charms_outgoing_ordinary_child_has_no_leech() -> TestResult {
    scenario(false, "leech", 3)
}

macro_rules! damage_cases {
    ($(($ordinary:ident, $native:ident, $charm:literal)),+ $(,)?) => { $(
        #[test]
        fn $ordinary() -> TestResult { scenario(false, $charm, 3) }
        #[test]
        fn $native() -> TestResult { scenario(true, $charm, 3) }
    )+ };
}
damage_cases!(
    (
        all_charms_outgoing_ordinary_wrong_species,
        all_charms_outgoing_native_wrong_species,
        "wrong_species"
    ),
    (
        all_charms_outgoing_ordinary_immunity,
        all_charms_outgoing_native_immunity,
        "immune"
    ),
    (
        all_charms_outgoing_ordinary_physical_refusal,
        all_charms_outgoing_native_physical_refusal,
        "physical_refusal"
    ),
    (
        all_charms_outgoing_ordinary_wound,
        all_charms_outgoing_native_wound,
        "wound"
    ),
    (
        all_charms_outgoing_ordinary_poison,
        all_charms_outgoing_native_poison,
        "poison"
    ),
    (
        all_charms_outgoing_ordinary_freeze,
        all_charms_outgoing_native_freeze,
        "freeze"
    ),
    (
        all_charms_outgoing_ordinary_zap,
        all_charms_outgoing_native_zap,
        "zap"
    ),
    (
        all_charms_outgoing_ordinary_curse,
        all_charms_outgoing_native_curse,
        "curse"
    ),
    (
        all_charms_outgoing_ordinary_enflame,
        all_charms_outgoing_native_enflame,
        "enflame"
    ),
    (
        all_charms_outgoing_ordinary_divine_wrath,
        all_charms_outgoing_native_divine_wrath,
        "divine_wrath"
    ),
    (
        all_charms_outgoing_ordinary_overpower,
        all_charms_outgoing_native_overpower,
        "overpower"
    ),
    (
        all_charms_outgoing_ordinary_overflux,
        all_charms_outgoing_native_overflux,
        "overflux"
    ),
    (
        all_charms_outgoing_ordinary_carnage,
        all_charms_outgoing_native_carnage,
        "carnage"
    ),
    (
        all_charms_outgoing_ordinary_fatal_hold,
        all_charms_outgoing_native_fatal_hold,
        "fatal_hold"
    ),
);
fn scenario(native_path: bool, charm: &'static str, stage: u8) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(move || {
            run(async move |admin| scenario_inner(admin, native_path, charm, stage).await)
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "outgoing charm thread panicked")?
        .map_err(Into::into)
}
async fn scenario_inner(admin: String, native_path: bool, charm: &str, stage: u8) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "all_charms_outgoing", false, false).await?;
    // Release this mechanic's temporary database even when an assertion unwinds.
    let result = {
        let mut scenario = std::pin::pin!(scenario_with_harness(&h, native_path, charm, stage));
        std::future::poll_fn(|context| {
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                std::future::Future::poll(scenario.as_mut(), context)
            })) {
                Ok(std::task::Poll::Pending) => std::task::Poll::Pending,
                Ok(std::task::Poll::Ready(result)) => std::task::Poll::Ready(Ok(result)),
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
}

async fn scenario_with_harness(
    h: &Harness,
    native_path: bool,
    charm: &str,
    stage: u8,
) -> TestResult {
    let race = match charm {
        "immune" | "cripple_immune" => "oteryn:creature.dragon",
        "fatal_hold" => "oteryn:creature.orc_spearman",
        _ => "oteryn:creature.cyclops",
    };
    let charm_key = match charm {
        "wrong_species" | "leech" | "physical_refusal" => "wound",
        "immune" => "enflame",
        "cripple_immune" | "cripple_refusal" | "cripple_primary_kill" => "cripple",
        other => other,
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
    let (controller, pin) = if charm == "leech" {
        super::item_spell_augments_fixtures::activate(world, true)?
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
    let room = pin.qualified_room().clone();
    let cp = ChannelContentPin::from_activation(
        world,
        1,
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (2, 11, -7),
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
                amount: oteryn_simulation_determinism::ExactI64::new(3_251_800),
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
            if charm == "cripple_primary_kill" {
                runtime.set_test_creature_health(target, 1).map_err(debug)?;
            }
            if charm == "carnage" {
                runtime.set_test_creature_health(target, 1).map_err(debug)?;
                let child = runtime
                    .visible_entities()
                    .creatures
                    .into_iter()
                    .find(|creature| {
                        runtime
                            .read_actor_position(creature.actor)
                            .is_ok_and(|snapshot| {
                                let at = snapshot.position();
                                at.x == position.x && at.y == position.y + 1
                            })
                    })
                    .ok_or("Carnage cardinal victim")?
                    .actor;
                runtime.set_test_creature_health(child, 1).map_err(debug)?;
            }
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
    if charm == "leech" {
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
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    h.root
        .initialize_familiar_group(&authority, &h.node, current_fence(&owner, session).await?)
        .await
        .map_err(debug)?;
    if matches!(charm, "leech" | "cripple_refusal") {
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
    let key =
        crate::domain::charm::CharmKey::new(format!("oteryn:charm.{charm_key}")).map_err(debug)?;
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
    let secondary = if native_path && charm != "carnage" {
        let runtime = deps.runtime.lock().await;
        Some(
            runtime
                .visible_entities()
                .creatures
                .into_iter()
                .find(|creature| creature.actor != target)
                .ok_or("AoE secondary target")?
                .actor,
        )
    } else {
        None
    };
    let command = (1..4096).find(|command| {
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
        triggered(target, 0) && secondary.is_none_or(|target| triggered(target, 1))
    }).ok_or("triggering cast")?;
    {
        let runtime = deps.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        let selected = if charm == "wrong_species" {
            let key = crate::domain::charm::CharmKey::new("oteryn:charm.wound").map_err(debug)?;
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
    }
    let spell = (1..=deps.spells.source_len())
        .filter_map(|i| u32::try_from(i).ok().and_then(std::num::NonZeroU32::new))
        .find(|i| {
            deps.spells.indexed(*i).is_some_and(|s| {
                s.name
                    == if native_path {
                        "Strong Ice Wave"
                    } else {
                        "Energy Strike"
                    }
            })
        })
        .ok_or("spell")?;
    let intent = actor_spell::SpellCastIntent {
        spell,
        target: actor_spell::SpellTarget::Position(actor_spell::SpellTargetPosition {
            x: position.x,
            y: position.y,
            floor: position.floor,
        }),
        aim_at_target: native_path,
    };
    if charm == "cripple_refusal" {
        // Current Premium is an independent admission transition, not spell cost.
        // Install it through its real owner before measuring this refused cast;
        // keep the exact full-vitals/revision assertion below unchanged.
        let access = owner.refresh_spell_access(actor, session).await;
        owner
            .apply_refreshed_spell_access(actor, session, &access)
            .await
            .map_err(debug)?;
        let before = owner
            .observe_vitals(actor, session)
            .await
            .ok_or("before canonical refusal")?;
        let (hp, conditions) = {
            let mut runtime = deps.runtime.lock().await;
            let future = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(
                owner.owner_now().get() + 60_000_000,
            );
            let plan = runtime
                .prepare_actor_condition_expiry(
                    target,
                    None,
                    oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(id(124)),
                    future,
                )
                .map_err(debug)?;
            runtime
                .commit_actor_condition(&plan, future)
                .map_err(debug)?;
            (
                runtime.companion_snapshot(target).map_err(debug)?.health,
                runtime
                    .actor_conditions(target, None)
                    .map_err(debug)?
                    .clone(),
            )
        };
        let refused = owner
            .cast_spell_dispatch(actor, session, command, intent)
            .await;
        assert!(
            refused.is_none_or(
                |outcome| outcome.disposition != actor_spell::SpellCastDisposition::Cast
            ),
            "canonical preparation refusal rejects the entire cast"
        );
        assert_eq!(
            owner.observe_vitals(actor, session).await,
            Some(before),
            "canonical refusal pays no mana or cooldown"
        );
        let runtime = deps.runtime.lock().await;
        assert_eq!(
            runtime.companion_snapshot(target).map_err(debug)?.health,
            hp,
            "canonical refusal publishes no primary HP"
        );
        assert_eq!(
            runtime.actor_conditions(target, None).map_err(debug)?,
            &conditions,
            "canonical refusal publishes no condition"
        );
        return Ok(());
    }
    let outcome = owner
        .cast_spell_dispatch(actor, session, command, intent)
        .await
        .ok_or("cast pending")?;
    assert_eq!(outcome.disposition, actor_spell::SpellCastDisposition::Cast);
    let mut primary_damage = 0;
    let mut runtime = deps.runtime.lock().await;
    let target_facts = runtime
        .companion_snapshot_including_dead(target)
        .map_err(debug)?;
    if matches!(charm, "carnage" | "cripple_primary_kill") {
        assert_eq!(target_facts.health, 0, "primary kill");
    } else {
        assert!(
            target_facts.health > 0 && target_facts.health < target_facts.maximum_health,
            "the real source spell must damage a living creature"
        );
    }
    let condition_before_replay = condition_snapshot_including_dead(&mut runtime, target)?;
    if charm == "cripple_primary_kill" {
        assert!(
            condition_before_replay.instances().is_empty(),
            "primary source death cannot acquire the minor"
        );
    }
    if charm == "cripple_immune" {
        assert!(
            target_facts
                .state
                .policy
                .condition_immunities
                .contains(&crate::foundation::ConditionType::Paralysis)
        );
        assert_eq!(
            runtime
                .actor_active_speed_delta(target, None, owner.owner_now())
                .map_err(debug)?,
            0
        );
        assert!(
            condition_before_replay.instances().is_empty(),
            "native immunity is a successful primary cast with no condition"
        );
    }
    if charm == "cripple" {
        let instance = condition_before_replay
            .instances()
            .iter()
            .find(|instance| instance.provenance().definition_key == "oteryn:charm.paralysis")
            .ok_or("canonical Cripple instance")?;
        assert_eq!(
            instance.remaining_duration_ms(instance.started_at()),
            Some(10_000)
        );
        assert_eq!(instance.provenance().source, Some(actor));
        if let Some(secondary) = secondary {
            assert!(
                runtime
                    .actor_active_speed_delta(secondary, None, owner.owner_now())
                    .map_err(debug)?
                    < 0,
                "each impact retains its own canonical condition"
            );
        }
        assert!(
            runtime
                .actor_active_speed_delta(target, None, owner.owner_now())
                .map_err(debug)?
                < 0,
            "Cripple must reach the real creature condition owner on native={native_path}"
        );
    } else if !matches!(
        charm,
        "fatal_hold" | "cripple_immune" | "cripple_primary_kill"
    ) {
        let batch = runtime
            .retained_spell_batch(
                actor,
                crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                2,
                crate::foundation::CommandRef::new(
                    session,
                    crate::foundation::CommandId::new(command).map_err(debug)?,
                ),
            )
            .map_err(debug)?
            .ok_or("retained source")?;
        let source_binding: serde_json::Value = serde_json::from_slice(&batch.binding)?;
        assert!(
            source_binding.get("source_charm_children").is_none(),
            "timer and magnitude consumers retain the original source identity"
        );
        let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
        if charm == "leech" {
            let health = staged
                .preview_effect_receipts()
                .iter()
                .find(|effect| effect.target == target && effect.sub_ordinal < 256)
                .and_then(|effect| effect.health)
                .ok_or("primary health")?;
            primary_damage = u32::try_from(health.health_before - health.health_after)?;
        }
        let children: Vec<_> = staged
            .preview_effect_receipts()
            .iter()
            .filter(|effect| effect.sub_ordinal >= 256)
            .collect();
        if matches!(charm, "wrong_species" | "immune") {
            assert!(
                children.is_empty(),
                "wrong species and native immunity grant no child"
            );
        } else {
            assert_eq!(
                children.len(),
                if secondary.is_some() { 2 } else { 1 },
                "{charm} must land a physical child on native={native_path}"
            );
            if let Some(secondary) = secondary {
                let child = children
                    .iter()
                    .find(|effect| effect.target == secondary)
                    .ok_or("AoE child")?
                    .health
                    .ok_or("AoE child HP")?;
                assert!(
                    child.health_before > child.health_after,
                    "each AoE target consumes its own hook"
                );
                assert_eq!(
                    runtime.companion_snapshot(secondary).map_err(debug)?.health,
                    child.health_after
                );
            }
            let child = children
                .iter()
                .find(|effect| effect.target == target)
                .or_else(|| children.first())
                .ok_or("first child")?
                .health
                .ok_or("child HP")?;
            assert!(
                child.health_before > child.health_after,
                "{charm} HP transition"
            );
            if charm == "carnage" {
                assert_ne!(
                    children[0].target, target,
                    "cardinal neighbour is outside source target set"
                );
                assert_eq!(
                    (child.health_before, child.health_after),
                    (1, 0),
                    "actual generated death"
                );
                let shares = runtime
                    .borrow_combat_death()
                    .reward_damage_shares(children[0].target)
                    .map_err(debug)?;
                assert_eq!(shares.len(), 1, "generated death retains its actual caster");
                assert_eq!(shares[0].character.as_bytes(), &id(41));
                assert_eq!(
                    shares[0].total, 1,
                    "generated death credits removed HP, not requested overkill"
                );
                for candidate in runtime.visible_entities().creatures {
                    if candidate.actor != target && candidate.actor != children[0].target {
                        assert_eq!(
                            runtime
                                .companion_snapshot(candidate.actor)
                                .map_err(debug)?
                                .health,
                            260,
                            "diagonal and grandchild cannot receive Carnage"
                        );
                    }
                }
            } else {
                assert_eq!(child.health_after, target_facts.health);
            }
        }
        let shares = runtime
            .borrow_combat_death()
            .reward_damage_shares(target)
            .map_err(debug)?;
        assert_eq!(
            shares.iter().map(|share| share.total).sum::<u64>(),
            u64::try_from(if charm == "carnage" {
                1
            } else {
                target_facts.maximum_health - target_facts.health
            })?,
            "contributions count actual removed primary plus child HP"
        );
    }
    let health = runtime
        .companion_snapshot_including_dead(target)
        .map_err(debug)?
        .health;
    drop(runtime);
    {
        let runtime = deps.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        assert!(states.install_charm_state(
            &runtime,
            actor,
            session,
            crate::durability::charm_state::CombatCharmState::new(
                crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?,
                crate::durability::charm_state::CharacterCharmState::default()
            )
        ));
    }
    let before_replay = owner.observe_vitals(actor, session).await.ok_or("vitals")?;
    if charm == "leech" {
        assert_eq!(
            before_replay.1.health,
            1 + primary_damage,
            "100% equipment Leech credits the source hit and excludes generated charm HP"
        );
    }
    let replay = owner
        .cast_spell_dispatch(actor, session, command, intent)
        .await
        .ok_or("replay pending")?;
    assert_eq!(replay.disposition, actor_spell::SpellCastDisposition::Cast);
    assert_eq!(
        owner.observe_vitals(actor, session).await,
        Some(before_replay)
    );
    assert_eq!(
        deps.runtime
            .lock()
            .await
            .companion_snapshot_including_dead(target)
            .map_err(debug)?
            .health,
        health
    );
    assert_eq!(
        condition_snapshot_including_dead(&mut *deps.runtime.lock().await, target)?,
        condition_before_replay,
        "duplicate occurrence neither reapplies nor refreshes the canonical condition"
    );
    let stale = GameSessionId::decode(&id(111)).map_err(debug)?;
    let refused = owner
        .cast_spell_dispatch(actor, stale, command + 1, intent)
        .await;
    assert!(
        refused
            .is_none_or(|outcome| outcome.disposition != actor_spell::SpellCastDisposition::Cast)
    );
    assert_eq!(
        owner.observe_vitals(actor, session).await,
        Some(before_replay)
    );
    assert_eq!(
        deps.runtime
            .lock()
            .await
            .companion_snapshot_including_dead(target)
            .map_err(debug)?
            .health,
        health
    );
    if charm == "physical_refusal" {
        let before = owner
            .observe_vitals(actor, session)
            .await
            .ok_or("before refusal")?;
        let before_secondary = if let Some(secondary) = secondary {
            Some(
                deps.runtime
                    .lock()
                    .await
                    .companion_snapshot(secondary)
                    .map_err(debug)?
                    .health,
            )
        } else {
            None
        };
        let blocker = {
            use crate::foundation::runtime_actor_spell_types::{
                OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
            };
            let mut runtime = deps.runtime.lock().await;
            let now = owner.owner_now();
            let batch = OwnerCombatBatch {
                caster: actor,
                attacker: crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                current_lease_generation: 2,
                command: crate::foundation::CommandRef::new(
                    session,
                    crate::foundation::CommandId::new(command + 20_000).map_err(debug)?,
                ),
                occurrence: SpellOccurrenceBinding {
                    id: "outgoing-physical-refusal".into(),
                    revisions: std::array::from_fn(|_| "fixture:1".into()),
                },
                binding: b"{\"outgoing_refusal\":true}".to_vec(),
                anchor: None,
                now_us: Some(now),
                now_ms: now.get() / 1000,
                effects: vec![OwnerCombatEffect {
                    target,
                    sub_ordinal: 0,
                    change: OwnerCombatChange::Damage {
                        target_atom: runtime.creature_spell_target_atom(target).map_err(debug)?,
                        magnitude: 1,
                    },
                }],
                deferred: None,
            };
            let mut staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
            runtime.reserve_spell_batch(&mut staged).map_err(debug)?;
            staged
        };
        let refused = owner
            .cast_spell_dispatch(actor, session, command + 10_000, intent)
            .await;
        assert!(
            refused.is_none_or(
                |outcome| outcome.disposition != actor_spell::SpellCastDisposition::Cast
            )
        );
        assert_eq!(
            owner.observe_vitals(actor, session).await,
            Some(before),
            "refused player owner pays nothing"
        );
        let mut runtime = deps.runtime.lock().await;
        assert_eq!(
            runtime.companion_snapshot(target).map_err(debug)?.health,
            health,
            "refused physical owner applies no primary or child HP"
        );
        if let Some(secondary) = secondary {
            assert_eq!(
                runtime.companion_snapshot(secondary).map_err(debug)?.health,
                before_secondary.ok_or("secondary before refusal")?,
                "AoE refusal is atomic"
            );
        }
        runtime
            .release_definitely_uncommitted_spell_batch(&blocker)
            .map_err(debug)?;
    }
    if charm == "cripple" {
        let before = owner
            .observe_vitals(actor, session)
            .await
            .ok_or("before condition boundary")?;
        {
            let mut runtime = deps.runtime.lock().await;
            cripple_retained_boundaries(
                &mut runtime,
                actor,
                session,
                target,
                command,
                native.source_digest(),
                owner.owner_now(),
            )?;
        }
        assert_eq!(
            owner.observe_vitals(actor, session).await,
            Some(before),
            "condition preparation and refused physical plans never charge payment"
        );
    }
    if charm == "fatal_hold" {
        {
            let mut runtime = deps.runtime.lock().await;
            let after = runtime.companion_snapshot(target).map_err(debug)?.health;
            runtime
                .commit_monster_lab_damage(target, b"outgoing-flee-threshold", after - 1)
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
                .ok_or("native flee owner")?
                .is_fleeing(),
            "Fatal Hold suppresses real fleeing"
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
                .ok_or("native flee owner")?
                .is_fleeing(),
            "fleeing resumes after Fatal Hold expires"
        );
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

/// Exercise retained preparation on the same native actors used by the real dispatch above.
#[allow(
    clippy::too_many_arguments,
    reason = "explicit fixture identities and original occurrence"
)]
fn cripple_retained_boundaries(
    runtime: &mut crate::foundation::ChannelRuntimeV1,
    actor: crate::foundation::ExactActorRef,
    session: GameSessionId,
    target: crate::foundation::ExactActorRef,
    command: u64,
    root: [u8; 32],
    now: oteryn_simulation_determinism::SemanticTimeMicros,
) -> TestResult {
    use crate::foundation::runtime_actor_spell_types::{
        MAX_EFFECTS, OwnerCombatChange, OwnerCombatEffect,
    };
    use oteryn_gameplay_entities::carrier::runtime_actor_spell::{
        QualifiedSpellChildren, SpellCharmChild,
    };
    use oteryn_simulation_determinism::{DecisionOccurrenceId, SemanticTimeMicros};
    let mut batch = runtime
        .retained_spell_batch(
            actor,
            crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
            2,
            crate::foundation::CommandRef::new(
                session,
                crate::foundation::CommandId::new(command).map_err(debug)?,
            ),
        )
        .map_err(debug)?
        .ok_or("retained cast")?;
    batch.command = crate::foundation::CommandRef::new(
        session,
        crate::foundation::CommandId::new(command + 40_000).map_err(debug)?,
    );
    batch.anchor = None;
    batch.binding = br#"{"cripple_boundary":true}"#.to_vec();
    batch.now_us = Some(now);
    batch.now_ms = now.get() / 1000;
    batch.effects = vec![OwnerCombatEffect {
        target,
        sub_ordinal: 0,
        change: OwnerCombatChange::Damage {
            target_atom: runtime.creature_spell_target_atom(target).map_err(debug)?,
            magnitude: 1,
        },
    }];
    let original = batch.clone();
    let occurrence = DecisionOccurrenceId::from_bytes(id(120));
    let child = SpellCharmChild::Paralysis {
        target,
        root,
        occurrence,
    };
    let proof =
        QualifiedSpellChildren::bind(runtime, &mut batch, vec![child.clone()]).map_err(debug)?;
    let health = runtime.companion_snapshot(target).map_err(debug)?.health;
    let conditions = runtime
        .actor_conditions(target, None)
        .map_err(debug)?
        .clone();
    let mut staged = runtime
        .stage_spell_batch_with_children(&batch, &proof)
        .map_err(debug)?;
    runtime.reserve_spell_batch(&mut staged).map_err(debug)?;
    assert!(
        runtime
            .stage_spell_batch_with_children(&batch, &proof)
            .map_err(debug)?
            .will_apply(),
        "retained own reservation can be retried"
    );
    runtime
        .release_definitely_uncommitted_spell_batch(&staged)
        .map_err(debug)?;
    assert_eq!(
        runtime.companion_snapshot(target).map_err(debug)?.health,
        health
    );
    assert_eq!(
        runtime.actor_conditions(target, None).map_err(debug)?,
        &conditions
    );

    let mut oversized = original.clone();
    assert!(
        QualifiedSpellChildren::bind(
            runtime,
            &mut oversized,
            vec![child.clone(); MAX_EFFECTS + 1]
        )
        .is_err(),
        "condition children share the retained leaf bound"
    );
    let mut duplicate = original.clone();
    assert!(
        QualifiedSpellChildren::bind(runtime, &mut duplicate, vec![child.clone(), child.clone()])
            .is_err(),
        "the same condition occurrence cannot be staged twice"
    );
    let mut multiple = original.clone();
    let multiple_proof = QualifiedSpellChildren::bind(
        runtime,
        &mut multiple,
        vec![
            child.clone(),
            SpellCharmChild::Paralysis {
                target,
                root,
                occurrence: DecisionOccurrenceId::from_bytes(id(121)),
            },
        ],
    )
    .map_err(debug)?;
    let multiple_staged = runtime
        .stage_spell_batch_with_children(&multiple, &multiple_proof)
        .map_err(debug)?;
    assert_eq!(
        multiple_staged.preview_effect_receipts().len(),
        3,
        "multiple impacts compose in source order"
    );
    let stale = runtime
        .prepare_actor_condition_expiry(
            target,
            None,
            DecisionOccurrenceId::from_bytes(id(122)),
            now,
        )
        .map_err(debug)?;
    runtime.commit_actor_condition(&stale, now).map_err(debug)?;
    assert!(
        runtime
            .stage_spell_batch_with_children(&batch, &proof)
            .is_err(),
        "changed canonical revision refuses retained preparation"
    );
    assert_eq!(
        runtime.companion_snapshot(target).map_err(debug)?.health,
        health,
        "stale condition plan applies no primary HP"
    );
    assert_eq!(
        runtime.actor_conditions(target, None).map_err(debug)?,
        &conditions
    );
    assert!(
        runtime.commit_spell_batch(multiple_staged).is_err(),
        "a changed preimage also refuses an already staged successor"
    );

    let mut changed = original.clone();
    let changed_proof =
        QualifiedSpellChildren::bind(runtime, &mut changed, vec![child.clone()]).map_err(debug)?;
    runtime
        .set_test_creature_health(target, health - 1)
        .map_err(debug)?;
    assert!(
        runtime
            .stage_spell_batch_with_children(&changed, &changed_proof)
            .is_err(),
        "changed target HP refuses retained leaf"
    );
    assert_eq!(
        runtime.companion_snapshot(target).map_err(debug)?.health,
        health - 1
    );
    runtime
        .set_test_creature_health(target, health)
        .map_err(debug)?;
    let mut changed_source = original.clone();
    changed_source.command = crate::foundation::CommandRef::new(
        GameSessionId::decode(&id(111)).map_err(debug)?,
        changed_source.command.command_id(),
    );
    let changed_source_proof =
        QualifiedSpellChildren::bind(runtime, &mut changed_source, vec![child.clone()]);
    assert!(
        changed_source_proof.is_err(),
        "changed source session cannot prepare a condition"
    );

    let mut lethal = original.clone();
    let lethal_proof = QualifiedSpellChildren::bind(
        runtime,
        &mut lethal,
        vec![
            SpellCharmChild::Damage(OwnerCombatEffect {
                target,
                sub_ordinal: MAX_EFFECTS as u16,
                change: OwnerCombatChange::Damage {
                    target_atom: runtime.creature_spell_target_atom(target).map_err(debug)?,
                    magnitude: health,
                },
            }),
            child.clone(),
        ],
    )
    .map_err(debug)?;
    let staged = runtime
        .stage_spell_batch_with_children(&lethal, &lethal_proof)
        .map_err(debug)?;
    assert_eq!(
        runtime.companion_snapshot(target).map_err(debug)?.health,
        health,
        "lethal preview is read-only"
    );
    runtime.commit_spell_batch(staged).map_err(debug)?;
    assert!(
        condition_snapshot_including_dead(runtime, target)?
            .instances()
            .is_empty(),
        "a prior generated major kills before the minor and death clears canonical conditions"
    );
    assert_eq!(
        runtime
            .companion_snapshot_including_dead(target)
            .map_err(debug)?
            .health,
        0
    );
    let replay = runtime
        .stage_spell_batch_with_children(&lethal, &lethal_proof)
        .map_err(debug)?;
    assert!(
        !runtime.commit_spell_batch(replay).map_err(debug)?.applied,
        "duplicate committed occurrence returns the original receipt even after target death"
    );

    // A different still-live native actor can carry a newer owner clock.
    if let Some(other) = runtime
        .visible_entities()
        .creatures
        .into_iter()
        .find(|entry| entry.actor != target)
    {
        let future = SemanticTimeMicros::from_micros(now.get() + 1_000_000);
        let expiry = runtime
            .prepare_actor_condition_expiry(
                other.actor,
                None,
                DecisionOccurrenceId::from_bytes(id(123)),
                future,
            )
            .map_err(debug)?;
        runtime
            .commit_actor_condition(&expiry, future)
            .map_err(debug)?;
        let before = runtime.companion_snapshot(other.actor).map_err(debug)?;
        let mut refused = original;
        assert!(
            QualifiedSpellChildren::bind(
                runtime,
                &mut refused,
                vec![SpellCharmChild::Paralysis {
                    target: other.actor,
                    root,
                    occurrence
                }]
            )
            .is_err(),
            "canonical time refusal rejects preparation without publishing primary damage"
        );
        assert_eq!(
            runtime.companion_snapshot(other.actor).map_err(debug)?,
            before
        );
    }
    Ok(())
}

// The production condition read intentionally requires a live creature. Observe the
// unchanged dead slot through the existing HP fixture, restoring HP before returning.
fn condition_snapshot_including_dead(
    runtime: &mut crate::foundation::ChannelRuntimeV1,
    target: crate::foundation::ExactActorRef,
) -> TestResult<crate::foundation::ConditionStore<crate::foundation::ExactActorRef>> {
    let health = runtime
        .companion_snapshot_including_dead(target)
        .map_err(debug)?
        .health;
    if health == 0 {
        runtime.set_test_creature_health(target, 1).map_err(debug)?;
    }
    let result = runtime.actor_conditions(target, None).cloned();
    if health == 0 {
        runtime.set_test_creature_health(target, 0).map_err(debug)?;
    }
    result.map_err(debug)
}
