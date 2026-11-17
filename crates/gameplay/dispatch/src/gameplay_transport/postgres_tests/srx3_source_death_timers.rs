//! Real native HP publication enters the copied source-death dispatcher.
//! The test never invokes a Quest kill consumer or inserts an XP member.
use super::fixture::{Harness, debug, id, run};
use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};
use super::{Dependencies, TestResult, reset};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::foundation::{CommandId, CommandRef, MovementLocalPosition, RuntimeScopeRefV1};
use oteryn_protocol_oteryn::boss_encounter as wire;

#[test]
fn srx3_source_death_actual_malice_notice_runs_older_cage_timer_at_real_now_postgres() -> TestResult
{
    run_malice_source_death_timers(false)
}

#[test]
fn srx3_source_death_runs_original_taint_check_before_later_malice_callback_postgres() -> TestResult
{
    run_malice_source_death_timers(true)
}

fn run_malice_source_death_timers(require_taint_prefix: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("srx3-source-death-timers", move || {
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
            let h = Harness::create_with_admission_seed(
                admin,
                "srx3_source_death_timers",
                false,
                false,
            )
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
            let catalogue =
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?;
            let source_rule = format!(
                "{}remorse",
                oteryn_gameplay_quests::quest::soul_war::source_items::TRANSITION_PREFIX
            );
            assert!(
                catalogue.catalogue().transition(&source_rule).is_some(),
                "the real embedded Soul War Quest catalogue must be installed before source work"
            );
            owner.quest_catalogue = Some(std::sync::Arc::new(catalogue.catalogue().clone()));
            install_scene(&mut owner, &scene, PinnedActivitySourceRoom::GoshnarsMalice)?;
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            let mut admitted = deps.admitted;
            let supported = [
                4,
                6,
                7,
                10,
                11,
                12,
                15,
                17,
                18,
                23,
                30,
                92,
                wire::CAPABILITY_BOSS_ENCOUNTER_V1,
                385,
            ];
            admitted.continuity.selected_capabilities =
                crate::gameplay_transport::connection::fresh_capability_selection(
                    &owner, &supported,
                )
                .await
                .ok_or("actual native source capability selection")?;
            assert!(admitted.continuity.selected_capabilities.contains(30));
            prepare_player(&h, &owner, &mut admitted, 1999).await?;
            let instance = super::activity_runtime::allocate_ready_instance(&h, 216).await?;
            let mut frames = Vec::new();
            let (_, reply, barrier) = super::boss_activity::exchange_capture(
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
            let player = admitted.runtime_actor.ok_or("actual transferred Player")?;
            let native = scene
                .controller
                .active()
                .and_then(|active| active.native_gameplay())
                .ok_or("actual full native gameplay")?;
            if !require_taint_prefix {
                owner.runtime_view(&registered).drain_monster_melee().await;
            }
            // The real lever owns a 23s Cage timer. Let it become due without
            // draining it, then publish a genuine later canonical HP death.
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(24))
                .ok_or("owner clock")?;
            let view = owner.runtime_view(&registered);
            let mut runtime = view.runtime.lock().await;
            assert_eq!(
                native.source_digest(),
                runtime.content_pin().server_artifact_digest(),
                "actual source timer and HP owner share the installed native generation"
            );
            let states = view.states.spell_states.try_lock().map_err(debug)?;
            let cutoff = view.owner_now().get();
            let original_due = states
                .soul_war_callbacks
                .malice
                .as_ref()
                .ok_or("actual Malice clock")?
                .next_due_ms()
                .ok_or("actual original 23s timer")?;
            assert!(original_due.checked_mul(1000).ok_or("timer arithmetic")? <= cutoff);
            if require_taint_prefix {
                let taint_due = states
                    .soul_war_callbacks
                    .next_taint_one_due()
                    .ok_or("the real source birth retained its first two-second taint check")?;
                assert!(taint_due < original_due.checked_mul(1000).ok_or("timer arithmetic")?);
                assert!(taint_due <= cutoff);
            }
            assert!(
                !runtime
                    .visible_entities()
                    .creatures
                    .iter()
                    .any(|row| runtime
                        .matches_live_creature_identity(row.actor, b"oteryn:creature.soul_cage")),
                "the original source callback has not executed yet"
            );
            let boss = runtime
                .visible_entities()
                .creatures
                .into_iter()
                .find(|row| {
                    runtime.matches_live_creature_identity(
                        row.actor,
                        b"oteryn:creature.goshnar_s_malice",
                    )
                })
                .ok_or("actual native source Malice")?
                .actor;
            let health = runtime.companion_snapshot(boss).map_err(debug)?.health;
            let hit = runtime
                .borrow_exact_actor_commit_at_us(cutoff)
                .commit_damage_for_bound_attacker(
                    boss,
                    player,
                    CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(32201).map_err(debug)?,
                    ),
                    0,
                    crate::foundation::OwnerDamageCommand {
                        target: b"oteryn:creature.goshnar_s_malice",
                        occurrence: b"source-timer-before-death",
                        binding: b"source-timer-before-death-current",
                        damage: health,
                    },
                )
                .map_err(debug)?;
            assert!(hit.applied && hit.health_before > 0 && hit.health_after == 0);
            let notice = runtime
                .next_source_creature_death()
                .map_err(debug)?
                .ok_or("real canonical death notice")?;
            assert_eq!(notice.actor(), boss);
            assert_eq!(notice.enqueued_at_us(), Some(cutoff));
            drop(states);
            drop(runtime);
            drop(view);
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(46))
                .ok_or("owner clock")?;
            let view = owner.runtime_view(&registered);
            let before_execution = view.owner_now().get();
            if require_taint_prefix {
                view.drain_monster_melee().await;
                let runtime = view.runtime.lock().await;
                let states = view.states.spell_states.try_lock().map_err(debug)?;
                assert!(
                    states
                        .soul_war_callbacks
                        .next_taint_one_due()
                        .is_none_or(|due| due > cutoff),
                    "the real current Quest read completes only the original two-second check"
                );
                assert_eq!(
                    states
                        .soul_war_callbacks
                        .malice
                        .as_ref()
                        .ok_or("actual retained Malice owner")?
                        .next_due_ms(),
                    Some(original_due),
                    "the later Cage callback must retain its original deadline"
                );
                assert_eq!(
                    runtime
                        .next_source_creature_death()
                        .map_err(debug)?
                        .ok_or("the same copied death remains behind the older Cage")?
                        .death_key(),
                    notice.death_key()
                );
                assert!(
                    !runtime
                        .visible_entities()
                        .creatures
                        .iter()
                        .any(|row| runtime.matches_live_creature_identity(
                            row.actor,
                            b"oteryn:creature.soul_cage"
                        )),
                    "draining TaintOne must not execute a fresh Think or the later Cage timer"
                );
            }
            view.drain_monster_melee().await;
            let after_execution = view.owner_now().get();
            let runtime = view.runtime.lock().await;
            let states = view.states.spell_states.try_lock().map_err(debug)?;
            assert_eq!(
                runtime
                    .visible_entities()
                    .creatures
                    .iter()
                    .filter(|row| runtime
                        .matches_live_creature_identity(row.actor, b"oteryn:creature.soul_cage"))
                    .count(),
                1,
                "older real source timer must execute before the canonical death head"
            );
            let successor = states
                .soul_war_callbacks
                .malice
                .as_ref()
                .ok_or("retained source clock")?
                .next_due_ms()
                .ok_or("real callback successor")?;
            assert!(
                successor >= before_execution / 1000 + 40000
                    && successor <= after_execution / 1000 + 40000,
                "Lua successor delay starts at actual execution, not the old cutoff"
            );
            assert!(successor > cutoff / 1000);
            assert_eq!(
                runtime
                    .next_source_creature_death()
                    .map_err(debug)?
                    .ok_or("copied Malice goal still pending")?
                    .death_key(),
                notice.death_key()
            );
            drop(states);
            drop(runtime);
            view.drain_monster_melee().await;
            let runtime = view.runtime.lock().await;
            assert_eq!(
                runtime
                    .visible_entities()
                    .creatures
                    .iter()
                    .filter(|row| runtime
                        .matches_live_creature_identity(row.actor, b"oteryn:creature.soul_cage"))
                    .count(),
                1,
                "repeating the owner pass cannot consume a future removal"
            );
            drop(runtime);
            drop(view);
            drop(owner);
            drop(authority);
            drop(seal);
            drop(evidence);
            h.cleanup().await
        })
    })?;
    eprintln!(
        "SRX3_SOURCE_NOTICE_TIMER_CASE_EXECUTED duration={:?}",
        started.elapsed()
    );
    Ok(())
}
