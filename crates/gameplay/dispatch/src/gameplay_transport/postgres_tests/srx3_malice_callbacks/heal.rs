//! Actual Malice/Cage production dispatch, after the real source entry fixture.
use super::super as pg;
use super::*;

// Crystal201189f data/scripts/spells/healing/heal_malice.lua:1-16:
// targetcallback heals only namedMalice15000..30000, while CombatNull dispels
// paralysis on all eligible area creatures, including nearby players.
fn seed_player_paralysis(
    runtime: &mut crate::foundation::ChannelRuntimeV1,
    states: &mut crate::gameplay_transport::actor_spell::ChannelSpellStates,
    player: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    now: u64,
) -> TestResult {
    let speed = u16::try_from(
        states
            .get(runtime, player, session)
            .ok_or("actual player state")?
            .owned_base_speed(),
    )?;
    let definition = crate::foundation::ConditionDefinition::new(
        "srx3.heal-malice.paralysis",
        1,
        crate::foundation::ConditionValues::Speed {
            paralysis: true,
            duration_ms: 60_000,
            range: crate::foundation::SpeedRange {
                a_min: 0,
                a_max: 0,
                b_min: -100,
                b_max: -100,
            },
        },
    )
    .ok_or("fixture paralysis")?;
    let stamp = runtime.issue_owner_work().map_err(debug)?;
    let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(
        runtime.content_pin().server_artifact_digest(),
    );
    let facts = crate::foundation::ApplicationFacts {
        now,
        base_speed: speed,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes([0xa7; 16]),
    };
    assert!(states.install_owned_player_self_use_test_condition(
        runtime,
        runtime.owner_fence().map_err(debug)?,
        stamp,
        player,
        session,
        &definition,
        &facts
    ));
    assert!(
        states
            .get(runtime, player, session)
            .ok_or("current player")?
            .owned_conditions()
            .instances()
            .iter()
            .any(|condition| condition.definition().condition_type()
                == crate::foundation::ConditionType::Paralysis)
    );
    Ok(())
}

