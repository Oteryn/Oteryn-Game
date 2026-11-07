//! A real retained Lua Combat stage can precede its caster's queued onDeath.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult};
use crate::foundation::{CommandId, CommandRef, MovementLocalPosition};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_simulation_determinism::SemanticTimeMicros;

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
                    cast_at = Some(now);
                    break;
                }
            }
            let cast_at = cast_at
                .ok_or("actual native AI must select and schedule the three-stage Foam callback")?;
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
