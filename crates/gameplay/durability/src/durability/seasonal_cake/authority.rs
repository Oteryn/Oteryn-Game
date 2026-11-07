//! Private SQL source authority is armed only by a current possessed Node owner.
use super::*;
pub(super) async fn arm(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    world: WorldId,
    channel: ChannelId,
    generation: ScopeOwnershipGeneration,
    now: i64,
) -> Result<Option<i64>> {
    let active = oteryn_gameplay_foundation::scheduled_events::seasonal_events_at(now)
        .into_iter()
        .find(|e| {
            e.active
                && e.event == oteryn_protocol_oteryn::world_event::SeasonalEventKind::BirthdayCake
        });
    let event = if let Some(active) = active {
        Some(i64::try_from(active.start_unix).map_err(|_| BossClockError::InvalidInput)?)
    } else {
        sqlx::query_scalar::<_,i64>("SELECT event_start FROM game_seasonal_cake_worlds WHERE world_id=encode($1,'hex')::uuid AND event_start<=$2 ORDER BY event_start DESC LIMIT 1")
            .bind(world.as_bytes().as_slice()).bind(now).fetch_optional(&mut **tx).await?
    };
    let Some(event) = event else {
        return Ok(None);
    };
    let fact = node.fact();
    sqlx::query("SELECT game_cake_arm_world_node(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,encode($4,'hex')::uuid,$5::text::numeric,$6,$7,$8)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(generation.get().to_string())
        .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).bind(node.sql_possession_secret()).bind(event).bind(now)
        .execute(&mut **tx).await?;
    Ok(Some(event))
}
