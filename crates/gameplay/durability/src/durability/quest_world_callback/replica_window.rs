//! Crystal 201189f, actions_servants_mechanism.lua:1-56.
use super::{CurrentCharacterGameplayFence, DurabilityError, SpellLanePermit};
use crate::durability::{
    DurabilityRoot,
    character_authority::{ReconciledCharacterAuthority, assert_recovery_fence},
    character_equipment::assert_current_equipment_in_transaction,
    runtime_scope_assignment::NodeIncarnationProof,
};
use crate::foundation::RuntimeScopeRefV1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplicaKind {
    Diamond,
    Golden,
}

impl ReplicaKind {
    fn number(self) -> i16 {
        match self {
            Self::Diamond => 1,
            Self::Golden => 2,
        }
    }

    pub(crate) fn from_death_transition(key: &str) -> Option<Self> {
        match key {
            "oteryn:quest-transition/forgotten-knowledge/replica-credit/diamond" => {
                Some(Self::Diamond)
            }
            "oteryn:quest-transition/forgotten-knowledge/replica-credit/golden" => {
                Some(Self::Golden)
            }
            _ => None,
        }
    }
}

impl DurabilityRoot {
    pub async fn open_replica_window(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        native_digest: [u8; 32],
        kind: ReplicaKind,
    ) -> Result<bool, DurabilityError> {
        permit.check_scope(fence.runtime_scope)?;
        let RuntimeScopeRefV1::Channel { world_id, .. } = fence.runtime_scope else {
            return Ok(false);
        };
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = crate::durability::db::begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &record).await?;
            assert_current_equipment_in_transaction(
                &mut tx, &record, &node, &fence.item_fence(), None, native_digest
            ).await.map_err(|_| DurabilityError::Unavailable)?;
            let opened = sqlx::query(
                "INSERT INTO game_forgotten_knowledge_replica_windows (world_id, replica_kind, expires_at)
                 VALUES (encode($1,'hex')::uuid, $2, statement_timestamp() + interval '10 minutes')
                 ON CONFLICT (world_id, replica_kind) DO UPDATE SET
                    expires_at=EXCLUDED.expires_at, kills=0
                 WHERE game_forgotten_knowledge_replica_windows.expires_at <= statement_timestamp()",
            )
            .bind(world_id.as_bytes().as_slice())
            .bind(kind.number())
            .execute(&mut **tx).await?.rows_affected() == 1;
            tx.commit(deadline).await?;
            Ok(opened)
        })).await
    }
}

pub(crate) async fn credit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    scope: RuntimeScopeRefV1,
    kind: ReplicaKind,
) -> Result<(), DurabilityError> {
    if let RuntimeScopeRefV1::Channel { world_id, .. } = scope {
        sqlx::query(
            "UPDATE game_forgotten_knowledge_replica_windows
             SET kills=kills+1
             WHERE world_id=encode($1,'hex')::uuid AND replica_kind=$2
               AND expires_at > statement_timestamp() AND kills < 5",
        )
        .bind(world_id.as_bytes().as_slice())
        .bind(kind.number())
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}
