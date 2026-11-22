//! A selected map specimen shares the quest writer's transaction and placement CAS.
use super::QuestWorldToolContext;
use crate::durability::{
    DurabilityError,
    character_progression::CurrentCharacterGameplayFence,
    terrain_tool::{MapMutation, TerrainScope, apply_object_in_transaction},
};
use crate::foundation::CommandRef;
use oteryn_gameplay_quests::quest::world_callbacks::{NativeSelector, Position};

pub(super) async fn apply(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    command: CommandRef,
    context: &QuestWorldToolContext,
    source: NativeSelector,
    replacement: Option<u16>,
) -> Result<bool, DurabilityError> {
    let Some(tool) = &context.selected_tool else {
        return Ok(false);
    };
    let Some(target) = super::selected_tool::map_target(&tool.intent) else {
        return Ok(false);
    };
    let Position { x, y, floor } = source.position;
    let Some(key) = oteryn_world_bundle::bundle::placement_key(floor, x, y, source.ordinal) else {
        return Ok(false);
    };
    if target.placement.len() != 40 || target.placement[32..] != key.to_be_bytes() {
        return Ok(false);
    }
    let Some(content) = context.content_revision.strip_prefix("sha256:") else {
        return Ok(false);
    };
    let digest: Option<Vec<u8>> =
        sqlx::query_scalar("SELECT decode($1,'hex') WHERE $2 = 'sha256:' || encode($3,'hex')")
            .bind(content)
            .bind(&context.map_revision)
            .bind(&target.placement[..32])
            .fetch_optional(&mut **tx)
            .await?;
    let Some(digest) = digest else {
        return Ok(false);
    };
    let Some(after) = replacement else {
        return Ok(true);
    };
    if after == source.appearance {
        return Ok(false);
    }
    let scope = TerrainScope {
        map_revision: context.map_revision.clone(),
        content_generation: digest
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        reset_epoch: i64::try_from(context.reset_epoch)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    };
    let mutation = MapMutation {
        placement: target.placement,
        expected_revision: i64::try_from(target.expected_revision)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        before: u32::from(source.appearance),
        after: u32::from(after),
        decay: Vec::new(),
    };
    if !apply_object_in_transaction(
        tx,
        fence.item_fence(),
        command,
        &tool.intent,
        &scope,
        Some(tool.source),
        &mutation,
    )
    .await?
    {
        return Ok(false);
    }
    // The terrain owner also accepts its own historical receipts. A fresh quest
    // receipt must instead share the transaction that changed this placement.
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_terrain_tool_receipts
         WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric
           AND xmin=pg_current_xact_id()::xid)",
    )
    .bind(command.game_session_id().as_bytes().as_slice())
    .bind(command.command_id().get().to_string())
    .fetch_one(&mut **tx)
    .await?)
}
