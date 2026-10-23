//! Staged TRADE-PLAYER-1 durable owner (PLAYER-TRADE0 §§5-6).
//! SESSION-INGRESS-1 owns future issuance and physical exclusion. These storage
//! ports do not admit commands, install a Channel owner or activate a wire family.
//! Freeze the complete request before awaiting storage. On an unknown result retain
//! it and the TRANSFERRING actor indexes; reconcile under the occurrence lock first.

pub mod audit;
mod snapshot;
mod storage;

use super::character_authority::ReconciledCharacterAuthority;
use super::item_loop_move::ItemLoopContent;
use super::item_transfer::{CurrentCharacterItemFence, scope_of};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::spell_owner_commit::SpellLanePermit;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::CommandRef;
use oteryn_gameplay_economy::player_trade::{Refusal, TradeFlag};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Must resolve the actual tradeable flag and pickupability from admitted Content.
/// A Market flag is never a substitute. Unknown *tradeable* is allowed; unknown
/// pickupability is not. The owner re-resolves this at every accept and at commit.
pub trait PlayerTradeContent: ItemLoopContent {
    fn player_trade_flags(
        &self,
        definition: &super::item_mint::TypedDefinitionRef,
    ) -> Option<(TradeFlag, bool)>;
}

/// Immutable complete before-state captured by this owner under both Character locks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeLeaf {
    item: Value,
    entry: Value,
    key: Option<Value>,
    timed: Option<Value>,
    text: Option<Value>,
    text_revision: Option<Value>,
}

/// First terminal accept and second pending accept, with captured session/lease/scope fences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeParticipant {
    pub accepted: CurrentCharacterItemFence,
    pub command: CommandRef,
    pub leaf: TradeLeaf,
}

/// Frozen native request, not a proof of legitimate FND issuance. Participants are
/// always [first accepter, triggering accepter]. Only the first connection may advance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerTradeRequest {
    /// Native counter-offer CommandRef identifying the immutable READY pair.
    pub pair: CommandRef,
    pub occurrence_id: [u8; 16],
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub participants: [TradeParticipant; 2],
    pub content_digest: [u8; 32],
    pub weight_digest: [u8; 32],
    pub node_id: [u8; 16],
    pub node_registration_revision: u64,
    pub content_revision: String,
    pub ruleset_revision: String,
    pub sim_revision: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeReceipt {
    Committed,
    AlreadyCommitted,
    Absent,
}

type Pass<T> = Result<Result<T, Refusal>, DurabilityError>;

