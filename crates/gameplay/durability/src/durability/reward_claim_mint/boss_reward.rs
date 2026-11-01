//! W03 personal reward collection through the existing reward-claim custody owner.
//! The command consumes immutable entitlements and mints fresh ItemInstances together.
//! Current owner/session/lease fences precede replay; only Content and reach are fresh-use checks.
use crate::durability::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use crate::durability::character_progression::assert_item_only_fence;
use crate::durability::db::{begin_type2_transaction, lock_admission_relations};
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::{DurabilityError, DurabilityRoot};
use crate::foundation::{CommandRef, RuntimeScopeRefV1};
use sqlx::{Postgres, Transaction};

mod browse;
mod delivery;
mod store;
#[cfg(test)]
mod tests;

/// Crystal's `collectAll` uses the configured maxMoveItems and may deliver a prefix.
/// The existing main-backpack custody owner has twenty direct destination slots.
pub const BOSS_REWARD_COLLECTION_MAX: usize = 20;
pub const BOSS_REWARD_PAGE_SIZE: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossRewardSelection {
    Item { death: [u8; 16], ordinal: u16 },
    Group { death: [u8; 16] },
    All,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossRewardCollectionStatus {
    Collected,
    Empty,
    Expired,
    NoRoom,
    Rejected,
    ContentUnavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BossRewardCollection {
    pub status: BossRewardCollectionStatus,
    pub moved: u16,
    pub remaining: u32,
}

impl BossRewardCollection {
    pub const fn refused(status: BossRewardCollectionStatus) -> Self {
        Self {
            status,
            moved: 0,
            remaining: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BossRewardCollectRequest {
    pub command: CommandRef,
    pub selection: BossRewardSelection,
    /// Exact command kind and bounded payload, retained independently of live map handles.
    pub intent: Vec<u8>,
    /// Current server-resolved physical reward chest. A replay does not re-resolve it.
    pub source_placement: String,
    pub content_revision: String,
    pub ruleset_revision: String,
    pub sim_revision: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BossRewardPageItem {
    pub ordinal: u16,
    pub definition: TypedDefinitionRef,
    pub quantity: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BossRewardPage {
    pub status: BossRewardCollectionStatus,
    pub death: [u8; 16],
    pub expires_at_unix_ms: u64,
    pub items: Vec<BossRewardPageItem>,
    pub more: bool,
}

impl DurabilityRoot {
    /// `probe` returns only an existing exact receipt, without requiring a physical chest.
    /// A fresh application requires the native reach proof, current participant fence and
    /// exact Content pin. Offline award creation never authorizes a stale collection command.
    #[allow(clippy::too_many_arguments)]
    pub async fn collect_boss_rewards<C: ItemLoopContent>(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: BossRewardCollectRequest,
        content: &C,
        probe: bool,
    ) -> Result<Option<BossRewardCollection>, DurabilityError> {
        if request.command.game_session_id() != fence.game_session_id
            || !valid_selection(request.selection)
            || request.intent.is_empty()
            || request.intent.len() > 32
        {
            return Ok(Some(BossRewardCollection::refused(
                BossRewardCollectionStatus::Rejected,
            )));
        }
        permit.check_scope(fence.runtime_scope)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let root = self.clone();
        let mut context = content;
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, content| {
                Box::pin(async move {
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    // A retained response never authorizes a displaced session, lease or node.
                    // Reach and current Content are fresh-application checks; owner fences precede replay.
                    if assert_item_only_fence(&mut tx, &fence, &node)
                        .await?
                        .is_err()
                    {
                        return Ok(Some(BossRewardCollection::refused(
                            BossRewardCollectionStatus::Rejected,
                        )));
                    }
                    let current =
                        crate::durability::fresh_admission::FreshAdmissionStore::from_root(root)
                            .current_session_in_transaction(&mut tx, fence.game_session_id)
                            .await?;
                    if current.session_state() != crate::foundation::GameSessionState::Active
                        || current.current_game_session_id() != fence.game_session_id
                        || current.commit().character_id().as_bytes()
                            != fence.character_id.as_bytes()
                        || current.current_connection_generation() != fence.connection_generation
                        || current.current_character_lease().generation()
                            != fence.character_lease_generation
                        || current.current_runtime_scope() != fence.runtime_scope
                        || current.current_scope_generation() != fence.scope_ownership_generation
                    {
                        return Ok(Some(BossRewardCollection::refused(
                            BossRewardCollectionStatus::Rejected,
                        )));
                    }
                    store::lock_command(&mut tx, request.command).await?;
                    if let Some(result) = store::receipt(&mut tx, fence, &request).await? {
                        tx.commit(deadline).await?;
                        return Ok(Some(result));
                    }
                    if probe {
                        tx.commit(deadline).await?;
                        return Ok(None);
                    }
                    if !valid_commit_request(&request)
                        || !admit(&mut tx, &node, &fence, content.loop_content_digest()).await?
                    {
                        return Ok(Some(BossRewardCollection::refused(
                            BossRewardCollectionStatus::Rejected,
                        )));
                    }
                    let now = store::now(&mut tx).await?;
                    store::expire(&mut tx, fence, now).await?;
                    let rows = store::pending(&mut tx, fence, request.selection, now).await?;
                    let result = if rows.is_empty() {
                        let expired =
                            store::selected_group_expired(&mut tx, fence, request.selection, now)
                                .await?;
                        BossRewardCollection::refused(if expired {
                            BossRewardCollectionStatus::Expired
                        } else {
                            BossRewardCollectionStatus::Empty
                        })
                    } else {
                        delivery::deliver(&mut tx, fence, &request, *content, rows, now).await?
                    };
                    store::record_receipt(
                        &mut tx,
                        fence,
                        &request,
                        result,
                        content.loop_content_digest(),
                        now,
                    )
                    .await?;
                    tx.commit(deadline).await?;
                    Ok(Some(result))
                })
            })
            .await
    }
}

fn valid_selection(selection: BossRewardSelection) -> bool {
    match selection {
        BossRewardSelection::All => true,
        BossRewardSelection::Group { death } => death != [0; 16],
        BossRewardSelection::Item { death, ordinal } => {
            death != [0; 16] && (1..=1_000).contains(&ordinal)
        }
    }
}

fn valid_commit_request(request: &BossRewardCollectRequest) -> bool {
    use crate::durability::item_mint_audit::check_content_key;
    [
        &request.source_placement,
        &request.content_revision,
        &request.ruleset_revision,
        &request.sim_revision,
    ]
    .iter()
    .all(|field| check_content_key(field).is_ok())
}

/// Instance content comes from its committed handoff pin; a Channel's latest activation is
/// irrelevant to an active Instance. The existing generic item fence locks the Character root.
async fn admit(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    content_digest: [u8; 32],
) -> Result<bool, DurabilityError> {
    if content_digest == [0; 32] {
        return Ok(false);
    }
    let world = fence.runtime_scope.world_id();
    if let RuntimeScopeRefV1::Channel { channel_id, .. } = fence.runtime_scope {
        sqlx::query("SELECT pg_advisory_xact_lock_shared(hashtextextended('oteryn:content-activation:' || encode($1::bytea,'hex')::uuid::text || ':' || encode($2::bytea,'hex')::uuid::text,0))")
            .bind(world.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).execute(&mut **tx).await?;
    }
    if assert_item_only_fence(tx, fence, node).await?.is_err() {
        return Ok(false);
    }
    let digest: Option<Vec<u8>> = match fence.runtime_scope {
        RuntimeScopeRefV1::Channel { channel_id, .. } => sqlx::query_scalar("SELECT server_artifact_digest FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid ORDER BY activation_sequence DESC LIMIT 1")
            .bind(world.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?,
        RuntimeScopeRefV1::Instance { instance_id, .. } => sqlx::query_scalar("SELECT e.destination_server_artifact_digest FROM game_activity_handoff_groups g JOIN game_activity_runtime_entries e USING(handoff_id) WHERE g.world_id=encode($1,'hex')::uuid AND g.instance_id=encode($2,'hex')::uuid AND g.ownership_generation=$3::text::numeric AND g.state=2")
            .bind(world.as_bytes().as_slice()).bind(instance_id.as_slice()).bind(fence.scope_ownership_generation.get().to_string()).fetch_optional(&mut **tx).await?,
    };
    Ok(digest.as_deref() == Some(content_digest.as_slice()))
}
