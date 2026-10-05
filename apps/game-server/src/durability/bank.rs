//! The Account bank balance writer (BANK-1, migration 0071; decision
//! `BANK0-ACCOUNT-WORLD-BANK-BALANCE-V1` §3-§5, owner answers 1b and Q1 b).
//!
//! One gold balance per (Account, World), shared by the Account's characters of that World. The
//! entry points [`DurabilityRoot::bank_deposit`], [`DurabilityRoot::bank_withdraw`] and
//! [`DurabilityRoot::bank_transfer`] each run one PostgreSQL transaction with the TransactionId,
//! EventId and planned output slots the caller fixed before the first attempt (BANK-0 §4.1). The
//! banker NPC runtime (BANK-NPC-1) calls them with the runtime occurrence its confirming `yes`
//! issued; this module has no wire and no dialogue.
//!
//! Lock order (BANK-0 §4.1, composition rule 4 extended): the recovery fence and admission
//! relations, the operation occurrence, the acting Character's session and guard checks (rule 2,
//! which read no recipient row), the root step, the main backpack and its coin entries, then the
//! balance rows by `account_id` (upsert, then FOR UPDATE). The root step of a deposit or
//! withdrawal is the acting root FOR UPDATE; a transfer locks both roots in CharacterId order,
//! the sender FOR UPDATE and the recipient FOR SHARE, and touches no backpack.
//!
//! Every check runs before the first ledger entry or balance change. A typed refusal
//! ([`BankResult`]) writes only its operation row (so a replay returns the same result) and, when
//! the balance step was reached, the value-neutral zero-balance row: no ledger entry, coin line or
//! event, and no balance change. The same occurrence and binding replay the first outcome; a
//! changed binding conflicts. No `CharacterRevision` advance.

use super::bank_audit::{
    self as audit, ASSET_GOLD, BANK_BALANCE_MAX, BANK_DEPOSIT_MAX, BANK_TRANSFER_MAX,
    BANK_WITHDRAW_MAX, BankCauseV1, BankCoinLineV1, BankConversionCauseV1, BankEventIdentity,
    BankItemDefinitionV1, BankItemStateV1, BankOperationCauseV1, BankOperationV1,
    BankTransferCauseV1, BankValueLineV1, COIN_LINE_INPUT, COIN_LINE_OUTPUT, CONVERSION_DEPOSIT,
    CONVERSION_WITHDRAW, LINE_CLASS_CONVERSION, LINE_CLASS_TRANSFER,
};
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{numeric_u64, uuid_text};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_mint::TypedDefinitionRef;
use super::item_mint_audit::{ITEM_LIFECYCLE_LIVE, check_technical_text, check_uuid_v7};
use super::item_transfer::{CurrentCharacterItemFence, ItemDefinitionFacts};
use super::item_transfer_audit::ITEM_LIFECYCLE_RETIRED;
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation, scope_key};
use super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterId;
use crate::domain::currency::{
    BACKPACK_ENTRIES_MAX, COIN_DEFINITION_FAMILY, Coin, CoinStack, FeePlanError, plan_fee_within,
};
use crate::foundation::{ChannelId, RuntimeScopeRefV1, WorldId};
use sha2::{Digest, Sha256};
use sqlx::Row;
use sqlx::postgres::PgConnection;

const REQUEST_BINDING_VERSION: u8 = 1;

/// The runtime occurrence of one confirmed bank operation (BANK-0 §6): a UUIDv7 bound 1:1 to the
/// confirming CommandRef by the NPC runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BankOperationOccurrence([u8; 16]);

impl BankOperationOccurrence {
    pub fn from_bytes(bytes: [u8; 16]) -> Result<Self> {
        check_uuid_v7(&bytes).map_err(|_| BankError::InvalidInput)?;
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

/// The identities fixed before the first attempt and reused on every retry of the occurrence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankOperationIdentity {
    pub occurrence: BankOperationOccurrence,
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub server_build_id: String,
}

/// The current compatible coin definitions and the main backpack's definition (the caller's
/// Content facts): every live coin stack a deposit reads must be at its coin's revision, outputs
/// are created at these revisions, and the backpack's declared capacity bounds them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankCoinFacts {
    pub gold: TypedDefinitionRef,
    pub platinum: TypedDefinitionRef,
    pub crystal: TypedDefinitionRef,
    pub backpack: ItemDefinitionFacts,
}

/// `deposit <amount> gold`: `deposit all` is resolved to an exact amount at the prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankDepositRequest {
    pub identity: BankOperationIdentity,
    pub amount: u64,
    pub coins: BankCoinFacts,
    /// The change slots: platinum, then gold.
    pub change_item_instance_ids: [[u8; 16]; 2],
}

/// `withdraw <amount> gold`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankWithdrawRequest {
    pub identity: BankOperationIdentity,
    pub amount: u64,
    pub coins: BankCoinFacts,
    /// The output slots: crystal, then platinum, then gold.
    pub output_item_instance_ids: [[u8; 16]; 3],
}

/// `transfer <amount> gold to <name>`: the recipient resolved at the prompt through the `0022`
/// name reservation and bound in the confirmation with its comparison key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankTransferRequest {
    pub identity: BankOperationIdentity,
    pub amount: u64,
    pub recipient_character_id: CharacterId,
    pub recipient_name_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BankOperationKind {
    Deposit,
    Withdraw,
    Transfer,
}

impl BankOperationKind {
    fn code(self) -> i16 {
        match self {
            Self::Deposit => 1,
            Self::Withdraw => 2,
            Self::Transfer => 3,
        }
    }

    fn from_code(code: i16) -> Option<Self> {
        match code {
            1 => Some(Self::Deposit),
            2 => Some(Self::Withdraw),
            3 => Some(Self::Transfer),
            _ => None,
        }
    }
}

/// The typed results of BANK-0 §4.1 and §4.3. BANK-NPC-1 maps them to the banker's replies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BankResult {
    Ok,
    /// A deposit's credit above `BANK0-RL-01`.
    BalanceLimit,
    InsufficientBalance,
    /// The main backpack's coins cannot pay a deposit.
    InsufficientCoins,
    /// A deposit's change or a withdrawal's coins do not fit the main backpack.
    NoRoom,
    JuniorAccount,
    UnknownRecipient,
    /// A junior recipient, or a credit above `BANK0-RL-01` on the recipient's balance.
    RecipientCannotReceiveTransfers,
    SameAccount,
}

impl BankResult {
    fn code(self) -> i16 {
        match self {
            Self::Ok => 0,
            Self::BalanceLimit => 1,
            Self::InsufficientBalance => 2,
            Self::InsufficientCoins => 3,
            Self::NoRoom => 4,
            Self::JuniorAccount => 5,
            Self::UnknownRecipient => 6,
            Self::RecipientCannotReceiveTransfers => 7,
            Self::SameAccount => 8,
        }
    }

    fn from_code(code: i16) -> Option<Self> {
        [
            Self::Ok,
            Self::BalanceLimit,
            Self::InsufficientBalance,
            Self::InsufficientCoins,
            Self::NoRoom,
            Self::JuniorAccount,
            Self::UnknownRecipient,
            Self::RecipientCannotReceiveTransfers,
            Self::SameAccount,
        ]
        .into_iter()
        .find(|result| result.code() == code)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BankLedgerKind {
    Deposit,
    Withdraw,
    TransferOut,
    TransferIn,
}

impl BankLedgerKind {
    fn code(self) -> u32 {
        match self {
            Self::Deposit => audit::LEDGER_DEPOSIT,
            Self::Withdraw => audit::LEDGER_WITHDRAW,
            Self::TransferOut => audit::LEDGER_TRANSFER_OUT,
            Self::TransferIn => audit::LEDGER_TRANSFER_IN,
        }
    }

    fn from_code(code: i16) -> Option<Self> {
        [
            Self::Deposit,
            Self::Withdraw,
            Self::TransferOut,
            Self::TransferIn,
        ]
        .into_iter()
        .find(|kind| i64::from(kind.code()) == i64::from(code))
    }
}

/// One immutable ledger entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BankLedgerEntry {
    pub entry_id: [u8; 16],
    pub account_id: [u8; 16],
    pub kind: BankLedgerKind,
    pub amount: u64,
    pub balance_before: u64,
    pub balance_after: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BankCoinDirection {
    Input,
    Output,
}

/// One coin line: an input a deposit consumed (wholly when `quantity_after` is 0) or an output
/// created in a new backpack entry (`quantity_before` 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BankCoinLine {
    pub direction: BankCoinDirection,
    pub item_instance_id: [u8; 16],
    pub coin: Coin,
    pub placement_ordinal: u64,
    pub quantity_before: u32,
    pub quantity_after: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankOperationRecord {
    pub occurrence: BankOperationOccurrence,
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub kind: BankOperationKind,
    pub amount: u64,
    pub result: BankResult,
    /// Empty for a refusal; one entry, or a transfer's TRANSFER_OUT then TRANSFER_IN.
    pub entries: Vec<BankLedgerEntry>,
    /// Inputs in plan order, then outputs; empty for a transfer or a refusal.
    pub coin_lines: Vec<BankCoinLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BankOperationOutcome {
    /// Written and committed by this call (a typed refusal included).
    Committed(BankOperationRecord),
    /// The occurrence was already decided with the same binding: its first outcome.
    Replayed(BankOperationRecord),
}

#[derive(Debug)]
pub enum BankError {
    /// A malformed request or an amount outside its `BANK0-RL-02` row; nothing is written.
    InvalidInput,
    /// A current recovery, session, lease, scope, node or root fence rejected the operation.
    AuthorityRejected,
    /// A deposit would consume more than 20 input stacks; nothing is written.
    CapacityExceeded,
    ConflictingOccurrence,
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for BankError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl From<sqlx::Error> for BankError {
    fn from(error: sqlx::Error) -> Self {
        Self::Unavailable(error.into())
    }
}

impl std::fmt::Display for BankError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => formatter.write_str("invalid bank operation input"),
            Self::AuthorityRejected => formatter.write_str("bank operation fence rejected"),
            Self::CapacityExceeded => formatter.write_str("bank deposit needs too many stacks"),
            Self::ConflictingOccurrence => {
                formatter.write_str("bank occurrence was reused with a different binding")
            }
            Self::Unavailable(error) => write!(formatter, "bank storage failed: {error:?}"),
        }
    }
}

impl std::error::Error for BankError {}

type Result<T> = std::result::Result<T, BankError>;

/// One operation of any kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BankRequest {
    Deposit(BankDepositRequest),
    Withdraw(BankWithdrawRequest),
    Transfer(BankTransferRequest),
}

