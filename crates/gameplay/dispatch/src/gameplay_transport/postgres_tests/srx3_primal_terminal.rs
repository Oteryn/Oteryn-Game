//! Real production physical final hit. Only the nonlethal initial injury is a
//! bounded test precondition; the source boss policy/maxHP and final death stay native.
use super::TestResult;
use super::fixture::{Harness, debug};
use crate::foundation::{MovementLocalPosition, RuntimeScopeRefV1};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};

pub(super) async fn final_hit(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    use crate::gameplay_transport::kill_reward::DrainOutcome;
    use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
    let fence = admitted.item_fence.ok_or("real private fence")?;
    let actor = admitted.runtime_actor.ok_or("real private player")?;
    let registered = owner
        .activity_runtimes
        .get(fence.runtime_scope, fence.scope_ownership_generation.get())
        .ok_or("real private owner")?;
    let (boss, target, route) = {
        let view = owner.runtime_view(&registered);
        let runtime = view.runtime.lock().await;
        let creature = runtime
            .visible_entities()
            .creatures
            .into_iter()
            .find(|c| {
                runtime
                    .matches_live_creature_identity(c.actor, b"oteryn:creature.the_primal_menace")
            })
            .ok_or("native source boss")?;
        let start = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        let destination = runtime
            .read_actor_position(creature.actor)
            .map_err(debug)?
            .position();
        let occupied = runtime
            .visible_entities()
            .creatures
            .into_iter()
            .map(|c| runtime.read_actor_position(c.actor).map(|p| p.position()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(debug)?;
        let route = source_route(
            view.effective_movement_cells(),
            start,
            destination,
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
            .ok_or("owner clock")?;
        let view = owner.runtime_view(&registered);
        let (step, _) = view
            .ordinary_paced_step(actor, admitted.game_session_id, direction, None, None)
            .await;
        assert_eq!(
            step.disposition,
            oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved,
            "real source-map path to native final hit"
        );
    }
    let view = owner.runtime_view(&registered);
    {
        let mut runtime = view.runtime.lock().await;
        let health = runtime.companion_snapshot(boss).map_err(debug)?;
        assert_eq!(
            health.maximum_health, 400_000,
            "initial injury never changes source maximum"
        );
        runtime.set_test_creature_health(boss, 1).map_err(debug)?;
        assert!(
            runtime
                .borrow_combat_death()
                .current_session_damage_shares(boss)
                .map_err(debug)?
                .is_empty(),
            "initial injury cannot fabricate damage credit"
        );
        assert_eq!(runtime.companion_snapshot(boss).map_err(debug)?.health, 1);
        assert!(
            !runtime.retains_corpse_projection(boss),
            "precondition cannot seed a death"
        );
    }
    assert_eq!(
        view.select_attack_target(actor, admitted.game_session_id, 932, Some(target))
            .await,
        oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
    );
    for _ in 0..32 {
        // The live connection refreshes ATTACK-2 equipment facts before its
        // combat drain; direct production-owner tests must run that same phase.
        view.refresh_attack_weapon(actor, admitted.game_session_id).await;
        view.drain_auto_attacks().await;
        if view.runtime.lock().await.retains_corpse_projection(boss) {
            break;
        }
        // Replayed drain is not a new occurrence; use actual elapsed swing cadence.
        tokio::time::sleep(std::time::Duration::from_millis(2100)).await;
    }
    assert!(
        view.runtime.lock().await.retains_corpse_projection(boss),
        "production physical final hit emits the native death"
    );
    assert_eq!(
        view.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );
    let RuntimeScopeRefV1::Instance { instance_id, .. } = fence.runtime_scope else {
        return Err("Instance".into());
    };
    let settled:i64=sqlx::query_scalar("SELECT count(*) FROM game_boss_reward_deaths WHERE scope_kind=2 AND channel_id IS NULL AND instance_id=encode($1,'hex')::uuid AND settlement_outcome=1 AND participant_count=1 AND eligible_count=1").bind(instance_id.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        settled, 1,
        "actual reward-boss native death composes the scoped personal reward settlement"
    );
    let contributor: (bool, i32) =
        sqlx::query_as("SELECT eligible,item_count FROM game_boss_reward_contributors")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        contributor,
        (true, 0),
        "source Primal loot is empty; eligibility must not invent a material reward"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_boss_reward_items")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_mint_receipts r JOIN game_item_ground_locations g USING(item_instance_id) WHERE r.loot_purpose_key='CORPSE_MATERIALIZATION' AND g.scope_kind=2 AND g.channel_id IS NULL AND g.instance_id=encode($1,'hex')::uuid").bind(instance_id.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        count, 1,
        "real native death reaches the same scoped corpse MINT owner"
    );
    let audit:Vec<u8>=sqlx::query_scalar("SELECT a.envelope FROM game_item_mint_receipts r JOIN game_item_audit_outbox a USING(event_id) WHERE r.loot_purpose_key='CORPSE_MATERIALIZATION' AND r.death_scope_kind=2 AND r.death_channel_id IS NULL").fetch_one(&h.pool).await?;
    let (envelope, mint) =
        crate::durability::item_mint_audit::decode_envelope(&audit).map_err(debug)?;
    assert!(envelope.channel_id.is_none());
    let typed = mint
        .source
        .ok_or("real corpse source")?
        .death_occurrence
        .ok_or("typed native death")?;
    assert_eq!(typed.runtime_scope().map_err(debug)?, fence.runtime_scope);
    assert_eq!(envelope.instance_id, typed.instance_id);
    let ground = mint.destination.ok_or("scoped actual Ground")?;
    assert_eq!(ground.runtime_scope().map_err(debug)?, fence.runtime_scope);
    let corpse = view
        .runtime
        .lock()
        .await
        .visible_corpses()
        .into_iter()
        .find(|c| c.item.production_key == "oteryn:item.tibia.i39530")
        .ok_or("actual source corpse projection")?;
    let stages:Vec<(i64,i64)>=sqlx::query_as("SELECT s.duration_millis,s.expires_at_unix_ms-r.materialized_at FROM game_spell_field_temporal_schedules s JOIN game_item_mint_receipts r USING(item_instance_id) WHERE s.item_instance_id=encode($1,'hex')::uuid ORDER BY source_stage").bind(corpse.item.item_instance_id.as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(
        stages,
        vec![(10_000, 10_000), (300_000, 310_000), (300_000, 610_000)]
    );
    let retirement:i64=sqlx::query_scalar("SELECT game_corpse_retirement_deadline(item_instance_id)-materialized_at FROM game_item_mint_receipts WHERE item_instance_id=encode($1,'hex')::uuid").bind(corpse.item.item_instance_id.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(retirement, 670_000);
    tokio::time::sleep(std::time::Duration::from_millis(10_100)).await;
    view.drain_source_item_deadlines().await;
    view.drain_corpse_deadlines().await;
    assert!(
        view.runtime
            .lock()
            .await
            .visible_corpses()
            .iter()
            .any(|c| c.item.item_instance_id == corpse.item.item_instance_id
                && c.item.production_key == "oteryn:item.tibia.i39529"),
        "real scoped temporal consumer refreshes the native source corpse stage"
    );
    Ok(())
}

pub(super) fn source_route(
    cells: &crate::content::NativeEntryMovementCells,
    start: MovementLocalPosition,
    target: MovementLocalPosition,
    occupied: &[MovementLocalPosition],
    adjacent: bool,
) -> TestResult<Vec<oteryn_protocol_oteryn::world_spatial::StepDirection>> {
    use oteryn_protocol_oteryn::world_spatial::StepDirection as D;
    use std::collections::{BTreeMap, VecDeque};
    let mut pending = VecDeque::from([(start.x, start.y)]);
    let mut previous = BTreeMap::new();
    previous.insert((start.x, start.y), None);
    while let Some((x, y)) = pending.pop_front() {
        if (x - target.x).abs().max((y - target.y).abs()) == i32::from(adjacent) {
            let mut route = Vec::new();
            let mut current = (x, y);
            while let Some(Some((parent, direction))) = previous.get(&current) {
                route.push(*direction);
                current = *parent;
            }
            route.reverse();
            return Ok(route);
        }
        for (dx, dy, direction) in [
            (0, -1, D::North),
            (1, 0, D::East),
            (0, 1, D::South),
            (-1, 0, D::West),
        ] {
            let next = (x + dx, y + dy);
            if previous.contains_key(&next)
                || occupied
                    .iter()
                    .any(|at| at.x == next.0 && at.y == next.1 && at.floor == start.floor)
            {
                continue;
            }
            if cells.index().lookup(
                cells.scope(),
                crate::content::LogicalCell {
                    x: next.0,
                    y: next.1,
                    z: i32::from(start.floor),
                },
            ) != Ok(crate::content::CollisionClass::Walkable)
            {
                continue;
            }
            previous.insert(next, Some(((x, y), direction)));
            pending.push_back(next);
            if previous.len() > 4096 {
                return Err("bounded source path".into());
            }
        }
    }
    Err("actual captured map has no current path to native boss".into())
}
