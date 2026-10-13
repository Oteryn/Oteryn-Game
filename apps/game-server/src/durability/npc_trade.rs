//! NPC BUY and SELL commit (NPC-TRADE-1, migration 0082; NPC-0 §5,
//! `reviews/OTERYN_GAME_NPC0_NPC_RUNTIME_SERVICE_DECISION_2026-09-30.md`; boundary gate
//! `NPC_SINGLE_TRADE_COMMIT_V1`).
//!
//! One trade is one item-only transaction of one physical PostgreSQL transaction, without a
//! CharacterRevision advance (§5.2): it reads nothing of the Character root but its lock in the
//! complete Character item fence, and writes its cause record, its lines, the item changes and one
//! audit event.
//! - BUY (§5.3): the price `unit_price x quantity` is paid by the deterministic coin fee plan of
//!   GOLD-FEE-1a over the direct entries of the equipped main backpack (the BURN lines realize
//!   `FeeBurnCause::NpcTrade` under the trade's own record; there is no bank part), the change is
//!   minted as platinum then gold, and the bought item is minted as one fresh stack. Every MINT
//!   takes a new direct entry after the highest ordinal the backpack held.
//! - SELL (§5.4): one BURN of the first live direct entry in display order holding the offer's
//!   definition at its current revision with enough units, no contents and its default state,
//!   then the coin MINT of the price: crystal, platinum, gold, each only when positive, each at
//!   most 100. Free entries are counted after the burn.
//!
//! The cause record is keyed by (occurrence, Character) forever: a replay with the same binding
//! returns the first outcome without reacquiring session authority, a changed binding conflicts.
//! Any refusal writes nothing. The caller supplies the offer from the server catalogue at the
//! bound revision; a charged or fluid definition is deferred by §5.4 and is refused by the caller
//! before this commit (the database refuses a sold item with a timed state or contents).

use super::character_authority::{
    ReconciledCharacterAuthority, SERVER_BUILD_ID, assert_recovery_fence,
};
use super::character_progression::numeric_u64;
use super::db::{begin_type2_transaction, lock_admission_relations};
use super::item_mint::{TypedDefinitionRef, uuid_text};
use super::item_mint_audit::{
    self as mint_audit, AuditError, ITEM_LIFECYCLE_LIVE, OneItemStateV1,
    OneItemTypedDefinitionRevisionV1, Type2EventTuple, check_content_key, check_uuid_v7,
};
use super::item_transfer::{
    CurrentCharacterItemFence, GAMEITEM01_CONTAINER_ENTRIES_MAX, ItemDefinitionFacts,
    character_item_fence_is_current, scope_of, stack_maximum,
};
use super::item_transfer_audit::{ITEM_LIFECYCLE_RETIRED, OneItemCommandRefV1};
use super::npc_trade_audit::{
    LINE_COIN_BURN, LINE_COIN_MINT, LINE_ITEM_BURN, LINE_ITEM_MINT, NPC_TRADE_BUY_GOLD_UNITS_MAX,
    NPC_TRADE_QUANTITY_MAX, NPC_TRADE_SELL_GOLD_UNITS_MAX, NpcTradeEventIdentity,
    OneItemNpcTradeCauseV1, OneItemNpcTradeLineV1, OneItemNpcTradeV1, SIDE_BUY, SIDE_SELL,
    buy_change_coins, encode_npc_trade_event, sell_coins,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterId;
use crate::domain::currency::{
    BACKPACK_ENTRIES_MAX, COIN_DEFINITION_FAMILY, Coin, CoinStack, FeePlanError, plan_fee_within,
};
use crate::foundation::{CommandRef, WorldId};
use sha2::{Digest, Sha256};
use sqlx::Row;

type Result<T> = std::result::Result<T, NpcTradeError>;
const REQUEST_BINDING_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcTradeSide {
    Buy,
    Sell,
}

impl NpcTradeSide {
    const fn code(self) -> u32 {
        match self {
            Self::Buy => SIDE_BUY,
            Self::Sell => SIDE_SELL,
        }
    }
}

/// `NpcTradeCause {npc, offer, side, occurrence}` (§5.5): the NPC's catalogue key, the offer's
/// index at the bound catalogue revision, the side and the command 8 occurrence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcTradeCause {
    pub npc_key: String,
    pub catalogue_revision: u64,
    pub offer_index: u32,
    pub side: NpcTradeSide,
    pub occurrence: [u8; 16],
}

/// The three coin definitions at their current compatible revisions: every live coin stack a BUY
/// pays from must be at its coin's revision, and change and proceeds are minted at them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcTradeCoins {
    pub gold: TypedDefinitionRef,
    pub platinum: TypedDefinitionRef,
    pub crystal: TypedDefinitionRef,
}

impl NpcTradeCoins {
    fn of(&self, coin: Coin) -> &TypedDefinitionRef {
        match coin {
            Coin::Gold => &self.gold,
            Coin::Platinum => &self.platinum,
            Coin::Crystal => &self.crystal,
        }
    }
}

