//! Durable NPC participant. Only lower-layer request and operation traits cross this boundary.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence, numeric_u64,
    state_matches_root, uuid_text,
};
use super::db::{begin_type2_transaction, lock_admission_relations};
use super::item_fee_burn::{
    FeeBurnCause, FeeBurnError, FeeBurnRequest, FeeChangeFacts, burn_fee_in_transaction,
};
use super::item_mint_audit::{Type2Transaction, check_uuid_v7};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::{CharacterId, CharacterRevision};
use crate::foundation::CommandRef;
use sqlx::Row;

mod receipt;
mod recovery;
pub(super) use receipt::load_by_command;
pub(super) use recovery::verify_purchase_integrity;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PurchaseService {
    Promotion(String),
    Blessing(String),
}
impl PurchaseService {
    pub fn key(&self) -> &str {
        match self {
            Self::Promotion(key) | Self::Blessing(key) => key,
        }
    }
    fn kind(&self) -> i16 {
        match self {
            Self::Promotion(_) => 2,
            Self::Blessing(_) => 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NpcPurchaseRequest {
    pub command: CommandRef,
    pub occurrence: [u8; 16],
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub account: [u8; 16],
    pub npc_key: String,
    pub npc_identity: [u8; 16],
    pub npc_generation: u64,
    pub service: PurchaseService,
    pub coins: FeeChangeFacts,
    pub expected_backpack: Option<[u8; 16]>,
    pub occurred_at_unix_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurchaseRefusal {
    NotEligible,
    AlreadyHeld,
    PremiumRequired,
    InsufficientFunds,
}

#[derive(Debug, Clone)]
pub struct PurchaseReceipt {
    pub character: CharacterId,
    pub original_revision: CharacterRevision,
    pub committed_revision: CharacterRevision,
    pub npc_identity: [u8; 16],
    pub npc_generation: u64,
    pub service: PurchaseService,
    pub price: u64,
}

#[derive(Debug, Clone)]
pub enum PurchaseOutcome {
    Committed(PurchaseReceipt),
    Refused(PurchaseRefusal),
}

/// The native Premium owner borrows its status inside the same physical transaction.
/// An error rolls the participant back; no nested transaction or savepoint is admitted.
pub trait NpcPurchaseGate: Sync {
    fn authorize(
        &self,
        tx: &mut Type2Transaction<'_>,
        account: [u8; 16],
        service: &PurchaseService,
    ) -> impl Future<Output = Result<bool, DurabilityError>> + Send;
}

impl DurabilityRoot {
    pub(super) async fn commit_npc_purchase<G: NpcPurchaseGate>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: NpcPurchaseRequest,
        gate: &G,
    ) -> Result<PurchaseOutcome, CharacterProgressionError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let mut gate = gate;
        self.try_issue_semantic_pass()?.run_with_context(&mut gate, move |holder, deadline, gate| {
            Box::pin(async move {
                let mut tx = begin_type2_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                // Same occurrence is serialized before replay or fresh authority resolution.
                let occurrence = request.occurrence;
                sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex'),138))")
                    .bind(occurrence.as_slice()).execute(&mut **tx).await?;
                let binding = receipt::binding(&fence, &request);
                if let Some((stored, receipt)) = load_by_command(&mut tx, fence.character_id, request.command).await? {
                    if stored != binding { return Ok(Err(CharacterProgressionError::ConflictingOccurrence)); }
                    tx.commit(deadline).await?;
                    return Ok(Ok(PurchaseOutcome::Committed(receipt)));
                }
                if request.npc_generation == 0 || request.occurred_at_unix_ms <= 0
                    || check_uuid_v7(&request.occurrence).is_err()
                { return Ok(Err(CharacterProgressionError::InvalidInput)); }
                if !gate.authorize(&mut tx,request.account,&request.service).await? {
                    return Ok(Ok(PurchaseOutcome::Refused(PurchaseRefusal::PremiumRequired)));
                }
                let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                    Ok(root) => root, Err(error) => return Ok(Err(error)),
                };
                let account: String = sqlx::query_scalar("SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
                if uuid_text(&account)? != request.account { return Ok(Err(CharacterProgressionError::AuthorityRejected)); }
                let state = sqlx::query("SELECT character_revision::text,level,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
                let Some(state) = state else { return Ok(Err(CharacterProgressionError::MissingProgressionState)); };
                if numeric_u64(&state,"character_revision")? != root.revision || !state_matches_root(&state,&root) {
                    return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
                }
                let pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid)")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
                if pending { return Ok(Err(CharacterProgressionError::RespawnPending)); }
                let backpack: Option<String> = sqlx::query_scalar("SELECT item_instance_id::text FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
                if backpack.as_deref().map(uuid_text).transpose()? != request.expected_backpack {
                    return Ok(Err(CharacterProgressionError::AuthorityRejected));
                }
                let level = u32::try_from(state.try_get::<i64,_>("level")?).map_err(|_| DurabilityError::InvalidStoredState)?;
                let quote = match &request.service {
                    PurchaseService::Promotion(_) => super::promotion_purchase::quote(&mut tx,fence.character_id,&request.npc_key,&request.service,level).await?,
                    PurchaseService::Blessing(_) => super::blessing_purchase::quote(&mut tx,fence.character_id,&request.npc_key,&request.service,level).await?,
                };
                let (price,blessing) = match quote {
                    Ok(quote) => quote,
                    Err(refusal) => return Ok(Ok(PurchaseOutcome::Refused(refusal))),
                };
                // Bind the participant to this physical XID before any fee/effect write.
                let xid: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text").fetch_one(&mut **tx).await?;
                let cause = match &request.service {
                    PurchaseService::Promotion(service) => FeeBurnCause::PromotionPurchase { service: service.clone(), occurrence },
                    PurchaseService::Blessing(service) => FeeBurnCause::BlessingPurchase { service: service.clone(), occurrence },
                };
                let fee = FeeBurnRequest { cause, fee_gold_units: price,
                    transaction_id: request.transaction_id,
                    event_id: request.event_id,
                    occurred_at_unix_ms: request.occurred_at_unix_ms, server_build_id: "oteryn-game-server".into(), change: request.coins.clone() };
                match burn_fee_in_transaction(&mut tx,&fence,&fee).await {
                    Ok(_) => (),
                    Err(FeeBurnError::InsufficientFunds | FeeBurnError::ChangeDoesNotFit | FeeBurnError::CapacityExceeded) => return Ok(Ok(PurchaseOutcome::Refused(PurchaseRefusal::InsufficientFunds))),
                    Err(FeeBurnError::Unavailable(error)) => return Err(error),
                    Err(_) => return Ok(Err(CharacterProgressionError::InvalidInput)),
                }
                let current_xid: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text").fetch_one(&mut **tx).await?;
                if xid != current_xid { return Err(DurabilityError::InvalidStoredState); }
                receipt::insert(&mut tx,&fence,&request,&fee,blessing.as_deref(),&binding).await?;
                match blessing {
                    Some(key) => super::blessing_purchase::grant(&mut tx,fence.character_id,&key,occurrence).await?,
                    None => super::promotion_purchase::grant(&mut tx,fence.character_id,occurrence).await?,
                }
                let receipt = load_by_command(&mut tx,fence.character_id,request.command).await?.ok_or(DurabilityError::InvalidStoredState)?.1;
                tx.commit(deadline).await?;
                Ok(Ok(PurchaseOutcome::Committed(receipt)))
            })
        }).await?
    }
}
