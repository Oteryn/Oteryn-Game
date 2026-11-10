//! The real Eye cast owns an uncredited Poor Soul death and its plain Item body.
use super::TestResult;
use super::fixture::{Harness, debug, id, run};
use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use oteryn_protocol_oteryn::boss_encounter as wire;

#[test]
fn srx3_cruelty_native_eye_uncredited_poor_soul_corpse_is_pickupable_and_replay_bound_postgres()
-> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("srx3-cruelty-uncredited-corpse", || {
        run(async |admin| {
            super::reset();
            let kind = PinnedActivitySourceRoom::GoshnarsCruelty;
            let scene = SourceScene::new(
                kind,
                MovementLocalPosition {
                    x: 33854,
                    y: 31854,
                    floor: 6,
                },
            )?;
            let deps = scene.dependencies()?;
            let h =
                Harness::create_with_admission_seed(admin, "srx3_cruelty_uncredited", false, false)
                    .await?;
            // Historical access only; entry, death, MINT and pickup use the current owners.
            let mut historical = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *historical)
                .await?;
            sqlx::query("INSERT INTO game_character_soul_war(character_id,teleport_access,defeated_bosses,first_taint_at,megalomania_completed,final_reward_claimed) VALUES(encode($1,'hex')::uuid,true,0,NULL,false,false)")
                .bind(id(41).as_slice()).execute(&mut *historical).await?;
            historical.commit().await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            super::item_look::seed_main_backpack(&h, &[]).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            owner.quest_catalogue = Some(std::sync::Arc::new(
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?
                    .catalogue()
                    .clone(),
            ));
            install_scene(&mut owner, &scene, kind)?;
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
            .ok_or("actual boss capabilities")?;
            prepare_player(&h, &owner, &mut admitted, 1999).await?;
            let instance = super::activity_runtime::allocate_ready_instance(&h, 217).await?;
            let (_, reply, barrier) = super::boss_activity::exchange_capture(
                &owner,
                &mut admitted,
                "oteryn:creature.goshnar_s_cruelty",
                true,
                &mut Vec::new(),
            )
            .await?;
            assert!(
                matches!(reply, wire::BossActivityReply::Accepted { .. }),
                "actual source entry: {reply:?}"
            );
            assert!(barrier.is_some());
            let fence = admitted.item_fence.ok_or("actual Instance fence")?;
            assert_eq!(fence.runtime_scope, instance);
            let registered = owner
                .activity_runtimes
                .get(instance, fence.scope_ownership_generation.get())
                .ok_or("actual source owner")?;
            let player = admitted.runtime_actor.ok_or("actual source Player")?;
            let (eye, soul) = {
                let view = owner.runtime_view(&registered);
                let mut runtime = view.runtime.lock().await;
                let eye = runtime
                    .current_live_creature_census()
                    .map_err(debug)?
                    .into_iter()
                    .find(|actor| {
                        runtime
                            .matches_live_creature_identity(*actor, b"oteryn:creature.a_greedy_eye")
                    })
                    .ok_or("literal lever Eye")?;
                assert_eq!(
                    runtime.read_actor_position(eye).map_err(debug)?.position(),
                    MovementLocalPosition {
                        x: 33856,
                        y: 31858,
                        floor: 7
                    }
                );
                assert_eq!(
                    runtime
                        .read_actor_position(player)
                        .map_err(debug)?
                        .position(),
                    MovementLocalPosition {
                        x: 33856,
                        y: 31872,
                        floor: 7
                    }
                );
                let cells = view.effective_movement_cells();
                // Move around the actual boss through the real current position owner.
                // Place a native Poor Soul directly between the stationary Eye and Player;
                // this tests the cast/death path, not the separate XML respawn timer.
                for step in std::iter::once(crate::movement::CardinalStep::West)
                    .chain(std::iter::repeat_n(
                        crate::movement::CardinalStep::North,
                        12,
                    ))
                    .chain(std::iter::once(crate::movement::CardinalStep::East))
                {
                    let before = runtime.read_actor_position(player).map_err(debug)?;
                    let selection = crate::movement::MovementEngineeringSelection {
                        owner_context: before.context(),
                        content_scope: cells.scope(),
                    };
                    crate::movement::step_cardinal(
                        &mut runtime.borrow_movement_position(),
                        player,
                        before,
                        &selection,
                        cells.index(),
                        step,
                    )
                    .map_err(debug)?;
                }
                assert_eq!(
                    runtime
                        .read_actor_position(player)
                        .map_err(debug)?
                        .position(),
                    MovementLocalPosition {
                        x: 33856,
                        y: 31860,
                        floor: 7
                    }
                );
                let at = MovementLocalPosition {
                    x: 33856,
                    y: 31859,
                    floor: 7,
                };
                let soul = runtime
                    .spawn_arena_opponent("oteryn:creature.poor_soul", at, |_, at| {
                        cells
                            .index()
                            .lookup(
                                cells.scope(),
                                crate::content::LogicalCell {
                                    x: at.x,
                                    y: at.y,
                                    z: i32::from(at.floor),
                                },
                            )
                            .ok()
                            .map(|class| class == crate::content::CollisionClass::Walkable)
                    })
                    .map_err(debug)?;
                assert_eq!(
                    runtime.companion_snapshot(soul).map_err(debug)?.health,
                    1000,
                    "unchanged native Poor Soul HP"
                );
                (eye, soul)
            };
            let ability = "oteryn:ability.creature.a_greedy_eye.optional-field-core-attacks-1";
            let mut selected = false;
            let mut dead = false;
            // Source interval is 2s at 100%; include the initial native scheduling turn.
            for _ in 0..4 {
                let view = owner.runtime_view(&registered);
                view.drain_monster_melee().await;
                let runtime = view.runtime.lock().await;
                let states = view.states.spell_states.lock().await;
                if let Some(report) = states.monster_melee.creature_ai.last_report_for_test(eye) {
                    selected |= report
                        .proposals
                        .iter()
                        .any(|proposal| proposal.ability.key == ability);
                }
                dead = runtime
                    .companion_snapshot_including_dead(soul)
                    .map_err(debug)?
                    .health
                    == 0;
                drop(states);
                drop(runtime);
                drop(view);
                if dead {
                    break;
                }
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(1))
                    .ok_or("native owner clock")?;
            }
            assert!(selected, "actual native Eye proposal must be selected");
            assert!(
                dead,
                "its source cast must kill the native 1000HP Poor Soul"
            );
            let view = owner.runtime_view(&registered);
            {
                let mut runtime = view.runtime.lock().await;
                let notice = runtime
                    .next_source_creature_death()
                    .map_err(debug)?
                    .ok_or("actual empty-vector death")?;
                assert_eq!(notice.actor(), soul);
                let begin = runtime
                    .begin_source_creature_death(&notice)
                    .map_err(debug)?;
                assert!(begin.events().is_empty());
                assert!(
                    !begin.has_reward_damage_credit(),
                    "no invented Player principal for Eye damage"
                );
                assert_eq!(
                    begin.uncredited_corpse_item_key(),
                    Some("oteryn:item.tibia.i33891")
                );
            }
            super::srx3_native_source_cast::drain_source_death(&view, soul, owner.owner_now())
                .await?;
            assert!(
                !view.runtime.lock().await.corpse_projection_pinned(soul),
                "physical ACK releases the one source pin"
            );
            let ground = view.root.read_item_loop_ground_for_scope(instance).await?;
            let bodies: Vec<_> = ground
                .iter()
                .filter(|item| item.item.definition.production_key == "oteryn:item.tibia.i33891")
                .collect();
            assert_eq!(bodies.len(), 1, "one uncredited physical plain Item");
            let body = bodies[0];
            assert!(body.source_plain_corpse && body.field_collision.is_none());
            assert!(
                view.runtime.lock().await.visible_corpses().is_empty(),
                "plain Mortal Essence is not a container"
            );
            let item = body.item.item_instance_id;
            let receipts: Vec<(String, String, String)> = sqlx::query_as("SELECT event_id::text,transaction_id::text,item_instance_id::text FROM game_item_mint_receipts WHERE loot_purpose_key='CORPSE_MATERIALIZATION' ORDER BY event_id")
                .fetch_all(&h.pool).await?;
            assert_eq!(receipts.len(), 1);
            let command = crate::gameplay_transport::connection::UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 1000,
                item_fence: Some(fence),
            };
            let movement = crate::gameplay_transport::item_move::LoopMove {
                item: crate::gameplay_transport::item_view::ItemKey::Instance(item),
                count: None,
                target: crate::gameplay_transport::item_move::LoopTarget::MainBackpack,
            };
            assert_eq!(
                crate::gameplay_transport::loop_move::move_loop_item(
                    &view, player, command, movement
                )
                .await,
                crate::gameplay_transport::item_move::LoopMoveResult::Committed
            );
            assert_eq!(
                crate::gameplay_transport::loop_move::move_loop_item(
                    &view, player, command, movement
                )
                .await,
                crate::gameplay_transport::item_move::LoopMoveResult::Committed,
                "same actual pickup wire replays"
            );
            view.drain_monster_melee_at(owner.owner_now()).await;
            assert_eq!(sqlx::query_as::<_, (String, String, String)>("SELECT event_id::text,transaction_id::text,item_instance_id::text FROM game_item_mint_receipts WHERE loot_purpose_key='CORPSE_MATERIALIZATION' ORDER BY event_id")
                .fetch_all(&h.pool).await?, receipts, "replayed owner turn cannot mint another body");
            assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid")
                .bind(item.as_slice()).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?, 1);
            assert!(
                !view
                    .root
                    .read_item_loop_ground_for_scope(instance)
                    .await?
                    .iter()
                    .any(|row| row.item.item_instance_id == item)
            );
            assert!(
                !sqlx::query_scalar::<_, bool>(
                    "SELECT game_item_plain_corpse_source(encode($1,'hex')::uuid)"
                )
                .bind(item.as_slice())
                .fetch_one(&h.pool)
                .await?
            );
            drop(view);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}
