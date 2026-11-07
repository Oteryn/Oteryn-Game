//! A unique physical cake bite shares the ordinary terrain CAS transaction.
use super::lock_world;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::{
    DurabilityError,
    item_craft::{MapHarvestOperand, MapHarvestRequest},
    item_mint_audit::Type2Transaction,
    terrain_tool,
};
use crate::foundation::WorldId;
use oteryn_protocol_oteryn::world_object::UseDisposition;

pub(crate) async fn apply(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &MapHarvestRequest,
    operand: &MapHarvestOperand,
    world: WorldId,
    node: &crate::durability::runtime_scope_assignment::NodeIncarnationProof,
) -> Result<UseDisposition, DurabilityError> {
    let reject = UseDisposition::NotPossible;
    let p = &operand.source_position;
    let mutation = &operand.mutation;
    if p.len() != 10
        || mutation.placement.len() != 40
        || mutation.after != 0
        || !mutation.decay.is_empty()
        || !matches!(mutation.before, 12085 | 12086)
    {
        return Ok(reject);
    }
    let x = i32::from_be_bytes(
        p[0..4]
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    );
    let y = i32::from_be_bytes(
        p[4..8]
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    );
    let floor = i16::from_be_bytes(
        p[8..10]
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    );
    let key = u64::from_be_bytes(
        mutation.placement[32..40]
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    );
    if !(33287..=33330).contains(&x)
        || !(32065..=32103).contains(&y)
        || floor != -7
        || !crate::durability::map_item_mint_audit::position_names_placement(p, key)
    {
        return Ok(reject);
    }
    let now = request.event_unix_seconds;
    let (_, channel) = crate::durability::item_transfer::scope_of(&fence)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    super::authority::arm(
        tx,
        node,
        world,
        channel,
        fence.scope_ownership_generation,
        now,
    )
    .await
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let Some(mut snapshot) = lock_world(tx, world, now)
        .await
        .map_err(|_| DurabilityError::InvalidStoredState)?
    else {
        return Ok(reject);
    };
    if snapshot.state.stage != 2
        || now >= snapshot.event_end
        || snapshot.state.layer_progress
            >= super::footprint::layer_goal(snapshot.state.layer)
                .ok_or(DurabilityError::InvalidStoredState)?
        || snapshot.state.window_start.is_none()
        || snapshot.state.window_end.is_none_or(|end| now >= end)
        || !super::footprint::is_edible_cell(
            u16::try_from(x).map_err(|_| DurabilityError::InvalidStoredState)?,
            u16::try_from(y).map_err(|_| DurabilityError::InvalidStoredState)?,
            u16::try_from(mutation.before).map_err(|_| DurabilityError::InvalidStoredState)?,
        )
    {
        return Ok(reject);
    }
    let start = snapshot
        .state
        .window_start
        .ok_or(DurabilityError::InvalidStoredState)?;
    let previous: Option<(i64,i16,i64)> = sqlx::query_as("SELECT window_start,bites,last_bite FROM game_seasonal_cake_bites WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND event_start=$3 FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(snapshot.event_start).fetch_optional(&mut ***tx).await?;
    let removed: Option<bool> = sqlx::query_scalar("SELECT removed FROM game_seasonal_cake_tiles WHERE world_id=encode($1,'hex')::uuid AND event_start=$2 AND layer=$3 AND x=$4 AND y=$5 AND floor=7")
        .bind(world.as_bytes().as_slice()).bind(snapshot.event_start).bind(i16::from(snapshot.state.layer)).bind(x).bind(y).fetch_optional(&mut ***tx).await?;
    if removed == Some(true) {
        return Ok(UseDisposition::StaleState);
    }
    if previous.is_some_and(|(window, bites, _)| window == start && bites >= 12) {
        return Ok(reject);
    }
    if !terrain_tool::apply_cake_in_transaction(
        tx,
        fence,
        request.command,
        &request.intent,
        &operand.scope,
        mutation,
    )
    .await?
    {
        return Ok(UseDisposition::StaleState);
    }
    let before = snapshot.state.clone();
    snapshot.state.window_start = Some(start);
    snapshot.state.window_end = Some(
        start
            .checked_add(1800)
            .ok_or(DurabilityError::InvalidStoredState)?,
    );
    snapshot.state.layer_progress += 1;
    // Closing a completed round is a following clock turn; this transaction contains
    // exactly one physical bite and one corresponding World revision.
    let revision = before.revision;
    snapshot.state.revision = revision
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?;
    super::save_world(tx, &snapshot)
        .await
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_seasonal_cake_tiles(world_id,event_start,layer,x,y,floor,appearance,window_start,removed) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5,7,$6,$7,true) ON CONFLICT(world_id,event_start,layer,x,y,floor) DO UPDATE SET removed=true,window_start=EXCLUDED.window_start")
        .bind(world.as_bytes().as_slice()).bind(snapshot.event_start).bind(i16::from(snapshot.state.layer)).bind(x).bind(y).bind(i32::try_from(mutation.before).map_err(|_|DurabilityError::InvalidStoredState)?).bind(start).execute(&mut ***tx).await?;
    let bites = previous
        .filter(|(window, _, _)| *window == start)
        .map_or(1, |(_, n, _)| n + 1);
    sqlx::query("INSERT INTO game_seasonal_cake_bites(character_id,world_id,event_start,window_start,bites,last_bite) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6) ON CONFLICT(character_id,world_id,event_start) DO UPDATE SET window_start=EXCLUDED.window_start,bites=EXCLUDED.bites,last_bite=EXCLUDED.last_bite")
        .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(snapshot.event_start).bind(start).bind(bites).bind(now).execute(&mut ***tx).await?;
    let (_, channel) = crate::durability::item_transfer::scope_of(&fence)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_seasonal_cake_bite_receipts(game_session_id,command_id,character_id,world_id,channel_id,event_start,layer,x,y,floor,window_start,appearance,placement,world_revision_before,world_revision_after) VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6,$7,$8,$9,7,$10,$11,$12,$13::text::numeric,$14::text::numeric)")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(snapshot.event_start).bind(i16::from(snapshot.state.layer)).bind(x).bind(y).bind(start).bind(i32::try_from(mutation.before).map_err(|_|DurabilityError::InvalidStoredState)?).bind(&mutation.placement).bind(revision.to_string()).bind(snapshot.state.revision.to_string()).execute(&mut ***tx).await?;
    Ok(UseDisposition::Committed)
}

