//! One transaction's complete World drain proof; never a retained authority token.
use super::*;
use crate::durability::admission_authority_guards::AdmissionGuardStore;
use crate::durability::db::lock_admission_relations;
use crate::durability::item_decay_retire::CorpseDecayFence;
use crate::durability::item_mint::uuid_text;
use crate::durability::runtime_scope_assignment::{
    NodeIncarnationProof, prove_current_incarnation,
};
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
};
use crate::foundation::{ChannelId, RuntimeScopeRefV1, ScopeOwnershipGeneration};

pub(super) const WORLD_RESET_CHANNELS_MAX: usize = 128;

pub(in crate::durability) enum WorldResetDrain {
    NotCurrent,
    Drained(Vec<CorpseDecayFence>),
    CapacityExceeded,
}

pub(in crate::durability) async fn drained_channels(
    root: &DurabilityRoot,
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    reset: RetiringWorldReset,
) -> Result<WorldResetDrain> {
    lock_admission_relations(tx).await?;
    // Admission relations -> reset record -> assignments/guards is also the
    // retirement/activation order. No caller supplies the Channel cohort.
    let current: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_world_reset_jobs j JOIN game_world_reset_active a USING(world_id,reset_epoch) WHERE world_id=encode($1,'hex')::uuid \
         AND reset_epoch=$2::text::numeric AND target_digest=$3 AND retirement_binding=$4 \
         AND phase='RETIRING' FOR UPDATE OF j)",
    )
    .bind(reset.world.as_bytes().as_slice())
    .bind(reset.epoch.to_string())
    .bind(reset.target.as_slice())
    .bind(reset.binding.as_slice())
    .fetch_one(&mut **tx)
    .await?;
    if !current || !prove_current_incarnation(tx, node).await? {
        return Ok(WorldResetDrain::NotCurrent);
    }
    let rows = sqlx::query(
        "SELECT a.channel_id::text,a.ownership_generation::text,a.state, \
         a.holder_node_id::text,a.holder_registration_revision::text,c.predecessor_generation::text \
         FROM game_runtime_scope_assignments a LEFT JOIN game_world_reset_channels c \
         ON c.world_id=a.world_id AND c.channel_id=a.channel_id AND c.reset_epoch=$2::text::numeric \
         WHERE a.world_id=encode($1,'hex')::uuid AND a.scope_kind=1 ORDER BY a.channel_id LIMIT $3 FOR SHARE OF a",
    )
    .bind(reset.world.as_bytes().as_slice())
    .bind(reset.epoch.to_string())
    .bind((WORLD_RESET_CHANNELS_MAX + 1) as i64)
    .fetch_all(&mut **tx)
    .await?;
    if rows.len() > WORLD_RESET_CHANNELS_MAX {
        return Ok(WorldResetDrain::CapacityExceeded);
    }
    if rows.is_empty() {
        return Err(DurabilityError::InvalidStoredState);
    }
    let cohort_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_world_reset_channels WHERE world_id=encode($1,'hex')::uuid \
         AND reset_epoch=$2::text::numeric",
    )
    .bind(reset.world.as_bytes().as_slice())
    .bind(reset.epoch.to_string())
    .fetch_one(&mut **tx)
    .await?;
    if cohort_count != rows.len() as i64 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let guards = AdmissionGuardStore::from_root(root.clone());
    let fact = node.fact();
    let mut channels = Vec::with_capacity(rows.len());
    for row in rows {
        let channel_id = ChannelId::decode(&uuid_text(row.try_get("channel_id")?)?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let generation = row
            .try_get::<String, _>("ownership_generation")?
            .parse::<u64>()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let predecessor = row
            .try_get::<Option<String>, _>("predecessor_generation")?
            .ok_or(DurabilityError::InvalidStoredState)?
            .parse::<u64>()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let holder = row
            .try_get::<Option<String>, _>("holder_node_id")?
            .map(|value| uuid_text(&value))
            .transpose()?;
        let registration = row
            .try_get::<Option<String>, _>("holder_registration_revision")?
            .map(|value| value.parse::<u64>())
            .transpose()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        if row.try_get::<i16, _>("state")? != 1
            || generation <= predecessor
            || holder.as_ref() != Some(fact.node_id().as_bytes())
            || registration != Some(fact.registration_revision())
        {
            return Ok(WorldResetDrain::NotCurrent);
        }
        let scope = RuntimeScopeRefV1::channel(reset.world, channel_id);
        let guard = guards
            .load_locked(tx, &AdmissionAuthorityGuardKeyV1::Runtime(scope))
            .await?;
        if !guard.is_some_and(|guard| {
            matches!(guard.state,
            AdmissionAuthorityGuardStateV1::Runtime { ready: false, ownership_generation, .. }
            if ownership_generation == generation)
        }) {
            return Ok(WorldResetDrain::NotCurrent);
        }
        channels.push(CorpseDecayFence {
            world_id: reset.world,
            channel_id,
            scope_ownership_generation: ScopeOwnershipGeneration::new(generation)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        });
    }
    let sessions: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions \
         WHERE world_id=encode($1,'hex')::uuid AND session_state<>3)",
    )
    .bind(reset.world.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await?;
    Ok(if sessions {
        WorldResetDrain::NotCurrent
    } else {
        WorldResetDrain::Drained(channels)
    })
}