fn fence_binding(fence: &CurrentCharacterItemFence) -> Result<Value, DurabilityError> {
    let (world, channel) = scope_of(fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(json!({
        "character": fence.character_id.as_bytes(), "session": fence.game_session_id.as_bytes(),
        "connection": fence.connection_generation.get(), "lease": fence.character_lease_generation,
        "world": world.as_bytes(), "channel": channel.as_bytes(),
        "scope_generation": fence.scope_ownership_generation.get(),
    }))
}

impl PlayerTradeRequest {
    fn binding(&self) -> Result<Vec<u8>, DurabilityError> {
        let parties = self
            .participants
            .iter()
            .map(|p| {
                Ok(json!({
                    "fence": fence_binding(&p.accepted)?, "command": p.command.command_id().get(),
                    "leaf": p.leaf,
                }))
            })
            .collect::<Result<Vec<_>, DurabilityError>>()?;
        serde_json::to_vec(&json!({
            "version": 1, "pair": {"session": self.pair.game_session_id().as_bytes(), "command": self.pair.command_id().get()}, "occurrence": self.occurrence_id, "transaction": self.transaction_id,
            "event": self.event_id, "time": self.occurred_at_unix_ms, "parties": parties,
            "content": self.content_digest, "weights": self.weight_digest,
            "node": self.node_id, "node_revision": self.node_registration_revision,
            "content_revision": self.content_revision, "ruleset_revision": self.ruleset_revision,
            "sim_revision": self.sim_revision,
        }))
        .map_err(|_| DurabilityError::InvalidStoredState)
    }

    fn valid(&self) -> bool {
        let [a, b] = &self.participants;
        self.participants.iter().any(|p| {
            p.accepted.game_session_id == self.pair.game_session_id()
                && self.pair.command_id() < p.command.command_id()
        }) && self.content_digest != [0; 32]
            && self.weight_digest != [0; 32]
            && self.node_registration_revision > 0
            && self.occurred_at_unix_ms > 0
            && a.accepted.character_id != b.accepted.character_id
            && a.accepted.game_session_id != b.accepted.game_session_id
            && a.command.game_session_id() == a.accepted.game_session_id
            && b.command.game_session_id() == b.accepted.game_session_id
            && scope_of(&a.accepted)
                .ok()
                .is_some_and(|s| Some(s) == scope_of(&b.accepted).ok())
            && self
                .participants
                .iter()
                .all(|p| p.accepted.character_lease_generation > 0)
            && [&self.occurrence_id, &self.transaction_id, &self.event_id]
                .iter()
                .all(|id| super::item_mint_audit::check_uuid_v7(*id).is_ok())
            && [
                &self.content_revision,
                &self.ruleset_revision,
                &self.sim_revision,
            ]
            .iter()
            .all(|s| super::item_mint_audit::check_technical_text(s).is_ok())
    }
}

impl DurabilityRoot {
    /// Observe whole direct-backpack leaves under both current fences. Does not
    /// reserve items or authorize a trade; the runtime still owns every L3 check.
    pub async fn observe_player_trade(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fences: [CurrentCharacterItemFence; 2],
        commands: [CommandRef; 2],
        items: [[u8; 16]; 2],
        content_digest: [u8; 32],
    ) -> Pass<[TradeLeaf; 2]> {
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                    if !snapshot::fence_pair(
                        &mut tx,
                        &record,
                        &node,
                        &fences,
                        &commands,
                        content_digest,
                    )
                    .await?
                    {
                        return Ok(Err(Refusal::Rejected));
                    }
                    let leaves = snapshot::read_pair(&mut tx, &fences, items).await?;
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(leaves)
                })
            })
            .await
    }

    /// One type-2 transaction, replay before live fences, no persistent reservation.
    /// Unknown storage/COMMIT results must keep TRANSFERRING until reconciliation.
    #[allow(clippy::too_many_arguments)]
    pub async fn exchange_player_trade<C: PlayerTradeContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        current: [CurrentCharacterItemFence; 2],
        content: &C,
        request: PlayerTradeRequest,
    ) -> Pass<TradeReceipt> {
        if !request.valid() {
            return Ok(Err(Refusal::Rejected));
        }
        permit.check_scope(request.participants[0].accepted.runtime_scope)?;
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = (permit, content);
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    let mut tx = super::db::begin_type2_transaction(holder, deadline).await?;
                    super::character_authority::assert_recovery_fence(&mut tx, &record).await?;
                    super::db::lock_admission_relations(&mut tx).await?;
                    storage::lock_occurrence(&mut tx, request.occurrence_id).await?;
                    let intent = request.binding()?;
                    if intent.len() > 32768 {
                        return Ok(Err(Refusal::Rejected));
                    }
                    match storage::lookup(&mut tx, &request, &intent).await? {
                        Ok(TradeReceipt::Absent) => {}
                        result => return Ok(result),
                    }
                    let result = storage::apply(
                        &mut tx, &record, &node, &current, context.1, &request, &intent,
                    )
                    .await?;
                    if result.is_ok() {
                        tx.commit(deadline).await?;
                    }
                    Ok(result)
                })
            })
            .await
    }

    /// A matching receipt is historical evidence: no current session/content fence
    /// is required. Absent is proven only after the in-flight occurrence lock.
    pub async fn reconcile_player_trade(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        request: PlayerTradeRequest,
    ) -> Pass<TradeReceipt> {
        if !request.valid() {
            return Ok(Err(Refusal::Rejected));
        }
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                    super::character_authority::assert_recovery_fence(&mut tx, &record).await?;
                    storage::lock_occurrence(&mut tx, request.occurrence_id).await?;
                    let result = storage::lookup(&mut tx, &request, &request.binding()?).await?;
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await
    }
}