impl BankRequest {
    fn identity(&self) -> &BankOperationIdentity {
        match self {
            Self::Deposit(request) => &request.identity,
            Self::Withdraw(request) => &request.identity,
            Self::Transfer(request) => &request.identity,
        }
    }

    fn kind(&self) -> BankOperationKind {
        match self {
            Self::Deposit(_) => BankOperationKind::Deposit,
            Self::Withdraw(_) => BankOperationKind::Withdraw,
            Self::Transfer(_) => BankOperationKind::Transfer,
        }
    }

    fn amount(&self) -> u64 {
        match self {
            Self::Deposit(request) => request.amount,
            Self::Withdraw(request) => request.amount,
            Self::Transfer(request) => request.amount,
        }
    }

    /// Crystal, platinum, gold; `None` where the kind plans no such output.
    fn planned_slots(&self) -> [Option<[u8; 16]>; 3] {
        match self {
            Self::Deposit(request) => [
                None,
                Some(request.change_item_instance_ids[0]),
                Some(request.change_item_instance_ids[1]),
            ],
            Self::Withdraw(request) => request.output_item_instance_ids.map(Some),
            Self::Transfer(_) => [None; 3],
        }
    }
}

/// The junior state of a character (BANK-0 §4.4, Q1 b): junior until it has left the starter
/// island (D119, DAWNPORT-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarterIslandFact {
    /// No starter island and departure fact exist yet: no character is junior.
    NoStarterIsland,
    OnIsland,
    Departed,
}

/// Whether the fact makes the character junior: only a character still on the island is.
pub fn is_junior(fact: StarterIslandFact) -> bool {
    matches!(fact, StarterIslandFact::OnIsland)
}

/// The departure fact of a character. Until the starter island exists (D119, DAWNPORT-1) there
/// is no fact to read, so every character is [`StarterIslandFact::NoStarterIsland`].
fn starter_island_fact(_character: CharacterId) -> StarterIslandFact {
    StarterIslandFact::NoStarterIsland
}

/// The party refusal of a transfer (BANK-0 §4.3): a junior sender is `JUNIOR_ACCOUNT`, a junior
/// recipient `RECIPIENT_CANNOT_RECEIVE_TRANSFERS`.
pub fn transfer_party_refusal(
    sender: StarterIslandFact,
    recipient: StarterIslandFact,
) -> Option<BankResult> {
    if is_junior(sender) {
        Some(BankResult::JuniorAccount)
    } else if is_junior(recipient) {
        Some(BankResult::RecipientCannotReceiveTransfers)
    } else {
        None
    }
}

impl DurabilityRoot {
    /// Deposit coins of the acting character's main backpack (BANK-0 §4.2). See the module
    /// documentation; every `Err` writes nothing.
    pub async fn bank_deposit(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: BankDepositRequest,
    ) -> Result<BankOperationOutcome> {
        self.bank_operation(authority, node, fence, BankRequest::Deposit(request))
            .await
    }

    /// Withdraw gold into new coin stacks of the acting character's main backpack (BANK-0 §4.2).
    pub async fn bank_withdraw(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: BankWithdrawRequest,
    ) -> Result<BankOperationOutcome> {
        self.bank_operation(authority, node, fence, BankRequest::Withdraw(request))
            .await
    }

    /// Transfer gold to another Account's character of the same World (BANK-0 §4.3).
    pub async fn bank_transfer(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: BankTransferRequest,
    ) -> Result<BankOperationOutcome> {
        self.bank_operation(authority, node, fence, BankRequest::Transfer(request))
            .await
    }

    async fn bank_operation(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: BankRequest,
    ) -> Result<BankOperationOutcome> {
        validate_request(&fence, &request)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| BankError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    match bank_operation_in_transaction(&mut tx, &node, &fence, &request).await {
                        Ok(outcome @ BankOperationOutcome::Committed(_)) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(outcome))
                        }
                        // A replay wrote nothing; the transaction rolls back on drop.
                        Ok(outcome) => Ok(Ok(outcome)),
                        Err(BankError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}

/// The request's own shape, before any read: identities, the amount within its row, the planned
/// slots and the coin facts.
pub(crate) fn validate_request(
    fence: &CurrentCharacterItemFence,
    request: &BankRequest,
) -> Result<()> {
    let identity = request.identity();
    let cap = match request {
        BankRequest::Deposit(_) => BANK_DEPOSIT_MAX,
        BankRequest::Withdraw(_) => BANK_WITHDRAW_MAX,
        BankRequest::Transfer(_) => BANK_TRANSFER_MAX,
    };
    let slots: Vec<[u8; 16]> = request.planned_slots().into_iter().flatten().collect();
    let mut identities = slots.clone();
    identities.extend([
        identity.transaction_id,
        identity.event_id,
        *identity.occurrence.as_bytes(),
    ]);
    let distinct = identities
        .iter()
        .enumerate()
        .all(|(index, id)| !identities[..index].contains(id));
    if !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        || !(1..=cap).contains(&request.amount())
        || identity.occurred_at_unix_ms <= 0
        || check_uuid_v7(&identity.transaction_id).is_err()
        || check_uuid_v7(&identity.event_id).is_err()
        || slots.iter().any(|slot| check_uuid_v7(slot).is_err())
        || !distinct
        || check_technical_text(&identity.server_build_id).is_err()
    {
        return Err(BankError::InvalidInput);
    }
    match request {
        BankRequest::Deposit(BankDepositRequest { coins, .. })
        | BankRequest::Withdraw(BankWithdrawRequest { coins, .. }) => {
            let coin = |definition: &TypedDefinitionRef, coin: Coin| {
                definition.family == COIN_DEFINITION_FAMILY
                    && definition.production_key == coin.production_key()
            };
            if !coin(&coins.gold, Coin::Gold)
                || !coin(&coins.platinum, Coin::Platinum)
                || !coin(&coins.crystal, Coin::Crystal)
                || !coins
                    .backpack
                    .container_capacity
                    .is_some_and(|capacity| (1..=BACKPACK_ENTRIES_MAX as u32).contains(&capacity))
            {
                return Err(BankError::InvalidInput);
            }
        }
        BankRequest::Transfer(request) => {
            let key = &request.recipient_name_key;
            if !(2..=29).contains(&key.len()) || !key.bytes().all(|byte| byte.is_ascii_lowercase())
            {
                return Err(BankError::InvalidInput);
            }
        }
    }
    Ok(())
}

/// SHA-256 over the whole semantic request (BANK-0 §3): the kind, the occurrence, the acting
/// character and World, the amount, the recipient and its confirmed name key, and the planned
/// output slots. The TransactionId, EventId, time, build and Content facts are excluded.
pub(crate) fn request_binding(fence: &CurrentCharacterItemFence, request: &BankRequest) -> Vec<u8> {
    let mut digest = Sha256::new();
    digest.update([REQUEST_BINDING_VERSION]);
    digest.update(request.kind().code().to_be_bytes());
    digest.update(request.identity().occurrence.as_bytes());
    digest.update(fence.character_id.as_bytes());
    digest.update(fence.runtime_scope.world_id().as_bytes());
    digest.update(request.amount().to_be_bytes());
    if let BankRequest::Transfer(transfer) = request {
        digest.update(transfer.recipient_character_id.as_bytes());
        digest.update(
            u32::try_from(transfer.recipient_name_key.len())
                .unwrap_or(u32::MAX)
                .to_be_bytes(),
        );
        digest.update(transfer.recipient_name_key.as_bytes());
    }
    for slot in request.planned_slots() {
        match slot {
            Some(slot) => {
                digest.update([1]);
                digest.update(slot);
            }
            None => digest.update([0]),
        }
    }
    digest.finalize().to_vec()
}

/// The acting character's scope and Account, proven by rule 2.
struct Acting {
    character_id: CharacterId,
    account_id: [u8; 16],
    world_id: WorldId,
    channel_id: ChannelId,
    scope_ownership_generation: u64,
}

/// One operation inside a transaction that already asserted the recovery fence and took the
/// admission relation locks. Never commits; a `Committed` outcome must be committed by the
/// caller, a `Replayed` one wrote nothing, and every error requires a rollback.
pub(crate) async fn bank_operation_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    request: &BankRequest,
) -> Result<BankOperationOutcome> {
    validate_request(fence, request)?;
    let binding = request_binding(fence, request);
    let occurrence = request.identity().occurrence;
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(\
         'oteryn:bank-operation:' || encode($1, 'hex'), 0))",
    )
    .bind(occurrence.as_bytes().as_slice())
    .execute(&mut **tx)
    .await?;
    if let Some(record) = load_operation(tx, occurrence, &binding).await? {
        return Ok(BankOperationOutcome::Replayed(record));
    }
    let Some(acting) = acting_fence(tx, node, fence).await? else {
        return Err(BankError::AuthorityRejected);
    };
    let record = match request {
        BankRequest::Deposit(request) => deposit(tx, &acting, request, &binding).await?,
        BankRequest::Withdraw(request) => withdraw(tx, &acting, request, &binding).await?,
        BankRequest::Transfer(request) => transfer(tx, &acting, request, &binding).await?,
    };
    Ok(BankOperationOutcome::Committed(record))
}

