//! PREM-2b policy and D1106 native one-transaction promotion source.
//!
//! ACCEPT_BATCH (2026-10-15) releases the NPC source and accepts WHEEL-0 §6.2's
//! pre-delivery exception. Callers borrow the native PremiumStatus inside the gate
//! closure once per command/use; the exception retains its native PreDelivery witness.
//! No entitlement classification or reusable authority is constructed here. The NPC
//! conversation (`npc_promotion`) is the only caller; migration 0137 installs the schema.

use crate::content::{ProjectV2DefinitionRef, ProjectV2Family};
use crate::domain::premium::{PremiumRefusal, PremiumStatus, require_promotion_premium};
use crate::domain::{CharacterId, CharacterRevision};
use crate::durability::DurabilityError;
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use crate::durability::character_revision_sequencer::{SequencedError, SequencedOutcome};
use crate::durability::item_fee_burn::{
    FeeBurnCause, FeeBurnError, FeeBurnOutcome, FeeBurnRequest, burn_fee_in_transaction,
};
use crate::durability::item_mint_audit::{Type2Transaction, check_technical_text, check_uuid_v7};
use crate::durability::npc_purchase::NpcPurchaseWriteContext;
use crate::gameplay_transport::npc_promotion::promotion_coin_fee_quote;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use sha2::{Digest, Sha256};
use sqlx::Row;

/// Borrowed once inside the native Premium gate. The durability port commits
/// the physical transaction before this operation's pending answer is exposed.
pub(crate) struct PromotionPurchaseOperation<'r, 'p> {
    pub(crate) fence: CurrentCharacterGameplayFence,
    pub(crate) request: &'r PromotionPurchaseRequest,
    pub(crate) premium: &'r PremiumStatus<'p>,
}

impl crate::durability::npc_purchase::NpcPurchaseOperation for PromotionPurchaseOperation<'_, '_> {
    type Output = PendingPromotionPurchase;
    type Error = PromotionPurchaseError;

    fn apply<'a, 't>(
        self,
        context: &'a NpcPurchaseWriteContext,
        tx: &'a mut Type2Transaction<'t>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Output, Self::Error>> + Send + 'a>,
    >
    where
        Self: 'a,
    {
        Box::pin(async move {
            prepare_promotion_purchase(context, tx, self.fence, self.request, self.premium).await
        })
    }
}

/// PREMIUM_ACTIVATION §4.2: promotion starts at level 20.
pub(crate) const PROMOTION_MINIMUM_LEVEL: u32 = 20;
pub(crate) const PROMOTION_PRICE: u64 = 20_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PromotionRefusal {
    Premium(PremiumRefusal),
    LevelTooLow,
    AlreadyPromoted,
}

/// Business eligibility only. Level/promotion inputs are unqualified values, not a locked
/// Character read or a session fence. A successful check neither spends coins nor grants a
/// durable promotion; the future fenced transaction must read and check its current facts.
pub(crate) fn check_promotion_eligibility(
    level: u32,
    durable_promoted: bool,
    premium: &PremiumStatus<'_>,
) -> Result<(), PromotionRefusal> {
    require_promotion_premium(premium).map_err(PromotionRefusal::Premium)?;
    if level < PROMOTION_MINIMUM_LEVEL {
        return Err(PromotionRefusal::LevelTooLow);
    }
    if durable_promoted {
        return Err(PromotionRefusal::AlreadyPromoted);
    }
    Ok(())
}

/// The promoted name/title projection chosen at login (§4.2 D73/D76). It is not a gameplay
/// permission: benefits re-read the native rule at each use, and a lapse never clears durable
/// promotion. Vocation/title selection and login composition stay with their existing owners.
#[allow(
    dead_code,
    reason = "Pending CP login projection caller retains this API."
)]
#[must_use]
pub(crate) const fn effective_promotion_at_login(
    durable_promoted: bool,
    premium: &PremiumStatus<'_>,
) -> bool {
    durable_promoted && require_promotion_premium(premium).is_ok()
}

/// Server-resolved NPC/service provenance, retained by the source receipt.
/// These immutable inputs do not establish current actor/session authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromotionNpcSource {
    pub(crate) npc: ProjectV2DefinitionRef,
    pub(crate) service: ProjectV2DefinitionRef,
    pub(crate) actor: EntityRef,
    pub(crate) content_revision: String,
}