/// One trade. The TransactionId, EventId, time and output identities are fixed before the first
/// attempt and reused on every retry of the same occurrence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcTradeRequest {
    pub cause: NpcTradeCause,
    /// The command 8 the occurrence is bound to (one occurrence per CommandRef).
    pub command: CommandRef,
    /// The intent's quantity (1..=100) and the server's unit price at the bound revision.
    pub quantity: u32,
    pub unit_price: u64,
    /// The offer's count per unit: a stackable offer trades `quantity x count` units in one stack.
    pub offer_count: u32,
    /// The offer's item definition at its current revision; never a coin.
    pub item: ItemDefinitionFacts,
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    /// BUY: platinum change, gold change, bought item. SELL: crystal, platinum, gold. A slot the
    /// trade does not need stays unused.
    pub output_item_instance_ids: [[u8; 16]; 3],
    pub coins: NpcTradeCoins,
    /// The equipped main backpack's definition: its declared capacity bounds the outputs.
    pub backpack: ItemDefinitionFacts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcTradeLineKind {
    CoinBurn,
    CoinMint,
    ItemBurn,
    ItemMint,
}

impl NpcTradeLineKind {
    const fn code(self) -> u32 {
        match self {
            Self::CoinBurn => LINE_COIN_BURN,
            Self::CoinMint => LINE_COIN_MINT,
            Self::ItemBurn => LINE_ITEM_BURN,
            Self::ItemMint => LINE_ITEM_MINT,
        }
    }

    fn of(code: i16) -> Option<Self> {
        [
            Self::CoinBurn,
            Self::CoinMint,
            Self::ItemBurn,
            Self::ItemMint,
        ]
        .into_iter()
        .find(|kind| i64::from(kind.code()) == i64::from(code))
    }

    const fn is_burn(self) -> bool {
        matches!(self, Self::CoinBurn | Self::ItemBurn)
    }
}

/// One line: a BURN keeps its entry while units are left; a MINT is a fresh stack in a new entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcTradeLine {
    pub kind: NpcTradeLineKind,
    pub item_instance_id: [u8; 16],
    pub placement_ordinal: u64,
    /// The coin of a coin line.
    pub coin: Option<Coin>,
    /// 0 for a MINT.
    pub quantity_before: u32,
    /// 0 for a whole BURN.
    pub quantity_after: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedNpcTrade {
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub character_id: CharacterId,
    pub side: NpcTradeSide,
    pub total_gold_units: u64,
    pub burned_gold_units: u64,
    pub minted_gold_units: u64,
    /// In order: every BURN line, then every MINT line.
    pub lines: Vec<NpcTradeLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NpcTradeOutcome {
    Committed(CommittedNpcTrade),
    /// The occurrence already committed with the same binding: its first outcome.
    AlreadyCommitted(CommittedNpcTrade),
}

#[derive(Debug)]
pub enum NpcTradeError {
    InvalidInput,
    /// The complete Character item fence is not current.
    AuthorityRejected,
    /// No equipped main backpack.
    NoBackpack,
    /// BUY: the eligible coins do not cover the price.
    InsufficientFunds,
    /// BUY: more than 20 coin stacks would be burned.
    CapacityExceeded,
    /// The outputs do not fit the backpack's free entries after the burn.
    DoesNotFit,
    /// SELL: no direct entry holds the offer's item as §5.4 requires.
    ItemNotFound,
    /// The occurrence committed with another binding.
    ConflictingOccurrence,
    /// The CommandRef or a retry-local identity is already used by another trade.
    ConflictingCandidate,
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for NpcTradeError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl From<sqlx::Error> for NpcTradeError {
    fn from(error: sqlx::Error) -> Self {
        Self::Unavailable(error.into())
    }
}

impl From<AuditError> for NpcTradeError {
    fn from(error: AuditError) -> Self {
        match error {
            AuditError::CapacityExceeded => Self::CapacityExceeded,
            _ => Self::InvalidInput,
        }
    }
}

impl std::fmt::Display for NpcTradeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => formatter.write_str("invalid NPC trade input"),
            Self::AuthorityRejected => formatter.write_str("NPC trade authority is not current"),
            Self::NoBackpack => formatter.write_str("NPC trade needs an equipped backpack"),
            Self::InsufficientFunds => formatter.write_str("insufficient coins for the NPC trade"),
            Self::CapacityExceeded => formatter.write_str("NPC trade needs too many coin stacks"),
            Self::DoesNotFit => formatter.write_str("NPC trade outputs do not fit the backpack"),
            Self::ItemNotFound => formatter.write_str("no item to sell"),
            Self::ConflictingOccurrence => {
                formatter.write_str("NPC trade occurrence was reused with a different binding")
            }
            Self::ConflictingCandidate => {
                formatter.write_str("NPC trade identity is already used by another trade")
            }
            Self::Unavailable(error) => write!(formatter, "NPC trade storage failed: {error:?}"),
        }
    }
}

impl std::error::Error for NpcTradeError {}

/// The units one trade moves (§5.3/§5.4): a stackable offer `quantity x count` within the stack
/// maximum; a non-stackable offer exactly one item.
fn item_units(request: &NpcTradeRequest) -> Result<u32> {
    let (maximum, stackable) =
        stack_maximum(request.item.stack).map_err(|_| NpcTradeError::InvalidInput)?;
    let units = request
        .quantity
        .checked_mul(request.offer_count)
        .ok_or(NpcTradeError::InvalidInput)?;
    if (!stackable && units != 1) || !(1..=maximum).contains(&units) {
        return Err(NpcTradeError::InvalidInput);
    }
    Ok(units)
}