/// Rule 2 for the acting character, in the XP writer's order: the reconnect-session row, the
/// runtime-scope assignment and current node incarnation, and the admission guards. Takes no
/// root lock (the root step follows) and reads no recipient row. `None` means rejected.
async fn acting_fence(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
) -> std::result::Result<Option<Acting>, DurabilityError> {
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Ok(None);
    };
    let session = sqlx::query(
        "SELECT account_id::text FROM game_durability_reconnect_sessions \
         WHERE game_session_id = encode($1,'hex')::uuid \
           AND character_id = encode($2,'hex')::uuid \
           AND world_id = encode($3,'hex')::uuid \
           AND runtime_scope_kind = 1 \
           AND runtime_scope_world_id = encode($3,'hex')::uuid \
           AND runtime_scope_channel_id = encode($4,'hex')::uuid \
           AND runtime_scope_instance_id IS NULL \
           AND current_generation = $5::text::numeric(20,0) \
           AND character_lease_generation = $6::text::numeric(20,0) \
           AND scope_ownership_generation = $7::text::numeric(20,0) \
           AND session_state IN (1,2) FOR SHARE",
    )
    .bind(fence.game_session_id.as_bytes().as_slice())
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .bind(fence.connection_generation.get().to_string())
    .bind(fence.character_lease_generation.to_string())
    .bind(fence.scope_ownership_generation.get().to_string())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(session) = session else {
        return Ok(None);
    };
    let account_text: String = session.try_get("account_id")?;

    let key = scope_key(world_id, channel_id);
    let fact = node.fact();
    let assignment = sqlx::query(
        "SELECT 1 FROM game_runtime_scope_assignments \
         WHERE scope_key = $1 AND world_id = encode($2,'hex')::uuid \
           AND channel_id = encode($3,'hex')::uuid AND state = 1 \
           AND ownership_generation = $4::text::numeric(20,0) \
           AND holder_node_id = encode($5,'hex')::uuid \
           AND holder_registration_revision = $6::text::numeric(20,0) \
         FOR SHARE",
    )
    .bind(key.as_slice())
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .bind(fence.scope_ownership_generation.get().to_string())
    .bind(fact.node_id().as_bytes().as_slice())
    .bind(fact.registration_revision().to_string())
    .fetch_optional(&mut **tx)
    .await?;
    if assignment.is_none() || !prove_current_incarnation(tx, node).await? {
        return Ok(None);
    }

    let guards_ok: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 \
           FROM game_durability_admission_character_guards c \
           JOIN game_durability_admission_account_guards a \
             ON a.account_id = c.account_id \
           JOIN game_durability_admission_runtime_guards g \
             ON g.scope_key = $1 \
          WHERE c.character_id = encode($2,'hex')::uuid \
            AND c.account_id = $3::uuid \
            AND c.world_id = encode($4,'hex')::uuid \
            AND c.eligible \
            AND c.lease_generation = $5::text::numeric(20,0) \
            AND c.holder_game_session_id = encode($6,'hex')::uuid \
            AND a.presence_character_id = c.character_id \
            AND a.holder_game_session_id = c.holder_game_session_id \
            AND g.ready \
            AND g.ownership_generation = $7::text::numeric(20,0))",
    )
    .bind(key.as_slice())
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(&account_text)
    .bind(world_id.as_bytes().as_slice())
    .bind(fence.character_lease_generation.to_string())
    .bind(fence.game_session_id.as_bytes().as_slice())
    .bind(fence.scope_ownership_generation.get().to_string())
    .fetch_one(&mut **tx)
    .await?;
    if !guards_ok {
        return Ok(None);
    }
    Ok(Some(Acting {
        character_id: fence.character_id,
        account_id: uuid_text(&account_text)?,
        world_id,
        channel_id,
        scope_ownership_generation: fence.scope_ownership_generation.get(),
    }))
}

/// The root step of a deposit or withdrawal: the acting root FOR UPDATE, live, of the fenced
/// Account and World.
async fn lock_acting_root(connection: &mut PgConnection, acting: &Acting) -> Result<()> {
    let root = sqlx::query(
        "SELECT account_id::text, world_id::text FROM game_character_roots \
          WHERE character_id = encode($1,'hex')::uuid AND lifecycle = 1 FOR UPDATE",
    )
    .bind(acting.character_id.as_bytes().as_slice())
    .fetch_optional(&mut *connection)
    .await?
    .ok_or(BankError::AuthorityRejected)?;
    if uuid_text(&root.try_get::<String, _>("account_id")?)? != acting.account_id
        || uuid_text(&root.try_get::<String, _>("world_id")?)? != *acting.world_id.as_bytes()
    {
        return Err(BankError::AuthorityRejected);
    }
    Ok(())
}

/// A balance row, locked.
#[derive(Debug, Clone, Copy)]
struct Balance {
    balance: u64,
    last_entry_id: Option<[u8; 16]>,
}

