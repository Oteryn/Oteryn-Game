//! D1106 / DEATH_COMPLETE0 §4.7: one fenced Premium blessing grant and native fee.
//! Catalogue, NPC conversation, Premium gate and RevisionSlot belong to the caller
//! (`npc_promotion`); migration 0137 installs the receipt table and its chain arms.
use crate::content::ProjectV2Family;
use crate::domain::premium::{PremiumRefusal, PremiumStatus};
use crate::domain::{CharacterId, CharacterRevision};
use crate::durability::DurabilityError;
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use crate::durability::character_revision_sequencer::{SequencedError, SequencedOutcome};
use crate::durability::item_fee_burn::{
    FeeBankDebit, FeeBurnCause, FeeBurnError, FeeBurnOutcome, FeeBurnRequest,
    burn_fee_in_transaction,
};
use crate::durability::item_mint_audit::{Type2Transaction, check_technical_text, check_uuid_v7};
use crate::durability::npc_purchase::NpcPurchaseWriteContext;
use crate::gameplay_transport::npc_promotion::{
    BlessingNpcOffer, BlessingPriceBand, PremiumBlessingService, check_premium_blessing_purchase,
};
use crate::gameplay_transport::premium_promotion::NpcPurchaseSource;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use sha2::{Digest, Sha256};
use sqlx::Row;

/// Native-gate borrowed operation; the lower port owns physical commit/rollback.
pub(crate) struct BlessingPurchaseOperation<'r, 'p> {
    pub(crate) fence: CurrentCharacterGameplayFence,
    pub(crate) request: &'r BlessingPurchaseRequest,
    pub(crate) premium: &'r PremiumStatus<'p>,
}