fn is_coin(definition: &TypedDefinitionRef) -> bool {
    definition.family == COIN_DEFINITION_FAMILY
        && Coin::from_production_key(&definition.production_key).is_some()
}

/// Closed request shape: identities, price bounds, the coin definitions, a non-coin item and a
/// backpack declaring a capacity within `GAMEITEM01-CONTAINER-ENTRIES-MAX`.
fn validate(request: &NpcTradeRequest) -> Result<(u64, u32)> {
    let cause = &request.cause;
    let total = request
        .unit_price
        .checked_mul(u64::from(request.quantity))
        .ok_or(NpcTradeError::InvalidInput)?;
    let total_max = match cause.side {
        NpcTradeSide::Buy => NPC_TRADE_BUY_GOLD_UNITS_MAX,
        NpcTradeSide::Sell => NPC_TRADE_SELL_GOLD_UNITS_MAX,
    };
    let ids = [request.transaction_id, request.event_id]
        .into_iter()
        .chain(request.output_item_instance_ids)
        .collect::<Vec<_>>();
    let distinct = ids
        .iter()
        .enumerate()
        .all(|(index, id)| !ids[..index].contains(id));
    let coin = |definition: &TypedDefinitionRef, coin: Coin| {
        definition.family == COIN_DEFINITION_FAMILY
            && definition.production_key == coin.production_key()
            && check_content_key(&definition.revision_ref).is_ok()
    };
    let definition = &request.item.definition;
    if check_content_key(&cause.npc_key).is_err()
        || check_uuid_v7(&cause.occurrence).is_err()
        || ids.iter().any(|id| check_uuid_v7(id).is_err())
        || !distinct
        || !(1..=NPC_TRADE_QUANTITY_MAX).contains(&request.quantity)
        || request.unit_price == 0
        || total > total_max
        || request.occurred_at_unix_ms <= 0
        || !coin(&request.coins.gold, Coin::Gold)
        || !coin(&request.coins.platinum, Coin::Platinum)
        || !coin(&request.coins.crystal, Coin::Crystal)
        || is_coin(definition)
        || definition.family.is_empty()
        || definition.family.len() > 128
        || check_content_key(&definition.production_key).is_err()
        || check_content_key(&definition.revision_ref).is_err()
        || !request
            .backpack
            .container_capacity
            .is_some_and(|capacity| (1..=GAMEITEM01_CONTAINER_ENTRIES_MAX).contains(&capacity))
    {
        return Err(NpcTradeError::InvalidInput);
    }
    Ok((total, item_units(request)?))
}

/// SHA-256 over the semantic request: the cause, the CommandRef, the Character, its World, the
/// quantity, the price and the offer's item. The TransactionId, EventId, time and output
/// identities are retry-local and excluded.
fn request_binding(
    fence: &CurrentCharacterItemFence,
    world_id: WorldId,
    request: &NpcTradeRequest,
) -> Vec<u8> {
    let text = |digest: &mut Sha256, value: &str| {
        digest.update(u32::try_from(value.len()).unwrap_or(u32::MAX).to_be_bytes());
        digest.update(value.as_bytes());
    };
    let cause = &request.cause;
    let mut digest = Sha256::new();
    digest.update([REQUEST_BINDING_VERSION]);
    text(&mut digest, &cause.npc_key);
    digest.update(cause.catalogue_revision.to_be_bytes());
    digest.update(cause.offer_index.to_be_bytes());
    digest.update(cause.side.code().to_be_bytes());
    digest.update(cause.occurrence);
    digest.update(request.command.game_session_id().as_bytes());
    digest.update(request.command.command_id().get().to_be_bytes());
    digest.update(fence.character_id.as_bytes());
    digest.update(world_id.as_bytes());
    digest.update(request.quantity.to_be_bytes());
    digest.update(request.unit_price.to_be_bytes());
    digest.update(request.offer_count.to_be_bytes());
    let item = &request.item.definition;
    text(&mut digest, &item.family);
    text(&mut digest, &item.production_key);
    text(&mut digest, &item.revision_ref);
    digest.finalize().to_vec()
}

/// One locked direct entry of the backpack, highest ordinal first.
#[derive(Clone)]
struct Entry {
    item_instance_id: [u8; 16],
    placement_ordinal: u64,
    definition: TypedDefinitionRef,
    quantity: u32,
    live: bool,
    /// Holds entries of its own or a timed state.
    not_default: bool,
}

/// The trade materialized from the locked before-state.
struct Plan {
    lines: Vec<NpcTradeLine>,
    burned_gold_units: u64,
    minted_gold_units: u64,
    /// The bought or sold item.
    item_instance_id: [u8; 16],
}

