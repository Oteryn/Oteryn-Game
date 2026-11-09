//! Channel-owner recovery remains available without a live Character session.
use super::*;
use crate::durability::boss_encounter::{self as clocks, BossClockError};
use crate::durability::item_decay_retire::CorpseDecayFence;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

/// Independently observed native owner pins, rather than pins copied from an intent.
#[derive(Debug, Clone, Copy)]
pub struct ElementalSummonProjection {
    pub scope: ElementalSummonScope,
    pub content: [u8; 32],
    pub frame: [u8; 32],
}

fn clock_error(error: BossClockError) -> DurabilityError {
    match error {
        BossClockError::Unavailable(error) => error,
        _ => DurabilityError::InvalidStoredState,
    }
}

impl DurabilityRoot {
    /// Claim the complete, bounded active set under the native Channel owner.
    /// Only a successful empty result proves absence. A stale binding, reset,
    /// Content pin or partial claim errors, so a projector must retain its actors.
    pub async fn recover_elemental_summons(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        owner: CorpseDecayFence,
        projection: ElementalSummonProjection,
    ) -> Result<Vec<ElementalSummonClock>, DurabilityError> {
        permit.check_scope(owner.runtime_scope)?;
        let world = owner.runtime_scope.world_id();
        let channel = owner.runtime_scope.channel_id().ok_or(DurabilityError::InvalidStoredState)?;
        let reset: i64 = projection.scope.reset_epoch.try_into().map_err(|_| DurabilityError::InvalidStoredState)?;
        if projection.content == [0; 32] || projection.frame == [0; 32] || projection.scope.map == [0; 32] {
            return Err(DurabilityError::InvalidStoredState);
        }
        let record = recovery.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = crate::durability::db::begin_type2_transaction(holder, deadline).await?;
            clocks::assert_channel_owner(&mut tx, &record, &node, world, channel, owner.scope_ownership_generation)
                .await.map_err(clock_error)?;
            // The activation writer uses this same lock; its independently read
            // Content/frame must stay current until the complete claim commits.
            sqlx::query("SELECT pg_advisory_xact_lock_shared(hashtextextended('oteryn:content-activation:' || encode($1::bytea,'hex')::uuid::text || ':' || encode($2::bytea,'hex')::uuid::text,0))")
                .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&mut **tx).await?;
            let current: Option<bool> = sqlx::query_scalar(
                "SELECT server_artifact_digest=$3 AND frame_binding_digest=$4 \
                   AND game_world_reset_map_epoch_current(world_id,$5::text::numeric) \
                 FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid \
                   AND channel_id=encode($2,'hex')::uuid ORDER BY activation_sequence DESC LIMIT 1"
            ).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
                .bind(projection.content.as_slice()).bind(projection.frame.as_slice()).bind(reset.to_string())
                .fetch_optional(&mut **tx).await?;
            if current != Some(true) { return Err(DurabilityError::InvalidStoredState); }
            // Do not filter away mismatched active records: that would turn a
            // failed projection into an authoritative empty set and erase actors.
            let rows = sqlx::query(
                "SELECT b.transaction_id::text,b.altar_id,b.creature_key,b.summon_position, \
                   (b.map_revision='sha256:'||encode($4,'hex') \
                     AND b.content_revision='sha256:'||encode($5,'hex') AND b.placement_context=$6) AS binding_current \
                 FROM game_elemental_boss_leases l JOIN game_elemental_summon_intents b USING(transaction_id) \
                 WHERE l.world_id=encode($1,'hex')::uuid AND l.channel_id=encode($2,'hex')::uuid \
                   AND l.reset_epoch=$3 ORDER BY l.altar_id LIMIT 4"
            ).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(reset)
                .bind(projection.scope.map.as_slice()).bind(projection.content.as_slice()).bind(projection.frame.as_slice())
                .fetch_all(&mut **tx).await?;
            let mut recovered = Vec::with_capacity(rows.len());
            for row in rows {
                if !row.try_get::<bool,_>("binding_current")? { return Err(DurabilityError::InvalidStoredState); }
                let transaction: String = row.try_get("transaction_id")?;
                let transaction_id = uuid_text(&transaction)?;
                let altar: i32 = row.try_get("altar_id")?;
                let (x,y,floor): (u16,u16,i8) = serde_json::from_value(row.try_get("summon_position")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                let source = clocks::ElementalClockSource {
                    world, channel, generation: owner.scope_ownership_generation,
                    transaction: transaction_id, map: projection.scope.map, reset_epoch: projection.scope.reset_epoch,
                };
                let (key,clock) = clocks::claim_elemental_clock(&mut tx, &record, &node, source)
                    .await.map_err(clock_error)?.ok_or(DurabilityError::Unavailable)?;
                recovered.push(ElementalSummonClock {
                    intent: ElementalSummonIntent { transaction_id,
                        altar: altar.try_into().map_err(|_| DurabilityError::InvalidStoredState)?,
                        creature_key: row.try_get("creature_key")?, position: Position { x,y,floor } },
                    spawn_key: key.spawn, cycle: clock.cycle, owner_generation: owner.scope_ownership_generation,
                });
            }
            tx.commit(deadline).await?;
            Ok(recovered)
        })).await
    }
}
