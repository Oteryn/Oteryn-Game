//! Crystal hireling.lua:562-575: both capacity and StoreInbox room precede payment.
use super::super::character_progression::CurrentCharacterGameplayFence;
use super::super::item_loop_move::{LoopError, carried_weight};
use super::super::item_mint::TypedDefinitionRef;
use super::super::item_transfer::{ItemWeightLookup, read_derived_transfer_capacity_centi_oz};
use super::super::DurabilityError;
use sqlx::{Postgres, Transaction};

pub(super) async fn can_reserve_lamp(
    tx:&mut Transaction<'_,Postgres>,
    fence:&CurrentCharacterGameplayFence,
    weights:Option<&dyn ItemWeightLookup>,
) -> Result<bool,DurabilityError> {
    // Crystal items.xml StoreInbox23396 has20 slots. Outstanding lamp reservations consume
    // eligibility too, so two accepted Hireling requests cannot both reserve its last slot.
    let occupied:i64=sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM game_item_character_inbox_locations WHERE character_id=encode($1,'hex')::uuid) \
         +(SELECT count(*) FROM game_hireling_store_reservations WHERE character_id=encode($1,'hex')::uuid AND state=1 AND kind=1)",
    ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    if occupied>=20 {return Ok(false)}
    let Some(weights)=weights else {return Ok(false)};
    let lamp=TypedDefinitionRef {family:"Item".into(),production_key:"oteryn:item.tibia.i29432".into(),revision_ref:"definition-r1".into()};
    let Ok(unit)=weights.weight_of(&lamp) else {return Ok(false)};
    let carried=match carried_weight(tx,fence.character_id,*fence.runtime_scope.world_id().as_bytes(),weights).await {
        Ok(value)=>value,
        Err(LoopError::Durability(error))=>return Err(error),
        Err(LoopError::Database(error))=>return Err(error.into()),
        Err(_)=>return Ok(false),
    };
    let capacity=read_derived_transfer_capacity_centi_oz(tx,fence.character_id,weights).await?;
    Ok(carried.checked_add(u64::from(unit)).is_some_and(|total|total<=capacity))
}