/// Lines of a BUY: the fee plan's BURN lines, the change MINT and the item MINT.
fn plan_buy(
    request: &NpcTradeRequest,
    total: u64,
    units: u32,
    capacity: usize,
    entries: &[Entry],
) -> Result<Plan> {
    let mut inputs = Vec::new();
    for entry in entries.iter().filter(|entry| entry.live) {
        let Some(coin) = Coin::from_production_key(&entry.definition.production_key) else {
            continue;
        };
        if entry.definition.family != COIN_DEFINITION_FAMILY {
            continue;
        }
        // An input is at the compatible revision; a stack at another one is not skipped (the
        // database plan guard counts every coin stack).
        if entry.definition.revision_ref != request.coins.of(coin).revision_ref {
            return Err(NpcTradeError::InvalidInput);
        }
        inputs.push((
            CoinStack {
                coin,
                quantity: entry.quantity,
                placement_ordinal: entry.placement_ordinal,
            },
            entry.item_instance_id,
        ));
    }
    let stacks: Vec<CoinStack> = inputs.iter().map(|(stack, _)| *stack).collect();
    let plan = match plan_fee_within(total, &stacks, entries.len(), BACKPACK_ENTRIES_MAX) {
        Ok(plan) => plan,
        Err(FeePlanError::InsufficientFunds) => return Err(NpcTradeError::InsufficientFunds),
        Err(FeePlanError::CapacityExceeded) => return Err(NpcTradeError::CapacityExceeded),
        Err(FeePlanError::ChangeDoesNotFit) => return Err(NpcTradeError::DoesNotFit),
        Err(FeePlanError::InvalidInput) => return Err(DurabilityError::InvalidStoredState.into()),
    };
    let mut lines: Vec<NpcTradeLine> = plan
        .lines
        .iter()
        .map(|line| {
            let (stack, item) = &inputs[line.input];
            NpcTradeLine {
                kind: NpcTradeLineKind::CoinBurn,
                item_instance_id: *item,
                placement_ordinal: stack.placement_ordinal,
                coin: Some(stack.coin),
                quantity_before: stack.quantity,
                quantity_after: stack.quantity - line.burned,
            }
        })
        .collect();
    let whole = lines.iter().filter(|line| line.quantity_after == 0).count();
    let change = buy_change_coins(plan.change);
    // §5.3: the change and the item each take a new entry after the burn.
    if entries.len() - whole + change.len() + 1 > capacity {
        return Err(NpcTradeError::DoesNotFit);
    }
    let [platinum, gold, item] = request.output_item_instance_ids;
    let mut ordinal = first_mint_ordinal(entries)?;
    for (coin, quantity) in change {
        lines.push(NpcTradeLine {
            kind: NpcTradeLineKind::CoinMint,
            item_instance_id: if coin == Coin::Platinum {
                platinum
            } else {
                gold
            },
            placement_ordinal: ordinal,
            coin: Some(coin),
            quantity_before: 0,
            quantity_after: quantity,
        });
        ordinal = ordinal.checked_add(1).ok_or(NpcTradeError::DoesNotFit)?;
    }
    lines.push(NpcTradeLine {
        kind: NpcTradeLineKind::ItemMint,
        item_instance_id: item,
        placement_ordinal: ordinal,
        coin: None,
        quantity_before: 0,
        quantity_after: units,
    });
    Ok(Plan {
        lines,
        burned_gold_units: total + plan.change,
        minted_gold_units: plan.change,
        item_instance_id: item,
    })
}

/// Lines of a SELL: the item BURN and the coin MINT of the price.
fn plan_sell(
    request: &NpcTradeRequest,
    total: u64,
    units: u32,
    capacity: usize,
    entries: &[Entry],
) -> Result<Plan> {
    let sold = entries
        .iter()
        .find(|entry| {
            entry.live
                && entry.definition == request.item.definition
                && entry.quantity >= units
                && !entry.not_default
        })
        .ok_or(NpcTradeError::ItemNotFound)?;
    let coins = sell_coins(total).ok_or(NpcTradeError::InvalidInput)?;
    let whole = sold.quantity == units;
    if entries.len() - usize::from(whole) + coins.len() > capacity {
        return Err(NpcTradeError::DoesNotFit);
    }
    let mut lines = vec![NpcTradeLine {
        kind: NpcTradeLineKind::ItemBurn,
        item_instance_id: sold.item_instance_id,
        placement_ordinal: sold.placement_ordinal,
        coin: None,
        quantity_before: sold.quantity,
        quantity_after: sold.quantity - units,
    }];
    let [crystal, platinum, gold] = request.output_item_instance_ids;
    let mut ordinal = first_mint_ordinal(entries)?;
    for (coin, quantity) in coins {
        lines.push(NpcTradeLine {
            kind: NpcTradeLineKind::CoinMint,
            item_instance_id: match coin {
                Coin::Crystal => crystal,
                Coin::Platinum => platinum,
                Coin::Gold => gold,
            },
            placement_ordinal: ordinal,
            coin: Some(coin),
            quantity_before: 0,
            quantity_after: quantity,
        });
        ordinal = ordinal.checked_add(1).ok_or(NpcTradeError::DoesNotFit)?;
    }
    Ok(Plan {
        lines,
        burned_gold_units: 0,
        minted_gold_units: total,
        item_instance_id: sold.item_instance_id,
    })
}

/// After the highest ordinal the backpack held before the trade (entries are highest first).
fn first_mint_ordinal(entries: &[Entry]) -> Result<u64> {
    entries
        .first()
        .map_or(0, |entry| entry.placement_ordinal)
        .checked_add(1)
        .ok_or(NpcTradeError::DoesNotFit)
}

