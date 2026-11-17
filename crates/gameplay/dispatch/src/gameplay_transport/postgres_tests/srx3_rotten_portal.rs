//! Real reserved onThink -> addEvent -> scoped Item transaction -> current expiry.
use super::{TestResult, fixture::Harness};
use crate::foundation::RuntimeScopeRefV1;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
pub(super) async fn exercise(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    let fence = admitted.item_fence.ok_or("actual portal Instance fence")?;
    let RuntimeScopeRefV1::Instance { instance_id, .. } = fence.runtime_scope else {
        return Err("actual portal Instance".into());
    };
    let registered = owner
        .activity_runtimes
        .get(fence.runtime_scope, fence.scope_ownership_generation.get())
        .ok_or("actual portal runtime")?;
    for _ in 0..90 {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(1))
            .ok_or("portal owner clock")?;
        let view = owner.runtime_view(&registered);
        view.drain_monster_melee().await;
        view.drain_rotten_portal_callbacks().await;
    }
    let (minted,ground):(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_spell_item_receipts r JOIN game_spell_item_lines l USING(transaction_id) WHERE r.cause_kind=3 AND r.runtime_scope_kind=2 AND r.channel_id IS NULL AND r.instance_id=encode($1,'hex')::uuid AND r.creature_definition_key='oteryn:creature.vemiath' AND convert_from(r.intent,'UTF8')::jsonb->'source_event'->>'action_id'='50001' AND l.definition_production_key='oteryn:item.tibia.i37000' AND l.operation_kind=1), (SELECT count(*) FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.scope_kind=2 AND g.channel_id IS NULL AND g.instance_id=encode($1,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i37000' AND i.lifecycle=1)")
        .bind(instance_id.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        (minted, ground),
        (1, 1),
        "real source90s callback must mint its physical scoped portal"
    );
    {
        let view = owner.runtime_view(&registered);
        assert_dynamic_teleport(&view).await?;
    }
    for _ in 0..15 {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(1))
            .ok_or("portal expiry clock")?;
        let view = owner.runtime_view(&registered);
        view.drain_monster_melee().await;
        view.drain_rotten_portal_callbacks().await;
    }
    let (expired,ground):(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_spell_item_receipts r JOIN game_spell_item_lines l USING(transaction_id) WHERE r.cause_kind=3 AND r.instance_id=encode($1,'hex')::uuid AND convert_from(r.intent,'UTF8')::jsonb->'source_event'->>'rule'='rotten-blood.portal.expire' AND l.operation_kind=2 AND l.definition_production_key='oteryn:item.tibia.i37000'), (SELECT count(*) FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.scope_kind=2 AND g.instance_id=encode($1,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i37000' AND i.lifecycle=1)")
        .bind(instance_id.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        (expired, ground),
        (1, 0),
        "real source15s callback removes CURRENT portal through existing custody owner"
    );
    Ok(())
}

async fn assert_dynamic_teleport(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    use crate::durability::DurabilityError;
    use crate::durability::spell_item_transaction::{
        assert_spell_item_scope_in_transaction, begin_spell_owner_transaction,
    };
    use crate::movement::monster_map::CurrentMonsterMap;
    let permit = owner
        .spell_lane_permit()
        .await
        .ok_or("current portal read lane")?;
    let runtime = owner.runtime.lock().await;
    let room = owner
        .effective_rooms()
        .gate_monster_ai()
        .ok_or("current portal source room")?;
    let native = owner
        .active_generation
        .and_then(|a| a.native_gameplay())
        .ok_or("current portal native definitions")?;
    let pass = owner.root.try_issue_semantic_pass()?;
    let mut context = (owner, &*runtime, &permit, room, native);
    let teleport = pass
        .run_with_context(&mut context, move |holder, deadline, context| {
            Box::pin(async move {
                let (owner, runtime, permit, room, native) = context;
                let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                let binding = runtime.binding();
                let authority = assert_spell_item_scope_in_transaction(
                    &mut tx,
                    permit,
                    owner.root,
                    owner.character,
                    owner.holder,
                    binding.runtime_scope(),
                    binding.scope_generation().get(),
                )
                .await
                .map_err(|_| DurabilityError::Unavailable)?;
                let objects = owner.door.lock().await;
                let teleport = {
                    let mut map = CurrentMonsterMap::new(
                        &mut tx, &authority, runtime, room, &objects, native,
                    );
                    let at = crate::foundation::MovementLocalPosition {
                        x: 33043,
                        y: 32335,
                        floor: 15,
                    };
                    map.read(runtime, at).await;
                    map.source_placement_cell(runtime, "oteryn:creature.elder_bloodjaw", at)
                        .ok_or(DurabilityError::Unavailable)?
                        .teleport
                };
                tx.rollback().await?;
                Ok(teleport)
            })
        })
        .await?;
    assert!(
        teleport,
        "actual scoped37000 must be a Teleport in the current canonical monster placement map"
    );
    Ok(())
}