/// The balance step: upsert the value-neutral zero row of each Account, then lock them all by
/// `account_id`. Returned in the order of `accounts`.
async fn lock_balances(
    connection: &mut PgConnection,
    world_id: WorldId,
    accounts: &[[u8; 16]],
) -> Result<Vec<Balance>> {
    let mut ordered = accounts.to_vec();
    ordered.sort_unstable();
    for account in &ordered {
        sqlx::query(
            "INSERT INTO game_account_bank_balances(account_id, world_id, balance, last_entry_id) \
             VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, 0, NULL) \
             ON CONFLICT (account_id, world_id) DO NOTHING",
        )
        .bind(account.as_slice())
        .bind(world_id.as_bytes().as_slice())
        .execute(&mut *connection)
        .await?;
    }
    let mut locked = Vec::new();
    for account in &ordered {
        let row = sqlx::query(
            "SELECT balance, last_entry_id::text FROM game_account_bank_balances \
              WHERE account_id = encode($1,'hex')::uuid AND world_id = encode($2,'hex')::uuid \
              FOR UPDATE",
        )
        .bind(account.as_slice())
        .bind(world_id.as_bytes().as_slice())
        .fetch_one(&mut *connection)
        .await?;
        let balance = u64::try_from(row.try_get::<i64, _>("balance")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let last_entry_id = row
            .try_get::<Option<String>, _>("last_entry_id")?
            .map(|text| uuid_text(&text))
            .transpose()?;
        locked.push((
            *account,
            Balance {
                balance,
                last_entry_id,
            },
        ));
    }
    accounts
        .iter()
        .map(|account| {
            locked
                .iter()
                .find(|(locked, _)| locked == account)
                .map(|(_, balance)| *balance)
                .ok_or_else(|| DurabilityError::InvalidStoredState.into())
        })
        .collect()
}

/// The equipped main backpack and its direct entries (highest ordinal first), the coin entries'
/// items locked. `None` when no backpack is equipped.
struct Backpack {
    item_instance_id: [u8; 16],
    capacity: usize,
    entries: usize,
    highest_ordinal: u64,
    /// Live coin stacks with their item id and definition revision.
    coins: Vec<(CoinStack, [u8; 16], String)>,
}

/// `inputs`: a deposit reads the coin stacks as its inputs, so each must be at its coin's current
/// revision; a withdrawal only places new entries and reads none.
async fn lock_backpack(
    connection: &mut PgConnection,
    acting: &Acting,
    coins: &BankCoinFacts,
    inputs: bool,
) -> Result<Option<Backpack>> {
    let backpack = sqlx::query(
        "SELECT i.item_instance_id::text, i.definition_family, i.definition_production_key, \
                i.definition_revision_ref \
           FROM game_item_container_slots s \
           JOIN game_item_instances i ON i.item_instance_id = s.item_instance_id \
          WHERE s.character_id = encode($1,'hex')::uuid",
    )
    .bind(acting.character_id.as_bytes().as_slice())
    .fetch_optional(&mut *connection)
    .await?;
    let Some(backpack) = backpack else {
        return Ok(None);
    };
    let declared = &coins.backpack.definition;
    if backpack.try_get::<String, _>("definition_family")? != declared.family
        || backpack.try_get::<String, _>("definition_production_key")? != declared.production_key
        || backpack.try_get::<String, _>("definition_revision_ref")? != declared.revision_ref
    {
        return Err(BankError::InvalidInput);
    }
    let capacity = coins
        .backpack
        .container_capacity
        .and_then(|capacity| usize::try_from(capacity).ok())
        .ok_or(BankError::InvalidInput)?;
    let item_instance_id = uuid_text(&backpack.try_get::<String, _>("item_instance_id")?)?;
    let entries = sqlx::query(
        "SELECT i.item_instance_id::text, e.placement_ordinal::text, i.world_id::text, \
                i.definition_family, i.definition_production_key, i.definition_revision_ref, \
                i.quantity, i.lifecycle \
           FROM game_item_container_entries e \
           JOIN game_item_instances i ON i.item_instance_id = e.item_instance_id \
          WHERE e.parent_item_instance_id = encode($1,'hex')::uuid \
          ORDER BY e.placement_ordinal DESC FOR UPDATE OF i",
    )
    .bind(item_instance_id.as_slice())
    .fetch_all(&mut *connection)
    .await?;
    let mut stacks = Vec::new();
    for row in entries.iter().filter(|_| inputs) {
        let family: String = row.try_get("definition_family")?;
        let key: String = row.try_get("definition_production_key")?;
        let Some(coin) = Coin::from_production_key(&key) else {
            continue;
        };
        if family != COIN_DEFINITION_FAMILY || row.try_get::<i16, _>("lifecycle")? != 1 {
            continue;
        }
        if uuid_text(&row.try_get::<String, _>("world_id")?)? != *acting.world_id.as_bytes() {
            return Err(DurabilityError::InvalidStoredState.into());
        }
        // A coin stack at another revision is not skipped: the database plan guard counts
        // every coin stack (as for a fee, decision §4.2).
        let compatible = match coin {
            Coin::Gold => &coins.gold,
            Coin::Platinum => &coins.platinum,
            Coin::Crystal => &coins.crystal,
        };
        let revision: String = row.try_get("definition_revision_ref")?;
        if revision != compatible.revision_ref {
            return Err(BankError::InvalidInput);
        }
        stacks.push((
            CoinStack {
                coin,
                quantity: u32::try_from(row.try_get::<i64, _>("quantity")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                placement_ordinal: numeric_u64(row, "placement_ordinal")?,
            },
            uuid_text(&row.try_get::<String, _>("item_instance_id")?)?,
            revision,
        ));
    }
    let highest_ordinal = match entries.first() {
        Some(row) => numeric_u64(row, "placement_ordinal")?,
        None => 0,
    };
    Ok(Some(Backpack {
        item_instance_id,
        capacity,
        entries: entries.len(),
        highest_ordinal,
        coins: stacks,
    }))
}

/// The output coins of `split` in `slots` (crystal, platinum, gold) from `first_ordinal`, each
/// only when positive, in consecutive new entries. `None` past `u64::MAX`.
fn outputs(
    split: [(Coin, u64); 3],
    slots: [[u8; 16]; 3],
    first_ordinal: u64,
) -> Option<Vec<BankCoinLine>> {
    let mut ordinal = first_ordinal;
    let mut lines = Vec::new();
    for ((coin, quantity), slot) in split.into_iter().zip(slots) {
        if quantity == 0 {
            continue;
        }
        if ordinal == 0 {
            return None;
        }
        lines.push(BankCoinLine {
            direction: BankCoinDirection::Output,
            item_instance_id: slot,
            coin,
            placement_ordinal: ordinal,
            quantity_before: 0,
            quantity_after: u32::try_from(quantity).ok()?,
        });
        ordinal = ordinal.checked_add(1).unwrap_or(0);
    }
    Some(lines)
}

/// A withdrawal's canonical split: crystal, then platinum, then gold (BANK-0 §4.2).
fn withdraw_split(amount: u64) -> [(Coin, u64); 3] {
    [
        (Coin::Crystal, amount / 10_000),
        (Coin::Platinum, amount % 10_000 / 100),
        (Coin::Gold, amount % 100),
    ]
}

async fn new_entry_id(connection: &mut PgConnection) -> Result<[u8; 16]> {
    let text: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
        .fetch_one(&mut *connection)
        .await?;
    Ok(uuid_text(&text)?)
}

fn to_i64(value: u64) -> Result<i64> {
    i64::try_from(value).map_err(|_| BankError::InvalidInput)
}

/// Insert the operation row: the outcome, and for a committed deposit or withdrawal its backpack
/// and for a committed operation its envelope digest.
#[allow(clippy::too_many_arguments)]
async fn insert_operation(
    connection: &mut PgConnection,
    acting: &Acting,
    request: &BankRequest,
    binding: &[u8],
    result: BankResult,
    backpack: Option<[u8; 16]>,
    envelope: Option<&[u8]>,
) -> Result<()> {
    let identity = request.identity();
    let (recipient, name_key) = match request {
        BankRequest::Transfer(transfer) => (
            Some(transfer.recipient_character_id.as_bytes().to_vec()),
            Some(transfer.recipient_name_key.clone()),
        ),
        _ => (None, None),
    };
    let [crystal, platinum, gold] = request.planned_slots();
    sqlx::query(
        "INSERT INTO game_account_bank_operations(operation_occurrence_id, transaction_id, \
           event_id, kind, request_binding, acting_character_id, account_id, world_id, \
           channel_id, runtime_scope_ownership_generation, amount, recipient_character_id, \
           recipient_name_key, planned_crystal_item_instance_id, \
           planned_platinum_item_instance_id, planned_gold_item_instance_id, \
           backpack_item_instance_id, outcome, occurred_at, envelope_sha256, committed_at) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, $4, $5, \
           encode($6,'hex')::uuid, encode($7,'hex')::uuid, encode($8,'hex')::uuid, \
           encode($9,'hex')::uuid, $10::text::numeric(20,0), $11, encode($12,'hex')::uuid, $13, \
           encode($14,'hex')::uuid, encode($15,'hex')::uuid, encode($16,'hex')::uuid, \
           encode($17,'hex')::uuid, $18, $19, sha256($20), \
           floor(extract(epoch FROM statement_timestamp())*1000)::bigint)",
    )
    .bind(identity.occurrence.as_bytes().as_slice())
    .bind(identity.transaction_id.as_slice())
    .bind(identity.event_id.as_slice())
    .bind(request.kind().code())
    .bind(binding)
    .bind(acting.character_id.as_bytes().as_slice())
    .bind(acting.account_id.as_slice())
    .bind(acting.world_id.as_bytes().as_slice())
    .bind(acting.channel_id.as_bytes().as_slice())
    .bind(acting.scope_ownership_generation.to_string())
    .bind(to_i64(request.amount())?)
    .bind(recipient)
    .bind(name_key)
    .bind(crystal.map(|slot| slot.to_vec()))
    .bind(platinum.map(|slot| slot.to_vec()))
    .bind(gold.map(|slot| slot.to_vec()))
    .bind(backpack.map(|slot| slot.to_vec()))
    .bind(result.code())
    .bind(identity.occurred_at_unix_ms)
    .bind(envelope)
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// Record a typed refusal: its operation row only.
async fn refuse(
    connection: &mut PgConnection,
    acting: &Acting,
    request: &BankRequest,
    binding: &[u8],
    result: BankResult,
) -> Result<BankOperationRecord> {
    insert_operation(connection, acting, request, binding, result, None, None).await?;
    let identity = request.identity();
    Ok(BankOperationRecord {
        occurrence: identity.occurrence,
        transaction_id: identity.transaction_id,
        event_id: identity.event_id,
        kind: request.kind(),
        amount: request.amount(),
        result,
        entries: Vec::new(),
        coin_lines: Vec::new(),
    })
}

/// Insert one ledger entry chained from `balance` and move the balance row to it.
#[allow(clippy::too_many_arguments)]
async fn insert_entry(
    connection: &mut PgConnection,
    transaction_id: &[u8; 16],
    world_id: WorldId,
    entry: &BankLedgerEntry,
    previous: Option<[u8; 16]>,
    acting: CharacterId,
    recipient: Option<CharacterId>,
    counterpart: Option<[u8; 16]>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO game_account_bank_entries(entry_id, transaction_id, account_id, world_id, \
           kind, amount, balance_before, balance_after, previous_entry_id, acting_character_id, \
           recipient_character_id, counterpart_entry_id) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, \
           encode($4,'hex')::uuid, $5, $6, $7, $8, encode($9,'hex')::uuid, \
           encode($10,'hex')::uuid, encode($11,'hex')::uuid, encode($12,'hex')::uuid)",
    )
    .bind(entry.entry_id.as_slice())
    .bind(transaction_id.as_slice())
    .bind(entry.account_id.as_slice())
    .bind(world_id.as_bytes().as_slice())
    .bind(i16::try_from(entry.kind.code()).map_err(|_| BankError::InvalidInput)?)
    .bind(to_i64(entry.amount)?)
    .bind(to_i64(entry.balance_before)?)
    .bind(to_i64(entry.balance_after)?)
    .bind(previous.map(|id| id.to_vec()))
    .bind(acting.as_bytes().as_slice())
    .bind(recipient.map(|id| id.as_bytes().to_vec()))
    .bind(counterpart.map(|id| id.to_vec()))
    .execute(&mut *connection)
    .await?;
    let moved = sqlx::query(
        "UPDATE game_account_bank_balances SET balance = $3, last_entry_id = encode($4,'hex')::uuid \
          WHERE account_id = encode($1,'hex')::uuid AND world_id = encode($2,'hex')::uuid \
            AND balance = $5",
    )
    .bind(entry.account_id.as_slice())
    .bind(world_id.as_bytes().as_slice())
    .bind(to_i64(entry.balance_after)?)
    .bind(entry.entry_id.as_slice())
    .bind(to_i64(entry.balance_before)?)
    .execute(&mut *connection)
    .await?;
    if moved.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    Ok(())
}

/// Whether a fee payer is junior (BANK-0 §4.4, Q1 b): a junior payer keeps stage 1 and never
/// pays a fee from the bank (BANK-FEE-0 §3).
pub(super) fn fee_payer_is_junior(character: CharacterId) -> bool {
    is_junior(starter_island_fact(character))
}

/// The payer's balance row, locked for a fee's bank part (GOLD-FEE-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FeeDebitBalance {
    pub(super) balance: u64,
    last_entry_id: Option<[u8; 16]>,
}

/// The `FEE_DEBIT` entry of one fee, as written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FeeDebitEntry {
    pub(super) entry_id: [u8; 16],
    pub(super) amount: u64,
    pub(super) balance_before: u64,
    pub(super) balance_after: u64,
}