impl DurabilityRoot {
    /// Commit one NPC trade under the complete current Character item fence (see the module
    /// documentation). An occurrence that already committed returns its first outcome under the
    /// current recovery fence; a changed binding conflicts. Any error writes nothing.
    pub async fn commit_npc_trade(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: NpcTradeRequest,
    ) -> Result<NpcTradeOutcome> {
        let (world_id, channel_id) =
            scope_of(&fence).map_err(|_| NpcTradeError::AuthorityRejected)?;
        let (total, units) = validate(&request)?;
        let binding = request_binding(&fence, world_id, &request);
        let recovery = authority
            .record_for(self)
            .map_err(|_| NpcTradeError::AuthorityRejected)?;
        let node = node.clone();

        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let occurrence = request.cause.occurrence;
                    sqlx::query(
                        "SELECT pg_advisory_xact_lock(hashtextextended(\
                           'oteryn:npc-trade:' || encode($1,'hex'), 0))",
                    )
                    .bind(occurrence.as_slice())
                    .execute(&mut **tx)
                    .await?;

                    if let Some(committed) =
                        load_trade(&mut tx, &occurrence, fence.character_id, &binding).await?
                    {
                        let committed = match committed {
                            Ok(committed) => committed,
                            Err(error) => return Ok(Err(error)),
                        };
                        tx.commit(deadline).await?;
                        return Ok(Ok(NpcTradeOutcome::AlreadyCommitted(committed)));
                    }
                    let [first, second, third] = request.output_item_instance_ids;
                    let identity_reused: bool = sqlx::query_scalar(
                        "SELECT EXISTS (SELECT 1 FROM game_npc_trades \
                                         WHERE transaction_id = encode($1,'hex')::uuid \
                                            OR event_id = encode($2,'hex')::uuid \
                                            OR (game_session_id = encode($3,'hex')::uuid \
                                                AND command_id = $4::text::numeric(20,0))) \
                             OR EXISTS (SELECT 1 FROM game_item_audit_outbox \
                                         WHERE transaction_id = encode($1,'hex')::uuid \
                                            OR event_id = encode($2,'hex')::uuid) \
                             OR EXISTS (SELECT 1 FROM game_item_instances \
                                         WHERE item_instance_id IN (encode($5,'hex')::uuid, \
                                                 encode($6,'hex')::uuid, encode($7,'hex')::uuid) \
                                            OR minted_transaction_id = encode($1,'hex')::uuid)",
                    )
                    .bind(request.transaction_id.as_slice())
                    .bind(request.event_id.as_slice())
                    .bind(request.command.game_session_id().as_bytes().as_slice())
                    .bind(request.command.command_id().get().to_string())
                    .bind(first.as_slice())
                    .bind(second.as_slice())
                    .bind(third.as_slice())
                    .fetch_one(&mut **tx)
                    .await?;
                    if identity_reused {
                        return Ok(Err(NpcTradeError::ConflictingCandidate));
                    }
                    if !character_item_fence_is_current(
                        &mut tx,
                        &node,
                        &fence,
                        request.command,
                        fence.character_id,
                        *world_id.as_bytes(),
                        *channel_id.as_bytes(),
                    )
                    .await?
                    {
                        return Ok(Err(NpcTradeError::AuthorityRejected));
                    }

                    let backpack = sqlx::query(
                        "SELECT i.item_instance_id::text, i.definition_family, \
                                i.definition_production_key, i.definition_revision_ref \
                           FROM game_item_container_slots s \
                           JOIN game_item_instances i ON i.item_instance_id = s.item_instance_id \
                          WHERE s.character_id = encode($1,'hex')::uuid",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .fetch_optional(&mut **tx)
                    .await?;
                    let Some(backpack) = backpack else {
                        return Ok(Err(NpcTradeError::NoBackpack));
                    };
                    let declared = &request.backpack.definition;
                    if backpack.try_get::<String, _>("definition_family")? != declared.family
                        || backpack.try_get::<String, _>("definition_production_key")?
                            != declared.production_key
                        || backpack.try_get::<String, _>("definition_revision_ref")?
                            != declared.revision_ref
                    {
                        return Ok(Err(NpcTradeError::InvalidInput));
                    }
                    let backpack = uuid_text(&backpack.try_get::<String, _>("item_instance_id")?)?;
                    let capacity = request
                        .backpack
                        .container_capacity
                        .and_then(|capacity| usize::try_from(capacity).ok())
                        .unwrap_or(0);
                    let entries = lock_entries(&mut tx, &backpack, world_id).await?;
                    let plan = match request.cause.side {
                        NpcTradeSide::Buy => plan_buy(&request, total, units, capacity, &entries),
                        NpcTradeSide::Sell => plan_sell(&request, total, units, capacity, &entries),
                    };
                    let plan = match plan {
                        Ok(plan) => plan,
                        Err(NpcTradeError::Unavailable(error)) => return Err(error),
                        Err(error) => return Ok(Err(error)),
                    };

                    let tuple = tx.tuple().tuple();
                    let message = trade_message(
                        &fence, world_id, channel_id, &backpack, &request, total, &plan,
                    );
                    let envelope = match encode_npc_trade_event(
                        NpcTradeEventIdentity {
                            event_id: request.event_id,
                            transaction_id: request.transaction_id,
                            occurred_at_unix_ms: request.occurred_at_unix_ms,
                            server_build_id: SERVER_BUILD_ID,
                        },
                        message,
                        tuple,
                    ) {
                        Ok(envelope) => envelope,
                        Err(error) => return Ok(Err(error.into())),
                    };
                    apply_trade(
                        &mut tx,
                        &fence,
                        (world_id, channel_id),
                        &backpack,
                        &request,
                        (total, units),
                        &binding,
                        &plan,
                        &envelope,
                        tuple,
                    )
                    .await?;
                    tx.commit(deadline).await?;
                    Ok(Ok(NpcTradeOutcome::Committed(CommittedNpcTrade {
                        transaction_id: request.transaction_id,
                        event_id: request.event_id,
                        character_id: fence.character_id,
                        side: request.cause.side,
                        total_gold_units: total,
                        burned_gold_units: plan.burned_gold_units,
                        minted_gold_units: plan.minted_gold_units,
                        lines: plan.lines,
                    })))
                })
            })
            .await?
    }
}

