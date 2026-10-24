use super::*;
use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    deps: &Dependencies,
    admitted: AdmittedSession,
) -> TestResult {
    owner.clock_origin -= std::time::Duration::from_secs(1);
    let npc = {
        let mut runtime = deps.runtime.lock().await;
        let position = runtime
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position();
        runtime
            .place_npcs(
                &NpcPlacementTable {
                    npcs: vec![NpcPlacements {
                        key: "oteryn:npc.aldo".into(),
                        placements: vec![NpcPlacement {
                            floor: -i8::try_from(position.floor)?,
                            x: u16::try_from(position.x + 1)?,
                            y: u16::try_from(position.y)?,
                            direction: MovementFacing::South,
                        }],
                    }],
                },
                |_| true,
                true,
            )
            .map_err(debug)?;
        let placed = runtime.visible_entities().npcs[0];
        EntityRef {
            identity: placed.actor.placement_identity(),
            generation: placed.generation,
        }
    };
    let intent = TimedItemIntent {
        action: TimedAction::Repair,
        target: ItemHandle::new(4).ok_or("worn handle")?,
        expected_revision: 0,
        npc: Some(npc),
    };
    let payload = intent_bytes(intent)?;
    // Failure after fee execution must roll back the entire physical transaction.
    sqlx::raw_sql("CREATE FUNCTION timed_test_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.cause=8 THEN RAISE EXCEPTION 'test repair commit failure' USING ERRCODE='23514'; END IF; RETURN NULL; END $$; CREATE CONSTRAINT TRIGGER timed_test_fail AFTER INSERT ON game_item_timed_state_writes DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION timed_test_fail();").execute(&h.pool).await?;
    assert!(
        exchange(owner, admitted, &[(10, payload.clone())])
            .await?
            .is_empty()
    );
    let coins: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(110).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(coins, 2, "unknown repair rolled its fee back");
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_fee_burns")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    sqlx::raw_sql("DROP TRIGGER timed_test_fail ON game_item_timed_state_writes; DROP FUNCTION timed_test_fail();").execute(&h.pool).await?;
    let repaired = exchange(owner, admitted, &[(10, payload.clone())]).await?;
    assert_eq!(repaired.len(), 1);
    let result = result_value(&repaired[0])?;
    assert_eq!(result.disposition, TimedDisposition::Ok);
    assert_eq!(result.fee_gold, 10_000);
    assert_eq!(result.remaining_ms, Some(14_400_000));
    assert_eq!(result.revision, 1);
    let row:(String,i64,String)=sqlx::query_as("SELECT definition_production_key,quantity,state_revision::text FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(80).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(row.0, "oteryn:item.tibia.i6529");
    assert_eq!(row.1, 1);
    let current = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("current")?;
    assert_eq!(
        current.expected_character_revision.get(),
        1,
        "item-only fee never advances CharacterRevision"
    );
    assert_eq!(exchange(owner, admitted, &[(10, payload)]).await?, repaired);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_fee_burns")
            .fetch_one(&h.pool)
            .await?,
        1,
        "repair replay never charges twice"
    );
    let coins: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(110).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(coins, 1);
    let envelope:Vec<u8>=sqlx::query_scalar("SELECT envelope FROM game_item_audit_outbox WHERE transaction_id=(SELECT transaction_id FROM game_item_fee_burns)").fetch_one(&h.pool).await?;
    let (_, fee) = crate::durability::item_fee_burn_audit::decode_fee_burn_envelope(&envelope)
        .map_err(debug)?;
    assert!(matches!(
        fee.cause.and_then(|c| c.cause),
        Some(crate::durability::item_fee_burn_audit::FeeBurnCauseV1::NpcRepair(_))
    ));
    Ok(())
}
