//! Legacy mature House writer must retain a real source clock without inventing a new one.
use super::*;
use crate::durability::consume_transform::HouseActionContext;
use crate::durability::house_interior::{
    HouseItemOutcome, NativeHouseMoveContext, NativeHouseTarget,
};
pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    source: [u8; 16],
) -> TestResult {
    let fence = admitted.item_fence.ok_or("actual legacy House fence")?;
    let actor = admitted.runtime_actor.ok_or("actual legacy House actor")?;
    let context = {
        let runtime = owner.runtime.lock().await;
        HouseActionContext::from_native_owner(&runtime, actor, admitted.game_session_id, fence)
            .map_err(debug)?
    };
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("actual legacy native content")?;
    let clock:Value=sqlx::query_scalar("SELECT to_jsonb(p) FROM game_spell_item_temporal_schedules p WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        clock["definition_production_key"],
        "oteryn:item.tibia.i9034"
    );
    assert_eq!(clock["duration_millis"], 604800000);
    let prepared = owner
        .root
        .prepare_native_house_item(
            owner.character,
            owner.holder,
            NativeHouseMoveContext {
                fence,
                origin_channel: *owner.channel_id.as_bytes(),
                house: context.house().clone(),
                command: crate::foundation::CommandRef::new(
                    admitted.game_session_id,
                    crate::foundation::CommandId::new(1).map_err(debug)?,
                ),
            },
            source,
            NativeHouseTarget::Position(POSITION),
            native,
        )
        .await
        .map_err(debug)?
        .ok_or("actual existing-clock House source")?;
    let candidate = owner
        .root
        .freeze_house_item(owner.character, owner.holder, fence, prepared.request)
        .await
        .map_err(debug)?;
    let result = owner
        .root
        .commit_house_item(owner.character, owner.holder, fence, &candidate)
        .await;
    assert!(
        matches!(result, Ok(HouseItemOutcome::Committed(_))),
        "genuine existing-clock legacyRootPlace must retain its source lifetime without optionalnewclockdigest: {result:?}"
    );
    assert_eq!(sqlx::query_scalar::<_,Value>("SELECT to_jsonb(p) FROM game_spell_item_temporal_schedules p WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?,clock);
    assert!(sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=encode($1,'hex')::uuid)").bind(source.as_slice()).fetch_one(&h.pool).await?);
    assert!(sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM game_item_transfer_receipts WHERE source_item_instance_id=encode($1,'hex')::uuid AND shape=5 AND furniture_content_digest IS NULL)").bind(source.as_slice()).fetch_one(&h.pool).await?);
    Ok(())
}