/// The backpack's direct entries, highest ordinal first, each item row locked.
async fn lock_entries(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    backpack: &[u8; 16],
    world_id: WorldId,
) -> std::result::Result<Vec<Entry>, DurabilityError> {
    let rows = sqlx::query(
        "SELECT i.item_instance_id::text, e.placement_ordinal::text, i.world_id::text, \
                i.definition_family, i.definition_production_key, i.definition_revision_ref, \
                i.quantity, i.lifecycle, \
                EXISTS (SELECT 1 FROM game_item_container_entries c \
                         WHERE c.parent_item_instance_id = i.item_instance_id) \
                OR EXISTS (SELECT 1 FROM game_item_timed_states t \
                            WHERE t.item_instance_id = i.item_instance_id) AS not_default \
           FROM game_item_container_entries e \
           JOIN game_item_instances i ON i.item_instance_id = e.item_instance_id \
          WHERE e.parent_item_instance_id = encode($1,'hex')::uuid \
          ORDER BY e.placement_ordinal DESC FOR UPDATE OF i",
    )
    .bind(backpack.as_slice())
    .fetch_all(&mut **tx)
    .await?;
    rows.iter()
        .map(|row| {
            if uuid_text(&row.try_get::<String, _>("world_id")?)? != *world_id.as_bytes() {
                return Err(DurabilityError::InvalidStoredState);
            }
            Ok(Entry {
                item_instance_id: uuid_text(&row.try_get::<String, _>("item_instance_id")?)?,
                placement_ordinal: numeric_u64(row, "placement_ordinal")?,
                definition: TypedDefinitionRef {
                    family: row.try_get("definition_family")?,
                    production_key: row.try_get("definition_production_key")?,
                    revision_ref: row.try_get("definition_revision_ref")?,
                },
                quantity: u32::try_from(row.try_get::<i64, _>("quantity")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                live: row.try_get::<i16, _>("lifecycle")? == 1,
                not_default: row.try_get("not_default")?,
            })
        })
        .collect()
}

/// The audit payload of a materialized trade.
fn trade_message(
    fence: &CurrentCharacterItemFence,
    world_id: WorldId,
    channel_id: crate::foundation::ChannelId,
    backpack: &[u8; 16],
    request: &NpcTradeRequest,
    total: u64,
    plan: &Plan,
) -> OneItemNpcTradeV1 {
    let definition = |line: &NpcTradeLine| match line.coin {
        Some(coin) => request.coins.of(coin),
        None => &request.item.definition,
    };
    let state = |line: &NpcTradeLine, quantity: u32| OneItemStateV1 {
        item_instance_id: line.item_instance_id.to_vec(),
        world_id: world_id.as_bytes().to_vec(),
        definition: Some(OneItemTypedDefinitionRevisionV1 {
            family: definition(line).family.clone(),
            production_key: definition(line).production_key.clone(),
            revision_ref: definition(line).revision_ref.clone(),
        }),
        quantity,
        lifecycle: if quantity == 0 {
            ITEM_LIFECYCLE_RETIRED
        } else {
            ITEM_LIFECYCLE_LIVE
        },
    };
    let cause = &request.cause;
    OneItemNpcTradeV1 {
        cause: Some(OneItemNpcTradeCauseV1 {
            npc_key: cause.npc_key.clone(),
            catalogue_revision: cause.catalogue_revision,
            offer_index: cause.offer_index,
            side: cause.side.code(),
            occurrence: cause.occurrence.to_vec(),
        }),
        command: Some(OneItemCommandRefV1 {
            game_session_id: request.command.game_session_id().as_bytes().to_vec(),
            command_id: request.command.command_id().get(),
        }),
        character_id: fence.character_id.as_bytes().to_vec(),
        world_id: world_id.as_bytes().to_vec(),
        channel_id: channel_id.as_bytes().to_vec(),
        runtime_scope_ownership_generation: fence.scope_ownership_generation.get(),
        backpack_item_instance_id: backpack.to_vec(),
        quantity: request.quantity,
        unit_price: request.unit_price,
        total_gold_units: total,
        burned_gold_units: plan.burned_gold_units,
        minted_gold_units: plan.minted_gold_units,
        lines: plan
            .lines
            .iter()
            .map(|line| OneItemNpcTradeLineV1 {
                kind: line.kind.code(),
                before: line
                    .kind
                    .is_burn()
                    .then(|| state(line, line.quantity_before)),
                after: Some(state(line, line.quantity_after)),
                placement_ordinal: line.placement_ordinal,
            })
            .collect(),
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "one write of the materialized trade"
)]
async fn apply_trade(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: &CurrentCharacterItemFence,
    (world_id, channel_id): (WorldId, crate::foundation::ChannelId),
    backpack: &[u8; 16],
    request: &NpcTradeRequest,
    (total, units): (u64, u32),
    binding: &[u8],
    plan: &Plan,
    envelope: &[u8],
    tuple: Type2EventTuple,
) -> std::result::Result<(), DurabilityError> {
    let gold = |value: u64| i64::try_from(value).map_err(|_| DurabilityError::InvalidStoredState);
    let cause = &request.cause;
    let item = &request.item.definition;
    sqlx::query(
        "INSERT INTO game_npc_trades(transaction_id, event_id, occurrence_id, game_session_id, \
           command_id, character_id, request_binding, world_id, channel_id, \
           runtime_scope_ownership_generation, npc_key, catalogue_revision, offer_index, side, \
           quantity, unit_price, total_gold_units, item_definition_family, \
           item_definition_production_key, item_definition_revision_ref, item_instance_id, \
           item_units, backpack_item_instance_id, burned_gold_units, minted_gold_units, \
           line_count, occurred_at, envelope_sha256, committed_at) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, \
           encode($4,'hex')::uuid, $5::text::numeric(20,0), encode($6,'hex')::uuid, $7, \
           encode($8,'hex')::uuid, encode($9,'hex')::uuid, $10::text::numeric(20,0), $11, \
           $12::text::numeric(20,0), $13, $14, $15, $16, $17, $18, $19, $20, \
           encode($21,'hex')::uuid, $22, encode($23,'hex')::uuid, $24, $25, $26, $27, \
           sha256($28), floor(extract(epoch FROM statement_timestamp())*1000)::bigint)",
    )
    .bind(request.transaction_id.as_slice())
    .bind(request.event_id.as_slice())
    .bind(cause.occurrence.as_slice())
    .bind(request.command.game_session_id().as_bytes().as_slice())
    .bind(request.command.command_id().get().to_string())
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(binding)
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .bind(fence.scope_ownership_generation.get().to_string())
    .bind(&cause.npc_key)
    .bind(cause.catalogue_revision.to_string())
    .bind(i32::try_from(cause.offer_index).map_err(|_| DurabilityError::InvalidStoredState)?)
    .bind(i16::try_from(cause.side.code()).map_err(|_| DurabilityError::InvalidStoredState)?)
    .bind(i64::from(request.quantity))
    .bind(gold(request.unit_price)?)
    .bind(gold(total)?)
    .bind(&item.family)
    .bind(&item.production_key)
    .bind(&item.revision_ref)
    .bind(plan.item_instance_id.as_slice())
    .bind(i64::from(units))
    .bind(backpack.as_slice())
    .bind(gold(plan.burned_gold_units)?)
    .bind(gold(plan.minted_gold_units)?)
    .bind(i16::try_from(plan.lines.len()).map_err(|_| DurabilityError::InvalidStoredState)?)
    .bind(request.occurred_at_unix_ms)
    .bind(envelope)
    .execute(&mut **tx)
    .await?;

    for (ordinal, line) in (1_i16..).zip(&plan.lines) {
        let definition = match line.coin {
            Some(coin) => request.coins.of(coin),
            None => item,
        };
        if line.kind.is_burn() {
            let changed = sqlx::query(
                "UPDATE game_item_instances \
                    SET quantity = $2, lifecycle = $3, last_transaction_id = encode($4,'hex')::uuid \
                  WHERE item_instance_id = encode($1,'hex')::uuid AND lifecycle = 1 \
                    AND quantity = $5",
            )
            .bind(line.item_instance_id.as_slice())
            .bind(i64::from(line.quantity_after))
            .bind(if line.quantity_after == 0 { 2_i16 } else { 1 })
            .bind(request.transaction_id.as_slice())
            .bind(i64::from(line.quantity_before))
            .execute(&mut **tx)
            .await?;
            if changed.rows_affected() != 1 {
                return Err(DurabilityError::InvalidStoredState);
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
                .execute(&mut **tx)
                .await?;
                if removed.rows_affected() != 1 {
                    return Err(DurabilityError::InvalidStoredState);
                }
            }
        } else {
            sqlx::query(
                "INSERT INTO game_item_instances(item_instance_id, world_id, definition_family, \
                   definition_production_key, definition_revision_ref, quantity, lifecycle, \
                   minted_transaction_id) \
                 VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3, $4, $5, $6, 1, \
                   encode($7,'hex')::uuid)",
            )
            .bind(line.item_instance_id.as_slice())
            .bind(world_id.as_bytes().as_slice())
            .bind(&definition.family)
            .bind(&definition.production_key)
            .bind(&definition.revision_ref)
            .bind(i64::from(line.quantity_after))
            .bind(request.transaction_id.as_slice())
            .execute(&mut **tx)
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
            .bind(world_id.as_bytes().as_slice())
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(backpack.as_slice())
            .bind(line.placement_ordinal.to_string())
            .bind(request.transaction_id.as_slice())
            .execute(&mut **tx)
            .await?;
        }
        sqlx::query(
            "INSERT INTO game_npc_trade_lines(transaction_id, line_ordinal, line_kind, \
               item_instance_id, placement_ordinal, coin_worth, quantity_before, quantity_after) \
             VALUES (encode($1,'hex')::uuid, $2, $3, encode($4,'hex')::uuid, \
               $5::text::numeric(20,0), $6, $7, $8)",
        )
        .bind(request.transaction_id.as_slice())
        .bind(ordinal)
        .bind(i16::try_from(line.kind.code()).map_err(|_| DurabilityError::InvalidStoredState)?)
        .bind(line.item_instance_id.as_slice())
        .bind(line.placement_ordinal.to_string())
        .bind(line.coin.map(|coin| coin.worth()).map(gold).transpose()?)
        .bind(i64::from(line.quantity_before))
        .bind(i64::from(line.quantity_after))
        .execute(&mut **tx)
        .await?;
    }

    sqlx::query(
        "INSERT INTO game_item_audit_outbox(event_id, transaction_id, transaction_ordinal, \
           transaction_count, event_type_id, schema_revision, retention_profile_id, \
           item_instance_id, occurred_at, expires_at, envelope, envelope_sha256, \
           publication_state) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, 1, $3, $4, $5, \
           encode($6,'hex')::uuid, $7, $7 + $8, $9, sha256($9), 1)",
    )
    .bind(request.event_id.as_slice())
    .bind(request.transaction_id.as_slice())
    .bind(i64::from(mint_audit::EVENT_TYPE_ID))
    .bind(i64::from(tuple.schema_revision()))
    .bind(tuple.retention_profile_id())
    .bind(plan.item_instance_id.as_slice())
    .bind(request.occurred_at_unix_ms)
    .bind(mint_audit::AUDIT_RETENTION_P90D_MS)
    .bind(envelope)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// The committed trade of `(occurrence, character)`: `Err(ConflictingOccurrence)` when it was
/// bound otherwise, `None` when the occurrence has not committed.
async fn load_trade(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    occurrence: &[u8; 16],
    character_id: CharacterId,
    binding: &[u8],
) -> std::result::Result<Option<Result<CommittedNpcTrade>>, DurabilityError> {
    let Some(row) = sqlx::query(
        "SELECT transaction_id::text, event_id::text, request_binding, side, total_gold_units, \
                burned_gold_units, minted_gold_units \
           FROM game_npc_trades \
          WHERE occurrence_id = encode($1,'hex')::uuid AND character_id = encode($2,'hex')::uuid",
    )
    .bind(occurrence.as_slice())
    .bind(character_id.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(None);
    };
    if row.try_get::<Vec<u8>, _>("request_binding")? != binding {
        return Ok(Some(Err(NpcTradeError::ConflictingOccurrence)));
    }
    let transaction_id = uuid_text(&row.try_get::<String, _>("transaction_id")?)?;
    let gold = |column: &str| -> std::result::Result<u64, DurabilityError> {
        u64::try_from(row.try_get::<i64, _>(column)?)
            .map_err(|_| DurabilityError::InvalidStoredState)
    };
    let side = match row.try_get::<i16, _>("side")? {
        1 => NpcTradeSide::Buy,
        2 => NpcTradeSide::Sell,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let lines = sqlx::query(
        "SELECT line_kind, item_instance_id::text, placement_ordinal::text, coin_worth, \
                quantity_before, quantity_after \
           FROM game_npc_trade_lines \
          WHERE transaction_id = encode($1,'hex')::uuid ORDER BY line_ordinal",
    )
    .bind(transaction_id.as_slice())
    .fetch_all(&mut **tx)
    .await?
    .iter()
    .map(|line| {
        let quantity = |column: &str| -> std::result::Result<u32, DurabilityError> {
            u32::try_from(line.try_get::<i64, _>(column)?)
                .map_err(|_| DurabilityError::InvalidStoredState)
        };
        let coin = match line.try_get::<Option<i64>, _>("coin_worth")? {
            None => None,
            Some(worth) => Some(
                Coin::ALL
                    .into_iter()
                    .find(|coin| i64::try_from(coin.worth()) == Ok(worth))
                    .ok_or(DurabilityError::InvalidStoredState)?,
            ),
        };
        Ok(NpcTradeLine {
            kind: NpcTradeLineKind::of(line.try_get("line_kind")?)
                .ok_or(DurabilityError::InvalidStoredState)?,
            item_instance_id: uuid_text(&line.try_get::<String, _>("item_instance_id")?)?,
            placement_ordinal: numeric_u64(line, "placement_ordinal")?,
            coin,
            quantity_before: quantity("quantity_before")?,
            quantity_after: quantity("quantity_after")?,
        })
    })
    .collect::<std::result::Result<Vec<_>, DurabilityError>>()?;
    Ok(Some(Ok(CommittedNpcTrade {
        transaction_id,
        event_id: uuid_text(&row.try_get::<String, _>("event_id")?)?,
        character_id,
        side,
        total_gold_units: gold("total_gold_units")?,
        burned_gold_units: gold("burned_gold_units")?,
        minted_gold_units: gold("minted_gold_units")?,
        lines,
    })))
}

#[cfg(test)]
#[path = "npc_trade_tests.rs"]
mod tests;
