//! Bind the independently decoded target catalogue and content to the frozen job.
use super::*;
use super::activation::ResetContent;
use super::bundle_houses::HouseTile;
use sqlx::types::Json;

pub(super) async fn check(
    root: &DurabilityRoot,
    reset: RetiringWorldReset,
    source: &ResetContent<'_>,
    target: &ResetContent<'_>,
    house_tiles: &[HouseTile],
) -> Result<()> {
    let source_map = source.map.to_owned();
    let (source_server, source_client, source_frame) = (source.server, source.client, source.frame);
    let target_map = target.map.to_owned();
    let (target_server, target_client, target_frame) = (target.server, target.client, target.frame);
    let house_tiles = house_tiles.to_vec();
    root.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
        let mut tx = begin_semantic_transaction(holder, deadline).await?;
        crate::durability::db::lock_admission_relations(&mut tx).await?;
        let valid: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_world_reset_jobs j JOIN game_world_reset_epochs e USING(world_id,reset_epoch) \
             WHERE j.world_id=encode($1,'hex')::uuid AND j.reset_epoch=$2::text::numeric \
             AND j.target_digest=$3 AND j.retirement_binding=$4 \
             AND (e.map_revision,e.server_digest,e.client_digest,e.frame_digest)=($5,$6,$7,$8) \
             AND (j.target_map_revision,j.target_server_digest,j.target_client_digest,j.target_frame_digest)=($9,$10,$11,$12) \
             AND j.target_house_tiles=$13::jsonb FOR UPDATE OF j)")
            .bind(reset.world.as_bytes().as_slice()).bind(reset.epoch.to_string())
            .bind(reset.target.as_slice()).bind(reset.binding.as_slice())
            .bind(source_map).bind(source_server.as_slice()).bind(source_client.as_slice()).bind(source_frame.as_slice())
            .bind(target_map).bind(target_server.as_slice()).bind(target_client.as_slice()).bind(target_frame.as_slice())
            .bind(Json(house_tiles))
            .fetch_one(&mut *tx).await?;
        if !valid { return Err(DurabilityError::InvalidStoredState); }
        sqlx::query("LOCK TABLE game_item_house_interior_locations IN SHARE MODE").execute(&mut *tx).await?;
        let houses: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE world_id=encode($1,'hex')::uuid) OR EXISTS(SELECT 1 FROM game_house_ownership WHERE world_id=encode($1,'hex')::uuid)")
            .bind(reset.world.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
        if houses { return Err(DurabilityError::InvalidStoredState); }
        commit_semantic_transaction(tx, deadline).await
    })).await
}
