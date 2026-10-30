//! C18c: delayed casts through real dispatch and fenced Character/Equipment owners.
use super::death_ml_loss_composed::{current_fence, production_generation_with_source};
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};
use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceRewardOccurrence,
};
use crate::foundation::{ChannelContentPin, CharacterLease, WorldId};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::gameplay_transport::item_fence_of;
use crate::gameplay_transport::spell_character_facts::{CastFactsLoad, load_character_cast_facts};
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_protocol_oteryn::actor_spell;
use std::time::Duration;

/// What one vocation's scenario casts: the durable vocation and experience that reach every
/// spell's level, the weapon the formulas read, and `(key, level, mana)` of each spell.
#[derive(Clone, Copy)]
struct Plan {
    tag: &'static str,
    vocation: &'static str,
    experience: i64,
    weapon: &'static str,
    spells: &'static [(&'static str, u32, i64)],
    case: Case,
}

/// The rat's health before every cast: far above any one hit, so the health lost is the damage.
const RAT_HEALTH: i64 = 100_000;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    Echo,
    Grenade,
    Pz,
    StageZero,
    Outburst(u8),
    Logout,
    DepartedTarget,
    QueuedOrder,
}

#[test]
fn delayed_strikes_postgres_death_echo_two_hits() -> TestResult {
    scenario(Plan {
        tag: "delayed_echo",
        vocation: "sorcerer",
        experience: 30_000_000,
        weapon: "oteryn:item.tibia.i3277",
        spells: &[("candidate:spell/death_echo", 120, 150)],
        case: Case::Echo,
    })
}

#[test]
fn delayed_strikes_postgres_grenade_position_three_seconds() -> TestResult {
    grenade(Case::Grenade, "delayed_grenade")
}
#[test]
fn delayed_strikes_postgres_grenade_pz_no_cost() -> TestResult {
    grenade(Case::Pz, "delayed_pz")
}
#[test]
fn delayed_strikes_postgres_grenade_stage_zero_no_cost() -> TestResult {
    grenade(Case::StageZero, "delayed_stage_zero")
}
fn grenade(case: Case, tag: &'static str) -> TestResult {
    scenario(Plan {
        tag,
        vocation: "paladin",
        experience: 30_000_000_000,
        weapon: "oteryn:item.tibia.i3277",
        spells: &[("candidate:spell/divine_grenade", 0, 160)],
        case,
    })
}
#[test]
fn delayed_strikes_postgres_outburst_full_harmony_repeats() -> TestResult {
    outburst(5, "delayed_full")
}
#[test]
fn delayed_strikes_postgres_outburst_partial_harmony_no_repeat() -> TestResult {
    outburst(4, "delayed_partial")
}
fn outburst(harmony: u8, tag: &'static str) -> TestResult {
    scenario(Plan {
        tag,
        vocation: "monk",
        experience: 30_000_000_000,
        weapon: "oteryn:item.tibia.i3277",
        spells: &[("candidate:spell/spiritual_outburst", 0, 425)],
        case: Case::Outburst(harmony),
    })
}
#[test]
fn delayed_strikes_postgres_logout_drops_pending() -> TestResult {
    scenario(Plan {
        tag: "delayed_logout",
        vocation: "sorcerer",
        experience: 30_000_000,
        weapon: "oteryn:item.tibia.i3277",
        spells: &[("candidate:spell/death_echo", 120, 150)],
        case: Case::Logout,
    })
}

#[test]
fn delayed_strikes_postgres_departed_target_does_not_block_timer_lane() -> TestResult {
    scenario(Plan {
        tag: "delayed_target_left",
        vocation: "sorcerer",
        experience: 30_000_000,
        weapon: "oteryn:item.tibia.i3277",
        spells: &[("candidate:spell/death_echo", 120, 150)],
        case: Case::DepartedTarget,
    })
}

