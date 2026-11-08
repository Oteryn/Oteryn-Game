//! G-8 physical kill batches: current independently resolved scope authority,
//! private occurrence dedup, World/key/day dimensions and outbox commit together.
use super::super::runtime_scope_assignment::NodeIncarnationProof;
use super::*;
use crate::foundation::{CreatureDeathOccurrenceKey, RuntimeScopeRefV1, ScopeOwnershipGeneration};
use serde::Serialize;

pub const KILL_BATCH_MAX: usize = 256;
pub const KILL_PENDING_MAX: usize = 4096;
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PhysicalKill {
    death_key: String,
    creature_key: String,
    boss: bool,
    occurred_ms: i64,
}
impl PhysicalKill {
    /// The sealed physical key cannot be minted from a credited character receipt.
    pub fn capture(
        death: CreatureDeathOccurrenceKey,
        creature: &str,
        boss: bool,
        occurred_ms: i64,
    ) -> Option<Self> {
        if occurred_ms < 0
            || creature.is_empty()
            || creature.len() > 255
            || matches!(creature, "player" | "unknown")
            || !creature
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b":._/-".contains(&b))
        {
            return None;
        }
        let key = super::super::party_kill_xp::encode_death_key(death);
        Some(Self {
            death_key: key.iter().map(|b| format!("{b:02x}")).collect(),
            creature_key: creature.to_owned(),
            boss,
            occurred_ms,
        })
    }
    pub fn occurrence_key(&self) -> &str {
        &self.death_key
    }
    pub const fn occurred_ms(&self) -> i64 {
        self.occurred_ms
    }
}
impl DurabilityRoot {
    /// No runtime/channel mutex is held across this bounded batch transaction.
    #[allow(
        clippy::too_many_arguments,
        reason = "independent scope authority plus bounded source batch/cut"
    )]
    pub async fn flush_public_daily_kills(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
        events: Vec<PhysicalKill>,
        cut_ms: i64,
        gap_ms: Option<i64>,
        final_cut: bool,
    ) -> Result<()> {
        if events.len() > KILL_BATCH_MAX {
            return Err(CharacterAuthorityError::Rejected);
        }
        let recovery = authority.record_for(self)?;
        let node = node.clone();
        let key = super::super::runtime_scope_assignment::assignment_scope_key(scope);
        let payload =
            serde_json::to_value(events).map_err(|_| CharacterAuthorityError::Rejected)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            super::super::db::lock_admission_relations(&mut tx).await?;
            if !super::super::character_progression::assert_runtime_scope_fence(&mut tx,scope,generation,&node).await? && !final_cut {
                return Ok(Err(CharacterAuthorityError::Rejected));
            }
            let fact=node.fact();
            sqlx::query("SELECT game_statistics_kill_batch($1,$2::text::numeric,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7,$8,$9)")
                .bind(key.as_slice()).bind(generation.get().to_string()).bind(fact.node_id().as_bytes().as_slice())
                .bind(fact.registration_revision().to_string()).bind(node.sql_possession_secret()).bind(payload)
                .bind(cut_ms).bind(gap_ms).bind(final_cut).execute(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await?;Ok(Ok(()))
        })).await?
    }
    pub async fn maintain_public_daily_kills(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
    ) -> Result<()> {
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    sqlx::query("SELECT game_statistics_kill_maintenance()")
                        .execute(&mut *tx)
                        .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(()))
                })
            })
            .await?
    }
}