/// The balance step of a fee's bank part (BANK-0 §4.1, BANK-FEE-0 §4.2): after the fee
/// transaction locked the payer root, the backpack and its coin entries, upsert the value-neutral
/// zero row of the payer's (Account, World) and lock it.
pub(super) async fn lock_fee_debit_balance(
    connection: &mut PgConnection,
    world_id: WorldId,
    account_id: [u8; 16],
) -> std::result::Result<FeeDebitBalance, DurabilityError> {
    match lock_balances(connection, world_id, &[account_id]).await {
        Ok(locked) => locked
            .first()
            .map(|balance| FeeDebitBalance {
                balance: balance.balance,
                last_entry_id: balance.last_entry_id,
            })
            .ok_or(DurabilityError::InvalidStoredState),
        Err(BankError::Unavailable(error)) => Err(error),
        Err(_) => Err(DurabilityError::InvalidStoredState),
    }
}

/// The `FEE_DEBIT` entry writer (GOLD-FEE-2, BANK-FEE-0 §4.2): one entry of `amount` that
/// references the fee record `fee_transaction_id` instead of a bank operation, chained from the
/// locked `balance`, and the balance row moved to it. The caller has checked the amount against
/// the balance; it writes no operation, coin line or bank event.
pub(super) async fn insert_fee_debit_entry(
    connection: &mut PgConnection,
    fee_transaction_id: &[u8; 16],
    world_id: WorldId,
    account_id: [u8; 16],
    payer: CharacterId,
    balance: &FeeDebitBalance,
    amount: u64,
) -> std::result::Result<FeeDebitEntry, DurabilityError> {
    let balance_after = balance
        .balance
        .checked_sub(amount)
        .filter(|_| amount > 0)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let entry = FeeDebitEntry {
        entry_id: new_entry_id(connection)
            .await
            .map_err(|error| match error {
                BankError::Unavailable(error) => error,
                _ => DurabilityError::InvalidStoredState,
            })?,
        amount,
        balance_before: balance.balance,
        balance_after,
    };
    let gold = |value: u64| i64::try_from(value).map_err(|_| DurabilityError::InvalidStoredState);
    sqlx::query(
        "INSERT INTO game_account_bank_entries(entry_id, fee_transaction_id, account_id, \
           world_id, kind, amount, balance_before, balance_after, previous_entry_id, \
           acting_character_id) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, \
           encode($4,'hex')::uuid, $5, $6, $7, $8, encode($9,'hex')::uuid, \
           encode($10,'hex')::uuid)",
    )
    .bind(entry.entry_id.as_slice())
    .bind(fee_transaction_id.as_slice())
    .bind(account_id.as_slice())
    .bind(world_id.as_bytes().as_slice())
    .bind(
        i16::try_from(super::item_fee_burn_audit::LEDGER_FEE_DEBIT)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    )
    .bind(gold(amount)?)
    .bind(gold(entry.balance_before)?)
    .bind(gold(entry.balance_after)?)
    .bind(balance.last_entry_id.map(|id| id.to_vec()))
    .bind(payer.as_bytes().as_slice())
    .execute(&mut *connection)
    .await?;
    let moved = sqlx::query(
        "UPDATE game_account_bank_balances SET balance = $3, last_entry_id = encode($4,'hex')::uuid \
          WHERE account_id = encode($1,'hex')::uuid AND world_id = encode($2,'hex')::uuid \
            AND balance = $5",
    )
    .bind(account_id.as_slice())
    .bind(world_id.as_bytes().as_slice())
    .bind(gold(entry.balance_after)?)
    .bind(entry.entry_id.as_slice())
    .bind(gold(entry.balance_before)?)
    .execute(&mut *connection)
    .await?;
    if moved.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(entry)
}

/// Insert the coin lines and apply them to the items: inputs change (a whole input retires and
/// its entry ends), outputs are created in new entries.
async fn apply_coin_lines(
    connection: &mut PgConnection,
    acting: &Acting,
    transaction_id: &[u8; 16],
    backpack: &[u8; 16],
    lines: &[BankCoinLine],
    coins: &BankCoinFacts,
) -> Result<()> {
    for (ordinal, line) in (1_i16..).zip(lines) {
        if line.direction == BankCoinDirection::Output {
            let definition = match line.coin {
                Coin::Gold => &coins.gold,
                Coin::Platinum => &coins.platinum,
                Coin::Crystal => &coins.crystal,
            };
            sqlx::query(
                "INSERT INTO game_item_instances(item_instance_id, world_id, definition_family, \
                   definition_production_key, definition_revision_ref, quantity, lifecycle, \
                   minted_transaction_id) \
                 VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3, $4, $5, $6, 1, \
                   encode($7,'hex')::uuid)",
            )
            .bind(line.item_instance_id.as_slice())
            .bind(acting.world_id.as_bytes().as_slice())
            .bind(&definition.family)
            .bind(&definition.production_key)
            .bind(&definition.revision_ref)
            .bind(i64::from(line.quantity_after))
            .bind(transaction_id.as_slice())
            .execute(&mut *connection)
            .await?;
            sqlx::query(
                "INSERT INTO game_item_container_entries(item_instance_id, world_id, \
                   character_id, parent_item_instance_id, placement_ordinal, \
                   placed_transaction_id) \
                 VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, \
                   encode($3,'hex')::uuid, encode($4,'hex')::uuid, $5::text::numeric(20,0), \
                   encode($6,'hex')::uuid)",
            )
            .bind(line.item_instance_id.as_slice())
            .bind(acting.world_id.as_bytes().as_slice())
            .bind(acting.character_id.as_bytes().as_slice())
            .bind(backpack.as_slice())
            .bind(line.placement_ordinal.to_string())
            .bind(transaction_id.as_slice())
            .execute(&mut *connection)
            .await?;
        }
        sqlx::query(
            "INSERT INTO game_account_bank_coin_lines(transaction_id, line_ordinal, direction, \
               item_instance_id, coin_worth, placement_ordinal, quantity_before, quantity_after) \
             VALUES (encode($1,'hex')::uuid, $2, $3, encode($4,'hex')::uuid, $5, \
               $6::text::numeric(20,0), $7, $8)",
        )
        .bind(transaction_id.as_slice())
        .bind(ordinal)
        .bind(match line.direction {
            BankCoinDirection::Input => 1_i16,
            BankCoinDirection::Output => 2,
        })
        .bind(line.item_instance_id.as_slice())
        .bind(to_i64(line.coin.worth())?)
        .bind(line.placement_ordinal.to_string())
        .bind(i64::from(line.quantity_before))
        .bind(i64::from(line.quantity_after))
        .execute(&mut *connection)
        .await?;
        if line.direction == BankCoinDirection::Output {
            continue;
        }
        let changed = sqlx::query(
            "UPDATE game_item_instances \
                SET quantity = $2, lifecycle = $3, last_transaction_id = encode($4,'hex')::uuid \
              WHERE item_instance_id = encode($1,'hex')::uuid AND lifecycle = 1 AND quantity = $5",
        )
        .bind(line.item_instance_id.as_slice())
        .bind(i64::from(line.quantity_after))
        .bind(if line.quantity_after == 0 { 2_i16 } else { 1 })
        .bind(transaction_id.as_slice())
        .bind(i64::from(line.quantity_before))
        .execute(&mut *connection)
        .await?;
        if changed.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState.into());
        }
        if line.quantity_after == 0 {
            let removed = sqlx::query(
                "DELETE FROM game_item_container_entries \
                  WHERE item_instance_id = encode($1,'hex')::uuid \
                    AND parent_item_instance_id = encode($2,'hex')::uuid \
                    AND placement_ordinal = $3::text::numeric(20,0)",
            )
            .bind(line.item_instance_id.as_slice())
            .bind(backpack.as_slice())
            .bind(line.placement_ordinal.to_string())
            .execute(&mut *connection)
            .await?;
            if removed.rows_affected() != 1 {
                return Err(DurabilityError::InvalidStoredState.into());
            }
        }
    }
    Ok(())
}