fn inside_current_cage_area(
    runtime: &crate::foundation::ChannelRuntimeV1,
    states: &crate::gameplay_transport::actor_spell::ChannelSpellStates,
    cage: crate::foundation::ExactActorRef,
    target: crate::foundation::ExactActorRef,
) -> TestResult<bool> {
    use crate::content::creature_attack_geometry::{self as geometry, Facing};
    let facing = match states
        .monster_melee
        .creature_ai
        .state(cage)
        .ok_or("actual Cage AI facing")?
        .attack_facing()
    {
        crate::movement::CardinalStep::North => Facing::North,
        crate::movement::CardinalStep::East => Facing::East,
        crate::movement::CardinalStep::South => Facing::South,
        crate::movement::CardinalStep::West => Facing::West,
        crate::movement::CardinalStep::NorthEast | crate::movement::CardinalStep::NorthWest => Facing::North,
        crate::movement::CardinalStep::SouthEast | crate::movement::CardinalStep::SouthWest => Facing::South,
    };
    let source = runtime.read_actor_position(cage).map_err(debug)?;
    let target = runtime.read_actor_position(target).map_err(debug)?;
    let (dx, dy) = geometry::step(facing);
    let center = (source.position().x + dx, source.position().y + dy);
    // Exact AREA_CIRCLE3X3 from the pinned healing callback, not a radius
    // approximation. needDirection=true moves its center one tile forward.
    let area = crate::content::ProjectV2AbilityArea::Matrix {
        north: [
            "..xxx..", ".xxxxx.", "xxxxxxx", "xxxCxxx", "xxxxxxx", ".xxxxx.", "..xxx..",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        diagonal: Vec::new(),
    };
    let offsets = geometry::offsets(&area, facing).map_err(debug)?;
    Ok(source.context() == target.context()
        && source.position().floor == target.position().floor
        && offsets.contains(&(
            target.position().x - center.0,
            target.position().y - center.1,
        )))
}

#[test]
fn srx3_heal_malice_native_defense_heals_named_boss_and_dispels_nearby_player_postgres()
-> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    pg::kill_reward_e2e::scenario_thread::run("srx3-malice-area-heal", || {
        run(async |admin| {
            reset();
            // Crystal201189f lib/quests/soul_war.lua:332-354: literal first
            // waiting tile and its real source map; no seeded catalogue recipe.
            let scene = SourceScene::new(
                PinnedActivitySourceRoom::GoshnarsMalice,
                MovementLocalPosition {
                    x: 33679,
                    y: 31599,
                    floor: 14,
                },
            )?;
            let deps: Dependencies = scene.dependencies()?;
            let h =
                Harness::create_with_admission_seed(admin, "srx3_malice_area_heal", false, false)
                    .await?;
            // Imported prior quest access is a historical fixture fact, following
            // srx3_taints. New admission/callback writes keep all normal guards.
            let mut historical = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *historical)
                .await?;
            sqlx::query("INSERT INTO game_character_soul_war(character_id,teleport_access,defeated_bosses,first_taint_at,megalomania_completed,final_reward_claimed) VALUES(encode($1,'hex')::uuid,true,0,NULL,false,false)")
                .bind(id(41).as_slice()).execute(&mut *historical).await?;
            historical.commit().await?;
            let evidence = pg::house_transfer::RecoverySource::start(&h).await?;
            pg::house_transfer::prepare_owned_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            install_scene(&mut owner, &scene, PinnedActivitySourceRoom::GoshnarsMalice)?;
            pg::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            let mut admitted = deps.admitted;
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
                &[
                    4,
                    6,
                    7,
                    10,
                    11,
                    15,
                    18,
                    23,
                    92,
                    wire::CAPABILITY_BOSS_ENCOUNTER_V1,
                ],
            )
            .ok_or("actual native boss capabilities")?;
            // Use the same supported native level as the other Malice scenarios.
            prepare_player(&h, &owner, &mut admitted, 1999).await?;
            let instance = pg::activity_runtime::allocate_ready_instance(&h, 215).await?;
            let mut frames = Vec::new();
            let (_, reply, barrier) = pg::boss_activity::exchange_capture(
                &owner,
                &mut admitted,
                "oteryn:creature.goshnar_s_malice",
                true,
                &mut frames,
            )
            .await?;
            assert!(
                matches!(reply, wire::BossActivityReply::Accepted { .. }),
                "actual Malice source entry: {reply:?}"
            );
            assert!(barrier.is_some());
            let fence = admitted.item_fence.ok_or("actual Instance fence")?;
            assert_eq!(fence.runtime_scope, instance);
            assert!(matches!(instance, RuntimeScopeRefV1::Instance { .. }));
            let registered = owner
                .activity_runtimes
                .get(instance, fence.scope_ownership_generation.get())
                .ok_or("actual source owner")?;
            owner.runtime_view(&registered).drain_monster_melee().await;
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(23))
                .ok_or("owner clock")?;
            let view = owner.runtime_view(&registered);
            view.drain_monster_melee().await;
            let runtime = view.runtime.lock().await;
            let creatures = runtime.visible_entities().creatures;
            let boss = creatures
                .iter()
                .find(|creature| {
                    runtime.matches_live_creature_identity(
                        creature.actor,
                        b"oteryn:creature.goshnar_s_malice",
                    )
                })
                .ok_or("actual Malice")?
                .actor;
            let cage = creatures
                .iter()
                .find(|creature| {
                    runtime.matches_live_creature_identity(
                        creature.actor,
                        b"oteryn:creature.soul_cage",
                    )
                })
                .ok_or("actual source Cage")?
                .actor;
            let player = admitted.runtime_actor.ok_or("actual Instance player")?;
            let start = runtime
                .read_actor_position(player)
                .map_err(debug)?
                .position();
            let cage_position = runtime.read_actor_position(cage).map_err(debug)?.position();
            let occupied = creatures
                .iter()
                .map(|creature| {
                    runtime
                        .read_actor_position(creature.actor)
                        .map(|p| p.position())
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(debug)?;
            let route = pg::srx3_primal_terminal::source_route(
                view.effective_movement_cells(),
                start,
                MovementLocalPosition {
                    x: cage_position.x,
                    y: cage_position.y + 1,
                    floor: cage_position.floor,
                },
                &occupied,
                false,
            )?;
            drop(runtime);
            drop(view);
            for direction in route {
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(1))
                    .ok_or("paced source clock")?;
                let view = owner.runtime_view(&registered);
                let (step, _) = view
                    .ordinary_paced_step(player, admitted.game_session_id, direction, None, None)
                    .await;
                assert_eq!(
                    step.disposition,
                    oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved,
                    "actual source walk to Cage healing area"
                );
            }
            let view = owner.runtime_view(&registered);
            let mut runtime = view.runtime.lock().await;
            let maximum = runtime
                .companion_snapshot(boss)
                .map_err(debug)?
                .maximum_health;
            let wounded = maximum.checked_sub(50_000).ok_or("source boss HP")?;
            runtime
                .set_test_creature_health(boss, wounded)
                .map_err(debug)?;
            let mut states = view.states.spell_states.lock().await;
            assert!(
                inside_current_cage_area(&runtime, &states, cage, player)?,
                "real paced player must enter source Cage area before condition setup"
            );
            // Nonlethal injury is the bounded HEAL test precondition only. The
            // native definition/maxHP and damage contribution ledger stay intact.
            assert!(
                runtime
                    .borrow_combat_death()
                    .current_session_damage_shares(boss)
                    .map_err(debug)?
                    .is_empty()
            );
            seed_player_paralysis(
                &mut runtime,
                &mut states,
                player,
                admitted.game_session_id,
                view.owner_now().get(),
            )?;
            drop(states);
            drop(runtime);
            drop(view);
            let mut selected = false;
            for _ in 0..12 {
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(2))
                    .ok_or("owner clock")?;
                let view = owner.runtime_view(&registered);
                let tick = view.owner_now();
                {
                    let mut runtime = view.runtime.lock().await;
                    let mut states = view.states.spell_states.lock().await;
                    runtime
                        .set_test_creature_health(boss, wounded)
                        .map_err(debug)?;
                    seed_player_paralysis(
                        &mut runtime,
                        &mut states,
                        player,
                        admitted.game_session_id,
                        tick.get(),
                    )?;
                }
                view.drain_monster_melee_at(tick).await;
                let runtime = view.runtime.lock().await;
                let states = view.states.spell_states.lock().await;
                let proposal = states
                    .monster_melee
                    .creature_ai
                    .last_report_for_test(cage)
                    .is_some_and(|report| {
                        report.proposals.iter().any(|proposal| {
                            proposal.ability.key == "oteryn:ability.spell.heal_malice"
                        })
                    });
                let simultaneous_self_heal = states
                    .monster_melee
                    .creature_ai
                    .last_report_for_test(boss)
                    .is_some_and(|report| {
                        report.proposals.iter().any(|proposal| {
                            proposal.ability.key
                                == "oteryn:ability.creature.goshnar_s_malice.defense-2"
                        })
                    });
                if !proposal || simultaneous_self_heal {
                    continue;
                }
                assert!(
                    inside_current_cage_area(&runtime, &states, cage, player)?,
                    "selected native HealMalice must actually contain current player"
                );
                assert!(
                    inside_current_cage_area(&runtime, &states, cage, boss)?,
                    "selected native HealMalice must actually contain current Malice"
                );
                selected = true;
                let after = i64::try_from(
                    runtime
                        .live_creature_health(boss)
                        .ok_or("live source boss")?,
                )?;
                assert!(
                    (wounded + 15_000..=wounded + 30_000).contains(&after),
                    "selected actual Cage native defense must heal named Malice15000..30000, HP={after} before={wounded}"
                );
                assert!(
                    !states
                        .get(&runtime, player, admitted.game_session_id)
                        .ok_or("current source player")?
                        .owned_conditions()
                        .instances()
                        .iter()
                        .any(|condition| condition.definition().condition_type()
                            == crate::foundation::ConditionType::Paralysis),
                    "source nonaggressive CombatNull dispels player paralysis independently of named-Malice HPselector"
                );
                drop(states);
                drop(runtime);
                view.drain_monster_melee_at(tick).await;
                assert_eq!(
                    view.runtime.lock().await.live_creature_health(boss),
                    Some(u64::try_from(after)?),
                    "same native occurrence cannot heal twice"
                );
                break;
            }
            assert!(
                selected,
                "actual scheduled HealMalice defense must reach dispatch"
            );
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })?;
    eprintln!(
        "SRX3_HEAL_MALICE_NATIVE_POSTGRES_CASE_EXECUTED duration={:?}",
        started.elapsed()
    );
    Ok(())
}
