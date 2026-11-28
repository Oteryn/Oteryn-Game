//! C18c: delayed casts through real dispatch and fenced Character/Equipment owners.
use super::death_ml_loss_composed_fixtures::{current_fence, production_generation_with_source};
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
use oteryn_protocol_oteryn::{actor_spell, attack, world_spatial_entities::EntityRef};
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
    EchoCorners,
    EchoHeldTarget,
    Grenade,
    GrenadeBlocked,
    GrenadeHeld { position_cast: bool },
    Pz,
    StageZero,
    Outburst(u8),
    MonkBond { cue: &'static str, targeted: bool },
    Logout,
    DepartedTarget,
    Lethal { retire_slot_before_mint: bool },
    QueuedOrder,
}

#[test]
fn delayed_strikes_postgres_death_echo_two_hits() -> TestResult {
    echo(Case::Echo, "delayed_echo")
}

#[test]
fn delayed_strikes_postgres_death_echo_hits_all_four_corners_twice() -> TestResult {
    echo(Case::EchoCorners, "echo_corners")
}

#[test]
fn delayed_strikes_postgres_death_echo_centers_on_caster_without_cursor() -> TestResult {
    echo(Case::EchoHeldTarget, "echo_held")
}

fn echo(case: Case, tag: &'static str) -> TestResult {
    scenario(Plan {
        tag,
        vocation: "sorcerer",
        experience: 30_000_000,
        weapon: "oteryn:item.tibia.i3277",
        spells: &[("candidate:spell/death_echo", 120, 150)],
        case,
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
#[test]
fn delayed_strikes_postgres_grenade_rejects_solid_endpoint_without_cost_marker_or_timer() -> TestResult {
    grenade(Case::GrenadeBlocked, "delayed_grenade_solid")
}

#[test]
fn delayed_strikes_postgres_grenade_hotkey_uses_caster_and_position_cast_uses_aim() -> TestResult {
    grenade(
        Case::GrenadeHeld { position_cast: true },
        "delayed_grenade_aim",
    )?;
    grenade(
        Case::GrenadeHeld { position_cast: false },
        "delayed_grenade_hotkey",
    )
}

fn grenade(case: Case, tag: &'static str) -> TestResult {
    scenario(Plan {
        tag,
        vocation: "paladin",
        experience: 30_000_000_000,
        weapon: "oteryn:item.tibia.i3277",
        spells: &[("candidate:spell/divine_grenade", 300, 160)],
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
#[test]
fn delayed_strikes_postgres_outburst_repeats_the_cast_time_earth_bond() -> TestResult {
    outburst_with_weapon(5, "delayed_earth", "oteryn:item.tibia.i50167")
}

#[test]
fn monk_equipment_postgres_flurry_hits_with_the_earth_bond() -> TestResult {
    monk_bond_attack("monk_flurry", &[("candidate:spell/flurry_of_blows", 0, 110)], "green_flurryofblows", false)
}

#[test]
fn monk_equipment_postgres_sweeping_takedown_hits_with_the_earth_bond() -> TestResult {
    monk_bond_attack("monk_takedown", &[("candidate:spell/sweeping_takedown", 0, 195)], "green_explosionhit", false)
}

#[test]
fn monk_ordinary_postgres_mystic_repulse_uses_the_earth_bond() -> TestResult {
    monk_bond_attack(
        "monk_mystic",
        &[("candidate:spell/mystic_repulse", 30, 150)],
        "green_energypulse",
        true,
    )
}

fn monk_bond_attack(tag: &'static str, spells: &'static [(&'static str, u32, i64)], cue: &'static str, targeted: bool) -> TestResult {
    scenario(Plan {
        tag,
        vocation: "monk",
        experience: 30_000_000_000,
        weapon: "oteryn:item.tibia.i50167",
        spells,
        case: Case::MonkBond { cue, targeted },
    })
}

fn outburst(harmony: u8, tag: &'static str) -> TestResult {
    outburst_with_weapon(harmony, tag, "oteryn:item.tibia.i3277")
}

fn outburst_with_weapon(harmony: u8, tag: &'static str, weapon: &'static str) -> TestResult {
    scenario(Plan {
        tag,
        vocation: "monk",
        experience: 30_000_000_000,
        weapon,
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
fn delayed_strikes_postgres_lethal_damage_precedes_removal_and_corpse() -> TestResult {
    scenario(Plan {
        tag: "delayed_lethal",
        vocation: "sorcerer",
        experience: 30_000_000,
        weapon: "oteryn:item.tibia.i3277",
        spells: &[("candidate:spell/death_echo", 120, 150)],
        case: Case::Lethal { retire_slot_before_mint: false },
    })
}

#[test]
fn delayed_strikes_postgres_lethal_keeps_retired_actor_visible_until_corpse_mint() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async |admin| {
        scenario_inner(admin, Plan {
            tag: "delayed_lethal_retired",
            vocation: "sorcerer",
            experience: 30_000_000,
            weapon: "oteryn:item.tibia.i3277",
            spells: &[("candidate:spell/death_echo", 120, 150)],
            case: Case::Lethal { retire_slot_before_mint: true },
        }).await
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
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
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

async fn assert_monk_bond_cue(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    position: crate::foundation::MovementLocalPosition,
    expected: &str,
) -> TestResult {
    let runtime = deps.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    let cues = states
        .presentations
        .as_mut()
        .ok_or("presentation owner")?
        .take_for_current_publisher(&runtime)
        .map_err(debug)?;
    assert!(
        cues.iter().any(|cue| cue.event.position == position
            && cue.event.source_binding == format!("canary.appearance:effect/{expected}")),
        "Earth impact at the target: {cues:?}"
    );
    Ok(())
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
    } else if matches!(plan.case, Case::Lethal { .. }) {
        let map = crate::map::spell_tiles::tests::movement_arena::world();
        let cells = map.movement_cells(entry.movement_cells()).map_err(debug)?;
        entry.bundle_view(
            cells,
            map.frame_binding_digest(world),
            map.map_revision_digest(),
            crate::content::NativeEntryStart { x: 100, y: 100, floor: 7 },
        ).ok_or("lethal strike visibility room")?
    } else {
        entry
    };
    // Reuse crosshair_spells' open combat tile; the imported entry is a PZ.
    let start = match plan.case {
        Case::Pz | Case::Lethal { .. } => room.entry_start(),
        Case::EchoCorners => crate::content::NativeEntryStart {
            x: 1,
            y: 13,
            floor: -7,
        },
        Case::EchoHeldTarget => crate::content::NativeEntryStart {
            x: 0,
            y: 13,
            floor: -7,
        },
        Case::GrenadeBlocked => crate::content::NativeEntryStart {
            // Thalom's solid rock five tiles east has ground and does not block projectiles.
            x: 1,
            y: -10,
            floor: -7,
        },
        Case::GrenadeHeld { .. } => crate::content::NativeEntryStart {
            // Thalom has clear ground six cells east.
            x: -10,
            y: -9,
            floor: -7,
        },
        _ => crate::content::NativeEntryStart {
            // The outer Takedown row is four tiles east. Start one tile west
            // so its target stands on open ground at (5, 11), before the wall.
            x: if plan.tag == "monk_takedown" { 1 } else { 2 },
            y: 11,
            floor: -7,
        },
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
    let frame_digest = if matches!(plan.case, Case::Lethal { .. }) {
        room.frame_binding().digest()
    } else {
        pin.frame_binding().digest()
    };
    // Live spawns reserve half of these small fixtures' slots for players.
    let capacity = match plan.case {
        Case::EchoCorners => 12,
        Case::EchoHeldTarget => 6,
        _ => 3,
    };
    let mut deps = Dependencies::with_content_and_capacity(
        cp,
        room.clone(),
        native.spell_book().clone(),
        capacity,
    )?;
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
    if matches!(
        plan.case,
        Case::Grenade | Case::GrenadeBlocked | Case::GrenadeHeld { .. } | Case::Pz | Case::Outburst(_)
    ) {
        let purchase = super::atelier_fixtures::fixture::prepare(&h).await?;
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
    let target_ref;
    let position;
    let mut corners = Vec::new();
    let mut held_target = None;
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
        if matches!(plan.case, Case::MonkBond { targeted: false, .. }) {
            let before = runtime.read_actor_position(actor).map_err(debug)?;
            runtime
                .borrow_movement_position()
                .commit_turn(before, crate::foundation::MovementFacing::East)
                .map_err(debug)?;
        }
        let p = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        let offset_x = match plan.case {
            Case::EchoCorners | Case::EchoHeldTarget => -1,
            Case::GrenadeHeld { .. } => 6,
            _ => match plan.tag {
                "monk_flurry" => 3,
                "monk_takedown" => 4,
                _ => 1,
            },
        };
        let primary = crate::foundation::MovementLocalPosition {
            x: p.x + offset_x,
            y: p.y,
            floor: p.floor,
        };
        position = primary;
        let policies = if plan.weapon == "oteryn:item.tibia.i50167" {
            let mut rat = native.policies().policies().iter()
                .find(|policy| policy.definition_key == "oteryn:creature.rat")
                .ok_or("rat policy")?
                .as_ref()
                .clone();
            rat.damage_immunities = vec!["physical".into()];
            crate::content::creature_policy::CompiledCreaturePolicies::from_active_artifact(
                native.source_digest(),
                vec![rat],
            ).map_err(debug)?
        } else {
            native.policies().clone()
        };
        runtime.install_companion_policies(policies).map_err(debug)?;
        let health = runtime
            .companion_policy("oteryn:creature.rat")
            .map_err(debug)?
            .maximum_health;
        let mut spawns = vec![(primary.x, primary.y, primary.floor)];
        if plan.case == Case::EchoCorners {
            for (dx, dy) in [(-2, -2), (-2, 2), (2, -2), (2, 2)] {
                spawns.push((position.x + dx, position.y + dy, position.floor));
            }
        }
        if plan.case == Case::EchoHeldTarget {
            spawns.push((p.x + 3, p.y, p.floor));
        }
        runtime
            .realize_activation_spawns(vec![(
                spawns,
                60_000,
                5_000,
                "oteryn:creature.rat".into(),
                u64::try_from(health)?,
            )])
            .map_err(|error| format!("realize crosshair target spawn: {error:?}"))?;
        let mut primary_target = None;
        for row in runtime.visible_entities().creatures {
            let at = runtime
                .read_actor_position(row.actor)
                .map_err(debug)?
                .position();
            if at == primary {
                primary_target = Some((
                    row.actor,
                    EntityRef {
                        identity: row.actor.placement_identity(),
                        generation: row.generation,
                    },
                ));
            } else if plan.case == Case::EchoCorners {
                corners.push((row.actor, at));
            } else if plan.case == Case::EchoHeldTarget {
                held_target = Some((
                    row.actor,
                    oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                        identity: row.actor.placement_identity(),
                        generation: row.generation,
                    },
                ));
            }
        }
        (target, target_ref) = primary_target.ok_or("rat at primary position")?;
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
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(frame_digest.as_slice()).execute(&h.pool).await?;
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
    if matches!(plan.case, Case::Lethal { .. }) {
        super::item_look_fixtures::seed_main_backpack(&h, &[]).await?;
        owner.states.test_owner_time.store(owner.owner_now().get() + 1, std::sync::atomic::Ordering::Relaxed);
    }
    if matches!(plan.case, Case::MonkBond { targeted: true, .. }) {
        let entity = {
            let runtime = deps.runtime.lock().await;
            let visible = runtime.visible_entities();
            let row = visible.creatures.iter().find(|row| row.actor == target)
                .ok_or("cast target")?;
            oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                identity: target.placement_identity(),
                generation: row.generation,
            }
        };
        assert_eq!(
            owner.select_attack_target(actor, session, 9, Some(entity)).await,
            oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
        );
    }
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
        {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .set_test_creature_health(target, RAT_HEALTH)
                .map_err(debug)?;
            for &(corner, _) in &corners {
                runtime
                    .set_test_creature_health(corner, RAT_HEALTH)
                    .map_err(debug)?;
            }
            if let Some((held, _)) = held_target {
                runtime
                    .set_test_creature_health(held, RAT_HEALTH)
                    .map_err(debug)?;
            }
        }
        if plan.case == Case::EchoHeldTarget {
            let (_, entity) = held_target.ok_or("held rat")?;
            assert_eq!(
                owner
                    .select_attack_target(actor, session, 9, Some(entity))
                    .await,
                oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok,
            );
        }
        let before = owner
            .observe_actor_vitals(actor, session)
            .await
            .ok_or("before vitals")?
            .1
            .mana;
        if matches!(plan.case, Case::GrenadeBlocked | Case::GrenadeHeld { .. }) {
            let caster = crate::foundation::MovementLocalPosition {
                x: start.x,
                y: start.y,
                floor: start.floor,
            };
            let mut commands = Vec::new();
            if matches!(plan.case, Case::GrenadeHeld { .. }) {
                commands.push((
                    attack::COMMAND_TYPE_ATTACK_TARGET_INTENT,
                    attack::encode_attack_target_intent(Some(&target_ref)).map_err(debug)?,
                ));
                admitted.continuity.selected_capabilities =
                    SelectedCapabilities::select(owner.offered_capabilities(), &[6, 7, 17])
                        .ok_or("attack capabilities")?;
            }
            let blocked = crate::foundation::MovementLocalPosition {
                x: caster.x + 5,
                ..caster
            };
            let wire_position = |p: crate::foundation::MovementLocalPosition| {
                actor_spell::SpellTarget::Position(actor_spell::SpellTargetPosition {
                    x: p.x,
                    y: p.y,
                    floor: p.floor.abs(),
                })
            };
            let (intent_target, expected_marker) = match plan.case {
                Case::GrenadeBlocked => (wire_position(blocked), None),
                Case::GrenadeHeld { position_cast: true } => {
                    let aimed = crate::foundation::MovementLocalPosition {
                        // Aim within Divine Grenade's four-tile range, with the rat still held at six.
                        x: caster.x + 4,
                        ..position
                    };
                    (wire_position(aimed), Some(aimed))
                }
                _ => (actor_spell::SpellTarget::AttackTarget, Some(caster)),
            };
            commands.push((
                actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT,
                actor_spell::encode_spell_cast_intent(&actor_spell::SpellCastIntent {
                    spell,
                    target: intent_target,
                    aim_at_target: intent_target == actor_spell::SpellTarget::AttackTarget,
                }),
            ));
            let (results, _) =
                super::parameter_spell_fixtures::exchange(&owner, &mut admitted, &commands, false, false)
                    .await?;
            if results.len() == 2 {
                assert_eq!(
                    attack::decode_attack_intent_result(&results[0].1).map_err(debug)?,
                    attack::AttackIntentDisposition::Ok,
                    "the monster six tiles away is held"
                );
            }
            let disposition = actor_spell::decode_spell_cast_result(
                &results.last().ok_or("spell result")?.1,
            )
            .map_err(debug)?;
            assert_eq!(
                disposition == actor_spell::SpellCastDisposition::Cast,
                expected_marker.is_some(),
                "cast disposition: {disposition:?}"
            );
            let after = owner
                .observe_actor_vitals(actor, session)
                .await
                .ok_or("after vitals")?
                .1
                .mana;
            assert_eq!(
                i64::from(before) - i64::from(after),
                if expected_marker.is_some() { *mana } else { 0 }
            );
            let runtime = deps.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let markers = states
                .presentations
                .as_mut()
                .ok_or("presentation owner")?
                .take_for_current_publisher(&runtime)
                .map_err(debug)?;
            assert_eq!(markers.len(), usize::from(expected_marker.is_some()));
            if let Some(expected) = expected_marker {
                assert_eq!(
                    markers[0].event.source_binding,
                    "canary.appearance:effect/divine_grenade"
                );
                assert_eq!(markers[0].event.position, expected);
            }
            assert_eq!(
                states.spell_timers.as_ref().map_or(0, |t| t.payloads.len()),
                usize::from(expected_marker.is_some())
            );
            continue;
        }
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
                    target: if matches!(plan.case, Case::MonkBond { targeted: true, .. }) {
                        actor_spell::SpellTarget::AttackTarget
                    } else if matches!(plan.case, Case::Outburst(_) | Case::MonkBond { .. } | Case::EchoHeldTarget) {
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
        if let Some((held, _)) = held_target {
            assert_eq!(
                deps.runtime.lock().await.live_creature_health(held),
                Some(u64::try_from(RAT_HEALTH)?),
                "the held rat is outside the caster-centered strike",
            );
        }
        let mut corner_health = Vec::new();
        if plan.case == Case::EchoCorners {
            assert_eq!(corners.len(), 4);
            let runtime = deps.runtime.lock().await;
            for &(corner, at) in &corners {
                let hp = runtime
                    .live_creature_health(corner)
                    .ok_or("corner rat survives")?;
                assert!(
                    hp < u64::try_from(RAT_HEALTH)?,
                    "initial strike missed corner {at:?}"
                );
                corner_health.push((corner, at, hp));
            }
        }
        let hit = deps
            .runtime
            .lock()
            .await
            .live_creature_health(target)
            .ok_or("rat survives")?;
        if let Case::MonkBond { cue, .. } = plan.case {
            assert!(hit < u64::try_from(RAT_HEALTH)?, "bonded attack hits a Physical-immune target");
            assert_monk_bond_cue(&owner, &deps, position, cue).await?;
            let costs: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(costs, 1, "one durable payment for the bonded attack");
            continue;
        }
        if plan.weapon == "oteryn:item.tibia.i50167" {
            assert_monk_bond_cue(&owner, &deps, position, "green_energyshock").await?;
        }
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
            let mana_before_due = owner
                .observe_actor_vitals(actor, session)
                .await
                .ok_or("before departed target callback")?
                .1
                .mana;
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
            // Crystal 201189f combat.cpp:1525-1537, 1630-1670 executes an empty
            // area Combat and its tile/post effects; only current creatures get hits.
            // The positional echo therefore commits an empty receipt, not a drop.
            assert_eq!(report.receipts.len(), 1);
            let receipt = &report.receipts[0];
            assert_eq!(receipt.occurrence.phase, 0);
            assert!(receipt.batch.applied);
            assert_eq!(receipt.batch.caster, actor);
            assert!(receipt.batch.effects.is_empty(), "no hit, leech or proc");
            assert!(report.dropped.is_empty());
            assert_eq!(
                owner
                    .observe_actor_vitals(actor, session)
                    .await
                    .ok_or("after departed target callback")?
                    .1
                    .mana,
                mana_before_due
            );
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
        // Both promoted red-quadrant fixtures include slot 12's 0.25% mana
        // Leech. Check the paid cast before draining the independent due hit.
        let mana_leech_rate = if matches!(plan.case, Case::Grenade | Case::Outburst(_)) {
            25
        } else {
            0
        };
        let leech_for_damage = |damage: u64| -> TestResult<i64> {
            Ok(i64::try_from((damage * mana_leech_rate).div_ceil(10_000))?)
        };
        let initial_leech = leech_for_damage(u64::try_from(RAT_HEALTH)? - hit)?;
        let mana_after_cast = owner
            .observe_actor_vitals(actor, session)
            .await
            .ok_or("paid cast vitals")?
            .1
            .mana;
        assert_eq!(
            i64::from(before) - i64::from(mana_after_cast) + initial_leech,
            *mana,
            "{key} exact one-time cast payment before the delayed hit"
        );
        if let Case::Lethal { retire_slot_before_mint } = plan.case {
            deps.runtime
                .lock()
                .await
                .set_test_creature_health(target, 1)
                .map_err(debug)?;
            lethal_wire(&owner, admitted, target, position, retire_slot_before_mint).await?;
            if retire_slot_before_mint {
                let item = owner.runtime.lock().await
                    .bound_corpse(target.corpse_identity()).ok_or("published corpse")?
                    .item.item_instance_id;
                let mints: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM game_item_mint_receipts
                      WHERE item_instance_id=encode($1,'hex')::uuid
                        AND loot_purpose_key='CORPSE_MATERIALIZATION' AND materialized_at IS NOT NULL",
                ).bind(item.as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(mints, 1, "the wire corpse is backed by exactly one durable mint");
            }
            continue;
        }
        if plan.case == Case::Grenade {
            use crate::spell::delayed_execution::TimerPayload;
            use oteryn_gameplay_domain::domain::condition::ConditionValues;
            use oteryn_gameplay_foundation::foundation::condition::{
                ApplicationFacts, ConditionDefinition, ConditionSourceKind,
            };
            use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
            let runtime = deps.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let timer = states
                .spell_timers
                .as_ref()
                .and_then(|timers| timers.payloads.first())
                .ok_or("grenade timer")?;
            let TimerPayload::Strike(saved) = &timer.payload else {
                return Err("grenade strike payload".into());
            };
            let cast_at = saved.binding.cast_at.get();
            let snapshot = saved.binding.cast_snapshot.as_ref().ok_or("cast snapshot")?;
            assert_eq!(snapshot.stage, 3);
            let state = states.get_mut(&runtime, actor, session).ok_or("caster")?;
            assert_eq!(
                state.cooldowns.spells.get(*key).map(|ready| ready.get() - cast_at),
                Some(16_000_000),
                "paid grade-three cooldown"
            );
            // Make the live range clearly distinguishable from the planted range,
            // regardless of the damage and critical rolls.
            let magic_boost = ConditionDefinition::new(
                "fixture:grenade_magic_level",
                1,
                ConditionValues::SpellSkills {
                    duration_ms: 60_000,
                    sub_id: 0,
                    magic_level: 1000,
                    fist: 0,
                    melee: 0,
                    distance: 0,
                    shielding: 0,
                },
            )
            .ok_or("magic boost definition")?;
            let root = GameplayDecisionRoot::from_bytes(native.source_digest());
            state
                .conditions
                .apply(
                    &magic_boost,
                    None,
                    ConditionSourceKind::SelfUse,
                    &[],
                    &ApplicationFacts {
                        now: owner.owner_now().get(),
                        base_speed: 100,
                        mana_shield_capacity: 0,
                        target_reentry_protected: false,
                        source_reentry_protected: false,
                        target_is_player: true,
                        decision_root: &root,
                        occurrence: DecisionOccurrenceId::from_bytes(id(98)),
                    },
                )
                .map_err(debug)?;
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
            let random = |ordinal| {
                oteryn_simulation_determinism::deterministic_decision_u64(
                    &oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(
                        native.source_digest(),
                    ),
                    oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(bytes),
                    "spell.strike.due.draw",
                    ordinal,
                )
                .map_err(debug)
            };
            let mut raw = crate::spell::uniform_draw(random(0)?, min, max) as f64;
            // The second draw is the base player critical (no critical equipment here).
            let (chance, extra) =
                oteryn_gameplay_combat_rules::resolution::equipped_offence::EquippedOffence::default()
                    .player_critical();
            if crate::spell::uniform_draw(random(1)?, 1, 10000) <= i64::from(chance) {
                raw = (raw * (1.0 + f64::from(extra) / 10000.0)).trunc();
            }
            let after_mitigation = (raw - raw * mitigation_percent / 100.0).trunc();
            Some(
                (after_mitigation * (100.0 - resistance) / 100.0)
                    .round()
                    .max(0.0) as u64,
            )
        } else {
            None
        };
        let advance_ms = if matches!(plan.case, Case::EchoCorners | Case::EchoHeldTarget) {
            1000
        } else {
            1100
        };
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_millis(advance_ms))
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
        if plan.weapon == "oteryn:item.tibia.i50167" {
            assert_monk_bond_cue(&owner, &deps, position, "green_energyshock").await?;
        }
        if plan.case == Case::Grenade {
            assert!(
                hit - echo >= 4000,
                "fresh 1000 magic levels contribute 8000 minimum raw damage before mitigation, got {} HP loss",
                hit - echo
            );
        }
        {
            let runtime = deps.runtime.lock().await;
            for (corner, at, hp) in corner_health {
                assert!(
                    runtime
                        .live_creature_health(corner)
                        .ok_or("corner rat survives")?
                        < hp,
                    "echo missed corner {at:?}",
                );
            }
            if let Some((held, _)) = held_target {
                assert_eq!(
                    runtime.live_creature_health(held),
                    Some(u64::try_from(RAT_HEALTH)?),
                    "the held rat is outside the saved caster-centered echo",
                );
            }
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
        // A surviving target without Prey makes each physical HP loss the
        // independent pre-overkill basis. Round each occurrence separately.
        let delayed_leech = leech_for_damage(hit - echo)?;
        assert_eq!(
            i64::from(after),
            i64::from(mana_after_cast) + delayed_leech,
            "{key} due hit restores only its exact Leech and pays no second cost"
        );
        assert_eq!(
            i64::from(before) - i64::from(after) + initial_leech + delayed_leech,
            *mana,
            "{key} total payment with both source occurrences accounted for"
        );
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

async fn lethal_wire(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: crate::gameplay_transport::connection::AdmittedSession,
    target: crate::foundation::ExactActorRef,
    position: crate::foundation::MovementLocalPosition,
    retire_slot_before_mint: bool,
) -> TestResult {
    use super::fixture::join_two;
    use super::item_look_fixtures::read_frame;
    use crate::foundation::{MessageType, decode_wire_envelope};
    use crate::gameplay_transport::connection::{IDLE_LIVENESS, serve_admitted};
    use oteryn_protocol_oteryn::{
        decode_state_delta, presentation, world_spatial_entities as spatial,
    };

    let offers = crate::gameplay_transport::native_cue_offer::offered(
        owner.offered_capabilities(),
        owner.native_map_cue_source_frame().await.is_some(),
    );
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        &offers,
        &[4, 6, 7, 18, presentation::CAPABILITY_PRESENTATION_V1],
    )
    .ok_or("death observation capabilities")?;
    assert!(
        admitted
            .continuity
            .selected_capabilities
            .contains(presentation::CAPABILITY_PRESENTATION_V1)
    );
    let corpse_identity = target.corpse_identity();
    let target_ref = spatial::EntityRef {
        identity: target.placement_identity(),
        generation: target.actor_local_generation(),
    };
    let at = {
        let runtime = owner.runtime.lock().await;
        owner
            .wire_position_frame(&runtime)
            .and_then(|frame| frame.project(position))
            .ok_or("death position")?
    };
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let _ = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
    };
    let observer = async {
        loop {
            let bytes = read_frame(&mut client).await?;
            if decode_wire_envelope(&bytes).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        owner.states.test_owner_time.store(
            owner.owner_now().get() + 1_100_001,
            std::sync::atomic::Ordering::Relaxed,
        );
        let report = owner.drain_native_ai_timers().await.map_err(debug)?;
        assert_eq!(report.receipts.len(), 1, "pending strike must commit");
        assert!(report.receipts[0].batch.effects.iter().any(|effect| {
            effect.target == target
                && effect
                    .health
                    .is_some_and(|health| health.health_before == 1 && health.health_after == 0)
        }));
        if retire_slot_before_mint {
            {
                let mut runtime = owner.runtime.lock().await;
                assert!(runtime.bound_corpse(corpse_identity).is_none());
                runtime.remove_test_actor(target).map_err(debug)?;
                assert!(runtime.read_actor_position(target).is_err());
            }
            let frame = owner.observe_channel_entities(admitted.runtime_actor.ok_or("caster")?)
                .await.ok_or("pending death frame")?;
            assert!(frame.entities.iter().any(|entity| {
                entity.identity == target_ref.identity && entity.position == at
                    && entity.kind == crate::gameplay_transport::world_spatial::VisibleKind::PendingDeath
            }), "the exact Rat must stay in the frame while its corpse is unminted");
        }
        assert_eq!(
            owner.drain_kill_rewards(admitted.game_session_id).await,
            crate::gameplay_transport::kill_reward::DrainOutcome::Drained
        );
        let selected = admitted.continuity.selected_capabilities;
        // The production presentation outbox drains in order. A later sound
        // marks that the reader has passed any duplicate in that outbox.
        const END_CUE: u16 = 1;
        {
            let runtime = owner.runtime.lock().await;
            owner.states.spell_states.lock().await.sounds.record(
                &runtime,
                admitted.runtime_actor.ok_or("caster")?,
                position,
                END_CUE,
                0,
            );
        }
        let mut damage_sequence = None;
        let mut removed = false;
        let mut passed_outbox = false;
        loop {
            let bytes = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
            if envelope.message_type() != MessageType::StateDelta {
                continue;
            }
            let delta = decode_state_delta(envelope.payload()).map_err(debug)?;
            if delta.domain_id == presentation::STATE_DOMAIN_WORLD_PRESENTATION {
                for event in
                    presentation::decode_presentation_batch(delta.payload).map_err(debug)?
                {
                    if matches!(event.kind, presentation::PresentationEventKind::Sound { cue_id, .. } if cue_id == u32::from(END_CUE))
                    {
                        passed_outbox = true;
                    }
                    if let presentation::PresentationEventKind::ValueText {
                        target,
                        position,
                        primary,
                        ..
                    } = event.kind
                        && target == target_ref
                    {
                        assert!(
                            damage_sequence.is_none(),
                            "the lethal strike must publish once"
                        );
                        assert_eq!(
                            (position, primary.value, primary.color),
                            (at, 1, presentation::PresentationColor::DarkRed)
                        );
                        damage_sequence = Some(envelope.server_sequence());
                    }
                }
            }
            if delta.domain_id
                == oteryn_protocol_oteryn::world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY
            {
                let spatial = spatial::decode_world_spatial_delta_view(
                    selected.as_slice(),
                    delta.delta_type,
                    delta.payload,
                )
                .map_err(debug)?;
                if let spatial::WorldSpatialDeltaView::Entities(change) = spatial
                    && change.leave.contains(&target_ref)
                {
                    assert_eq!(
                        damage_sequence.and_then(|sequence| sequence.checked_add(1)),
                        Some(envelope.server_sequence()),
                        "the exact final damage must immediately precede the removal"
                    );
                    assert!(
                        change
                            .enter
                            .iter()
                            .any(|entry| entry.kind == spatial::EntityKind::Corpse
                                && entry.entity.identity == corpse_identity
                                && entry.position == at),
                        "the corpse must enter in the removal delta"
                    );
                    removed = true;
                }
            }
            if removed && passed_outbox {
                break;
            }
        }
        drop(client);
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    tokio::time::timeout(IDLE_LIVENESS.interval, async {
        join_two(serving, observer).await.1
    })
    .await?
}
