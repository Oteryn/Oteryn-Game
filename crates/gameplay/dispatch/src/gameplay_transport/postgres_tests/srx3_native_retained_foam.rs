//! A real retained Lua Combat stage can precede its caster's queued onDeath.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult};
use crate::foundation::{CommandId, CommandRef, MovementLocalPosition};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_simulation_determinism::SemanticTimeMicros;

fn enter_retained_stage_area(
    runtime: &mut crate::foundation::ChannelRuntimeV1,
    room: &crate::content::QualifiedNativeEntryRoom,
    player: crate::foundation::ExactActorRef,
    tiles: &[(i32, i32, i16)],
) -> TestResult {
    let qualified = tiles
        .iter()
        .copied()
        .filter(|&(x, y, floor)| {
            let cell = crate::content::LogicalCell {
                x,
                y,
                z: i32::from(floor),
            };
            room.movement_cells()
                .index()
                .lookup(room.movement_cells().scope(), cell)
                == Ok(crate::content::CollisionClass::Walkable)
                && room
                    .movement_cells()
                    .spell_tiles()
                    .lookup(room.movement_cells().scope(), cell)
                    .is_ok_and(|tile| {
                        let flags = tile.flags();
                        !flags.protection_zone
                            && !flags.floor_change
                            && !flags.block_solid
                            && !flags.block_projectile
                    })
                && runtime
                    .position_occupied_by_other(player, MovementLocalPosition { x, y, floor })
                    .is_ok_and(|occupied| !occupied)
        })
        .collect::<Vec<_>>();
    if qualified.is_empty() {
        return Err("actual first Foam stage has no qualified free Player cell".into());
    }
    let distance = |at: MovementLocalPosition| {
        qualified
            .iter()
            .filter(|(_, _, floor)| *floor == at.floor)
            .map(|(x, y, _)| u64::from(at.x.abs_diff(*x)) + u64::from(at.y.abs_diff(*y)))
            .min()
    };
    for _ in 0..4 {
        let before = runtime.read_actor_position(player).map_err(debug)?;
        let at = before.position();
        let remaining = distance(at).ok_or("Player and retained stage share a floor")?;
        if remaining == 0 {
            break;
        }
        use crate::movement::CardinalStep;
        let step = [
            (CardinalStep::North, 0, -1),
            (CardinalStep::East, 1, 0),
            (CardinalStep::South, 0, 1),
            (CardinalStep::West, -1, 0),
        ]
        .into_iter()
        .filter_map(|(step, dx, dy)| {
            let next = MovementLocalPosition {
                x: at.x.checked_add(dx)?,
                y: at.y.checked_add(dy)?,
                floor: at.floor,
            };
            let next_distance = distance(next)?;
            (next_distance < remaining
                && room.movement_cells().index().lookup(
                    room.movement_cells().scope(),
                    crate::content::LogicalCell {
                        x: next.x,
                        y: next.y,
                        z: i32::from(next.floor),
                    },
                ) == Ok(crate::content::CollisionClass::Walkable)
                && runtime
                    .position_occupied_by_other(player, next)
                    .is_ok_and(|occupied| !occupied))
            .then_some((next_distance, step))
        })
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, step)| step)
        .ok_or("qualified cardinal route into the actual retained Foam cross")?;
        let selection = crate::movement::MovementEngineeringSelection {
            owner_context: before.context(),
            content_scope: room.movement_cells().scope(),
        };
        crate::movement::step_cardinal(
            &mut runtime.borrow_movement_position(),
            player,
            before,
            &selection,
            room.movement_cells().index(),
            step,
        )
        .map_err(debug)?;
    }
    let at = runtime
        .read_actor_position(player)
        .map_err(debug)?
        .position();
    assert!(
        qualified.contains(&(at.x, at.y, at.floor)),
        "the actual Player must occupy the retained first stage's qualified cross-cell"
    );
    Ok(())
}

