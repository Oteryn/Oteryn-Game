use super::*;
use oteryn_protocol_oteryn::item_view::{
    COMMAND_TYPE_ITEM_MOVE_INTENT, EquipmentSlot, ItemMoveDestination, ItemMoveIntent,
    ItemMoveSelection, encode_item_move_intent_for,
};

async fn move_item(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command: u64,
    handle: u64,
    destination: ItemMoveDestination,
) -> TestResult {
    let intent = encode_item_move_intent_for(
        &ItemMoveIntent {
            source: ItemHandle::new(handle).ok_or("boots handle")?,
            destination,
        },
        ItemMoveSelection {
            equip_drop: true,
            container_tree: false,
            world_interaction: false,
        },
    )
    .map_err(debug)?;
    let result = exchange_kind(
        owner,
        admitted,
        COMMAND_TYPE_ITEM_MOVE_INTENT,
        &[(command, intent)],
    )
    .await?;
    assert_eq!(result.len(), 1);
    assert_eq!(
        oteryn_protocol_oteryn::item_view::decode_item_move_result(&result[0]).map_err(debug)?,
        oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved
    );
    Ok(())
}
async fn stored(h: &Harness) -> TestResult<(String, Option<i64>, String)> {
    Ok(sqlx::query_as("SELECT i.definition_production_key,t.remaining_ms,t.state_revision::text FROM game_item_instances i JOIN game_item_timed_states t USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid").bind(id(90).as_slice()).fetch_one(&h.pool).await?)
}
async fn writes(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_item_timed_state_writes WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(90).as_slice()).fetch_one(&h.pool).await?)
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    _deps: &Dependencies,
    mut admitted: AdmittedSession,
) -> TestResult {
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[4, 6, 12, CAPABILITY_TIMED_ITEM_V1],
        )
        .ok_or("equip selection")?;
    move_item(
        owner,
        admitted,
        20,
        3,
        ItemMoveDestination::Equipment(EquipmentSlot::Feet),
    )
    .await?;
    let equipped = crate::gameplay_transport::equipment_move::read_current_equipment(
        owner,
        admitted.game_session_id,
    )
    .await
    .ok_or("equipped boots")?;
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native")?;
    assert_eq!(
        crate::gameplay_transport::equipment_move::equipment_effects_owner::speed_term(
            &equipped, native
        ),
        Some(0),
        "timed boots preserve the real movement speed consumer"
    );
    owner.clock_origin -= std::time::Duration::from_millis(60_001);
    owner.tick_timed_items().await;
    let checkpoint = stored(h).await?;
    assert_eq!(checkpoint.0, "oteryn:item.tibia.i6529");
    assert!(
        checkpoint
            .1
            .is_some_and(|remaining| (14_330_000..14_340_000).contains(&remaining))
    );
    assert_eq!(checkpoint.2, "1");
    // Unequip drains accrued time; time in the backpack never consumes the budget.
    move_item(owner, admitted, 21, 4, ItemMoveDestination::MainBackpack).await?;
    let paused = stored(h).await?;
    owner.clock_origin -= std::time::Duration::from_secs(100);
    owner.tick_timed_items().await;
    assert_eq!(stored(h).await?, paused, "backpack duration is paused");
    move_item(
        owner,
        admitted,
        22,
        2,
        ItemMoveDestination::Equipment(EquipmentSlot::Feet),
    )
    .await?;
    let before = stored(h).await?;
    let before_writes = writes(h).await?;
    // The owner clock, with no client packet, expires the remaining active budget.
    owner.clock_origin -= std::time::Duration::from_millis(14_400_001);
    owner.tick_timed_items().await;
    assert_eq!(
        stored(h).await?,
        (
            "oteryn:item.tibia.i6530".into(),
            None,
            (before.2.parse::<u64>()? + 1).to_string()
        )
    );
    assert_eq!(writes(h).await?, before_writes + 1);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("current")?;
    assert_eq!(
        owner
            .root
            .read_timed_row(owner.character, owner.holder, fence, id(90))
            .await
            .map_err(debug)?
            .ok_or("spent row")?
            .values,
        None
    );
    owner.tick_timed_items().await;
    assert_eq!(
        writes(h).await?,
        before_writes + 1,
        "expiry is exactly once"
    );
    Ok(())
}
