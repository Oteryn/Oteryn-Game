//! Actual physical defeat and source-composed Teleport return; no fabricated terminal row.
use super::{
    TestResult,
    fixture::{Harness, debug},
};
use crate::foundation::RuntimeScopeRefV1;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::{boss_encounter as wire, world_spatial_entities::EntityRef};

pub(super) async fn exercise(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let fence = admitted.item_fence.ok_or("real Vemiath Instance fence")?;
    let player = admitted.runtime_actor.ok_or("real Vemiath Player")?;
    let registered = owner
        .activity_runtimes
        .get(fence.runtime_scope, fence.scope_ownership_generation.get())
        .ok_or("real Vemiath runtime")?;
    let (boss, target, route) = {
        let view = owner.runtime_view(&registered);
        let runtime = view.runtime.lock().await;
        let creature = runtime
            .visible_entities()
            .creatures
            .into_iter()
            .find(|c| runtime.matches_live_creature_identity(c.actor, b"oteryn:creature.vemiath"))
            .ok_or("unchanged native source Vemiath")?;
        let occupied = runtime
            .visible_entities()
            .creatures
            .into_iter()
            .map(|c| runtime.read_actor_position(c.actor).map(|p| p.position()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(debug)?;
        let route = super::srx3_primal_terminal::source_route(
            view.effective_movement_cells(),
            runtime
                .read_actor_position(player)
                .map_err(debug)?
                .position(),
            runtime
                .read_actor_position(creature.actor)
                .map_err(debug)?
                .position(),
            &occupied,
            true,
        )?;
        (
            creature.actor,
            EntityRef {
                identity: creature.actor.placement_identity(),
                generation: creature.generation,
            },
            route,
        )
    };
    for direction in route {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(1))
            .ok_or("source pace")?;
        let view = owner.runtime_view(&registered);
        let (step, _) = view
            .ordinary_paced_step(player, admitted.game_session_id, direction, None, None)
            .await;
        assert_eq!(
            step.disposition,
            oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
        );
    }
    let view = owner.runtime_view(&registered);
    {
        let mut runtime = view.runtime.lock().await;
        let original = runtime
            .companion_snapshot(boss)
            .map_err(debug)?
            .maximum_health;
        assert!(original > 1);
        // Historical initial injury only: final damage, causal shares, death,
        // corpse, Quest result and terminal state all use their production owners.
        runtime.set_test_creature_health(boss, 1).map_err(debug)?;
        assert_eq!(
            runtime
                .companion_snapshot(boss)
                .map_err(debug)?
                .maximum_health,
            original
        );
        assert!(
            runtime
                .borrow_combat_death()
                .current_session_damage_shares(boss)
                .map_err(debug)?
                .is_empty()
        );
        assert!(!runtime.retains_corpse_projection(boss));
    }
    assert_eq!(
        view.select_attack_target(player, admitted.game_session_id, 944, Some(target))
            .await,
        oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
    );
    for _ in 0..32 {
        view.refresh_attack_weapon(player, admitted.game_session_id)
            .await;
        view.drain_auto_attacks().await;
        if view.runtime.lock().await.retains_corpse_projection(boss) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(2100)).await;
    }
    assert!(
        view.runtime.lock().await.retains_corpse_projection(boss),
        "actual physical Vemiath final hit"
    );
    assert_eq!(
        view.drain_kill_rewards(admitted.game_session_id).await,
        crate::gameplay_transport::kill_reward::DrainOutcome::Drained
    );
    drop(view);
    let (terminal,deadline,now):(i64,i64,i64)=sqlx::query_as("SELECT terminal_at,exit_deadline,floor(extract(epoch FROM clock_timestamp())*1000)::bigint FROM game_boss_room_activities WHERE boss_key='oteryn:boss.vemiath'")
        .fetch_one(&h.pool).await?;
    assert_eq!(deadline - terminal, 60_000);
    let remaining = u64::try_from((deadline - now).max(0))?;
    assert!(remaining < 60_000);
    tokio::time::sleep(std::time::Duration::from_millis(remaining + 5)).await;
    let mut frames = Vec::new();
    let (_, reply, barrier) = super::boss_activity::exchange_command_capture(
        owner,
        admitted,
        "oteryn:boss.vemiath",
        true,
        wire::COMMAND_TYPE_BOSS_DEFEATED,
        &mut frames,
    )
    .await?;
    assert!(matches!(reply, wire::BossActivityReply::Accepted { .. }));
    assert!(barrier.is_some(), "actual native scope99 return");
    assert!(matches!(
        admitted
            .item_fence
            .ok_or("returned actual fence")?
            .runtime_scope,
        RuntimeScopeRefV1::Channel { .. }
    ));
    let runtime = owner.runtime.lock().await;
    let position = runtime
        .read_actor_position(admitted.runtime_actor.ok_or("returned actual Player")?)
        .map_err(debug)?
        .position();
    assert_eq!(
        (position.x, position.y, position.floor),
        (34001, 31820, 15),
        "native return lands at the actual source34111 Teleport destination"
    );
    drop(runtime);
    let stored:(i32,i32,i16)=sqlx::query_as("SELECT tile_x,tile_y,tile_floor FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        stored,
        (34001, 31820, 15),
        "durable position agrees with the physical return"
    );
    assert!(!frames.is_empty());
    Ok(())
}
