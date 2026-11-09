//! Genuine movement delivery into the physical source-created action50001 Item.
use super::{
    TestResult,
    fixture::{Harness, debug},
};
use crate::foundation::{MovementLocalPosition, RuntimeScopeRefV1};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::world_spatial::{StepDirection as D, StepDisposition};
pub(super) async fn exercise(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    let fence = admitted.item_fence.ok_or("actual portal step fence")?;
    let RuntimeScopeRefV1::Instance { instance_id, .. } = fence.runtime_scope else {
        return Err("portal step Instance".into());
    };
    let player = admitted.runtime_actor.ok_or("actual portal Player")?;
    let registered = owner
        .activity_runtimes
        .get(fence.runtime_scope, fence.scope_ownership_generation.get())
        .ok_or("actual portal scope")?;
    owner.runtime_view(&registered).drain_monster_melee().await;
    // Move the real target away, allowing the native boss to vacate the centre.
    // One real lure step vacates the boss centre without drawing the source
    // Elders beyond the separate two-tile STEP predicate.
    for direction in [D::West] {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(1))
            .ok_or("source approach clock")?;
        let view = owner.runtime_view(&registered);
        let (step, _) = view
            .ordinary_paced_step(player, admitted.game_session_id, direction, None, None)
            .await;
        assert_eq!(
            step.disposition,
            StepDisposition::Moved,
            "actual source portal approach"
        );
        view.drain_monster_melee().await;
    }
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(90))
        .ok_or("source addEvent clock")?;
    {
        let view = owner.runtime_view(&registered);
        view.drain_monster_melee().await;
        view.drain_rotten_portal_callbacks().await;
    }
    {
        let view = owner.runtime_view(&registered);
        let runtime = view.runtime.lock().await;
        let states = view.states.spell_states.lock().await;
        eprintln!(
            "SRX3_PORTAL_BEFORE_PHYSICAL_ASSERT now_us={} {:?}",
            view.owner_now().get(),
            states
                .rotten_blood_callbacks
                .portal_diagnostics_for_test(&runtime)
        );
    }
    let present:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) JOIN game_spell_item_receipts r ON r.transaction_id=i.minted_transaction_id WHERE g.scope_kind=2 AND g.instance_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i37000' AND r.cause_kind=3")
        .bind(instance_id.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        present, 1,
        "actual source90s callback physically created the portal"
    );
    let centre = MovementLocalPosition {
        x: 33043,
        y: 32335,
        floor: 15,
    };
    // The actual Elders can be three tiles east of the portal when it opens.
    // Approach their current target through real paced movement so their own
    // native movement can bring one within the separate two-tile STEP test.
    let lure = {
        let view = owner.runtime_view(&registered);
        let runtime = view.runtime.lock().await;
        let occupied = runtime
            .visible_entities()
            .creatures
            .into_iter()
            .map(|c| runtime.read_actor_position(c.actor).map(|p| p.position()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(debug)?;
        super::srx3_primal_terminal::source_route(
            view.effective_movement_cells(),
            runtime
                .read_actor_position(player)
                .map_err(debug)?
                .position(),
            MovementLocalPosition {
                x: 33042,
                y: 32337,
                floor: 15,
            },
            &occupied,
            false,
        )?
    };
    assert!(
        lure.len() <= 5,
        "actual Elder lure stays inside the portal lifetime"
    );
    for direction in lure {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(1))
            .ok_or("source Elder lure clock")?;
        let view = owner.runtime_view(&registered);
        let (step, _) = view
            .ordinary_paced_step(player, admitted.game_session_id, direction, None, None)
            .await;
        assert_eq!(
            step.disposition,
            StepDisposition::Moved,
            "actual paced Elder lure"
        );
        view.drain_monster_melee().await;
    }
    let route = {
        let view = owner.runtime_view(&registered);
        let runtime = view.runtime.lock().await;
        let elders: Vec<_> = runtime
            .positioned_actor_census()
            .map_err(debug)?
            .into_iter()
            .filter_map(|(candidate, at, session)| {
                (session.is_none()
                    && runtime.companion_snapshot(candidate).is_ok_and(|s| {
                        s.health > 0
                            && s.state.policy.definition_key == "oteryn:creature.elder_bloodjaw"
                    }))
                .then_some(at.position())
            })
            .collect();
        assert!(
            elders.iter().any(|at| at.floor == centre.floor
                && at.x.abs_diff(centre.x) <= 2
                && at.y.abs_diff(centre.y) <= 2),
            "source STEP requires a current Elder within2, not merely the90s creator's15: {elders:?}"
        );
        let occupied = runtime
            .visible_entities()
            .creatures
            .into_iter()
            .map(|c| runtime.read_actor_position(c.actor).map(|p| p.position()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(debug)?;
        super::srx3_primal_terminal::source_route(
            view.effective_movement_cells(),
            runtime
                .read_actor_position(player)
                .map_err(debug)?
                .position(),
            centre,
            &occupied,
            false,
        )?
    };
    assert!(
        route.len() < 10,
        "source portal must be approached within its real15s lifetime"
    );
    for direction in route {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(1))
            .ok_or("source ingress clock")?;
        let view = owner.runtime_view(&registered);
        let (step, _) = view
            .ordinary_paced_step(player, admitted.game_session_id, direction, None, None)
            .await;
        assert_eq!(
            step.disposition,
            StepDisposition::Moved,
            "real committed STEP into source action50001"
        );
    }
    let consumed:i64=sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts r JOIN game_spell_item_lines l USING(transaction_id) WHERE r.cause_kind=3 AND r.instance_id=encode($1,'hex')::uuid AND convert_from(r.intent,'UTF8')::jsonb->'source_event'->>'rule'='rotten-blood.portal.step' AND l.operation_kind=2 AND l.definition_production_key='oteryn:item.tibia.i37000'")
        .bind(instance_id.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        consumed, 1,
        "real portal StepIn consumes the actual current action50001 Item through its canonical owner"
    );
    for (phase, seconds, percent) in [(1, 5, 15_u64), (2, 10, 25)] {
        for _ in 0..seconds {
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(1))
                .ok_or("massive attack clock")?;
            let view = owner.runtime_view(&registered);
            view.drain_monster_melee().await;
        }
        let view = owner.runtime_view(&registered);
        let runtime = view.runtime.lock().await;
        let states = view.states.spell_states.lock().await;
        let maximum = states
            .get(&runtime, player, admitted.game_session_id)
            .ok_or("source massive attack current Player")?
            .vitals()
            .max_health;
        let (amount, before, after) = states
            .rotten_blood_callbacks
            .actual_massive_commit_for_test(player, phase)
            .ok_or("actual source massive attack HP receipt at its retained deadline")?;
        assert_eq!(
            u64::from(amount),
            (u64::from(maximum) * percent).div_ceil(100)
        );
        assert!(
            after < before,
            "source Energy massive attack commits actual current target HP"
        );
    }
    Ok(())
}
