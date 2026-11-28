//! Current-owner recovery and exact native-death settlement for pending altars.
use super::*;
use crate::durability::spell_item_transaction::assert_spell_item_scope_with_recovery;
use crate::foundation::{ExactActorRef, RuntimeScopeRefV1, ScopeOwnershipGeneration};

#[derive(Debug)]
pub struct PendingAltarRealization {
    pub source: PendingAltarBoss,
    pub readset: Vec<(u64, u64)>,
    pub actor: Option<(u32, u64)>,
}

impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "recovery independently binds current scope, epoch and active content/map"
    )]
    pub async fn read_pending_altar_bosses(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
        epoch: i64,
        content: [u8; 32],
        map: &str,
    ) -> Result<Vec<PendingAltarRealization>, DurabilityError> {
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = (permit, map);
        self.try_issue_semantic_pass()?.run_with_context(&mut context,move |holder,deadline,context| Box::pin(async move {
            let (permit,map)=*context;
            let mut tx=crate::durability::db::begin_semantic_transaction(holder,deadline).await?;
            let owner=assert_spell_item_scope_with_recovery(&mut tx,permit,&record,&node,scope,generation.get()).await.map_err(owner_error)?;
            let channel=owner.channel_bytes().ok_or(DurabilityError::InvalidStoredState)?;
            let rows=sqlx::query("SELECT r.transaction_id::text,r.rule,r.readset,a.actor_local_id,a.actor_local_generation::text FROM game_elemental_boss_latches l JOIN game_elemental_altar_receipts r ON r.transaction_id=l.arrival_transaction_id JOIN game_item_loop_move_receipts m ON m.transaction_id=r.move_transaction_id LEFT JOIN game_elemental_boss_realizations a ON a.arrival_transaction_id=r.transaction_id AND a.ownership_generation=$4::text::numeric WHERE l.world_id=encode($1,'hex')::uuid AND l.channel_id=encode($2,'hex')::uuid AND l.reset_epoch=$3 AND m.content_digest=$5 AND r.inputs->0->'custody'->>'map_revision'=$6 AND game_world_reset_map_epoch_current(l.world_id,l.reset_epoch) AND EXISTS(SELECT 1 FROM game_content_activations c WHERE c.world_id=l.world_id AND c.channel_id=l.channel_id AND c.activation_sequence=(SELECT max(activation_sequence) FROM game_content_activations WHERE world_id=l.world_id AND channel_id=l.channel_id) AND c.server_artifact_digest=$5) ORDER BY r.rule LIMIT 4")
                .bind(owner.world_bytes().as_slice()).bind(channel.as_slice()).bind(epoch).bind(generation.get().to_string()).bind(content.as_slice()).bind(map).fetch_all(&mut *tx).await?;
            let mut pending=Vec::with_capacity(rows.len());
            for row in rows {
                let id:Option<i64>=row.try_get("actor_local_id")?;
                let rev:Option<String>=row.try_get("actor_local_generation")?;
                let actor=match (id,rev) {
                    (Some(id),Some(rev))=>Some((u32::try_from(id).map_err(|_|DurabilityError::InvalidStoredState)?,rev.parse().map_err(|_|DurabilityError::InvalidStoredState)?)),
                    (None,None)=>None,
                    _=>return Err(DurabilityError::InvalidStoredState),
                };
                pending.push(PendingAltarRealization {
                    source:PendingAltarBoss{transaction_id:uuid_text(&row.try_get::<String,_>("transaction_id")?)?,rule:row.try_get("rule")?},
                    readset:serde_json::from_value(row.try_get("readset")?).map_err(|_|DurabilityError::InvalidStoredState)?,actor,
                });
            }
            tx.rollback().await?;
            Ok(pending)
        })).await
    }

    /// Call only for an actor observed from the current runtime carrier while
    /// the Channel lane excludes other physical turns. A record is not spawn authority.
    pub async fn record_altar_boss_realization(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        arrival: [u8; 16],
        actor: ExactActorRef,
    ) -> Result<bool, DurabilityError> {
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = permit;
        self.try_issue_semantic_pass()?.run_with_context(&mut context,move |holder,deadline,permit|Box::pin(async move {
            let mut tx=crate::durability::db::begin_semantic_transaction(holder,deadline).await?;
            let owner=assert_spell_item_scope_with_recovery(&mut tx,permit,&record,&node,actor.runtime_scope(),actor.scope_generation().get()).await.map_err(owner_error)?;
            let channel=owner.channel_bytes().ok_or(DurabilityError::InvalidStoredState)?;
            sqlx::query("INSERT INTO game_elemental_boss_realizations(arrival_transaction_id,world_id,channel_id,ownership_generation,actor_local_id,actor_local_generation) SELECT l.arrival_transaction_id,l.world_id,l.channel_id,$4::text::numeric,$5,$6::text::numeric FROM game_elemental_boss_latches l WHERE l.arrival_transaction_id=encode($1,'hex')::uuid AND l.world_id=encode($2,'hex')::uuid AND l.channel_id=encode($3,'hex')::uuid ON CONFLICT(arrival_transaction_id,ownership_generation) DO NOTHING")
                .bind(arrival.as_slice()).bind(owner.world_bytes().as_slice()).bind(channel.as_slice()).bind(actor.scope_generation().get().to_string()).bind(i64::from(actor.0.actor_local_id.0)).bind(actor.0.actor_local_generation.0.to_string()).execute(&mut *tx).await?;
            let same:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_elemental_boss_realizations a JOIN game_elemental_boss_latches l USING(arrival_transaction_id) WHERE a.arrival_transaction_id=encode($1,'hex')::uuid AND a.ownership_generation=$2::text::numeric AND a.actor_local_id=$3 AND a.actor_local_generation=$4::text::numeric)")
                .bind(arrival.as_slice()).bind(actor.scope_generation().get().to_string()).bind(i64::from(actor.0.actor_local_id.0)).bind(actor.0.actor_local_generation.0.to_string()).fetch_one(&mut *tx).await?;
            crate::durability::db::commit_semantic_transaction(tx,deadline).await?;
            Ok(same)
        })).await
    }
}
fn owner_error(error: SpellItemError) -> DurabilityError {
    match error {
        SpellItemError::Durability(error) => error,
        SpellItemError::Database(error) => error.into(),
        _ => DurabilityError::InvalidStoredState,
    }
}