#[test]
fn delayed_strikes_postgres_overlapping_callbacks_follow_lane_order() -> TestResult {
    scenario(Plan {
        tag: "delayed_order",
        vocation: "sorcerer",
        experience: 30_000_000,
        weapon: "oteryn:item.tibia.i3277",
        spells: &[
            ("candidate:spell/death_echo", 120, 150),
            ("candidate:spell/death_echo", 120, 150),
        ],
        case: Case::QueuedOrder,
    })
}

fn scenario(plan: Plan) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(move || {
            run(async move |admin| scenario_inner(admin, plan).await)
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "crosshair spell thread panicked")?
        .map_err(Into::into);
    eprintln!(
        "C18C_CASE {} elapsed={:.2}s",
        plan.tag,
        started.elapsed().as_secs_f64()
    );
    result
}

async fn scenario_inner(admin: String, plan: Plan) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, plan.tag, false, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) = production_generation_with_source(
        world,
        "content-1",
        Some(include_bytes!(
            "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
        )),
    )
    .map_err(debug)?;
    let active = controller.active().ok_or("active")?;
    let native = active.native_gameplay().ok_or("native")?;
    let entry = pin.qualified_room().clone();
    let room = if plan.case == Case::Pz {
        let map = crate::map::spell_tiles::tests::world();
        let cells = map.movement_cells(entry.movement_cells()).map_err(debug)?;
        entry
            .bundle_view(
                cells,
                map.frame_binding_digest(world),
                map.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: 1,
                    y: 0,
                    floor: 7,
                },
            )
            .ok_or("PZ room")?
    } else {
        entry
    };
    // Reuse crosshair_spells' open combat tile; the imported entry is a PZ.
    let start = if plan.case == Case::Pz {
        room.entry_start()
    } else {
        crate::content::NativeEntryStart {
            x: 2,
            y: 11,
            floor: -7,
        }
    };
    let cp = ChannelContentPin::from_activation(
        world,
        1,
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let mut deps =
        Dependencies::with_content_and_capacity(cp, room.clone(), native.spell_book().clone(), 3)?;
    deps.combat_room = Some(room);
    // Every spell here is premium: the account holds a Current Premium.
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
    let data =
        oteryn_gameplay_progression::wheel_gem_data::WheelGemData::embedded().map_err(debug)?;
    let wheel_rules = std::sync::Arc::new(
        crate::durability::character_wheel::WheelRuleset::from_catalogue(
            data.revision(),
            data.wheel(),
        )
        .map_err(debug)?,
    );
    let mut owner = deps.owner(&h, &authority);
    owner.wheel_ruleset = Some(wheel_rules.clone());
    owner.active_generation = Some(active);
    owner.premium_coordinator = Some(&premium);
    let mut admitted = deps.admitted;
    let actor = deps.actor;
    let session = admitted.game_session_id;
    owner.admit_premium(id(40), session);
    owner.admit_progression_session(&mut admitted).await;
    let progression = owner.progression_binding(session).ok_or("progression")?;
    let formula = native.training_formula().ok_or("training")?;
    let build =
        convert_vocation(formula, &DurableBuildState::default(), plan.vocation).map_err(debug)?;
    h.root
        .commit_character_build(
            &authority,
            &h.node,
            current_fence(&owner, session).await?,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(90)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                before: DurableBuildState::default(),
                after: build.clone(),
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
                amount: oteryn_simulation_determinism::ExactI64::new(plan.experience),
                context: progression.context.clone(),
                policy_revision: progression.policy_revision.clone(),
                reward_revision: progression.reward_revision.clone(),
                policy: progression.policy.clone(),
            },
        )
        .await
        .map_err(debug)?;
    if matches!(plan.case, Case::Grenade | Case::Pz | Case::Outburst(_)) {
        let purchase = super::atelier::fixture::prepare(&h).await?;
        let fence = current_fence(&owner, session).await?;
        let promoted = owner
            .revision_sequencer
            .acquire(fence.character_id)
            .await
            .commit_npc_purchase(&h.root, &authority, &h.node, fence, purchase, &premium)
            .await
            .map_err(debug)?;
        assert!(matches!(
            promoted,
            crate::durability::npc_purchase::PurchaseOutcome::Committed(_)
        ));
        let mut points = [0; 36];
        for (i, slot) in data.wheel()["topology"]
            .as_array()
            .ok_or("topology")?
            .iter()
            .enumerate()
        {
            if slot["domain"] == "red" {
                points[i] = u16::try_from(slot["capacity"].as_u64().ok_or("capacity")?)?;
            }
        }
        let fence = current_fence(&owner, session).await?;
        let outcome = owner
            .revision_sequencer
            .acquire(fence.character_id)
            .await
            .commit_wheel(
                &h.root,
                &authority,
                &h.node,
                fence,
                crate::durability::character_wheel::WheelChangeRequest {
                    occurrence: crate::durability::character_wheel::WheelOccurrence::from_bytes(
                        id(92),
                    )
                    .map_err(debug)?,
                    expected_wheel_revision: 0,
                    slots: crate::durability::character_wheel::WheelSlots::new(points)
                        .map_err(debug)?,
                    vessels: None,
                },
                wheel_rules,
                crate::durability::character_wheel::WheelChangeFacts { at_temple: false },
                &premium,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(
            outcome,
            crate::durability::character_wheel::WheelCommitOutcome::Committed(_)
        ));
    }
    // The caster's facts are the durable ones, so they equal the Character owner's.
    let CastFactsLoad::Ready { facts, .. } = load_character_cast_facts(
        &h.root,
        &authority,
        current_fence(&owner, session).await?.character_id,
    )
    .await
    else {
        return Err("durable caster facts".into());
    };
    for (key, level, mana) in plan.spells {
        assert!(i64::from(facts.level) >= i64::from(*level), "{key} level");
        assert!(i64::from(facts.max_mana) >= *mana, "{key} max mana");
    }
    let target;
    let position;
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
        let p = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        position = crate::foundation::MovementLocalPosition {
            x: p.x + 1,
            y: p.y,
            floor: p.floor,
        };
        runtime
            .install_companion_policies(native.policies().clone())
            .map_err(debug)?;
        let health = runtime
            .companion_policy("oteryn:creature.rat")
            .map_err(debug)?
            .maximum_health;
        runtime
            .realize_activation_spawns(vec![(
                vec![(position.x, position.y, position.floor)],
                60_000,
                5_000,
                "oteryn:creature.rat".into(),
                u64::try_from(health)?,
            )])
            .map_err(|error| format!("realize crosshair target spawn: {error:?}"))?;
        target = runtime
            .visible_entities()
            .creatures
            .first()
            .ok_or("rat")?
            .actor;
        let mut states = owner.states.spell_states.lock().await;
        assert!(
            states
                .initialize(&runtime, actor, session, facts, (0, 0), owner.owner_now())
                .is_some()
        );
        assert!(states.load_owned_build(&runtime, actor, session, &build));
        states
            .get_mut(&runtime, actor, session)
            .ok_or("caster")?
            .enable_owned_training(&build, formula)
            .map_err(debug)?;
    }
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&h.pool).await?;
    {
        let mut tx = h.pool.begin().await?;
        // Fixture custody only; tested equipment reads use the production fences.
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($1,'hex')::uuid)")
            .bind(id(112).as_slice()).bind(id(42).as_slice()).bind(plan.weapon).execute(&mut *tx).await?;
        // Slot 5 is the weapon hand the attack formulas read.
        sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$4,encode($3,'hex')::uuid,encode($3,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(112).as_slice()).bind(5_i16).execute(&mut *tx).await?;
        // Combat mode 1 is Attack; the attack formulas refuse an unknown mode.
        sqlx::query("UPDATE game_character_equipment_state SET combat_mode=1 WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
    }
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    h.root
        .initialize_familiar_group(&authority, &h.node, current_fence(&owner, session).await?)
        .await
        .map_err(debug)?;
    let current =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(session)
            .await?
            .0;
    admitted.item_fence = item_fence_of(current);
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[6, 7])
            .ok_or("chat capabilities")?;
    for (index, (key, _, mana)) in plan.spells.iter().enumerate() {
        let spell = (1..=deps.spells.source_len())
            .find_map(|i| {
                std::num::NonZeroU32::new(u32::try_from(i).ok()?).filter(|i| {
                    deps.spells
                        .source_indexed(*i)
                        .is_some_and(|(s, active)| active && s.key == *key)
                })
            })
            .ok_or("spell index")?;
        if index > 0 {
            // Clear the 2 s attack group cooldown of the previous cast.
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_secs(7))
                .ok_or("clock advance")?;
        }
        deps.runtime
            .lock()
            .await
            .set_test_creature_health(target, RAT_HEALTH)
            .map_err(debug)?;
        let before = owner
            .observe_actor_vitals(actor, session)
            .await
            .ok_or("before vitals")?
            .1
            .mana;
        if let Case::Outburst(charges) = plan.case {
            let runtime = deps.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let monk = states
                .get_mut(&runtime, actor, session)
                .ok_or("monk")?
                .monk
                .as_mut()
                .ok_or("Harmony owner")?;
            for _ in 0..charges {
                monk.commit_builder().map_err(debug)?;
            }
            assert_eq!(monk.harmony(), charges);
        }
        if plan.case == Case::Echo {
            for (n, target) in [
                actor_spell::SpellTarget::Position(actor_spell::SpellTargetPosition {
                    x: position.x,
                    y: position.y,
                    floor: position.floor + 1,
                }),
                actor_spell::SpellTarget::Position(actor_spell::SpellTargetPosition {
                    x: position.x + 8,
                    y: position.y,
                    floor: position.floor,
                }),
            ]
            .into_iter()
            .enumerate()
            {
                let refused = owner
                    .complete_actor_spell(
                        actor,
                        session,
                        1 + n as u64,
                        actor_spell::SpellCastIntent {
                            spell,
                            target,
                            aim_at_target: false,
                        },
                    )
                    .await
                    .ok_or("refused cast completed")?;
                assert_ne!(refused.disposition, actor_spell::SpellCastDisposition::Cast);
                assert_eq!(
                    owner
                        .observe_actor_vitals(actor, session)
                        .await
                        .ok_or("refusal mana")?
                        .1
                        .mana,
                    before
                );
            }
        }
        let outcome = owner
            .complete_actor_spell(
                actor,
                session,
                10 + index as u64,
                actor_spell::SpellCastIntent {
                    spell,
                    target: if matches!(plan.case, Case::Outburst(_)) {
                        actor_spell::SpellTarget::None
                    } else {
                        actor_spell::SpellTarget::Position(actor_spell::SpellTargetPosition {
                            x: position.x,
                            y: position.y,
                            floor: position.floor,
                        })
                    },
                    aim_at_target: false,
                },
            )
            .await
            .ok_or("cast completed")?;
        if matches!(plan.case, Case::Pz | Case::StageZero) {
            assert_ne!(outcome.disposition, actor_spell::SpellCastDisposition::Cast);
            let costs: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(costs, 0, "refusal writes no durable spell cost receipt");
            assert_eq!(
                owner
                    .observe_actor_vitals(actor, session)
                    .await
                    .ok_or("refused vitals")?
                    .1
                    .mana,
                before
            );
            let runtime = deps.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            assert!(
                states
                    .get(&runtime, actor, session)
                    .ok_or("caster")?
                    .cooldowns
                    .spells
                    .is_empty()
            );
            assert!(
                states
                    .spell_timers
                    .as_ref()
                    .is_none_or(|t| t.payloads.is_empty())
            );
            continue;
        }
        assert_eq!(
            outcome.disposition,
            actor_spell::SpellCastDisposition::Cast,
            "{key}"
        );
        let hit = deps
            .runtime
            .lock()
            .await
            .live_creature_health(target)
            .ok_or("rat survives")?;
        if plan.case == Case::Grenade {
            assert_eq!(hit, u64::try_from(RAT_HEALTH)?, "marker does no damage");
            {
                let runtime = deps.runtime.lock().await;
                let mut states = owner.states.spell_states.lock().await;
                let cues = states
                    .presentations
                    .as_mut()
                    .ok_or("presentation owner")?
                    .take_for_current_publisher(&runtime)
                    .map_err(debug)?;
                assert_eq!(cues.len(), 1);
                assert_eq!(
                    cues[0].event.source_binding,
                    "canary.appearance:effect/divine_grenade"
                );
                assert_eq!(cues[0].event.position, position);
            }
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_millis(2000))
                .ok_or("clock")?;
            let early = owner.drain_native_ai_timers().await.map_err(debug)?;
            assert!(
                early.receipts.is_empty(),
                "no explosion before three seconds"
            );
            assert_eq!(
                deps.runtime.lock().await.live_creature_health(target),
                Some(hit)
            );
        } else {
            assert!(hit < u64::try_from(RAT_HEALTH)?, "initial strike");
        }
        {
            let states = owner.states.spell_states.lock().await;
            let expected = if plan.case == Case::QueuedOrder {
                index + 1
            } else {
                usize::from(plan.case != Case::Outburst(4))
            };
            assert_eq!(
                states.spell_timers.as_ref().map_or(0, |t| t.payloads.len()),
                expected
            );
            if let Case::Outburst(_) = plan.case {
                assert_eq!(
                    states
                        .actors
                        .iter()
                        .find(|(a, _, _)| *a == actor)
                        .ok_or("monk")?
                        .2
                        .monk
                        .as_ref()
                        .ok_or("Harmony")?
                        .harmony(),
                    0
                );
            }
        }
        if plan.case == Case::QueuedOrder {
            if index == 0 {
                continue;
            }
            // Storage traversal order is not timer order. Two accepted casts stay
            // pending across a stalled tick; permuting payload storage must not jam them.
            owner
                .states
                .spell_states
                .lock()
                .await
                .spell_timers
                .as_mut()
                .ok_or("timers")?
                .payloads
                .reverse();
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_millis(1100))
                .ok_or("clock")?;
            let first = owner.drain_native_ai_timers().await.map_err(debug)?;
            assert_eq!(
                first.receipts.len(),
                1,
                "the earliest lane callback commits first"
            );
            let second = owner.drain_native_ai_timers().await.map_err(debug)?;
            assert_eq!(second.receipts.len(), 1);
            assert!(second.blocked.is_none());
            assert_eq!(
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .spell_timers
                    .as_ref()
                    .ok_or("timers")?
                    .pending_len(),
                0
            );
            assert!(
                deps.runtime
                    .lock()
                    .await
                    .live_creature_health(target)
                    .ok_or("rat")?
                    < hit
            );
            continue;
        }
        if plan.case == Case::Logout {
            // Native logout uses the existing terminal session/lease release. Clear fight time
            // without draining the pending callback; the offline timer must then be discarded.
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_secs(120))
                .ok_or("clock")?;
            assert!(matches!(
                owner.logout_session(admitted).await,
                crate::gameplay_transport::connection::LogoutReply::Result(
                    oteryn_protocol_oteryn::logout::LogoutResult::Accepted
                )
            ));
            let report = owner.drain_native_ai_timers().await.map_err(debug)?;
            assert!(report.receipts.is_empty());
            assert_eq!(
                deps.runtime.lock().await.live_creature_health(target),
                Some(hit)
            );
            assert!(
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .spell_timers
                    .as_ref()
                    .is_none_or(|t| t.payloads.is_empty())
            );
            continue;
        }
        if plan.case == Case::DepartedTarget {
            deps.runtime
                .lock()
                .await
                .remove_test_actor(target)
                .map_err(debug)?;
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_millis(1100))
                .ok_or("clock")?;
            let report = owner.drain_native_ai_timers().await.map_err(debug)?;
            assert!(report.blocked.is_none());
            assert!(report.receipts.is_empty());
            assert_eq!(report.dropped.len(), 1);
            assert_eq!(
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .spell_timers
                    .as_ref()
                    .ok_or("timers")?
                    .pending_len(),
                0
            );
            continue;
        }
        let expected_echo = if plan.case == Case::Echo {
            let runtime = deps.runtime.lock().await;
            let creature = runtime.companion_snapshot(target).map_err(debug)?;
            let mitigation = creature.state.policy.mitigation.ok_or("mitigation")?;
            let mitigation_percent =
                (mitigation.numerator as f64 / mitigation.denominator as f64).min(30.0);
            let resistance = creature
                .state
                .policy
                .resistances
                .iter()
                .find(|r| r.damage_type == "death")
                .map_or(0.0, |r| {
                    r.percent.numerator as f64 / r.percent.denominator as f64
                });
            let states = owner.states.spell_states.lock().await;
            let saved = states
                .spell_timers
                .as_ref()
                .ok_or("timers")?
                .payloads
                .iter()
                .find_map(|p| {
                    if let crate::spell::delayed_execution::TimerPayload::Strike(s) = &p.payload {
                        Some(s)
                    } else {
                        None
                    }
                })
                .ok_or("saved strike")?;
            let inputs = saved
                .binding
                .cast_snapshot
                .as_ref()
                .ok_or("cast snapshot")?
                .inputs;
            let p = &saved.binding.spell.spell()["execution"]["native_behavior"]["parameters"]["formula"];
            let evaluate = |key| -> TestResult<f64> {
                let e =
                    oteryn_gameplay_content::content::spell_data::authoring::expression(&p[key], 1)
                        .map_err(debug)?;
                e.evaluate(&inputs).map_err(debug)
            };
            let min = (evaluate("minimum")? * 0.5).trunc() as i64;
            let max = (evaluate("maximum")? * 0.5).trunc() as i64;
            use sha2::{Digest, Sha256};
            let hash = Sha256::new()
                .chain_update(b"oteryn:delayed-strike-due:v1")
                .chain_update(&saved.binding.parent_binding)
                .finalize();
            let mut bytes = [0; 16];
            bytes.copy_from_slice(&hash[..16]);
            let random = oteryn_simulation_determinism::deterministic_decision_u64(
                &oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(
                    native.source_digest(),
                ),
                oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(bytes),
                "spell.strike.due.draw",
                0,
            )
            .map_err(debug)?;
            let raw = crate::spell::uniform_draw(random, min, max) as f64;
            let after_mitigation = (raw - raw * mitigation_percent / 100.0).trunc();
            Some(
                (after_mitigation * (100.0 - resistance) / 100.0)
                    .round()
                    .max(0.0) as u64,
            )
        } else {
            None
        };
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_millis(1100))
            .ok_or("clock")?;
        let report = owner.drain_native_ai_timers().await.map_err(debug)?;
        assert!(
            report.blocked.is_none(),
            "due callback: {:?}",
            report.blocked
        );
        assert_eq!(
            report.receipts.len(),
            usize::from(plan.case != Case::Outburst(4)),
            "due occurrence"
        );
        let echo = deps
            .runtime
            .lock()
            .await
            .live_creature_health(target)
            .ok_or("rat survives delayed")?;
        if plan.case == Case::Outburst(4) {
            assert_eq!(echo, hit);
        } else {
            assert!(echo < hit, "delayed strike damages live area");
        }
        if let Some(expected) = expected_echo {
            assert_eq!(
                hit - echo,
                expected,
                "fresh roll uses exactly 50% formula bounds"
            );
        }
        let after = owner
            .observe_actor_vitals(actor, session)
            .await
            .ok_or("after vitals")?
            .1
            .mana;
        assert_eq!(i64::from(before) - i64::from(after), *mana, "{key} mana");
        let costs: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(costs, 1, "one durable payment for both occurrences");
    }
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}
