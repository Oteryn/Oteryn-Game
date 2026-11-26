//! Exercise the authored Primal chair over the admitted boss command stream.
use super::fixture::{Harness, debug};
use super::srx3_activity_source_fixture::{SourceScene, prepare_player};
use super::{Dependencies, TestResult, reset};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::boss_activity::PRIMAL_ROOM_LOOKUP;
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::{SeamRooms, activity_runtime::source_bundle};
use oteryn_protocol_oteryn::boss_encounter as wire;
use std::time::Duration;
use tokio::sync::Notify;

#[test]
fn primal_lever_admits_from_authored_chair_as_runtime_role_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    primal_lever_scenario(admin, "w10_primal_runtime_role", None, true)
}

#[test]
fn primal_lever_admits_from_authored_chair_after_busy_lookup_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    primal_lever_scenario(
        admin,
        "w10_primal_busy",
        Some(Duration::from_millis(250)),
        true,
    )
}

#[test]
fn primal_lever_refuses_busy_lookup_without_closing_session_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    primal_lever_scenario(
        admin,
        "w10_primal_refusal",
        Some(Duration::from_millis(1_250)),
        false,
    )
}

fn primal_lever_scenario(
    admin: String,
    name: &'static str,
    hold_for: Option<Duration>,
    expect_switch: bool,
) -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |_| {
        reset();
        let source = PinnedActivitySourceRoom::Primal;
        let chair = MovementLocalPosition {
            x: 33548,
            y: 32752,
            floor: 14,
        };
        let (scene, activation_pin) = if hold_for.is_some() {
            SourceScene::new_with_activation(source, chair)?
        } else {
            let (scene, _, activation_pin) =
                super::srx3_soul_war_world_fixture_fixtures::source_world_with_bundle(chair)?;
            (scene, activation_pin)
        };
        let active = scene.controller.active().ok_or("active Primal content")?;
        let native = active.native_gameplay().ok_or("native Primal content")?;
        let private = if hold_for.is_some() {
            scene.room.clone()
        } else {
            let room = active
                .qualify_activity_source_room(&scene.room, source)
                .map_err(debug)?;
            let scope = super::fixture::fence(1)?.runtime_scope;
            let channel = scope.channel_id().ok_or("Primal source Channel")?;
            let private = source_bundle::qualify(&room, source, active, scope.world_id(), channel)
                .map_err(|_| "qualified private Primal room")?;
            assert_ne!(
                scene.pin.frame_binding_digest(),
                private.frame_binding().digest()
            );
            private
        };
        let template = active
            .qualify_primal_activity_template(&private, "content-1")
            .map_err(debug)?;
        let first = template
            .lever_positions()
            .first()
            .ok_or("authored Primal chair")?;
        assert_eq!(
            (first.x, first.y, first.z),
            (chair.x, chair.y, i32::from(chair.floor))
        );
        let h = Harness::create_with_admission_seed(admin, name, false, false).await?;
        let evidence = super::house_transfer_fixtures::RecoverySource::start(&h).await?;
        super::house_transfer_fixtures::prepare_owned_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::with_content(
            scene.pin.clone(),
            scene.room.clone(),
            native.spell_book().clone(),
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.evidence = &evidence.evidence;
        owner.active_generation = Some(active);
        owner.rooms = SeamRooms::new(Some(&scene.room), Some(&scene.room));
        owner
            .activity_runtimes
            .register_source_room(source, &private)
            .map_err(|_| "register pinned Primal room")?;
        let map_revision = crate::map::overlay::map_revision(
            owner
                .bundle_map()
                .ok_or("Primal source bundle map")?
                .overlay
                .base(),
        );
        super::boss_activity_fixtures::issue_content_and_reset_with_runtime_pin(
            &h,
            &activation_pin,
            &scene.pin,
            &map_revision,
        )
        .await?;
        let mut admitted = deps.admitted;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[wire::CAPABILITY_BOSS_ENCOUNTER_V1],
        )
        .ok_or("boss capability")?;
        prepare_player(&h, &owner, &mut admitted, template.minimum_level()).await?;
        super::item_look_fixtures::seed_main_backpack(&h, &[]).await?;
        let instance = super::activity_runtime_fixtures::allocate_ready_instance(&h, 209).await?;

        let before = admitted;
        let (_, reply, barrier) = if let Some(hold_for) = hold_for {
            let reached = std::sync::Arc::new(Notify::new());
            let held = std::sync::Arc::new(Notify::new());
            let hold = async {
                reached.notified().await;
                let pass = h.root.try_issue_semantic_pass()?;
                let held = held.clone();
                pass.run(move |_, _| {
                    Box::pin(async move {
                        held.notify_one();
                        tokio::time::sleep(hold_for).await;
                        Ok(())
                    })
                })
                .await
            };
            let exchange = PRIMAL_ROOM_LOOKUP.scope((reached.clone(), held.clone()), async {
                if expect_switch {
                    super::boss_activity_fixtures::exchange(
                        &owner,
                        &mut admitted,
                        template.creature_key(),
                        true,
                    )
                    .await
                } else {
                    super::boss_activity_fixtures::exchange_replayed_refusal(
                        &owner,
                        &mut admitted,
                        template.creature_key(),
                    )
                    .await
                }
            });
            let (result, holder_result) = tokio::join!(
                exchange,
                tokio::time::timeout(Duration::from_secs(30), hold)
            );
            holder_result??;
            result?
        } else {
            super::journal_inventory::runtime_role(&h.root).await?;
            super::boss_activity_fixtures::exchange(
                &owner,
                &mut admitted,
                template.creature_key(),
                true,
            )
            .await?
        };
        if expect_switch {
            let wire::BossActivityReply::Accepted { handoff, .. } = reply else {
                return Err(format!("Primal lever result: {reply:?}").into());
            };
            assert_eq!(barrier.ok_or("Instance handoff barrier")?.handoff, handoff);
            assert_eq!(
                admitted.item_fence.ok_or("Instance fence")?.runtime_scope,
                instance
            );
            assert_ne!(admitted.game_session_id, before.game_session_id);
        } else {
            assert_eq!(
                reply,
                wire::BossActivityReply::Refused(wire::BossActivityRefusal::ContentUnavailable)
            );
            assert!(barrier.is_none());
            assert_eq!(admitted.game_session_id, before.game_session_id);
            assert_eq!(
                admitted.continuity.next_command_id,
                before.continuity.next_command_id + 1
            );
        }
        drop(owner);
        drop(seal);
        h.cleanup().await
    })
}
