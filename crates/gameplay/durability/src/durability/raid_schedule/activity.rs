//! A fenced projection of the existing gameplay-idle owner, never an idle clock.
use super::WorldId;
use crate::durability::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use crate::durability::runtime_scope_assignment::{
    NodeIncarnationProof, assignment_scope_key, prove_current_incarnation,
};
use crate::durability::{DurabilityError, DurabilityRoot};
use crate::foundation::{GameSessionId, RuntimeScopeRefV1, ScopeOwnershipGeneration};
use oteryn_gameplay_encounter_rules::raid::{CHECK_INTERVAL_MS, LUA_RAID_ACTIVE_IDLE_MS};
use sqlx::{Postgres, Transaction};

/// The existing CREATUREAI0-RL-04 physical actor envelope bounds this projection;
/// an over-limit source fails closed instead of supplying a partial player count.
pub const MAX_RAID_ACTIVITY_PLAYERS: usize = 262_144;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaidActivityPlayer {
    pub session: GameSessionId,
    pub active_until_ms: i64,
}

#[derive(Debug, Clone)]
pub struct RaidActivitySnapshot {
    pub world: WorldId,
    pub scope: RuntimeScopeRefV1,
    pub generation: ScopeOwnershipGeneration,
    pub reset_epoch: u64,
    pub observed_at_ms: i64,
    /// None invalidates an earlier report while an authoritative source fact is
    /// missing. Some(empty) is a complete Channel with no active players.
    pub players: Option<Vec<RaidActivityPlayer>>,
}

impl DurabilityRoot {
    pub async fn publish_raid_activity_snapshot(
        &self,
        node: &NodeIncarnationProof,
        snapshot: RaidActivitySnapshot,
    ) -> Result<bool, DurabilityError> {
        if snapshot.observed_at_ms < 0
            || snapshot.reset_epoch == 0
            || snapshot
                .players
                .as_ref()
                .is_some_and(|players| players.len() > MAX_RAID_ACTIVITY_PLAYERS)
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let scope_world = match snapshot.scope {
            RuntimeScopeRefV1::Channel { world_id, .. }
            | RuntimeScopeRefV1::Instance { world_id, .. } => world_id,
        };
        if scope_world != snapshot.world {
            return Err(DurabilityError::InvalidStoredState);
        }
        let scope_key = assignment_scope_key(snapshot.scope);
        let upper = snapshot
            .observed_at_ms
            .checked_add(
                i64::try_from(LUA_RAID_ACTIVE_IDLE_MS)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
            )
            .ok_or(DurabilityError::InvalidStoredState)?;
        let mut sessions = std::collections::BTreeSet::new();
        let mut body = Vec::with_capacity(snapshot.players.as_ref().map_or(0, Vec::len));
        for player in snapshot.players.iter().flatten() {
            if player.active_until_ms <= snapshot.observed_at_ms
                || player.active_until_ms > upper
                || !sessions.insert(*player.session.as_bytes())
            {
                return Err(DurabilityError::InvalidStoredState);
            }
            let session = player
                .session
                .as_bytes()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            body.push(
                serde_json::json!({"session":session,"active_until_ms":player.active_until_ms}),
            );
        }
        let body = snapshot
            .players
            .as_ref()
            .map(|_| serde_json::to_string(&body))
            .transpose()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            lock_admission_relations(&mut tx).await?;
            let world_current: bool = sqlx::query_scalar("SELECT game_world_reset_map_epoch_current(encode($1,'hex')::uuid,$2::text::numeric)")
                .bind(snapshot.world.as_bytes().as_slice()).bind(snapshot.reset_epoch.to_string()).fetch_one(&mut *tx).await?;
            if !world_current || !prove_current_incarnation(&mut tx, &node).await? {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(false);
            }
            let fact = node.fact();
            let current: Option<String> = sqlx::query_scalar("SELECT a.ownership_generation::text FROM game_runtime_scope_assignments a JOIN game_durability_admission_runtime_guards g ON g.scope_key=CASE WHEN a.scope_kind=3 THEN set_byte(a.scope_key,0,2) ELSE a.scope_key END WHERE a.world_id=encode($1,'hex')::uuid AND a.scope_key=$2 AND a.scope_kind IN(1,3) AND a.state=1 AND a.holder_node_id=encode($3,'hex')::uuid AND a.holder_registration_revision=$4::text::numeric AND g.ready AND g.ownership_generation=a.ownership_generation FOR SHARE OF a,g")
                .bind(snapshot.world.as_bytes().as_slice()).bind(scope_key.as_slice())
                .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).fetch_optional(&mut *tx).await?;
            if current.as_deref() != Some(&snapshot.generation.get().to_string()) {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(false);
            }
            let written = sqlx::query("INSERT INTO game_world_raid_activity(world_id,scope_key,reset_epoch,owner_generation,observed_at_ms,players) VALUES(encode($1,'hex')::uuid,$2,$3::text::numeric,$4::text::numeric,$5,$6::text::jsonb) ON CONFLICT(scope_key) DO UPDATE SET reset_epoch=EXCLUDED.reset_epoch,owner_generation=EXCLUDED.owner_generation,observed_at_ms=EXCLUDED.observed_at_ms,players=EXCLUDED.players WHERE (game_world_raid_activity.reset_epoch,game_world_raid_activity.owner_generation,game_world_raid_activity.observed_at_ms)<=(EXCLUDED.reset_epoch,EXCLUDED.owner_generation,EXCLUDED.observed_at_ms)")
                .bind(snapshot.world.as_bytes().as_slice()).bind(scope_key.as_slice())
                .bind(snapshot.reset_epoch.to_string()).bind(snapshot.generation.get().to_string())
                .bind(snapshot.observed_at_ms).bind(body).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(written.rows_affected() != 0)
        })).await
    }
}

