//! Actual pinned Greed entry and native spawn chain; no seeded catalogue.
use super::fixture::{Harness, debug, id, run};
use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};
use super::{Dependencies, TestResult, reset};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::foundation::{MovementLocalPosition, RuntimeScopeRefV1};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use oteryn_protocol_oteryn::boss_encounter as wire;

#[test]
fn srx3_greed_actual_source_entry_and_native_minion_chain_postgres() -> TestResult {
    run_source_entry(false)
}

pub(super) fn run_source_entry(retention: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("srx3-greed-native", move || {
        run(async move |admin| {
            reset();
            // Crystal201189f lib/quests/soul_war.lua:355-392: literal first
            // waiting tile and its real source map; no seeded catalogue recipe.
            let scene = SourceScene::new(
                PinnedActivitySourceRoom::GoshnarsGreed,
                MovementLocalPosition {
                    x: 33776,
                    y: 31665,
                    floor: 14,
                },
            )?;
            let deps: Dependencies = scene.dependencies()?;
            let h = Harness::create_with_admission_seed(admin, "srx3_greed_native", false, false)
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
            install_scene(&mut owner, &scene, PinnedActivitySourceRoom::GoshnarsGreed)?;
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
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
            prepare_player(&h, &owner, &mut admitted, 1999).await?;
            let instance = super::activity_runtime::allocate_ready_instance(&h, 216).await?;
            let source_actor = admitted.runtime_actor.ok_or("original physical player")?;
            let mut frames = Vec::new();
            let (_, reply, barrier) = super::boss_activity::exchange_capture(
                &owner,
                &mut admitted,
                "oteryn:creature.goshnar_s_greed",
                true,
                &mut frames,
            )
            .await?;
            assert!(
                matches!(reply, wire::BossActivityReply::Accepted { .. }),
                "actual Greed source entry: {reply:?}"
            );
            assert!(barrier.is_some());
            let fence = admitted.item_fence.ok_or("actual Instance fence")?;
            assert_eq!(fence.runtime_scope, instance);
            assert!(matches!(instance, RuntimeScopeRefV1::Instance { .. }));
            let registered = owner
                .activity_runtimes
                .get(instance, fence.scope_ownership_generation.get())
                .ok_or("actual source owner")?;
            {
                let view = owner.runtime_view(&registered);
                let permit = view
                    .spell_lane_permit()
                    .await
                    .ok_or("current destination lane")?;
                let actor = admitted.runtime_actor.ok_or("current physical entrant")?;
                let handoff = match reply {
                    wire::BossActivityReply::Accepted { handoff, .. } => handoff,
                    _ => return Err("accepted source handoff".into()),
                };
                let current = view
                    .current_soul_war_entry_clock_with_permit(
                        &permit,
                        actor,
                        admitted.game_session_id,
                        handoff,
                    )
                    .await
                    .map_err(|_| "current source entry clock")?
                    .ok_or("source clock row")?;
                assert!(current.0.observed_at_unix_ms >= current.0.opened_at_unix_ms);
                assert_eq!(
                    current.0.elapsed_ms,
                    u64::try_from(current.0.observed_at_unix_ms - current.0.opened_at_unix_ms)?
                );
                assert!(
                    view.current_soul_war_entry_clock_with_permit(
                        &permit,
                        actor,
                        admitted.game_session_id,
                        id(251)
                    )
                    .await
                    .map_err(|_| "wrong handoff clock read")?
                    .is_none(),
                    "foreign handoff supplies no source clock"
                );
                let wrong_session =
                    crate::foundation::GameSessionId::decode(&id(252)).map_err(debug)?;
                assert!(
                    view.current_soul_war_entry_clock_with_permit(
                        &permit,
                        actor,
                        wrong_session,
                        handoff
                    )
                    .await
                    .ok()
                    .flatten()
                    .is_none(),
                    "different current session supplies no source clock"
                );
                assert!(
                    view.current_soul_war_entry_clock_with_permit(
                        &permit,
                        source_actor,
                        admitted.game_session_id,
                        handoff
                    )
                    .await
                    .ok()
                    .flatten()
                    .is_none(),
                    "retired source-scope actor supplies no source clock"
                );
                // Hold the actual current journal row from another transaction.
                // A source-clock read must wait without holding the Runtime mutex.
                let mut blocker = h.pool.begin().await?;
                sqlx::query("SELECT 1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid FOR UPDATE")
                    .bind(admitted.game_session_id.as_bytes().as_slice()).execute(&mut *blocker).await?;
                let waiting_clock = view.current_soul_war_entry_clock_with_permit(
                    &permit,
                    actor,
                    admitted.game_session_id,
                    handoff,
                );
                tokio::pin!(waiting_clock);
                assert!(
                    tokio::time::timeout(std::time::Duration::from_millis(10), &mut waiting_clock)
                        .await
                        .is_err(),
                    "real journal lock keeps the current reader pending"
                );
                assert!(
                    view.runtime.try_lock().is_ok(),
                    "SQL waits retain only the destination lane, never the Runtime mutex"
                );
                blocker.rollback().await?;
                let after_wait = waiting_clock
                    .await
                    .map_err(|_| "source clock after real row wait")?
                    .ok_or("source clock after row wait")?;
                assert!(after_wait.0.elapsed_ms >= current.0.elapsed_ms);
            }
            if retention {
                super::srx3_greed_retention::exercise(&owner.runtime_view(&registered), &admitted)
                    .await?;
            } else {
                owner.runtime_view(&registered).drain_monster_melee().await;
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(10))
                    .ok_or("owner clock")?;
                let view = owner.runtime_view(&registered);
                view.drain_monster_melee().await;
                let runtime = view.runtime.lock().await;
                let creatures = runtime.visible_entities().creatures;
                for key in [
                    b"oteryn:creature.goshnar_s_greed".as_slice(),
                    b"oteryn:creature.greedbeast".as_slice(),
                    b"oteryn:creature.soulsnatcher".as_slice(),
                    b"oteryn:creature.weak_soul".as_slice(),
                ] {
                    assert_eq!(
                        creatures
                            .iter()
                            .filter(|c| runtime.matches_live_creature_identity(c.actor, key))
                            .count(),
                        1,
                        "literal onUseExtra→7/8/9s warning→10s native spawn: {:?}",
                        key
                    );
                }
                assert!(
                    creatures
                        .iter()
                        .all(|c| c.actor.runtime_scope() == instance)
                );
                drop(runtime);
                view.drain_monster_melee().await;
                let runtime = view.runtime.lock().await;
                assert_eq!(
                    runtime.visible_entities().creatures.len(),
                    creatures.len(),
                    "replayed scheduler drain creates no additional source actors"
                );
                drop(runtime);
                drop(view);
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })?;
    eprintln!(
        "SRX3_GREED_NATIVE_POSTGRES_CASE_EXECUTED retention={retention} duration={:?}",
        started.elapsed()
    );
    Ok(())
}