async fn insert_event(
    connection: &mut PgConnection,
    identity: &BankOperationIdentity,
    world_id: WorldId,
    envelope: &[u8],
) -> Result<()> {
    sqlx::query(
        "INSERT INTO game_account_bank_audit_outbox(event_id, transaction_id, \
           transaction_ordinal, transaction_count, event_type_id, schema_revision, \
           retention_profile_id, world_id, occurred_at, expires_at, envelope, envelope_sha256, \
           publication_state) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, 1, $3, $4, $5, \
           encode($6,'hex')::uuid, $7, $7 + $8, $9, sha256($9), 1)",
    )
    .bind(identity.event_id.as_slice())
    .bind(identity.transaction_id.as_slice())
    .bind(i64::from(audit::EVENT_TYPE_ID))
    .bind(i64::from(audit::EVENT_SCHEMA_REVISION))
    .bind(audit::RETENTION_PROFILE_ID)
    .bind(world_id.as_bytes().as_slice())
    .bind(identity.occurred_at_unix_ms)
    .bind(audit::RETENTION_P30D_MS)
    .bind(envelope)
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// The event payload's item state of a coin line.
fn coin_state(
    acting: &Acting,
    item: &[u8; 16],
    coin: Coin,
    revision: &str,
    quantity: u32,
) -> BankItemStateV1 {
    BankItemStateV1 {
        item_instance_id: item.to_vec(),
        world_id: acting.world_id.as_bytes().to_vec(),
        definition: Some(BankItemDefinitionV1 {
            family: COIN_DEFINITION_FAMILY.into(),
            production_key: coin.production_key().into(),
            revision_ref: revision.into(),
        }),
        quantity,
        lifecycle: if quantity == 0 {
            ITEM_LIFECYCLE_RETIRED
        } else {
            ITEM_LIFECYCLE_LIVE
        },
    }
}

fn value_line(acting: &Acting, entry: &BankLedgerEntry, class: u32) -> BankValueLineV1 {
    BankValueLineV1 {
        entry_id: entry.entry_id.to_vec(),
        asset: ASSET_GOLD.into(),
        account_id: entry.account_id.to_vec(),
        world_id: acting.world_id.as_bytes().to_vec(),
        kind: entry.kind.code(),
        line_class: class,
        amount: entry.amount,
        balance_before: entry.balance_before,
        balance_after: entry.balance_after,
    }
}

fn encode_event(
    acting: &Acting,
    request: &BankRequest,
    cause: BankCauseV1,
    backpack: Option<[u8; 16]>,
    coin_lines: Vec<BankCoinLineV1>,
    value_lines: Vec<BankValueLineV1>,
    recipient: Option<CharacterId>,
) -> Result<Vec<u8>> {
    let identity = request.identity();
    audit::encode_bank_event(
        BankEventIdentity {
            event_id: identity.event_id,
            transaction_id: identity.transaction_id,
            occurred_at_unix_ms: identity.occurred_at_unix_ms,
            server_build_id: &identity.server_build_id,
        },
        BankOperationV1 {
            interpretation_revision: audit::INTERPRETATION_REVISION,
            cause: Some(BankOperationCauseV1 { cause: Some(cause) }),
            acting_character_id: acting.character_id.as_bytes().to_vec(),
            world_id: acting.world_id.as_bytes().to_vec(),
            channel_id: acting.channel_id.as_bytes().to_vec(),
            runtime_scope_ownership_generation: acting.scope_ownership_generation,
            amount: request.amount(),
            backpack_item_instance_id: backpack.map(|id| id.to_vec()).unwrap_or_default(),
            coin_lines,
            value_lines,
            recipient_character_id: recipient
                .map(|id| id.as_bytes().to_vec())
                .unwrap_or_default(),
        },
    )
    .map_err(|_| BankError::InvalidInput)
}

fn committed(
    request: &BankRequest,
    entries: Vec<BankLedgerEntry>,
    coin_lines: Vec<BankCoinLine>,
) -> BankOperationRecord {
    let identity = request.identity();
    BankOperationRecord {
        occurrence: identity.occurrence,
        transaction_id: identity.transaction_id,
        event_id: identity.event_id,
        kind: request.kind(),
        amount: request.amount(),
        result: BankResult::Ok,
        entries,
        coin_lines,
    }
}

async fn deposit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    acting: &Acting,
    deposit: &BankDepositRequest,
    binding: &[u8],
) -> Result<BankOperationRecord> {
    let request = BankRequest::Deposit(deposit.clone());
    lock_acting_root(tx, acting).await?;
    if is_junior(starter_island_fact(acting.character_id)) {
        return refuse(tx, acting, &request, binding, BankResult::JuniorAccount).await;
    }
    let Some(backpack) = lock_backpack(tx, acting, &deposit.coins, true).await? else {
        return refuse(tx, acting, &request, binding, BankResult::InsufficientCoins).await;
    };
    let stacks: Vec<CoinStack> = backpack.coins.iter().map(|(stack, _, _)| *stack).collect();
    let plan = match plan_fee_within(deposit.amount, &stacks, backpack.entries, backpack.capacity) {
        Ok(plan) => plan,
        Err(FeePlanError::InsufficientFunds) => {
            return refuse(tx, acting, &request, binding, BankResult::InsufficientCoins).await;
        }
        Err(FeePlanError::ChangeDoesNotFit) => {
            return refuse(tx, acting, &request, binding, BankResult::NoRoom).await;
        }
        Err(FeePlanError::CapacityExceeded) => return Err(BankError::CapacityExceeded),
        // Stored stacks outside 1..=100 or repeated ordinals are not a caller error.
        Err(FeePlanError::InvalidInput) => return Err(DurabilityError::InvalidStoredState.into()),
    };
    let first_ordinal = backpack.highest_ordinal.checked_add(1).unwrap_or(0);
    let [platinum_slot, gold_slot] = deposit.change_item_instance_ids;
    let Some(change) = outputs(
        [
            (Coin::Crystal, 0),
            (Coin::Platinum, plan.change / 100),
            (Coin::Gold, plan.change % 100),
        ],
        [[0; 16], platinum_slot, gold_slot],
        first_ordinal,
    ) else {
        return refuse(tx, acting, &request, binding, BankResult::NoRoom).await;
    };

    let [balance] = lock_balances(tx, acting.world_id, &[acting.account_id]).await?[..] else {
        return Err(DurabilityError::InvalidStoredState.into());
    };
    let Some(after) = balance
        .balance
        .checked_add(deposit.amount)
        .filter(|after| *after <= BANK_BALANCE_MAX)
    else {
        return refuse(tx, acting, &request, binding, BankResult::BalanceLimit).await;
    };

    let mut lines: Vec<BankCoinLine> = plan
        .lines
        .iter()
        .map(|line| {
            let (stack, item, _) = &backpack.coins[line.input];
            BankCoinLine {
                direction: BankCoinDirection::Input,
                item_instance_id: *item,
                coin: stack.coin,
                placement_ordinal: stack.placement_ordinal,
                quantity_before: stack.quantity,
                quantity_after: stack.quantity - line.burned,
            }
        })
        .collect();
    let mut audit_lines: Vec<BankCoinLineV1> = plan
        .lines
        .iter()
        .zip(&lines)
        .map(|(planned, line)| {
            let (_, item, revision) = &backpack.coins[planned.input];
            BankCoinLineV1 {
                direction: COIN_LINE_INPUT,
                before: Some(coin_state(
                    acting,
                    item,
                    line.coin,
                    revision,
                    line.quantity_before,
                )),
                after: Some(coin_state(
                    acting,
                    item,
                    line.coin,
                    revision,
                    line.quantity_after,
                )),
                placement_ordinal: line.placement_ordinal,
                line_class: LINE_CLASS_CONVERSION,
            }
        })
        .collect();
    audit_lines.extend(
        change
            .iter()
            .map(|line| output_audit_line(acting, line, &deposit.coins)),
    );
    lines.extend(change);
    let entry = BankLedgerEntry {
        entry_id: new_entry_id(tx).await?,
        account_id: acting.account_id,
        kind: BankLedgerKind::Deposit,
        amount: deposit.amount,
        balance_before: balance.balance,
        balance_after: after,
    };
    let envelope = encode_event(
        acting,
        &request,
        BankCauseV1::Conversion(BankConversionCauseV1 {
            kind: CONVERSION_DEPOSIT,
            occurrence: deposit.identity.occurrence.as_bytes().to_vec(),
        }),
        Some(backpack.item_instance_id),
        audit_lines,
        vec![value_line(acting, &entry, LINE_CLASS_CONVERSION)],
        None,
    )?;
    write_conversion(
        tx,
        acting,
        &request,
        binding,
        &backpack.item_instance_id,
        &entry,
        balance,
        &lines,
        &deposit.coins,
        &envelope,
    )
    .await?;
    Ok(committed(&request, vec![entry], lines))
}

fn output_audit_line(
    acting: &Acting,
    line: &BankCoinLine,
    coins: &BankCoinFacts,
) -> BankCoinLineV1 {
    let revision = match line.coin {
        Coin::Gold => &coins.gold.revision_ref,
        Coin::Platinum => &coins.platinum.revision_ref,
        Coin::Crystal => &coins.crystal.revision_ref,
    };
    BankCoinLineV1 {
        direction: COIN_LINE_OUTPUT,
        before: None,
        after: Some(coin_state(
            acting,
            &line.item_instance_id,
            line.coin,
            revision,
            line.quantity_after,
        )),
        placement_ordinal: line.placement_ordinal,
        line_class: LINE_CLASS_CONVERSION,
    }
}

/// The writes of a committed deposit or withdrawal, in foreign-key order.
#[allow(clippy::too_many_arguments)]
async fn write_conversion(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    acting: &Acting,
    request: &BankRequest,
    binding: &[u8],
    backpack: &[u8; 16],
    entry: &BankLedgerEntry,
    balance: Balance,
    lines: &[BankCoinLine],
    coins: &BankCoinFacts,
    envelope: &[u8],
) -> Result<()> {
    let identity = request.identity();
    insert_operation(
        tx,
        acting,
        request,
        binding,
        BankResult::Ok,
        Some(*backpack),
        Some(envelope),
    )
    .await?;
    insert_entry(
        tx,
        &identity.transaction_id,
        acting.world_id,
        entry,
        balance.last_entry_id,
        acting.character_id,
        None,
        None,
    )
    .await?;
    apply_coin_lines(tx, acting, &identity.transaction_id, backpack, lines, coins).await?;
    insert_event(tx, identity, acting.world_id, envelope).await
}

