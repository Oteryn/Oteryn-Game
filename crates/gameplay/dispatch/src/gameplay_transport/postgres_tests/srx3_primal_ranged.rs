//! Portable ammunition retains actual Instance scope through the existing ranged owner.
use super::fixture::{Harness, debug, id, run};
use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player_vocation};
use super::{Dependencies, TestResult, reset};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use oteryn_protocol_oteryn::boss_encounter as wire;

#[test]
fn srx3_primal_real_instance_ranged_ammunition_consumes_once_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("srx3-primal-ranged", || {
        run(async |admin| {
            reset();
            let scene = SourceScene::new(
                PinnedActivitySourceRoom::Primal,
                MovementLocalPosition {
                    x: 33548,
                    y: 32752,
                    floor: 14,
                },
            )?;
            let deps: Dependencies = scene.dependencies()?;
            let h = Harness::create_with_admission_seed(admin, "srx3_primal_ranged", false, false)
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
            install_scene(&mut owner, &scene, PinnedActivitySourceRoom::Primal)?;
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
            .ok_or("native boss capability")?;
            prepare_player_vocation(&h, &owner, &mut admitted, 1999, "paladin").await?;
            let source = admitted.item_fence.ok_or("actual source fence")?;
            // Existing owned equipment is a test precondition, not fabricated
            // consumption evidence. Every tested change below uses the real writer.
            {
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica")
                    .execute(&mut *tx)
                    .await?;
                for (seed, key, quantity, slot) in [(70, 3350, 1, 6i16), (72, 3447, 3, 10i16)] {
                    sqlx::raw_sql(sqlx::AssertSqlSafe(
                        crate::durability::item_fee_burn::test_support::item(
                            seed,
                            &format!("oteryn:item.tibia.i{key}"),
                            "definition-r1",
                            quantity,
                        ),
                    ))
                    .execute(&mut *tx)
                    .await?;
                    sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid)")
                        .bind(source.character_id.as_bytes().as_slice())
                        .bind(source.runtime_scope.world_id().as_bytes().as_slice())
                        .bind(slot).bind(id(seed).as_slice()).bind(id(seed+1).as_slice())
                        .execute(&mut *tx).await?;
                }
                tx.commit().await?;
            }
            let instance = super::activity_runtime::allocate_ready_instance(&h, 209).await?;
            let mut frames = Vec::new();
            let (_, reply, scope) = super::boss_activity::exchange_capture(
                &owner,
                &mut admitted,
                "oteryn:creature.the_primal_menace",
                true,
                &mut frames,
            )
            .await?;
            assert!(matches!(reply, wire::BossActivityReply::Accepted { .. }));
            assert!(scope.is_some());
            let fence = admitted.item_fence.ok_or("actual Instance fence")?;
            assert_eq!(fence.runtime_scope, instance);
            let registered = owner
                .activity_runtimes
                .get(instance, fence.scope_ownership_generation.get())
                .ok_or("actual private owner")?;
            let actor = admitted.runtime_actor.ok_or("actual private player")?;
            let (target, route) = {
                let view = owner.runtime_view(&registered);
                let runtime = view.runtime.lock().await;
                let boss = runtime
                    .visible_entities()
                    .creatures
                    .into_iter()
                    .find(|creature| {
                        runtime.matches_live_creature_identity(
                            creature.actor,
                            b"oteryn:creature.the_primal_menace",
                        )
                    })
                    .ok_or("actual native Primal")?;
                let start = runtime
                    .read_actor_position(actor)
                    .map_err(debug)?
                    .position();
                let at = runtime
                    .read_actor_position(boss.actor)
                    .map_err(debug)?
                    .position();
                let route = super::srx3_primal_terminal::source_route(
                    view.effective_movement_cells(),
                    start,
                    at,
                    &[at],
                    true,
                )?;
                (
                    oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                        identity: boss.actor.placement_identity(),
                        generation: boss.generation,
                    },
                    route,
                )
            };
            for direction in route {
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(1))
                    .ok_or("owner clock")?;
                let view = owner.runtime_view(&registered);
                let (step, _) = view
                    .ordinary_paced_step(actor, admitted.game_session_id, direction, None, None)
                    .await;
                assert_eq!(
                    step.disposition,
                    oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
                );
            }
            let view = owner.runtime_view(&registered);
            let equipment = crate::gameplay_transport::equipment_move::read_current_equipment(
                &view,
                admitted.game_session_id,
            )
            .await
            .ok_or("actual portable ranged equipment after handoff")?;
            assert!(
                equipment
                    .items
                    .iter()
                    .any(|item| item.item_instance_id == id(70)
                        && item.definition.production_key == "oteryn:item.tibia.i3350")
            );
            assert!(
                equipment
                    .items
                    .iter()
                    .any(|item| item.item_instance_id == id(72)
                        && item.quantity == 3
                        && item.slot == 10)
            );
            assert_eq!(
                view.select_attack_target(actor, admitted.game_session_id, 947, Some(target))
                    .await,
                oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
            );
            view.refresh_attack_weapon(actor, admitted.game_session_id)
                .await;
            view.drain_auto_attacks().await;
            let quantity:i64=sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(72).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                quantity, 2,
                "actual source Instance ranged attack consumes exactly one portable arrow"
            );
            view.drain_auto_attacks().await;
            let replay:i64=sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(72).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(replay, 2, "same held swing drain cannot consume twice");
            let crate::foundation::RuntimeScopeRefV1::Instance { instance_id, .. } = instance
            else {
                return Err("real Instance".into());
            };
            let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts r JOIN game_spell_item_lines l USING(transaction_id) WHERE r.runtime_scope_kind=2 AND r.channel_id IS NULL AND r.instance_id=encode($1,'hex')::uuid AND r.spell_family='Item' AND r.swing_ordinal IS NOT NULL AND l.runtime_scope_kind=2 AND l.channel_id IS NULL AND l.instance_id=r.instance_id AND l.operation_kind=5 AND l.quantity_before=3 AND l.quantity_after=2")
                .bind(instance_id.as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                receipts, 1,
                "actual immutable ranged receipt and consumption line retain tagged Instance"
            );
            Ok(())
        })
    })?;
    eprintln!(
        "SRX3_PRIMAL_INSTANCE_RANGED_EXECUTED {:?}",
        started.elapsed()
    );
    Ok(())
}
