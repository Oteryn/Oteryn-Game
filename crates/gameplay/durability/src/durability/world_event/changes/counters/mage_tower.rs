use super::*;
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, assert_item_only_fence,
};
use oteryn_gameplay_entities::carrier::ExactActorRef;

const PERSONAL_KILL_MINIMUM: i64 = 250;
const DAMAGE_RECEIPT: i16 = 5;
const HIT_DOMAIN: &[u8] = b"oteryn.world_change.mage_tower_hit.v1\0";

impl DurabilityRoot {
    /// Resolve the supplied, exact local spectators against the current Raging phase.
    pub async fn mage_tower_unworthy(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        epoch: u64,
        spectators: Vec<Vec<u8>>,
        raging: Option<ExactActorRef>,
    ) -> Result<Option<Vec<Vec<u8>>>, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let world = fence.runtime_scope.world_id();
        let raging = raging.map(|actor| actor.placement_identity().to_vec());
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if assert_item_only_fence(&mut tx, &fence.item_fence(), &node).await?.is_err()
                        || !super::super::load(&mut tx, world, epoch).await?.is_some_and(|day| {
                            day.world_counters && day.counter_stages[CounterKind::MagesTower.index()] == 2
                        })
                    {
                        return Ok(None);
                    }
                    let unworthy = sqlx::query_scalar("SELECT candidate FROM unnest($3::bytea[]) AS s(candidate)
                        WHERE NOT EXISTS (
                            SELECT 1 FROM game_world_change_personal_counters c
                            WHERE c.world_id=encode($1,'hex')::uuid AND c.change_key=$2
                                AND c.character_id=encode(candidate,'hex')::uuid AND c.counter >= $4::bigint
                        ) AND NOT EXISTS (
                            SELECT 1 FROM game_world_change_credit_receipts r
                            WHERE r.world_id=encode($1,'hex')::uuid AND r.source_kind=$5
                                AND r.occurrence=sha256($6 || candidate || $7::bytea)
                        )")
                        .bind(world.as_bytes().as_slice()).bind(CounterKind::MagesTower.key())
                        .bind(spectators).bind(PERSONAL_KILL_MINIMUM).bind(DAMAGE_RECEIPT)
                        .bind(HIT_DOMAIN).bind(raging).fetch_all(&mut *tx).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Some(unworthy))
                })
            })
            .await
    }

    /// Consume qualification once when the current Character first damages this Raging Mage.
    pub async fn consume_mage_tower_kills(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        epoch: u64,
        target: ExactActorRef,
    ) -> Result<(), DurabilityError> {
        if target.runtime_scope() != fence.runtime_scope
            || target.scope_generation() != fence.scope_ownership_generation
        {
            return Ok(());
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let world = fence.runtime_scope.world_id();
        let character = fence.character_id;
        let occurrence = Sha256::new()
            .chain_update(HIT_DOMAIN)
            .chain_update(character.as_bytes())
            .chain_update(target.placement_identity())
            .finalize();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if assert_item_only_fence(&mut tx, &fence.item_fence(), &node).await?.is_err()
                        || !super::super::load(&mut tx, world, epoch).await?.is_some_and(|day| {
                            day.world_counters && matches!(day.counter_stages[CounterKind::MagesTower.index()], 2 | 3)
                        })
                    {
                        return Ok(());
                    }
                    let mage = CounterKind::MagesTower.key();
                    // Lock the phase through the debit. The already locked Character root
                    // serializes this write with that Character's Yielothax kill credits.
                    let stage: i32 = sqlx::query_scalar("SELECT (state->>'stage')::integer FROM game_world_change_counters WHERE world_id=encode($1,'hex')::uuid AND change_key=$2 AND reset_epoch=$3::text::numeric FOR UPDATE")
                        .bind(world.as_bytes().as_slice()).bind(mage).bind(epoch.to_string()).fetch_one(&mut *tx).await?;
                    if matches!(stage, 2 | 3) {
                        sqlx::query("WITH debit AS (
                            UPDATE game_world_change_personal_counters
                            SET counter=0
                            WHERE world_id=encode($1,'hex')::uuid AND change_key=$2
                                AND character_id=encode($3,'hex')::uuid AND counter >= $4::bigint
                                AND NOT EXISTS (SELECT 1 FROM game_world_change_credit_receipts
                                    WHERE world_id=encode($1,'hex')::uuid AND source_kind=$5 AND occurrence=$6)
                            RETURNING world_id
                        ) INSERT INTO game_world_change_credit_receipts(world_id,source_kind,occurrence,reset_epoch,binding_digest)
                            SELECT world_id,$5,$6,$7::text::numeric,$6 FROM debit")
                            .bind(world.as_bytes().as_slice()).bind(mage).bind(character.as_bytes().as_slice())
                            .bind(PERSONAL_KILL_MINIMUM).bind(DAMAGE_RECEIPT).bind(occurrence.as_slice())
                            .bind(epoch.to_string()).execute(&mut *tx).await?;
                    }
                    commit_semantic_transaction(tx, deadline).await
                })
            })
            .await
    }
}