async fn withdraw(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    acting: &Acting,
    withdraw: &BankWithdrawRequest,
    binding: &[u8],
) -> Result<BankOperationRecord> {
    let request = BankRequest::Withdraw(withdraw.clone());
    lock_acting_root(tx, acting).await?;
    if is_junior(starter_island_fact(acting.character_id)) {
        return refuse(tx, acting, &request, binding, BankResult::JuniorAccount).await;
    }
    let Some(backpack) = lock_backpack(tx, acting, &withdraw.coins, false).await? else {
        return refuse(tx, acting, &request, binding, BankResult::NoRoom).await;
    };
    let first_ordinal = backpack.highest_ordinal.checked_add(1).unwrap_or(0);
    let lines = match outputs(
        withdraw_split(withdraw.amount),
        withdraw.output_item_instance_ids,
        first_ordinal,
    ) {
        Some(lines) if backpack.entries + lines.len() <= backpack.capacity => lines,
        _ => return refuse(tx, acting, &request, binding, BankResult::NoRoom).await,
    };

    let [balance] = lock_balances(tx, acting.world_id, &[acting.account_id]).await?[..] else {
        return Err(DurabilityError::InvalidStoredState.into());
    };
    let Some(after) = balance.balance.checked_sub(withdraw.amount) else {
        return refuse(
            tx,
            acting,
            &request,
            binding,
            BankResult::InsufficientBalance,
        )
        .await;
    };
    let entry = BankLedgerEntry {
        entry_id: new_entry_id(tx).await?,
        account_id: acting.account_id,
        kind: BankLedgerKind::Withdraw,
        amount: withdraw.amount,
        balance_before: balance.balance,
        balance_after: after,
    };
    let envelope = encode_event(
        acting,
        &request,
        BankCauseV1::Conversion(BankConversionCauseV1 {
            kind: CONVERSION_WITHDRAW,
            occurrence: withdraw.identity.occurrence.as_bytes().to_vec(),
        }),
        Some(backpack.item_instance_id),
        lines
            .iter()
            .map(|line| output_audit_line(acting, line, &withdraw.coins))
            .collect(),
        vec![value_line(acting, &entry, LINE_CLASS_CONVERSION)],
        None,
    )?;
    write_conversion(
        tx,
        acting,
        &request,
        binding,
        &backpack.item_instance_id,
        &entry,
        balance,
        &lines,
        &withdraw.coins,
        &envelope,
    )
    .await?;
    Ok(committed(&request, vec![entry], lines))
}

async fn transfer(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    acting: &Acting,
    transfer: &BankTransferRequest,
    binding: &[u8],
) -> Result<BankOperationRecord> {
    let request = BankRequest::Transfer(transfer.clone());
    let recipient_id = transfer.recipient_character_id;
    // The root step: both roots in CharacterId order, the sender FOR UPDATE and the recipient
    // FOR SHARE, before any balance row.
    let mut sender_ok = false;
    let mut recipient = None;
    let mut ids = vec![acting.character_id];
    if recipient_id != acting.character_id {
        ids.push(recipient_id);
    }
    ids.sort_unstable_by_key(|id| *id.as_bytes());
    for id in ids {
        if id == acting.character_id {
            lock_acting_root(tx, acting).await?;
            sender_ok = true;
        } else {
            recipient = sqlx::query(
                "SELECT account_id::text, world_id::text, name_key FROM game_character_roots \
                  WHERE character_id = encode($1,'hex')::uuid AND lifecycle = 1 FOR SHARE",
            )
            .bind(id.as_bytes().as_slice())
            .fetch_optional(&mut **tx)
            .await?;
        }
    }
    if !sender_ok {
        return Err(BankError::AuthorityRejected);
    }
    if is_junior(starter_island_fact(acting.character_id)) {
        return refuse(tx, acting, &request, binding, BankResult::JuniorAccount).await;
    }
    if recipient_id == acting.character_id {
        return refuse(tx, acting, &request, binding, BankResult::SameAccount).await;
    }
    let recipient_account = match recipient {
        Some(row)
            if uuid_text(&row.try_get::<String, _>("world_id")?)?
                == *acting.world_id.as_bytes()
                && row.try_get::<String, _>("name_key")? == transfer.recipient_name_key =>
        {
            uuid_text(&row.try_get::<String, _>("account_id")?)?
        }
        _ => return refuse(tx, acting, &request, binding, BankResult::UnknownRecipient).await,
    };
    if recipient_account == acting.account_id {
        return refuse(tx, acting, &request, binding, BankResult::SameAccount).await;
    }
    if let Some(result) = transfer_party_refusal(
        starter_island_fact(acting.character_id),
        starter_island_fact(recipient_id),
    ) {
        return refuse(tx, acting, &request, binding, result).await;
    }

    let [sender, receiver] =
        lock_balances(tx, acting.world_id, &[acting.account_id, recipient_account]).await?[..]
    else {
        return Err(DurabilityError::InvalidStoredState.into());
    };
    let Some(sender_after) = sender.balance.checked_sub(transfer.amount) else {
        return refuse(
            tx,
            acting,
            &request,
            binding,
            BankResult::InsufficientBalance,
        )
        .await;
    };
    let Some(receiver_after) = receiver
        .balance
        .checked_add(transfer.amount)
        .filter(|after| *after <= BANK_BALANCE_MAX)
    else {
        return refuse(
            tx,
            acting,
            &request,
            binding,
            BankResult::RecipientCannotReceiveTransfers,
        )
        .await;
    };

    let out = BankLedgerEntry {
        entry_id: new_entry_id(tx).await?,
        account_id: acting.account_id,
        kind: BankLedgerKind::TransferOut,
        amount: transfer.amount,
        balance_before: sender.balance,
        balance_after: sender_after,
    };
    let into = BankLedgerEntry {
        entry_id: new_entry_id(tx).await?,
        account_id: recipient_account,
        kind: BankLedgerKind::TransferIn,
        amount: transfer.amount,
        balance_before: receiver.balance,
        balance_after: receiver_after,
    };
    let envelope = encode_event(
        acting,
        &request,
        BankCauseV1::Transfer(BankTransferCauseV1 {
            occurrence: transfer.identity.occurrence.as_bytes().to_vec(),
        }),
        None,
        Vec::new(),
        vec![
            value_line(acting, &out, LINE_CLASS_TRANSFER),
            value_line(acting, &into, LINE_CLASS_TRANSFER),
        ],
        Some(recipient_id),
    )?;
    let identity = &transfer.identity;
    insert_operation(
        tx,
        acting,
        &request,
        binding,
        BankResult::Ok,
        None,
        Some(&envelope),
    )
    .await?;
    for (entry, previous, counterpart) in [
        (&out, sender.last_entry_id, into.entry_id),
        (&into, receiver.last_entry_id, out.entry_id),
    ] {
        insert_entry(
            tx,
            &identity.transaction_id,
            acting.world_id,
            entry,
            previous,
            acting.character_id,
            Some(recipient_id),
            Some(counterpart),
        )
        .await?;
    }
    insert_event(tx, identity, acting.world_id, &envelope).await?;
    Ok(committed(&request, vec![out, into], Vec::new()))
}

/// The stored operation of `occurrence`: its first outcome when `binding` matches, a conflict
/// otherwise, `None` when it was never decided.
async fn load_operation(
    connection: &mut PgConnection,
    occurrence: BankOperationOccurrence,
    binding: &[u8],
) -> Result<Option<BankOperationRecord>> {
    let row = sqlx::query(
        "SELECT transaction_id::text, event_id::text, kind, request_binding, amount, outcome \
           FROM game_account_bank_operations \
          WHERE operation_occurrence_id = encode($1,'hex')::uuid",
    )
    .bind(occurrence.as_bytes().as_slice())
    .fetch_optional(&mut *connection)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    if row.try_get::<Vec<u8>, _>("request_binding")? != binding {
        return Err(BankError::ConflictingOccurrence);
    }
    let transaction_id = uuid_text(&row.try_get::<String, _>("transaction_id")?)?;
    let invalid = || BankError::from(DurabilityError::InvalidStoredState);
    let entries = sqlx::query(
        "SELECT entry_id::text, account_id::text, kind, amount, balance_before, balance_after \
           FROM game_account_bank_entries WHERE transaction_id = encode($1,'hex')::uuid \
          ORDER BY kind",
    )
    .bind(transaction_id.as_slice())
    .fetch_all(&mut *connection)
    .await?
    .iter()
    .map(|row| {
        let amount = |column: &str| -> Result<u64> {
            u64::try_from(row.try_get::<i64, _>(column)?).map_err(|_| invalid())
        };
        Ok(BankLedgerEntry {
            entry_id: uuid_text(&row.try_get::<String, _>("entry_id")?)?,
            account_id: uuid_text(&row.try_get::<String, _>("account_id")?)?,
            kind: BankLedgerKind::from_code(row.try_get("kind")?).ok_or_else(invalid)?,
            amount: amount("amount")?,
            balance_before: amount("balance_before")?,
            balance_after: amount("balance_after")?,
        })
    })
    .collect::<Result<Vec<_>>>()?;
    let coin_lines = sqlx::query(
        "SELECT direction, item_instance_id::text, coin_worth, placement_ordinal::text, \
                quantity_before, quantity_after \
           FROM game_account_bank_coin_lines WHERE transaction_id = encode($1,'hex')::uuid \
          ORDER BY line_ordinal",
    )
    .bind(transaction_id.as_slice())
    .fetch_all(&mut *connection)
    .await?
    .iter()
    .map(|row| {
        let quantity = |column: &str| -> Result<u32> {
            u32::try_from(row.try_get::<i64, _>(column)?).map_err(|_| invalid())
        };
        let worth = u64::try_from(row.try_get::<i64, _>("coin_worth")?).map_err(|_| invalid())?;
        Ok(BankCoinLine {
            direction: match row.try_get::<i16, _>("direction")? {
                1 => BankCoinDirection::Input,
                2 => BankCoinDirection::Output,
                _ => return Err(invalid()),
            },
            item_instance_id: uuid_text(&row.try_get::<String, _>("item_instance_id")?)?,
            coin: Coin::ALL
                .into_iter()
                .find(|coin| coin.worth() == worth)
                .ok_or_else(invalid)?,
            placement_ordinal: numeric_u64(row, "placement_ordinal")?,
            quantity_before: quantity("quantity_before")?,
            quantity_after: quantity("quantity_after")?,
        })
    })
    .collect::<Result<Vec<_>>>()?;
    Ok(Some(BankOperationRecord {
        occurrence,
        transaction_id,
        event_id: uuid_text(&row.try_get::<String, _>("event_id")?)?,
        kind: BankOperationKind::from_code(row.try_get("kind")?).ok_or_else(invalid)?,
        amount: u64::try_from(row.try_get::<i64, _>("amount")?).map_err(|_| invalid())?,
        result: BankResult::from_code(row.try_get("outcome")?).ok_or_else(invalid)?,
        entries,
        coin_lines,
    }))
}

