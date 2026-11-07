//! Native paced Player ingress into an actually minted source Pod.
use super::TestResult;
use super::fixture::{Harness, debug};
use crate::foundation::{MovementLocalPosition, RuntimeScopeRefV1};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};

pub(super) async fn step(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    capture: &mut Vec<Vec<u8>>,
) -> TestResult<i64> {
    let fence = admitted.item_fence.ok_or("real Pod step fence")?;
    let actor = admitted.runtime_actor.ok_or("real Pod step player")?;
    let RuntimeScopeRefV1::Instance { instance_id, .. } = fence.runtime_scope else {
        return Err("Instance".into());
    };
    let registered = owner
        .activity_runtimes
        .get(fence.runtime_scope, fence.scope_ownership_generation.get())
        .ok_or("real Pod runtime")?;
    let pods:Vec<(Vec<u8>,i64)>=sqlx::query_as("SELECT spatial_position,max(source_timer_unix_ms) FROM game_primal_pod_callbacks WHERE instance_id=encode($1,'hex')::uuid AND completion_kind=0 GROUP BY spatial_position ORDER BY min(due_unix_ms)").bind(instance_id.as_slice()).fetch_all(&h.pool).await?;
    let (route, timer) = {
        let view = owner.runtime_view(&registered);
        let runtime = view.runtime.lock().await;
        let start = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        let occupied = runtime
            .visible_entities()
            .creatures
            .into_iter()
            .map(|c| runtime.read_actor_position(c.actor).map(|p| p.position()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(debug)?;
        let mut choices = Vec::new();
        for (bytes, timer) in pods {
            if bytes.len() != 10 {
                return Err("actual Pod position bytes".into());
            }
            let at = MovementLocalPosition {
                x: i32::from_be_bytes(bytes[..4].try_into()?),
                y: i32::from_be_bytes(bytes[4..8].try_into()?),
                floor: i16::from_be_bytes(bytes[8..].try_into()?),
            };
            if let Ok(route) = super::srx3_primal_terminal::source_route(
                view.effective_movement_cells(),
                start,
                at,
                &occupied,
                false,
            ) && !route.is_empty()
            {
                choices.push((route, timer));
            }
        }
        choices
            .into_iter()
            .min_by_key(|(route, _)| route.len())
            .ok_or("real captured-map path to a Pod")?
    };
    let last = route.len() - 1;
    let mut before = None;
    for (index, direction) in route.into_iter().enumerate() {
        if index == last {
            // This is an independent real cold observation after legal paced
            // approach. The physical Pod is now adjacent and inside Map17;
            // the following STEP still consumes that same canonical Item.
            let view = owner.runtime_view(&registered);
            super::boss_activity::capture_current_snapshot(&view, admitted, capture).await?;
            let now: i64 = sqlx::query_scalar(
                "SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
            )
            .fetch_one(&h.pool)
            .await?;
            if now < timer + 2050 {
                tokio::time::sleep(std::time::Duration::from_millis(u64::try_from(
                    timer + 2050 - now,
                )?))
                .await;
            }
            let view = owner.runtime_view(&registered);
            let runtime = view.runtime.lock().await;
            let states = view.states.spell_states.lock().await;
            let vitals = states
                .get(&runtime, actor, admitted.game_session_id)
                .ok_or("actual Player state")?
                .vitals();
            before = Some((vitals.health, vitals.max_health));
        }
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_millis(500))
            .ok_or("owner pace clock")?;
        let view = owner.runtime_view(&registered);
        let (result, _) = view
            .ordinary_paced_step(actor, admitted.game_session_id, direction, None, None)
            .await;
        assert_eq!(
            result.disposition,
            oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved,
            "actual native paced ingress into durable Pod"
        );
    }
    let view = owner.runtime_view(&registered);
    let runtime = view.runtime.lock().await;
    let states = view.states.spell_states.lock().await;
    let after = states
        .get(&runtime, actor, admitted.game_session_id)
        .ok_or("Player after real step")?
        .vitals()
        .health;
    let (before, maximum) = before.ok_or("final physical step")?;
    let expected = u64::from(maximum)
        .saturating_mul(5)
        .div_ceil(100)
        .saturating_mul(101)
        .div_ceil(100)
        + 500;
    assert_eq!(
        u64::from(before.saturating_sub(after)),
        expected,
        "actual source2..4sec LifeDrain with hazard1 and doubleceil"
    );
    drop(states);
    drop(runtime);
    let removed:i64=sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts WHERE runtime_scope_kind=2 AND channel_id IS NULL AND instance_id=encode($1,'hex')::uuid AND cause_kind=2 AND creature_occurrence LIKE 'primal-pod-step:%'").bind(instance_id.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(removed, 1, "one exact real StepIn source Item removal");
    Ok(removed)
}
