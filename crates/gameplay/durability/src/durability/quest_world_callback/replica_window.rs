//! Crystal 201189f, actions_servants_mechanism.lua:1-56.
use super::{CurrentCharacterGameplayFence, DurabilityError, SpellLanePermit};
use crate::durability::{
    DurabilityRoot, character_authority::ReconciledCharacterAuthority,
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
    pub const fn index(self) -> usize {
        match self {
            Self::Diamond => 0,
            Self::Golden => 1,
        }
    }

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplicaPortalAccess {
    Dormant,
    InsufficientEnergy,
    LloydCooldown,
    Ready,
}

impl DurabilityRoot {
    pub async fn read_replica_portal(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        native_digest: [u8; 32],
    ) -> Result<(ReplicaPortalAccess, u64), DurabilityError> {
        let RuntimeScopeRefV1::Channel { world_id, .. } = fence.runtime_scope else {
            return Ok((ReplicaPortalAccess::Dormant, 0));
        };
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = crate::durability::db::begin_type2_transaction(holder, deadline).await?;
            assert_current_equipment_in_transaction(
                &mut tx, &record, &node, &fence.item_fence(), None, native_digest
            ).await.map_err(|_| DurabilityError::Unavailable)?;
            let (remaining, cooldown, charged): (i64, bool, bool) = sqlx::query_as(
                "SELECT
                   (SELECT CASE WHEN count(*) = 2 THEN
                        floor(extract(epoch FROM (min(expires_at) - statement_timestamp()))*1000000)::bigint
                      ELSE 0 END FROM game_forgotten_knowledge_replica_windows
                    WHERE world_id=encode($1,'hex')::uuid AND kills=5
                      AND expires_at > statement_timestamp()),
                   EXISTS (SELECT 1 FROM game_character_boss_cooldowns
                    WHERE character_id=encode($2,'hex')::uuid
                      AND boss_key IN ('oteryn:boss.lloyd','oteryn:creature.lloyd')
                      AND available_at > floor(extract(epoch FROM statement_timestamp())*1000)::bigint),
                   (SELECT count(*) = 2 FROM game_character_quest_tracks
                    WHERE character_id=encode($2,'hex')::uuid AND track_key IN ($3,$4) AND value >= 5)",
            )
            .bind(world_id.as_bytes().as_slice())
            .bind(fence.character_id.as_bytes().as_slice())
            .bind("oteryn:quest-progress/quest/u11_02/forgotten_knowledge/diamond_servant_counter")
            .bind("oteryn:quest-progress/quest/u11_02/forgotten_knowledge/golden_servant_counter")
            .fetch_one(&mut **tx).await?;
            tx.commit(deadline).await?;
            let remaining = u64::try_from(remaining).map_err(|_| DurabilityError::InvalidStoredState)?;
            let access = match (remaining > 0, cooldown, charged) {
                (false, _, _) => ReplicaPortalAccess::Dormant,
                (true, true, _) => ReplicaPortalAccess::LloydCooldown,
                (true, false, false) => ReplicaPortalAccess::InsufficientEnergy,
                (true, false, true) => ReplicaPortalAccess::Ready,
            };
            Ok((access, remaining))
        })).await
    }

    pub async fn open_replica_window(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        native_digest: [u8; 32],
        kind: ReplicaKind,
    ) -> Result<Option<[Option<u64>; 2]>, DurabilityError> {
        permit.check_scope(fence.runtime_scope)?;
        let RuntimeScopeRefV1::Channel { world_id, .. } = fence.runtime_scope else {
            return Ok(None);
        };
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = crate::durability::db::begin_type2_transaction(holder, deadline).await?;
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
            let mut windows = [None; 2];
            if opened {
                let rows: Vec<(i16, i64)> = sqlx::query_as(
                    "SELECT replica_kind,
                        floor(extract(epoch FROM (expires_at - statement_timestamp()))*1000000)::bigint
                     FROM game_forgotten_knowledge_replica_windows
                     WHERE world_id=encode($1,'hex')::uuid AND expires_at > statement_timestamp()",
                )
                .bind(world_id.as_bytes().as_slice())
                .fetch_all(&mut **tx)
                .await?;
                for (kind, remaining) in rows {
                    let index = usize::try_from(kind - 1)
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    *windows.get_mut(index).ok_or(DurabilityError::InvalidStoredState)? =
                        Some(u64::try_from(remaining).map_err(|_| DurabilityError::InvalidStoredState)?);
                }
            }
            tx.commit(deadline).await?;
            Ok(opened.then_some(windows))
        })).await
    }
}

pub(crate) async fn credit(
    tx: &mut sqlx::PgConnection,
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
        .execute(&mut *tx)
        .await?;
    }
    Ok(())
}