/// Keeps the entry points and their types linked in every target that path-loads this module
/// (BANK-NPC-1 is their first production caller).
#[cfg(test)]
mod linkage {
    use super::*;

    #[test]
    fn bank_api_is_linked() {
        let _ = std::mem::size_of::<BankOperationOutcome>();
        let _ = std::mem::size_of::<BankOperationRecord>();
        let _ = std::mem::size_of::<BankLedgerEntry>();
        let _ = std::mem::size_of::<BankCoinLine>();
        let _ = std::mem::size_of::<BankError>();
        let _ = BankCoinDirection::Input;
        let _ = (
            BankError::AuthorityRejected,
            BankError::CapacityExceeded,
            BankError::ConflictingOccurrence,
        );
        let _ = DurabilityRoot::bank_deposit;
        let _ = DurabilityRoot::bank_withdraw;
        let _ = DurabilityRoot::bank_transfer;
        let _ = bank_operation_in_transaction;
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;
    use crate::durability::item_transfer::ItemStackClass;
    use crate::foundation::{ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration};

    fn id(seed: u8) -> [u8; 16] {
        [
            seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
        ]
    }

    fn fence() -> CurrentCharacterItemFence {
        CurrentCharacterItemFence {
            character_id: CharacterId::from_bytes(id(1)).unwrap(),
            game_session_id: GameSessionId::decode(&id(2)).unwrap(),
            connection_generation: ConnectionGeneration::new(1).unwrap(),
            character_lease_generation: 1,
            runtime_scope: RuntimeScopeRefV1::channel(
                WorldId::decode(&id(3)).unwrap(),
                ChannelId::decode(&id(4)).unwrap(),
            ),
            scope_ownership_generation: ScopeOwnershipGeneration::new(1).unwrap(),
        }
    }

    fn identity() -> BankOperationIdentity {
        BankOperationIdentity {
            occurrence: BankOperationOccurrence::from_bytes(id(5)).unwrap(),
            transaction_id: id(6),
            event_id: id(7),
            occurred_at_unix_ms: 1,
            server_build_id: "build".into(),
        }
    }

    fn definition(key: &str) -> TypedDefinitionRef {
        TypedDefinitionRef {
            family: COIN_DEFINITION_FAMILY.into(),
            production_key: key.into(),
            revision_ref: "rev-1".into(),
        }
    }

    fn coins() -> BankCoinFacts {
        BankCoinFacts {
            gold: definition(Coin::Gold.production_key()),
            platinum: definition(Coin::Platinum.production_key()),
            crystal: definition(Coin::Crystal.production_key()),
            backpack: ItemDefinitionFacts {
                definition: definition("oteryn:item.tibia.i2854"),
                stack: ItemStackClass::NonStackable,
                container_capacity: Some(20),
                container_slot_equip_pattern: true,
            },
        }
    }

    fn deposit(amount: u64) -> BankRequest {
        BankRequest::Deposit(BankDepositRequest {
            identity: identity(),
            amount,
            coins: coins(),
            change_item_instance_ids: [id(11), id(12)],
        })
    }

    fn withdraw(amount: u64) -> BankRequest {
        BankRequest::Withdraw(BankWithdrawRequest {
            identity: identity(),
            amount,
            coins: coins(),
            output_item_instance_ids: [id(10), id(11), id(12)],
        })
    }

    fn transfer(amount: u64, key: &str) -> BankRequest {
        BankRequest::Transfer(BankTransferRequest {
            identity: identity(),
            amount,
            recipient_character_id: CharacterId::from_bytes(id(9)).unwrap(),
            recipient_name_key: key.into(),
        })
    }

    #[test]
    fn junior_is_a_function_of_the_departure_fact() {
        // No starter island exists yet: nobody is junior, so no bank use is refused.
        assert!(!is_junior(starter_island_fact(fence().character_id)));
        assert!(!is_junior(StarterIslandFact::Departed));
        // Once the fact says "on the island" the bank refuses (owner answer Q1 b).
        assert!(is_junior(StarterIslandFact::OnIsland));
        use StarterIslandFact::{Departed, NoStarterIsland, OnIsland};
        assert_eq!(
            transfer_party_refusal(OnIsland, Departed),
            Some(BankResult::JuniorAccount)
        );
        assert_eq!(
            transfer_party_refusal(OnIsland, OnIsland),
            Some(BankResult::JuniorAccount)
        );
        assert_eq!(
            transfer_party_refusal(Departed, OnIsland),
            Some(BankResult::RecipientCannotReceiveTransfers)
        );
        assert_eq!(transfer_party_refusal(NoStarterIsland, Departed), None);
    }

    #[test]
    fn amounts_are_bounded_by_their_rows_before_any_read() {
        let fence = fence();
        for (request, valid) in [
            (deposit(BANK_DEPOSIT_MAX), true),
            (deposit(BANK_DEPOSIT_MAX + 1), false),
            (deposit(0), false),
            (withdraw(BANK_WITHDRAW_MAX), true),
            (withdraw(BANK_WITHDRAW_MAX + 1), false),
            (transfer(BANK_TRANSFER_MAX, "bankfriend"), true),
            (transfer(BANK_TRANSFER_MAX + 1, "bankfriend"), false),
            (transfer(1, "Bank Friend"), false),
            (transfer(1, "b"), false),
        ] {
            assert_eq!(
                validate_request(&fence, &request).is_ok(),
                valid,
                "{request:?}"
            );
        }
        // Identities and slots are distinct UUIDv7 values; the coin facts name the three coins.
        let BankRequest::Withdraw(mut repeated) = withdraw(1) else {
            unreachable!()
        };
        repeated.output_item_instance_ids[2] = repeated.identity.transaction_id;
        assert!(validate_request(&fence, &BankRequest::Withdraw(repeated)).is_err());
        let BankRequest::Deposit(mut swapped) = deposit(1) else {
            unreachable!()
        };
        swapped.coins.gold = definition(Coin::Platinum.production_key());
        assert!(validate_request(&fence, &BankRequest::Deposit(swapped)).is_err());
        assert!(BankOperationOccurrence::from_bytes([0; 16]).is_err());
    }

    #[test]
    fn binding_covers_the_whole_request_but_not_retry_identity() {
        let fence = fence();
        let base = request_binding(&fence, &deposit(100));
        let BankRequest::Deposit(mut retried) = deposit(100) else {
            unreachable!()
        };
        retried.identity.transaction_id = id(20);
        retried.identity.event_id = id(21);
        retried.identity.occurred_at_unix_ms = 2;
        retried.identity.server_build_id = "other".into();
        retried.coins.gold.revision_ref = "rev-2".into();
        assert_eq!(
            request_binding(&fence, &BankRequest::Deposit(retried)),
            base
        );
        let BankRequest::Deposit(mut slots) = deposit(100) else {
            unreachable!()
        };
        slots.change_item_instance_ids = [id(12), id(11)];
        for other in [deposit(101), withdraw(100), BankRequest::Deposit(slots)] {
            assert_ne!(request_binding(&fence, &other), base);
        }
        let mut moved = fence;
        moved.character_id = CharacterId::from_bytes(id(30)).unwrap();
        assert_ne!(request_binding(&moved, &deposit(100)), base);
        assert_ne!(
            request_binding(&fence, &transfer(5, "bankfriend")),
            request_binding(&fence, &transfer(5, "bankfoe"))
        );
    }

    #[test]
    fn a_withdrawal_is_the_canonical_split_in_consecutive_entries() {
        let lines = outputs(withdraw_split(1_009_999), [id(10), id(11), id(12)], 7).unwrap();
        assert_eq!(
            lines
                .iter()
                .map(|l| (
                    l.item_instance_id,
                    l.coin,
                    l.quantity_after,
                    l.placement_ordinal
                ))
                .collect::<Vec<_>>(),
            vec![
                (id(10), Coin::Crystal, 100, 7),
                (id(11), Coin::Platinum, 99, 8),
                (id(12), Coin::Gold, 99, 9),
            ]
        );
        // Only the needed slots are used.
        let lines = outputs(withdraw_split(10_005), [id(10), id(11), id(12)], 1).unwrap();
        assert_eq!(
            lines
                .iter()
                .map(|l| (l.item_instance_id, l.placement_ordinal))
                .collect::<Vec<_>>(),
            vec![(id(10), 1), (id(12), 2)]
        );
        // No ordinal past u64::MAX.
        assert!(outputs(withdraw_split(101), [id(10), id(11), id(12)], u64::MAX).is_none());
        assert!(outputs(withdraw_split(101), [id(10), id(11), id(12)], 0).is_none());
    }

    #[test]
    fn stored_codes_round_trip() {
        for code in 0..=8 {
            assert_eq!(
                BankResult::from_code(code).map(BankResult::code),
                Some(code)
            );
        }
        assert_eq!(BankResult::from_code(9), None);
        for code in 1..=3 {
            assert_eq!(
                BankOperationKind::from_code(code).map(BankOperationKind::code),
                Some(code)
            );
        }
        for code in 1..=4_i16 {
            assert_eq!(
                BankLedgerKind::from_code(code).map(|kind| i16::try_from(kind.code()).unwrap()),
                Some(code)
            );
        }
    }
}
