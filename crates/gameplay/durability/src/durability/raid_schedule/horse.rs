//! Horse projections reuse the existing scheduler, runner and LOST actor cleanup.
use super::{DurabilityError, WorldId};
use crate::durability::world_event::changes::actions::{LiveChanges, projection_matches};
use sqlx::{Postgres, Transaction};

const KEYS: [&str; 10] = [
    "raid:horsestation.stable.1",
    "raid:horsestation.stable.2",
    "raid:horsestation.stable.3",
    "raid:horsestation.stable.4",
    "raid:horsestation.stable.5",
    "raid:horsestation.stable.6",
    "raid:horsestation.stable.7",
    "raid:horsestation.stable.8",
    "raid:horsestation.stable.9",
    "raid:horsestation.wild",
];

pub(super) async fn selected(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
    key: &str,
    live: Option<&LiveChanges>,
    now_ms: i64,
) -> Result<bool, DurabilityError> {
    if !live.is_some_and(|live| projection_matches(key, &live.horse, now_ms)) {
        return Ok(false);
    }
    // Ordinary raid targets stay frozen. Source-kind4 is a persistent current
    // World projection: a newly admitted Channel needs its own fresh run even
    // when the original firing's event sequence is already DONE elsewhere.
    let retained:Option<Vec<u8>>=sqlx::query_scalar(
        "SELECT uuid_send(f.firing_id) FROM game_world_raid_firings f \
         JOIN game_world_raid_schedule_sources s USING(world_id,raid_key) \
         WHERE f.world_id=encode($1,'hex')::uuid AND f.reset_epoch=$2::text::numeric \
           AND f.raid_key=$3 AND f.state IN(1,2) AND s.source_kind=4 \
           AND NOT EXISTS(SELECT 1 FROM game_world_raid_effects e WHERE e.firing_id=f.firing_id AND e.action_kind=4) \
         ORDER BY f.ordinal DESC LIMIT 1",
    ).bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(key)
        .fetch_optional(&mut **tx).await?;
    if let Some(firing) = retained {
        sqlx::query(
            "WITH added AS(INSERT INTO game_world_raid_targets(firing_id,channel_id,owner_generation) \
             SELECT encode($1,'hex')::uuid,a.channel_id,a.ownership_generation \
             FROM game_runtime_scope_assignments a JOIN game_durability_admission_runtime_guards g USING(scope_key) \
             WHERE a.world_id=encode($2,'hex')::uuid AND a.scope_kind=1 AND a.state=1 \
               AND g.ready AND g.ownership_generation=a.ownership_generation \
               AND NOT EXISTS(SELECT 1 FROM game_world_raid_targets t \
                 WHERE t.firing_id=encode($1,'hex')::uuid AND t.channel_id=a.channel_id) \
               AND game_node_lock_current_registration(a.holder_node_id,a.holder_registration_revision) \
             ORDER BY a.channel_id LIMIT 32 FOR SHARE OF a,g \
             ON CONFLICT DO NOTHING RETURNING firing_id,channel_id,owner_generation) \
             INSERT INTO game_world_raid_channel_runs(firing_id,channel_id,owner_generation,updated_at_ms) \
             SELECT firing_id,channel_id,owner_generation,$3 FROM added ON CONFLICT DO NOTHING",
        ).bind(&firing).bind(world.as_bytes().as_slice()).bind(now_ms).execute(&mut **tx).await?;
    }
    // A persistent DONE stable still owns live horses. A phase/count change makes
    // its physical World predicate false and the existing runner commits LOST.
    // Wait for cleanup ACK before any replacement wave, including another count.
    // Inactive historical targets cannot permanently block a current projection.
    let pending: bool=sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_world_raid_firings f \
         JOIN game_world_raid_targets t USING(firing_id) \
         JOIN game_runtime_scope_assignments a ON a.world_id=f.world_id AND a.channel_id=t.channel_id \
           AND a.scope_kind=1 AND a.state=1 \
         JOIN game_durability_admission_runtime_guards g USING(scope_key) \
         JOIN game_node_registrations n ON n.node_id=a.holder_node_id \
           AND n.registration_revision=a.holder_registration_revision AND n.state=1 \
         WHERE f.world_id=encode($1,'hex')::uuid AND f.reset_epoch=$2::text::numeric \
           AND f.raid_key=ANY($3::text[]) AND g.ready AND g.ownership_generation=a.ownership_generation \
           AND NOT EXISTS(SELECT 1 FROM game_world_raid_effects e \
             WHERE e.firing_id=f.firing_id AND e.channel_id=t.channel_id AND e.action_kind=4 AND e.state=2) \
           AND (f.state=1 OR EXISTS(SELECT 1 FROM game_world_raid_creatures c \
             WHERE c.firing_id=f.firing_id AND c.channel_id=t.channel_id AND c.state=1) \
             OR EXISTS(SELECT 1 FROM game_world_raid_channel_runs r \
               WHERE r.firing_id=f.firing_id AND r.channel_id=t.channel_id AND r.state=1) \
             OR EXISTS(SELECT 1 FROM game_world_raid_effects e \
               WHERE e.firing_id=f.firing_id AND e.channel_id=t.channel_id AND e.state=1)))",
    ).bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(KEYS.as_slice())
        .fetch_one(&mut **tx).await?;
    Ok(!pending)
}