/// Shared NPC purchase provenance data, never current authority or a reusable permit.
pub(crate) type NpcPurchaseSource = PromotionNpcSource;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromotionPurchaseRequest {
    pub(crate) occurrence: [u8; 16],
    pub(crate) account_id: [u8; 16],
    pub(crate) source: PromotionNpcSource,
    pub(crate) fee: FeeBurnRequest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommittedPromotionPurchase {
    pub(crate) occurrence: [u8; 16],
    pub(crate) character_id: CharacterId,
    pub(crate) original_revision: CharacterRevision,
    pub(crate) committed_revision: CharacterRevision,
    pub(crate) fee_transaction: [u8; 16],
    pub(crate) fee_event: [u8; 16],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PromotionPurchaseOutcome {
    Purchased(CommittedPromotionPurchase),
    AlreadyPurchased(CommittedPromotionPurchase),
}

impl PromotionPurchaseOutcome {
    pub(crate) fn receipt(&self) -> &CommittedPromotionPurchase {
        let (Self::Purchased(receipt) | Self::AlreadyPurchased(receipt)) = self;
        receipt
    }
}

/// Kept inside the source's commit operation. It may leave the native Premium
/// gate only after that gate's actual SQL COMMIT has succeeded.
pub(crate) struct PendingPromotionPurchase(PromotionPurchaseOutcome);

impl PendingPromotionPurchase {
    pub(crate) fn after_native_commit(self) -> PromotionPurchaseOutcome {
        self.0
    }
}

#[derive(Debug)]
pub(crate) enum PromotionPurchaseError {
    InvalidInput,
    Eligibility(PromotionRefusal),
    ConflictingOccurrence,
    Current(CharacterProgressionError),
    Fee(FeeBurnError),
    #[allow(
        dead_code,
        reason = "the durability cause is kept for Debug diagnostics; callers only classify it"
    )]
    Unavailable(DurabilityError),
}

impl SequencedOutcome for PromotionPurchaseOutcome {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        Some(self.receipt().committed_revision)
    }
}

