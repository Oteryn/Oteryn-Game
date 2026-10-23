//! HOUSE-1a seam: only proven absence of Houses is admitted by this slice.
use super::*;
use oteryn_gameplay_content::content::QualifiedNativeEntryRoom;

pub(super) async fn check(
    root: &DurabilityRoot,
    reset: RetiringWorldReset,
    source: &QualifiedNativeEntryRoom,
    target: &QualifiedNativeEntryRoom,
) -> Result<()> {
    if reset.target != target.compiled().server_digest() {
        return Err(DurabilityError::InvalidStoredState);
    }
    for room in [source, target] {
        let cells = room.movement_cells();
        if cells.scope().world_id != reset.world || !cells.house_tiles().is_known_empty() {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    let source_map = source.movement_cells().scope().map_revision.as_str().to_owned();
    let source_server = source.compiled().server_digest();
    let source_client = source.compiled().client_digest();
    let source_frame = source.frame_binding().digest();
    let target_map = target.movement_cells().scope().map_revision.as_str().to_owned();
    let target_server = target.compiled().server_digest();
    let target_client = target.compiled().client_digest();
    let target_frame = target.frame_binding().digest();
    root.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
        let mut tx = begin_semantic_transaction(holder, deadline).await?;
        crate::durability::db::lock_admission_relations(&mut tx).await?;
        let valid: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_world_reset_jobs j JOIN game_world_reset_epochs e USING(world_id,reset_epoch) \
             WHERE j.world_id=encode($1,'hex')::uuid AND j.reset_epoch=$2::text::numeric \
             AND j.target_digest=$3 AND j.retirement_binding=$4 \
             AND (e.map_revision,e.server_digest,e.client_digest,e.frame_digest)=($5,$6,$7,$8) \
             AND (j.target_map_revision,j.target_server_digest,j.target_client_digest,j.target_frame_digest)=($9,$10,$11,$12) \
             AND j.target_house_tiles='[]'::jsonb FOR UPDATE OF j)")
            .bind(reset.world.as_bytes().as_slice()).bind(reset.epoch.to_string())
            .bind(reset.target.as_slice()).bind(reset.binding.as_slice())
            .bind(source_map).bind(source_server.as_slice()).bind(source_client.as_slice()).bind(source_frame.as_slice())
            .bind(target_map).bind(target_server.as_slice()).bind(target_client.as_slice()).bind(target_frame.as_slice())
            .fetch_one(&mut *tx).await?;
        if !valid { return Err(DurabilityError::InvalidStoredState); }
        sqlx::query("LOCK TABLE game_item_house_interior_locations IN SHARE MODE").execute(&mut *tx).await?;
        let houses: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE world_id=encode($1,'hex')::uuid) OR EXISTS(SELECT 1 FROM game_house_ownership WHERE world_id=encode($1,'hex')::uuid)")
            .bind(reset.world.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
        if houses { return Err(DurabilityError::InvalidStoredState); }
        commit_semantic_transaction(tx, deadline).await
    })).await
}
