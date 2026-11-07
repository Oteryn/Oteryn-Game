//! Actual source onMove delivery, not a client Item creation request.
use super::{TestResult, fixture::Harness};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom as Room;
use crate::foundation::{MovementLocalPosition, RuntimeScopeRefV1};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::world_spatial::{StepDirection as D, StepDisposition};
use sqlx::Row;

#[test]
fn srx3_vemiath_actual_source_movement_footprint_postgres() -> TestResult {
    super::srx3_rotten_advanced::footprints(
        Room::Vemiath,
        MovementLocalPosition {
            x: 33078,
            y: 32333,
            floor: 15,
        },
        "oteryn:creature.vemiath",
    )
}
#[test]
fn srx3_chagorz_actual_source_movement_footprint_postgres() -> TestResult {
    super::srx3_rotten_advanced::footprints(
        Room::Chagorz,
        MovementLocalPosition {
            x: 33074,
            y: 32367,
            floor: 15,
        },
        "oteryn:creature.chagorz",
    )
}
#[test]
fn srx3_bakragore_actual_source_movement_footprint_postgres() -> TestResult {
    super::srx3_rotten_advanced::footprints(
        Room::Bakragore,
        MovementLocalPosition {
            x: 33010,
            y: 32392,
            floor: 15,
        },
        "oteryn:creature.bakragore",
    )
}
pub(super) async fn exercise(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    kind: Room,
) -> TestResult {
    let fence = admitted.item_fence.ok_or("actual footprint Item fence")?;
    let RuntimeScopeRefV1::Instance { instance_id, .. } = fence.runtime_scope else {
        return Err("source footprint Instance".into());
    };
    let actor = admitted.runtime_actor.ok_or("source footprint Player")?;
    let registered = owner
        .activity_runtimes
        .get(fence.runtime_scope, fence.scope_ownership_generation.get())
        .ok_or("actual footprint runtime")?;
    {
        let view = owner.runtime_view(&registered);
        let runtime = view.runtime.lock().await;
        eprintln!(
            "SRX3_ROTTEN_FOOTPRINT_BEFORE kind={kind:?} {:?}",
            runtime.rotten_movement_diagnostics_for_test(actor)
        );
    }
    let directions: &[D] = if kind == Room::Bakragore {
        &[D::North, D::North]
    } else {
        &[D::West]
    };
    for direction in directions {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(1))
            .ok_or("actual footprint pace clock")?;
        let view = owner.runtime_view(&registered);
        let (result, _) = view
            .ordinary_paced_step(actor, admitted.game_session_id, *direction, None, None)
            .await;
        assert_eq!(
            result.disposition,
            StepDisposition::Moved,
            "real observed Player movement"
        );
    }
    let view = owner.runtime_view(&registered);
    {
        let runtime = view.runtime.lock().await;
        eprintln!(
            "SRX3_ROTTEN_FOOTPRINT_AFTER kind={kind:?} {:?}",
            runtime.rotten_movement_diagnostics_for_test(actor)
        );
    }
    let at = view
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(super::fixture::debug)?
        .position();
    let mut bytes = Vec::with_capacity(10);
    bytes.extend_from_slice(&at.x.to_be_bytes());
    bytes.extend_from_slice(&at.y.to_be_bytes());
    bytes.extend_from_slice(&at.floor.to_be_bytes());
    let minted: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) JOIN game_spell_item_receipts r ON r.transaction_id=i.minted_transaction_id WHERE g.scope_kind=2 AND g.instance_id=encode($1,'hex')::uuid AND g.spatial_position=$2 AND i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i43589' AND r.cause_kind=3")
        .bind(instance_id.as_slice()).bind(&bytes).fetch_one(&h.pool).await?;
    assert_eq!(
        minted, 1,
        "actual source onMove creates one canonical physical43589 footprint at the committed current Player position"
    );
    let row=sqlx::query("SELECT i.item_instance_id::text,i.minted_transaction_id::text,i.state_revision::text,g.stack_ordinal::text FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.scope_kind=2 AND g.instance_id=encode($1,'hex')::uuid AND g.spatial_position=$2 AND i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i43589'")
        .bind(instance_id.as_slice()).bind(&bytes).fetch_one(&h.pool).await?;
    let item: String = row.try_get("item_instance_id")?;
    let mint: String = row.try_get("minted_transaction_id")?;
    let ordinal: String = row.try_get("stack_ordinal")?;
    let revision: u64 = row.try_get::<String, _>("state_revision")?.parse()?;
    let final_key = if kind == Room::Chagorz {
        "oteryn:item.tibia.i43625"
    } else {
        "oteryn:item.tibia.i43297"
    };
    for (delay, expected, successor) in [(1, "oteryn:item.tibia.i43590", 1u64), (3, final_key, 2)] {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(delay))
            .ok_or("actual nested footprint clock")?;
        owner
            .runtime_view(&registered)
            .drain_rotten_movement_callbacks()
            .await;
        let row=sqlx::query("SELECT i.definition_production_key,i.quantity,i.lifecycle,i.state_revision::text,i.minted_transaction_id::text,g.stack_ordinal::text FROM game_item_instances i LEFT JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.item_instance_id=$1::uuid")
            .bind(&item).fetch_one(&h.pool).await?;
        assert_eq!(
            row.try_get::<String, _>("definition_production_key")?,
            expected,
            "CURRENT source transform targets the same actual physical Item"
        );
        assert_eq!(row.try_get::<i64, _>("quantity")?, 1);
        assert_eq!(row.try_get::<i16, _>("lifecycle")?, 1);
        assert_eq!(
            row.try_get::<String, _>("state_revision")?.parse::<u64>()?,
            revision + successor
        );
        assert_eq!(row.try_get::<String, _>("minted_transaction_id")?, mint);
        assert_eq!(row.try_get::<String, _>("stack_ordinal")?, ordinal);
        if expected == "oteryn:item.tibia.i43297" {
            let schedules: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_spell_field_temporal_schedules s JOIN game_spell_item_receipts r ON r.transaction_id=s.source_transaction_id WHERE s.item_instance_id=$1::uuid AND s.source_stage=1 AND s.duration_millis=248000 AND s.target_family IS NULL AND s.target_production_key IS NULL AND s.target_revision IS NULL AND r.cause_kind=3 AND r.runtime_scope_kind=2 AND r.instance_id=encode($2,'hex')::uuid AND s.definition_production_key='oteryn:item.tibia.i43297' AND s.expires_at_unix_ms=(convert_from(r.intent,'UTF8')::jsonb->'operations'->(s.source_ordinal-1)->'intrinsic_decay'->>'execution_unix_ms')::bigint+248000",
            ).bind(&item).bind(instance_id.as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                schedules, 1,
                "same-UID native transform starts its intrinsic248s clock at the retained source execution"
            );
        }
        let mut tx = h.pool.begin().await?;
        crate::durability::spell_item_transaction::verify_spell_item_chain(&mut tx)
            .await
            .map_err(super::fixture::debug)?;
        tx.rollback().await?;
    }
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(3))
        .ok_or("actual final footprint clock")?;
    owner
        .runtime_view(&registered)
        .drain_rotten_movement_callbacks()
        .await;
    // An empty extra drain cannot replay a movement or create another timer.
    owner
        .runtime_view(&registered)
        .drain_rotten_movement_callbacks()
        .await;
    let row=sqlx::query("SELECT i.lifecycle,i.quantity,i.state_revision::text,i.minted_transaction_id::text,EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id) AS on_ground FROM game_item_instances i WHERE i.item_instance_id=$1::uuid")
        .bind(&item).fetch_one(&h.pool).await?;
    assert_eq!(row.try_get::<i16, _>("lifecycle")?, 2);
    assert_eq!(row.try_get::<i64, _>("quantity")?, 0);
    assert_eq!(
        row.try_get::<String, _>("state_revision")?.parse::<u64>()?,
        revision + 3
    );
    assert_eq!(row.try_get::<String, _>("minted_transaction_id")?, mint);
    assert!(!row.try_get::<bool, _>("on_ground")?);
    let counts:(i64,i64)=sqlx::query_as("SELECT count(*) FILTER(WHERE l.operation_kind=7),count(*) FILTER(WHERE l.operation_kind=2) FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id) WHERE l.item_instance_id=$1::uuid AND r.cause_kind=3 AND r.runtime_scope_kind=2 AND r.channel_id IS NULL AND r.instance_id=encode($2,'hex')::uuid")
        .bind(&item).bind(instance_id.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        counts,
        (2, 1),
        "two in-place transformations and one final removal; no replacement MINT"
    );
    Ok(())
}
