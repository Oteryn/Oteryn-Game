//! Actual pinned Outburst BossLever enters through shared native command178.
use super::TestResult;
use super::fixture::{Harness, debug, run};
use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::foundation::{MovementLocalPosition, RuntimeScopeRefV1};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use oteryn_protocol_oteryn::boss_encounter as wire;
#[test]
fn srx3_outburst_actual_source_bosslever_enters_private_native_party_room_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("srx3-outburst-entry", || {
        run(async |admin| {
            super::reset();
            // actions_outburst_lever.lua:7 is the actual first waiting tile, not
            // an invented chamber admission or an administrative actor movement.
            let source = PinnedActivitySourceRoom::CallbackOutburst;
            let scene = SourceScene::new(
                source,
                MovementLocalPosition {
                    x: 32207,
                    y: 31284,
                    floor: 14,
                },
            )?;
            let active = scene
                .controller
                .active()
                .ok_or("actual Outburst active generation")?;
            let native = active
                .native_gameplay()
                .ok_or("actual Outburst native closure")?;
            native
                .reward_table()
                .row("oteryn:creature.outburst")
                .map_err(|error| format!("actual Outburst reward row: {error:?}"))?;
            let revision = native
                .training_formula()
                .ok_or("actual Outburst formula")?
                .content_revision();
            active
                .qualify_outburst_activity_template(&scene.room, revision)
                .map_err(|error| format!("actual Outburst source template: {error:?}"))?;
            let deps = scene.dependencies()?;
            let h = Harness::create_with_admission_seed(admin, "srx3_outburst_entry", false, false)
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
            install_scene(&mut owner, &scene, source)?;
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
            .ok_or("native Outburst capabilities")?;
            prepare_player(&h, &owner, &mut admitted, 2).await?;
            let destination = super::activity_runtime::allocate_ready_instance(&h, 231).await?;
            let mut frames = Vec::new();
            let (_, reply, barrier) = super::boss_activity::exchange_capture(
                &owner,
                &mut admitted,
                "oteryn:boss.outburst",
                true,
                &mut frames,
            )
            .await?;
            assert!(
                matches!(reply, wire::BossActivityReply::Accepted { .. }),
                "actual Outburst source lever must reach the production Instance handoff: {reply:?}"
            );
            assert!(barrier.is_some(), "actual native99 scope barrier");
            let fence = admitted
                .item_fence
                .ok_or("transferred source Outburst fence")?;
            assert_eq!(fence.runtime_scope, destination);
            assert!(matches!(destination, RuntimeScopeRefV1::Instance { .. }));
            let registered = owner
                .activity_runtimes
                .get(destination, fence.scope_ownership_generation.get())
                .ok_or("actual Outburst destination owner")?;
            let view = owner.runtime_view(&registered);
            let runtime = view.runtime.lock().await;
            let player = admitted.runtime_actor.ok_or("actual transferred Player")?;
            assert_eq!(
                runtime
                    .read_actor_position(player)
                    .map_err(debug)?
                    .position(),
                MovementLocalPosition {
                    x: 32234,
                    y: 31292,
                    floor: 14
                }
            );
            let actors = runtime.positioned_actor_census().map_err(debug)?;
            assert_eq!(
                actors
                    .iter()
                    .filter(|entry| runtime
                        .matches_live_creature_identity(entry.0, b"oteryn:creature.outburst"))
                    .count(),
                1
            );
            assert_eq!(
                actors
                    .iter()
                    .filter(|entry| runtime.matches_live_creature_identity(
                        entry.0,
                        b"oteryn:creature.spark_of_destruction"
                    ))
                    .count(),
                4,
                "literal initial roster belongs to the committed source entry"
            );
            drop(runtime);
            drop(view);
            drop(registered);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    });
    eprintln!(
        "SRX3_OUTBURST_ACTUAL_ENTRY_POSTGRES_CASE_EXECUTED duration={:?} passed={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
