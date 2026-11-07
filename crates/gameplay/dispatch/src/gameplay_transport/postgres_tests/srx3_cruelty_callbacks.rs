//! Actual pinned Cruelty entry and native defense callback; no seeded catalogue.
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
fn srx3_cruelty_actual_source_entry_and_native_defense_chain_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("srx3-cruelty-native", || {
        run(async |admin| {
            reset();
            // Crystal201189f lib/quests/soul_war.lua:422-448: literal first
            // waiting tile and its real source map; no seeded catalogue recipe.
            let scene = SourceScene::new(
                PinnedActivitySourceRoom::GoshnarsCruelty,
                MovementLocalPosition {
                    x: 33854,
                    y: 31854,
                    floor: 6,
                },
            )?;
            let deps: Dependencies = scene.dependencies()?;
            let h = Harness::create_with_admission_seed(admin, "srx3_cruelty_native", false, false)
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
            install_scene(
                &mut owner,
                &scene,
                PinnedActivitySourceRoom::GoshnarsCruelty,
            )?;
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

            let mut frames = Vec::new();
            let (_, reply, barrier) = super::boss_activity::exchange_capture(
                &owner,
                &mut admitted,
                "oteryn:creature.goshnar_s_cruelty",
                true,
                &mut frames,
            )
            .await?;
            assert!(
                matches!(reply, wire::BossActivityReply::Accepted { .. }),
                "actual Cruelty source entry: {reply:?}"
            );
            assert!(barrier.is_some());
            let fence = admitted.item_fence.ok_or("actual Instance fence")?;
            assert_eq!(fence.runtime_scope, instance);
            assert!(matches!(instance, RuntimeScopeRefV1::Instance { .. }));
            let registered = owner
                .activity_runtimes
                .get(instance, fence.scope_ownership_generation.get())
                .ok_or("actual source owner")?;
            let view = owner.runtime_view(&registered);
            let boss = {
                let runtime = view.runtime.lock().await;
                runtime
                    .visible_entities()
                    .creatures
                    .into_iter()
                    .find(|c| {
                        runtime.matches_live_creature_identity(
                            c.actor,
                            b"oteryn:creature.goshnar_s_cruelty",
                        )
                    })
                    .ok_or("actual source Cruelty actor")?
                    .actor
            };
            let initial_defense = {
                let runtime = view.runtime.lock().await;
                runtime
                    .effective_creature_defense(&runtime.companion_snapshot(boss).map_err(debug)?)
                    .map_err(debug)?
            };
            drop(view);
            // Advance actual current native turns, never a wall-time shortcut to
            // the source accumulated 15,000 ms onThink counter.
            for _ in 0..16 {
                owner.runtime_view(&registered).drain_monster_melee().await;
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(1))
                    .ok_or("owner clock")?;
            }
            let view = owner.runtime_view(&registered);
            let runtime = view.runtime.lock().await;
            let current = runtime.companion_snapshot(boss).map_err(debug)?;
            assert_eq!(
                runtime
                    .effective_creature_defense(&current)
                    .map_err(debug)?,
                initial_defense + 2,
                "actual native source 15s helper raises defense by two once"
            );
            drop(runtime);
            drop(view);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })?;
    eprintln!(
        "SRX3_CRUELTY_NATIVE_POSTGRES_CASE_EXECUTED duration={:?}",
        started.elapsed()
    );
    Ok(())
}
