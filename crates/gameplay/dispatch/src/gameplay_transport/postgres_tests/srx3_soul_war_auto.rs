//! Mirror Image is reactive everywhere: its pinned Lua has no scene/zone predicate.
//! Use an actual pinned source map/native body and real Character writers;
//! no fictional boss recipe or source-coordinate authority is introduced.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, reset};
use crate::foundation::MovementLocalPosition;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

#[test]
fn srx3_mirror_actual_auto_attack_publishes_current_character_vocation_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("srx3-mirror-auto", || {
        run(async |admin| {
            reset();
            let scene = super::srx3_activity_source_fixture::SourceScene::new(
                crate::content::bosses::activity_template::PinnedActivitySourceRoom::GoshnarsGreed,
                MovementLocalPosition {
                    x: 33776,
                    y: 31665,
                    floor: 14,
                },
            )?;
            let room = &scene.room;
            let pin = &scene.pin;
            let deps = Dependencies::with_content_and_capacity(
                pin.clone(),
                room.clone(),
                crate::spell::cast::v1_spell_book().map_err(debug)?,
                8,
            )?;
            let active = scene
                .controller
                .active()
                .ok_or("actual native activation")?;
            let native = active
                .native_gameplay()
                .ok_or("actual full native gameplay")?;
            assert!(native.qualifies_soul_war_callbacks(native.source_digest()));
            let h = Harness::create_with_admission_seed(admin, "srx3_mirror_auto", false, false)
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
            super::srx3_activity_source_fixture::install_scene(
                &mut owner,
                &scene,
                crate::content::bosses::activity_template::PinnedActivitySourceRoom::GoshnarsGreed,
            )?;
            super::boss_activity::issue_content_and_reset(&h, pin).await?;
            let mut admitted = deps.admitted;
            super::srx3_activity_source_fixture::prepare_player(&h, &owner, &mut admitted, 500)
                .await?;
            let player = admitted.runtime_actor.ok_or("actual admitted player")?;
            let (image, target) = {
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
                        room.movement_cells().index().lookup(
                            room.movement_cells().scope(),
                            crate::content::LogicalCell {
                                x: at.x,
                                y: at.y,
                                z: i32::from(at.floor),
                            },
                        ) == Ok(crate::content::CollisionClass::Walkable)
                            && runtime.native_summon_cell_free(*at)
                    })
                    .ok_or("real adjacent walkable native cell")?;
                let image = runtime
                    .spawn_arena_opponent("oteryn:creature.mirror_image", at, |r, p| {
                        Some(p == at && r.native_summon_cell_free(p))
                    })
                    .map_err(debug)?;
                assert!(
                    runtime
                        .soul_war_health_callback(image)
                        .map_err(debug)?
                        .is_some(),
                    "actual bounded native source callback token"
                );
                let row = runtime
                    .visible_entities()
                    .creatures
                    .into_iter()
                    .find(|row| row.actor == image)
                    .ok_or("actual visible source image")?;
                (
                    image,
                    oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                        identity: image.placement_identity(),
                        generation: row.generation,
                    },
                )
            };
            assert_eq!(
                owner
                    .select_attack_target(player, admitted.game_session_id, 947, Some(target))
                    .await,
                oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
            );
            owner
                .refresh_attack_weapon(player, admitted.game_session_id)
                .await;
            for _ in 0..16 {
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(3))
                    .ok_or("semantic swing clock")?;
                owner
                    .refresh_attack_weapon(player, admitted.game_session_id)
                    .await;
                owner.drain_auto_attacks().await;
                if !owner.runtime.lock().await.contains_live_creature(image) {
                    break;
                }
            }
            {
                let runtime = owner.runtime.lock().await;
                assert!(
                    !runtime.contains_live_creature(image),
                    "real auto attack must resolve current native vocation and execute Mirror callback"
                );
                let children = runtime
                    .visible_entities()
                    .creatures
                    .into_iter()
                    .filter_map(|row| runtime.companion_snapshot(row.actor).ok())
                    .filter(|snapshot| {
                        snapshot
                            .state
                            .policy
                            .definition_key
                            .ends_with("_s_apparition")
                    })
                    .collect::<Vec<_>>();
                assert_eq!(children.len(), 1);
                assert_eq!(
                    children[0].health, children[0].maximum_health,
                    "original swing is lost; source birth is full native health"
                );
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })?;
    eprintln!(
        "SRX3 Mirror actual auto attack elapsed {:?}",
        started.elapsed()
    );
    Ok(())
}