/// Feedback is read under the same fenced transaction as the attempted source action.
pub(crate) async fn feedback(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &MapHarvestRequest,
    disposition: UseDisposition,
) -> Result<Option<crate::durability::item_craft::MapHarvestFeedback>, DurabilityError> {
    use crate::durability::item_craft::MapHarvestFeedback as Feedback;
    let only_window:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_seasonal_cake_window_starts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric)")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).fetch_one(&mut ***tx).await?;
    if disposition == UseDisposition::Committed && !only_window {
        return Ok(Some(Feedback::CakeConsumed));
    }
    let world = fence.runtime_scope.world_id();
    let Some(snapshot) = lock_world(tx, world, request.event_unix_seconds)
        .await
        .map_err(|_| DurabilityError::InvalidStoredState)?
    else {
        return Ok(Some(Feedback::CakeNotReady));
    };
    if snapshot.state.stage != 2 || request.event_unix_seconds >= snapshot.event_end {
        return Ok(Some(Feedback::CakeNotReady));
    }
    let personal:Option<(i64,i16,i64)>=sqlx::query_as("SELECT window_start,bites,last_bite FROM game_seasonal_cake_bites WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND event_start=$3")
        .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(snapshot.event_start).fetch_optional(&mut ***tx).await?;
    if let Some((window, bites, _last)) = personal
        && snapshot.state.window_start == Some(window)
        && bites >= 12
    {
        return Ok(Some(Feedback::CakeWindowLimit));
    }
    Ok(Some(Feedback::CakeNotReady))
}
