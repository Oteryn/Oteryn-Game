//! A completed activity uses the existing frozen one-item retirement owner.
//! The retained handoff is a cause, never an assignment or a runtime grant.
use super::*;
use crate::durability::activity_handoff::{
    RetiringActivity, read_retiring_activity_in_transaction,
};

/// One bounded durable step. Only `Complete` permits the caller to remove an
/// empty native scope; native actor/reservation absence must also be checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivityGroundRetirement {
    Progress(CommittedDecayRetire),
    Complete,
}

/// A decoded receipt carries historical facts. It is intentionally distinct
/// from the sealed `RetiringActivity` read proof and cannot grant authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ActivityRetirementBinding {
    pub(super) handoff_id: [u8; 16],
    pub(super) scope: RuntimeScopeRefV1,
    pub(super) generation: ScopeOwnershipGeneration,
}

impl From<&RetiringActivity> for ActivityRetirementBinding {
    fn from(proof: &RetiringActivity) -> Self {
        Self {
            handoff_id: proof.handoff_id(),
            scope: proof.scope(),
            generation: proof.generation(),
        }
    }
}

pub(super) async fn binding_current(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    fence: &CorpseDecayFence,
    binding: ActivityRetirementBinding,
) -> std::result::Result<bool, DurabilityError> {
    if fence.runtime_scope != binding.scope
        || fence.scope_ownership_generation.get() < binding.generation.get()
    {
        return Ok(false);
    }
    let current: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_runtime_scope_assignments WHERE scope_key=$1 AND ownership_generation=$2::text::numeric)")
        .bind(assignment_scope_key(binding.scope).as_slice()).bind(fence.scope_ownership_generation.get().to_string()).fetch_one(&mut **tx).await?;
    if !current {
        return Ok(false);
    }
    Ok(read_retiring_activity_in_transaction(
        tx,
        node,
        binding.handoff_id,
        binding.scope,
        binding.generation,
    )
    .await?
    .is_some())
}

/// Activity replay remains fenced. Legacy Channel retirement replay retains
/// its existing historical-result behavior.
pub(super) async fn replay_current(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: Option<&NodeIncarnationProof>,
    cause: RetireCause,
    generation: u64,
) -> std::result::Result<bool, DurabilityError> {
    let RetireCause::ActivityRetirement(binding) = cause else {
        return Ok(true);
    };
    let Some(node) = node else {
        return Ok(false);
    };
    binding_current(
        tx,
        node,
        &CorpseDecayFence {
            runtime_scope: binding.scope,
            scope_ownership_generation: ScopeOwnershipGeneration::new(generation)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        },
        binding,
    )
    .await
}

/// A revoked, fully returned Instance may be cleared by the retained activity
/// owner. It must not be mistaken for ordinary current-runtime authority.
pub(super) async fn cause_fence_is_live(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    fence: &CorpseDecayFence,
    cause: RetireCause,
) -> std::result::Result<bool, DurabilityError> {
    match cause {
        RetireCause::ActivityRetirement(binding) => binding_current(tx, node, fence, binding).await,
        _ => fence_is_live(tx, fence, node).await,
    }
}

impl DurabilityRoot {
    /// Select one remaining root/entry from durable Ground, revalidate the
    /// complete returned group, then use the same reservation, receipt, audit
    /// and one-item commit as corpse decay and World reset. Unknown commits
    /// retry from those receipts; there is no second progress journal.
    pub async fn retire_activity_ground_step(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        activity: &RetiringActivity,
    ) -> Result<ActivityGroundRetirement> {
        permit.check_scope(activity.scope())?;
        let RuntimeScopeRefV1::Instance {
            world_id,
            instance_id,
        } = activity.scope()
        else {
            return Err(DecayRetireError::InvalidInput);
        };
        let recovery = authority
            .record_for(self)
            .map_err(|_| DecayRetireError::AuthorityRejected)?;
        let binding = ActivityRetirementBinding::from(activity);
        let selected_node = node.clone();
        let step = self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            // The control writer's current generation is the cleanup fence;
            // the immutable cause retains the original activity generation.
            let generation: Option<String> = sqlx::query_scalar("SELECT ownership_generation::text FROM game_runtime_scope_assignments WHERE scope_key=$1 FOR SHARE")
                .bind(assignment_scope_key(binding.scope).as_slice()).fetch_optional(&mut *tx).await?;
            let Some(generation) = generation else { return Ok(Err(DecayRetireError::AuthorityRejected)); };
            let fence = CorpseDecayFence { runtime_scope: binding.scope,
                scope_ownership_generation: ScopeOwnershipGeneration::new(generation.parse().map_err(|_| DurabilityError::InvalidStoredState)?)
                    .map_err(|_| DurabilityError::InvalidStoredState)? };
            if !binding_current(&mut tx, &selected_node, &fence, binding).await? {
                return Ok(Err(DecayRetireError::AuthorityRejected));
            }
            // The exact scope predicate excludes origin Channel items even
            // when coordinates or UUID bytes happen to coincide.
            let root: Option<String> = sqlx::query_scalar("SELECT g.item_instance_id::text FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.scope_kind=2 AND g.world_id=encode($1,'hex')::uuid AND g.instance_id=encode($2,'hex')::uuid AND i.lifecycle=1 ORDER BY g.item_instance_id LIMIT 1")
                .bind(world_id.as_bytes().as_slice()).bind(instance_id.as_slice()).fetch_optional(&mut *tx).await?;
            let next = if let Some(root) = root {
                let root = uuid_text(&root)?;
                crate::durability::world_reset::ground_step::validate_root(&mut tx, root).await?;
                let entry: Option<String> = sqlx::query_scalar("SELECT item_instance_id::text FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid ORDER BY placement_ordinal LIMIT 1")
                    .bind(root.as_slice()).fetch_optional(&mut *tx).await?;
                let item = entry.as_deref().map(uuid_text).transpose()?.unwrap_or(root);
                let mut cause = RetireCause::ActivityRetirement(binding);
                // A due corpse timer may already have frozen its original
                // one-item operation in this generation. Finish that exact
                // operation; never replace its immutable cause or identities.
                if let Some(row) = load_reservation(&mut tx, item, &fence.scope_ownership_generation.get().to_string()).await? {
                    let pending = decode_reservation(&row)?;
                    if pending.cause == RetireCause::CorpseDecay && pending.corpse_item_instance_id == root {
                        cause = RetireCause::CorpseDecay;
                    }
                }
                Some((fence, DecayRetireStep { corpse_item_instance_id: root, item_instance_id: item, cause }))
            } else { None };
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(next))
        })).await??;
        let Some((fence, step)) = step else {
            return Ok(ActivityGroundRetirement::Complete);
        };
        Ok(ActivityGroundRetirement::Progress(
            self.retire_step(permit, authority, node, fence, step)
                .await?,
        ))
    }
}