impl crate::durability::npc_purchase::NpcPurchaseOperation for BlessingPurchaseOperation<'_, '_> {
    type Output = PendingBlessingPurchase;
    type Error = BlessingPurchaseError;

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
            prepare_blessing_purchase(context, tx, self.fence, self.request, self.premium).await
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct BlessingPurchaseRequest {
    pub(crate) occurrence: [u8; 16],
    pub(crate) account_id: [u8; 16],
    /// Qualified by the native blessing conversation/catalogue owner, never a promotion permit.
    pub(crate) source: NpcPurchaseSource,
    pub(crate) offer: BlessingNpcOffer,
    /// Stable native fee identities/change facts; its quoted amount is NOT authoritative.
    pub(crate) fee: FeeBurnRequest,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommittedBlessingPurchase {
    pub(crate) occurrence: [u8; 16],
    pub(crate) character_id: CharacterId,
    pub(crate) original_revision: CharacterRevision,
    pub(crate) committed_revision: CharacterRevision,
    pub(crate) blessing: String,
    pub(crate) service: PremiumBlessingService,
    pub(crate) fee_transaction: [u8; 16],
    pub(crate) fee_event: [u8; 16],
    pub(crate) price_gold_units: u64,
    pub(crate) bank_debit: Option<FeeBankDebit>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BlessingPurchaseOutcome {
    Purchased(CommittedBlessingPurchase),
    AlreadyPurchased(CommittedBlessingPurchase),
}
impl BlessingPurchaseOutcome {
    pub(crate) fn receipt(&self) -> &CommittedBlessingPurchase {
        let (Self::Purchased(receipt) | Self::AlreadyPurchased(receipt)) = self;
        receipt
    }
}
/// May leave the caller's gate only after its ONE outer physical COMMIT succeeds.
pub(crate) struct PendingBlessingPurchase(BlessingPurchaseOutcome);
impl PendingBlessingPurchase {
    pub(crate) fn after_native_commit(self) -> BlessingPurchaseOutcome {
        self.0
    }
}
#[derive(Debug)]
pub(crate) enum BlessingPurchaseError {
    InvalidInput,
    Premium(PremiumRefusal),
    AlreadyHeld,
    PriceUnavailable,
    ConflictingOccurrence,
    Current(CharacterProgressionError),
    Fee(FeeBurnError),
    #[allow(
        dead_code,
        reason = "the durability cause is kept for Debug diagnostics; callers only classify it"
    )]
    Unavailable(DurabilityError),
}
impl SequencedOutcome for BlessingPurchaseOutcome {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        Some(self.receipt().committed_revision)
    }
}
impl SequencedError for BlessingPurchaseError {
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
impl From<sqlx::Error> for BlessingPurchaseError {
    fn from(error: sqlx::Error) -> Self {
        Self::Unavailable(error.into())
    }
}
impl From<DurabilityError> for BlessingPurchaseError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}
impl From<CharacterProgressionError> for BlessingPurchaseError {
    fn from(error: CharacterProgressionError) -> Self {
        Self::Current(error)
    }
}
impl From<FeeBurnError> for BlessingPurchaseError {
    fn from(error: FeeBurnError) -> Self {
        Self::Fee(error)
    }
}
fn service_code(service: PremiumBlessingService) -> i16 {
    match service {
        PremiumBlessingService::WisdomOfSolitude => 1,
        PremiumBlessingService::BloodOfTheMountain => 2,
        PremiumBlessingService::HeartOfTheMountain => 3,
    }
}
fn decode_service(value: i16) -> Result<PremiumBlessingService, BlessingPurchaseError> {
    match value {
        1 => Ok(PremiumBlessingService::WisdomOfSolitude),
        2 => Ok(PremiumBlessingService::BloodOfTheMountain),
        3 => Ok(PremiumBlessingService::HeartOfTheMountain),
        _ => Err(DurabilityError::InvalidStoredState.into()),
    }
}
fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 128
        && key.as_bytes()[0].is_ascii_alphanumeric()
        && key
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._:-".contains(&c))
}
/// Prices are admitted source data, not a newly invented game-price formula. Returns the band
/// of `level` and its price there.
fn price_at(
    offer: &BlessingNpcOffer,
    level: u32,
) -> Result<(&BlessingPriceBand, u64), BlessingPurchaseError> {
    let mut found = None;
    let mut previous = None;
    for band in &offer.prices {
        if band.minimum_level == 0
            || band.maximum_level < band.minimum_level
            || (band.gold_per_level != 0 && band.base_level > band.minimum_level)
            || previous.is_some_and(|maximum| band.minimum_level <= maximum)
        {
            return Err(BlessingPurchaseError::InvalidInput);
        }
        previous = Some(band.maximum_level);
        if (band.minimum_level..=band.maximum_level).contains(&level) {
            found = Some(band);
        }
    }
    let band = found.ok_or(BlessingPurchaseError::PriceUnavailable)?;
    let slope = if band.gold_per_level == 0 {
        Some(0)
    } else {
        level
            .checked_sub(band.base_level)
            .and_then(|steps| band.gold_per_level.checked_mul(u64::from(steps)))
    };
    let price = slope
        .and_then(|slope| band.gold_units.checked_add(slope))
        .filter(|price| (1..=i64::MAX as u64).contains(price))
        .ok_or(BlessingPurchaseError::InvalidInput)?;
    Ok((band, price))
}
fn binding(
    fence: &CurrentCharacterGameplayFence,
    r: &BlessingPurchaseRequest,
) -> Result<Vec<u8>, BlessingPurchaseError> {
    for id in [
        &r.occurrence,
        &r.account_id,
        &r.fee.transaction_id,
        &r.fee.event_id,
    ] {
        check_uuid_v7(id).map_err(|_| BlessingPurchaseError::InvalidInput)?;
    }
    if !valid_key(&r.offer.storage_key)
        || r.source.actor.generation == 0
        || r.source.npc != r.offer.source.npc
        || r.source.service != r.offer.source.service
        || r.fee.cause
            != (FeeBurnCause::BlessingPurchase {
                occurrence: r.occurrence,
                blessing: r.offer.storage_key.clone(),
            })
    {
        return Err(BlessingPurchaseError::InvalidInput);
    }
    check_technical_text(&r.source.content_revision)
        .map_err(|_| BlessingPurchaseError::InvalidInput)?;
    let mut hash = Sha256::new()
        .chain_update(b"oteryn:blessing-purchase:v1")
        .chain_update(fence.character_id.as_bytes())
        .chain_update(fence.runtime_scope.world_id().as_bytes())
        .chain_update(r.occurrence)
        .chain_update(r.account_id)
        .chain_update(service_code(r.offer.blessing).to_be_bytes())
        .chain_update(r.source.actor.identity)
        .chain_update(r.source.actor.generation.to_be_bytes())
        .chain_update(r.fee.transaction_id)
        .chain_update(r.fee.event_id);
    for (reference, family) in [
        (&r.source.npc, ProjectV2Family::Npc),
        (&r.source.service, ProjectV2Family::Service),
    ] {
        if reference.family != family {
            return Err(BlessingPurchaseError::InvalidInput);
        }
        check_technical_text(&reference.key).map_err(|_| BlessingPurchaseError::InvalidInput)?;
        check_technical_text(&reference.revision)
            .map_err(|_| BlessingPurchaseError::InvalidInput)?;
        let value =
            serde_json::to_vec(reference).map_err(|_| BlessingPurchaseError::InvalidInput)?;
        hash.update((value.len() as u64).to_be_bytes());
        hash.update(value);
    }
    for text in [&r.source.content_revision, &r.offer.storage_key] {
        hash.update((text.len() as u64).to_be_bytes());
        hash.update(text.as_bytes());
    }
    // Retain the exact server offer, while the authoritative band is selected only AFTER locking.
    hash.update((r.offer.prices.len() as u64).to_be_bytes());
    for band in &r.offer.prices {
        hash.update(band.minimum_level.to_be_bytes());
        hash.update(band.maximum_level.to_be_bytes());
        hash.update(band.gold_units.to_be_bytes());
        hash.update(band.gold_per_level.to_be_bytes());
        hash.update(band.base_level.to_be_bytes());
    }
    hash.update([u8::from(r.offer.promoted_price_admitted)]);
    let mut value = vec![1];
    value.extend_from_slice(&hash.finalize());
    Ok(value)
}

