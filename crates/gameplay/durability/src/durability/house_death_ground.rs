//! Named-House projection of receipt-proven ordinary-death Ground roots.
//! HouseInterior custody and the full dropped container graph remain unchanged.
use super::consume_transform::HouseActionContext;
use super::item_loop_move::{ITEM_LOOP_CHANNEL_DROPS_MAX, LoopGroundItem};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use sqlx::Row;
use std::collections::BTreeSet;

impl DurabilityRoot {
    /// Reads visible Ground roots linked to an ordinary death in this current named House.
    pub async fn read_house_death_ground(
        &self,
        node: &NodeIncarnationProof,
        context: &HouseActionContext,
    ) -> Result<Option<Vec<LoopGroundItem>>, DurabilityError> {
        if !self.house_action_current(node, context).await? {
            return Ok(None);
        }
        let scope = context
            .house()
            .runtime_scope()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let items = self.read_item_loop_ground_for_scope(scope).await?;
        let world = *scope.world_id().as_bytes();
        let instance = context.house().instance_id();
        let roots = self.try_issue_semantic_pass()?.run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                let rows = sqlx::query(
                    "SELECT i.item_instance_id::text AS item \
                     FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id,world_id) \
                     JOIN game_player_death_item_records r ON r.item_instance_id=i.item_instance_id \
                       AND r.operation=1 AND g.corpse_ref=convert_to('OTERYN_ITEM_LOOP_DROP/v1:','UTF8')||uuid_send(r.transaction_id) \
                       AND g.native_room_placement_context=uuid_send(r.death_occurrence_id) \
                     JOIN game_character_death_receipts d USING(death_occurrence_id) \
                     WHERE g.world_id=encode($1,'hex')::uuid AND g.scope_kind=2 \
                       AND g.channel_id IS NULL AND g.instance_id=encode($2,'hex')::uuid \
                       AND r.world_id=g.world_id AND r.scope_kind=2 AND r.instance_id=g.instance_id \
                       AND d.death_world_id=g.world_id AND d.death_scope_kind=2 \
                       AND d.death_instance_id=g.instance_id AND d.death_channel_id IS NULL \
                       AND g.spatial_position=d.death_spatial_position AND i.lifecycle=1 \
                     ORDER BY i.item_instance_id LIMIT $3")
                    .bind(world.as_slice()).bind(instance.as_slice()).bind(ITEM_LOOP_CHANNEL_DROPS_MAX+1).fetch_all(&mut *tx).await?;
                if rows.len()>ITEM_LOOP_CHANNEL_DROPS_MAX as usize { return Err(DurabilityError::InvalidStoredState); }
                let roots=rows.iter().map(|row| super::character_progression::uuid_text(row.try_get("item")?))
                    .collect::<Result<BTreeSet<_>,DurabilityError>>()?;
                super::db::commit_semantic_transaction(tx,deadline).await?;
                Ok(roots)
            })
        }).await?;
        if !self.house_action_current(node, context).await? {
            return Ok(None);
        }
        let at = context.position();
        let items = items
            .into_iter()
            .filter(|entry| {
                if !roots.contains(&entry.item.item_instance_id) {
                    return false;
                }
                let Ok(cell) =
                    super::spell_items_abi::decode_ground_cell(&entry.item.ground.spatial_position)
                else {
                    return false;
                };
                cell.z == at[2] && cell.x.abs_diff(at[0]) <= 8 && cell.y.abs_diff(at[1]) <= 6
            })
            .collect();
        Ok(Some(items))
    }
}