impl SequencedError for PromotionPurchaseError {
    fn is_mismatch(&self) -> bool {
        matches!(
            self,
            Self::Current(CharacterProgressionError::CharacterRevisionMismatch)
        )
    }
    fn is_unknown(&self) -> bool {
        matches!(
            self,
            Self::Unavailable(_)
                | Self::Current(CharacterProgressionError::Unavailable(_))
                | Self::Fee(FeeBurnError::Unavailable(_))
        )
    }
    fn rejected() -> Self {
        Self::Current(CharacterProgressionError::AuthorityRejected)
    }
    fn unavailable(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl From<sqlx::Error> for PromotionPurchaseError {
    fn from(error: sqlx::Error) -> Self {
        Self::Unavailable(error.into())
    }
}
impl From<DurabilityError> for PromotionPurchaseError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}
impl From<CharacterProgressionError> for PromotionPurchaseError {
    fn from(error: CharacterProgressionError) -> Self {
        Self::Current(error)
    }
}
impl From<FeeBurnError> for PromotionPurchaseError {
    fn from(error: FeeBurnError) -> Self {
        Self::Fee(error)
    }
}

fn promotion_binding(
    fence: &CurrentCharacterGameplayFence,
    request: &PromotionPurchaseRequest,
) -> Result<Vec<u8>, PromotionPurchaseError> {
    check_uuid_v7(&request.occurrence).map_err(|_| PromotionPurchaseError::InvalidInput)?;
    check_uuid_v7(&request.account_id).map_err(|_| PromotionPurchaseError::InvalidInput)?;
    check_technical_text(&request.source.content_revision)
        .map_err(|_| PromotionPurchaseError::InvalidInput)?;
    for (reference, family) in [
        (&request.source.npc, ProjectV2Family::Npc),
        (&request.source.service, ProjectV2Family::Service),
    ] {
        if reference.family != family {
            return Err(PromotionPurchaseError::InvalidInput);
        }
        check_technical_text(&reference.key).map_err(|_| PromotionPurchaseError::InvalidInput)?;
        check_technical_text(&reference.revision)
            .map_err(|_| PromotionPurchaseError::InvalidInput)?;
    }
    if request.source.actor.generation == 0
        || request.fee.fee_gold_units != PROMOTION_PRICE
        || request.fee.cause
            != (FeeBurnCause::PromotionPurchase {
                occurrence: request.occurrence,
            })
    {
        return Err(PromotionPurchaseError::InvalidInput);
    }
    let mut hash = Sha256::new()
        .chain_update(b"oteryn:promotion-purchase:v1")
        .chain_update(fence.character_id.as_bytes())
        .chain_update(fence.runtime_scope.world_id().as_bytes())
        .chain_update(request.occurrence)
        .chain_update(request.account_id)
        .chain_update(request.source.actor.identity)
        .chain_update(request.source.actor.generation.to_be_bytes())
        .chain_update(request.fee.transaction_id)
        .chain_update(request.fee.event_id);
    // Native definition references use their canonical serde struct encoding;
    // length-prefix each field so no pair of distinct provenance fields aliases.
    for field in [
        serde_json::to_vec(&request.source.npc),
        serde_json::to_vec(&request.source.service),
        serde_json::to_vec(&request.source.content_revision),
    ] {
        let field = field.map_err(|_| PromotionPurchaseError::InvalidInput)?;
        if field.len() > 2_048 {
            return Err(PromotionPurchaseError::InvalidInput);
        }
        hash.update(
            u64::try_from(field.len())
                .map_err(|_| PromotionPurchaseError::InvalidInput)?
                .to_be_bytes(),
        );
        hash.update(field);
    }
    Ok(hash.finalize().to_vec())
}

/// The actual SQL source, joined to either the native pre-delivery Type2 gate
/// transaction or the native delivered purchase transaction. Its caller holds
/// the Character revision slot across this operation AND the final commit.
/// Both native callers roll the whole Type2 transaction back on a typed error.
/// Writes remain in the top-level transaction so the native fee/root xmin proof holds.
pub(crate) async fn prepare_promotion_purchase(
    context: &NpcPurchaseWriteContext,
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    request: &PromotionPurchaseRequest,
    premium: &PremiumStatus<'_>,
) -> Result<PendingPromotionPurchase, PromotionPurchaseError> {
    let binding = promotion_binding(&fence, request)?;
    apply_promotion_purchase(context, tx, fence, request, premium, &binding).await
}

async fn apply_promotion_purchase(
    context: &NpcPurchaseWriteContext,
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    request: &PromotionPurchaseRequest,
    premium: &PremiumStatus<'_>,
    binding: &[u8],
) -> Result<PendingPromotionPurchase, PromotionPurchaseError> {
    // Historical exact replay resolves recovery, but never reacquires gameplay
    // authority. A new purchase then resolves every live fence independently.
    context.check_recovery(tx).await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:promotion-purchase:' || encode($1,'hex'),0))")
        .bind(request.occurrence.as_slice()).execute(&mut ***tx).await?;
    if let Some(row) = load_promotion_receipt(tx, &request.occurrence).await? {
        if row.try_get::<Vec<u8>, _>("command_binding")? != binding {
            return Err(PromotionPurchaseError::ConflictingOccurrence);
        }
        return Ok(PendingPromotionPurchase(
            PromotionPurchaseOutcome::AlreadyPurchased(decode_promotion_receipt(&row)?),
        ));
    }
    context
        .admit(
            tx,
            fence,
            request.account_id,
            &request.source.content_revision,
        )
        .await?;
    let level: i64 = sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
    let promoted: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_promotions WHERE character_id=encode($1,'hex')::uuid AND promoted)")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
    let quoted = promotion_coin_fee_quote(
        u32::try_from(level).map_err(|_| DurabilityError::InvalidStoredState)?,
        promoted,
        premium,
    )
    .map_err(PromotionPurchaseError::Eligibility)?;
    // The client never quotes a fee: the request carries the native quote it was built from.
    if quoted != request.fee.fee_gold_units {
        return Err(PromotionPurchaseError::InvalidInput);
    }

    let paid = burn_fee_in_transaction(tx, &fence, &request.fee).await?;
    let FeeBurnOutcome::Burned(paid) = paid else {
        // A fee cannot predate its source receipt: never turn an orphan payment
        // into a second grant or silently advance another revision.
        return Err(DurabilityError::InvalidStoredState.into());
    };
    let committed = fence
        .expected_character_revision
        .get()
        .checked_add(1)
        .and_then(|revision| CharacterRevision::new(revision).ok())
        .ok_or(PromotionPurchaseError::InvalidInput)?;
    if paid.character_id != fence.character_id
        || paid.committed_character_revision != committed
        || paid.fee_gold_units != PROMOTION_PRICE
        || paid.bank_debit.is_some()
    {
        return Err(DurabilityError::InvalidStoredState.into());
    }

