//! Bind the reset to the loaded content and its verified durable House tiles.
use crate::durability::db::commit_semantic_read;
use super::*;
use oteryn_gameplay_content::content::QualifiedNativeEntryRoom;

impl DurabilityRoot {
    /// Validate the reset against the loaded content and locked durable catalogue.
    /// Returns true for a qualified nonempty House tile set.
    pub async fn preflight_world_reset_content(
        &self,
        reset: &RetiringWorldReset,
        source: &QualifiedNativeEntryRoom,
        target: &QualifiedNativeEntryRoom,
    ) -> Result<bool> {
        check(self, *reset, source, target).await
    }
}

pub(super) async fn check(
    root: &DurabilityRoot,
    reset: RetiringWorldReset,
    source: &QualifiedNativeEntryRoom,
    target: &QualifiedNativeEntryRoom,
) -> Result<bool> {
    if reset.target != target.compiled().server_digest() {
        return Err(DurabilityError::InvalidStoredState);
    }
    let source_cells = source.movement_cells();
    let target_cells = target.movement_cells();
    if source_cells.scope().world_id != reset.world || target_cells.scope().world_id != reset.world {
        return Err(DurabilityError::InvalidStoredState);
    }
    let known_empty_rooms =
        source_cells.house_tiles().is_known_empty() && target_cells.house_tiles().is_known_empty();

    let source_map = source_cells.scope().map_revision.as_str().to_owned();
    let source_server = source.compiled().server_digest();
    let source_client = source.compiled().client_digest();
    let source_frame = source.frame_binding().digest();
    let target_map = target_cells.scope().map_revision.as_str().to_owned();
    let target_server = target.compiled().server_digest();
    let target_client = target.compiled().client_digest();
    let target_frame = target.frame_binding().digest();
    let same_content = (&source_map, source_server, source_client, source_frame)
        == (&target_map, target_server, target_client, target_frame);
    root.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
        let mut tx = begin_semantic_transaction(holder, deadline).await?;
        crate::durability::db::lock_admission_relations(&mut tx).await?;
        let durable_houses = sqlx::query_scalar(
            "SELECT j.target_house_tiles<>'[]'::jsonb \
             FROM game_world_reset_house_preflight(encode($1,'hex')::uuid,$2::text::numeric) j \
             JOIN game_world_reset_epochs e USING(world_id,reset_epoch) \
             WHERE j.target_digest=$3 AND j.retirement_binding=$4 \
             AND (e.map_revision,e.server_digest,e.client_digest,e.frame_digest)=($5,$6,$7,$8) \
             AND (j.target_map_revision,j.target_server_digest,j.target_client_digest,j.target_frame_digest)=($9,$10,$11,$12)")
            .bind(reset.world.as_bytes().as_slice()).bind(reset.epoch.to_string())
            .bind(reset.target.as_slice()).bind(reset.binding.as_slice())
            .bind(source_map).bind(source_server.as_slice()).bind(source_client.as_slice()).bind(source_frame.as_slice())
            .bind(target_map).bind(target_server.as_slice()).bind(target_client.as_slice()).bind(target_frame.as_slice())
            .fetch_optional(&mut *tx).await?;
        let qualified = validate_house_qualification(durable_houses, same_content, known_empty_rooms)
            .ok_or(DurabilityError::InvalidStoredState)?;
        commit_semantic_read(tx, deadline).await?;
        Ok(qualified)
    })).await
}

const fn validate_house_qualification(
    durable_houses: Option<bool>,
    same_content: bool,
    known_empty_rooms: bool,
) -> Option<bool> {
    match durable_houses {
        Some(true) if same_content => Some(true),
        Some(false) if known_empty_rooms => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