pub(super) async fn world_count(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    reset_epoch: u64,
    now_ms: i64,
) -> Result<Option<u32>, DurabilityError> {
    let window = now_ms / CHECK_INTERVAL_MS * CHECK_INTERVAL_MS;
    // Assignment kind 2 is house custody, whose physical player stays in its
    // Channel. Native Activity Instances are kind 3 and own separate actors.
    let result: (bool, i64) = sqlx::query_as("WITH current_scopes AS (SELECT a.scope_key,a.ownership_generation FROM game_runtime_scope_assignments a JOIN game_durability_admission_runtime_guards g ON g.scope_key=CASE WHEN a.scope_kind=3 THEN set_byte(a.scope_key,0,2) ELSE a.scope_key END JOIN game_node_registrations n ON n.node_id=a.holder_node_id AND n.registration_revision=a.holder_registration_revision WHERE a.world_id=encode($1,'hex')::uuid AND a.scope_kind IN(1,3) AND a.state=1 AND g.ready AND g.ownership_generation=a.ownership_generation AND n.state=1), reports AS (SELECT c.scope_key,r.players FROM current_scopes c LEFT JOIN game_world_raid_activity r ON r.world_id=encode($1,'hex')::uuid AND r.scope_key=c.scope_key AND r.owner_generation=c.ownership_generation AND r.reset_epoch=$2::text::numeric AND r.observed_at_ms BETWEEN $3 AND $4) SELECT EXISTS(SELECT 1 FROM reports) AND NOT EXISTS(SELECT 1 FROM reports WHERE players IS NULL),(SELECT count(DISTINCT player->>'session') FROM reports,LATERAL jsonb_array_elements(players) AS player WHERE (player->>'active_until_ms')::bigint>$4)")
        .bind(world.as_bytes().as_slice()).bind(reset_epoch.to_string()).bind(window).bind(now_ms).fetch_one(&mut **tx).await?;
    if !result.0 {
        return Ok(None);
    }
    Ok(Some(
        u32::try_from(result.1).map_err(|_| DurabilityError::InvalidStoredState)?,
    ))
}