    let npc = serde_json::to_vec(&request.source.npc)
        .map_err(|_| PromotionPurchaseError::InvalidInput)?;
    let service = serde_json::to_vec(&request.source.service)
        .map_err(|_| PromotionPurchaseError::InvalidInput)?;
    let inserted = sqlx::query(
        "INSERT INTO game_character_promotion_receipts(purchase_occurrence_id,command_binding,character_id,original_character_revision,committed_character_revision,fee_transaction_id,fee_event_id,npc_definition,service_definition,npc_actor_id,npc_actor_generation,world_id,account_id,premium_admission,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at) \
         SELECT encode($1,'hex')::uuid,$2,character_id,character_revision,$3::text::numeric(20,0),encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6,$7,encode($8,'hex')::uuid,$9::text::numeric(20,0),encode($12,'hex')::uuid,encode($13,'hex')::uuid,$14,level,level,total_experience,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,floor(extract(epoch FROM statement_timestamp())*1000)::bigint FROM game_character_progression_state WHERE character_id=encode($10,'hex')::uuid AND character_revision=$11::text::numeric(20,0)")
        .bind(request.occurrence.as_slice()).bind(binding).bind(committed.get().to_string())
        .bind(paid.transaction_id.as_slice()).bind(paid.event_id.as_slice()).bind(npc).bind(service)
        .bind(request.source.actor.identity.as_slice()).bind(request.source.actor.generation.to_string())
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.expected_character_revision.get().to_string())
        .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(request.account_id.as_slice())
        .bind(match premium { PremiumStatus::Current => 1_i16, PremiumStatus::NotActivated(_) => 2_i16, PremiumStatus::NotCurrent => return Err(PromotionPurchaseError::Eligibility(PromotionRefusal::Premium(PremiumRefusal::NotCurrent))) })
        .execute(&mut ***tx).await?;
    if inserted.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    sqlx::query("INSERT INTO game_character_promotions(character_id,promoted,purchase_occurrence_id) VALUES(encode($1,'hex')::uuid,true,encode($2,'hex')::uuid)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(request.occurrence.as_slice()).execute(&mut ***tx).await?;
    let updated = sqlx::query(
        "UPDATE game_character_roots SET character_revision=$2::text::numeric(20,0) WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric(20,0)",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(committed.get().to_string())
    .bind(fence.expected_character_revision.get().to_string())
    .execute(&mut ***tx)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    let updated = sqlx::query(
        "UPDATE game_character_progression_state SET character_revision=$2::text::numeric(20,0) WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric(20,0)",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(committed.get().to_string())
    .bind(fence.expected_character_revision.get().to_string())
    .execute(&mut ***tx)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    Ok(PendingPromotionPurchase(
        PromotionPurchaseOutcome::Purchased(CommittedPromotionPurchase {
            occurrence: request.occurrence,
            character_id: fence.character_id,
            original_revision: fence.expected_character_revision,
            committed_revision: committed,
            fee_transaction: paid.transaction_id,
            fee_event: paid.event_id,
        }),
    ))
}

async fn load_promotion_receipt(
    tx: &mut Type2Transaction<'_>,
    occurrence: &[u8; 16],
) -> Result<Option<sqlx::postgres::PgRow>, sqlx::Error> {
    sqlx::query("SELECT command_binding,character_id::text,original_character_revision::text,committed_character_revision::text,fee_transaction_id::text,fee_event_id::text,purchase_occurrence_id::text FROM game_character_promotion_receipts WHERE purchase_occurrence_id=encode($1,'hex')::uuid")
        .bind(occurrence.as_slice()).fetch_optional(&mut ***tx).await
}

/// Exact historical answer of one occurrence, read before any live NPC/catalogue lookup: the NPC
/// may have gone away since the first COMMIT. The owner checks recovery on this transaction; the
/// answer never creates current actor authority.
pub(crate) struct PromotionReplayRead {
    pub(crate) character: CharacterId,
    pub(crate) occurrence: [u8; 16],
    pub(crate) npc: EntityRef,
}

impl crate::durability::npc_purchase::NpcPurchaseOperation for PromotionReplayRead {
    type Output = Option<PromotionPurchaseOutcome>;
    type Error = PromotionPurchaseError;

    fn apply<'a, 't>(
        self,
        _context: &'a NpcPurchaseWriteContext,
        tx: &'a mut Type2Transaction<'t>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Output, Self::Error>> + Send + 'a>,
    >
    where
        Self: 'a,
    {
        Box::pin(async move {
            check_uuid_v7(&self.occurrence).map_err(|_| PromotionPurchaseError::InvalidInput)?;
            let row = sqlx::query("SELECT command_binding,character_id::text,original_character_revision::text,committed_character_revision::text,fee_transaction_id::text,fee_event_id::text,purchase_occurrence_id::text, npc_actor_id=encode($2,'hex')::uuid AND npc_actor_generation=$3::text::numeric(20,0) AS target_matches FROM game_character_promotion_receipts WHERE purchase_occurrence_id=encode($1,'hex')::uuid")
                .bind(self.occurrence.as_slice()).bind(self.npc.identity.as_slice()).bind(self.npc.generation.to_string())
                .fetch_optional(&mut ***tx).await?;
            let Some(row) = row else {
                return Ok(None);
            };
            let receipt = decode_promotion_receipt(&row)?;
            if receipt.character_id != self.character
                || !row.try_get::<bool, _>("target_matches")?
            {
                return Err(PromotionPurchaseError::ConflictingOccurrence);
            }
            Ok(Some(PromotionPurchaseOutcome::AlreadyPurchased(receipt)))
        })
    }
}

fn decode_promotion_receipt(
    row: &sqlx::postgres::PgRow,
) -> Result<CommittedPromotionPurchase, PromotionPurchaseError> {
    let uuid = |column| -> Result<[u8; 16], PromotionPurchaseError> {
        let value: String = row.try_get(column)?;
        let value: String = value.chars().filter(|ch| *ch != '-').collect();
        if value.len() != 32 || !value.is_ascii() {
            return Err(DurabilityError::InvalidStoredState.into());
        }
        let mut bytes = [0; 16];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
        }
        check_uuid_v7(&bytes).map_err(|_| DurabilityError::InvalidStoredState)?;
        Ok(bytes)
    };
    let revision = |column| -> Result<CharacterRevision, PromotionPurchaseError> {
        let value: String = row.try_get(column)?;
        CharacterRevision::new(
            value
                .parse()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        )
        .map_err(|_| DurabilityError::InvalidStoredState.into())
    };
    let original = revision("original_character_revision")?;
    let committed = revision("committed_character_revision")?;
    if original.get().checked_add(1) != Some(committed.get()) {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    Ok(CommittedPromotionPurchase {
        occurrence: uuid("purchase_occurrence_id")?,
        character_id: CharacterId::from_bytes(uuid("character_id")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        original_revision: original,
        committed_revision: committed,
        fee_transaction: uuid("fee_transaction_id")?,
        fee_event: uuid("fee_event_id")?,
    })
}

#[cfg(test)]
mod policy_tests {
    use super::{PromotionRefusal, check_promotion_eligibility, effective_promotion_at_login};
    use crate::domain;
    use crate::domain::premium::{
        PremiumGate, PremiumRefusal, PremiumStatus, promotion_benefit_current,
    };
    use crate::gameplay_transport::npc_promotion::{
        PremiumBlessingService, check_premium_blessing_purchase, promotion_coin_fee_quote,
    };

    fn current() -> PremiumStatus<'static> {
        PremiumStatus::Current
    }

    #[test]
    fn promotion_starts_at_level_twenty_without_a_manufactured_upper_level_limit()
    -> Result<(), PromotionRefusal> {
        for level in [0, 1, 19] {
            assert_eq!(
                check_promotion_eligibility(level, false, &current()),
                Err(PromotionRefusal::LevelTooLow)
            );
        }
        for level in [20, 21, u32::MAX] {
            check_promotion_eligibility(level, false, &current())?;
        }
        Ok(())
    }

    #[test]
    fn absent_native_gates_stay_closed_and_noncurrent_status_refuses_promotion() {
        for closed in [
            PremiumGate::CLOSED,
            PremiumGate::default(),
            PremiumStatus::NotCurrent.gate(),
        ] {
            assert_eq!(closed.require(), Err(PremiumRefusal::NotCurrent));
        }
        assert_eq!(
            check_promotion_eligibility(20, false, &PremiumStatus::NotCurrent),
            Err(PromotionRefusal::Premium(PremiumRefusal::NotCurrent))
        );
        assert!(!effective_promotion_at_login(
            true,
            &PremiumStatus::NotCurrent
        ));
    }

    #[test]
    fn a_durable_promotion_cannot_be_purchased_again_after_premium_renewal() {
        assert_eq!(
            check_promotion_eligibility(20, true, &current()),
            Err(PromotionRefusal::AlreadyPromoted)
        );
        assert!(!effective_promotion_at_login(
            true,
            &PremiumStatus::NotCurrent
        ));
        assert!(effective_promotion_at_login(true, &current()));
        assert_eq!(
            promotion_coin_fee_quote(20, true, &current()),
            Err(PromotionRefusal::AlreadyPromoted)
        );
    }

    #[test]
    fn npc_quote_is_exactly_twenty_thousand_gold_at_every_eligible_level()
    -> Result<(), PromotionRefusal> {
        for level in [20, 21, 100, u32::MAX] {
            assert_eq!(promotion_coin_fee_quote(level, false, &current())?, 20_000);
        }
        Ok(())
    }

    #[test]
    fn refused_character_or_premium_inputs_never_yield_a_fee_quote() {
        assert_eq!(
            promotion_coin_fee_quote(19, false, &current()),
            Err(PromotionRefusal::LevelTooLow)
        );
        assert_eq!(
            promotion_coin_fee_quote(20, true, &current()),
            Err(PromotionRefusal::AlreadyPromoted)
        );
        assert_eq!(
            promotion_coin_fee_quote(20, false, &PremiumStatus::NotCurrent),
            Err(PromotionRefusal::Premium(PremiumRefusal::NotCurrent))
        );
    }

    #[test]
    fn each_quote_uses_the_supplied_current_status_and_keeps_no_previous_approval()
    -> Result<(), PromotionRefusal> {
        assert_eq!(promotion_coin_fee_quote(20, false, &current())?, 20_000);
        assert_eq!(
            promotion_coin_fee_quote(20, false, &PremiumStatus::NotCurrent),
            Err(PromotionRefusal::Premium(PremiumRefusal::NotCurrent))
        );
        assert_eq!(promotion_coin_fee_quote(20, false, &current())?, 20_000);
        Ok(())
    }

    #[test]
    fn benefits_stop_on_lapse_while_the_login_projection_is_only_a_display_snapshot() {
        let displayed_at_login = effective_promotion_at_login(true, &current());
        assert!(displayed_at_login);
        let expired = PremiumStatus::NotCurrent;
        assert!(!promotion_benefit_current(
            true,
            expired.gate().premium_current()
        ));
        assert!(displayed_at_login);
        assert!(!effective_promotion_at_login(true, &expired));
        assert!(promotion_benefit_current(
            true,
            current().gate().premium_current()
        ));
        assert!(effective_promotion_at_login(true, &current()));
        assert!(!effective_promotion_at_login(false, &current()));
    }

    #[test]
    fn the_existing_native_soul_rule_is_not_replaced_by_effective_promotion() {
        assert!(!effective_promotion_at_login(false, &current()));
        assert_eq!(
            domain::premium::soul_maximum(current().gate().premium_current()),
            200
        );
        assert_eq!(domain::premium::apply_account_transition(200, false), 100);
    }

    #[test]
    fn all_three_premium_blessing_purchases_use_the_native_gate() -> Result<(), PremiumRefusal> {
        for service in [
            PremiumBlessingService::WisdomOfSolitude,
            PremiumBlessingService::BloodOfTheMountain,
            PremiumBlessingService::HeartOfTheMountain,
        ] {
            assert_eq!(
                check_premium_blessing_purchase(service, PremiumGate::CLOSED),
                Err(PremiumRefusal::NotCurrent)
            );
            check_premium_blessing_purchase(service, current().gate())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod purchase_binding_tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::durability::item_fee_burn::FeeChangeFacts;
    use crate::durability::item_mint::TypedDefinitionRef;
    use crate::durability::item_transfer::{ItemDefinitionFacts, ItemStackClass};
    use crate::foundation::{
        ChannelId, ConnectionGeneration, GameSessionId, RuntimeScopeRefV1,
        ScopeOwnershipGeneration, WorldId,
    };

    fn id(seed: u8) -> [u8; 16] {
        let mut id = [seed; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id
    }
    fn intent() -> (CurrentCharacterGameplayFence, PromotionPurchaseRequest) {
        let definition = |key: &str| TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: "fixture-r1".into(),
        };
        let fence = CurrentCharacterGameplayFence {
            character_id: CharacterId::from_bytes(id(1)).expect("fixture"),
            game_session_id: GameSessionId::decode(&id(2)).expect("fixture"),
            connection_generation: ConnectionGeneration::new(1).expect("fixture"),
            character_lease_generation: 1,
            runtime_scope: RuntimeScopeRefV1::channel(
                WorldId::decode(&id(3)).expect("fixture"),
                ChannelId::decode(&id(4)).expect("fixture"),
            ),
            scope_ownership_generation: ScopeOwnershipGeneration::new(1).expect("fixture"),
            expected_character_revision: CharacterRevision::new(1).expect("fixture"),
        };
        let request = PromotionPurchaseRequest {
            occurrence: id(5),
            account_id: id(6),
            source: PromotionNpcSource {
                npc: ProjectV2DefinitionRef {
                    family: ProjectV2Family::Npc,
                    key: "oteryn:npc.fixture".into(),
                    revision: "fixture-r1".into(),
                },
                service: ProjectV2DefinitionRef {
                    family: ProjectV2Family::Service,
                    key: "oteryn:service.fixture".into(),
                    revision: "fixture-r1".into(),
                },
                actor: EntityRef {
                    identity: id(7),
                    generation: 1,
                },
                content_revision: "fixture-content-r1".into(),
            },
            fee: FeeBurnRequest {
                cause: FeeBurnCause::PromotionPurchase { occurrence: id(5) },
                fee_gold_units: 20_000,
                transaction_id: id(8),
                event_id: id(9),
                occurred_at_unix_ms: 1,
                server_build_id: "fixture".into(),
                change: FeeChangeFacts {
                    item_instance_ids: [id(10), id(11)],
                    gold: definition("oteryn:item.gold_coin"),
                    platinum: definition("oteryn:item.platinum_coin"),
                    crystal: definition("oteryn:item.crystal_coin"),
                    backpack: ItemDefinitionFacts {
                        definition: definition("oteryn:item.fixture_bag"),
                        stack: ItemStackClass::NonStackable,
                        container_capacity: Some(20),
                        container_slot_equip_pattern: true,
                    },
                },
            },
        };
        (fence, request)
    }

    #[test]
    fn promotion_binding_keeps_intent_identity_without_reconstructing_retry_authority() {
        let (mut fence, mut request) = intent();
        let original = promotion_binding(&fence, &request).expect("bounded immutable fixture only");
        fence.character_lease_generation = 2;
        fence.connection_generation = ConnectionGeneration::new(2).expect("fixture");
        fence.expected_character_revision = CharacterRevision::new(2).expect("fixture");
        assert_eq!(
            promotion_binding(&fence, &request).expect("historical binding"),
            original
        );
        request.source.actor.generation += 1;
        assert_ne!(
            promotion_binding(&fence, &request).expect("different target"),
            original
        );
        request.source.actor.generation -= 1;
        request.fee.event_id = id(12);
        assert_ne!(
            promotion_binding(&fence, &request).expect("different audit identity"),
            original
        );
    }

    #[test]
    fn promotion_rejects_mismatched_fee_occurrence_price_and_service_family() {
        let (fence, request) = intent();
        let mut wrong = request.clone();
        wrong.fee.cause = FeeBurnCause::PromotionPurchase { occurrence: id(12) };
        assert!(matches!(
            promotion_binding(&fence, &wrong),
            Err(PromotionPurchaseError::InvalidInput)
        ));
        let mut wrong = request.clone();
        wrong.fee.fee_gold_units = 19_999;
        assert!(matches!(
            promotion_binding(&fence, &wrong),
            Err(PromotionPurchaseError::InvalidInput)
        ));
        let mut wrong = request;
        wrong.source.service.family = ProjectV2Family::Npc;
        assert!(matches!(
            promotion_binding(&fence, &wrong),
            Err(PromotionPurchaseError::InvalidInput)
        ));
    }
}