#[test]
fn srx3_native_foam_older_stage_runs_with_registered_hp0_caster_before_death_postgres() -> TestResult
{
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("srx3-native-retained-foam", || {
        run(async |admin| {
            super::reset();
            let kind =
                crate::content::bosses::activity_template::PinnedActivitySourceRoom::BrainHead;
            let scene = super::srx3_activity_source_fixture::SourceScene::new(
                kind,
                MovementLocalPosition {
                    x: 31953,
                    y: 32325,
                    floor: 10,
                },
            )?;
            let deps = Dependencies::with_content_and_capacity(
                scene.pin.clone(),
                scene.room.clone(),
                crate::spell::cast::v1_spell_book().map_err(debug)?,
                8,
            )?;
            let active = scene
                .controller
                .active()
                .ok_or("actual native activation")?;
            let native = active.native_gameplay().ok_or("actual native gameplay")?;
            const CREATURE: &str = "oteryn:creature.foam_stalker";
            const ABILITY: &str = "oteryn:ability.creature.foam_stalker.field-fill-foamsplash";
            let record = native
                .creature_profiles()
                .records
                .iter()
                .find(|record| record.profile.target.key == CREATURE)
                .ok_or("actual unmodified Foam body")?;
            let crate::content::ProjectV2AuthoringProfileData::Behavior(behavior) =
                &record.behavior.as_ref().ok_or("actual Foam behavior")?.data
            else {
                return Err("Foam behavior family".into());
            };
            let index = behavior
                .attacks
                .iter()
                .position(|entry| entry.ability.key == ABILITY)
                .ok_or("actual Foam callback slot")?;
            let source=oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_cast::SourceCallbackCast::qualify_native(
                native,&record.profile.target,index,crate::foundation::profile_schedule::ScheduleList::Attack)
                .map_err(debug)?.ok_or("sealed actual Foam source")?;
            assert_eq!(
                source
                    .stages()
                    .iter()
                    .map(|stage| stage.0)
                    .collect::<Vec<_>>(),
                vec![1000, 2000, 3000]
            );

            let h = Harness::create_with_admission_seed(
                admin,
                "srx3_native_retained_foam",
                false,
                false,
            )
            .await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            super::srx3_activity_source_fixture::install_scene(&mut owner, &scene, kind)?;
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            let mut admitted = deps.admitted;
            super::srx3_activity_source_fixture::prepare_player(&h, &owner, &mut admitted, 2000)
                .await?;
            let player = admitted.runtime_actor.ok_or("actual admitted Player")?;
            let caster = {
                let mut runtime = owner.runtime.lock().await;
                native
                    .install_companion_policies(&mut runtime)
                    .map_err(debug)?;
                let origin = runtime
                    .read_actor_position(player)
                    .map_err(debug)?
                    .position();
                let at = [(1, 0), (0, 1), (-1, 0), (0, -1)]
                    .into_iter()
                    .map(|(x, y)| MovementLocalPosition {
                        x: origin.x + x,
                        y: origin.y + y,
                        floor: origin.floor,
                    })
                    .find(|at| {
                        scene.room.movement_cells().index().lookup(
                            scene.room.movement_cells().scope(),
                            crate::content::LogicalCell {
                                x: at.x,
                                y: at.y,
                                z: i32::from(at.floor),
                            },
                        ) == Ok(crate::content::CollisionClass::Walkable)
                            && runtime.native_summon_cell_free(*at)
                    })
                    .ok_or("actual adjacent cross-area source cell")?;
                runtime
                    .spawn_arena_opponent(CREATURE, at, |r, p| {
                        Some(p == at && r.native_summon_cell_free(p))
                    })
                    .map_err(debug)?
            };
            owner.clock_origin -= std::time::Duration::from_secs(10);
            let base = owner.owner_now().get();
            let mut cast_at = None;
            for tick in 0..128_u64 {
                let now = base + tick * 1_000_000;
                owner
                    .drain_monster_melee_at(SemanticTimeMicros::from_micros(now))
                    .await;
                let runtime = owner.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                let selected = states
                    .monster_melee
                    .creature_ai
                    .last_report_for_test(caster)
                    .is_some_and(|report| {
                        report.proposals.iter().any(|p| p.ability.key == ABILITY)
                    });
                if selected
                    && states
                        .native_callbacks
                        .source_cast_status_for_test(caster)
                        .1
                {
                    assert!(runtime.contains_live_creature(caster));
                    assert_eq!(
                        states
                            .native_callbacks
                            .source_stage_done_for_test(caster)
                            .await,
                        Some(vec![false, false, false])
                    );
                    // This owner turn finishes Movement before scheduling the native
                    // callback, whose SourcePositionAnchor retains this exact pose.
                    cast_at = Some((
                        now,
                        runtime
                            .read_actor_position(caster)
                            .map_err(debug)?
                            .position(),
                    ));
                    break;
                }
            }
            let (cast_at, cast_position) = cast_at
                .ok_or("actual native AI must select and schedule the three-stage Foam callback")?;
            {
                let mut runtime = owner.runtime.lock().await;
                let first = &source.stages().first().ok_or("actual first Foam stage")?.1;
                let tiles = first
                    .target_tiles(cast_position, cast_position, None)
                    .map_err(debug)?;
                // Native Foam keeps distance2 while this first stage is only a
                // radius1 cross. Enter its saved area after the real cast selection.
                enter_retained_stage_area(&mut runtime, &scene.room, player, &tiles)?;
                assert_eq!(
                    runtime
                        .read_actor_position(caster)
                        .map_err(debug)?
                        .position(),
                    cast_position,
                    "no owner driver may move the caster after its stage anchor was retained"
                );
            }
            let lethal_at = cast_at + 1_500_000;
            let before = {
                let mut runtime = owner.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                let before = states
                    .get(&runtime, player, admitted.game_session_id)
                    .ok_or("actual Player HP owner")?
                    .vitals()
                    .health;
                assert!(
                    before > 300,
                    "actual classed Player must survive the isolated first source stage"
                );
                let maximum = runtime
                    .companion_snapshot(caster)
                    .map_err(debug)?
                    .maximum_health;
                let lease = runtime
                    .bound_attacker_lease(player, admitted.game_session_id)
                    .map_err(debug)?;
                let result = runtime
                    .borrow_exact_actor_commit_at_us(lethal_at)
                    .commit_damage_for_bound_attacker(
                        caster,
                        player,
                        CommandRef::new(
                            admitted.game_session_id,
                            CommandId::new(25601).map_err(debug)?,
                        ),
                        0,
                        crate::foundation::OwnerDamageCommand {
                            target: CREATURE.as_bytes(),
                            occurrence: b"srx3-retained-foam-lethal",
                            binding: b"srx3-retained-foam-lethal-v1",
                            damage: maximum,
                        },
                    )
                    .map_err(debug)?;
                assert!(
                    result.applied && result.health_after == 0,
                    "canonical lethal commit, never a health-zero setter"
                );
                assert!(
                    runtime
                        .borrow_combat_death()
                        .committed_lethal_receipt(caster)
                        .is_ok()
                );
                assert!(
                    runtime
                        .source_creature_is_registered(caster)
                        .map_err(debug)?,
                    "HP0 source caster remains registered before its queued death"
                );
                let notice = runtime
                    .next_source_creature_death()
                    .map_err(debug)?
                    .ok_or("canonical deferred source death notice")?;
                assert_eq!(notice.actor(), caster);
                assert_eq!(notice.enqueued_at_us(), Some(lethal_at));
                let queue = owner.states.attack.lock().await.kills().clone();
                crate::gameplay_transport::kill_reward::record_creature_kill(
                    &mut runtime,
                    crate::gameplay_transport::kill_reward::KillSink {
                        prey: None,
                        activity: &[],
                        queue: &queue,
                        rewards: native.reward_table(),
                        charms: None,
                    },
                    caster,
                    crate::combat::CapturedRewardPrincipal {
                        character: lease.character_id(),
                        lease_generation: lease.generation(),
                        session: admitted.game_session_id,
                        actor: player,
                    },
                    lethal_at / 1000,
                    Some(SemanticTimeMicros::from_micros(lethal_at)),
                );
                assert!(
                    runtime
                        .source_creature_is_registered(caster)
                        .map_err(debug)?,
                    "ordinary projected-death admission must retain the actual source caster until its queued task"
                );
                assert_eq!(
                    runtime
                        .next_source_creature_death()
                        .map_err(debug)?
                        .ok_or("retained source notice")?
                        .actor(),
                    caster
                );
                before
            };
            // Deliberately no owner driver ran between scheduling and this lethal.
            // At real3.5s, only the original1s task predates the1.5s death notice.
            owner
                .drain_monster_melee_at(SemanticTimeMicros::from_micros(cast_at + 3_500_000))
                .await;
            let (after, done) = {
                let runtime = owner.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                (
                    states
                        .get(&runtime, player, admitted.game_session_id)
                        .ok_or("actual post-stage Player HP")?
                        .vitals()
                        .health,
                    states
                        .native_callbacks
                        .source_stage_done_for_test(caster)
                        .await,
                )
            };
            assert_eq!(
                done,
                Some(vec![true, false, false]),
                "only the original1s native callback may execute before source death"
            );
            assert!(
                after < before && before - after <= 300,
                "older real Foam stage must damage its current Player audience with registeredHP0 caster"
            );
            assert!(
                !owner
                    .runtime
                    .lock()
                    .await
                    .source_creature_is_registered(caster)
                    .map_err(debug)?,
                "actual empty source death task must complete before checking cancellation of later callbacks"
            );
            owner
                .drain_monster_melee_at(SemanticTimeMicros::from_micros(cast_at + 4_500_000))
                .await;
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            assert_eq!(
                states
                    .get(&runtime, player, admitted.game_session_id)
                    .ok_or("actual post-death Player HP")?
                    .vitals()
                    .health,
                after,
                "the later2s/3s Lua callbacks cannot run after the source death boundary"
            );
            drop(states);
            drop(runtime);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}
