//! Read-only carried quantities and personally completed native pickup milestones.
use crate::durability::{DurabilityError,character_progression::CurrentCharacterGameplayFence};
use oteryn_gameplay_quests::quest::reward_callbacks::CarriedItemRequirement;
use sqlx::{PgConnection,Row};

/// The live Character root serializes custody. Pickup milestones use the existing
/// immutable map-MINT and successful item-loop pickup receipts, including merges.
pub(super) async fn holds(connection:&mut PgConnection,fence:CurrentCharacterGameplayFence,required:&[CarriedItemRequirement])->Result<bool,DurabilityError>{
 let keys:Vec<_>=required.iter().map(|r|r.item.as_str()).collect();
 let character=fence.character_id.as_bytes().as_slice();
 let world=fence.runtime_scope.world_id();
 let rows=sqlx::query("SELECT i.definition_production_key,i.quantity FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=ANY($3::text[]) ORDER BY i.item_instance_id LIMIT 501 FOR UPDATE OF i")
  .bind(character).bind(world.as_bytes().as_slice()).bind(keys).fetch_all(&mut *connection).await?;
 if rows.len()>500{return Ok(false);}
 for requirement in required {
  let mut needed=i64::from(requirement.count);
  for row in &rows {
   if row.try_get::<&str,_>("definition_production_key")?==requirement.item {
    let quantity:i64=row.try_get("quantity")?;needed-=needed.min(quantity);
   }
  }
  if needed!=0{return Ok(false);}
  if let Some(source)=requirement.map_source {
   // At most100 positive pickups suffice for any admitted requirement (count<=100).
   // operation5 proves Ground -> carried custody; a map MINT stranded on Ground
   // or a scripted Ground relocation cannot stand in for a completed pickup.
   let picked:i64=sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM (SELECT m.quantity FROM game_map_item_mint_receipts m JOIN game_item_loop_move_receipts r ON r.game_session_id=m.game_session_id AND r.command_id=m.command_id AND r.character_id=m.character_id AND r.world_id=m.world_id JOIN game_item_loop_move_lines l ON l.transaction_id=r.transaction_id AND l.item_instance_id=m.item_instance_id AND l.from_kind=3 WHERE m.character_id=encode($1,'hex')::uuid AND m.world_id=encode($2,'hex')::uuid AND m.placement_key=$3::text::numeric AND m.definition_production_key=$4 AND r.operation=5 LIMIT 100) successful_pickups")
    .bind(character).bind(world.as_bytes().as_slice()).bind(source.to_string()).bind(&requirement.item).fetch_one(&mut *connection).await?;
   if picked<i64::from(requirement.count){return Ok(false);}
  }
 }
 Ok(true)
}