/// The caller holds its RevisionSlot and one current Premium gate across this participant
/// and COMMIT. Errors roll back the entire Type2 transaction. No savepoint or nested commit.
pub(crate) async fn prepare_blessing_purchase(
    context: &NpcPurchaseWriteContext,
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    request: &BlessingPurchaseRequest,
    premium: &PremiumStatus<'_>,
) -> Result<PendingBlessingPurchase, BlessingPurchaseError> {
    let intent = binding(&fence, request)?;
    context.check_recovery(tx).await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:blessing-purchase:' || encode($1,'hex'),0))")
        .bind(request.occurrence.as_slice()).execute(&mut ***tx).await?;
    if let Some(row) = load_receipt(tx, &request.occurrence).await? {
        if row.try_get::<Vec<u8>, _>("command_binding")? != intent {
            return Err(BlessingPurchaseError::ConflictingOccurrence);
        }
        return Ok(PendingBlessingPurchase(
            BlessingPurchaseOutcome::AlreadyPurchased(decode_receipt(&row)?),
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
    // Unlike promotion, neither NotActivated nor NotCurrent has an exception here.
    check_premium_blessing_purchase(request.offer.blessing, premium.gate())
        .map_err(BlessingPurchaseError::Premium)?;
    let level:i64=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
    let level = u32::try_from(level).map_err(|_| DurabilityError::InvalidStoredState)?;
    let (band, price) = price_at(&request.offer, level)?;
    // A promoted buyer's price is not admitted while the source leaves its effect unknown.
    if !request.offer.promoted_price_admitted {
        let promoted: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_promotions WHERE character_id=encode($1,'hex')::uuid AND promoted)")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
        if promoted {
            return Err(BlessingPurchaseError::PriceUnavailable);
        }
    }
    // Admission already holds the character root and progression row locks through COMMIT.
    // Blessing purchases and deaths serialize there; held rows need no additional lock.
    // FOR UPDATE would require UPDATE authority on the append/consume-only blessing table.
    let before:Vec<String>=sqlx::query_scalar("SELECT blessing_key COLLATE \"C\" FROM game_character_blessings WHERE character_id=encode($1,'hex')::uuid ORDER BY 1")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_all(&mut ***tx).await?;
    if before.iter().any(|key| key == &request.offer.storage_key) {
        return Err(BlessingPurchaseError::AlreadyHeld);
    }
    // Existing blessing-set storage bound, not a content or gameplay cardinality.
    if before.len() >= 32 {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    let mut after = before.clone();
    after.push(request.offer.storage_key.clone());
    after.sort();
    let mut fee = request.fee.clone();
    fee.fee_gold_units = price;
    let FeeBurnOutcome::Burned(paid) = burn_fee_in_transaction(tx, &fence, &fee).await? else {
        return Err(DurabilityError::InvalidStoredState.into());
    };
    let committed = fence
        .expected_character_revision
        .get()
        .checked_add(1)
        .and_then(|v| CharacterRevision::new(v).ok())
        .ok_or(BlessingPurchaseError::InvalidInput)?;
    if paid.character_id != fence.character_id
        || paid.committed_character_revision != committed
        || paid.fee_gold_units != price
        || paid.bank_debit.is_some_and(|bank| {
            bank.account_id != request.account_id
                || bank.debit_gold_units == 0
                || bank.debit_gold_units > price
                || bank
                    .balance_before_gold_units
                    .checked_sub(bank.debit_gold_units)
                    != Some(bank.balance_after_gold_units)
                || bank.balance_before_gold_units > i64::MAX as u64
                || bank.balance_after_gold_units > i64::MAX as u64
        })
    {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    let bank = paid.bank_debit;
    let inserted=sqlx::query("INSERT INTO game_character_blessing_purchase_receipts(purchase_occurrence_id,command_binding,character_id,original_character_revision,committed_character_revision,fee_transaction_id,fee_event_id,npc_definition,service_definition,npc_actor_id,npc_actor_generation,world_id,account_id,blessing_key,blessing_service,price_minimum_level,price_maximum_level,price_gold_units,bank_entry_id,bank_debit_gold_units,bank_balance_before,bank_balance_after,blessings_before,blessings_after,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at) \
        SELECT encode($1,'hex')::uuid,$2,character_id,character_revision,$3::text::numeric(20,0),encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6,$7,encode($8,'hex')::uuid,$9::text::numeric(20,0),encode($10,'hex')::uuid,encode($11,'hex')::uuid,$12,$13,$14,$15,$16,encode($17,'hex')::uuid,$18,$19,$20,$21,$22,level,level,total_experience,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,floor(extract(epoch FROM statement_timestamp())*1000)::bigint FROM game_character_progression_state WHERE character_id=encode($23,'hex')::uuid AND character_revision=$24::text::numeric(20,0)")
        .bind(request.occurrence.as_slice()).bind(&intent).bind(committed.get().to_string())
        .bind(paid.transaction_id.as_slice()).bind(paid.event_id.as_slice())
        .bind(serde_json::to_vec(&request.source.npc).map_err(|_|BlessingPurchaseError::InvalidInput)?)
        .bind(serde_json::to_vec(&request.source.service).map_err(|_|BlessingPurchaseError::InvalidInput)?)
        .bind(request.source.actor.identity.as_slice()).bind(request.source.actor.generation.to_string())
        .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(request.account_id.as_slice())
        .bind(&request.offer.storage_key).bind(service_code(request.offer.blessing))
        .bind(i64::from(band.minimum_level)).bind(i64::from(band.maximum_level)).bind(i64::try_from(price).map_err(|_|BlessingPurchaseError::InvalidInput)?)
        .bind(bank.as_ref().map(|b|b.entry_id.as_slice())).bind(bank.map_or(0,|b|b.debit_gold_units as i64))
        .bind(bank.map(|b|b.balance_before_gold_units as i64)).bind(bank.map(|b|b.balance_after_gold_units as i64))
        .bind(&before).bind(&after).bind(fence.character_id.as_bytes().as_slice()).bind(fence.expected_character_revision.get().to_string())
        .execute(&mut ***tx).await?;
    if inserted.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    sqlx::query("INSERT INTO game_character_blessings(character_id,blessing_key,provenance) VALUES(encode($1,'hex')::uuid,$2,'purchase:' || encode($3,'hex'))")
        .bind(fence.character_id.as_bytes().as_slice()).bind(&request.offer.storage_key).bind(request.occurrence.as_slice()).execute(&mut ***tx).await?;
    let updated = sqlx::query("UPDATE game_character_roots SET character_revision=$2::text::numeric(20,0) WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric(20,0)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(committed.get().to_string())
        .bind(fence.expected_character_revision.get().to_string()).execute(&mut ***tx).await?;
    if updated.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    let updated = sqlx::query("UPDATE game_character_progression_state SET character_revision=$2::text::numeric(20,0) WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric(20,0)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(committed.get().to_string())
        .bind(fence.expected_character_revision.get().to_string()).execute(&mut ***tx).await?;
    if updated.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    Ok(PendingBlessingPurchase(BlessingPurchaseOutcome::Purchased(
        CommittedBlessingPurchase {
            occurrence: request.occurrence,
            character_id: fence.character_id,
            original_revision: fence.expected_character_revision,
            committed_revision: committed,
            blessing: request.offer.storage_key.clone(),
            service: request.offer.blessing,
            fee_transaction: paid.transaction_id,
            fee_event: paid.event_id,
            price_gold_units: price,
            bank_debit: bank,
        },
    )))
}
async fn load_receipt(
    tx: &mut Type2Transaction<'_>,
    occurrence: &[u8; 16],
) -> Result<Option<sqlx::postgres::PgRow>, sqlx::Error> {
    sqlx::query("SELECT command_binding,purchase_occurrence_id::text,character_id::text,original_character_revision::text,committed_character_revision::text,fee_transaction_id::text,fee_event_id::text,blessing_key,blessing_service,price_gold_units,account_id::text,bank_entry_id::text,bank_debit_gold_units,bank_balance_before,bank_balance_after FROM game_character_blessing_purchase_receipts WHERE purchase_occurrence_id=encode($1,'hex')::uuid")
        .bind(occurrence.as_slice())
        .fetch_optional(&mut ***tx)
        .await
}
/// Exact historical answer of one occurrence, read BEFORE live NPC/offer/Premium lookup. The
/// owner checks recovery on this transaction; the answer never creates current actor authority.
pub(crate) struct BlessingReplayRead {
    pub(crate) character: CharacterId,
    pub(crate) occurrence: [u8; 16],
    pub(crate) npc: EntityRef,
    pub(crate) service: PremiumBlessingService,
}
impl crate::durability::npc_purchase::NpcPurchaseOperation for BlessingReplayRead {
    type Output = Option<BlessingPurchaseOutcome>;
    type Error = BlessingPurchaseError;

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
            check_uuid_v7(&self.occurrence).map_err(|_| BlessingPurchaseError::InvalidInput)?;
            let row = sqlx::query("SELECT command_binding,purchase_occurrence_id::text,character_id::text,original_character_revision::text,committed_character_revision::text,fee_transaction_id::text,fee_event_id::text,blessing_key,blessing_service,price_gold_units,account_id::text,bank_entry_id::text,bank_debit_gold_units,bank_balance_before,bank_balance_after,npc_actor_id=encode($2,'hex')::uuid AND npc_actor_generation=$3::text::numeric(20,0) AS target_matches FROM game_character_blessing_purchase_receipts WHERE purchase_occurrence_id=encode($1,'hex')::uuid")
                .bind(self.occurrence.as_slice())
                .bind(self.npc.identity.as_slice())
                .bind(self.npc.generation.to_string())
                .fetch_optional(&mut ***tx)
                .await?;
            let Some(row) = row else {
                return Ok(None);
            };
            let receipt = decode_receipt(&row)?;
            if receipt.character_id != self.character
                || receipt.service != self.service
                || !row.try_get::<bool, _>("target_matches")?
            {
                return Err(BlessingPurchaseError::ConflictingOccurrence);
            }
            Ok(Some(BlessingPurchaseOutcome::AlreadyPurchased(receipt)))
        })
    }
}
fn decode_uuid(text: &str) -> Result<[u8; 16], BlessingPurchaseError> {
    let text = text.replace('-', "");
    if text.len() != 32 || !text.is_ascii() {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    let mut bytes = [0; 16];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    }
    check_uuid_v7(&bytes).map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(bytes)
}
fn decode_receipt(
    row: &sqlx::postgres::PgRow,
) -> Result<CommittedBlessingPurchase, BlessingPurchaseError> {
    let version: Vec<u8> = row.try_get("command_binding")?;
    if version.len() != 33 || version[0] != 1 {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    let uuid = |column| -> Result<[u8; 16], BlessingPurchaseError> {
        decode_uuid(&row.try_get::<String, _>(column)?)
    };
    let rev = |column| -> Result<CharacterRevision, BlessingPurchaseError> {
        let value: String = row.try_get(column)?;
        CharacterRevision::new(
            value
                .parse()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        )
        .map_err(|_| DurabilityError::InvalidStoredState.into())
    };
    let original = rev("original_character_revision")?;
    let committed = rev("committed_character_revision")?;
    let key: String = row.try_get("blessing_key")?;
    let price: i64 = row.try_get("price_gold_units")?;
    if original.get().checked_add(1) != Some(committed.get()) || !valid_key(&key) || price <= 0 {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    let entry: Option<String> = row.try_get("bank_entry_id")?;
    let amount: i64 = row.try_get("bank_debit_gold_units")?;
    let before: Option<i64> = row.try_get("bank_balance_before")?;
    let after: Option<i64> = row.try_get("bank_balance_after")?;
    let bank = match (entry, amount, before, after) {
        (None, 0, None, None) => None,
        (Some(entry), amount, Some(before), Some(after))
            if amount > 0
                && amount <= price
                && before >= amount
                && before.checked_sub(amount) == Some(after) =>
        {
            Some(FeeBankDebit {
                entry_id: decode_uuid(&entry)?,
                account_id: uuid("account_id")?,
                debit_gold_units: amount as u64,
                balance_before_gold_units: before as u64,
                balance_after_gold_units: after as u64,
            })
        }
        _ => return Err(DurabilityError::InvalidStoredState.into()),
    };
    Ok(CommittedBlessingPurchase {
        occurrence: uuid("purchase_occurrence_id")?,
        character_id: CharacterId::from_bytes(uuid("character_id")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        original_revision: original,
        committed_revision: committed,
        blessing: key,
        service: decode_service(row.try_get("blessing_service")?)?,
        fee_transaction: uuid("fee_transaction_id")?,
        fee_event: uuid("fee_event_id")?,
        price_gold_units: price as u64,
        bank_debit: bank,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use super::*;
    use crate::content::ProjectV2DefinitionRef;
    use crate::gameplay_transport::npc_promotion::PromotionNpcOffer;
    use crate::gameplay_transport::premium_blessing_catalogue::BlessingSources;

    fn offer(prices: Vec<BlessingPriceBand>) -> BlessingNpcOffer {
        let reference = |family, key: &str| ProjectV2DefinitionRef {
            family,
            key: key.into(),
            revision: "1".into(),
        };
        BlessingNpcOffer {
            source: PromotionNpcOffer {
                npc: reference(ProjectV2Family::Npc, "oteryn:npc.kais"),
                service: reference(
                    ProjectV2Family::Service,
                    "oteryn:service.blessing.blood_of_the_mountain",
                ),
            },
            blessing: PremiumBlessingService::BloodOfTheMountain,
            storage_key: "oteryn:blessing.blood-of-the-mountain".into(),
            prices,
            promoted_price_admitted: false,
        }
    }

    #[test]
    fn the_writer_prices_the_source_bands_at_the_locked_level() {
        let sources = BlessingSources::embedded().expect("source");
        let blood = offer(
            sources
                .get(PremiumBlessingService::BloodOfTheMountain)
                .expect("blood")
                .prices
                .clone(),
        );
        for (level, gold) in [
            (1, 2_600),
            (30, 2_600),
            (31, 2_860),
            (50, 7_800),
            (119, 25_740),
            (120, 26_000),
            (121, 26_100),
        ] {
            assert_eq!(price_at(&blood, level).expect("price").1, gold, "{level}");
        }
        // No band, a flat zero price and a slope below its base fail closed.
        assert!(matches!(
            price_at(&offer(Vec::new()), 50),
            Err(BlessingPurchaseError::PriceUnavailable)
        ));
        let band = |minimum_level, gold_units, gold_per_level, base_level| BlessingPriceBand {
            minimum_level,
            maximum_level: u32::MAX,
            gold_units,
            gold_per_level,
            base_level,
        };
        for prices in [vec![band(1, 0, 0, 0)], vec![band(1, 0, 5, 20)]] {
            assert!(matches!(
                price_at(&offer(prices), 50),
                Err(BlessingPurchaseError::InvalidInput)
            ));
        }
        assert!(matches!(
            price_at(&offer(vec![band(1, 1, u64::MAX, 1)]), 3),
            Err(BlessingPurchaseError::InvalidInput)
        ));
    }
}
