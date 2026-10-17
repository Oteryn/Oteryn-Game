//! PLAYER_TRADE0 §4 native borrowed Channel TradeOwner plus its original leaf lifecycle.
//!
//! Native TradeOwner re-reads actor/control/lease/visibility/range/map and durable inventory
//! at every offer/accept. Durable inventory reads live in durability::player_trade;
//! this gameplay child retains the owner-local book. The original leaf remains policy DATA;
//! accept_pending composes those reads with the real FND CommandIngress implementation and
//! retains one occurrence after a terminal first accept and a still-pending second accept.
//! CommandIngress does not bind itself to a GameSession. The shared session owner must supply
//! and retain its actual ingress; that installation and provenance check are still absent.
//! The owner retains one frozen writer request before an await, reconciles it before each
//! submission and closes TRANSFERRING only from the native writer/reconciler's result and
//! successful native FND terminalization. Shared session ownership and physical cross-command
//! reservations remain uninstalled; these methods do not establish that installation.

#![allow(
    dead_code,
    reason = "private Trade component awaits shared session ownership and physical reservations"
)]

use crate::durability::player_trade::{
    CommittedPlayerTrade, PlayerTradeError, PlayerTradeEventIdentity, PlayerTradeOutcome,
    TradeData, TradeReadError,
};
use crate::foundation::{
    CommandIngress, CommandLifecycleError, CommandRef, CommandSemanticIdentity,
    DuplicateDisposition, GameSessionId, NormalizedSemanticIntentIdentity, RetainedBindingIdentity,
    RuntimeScopeRefV1, TerminalSemanticOutcome,
};
use sha2::{Digest, Sha256};

const IDLE_MS: u64 = 120_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Side {
    First,
    Second,
}
impl Side {
    fn index(self) -> usize {
        match self {
            Self::First => 0,
            Self::Second => 1,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CloseReason {
    Completed,
    Cancelled,
    NoRoom,
    Stale,
    Rejected,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    Offered,
    Ready,
    Transferring,
    Closed(CloseReason),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TradeRefusal {
    Stale,
    Rejected,
    Transferring,
    NotReady,
}

/// Immutable native cause/binding data, not an issued occurrence, reservation or commit token.
/// pair identifies the counter-offer internally; TradeOwner exposes its issued 128-bit trade id.
/// The wire codec and shared command installation remain integration work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransferBinding {
    pub pair: CommandRef,
    pub accepts: [CommandRef; 2],
    pub trigger: CommandRef,
    pub data: TradeData,
}

/// Supplied only after the real writer/reconciler verifies the matching outcome. Unknown is
/// not rollback. This enum cannot authenticate an outcome or release a physical reservation.
pub(crate) enum ObservedSwap {
    Unknown,
    Committed,
    NoRoom,
    Stale,
    Rejected,
}

#[derive(Clone)]
pub(crate) struct TradeSession {
    phase: Phase,
    data: TradeData,
    pair: Option<CommandRef>,
    accepts: [Option<CommandRef>; 2],
    last_activity_ms: u64,
    transfer: Option<TransferBinding>,
}

impl TradeSession {
    /// The owner must atomically enforce one open trade per actor, after resolving the real
    /// target and all start gates. An offer is only a reference and moves nothing.
    pub(crate) fn record_revalidated_start(
        data: TradeData,
        command: CommandRef,
        now_ms: u64,
    ) -> Result<Self, TradeRefusal> {
        let [a, b] = data.traders;
        if a.account_id == b.account_id
            || a.fence.character_id == b.fence.character_id
            || a.fence.runtime_scope != b.fence.runtime_scope
            || !matches!(a.fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
            || command.game_session_id() != a.fence.game_session_id
            || data.offers[1].is_some()
            || data.offers[0].as_ref().is_none_or(|offer| {
                !offer.matches_backpack(
                    &data.backpacks[0],
                    a.fence.runtime_scope.world_id().as_bytes(),
                )
            })
        {
            return Err(TradeRefusal::Rejected);
        }
        Ok(Self {
            phase: Phase::Offered,
            data,
            pair: None,
            accepts: [None, None],
            last_activity_ms: now_ms,
            transfer: None,
        })
    }

    pub(crate) fn phase(&self) -> Phase {
        self.phase
    }
    pub(crate) fn transfer(&self) -> Option<&TransferBinding> {
        self.transfer.as_ref()
    }

    /// Counter-offer only. Any replacement/other issued change must cancel through invalidate.
    pub(crate) fn record_revalidated_counter_offer(
        &mut self,
        current: TradeData,
        command: CommandRef,
        now_ms: u64,
    ) -> Result<(), TradeRefusal> {
        self.pre_transfer(now_ms)?;
        if self.phase != Phase::Offered {
            return Err(TradeRefusal::NotReady);
        }
        if command.game_session_id() != self.data.traders[1].fence.game_session_id {
            return Err(TradeRefusal::Stale);
        }
        if current.traders != self.data.traders
            || current.backpacks != self.data.backpacks
            || current.backpack_versions != self.data.backpack_versions
            || current.offers[0] != self.data.offers[0]
            || current.offers[1].as_ref().is_none_or(|offer| {
                !offer.matches_backpack(
                    &current.backpacks[1],
                    current.traders[1].fence.runtime_scope.world_id().as_bytes(),
                )
            })
        {
            return self.stale();
        }
        self.data = current;
        self.pair = Some(command);
        self.phase = Phase::Ready;
        self.last_activity_ms = now_ms;
        Ok(())
    }

    /// The owner rechecks visibility/range/sightline for EACH accept. Current data must come
    /// from independent live reads, not by cloning this session's captured binding.
    pub(crate) fn record_revalidated_accept(
        &mut self,
        side: Side,
        pair: CommandRef,
        command: CommandRef,
        current: &TradeData,
        now_ms: u64,
    ) -> Result<(), TradeRefusal> {
        self.pre_transfer(now_ms)?;
        if self.phase != Phase::Ready {
            return Err(TradeRefusal::NotReady);
        }
        if self.pair != Some(pair)
            || command.game_session_id() != self.data.traders[side.index()].fence.game_session_id
        {
            return Err(TradeRefusal::Stale);
        }
        if current != &self.data {
            return self.stale();
        }
        // Preserve the first terminal accept's CommandRef; a later accept cannot rebind it.
        if self.accepts[side.index()] != Some(command) {
            self.last_activity_ms = now_ms;
        }
        self.accepts[side.index()].get_or_insert(command);
        if let [Some(first), Some(second)] = self.accepts {
            self.transfer = Some(TransferBinding {
                pair,
                accepts: [first, second],
                trigger: command,
                data: self.data.clone(),
            });
            self.phase = Phase::Transferring;
        }
        Ok(())
    }

    /// All relevant issued/committed item/backpack changes and pre-transfer boundary events
    /// must reach this hook. During transfer the owner refuses/defer such operations instead.
    pub(crate) fn invalidate(&mut self) -> Result<(), TradeRefusal> {
        if self.phase == Phase::Transferring {
            return Err(TradeRefusal::Transferring);
        }
        if !matches!(self.phase, Phase::Closed(_)) {
            self.phase = Phase::Closed(CloseReason::Cancelled);
        }
        Ok(())
    }

    pub(crate) fn reconnect(
        &mut self,
        side: Side,
        session: GameSessionId,
    ) -> Result<(), TradeRefusal> {
        if self.phase == Phase::Transferring {
            return if session == self.data.traders[side.index()].fence.game_session_id {
                Ok(())
            } else {
                Err(TradeRefusal::Stale)
            };
        }
        self.invalidate()
    }

    /// Cancel/timeout never close TRANSFERRING. No outcome-derived data is treated as a fresh
    /// session or recovery fence; actual receipt/recovery checks remain with the writer.
    pub(crate) fn record_outcome_data(
        &mut self,
        binding: &TransferBinding,
        outcome: ObservedSwap,
    ) -> Result<(), TradeRefusal> {
        if self.phase != Phase::Transferring || self.transfer.as_ref() != Some(binding) {
            return Err(TradeRefusal::Stale);
        }
        let reason = match outcome {
            ObservedSwap::Unknown => return Ok(()),
            ObservedSwap::Committed => CloseReason::Completed,
            ObservedSwap::NoRoom => CloseReason::NoRoom,
            ObservedSwap::Stale => CloseReason::Stale,
            ObservedSwap::Rejected => CloseReason::Rejected,
        };
        self.phase = Phase::Closed(reason);
        Ok(())
    }

    pub(crate) fn expire_idle(&mut self, now_ms: u64) -> Result<(), TradeRefusal> {
        if matches!(self.phase, Phase::Offered | Phase::Ready) {
            let elapsed = now_ms
                .checked_sub(self.last_activity_ms)
                .ok_or(TradeRefusal::Rejected)?;
            if elapsed >= IDLE_MS {
                self.phase = Phase::Closed(CloseReason::Cancelled);
            }
        }
        Ok(())
    }
    fn pre_transfer(&mut self, now_ms: u64) -> Result<(), TradeRefusal> {
        self.expire_idle(now_ms)?;
        match self.phase {
            Phase::Transferring => Err(TradeRefusal::Transferring),
            Phase::Closed(_) => Err(TradeRefusal::Stale),
            _ => Ok(()),
        }
    }
    fn stale<T>(&mut self) -> Result<T, TradeRefusal> {
        self.phase = Phase::Closed(CloseReason::Stale);
        Err(TradeRefusal::Stale)
    }
}

// Native gameplay owner dependencies. Durable observation remains in its own leaf;
// physical cross-command reservations and atomic swap still require their owning integration.
use crate::content::{ActiveGeneration, QualifiedNativeEntryRoom, ReferenceItemField};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::{DurabilityError, DurabilityRoot};
use crate::foundation::{ChannelRuntimeAssignmentBinding, ChannelRuntimeV1, ExactActorRef};
use crate::movement::interest::{
    InterestEntity, InterestIndex, VisibilityPosition, VisibilitySettings,
};
use crate::spell::world_execution::{qualified_combat_tile, source_sight_steps};
use crate::world_runtime::LocalObjectRuntime;
use std::collections::BTreeMap;
use std::time::Instant;

#[derive(Debug)]
pub(crate) enum OwnerError {
    Policy(TradeRefusal),
    Ingress(TradeIngressRefusal),
    CommandLifecycle(CommandLifecycleError),
    Authority,
    TooFar,
    NotVisible,
    UnknownMap,
    NotTradeable,
    NoBackpack,
    NoRoom,
    Busy,
    Storage(DurabilityError),
    Swap(PlayerTradeError),
}
impl std::fmt::Display for OwnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Policy(refusal) => write!(f, "trade policy: {refusal:?}"),
            Self::Storage(error) => write!(f, "trade storage: {error}"),
            Self::Swap(error) => write!(f, "trade swap: {error}"),
            other => write!(f, "trade owner: {other:?}"),
        }
    }
}
impl std::error::Error for OwnerError {}
impl From<CommandLifecycleError> for OwnerError {
    fn from(value: CommandLifecycleError) -> Self {
        Self::CommandLifecycle(value)
    }
}
impl From<DurabilityError> for OwnerError {
    fn from(value: DurabilityError) -> Self {
        Self::Storage(value)
    }
}
impl From<PlayerTradeError> for OwnerError {
    fn from(value: PlayerTradeError) -> Self {
        Self::Swap(value)
    }
}
impl From<sqlx::Error> for OwnerError {
    fn from(value: sqlx::Error) -> Self {
        Self::Storage(value.into())
    }
}
impl From<TradeReadError> for OwnerError {
    fn from(error: TradeReadError) -> Self {
        match error {
            TradeReadError::Authority => Self::Authority,
            TradeReadError::NoBackpack => Self::NoBackpack,
            TradeReadError::NotTradeable => Self::NotTradeable,
            TradeReadError::Stale => Self::Policy(TradeRefusal::Stale),
            TradeReadError::Storage(error) => Self::Storage(error),
        }
    }
}
impl From<TradeRefusal> for OwnerError {
    fn from(value: TradeRefusal) -> Self {
        Self::Policy(value)
    }
}

/// Exact runtime actor and expected fence are request names. The carrier and database
/// independently resolve each binding; these public-to-the-crate fields mint no authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TradeActor {
    pub(crate) actor: ExactActorRef,
    pub(crate) fence: CurrentCharacterItemFence,
}

/// Borrowed real native owners, not injected current/visibility/LOS callbacks.
pub(crate) struct TradeReadContext<'a, 'f, 's> {
    pub(crate) runtime: &'a ChannelRuntimeV1,
    pub(crate) root: &'a DurabilityRoot,
    pub(crate) recovery: &'a ReconciledCharacterAuthority<'f, 's>,
    pub(crate) node: &'a NodeIncarnationProof,
    pub(crate) room: &'a QualifiedNativeEntryRoom,
    pub(crate) objects: &'a LocalObjectRuntime,
    pub(crate) generation: &'a ActiveGeneration,
}

struct OwnedTrade {
    actors: [TradeActor; 2],
    session: TradeSession,
    intent: Option<ReservationIntent>,
    issuance: Option<TradeIssuance>,
    retained_swap: Option<RetainedTradeSwap>,
}

/// The exact immutable request is installed before any storage await. A cancellation or
/// unknown outcome leaves it in the same owner book; a later attempt cannot choose new IDs.
struct RetainedTradeSwap {
    identity: PlayerTradeEventIdentity,
    request: FrozenPlayerTrade,
    resolution: Option<ResolvedTradeSwap>,
}

/// Private and populated only by the actual native writer/reconciler below. A caller cannot
/// submit outcome DATA as authority to release the trade's in-memory exclusion.
#[derive(Clone)]
enum ResolvedTradeSwap {
    Committed(Box<CommittedPlayerTrade>),
    Refused(CloseReason),
}

/// Terminal output of this component. It describes an already settled outcome, not authority
/// to execute another swap. Replayed wire output remains the native ingress owner's work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NativeTradeCompletion {
    Completed(Box<CommittedPlayerTrade>),
    Refused(CloseReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TradeIngressRefusal {
    MissingPending,
    ConflictChangedInput,
    OutcomeExpired,
    UnexpectedTerminal,
    UnissuedAcceptance,
    PendingOutOfOrder,
    UnfrozenTransfer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeAcceptOutcome {
    Accepted,
    /// Historical result only: no current actor/DB read or new consent is implied.
    ReplayedAccepted,
    Transferring,
}

/// Created only inside this component's successful native CommandIngress::terminalize mutation.
/// It survives eviction of the FND retained terminal record while this trade remains open.
/// Session-owner ingress provenance is an uninstalled caller responsibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TerminalTradeAccept {
    side: Side,
    command: CommandRef,
    fence: CurrentCharacterItemFence,
}

/// A component-issued occurrence, not authenticated session consent or a committed-swap proof.
/// The native ingress lifecycle is enforced here; its session-owner provenance must be
/// enforced at the shared caller installation, which is not present in this component.
/// The native writer consumes recovery/node/current fences and rechecks both sources
/// and destinations. There is no public constructor or decoding path.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct IssuedTradeOccurrence {
    trade_id: [u8; 16],
    occurrence: [u8; 16],
    first: TerminalTradeAccept,
    trigger: CommandRef,
    /// Exact owner-produced addresses and before-data; a caller cannot substitute another
    /// DATA plan when freezing the native writer request from this private issuance.
    intent: Box<ReservationIntent>,
}
impl IssuedTradeOccurrence {
    pub(crate) fn trade_id(&self) -> &[u8; 16] {
        &self.trade_id
    }
    pub(crate) fn occurrence(&self) -> &[u8; 16] {
        &self.occurrence
    }
    pub(crate) fn first_accept(&self) -> CommandRef {
        self.first.command
    }
    pub(crate) fn first_fence(&self) -> CurrentCharacterItemFence {
        self.first.fence
    }
    pub(crate) fn trigger(&self) -> CommandRef {
        self.trigger
    }
    pub(crate) fn intent(&self) -> &ReservationIntent {
        &self.intent
    }
}

enum TradeIssuance {
    FirstAccepted(TerminalTradeAccept),
    Transferring(IssuedTradeOccurrence),
}

struct PreparedAccept {
    key: ([u8; 16], u64),
    side: Side,
    session: TradeSession,
    intent: Option<ReservationIntent>,
}

/// Same full-width command-bound identity convention as native NPC trade. The tag
/// separates the server trade id from the second-accept occurrence. Intent is bound
/// separately: changing it must conflict under the same occurrence, not mint a new cause.
fn command_identity(tag: &[u8], command: CommandRef) -> [u8; 16] {
    let hash = Sha256::new()
        .chain_update(tag)
        .chain_update(command.game_session_id().as_bytes())
        .chain_update(command.command_id().get().to_be_bytes())
        .finalize();
    let mut id = [0; 16];
    id.copy_from_slice(&hash[..16]);
    id[6] = (id[6] & 0x0f) | 0x70;
    id[8] = (id[8] & 0x3f) | 0x80;
    id
}
fn issued_trade_id(pair: CommandRef) -> [u8; 16] {
    command_identity(b"oteryn:player-trade-id:v1", pair)
}
fn trade_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(HEX[usize::from(byte >> 4)]));
        text.push(char::from(HEX[usize::from(byte & 15)]));
    }
    text
}

/// Planned addresses only, never a reservation token or durable swap authorization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReservationIntent {
    pub(crate) binding: TransferBinding,
    pub(crate) destinations: [crate::durability::item_transfer::ContainerEntryPosition; 2],
}

/// One owner-local book, bounded by the actual native runtime actor capacity. CP must install
/// exactly one book in the Channel owner and route physical invalidations to it. No constructor
/// can prove that uninstalled global composition. Runtime is borrowed only per operation.
pub(crate) struct TradeOwner {
    binding: (ChannelRuntimeAssignmentBinding, [u8; 32]),
    origin: Instant,
    trades: BTreeMap<([u8; 16], u64), OwnedTrade>,
    by_actor: BTreeMap<[u8; 16], ([u8; 16], u64)>,
}

fn runtime_key(runtime: &ChannelRuntimeV1) -> (ChannelRuntimeAssignmentBinding, [u8; 32]) {
    (
        runtime.binding(),
        runtime.content_pin().server_artifact_digest(),
    )
}

fn trade_key(command: CommandRef) -> ([u8; 16], u64) {
    (
        *command.game_session_id().as_bytes(),
        command.command_id().get(),
    )
}

impl TradeOwner {
    pub(crate) fn new(runtime: &ChannelRuntimeV1, now: Instant) -> Result<Self, OwnerError> {
        runtime.owner_fence().map_err(|_| OwnerError::Authority)?;
        Ok(Self {
            binding: runtime_key(runtime),
            origin: now,
            trades: BTreeMap::new(),
            by_actor: BTreeMap::new(),
        })
    }
    fn millis(&self, now: Instant) -> Result<u64, OwnerError> {
        u64::try_from(
            now.checked_duration_since(self.origin)
                .ok_or(OwnerError::Authority)?
                .as_millis(),
        )
        .map_err(|_| OwnerError::Authority)
    }
    /// Observes offered leaf membership in this owner-local book, not physical custody or
    /// a durable reservation. The caller must borrow the actual installed sole book under
    /// its serialization guard through the same transaction that consumes this answer.
    /// That transaction must independently enforce native current session, assignment and
    /// item fences. This runtime carrier/book check does not establish DB-current authority.
    /// No constructor or this query proves that installation; caller wiring remains CP.
    /// A retired or differently bound runtime refuses even when the book is empty.
    pub(crate) fn item_reserved_by_trade(
        &self,
        runtime: &ChannelRuntimeV1,
        item: [u8; 16],
    ) -> Result<bool, OwnerError> {
        runtime.owner_fence().map_err(|_| OwnerError::Authority)?;
        if runtime_key(runtime) != self.binding {
            return Err(OwnerError::Authority);
        }
        Ok(self.trades.values().any(|trade| {
            matches!(
                trade.session.phase(),
                Phase::Offered | Phase::Ready | Phase::Transferring
            ) && trade
                .session
                .data
                .offers
                .iter()
                .flatten()
                .any(|offer| offer.state.item_instance_id.as_slice() == item.as_slice())
        }))
    }
    pub(crate) fn phase(&self, actor: ExactActorRef) -> Option<Phase> {
        self.by_actor
            .get(&actor.placement_identity())
            .and_then(|key| self.trades.get(key))
            .map(|trade| trade.session.phase())
    }
    pub(crate) fn reservation_intent(&self, actor: ExactActorRef) -> Option<&ReservationIntent> {
        self.by_actor
            .get(&actor.placement_identity())
            .and_then(|key| self.trades.get(key))
            .and_then(|trade| trade.intent.as_ref())
    }
    pub(crate) fn pending_transfer(&self, actor: ExactActorRef) -> Option<&TransferBinding> {
        self.by_actor
            .get(&actor.placement_identity())
            .and_then(|key| self.trades.get(key))
            .and_then(|trade| trade.session.transfer())
    }
    pub(crate) fn expire(&mut self, now: Instant) -> Result<(), OwnerError> {
        let ms = self.millis(now)?;
        for trade in self.trades.values_mut() {
            trade.session.expire_idle(ms)?;
        }
        self.prune_closed();
        Ok(())
    }
    fn prune_closed(&mut self) {
        self.trades
            .retain(|_, trade| !matches!(trade.session.phase(), Phase::Closed(_)));
        self.by_actor.retain(|_, key| self.trades.contains_key(key));
    }
    fn current_geometry(
        &self,
        context: &TradeReadContext<'_, '_, '_>,
        actors: [TradeActor; 2],
    ) -> Result<(), OwnerError> {
        let runtime = context.runtime;
        runtime.owner_fence().map_err(|_| OwnerError::Authority)?;
        if runtime_key(runtime) != self.binding {
            return Err(OwnerError::Authority);
        }
        // Binding equality above is immutable expected DATA only. Current DB assignment,
        // node possession, Content and sessions are resolved independently by the native
        // equipment owner inside read_player_trade_data; no record-derived grant is minted.
        if context.generation.identity().server_artifact_digest()
            != context.runtime.content_pin().server_artifact_digest()
        {
            return Err(OwnerError::Authority);
        }
        qualify_runtime_pair(runtime, context.room, context.objects, actors)
    }
    async fn read(
        &self,
        context: &TradeReadContext<'_, '_, '_>,
        actors: [TradeActor; 2],
        items: [Option<[u8; 16]>; 2],
    ) -> Result<TradeData, OwnerError> {
        self.current_geometry(context, actors)?;
        let data = context
            .root
            .read_player_trade_data(
                context.recovery,
                context.node,
                context.runtime,
                actors.map(|a| a.fence),
                items,
                context.runtime.content_pin().server_artifact_digest(),
            )
            .await?;
        // The actual native runtime stays immutably borrowed throughout the read. Recheck
        // nevertheless; retained DB data is not a grant for any later owner turn or swap.
        self.current_geometry(context, actors)?;
        for offer in data.offers.iter().flatten() {
            let definition = offer
                .state
                .definition
                .as_ref()
                .ok_or(OwnerError::NotTradeable)?;
            let policy = context
                .generation
                .native_gameplay()
                .and_then(|native| {
                    native.item_policy(&definition.production_key, &definition.revision_ref)
                })
                .ok_or(OwnerError::NotTradeable)?;
            let record = policy.record();
            if record.production_definition.family != definition.family
                || record.production_definition.production_key != definition.production_key
                || record.production_definition.revision_ref != definition.revision_ref
            {
                return Err(OwnerError::NotTradeable);
            }
            let ReferenceItemField::Known(physical) = &record.semantics.physical else {
                return Err(OwnerError::NotTradeable);
            };
            if physical.pickupable != ReferenceItemField::Known(true) {
                return Err(OwnerError::NotTradeable);
            }
            match &record.semantics.trade_restrictions {
                ReferenceItemField::Unknown => {}
                ReferenceItemField::Known(flags)
                    if crate::domain::player_trade::tradeable_flag_allows(flags) => {}
                _ => return Err(OwnerError::NotTradeable),
            }
        }
        Ok(data)
    }
    pub(crate) async fn offer(
        &mut self,
        context: &TradeReadContext<'_, '_, '_>,
        actors: [TradeActor; 2],
        item: [u8; 16],
        command: CommandRef,
        now: Instant,
    ) -> Result<Phase, OwnerError> {
        self.expire(now)?;
        if actors[0].actor == actors[1].actor {
            return Err(OwnerError::Authority);
        }
        let first_key = self
            .by_actor
            .get(&actors[0].actor.placement_identity())
            .copied();
        let second_key = self
            .by_actor
            .get(&actors[1].actor.placement_identity())
            .copied();
        if let Some(key) = first_key {
            if second_key != Some(key) {
                return Err(OwnerError::Busy);
            }
            let trade = self.trades.get(&key).ok_or(OwnerError::Authority)?;
            if trade.actors != [actors[1], actors[0]] {
                return Err(OwnerError::Busy);
            }
            if trade.session.phase() == Phase::Transferring {
                return Err(OwnerError::Policy(TradeRefusal::Transferring));
            }
            let original = trade.actors;
            let first = trade.session.data.offers[0]
                .as_ref()
                .ok_or(OwnerError::Authority)?
                .state
                .item_instance_id
                .as_slice()
                .try_into()
                .map_err(|_| OwnerError::Authority)?;
            let current = self
                .read(context, original, [Some(first), Some(item)])
                .await;
            let ms = self.millis(now)?;
            let trade = self.trades.get_mut(&key).ok_or(OwnerError::Authority)?;
            match current {
                Ok(current) => trade
                    .session
                    .record_revalidated_counter_offer(current, command, ms)?,
                Err(error) => {
                    let _ = trade.session.invalidate();
                    self.prune_closed();
                    return Err(error);
                }
            }
            return Ok(trade.session.phase());
        }
        if second_key.is_some() {
            return Err(OwnerError::Busy);
        }
        let current = self.read(context, actors, [Some(item), None]).await?;
        let session = TradeSession::record_revalidated_start(current, command, self.millis(now)?)?;
        let key = trade_key(command);
        if self.trades.contains_key(&key)
            || self
                .by_actor
                .len()
                .checked_add(2)
                .is_none_or(|n| n > context.runtime.actor_slot_capacity())
        {
            return Err(OwnerError::Busy);
        }
        for actor in actors {
            self.by_actor.insert(actor.actor.placement_identity(), key);
        }
        self.trades.insert(
            key,
            OwnedTrade {
                actors,
                session,
                intent: None,
                issuance: None,
                retained_swap: None,
            },
        );
        Ok(Phase::Offered)
    }
    pub(crate) async fn accept(
        &mut self,
        context: &TradeReadContext<'_, '_, '_>,
        actor: TradeActor,
        pair: CommandRef,
        command: CommandRef,
        now: Instant,
    ) -> Result<Phase, OwnerError> {
        // The original DATA adapter cannot mutate a lifecycle owned by native FND ingress.
        if self
            .by_actor
            .get(&actor.actor.placement_identity())
            .and_then(|key| self.trades.get(key))
            .is_some_and(|trade| trade.issuance.is_some())
        {
            return Err(OwnerError::Ingress(TradeIngressRefusal::UnissuedAcceptance));
        }
        let prepared = self
            .prepare_accept(context, actor, pair, command, now)
            .await?;
        let trade = self
            .trades
            .get_mut(&prepared.key)
            .ok_or(OwnerError::Authority)?;
        trade.session = prepared.session;
        trade.intent = prepared.intent;
        Ok(trade.session.phase())
    }

    /// Server-owned 128-bit id, issued when the second offer makes this trade READY.
    /// Item handles and D85 actor identities retain their existing protocol types.
    pub(crate) fn trade_id(&self, actor: ExactActorRef) -> Option<[u8; 16]> {
        let key = self.by_actor.get(&actor.placement_identity())?;
        let trade = self.trades.get(key)?;
        if !trade.actors.iter().any(|trader| trader.actor == actor)
            || !matches!(trade.session.phase(), Phase::Ready | Phase::Transferring)
        {
            return None;
        }
        trade.session.pair.map(issued_trade_id)
    }

    /// Canonical semantic DATA for the actual session owner's FND ingress reservation.
    /// A new id denotes a new trade, so each id has one immutable incarnation/revision.
    pub(crate) fn accept_semantic(
        &self,
        command: CommandRef,
        trade_id: [u8; 16],
    ) -> Result<CommandSemanticIdentity, OwnerError> {
        Ok(CommandSemanticIdentity::new(
            NormalizedSemanticIntentIdentity::new(
                &trade_hex(&trade_id),
                1,
                "player_trade_accept",
                1,
            )?,
            RetainedBindingIdentity::new(
                &trade_hex(&self.binding.1),
                &format!(
                    "{}:{}",
                    trade_hex(command.game_session_id().as_bytes()),
                    command.command_id().get()
                ),
            )?,
        ))
    }

    /// Borrow this component's issued record together with its captured intent. Neither
    /// serialized identities nor a terminal outcome alone can construct it. This borrow
    /// does not prove that the caller supplied the admitted session owner's actual ingress.
    pub(crate) fn issued_transfer(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
    ) -> Result<Option<(&IssuedTradeOccurrence, &ReservationIntent)>, OwnerError> {
        runtime.owner_fence().map_err(|_| OwnerError::Authority)?;
        if runtime_key(runtime) != self.binding {
            return Err(OwnerError::Authority);
        }
        let Some(trade) = self
            .by_actor
            .get(&actor.placement_identity())
            .and_then(|key| self.trades.get(key))
        else {
            return Ok(None);
        };
        if !trade.actors.iter().any(|trader| trader.actor == actor) {
            return Err(OwnerError::Authority);
        }
        match (&trade.issuance, &trade.intent) {
            (Some(TradeIssuance::Transferring(issued)), Some(intent)) => Ok(Some((issued, intent))),
            _ => Ok(None),
        }
    }

    /// Consumes an already-pending command from the supplied native FND ingress. The first valid
    /// accept is installed only inside terminalize's successful mutation; the second
    /// remains pending and issues exactly once. Refusals leave command terminalization
    /// to the caller. Unknown swap outcomes retain this record and the owner-local intent.
    /// The borrowed Root/node/runtime context performs the existing native authority reads;
    /// there is no supplied "current", "accepted" or "pending" boolean.
    /// The installed caller must bind this ingress to the current admitted GameSession and
    /// retain it across continuations. CommandIngress itself has no session ownership field;
    /// an arbitrary replacement ingress would not establish native session consent.
    /// ReplayedAccepted is historical output only and may outlive this book. That branch
    /// checks the runtime key and command session but does not revalidate the current actor,
    /// closed/expired trade or database generation, and never reinstalls an acceptance.
    pub(crate) async fn accept_pending(
        &mut self,
        context: &TradeReadContext<'_, '_, '_>,
        actor: TradeActor,
        trade_id: [u8; 16],
        command: CommandRef,
        ingress: &mut CommandIngress,
        now: Instant,
    ) -> Result<NativeAcceptOutcome, OwnerError> {
        context
            .runtime
            .owner_fence()
            .map_err(|_| OwnerError::Authority)?;
        if runtime_key(context.runtime) != self.binding
            || command.game_session_id() != actor.fence.game_session_id
        {
            return Err(OwnerError::Authority);
        }
        let semantic = self.accept_semantic(command, trade_id)?;
        match ingress.classify_duplicate(command.command_id(), &semantic) {
            DuplicateDisposition::PendingOriginal => {}
            DuplicateDisposition::ReplayRetainedOutcome(outcome)
                if outcome.disposition() == "ACCEPTED"
                    && outcome.state() == trade_hex(&trade_id) =>
            {
                // Replay is an outcome only. It never installs a terminal-accept marker.
                return Ok(NativeAcceptOutcome::ReplayedAccepted);
            }
            DuplicateDisposition::ReplayRetainedOutcome(_) => {
                return Err(OwnerError::Ingress(TradeIngressRefusal::UnexpectedTerminal));
            }
            DuplicateDisposition::ConflictChangedInput => {
                return Err(OwnerError::Ingress(
                    TradeIngressRefusal::ConflictChangedInput,
                ));
            }
            DuplicateDisposition::OutcomeExpired => {
                return Err(OwnerError::Ingress(TradeIngressRefusal::OutcomeExpired));
            }
            DuplicateDisposition::NotDuplicate => {
                return Err(OwnerError::Ingress(TradeIngressRefusal::MissingPending));
            }
        }
        let key = *self
            .by_actor
            .get(&actor.actor.placement_identity())
            .ok_or(OwnerError::Policy(TradeRefusal::Stale))?;
        let trade = self.trades.get(&key).ok_or(OwnerError::Authority)?;
        if !trade.actors.contains(&actor) {
            return Err(OwnerError::Authority);
        }
        let pair = trade
            .session
            .pair
            .ok_or(OwnerError::Policy(TradeRefusal::NotReady))?;
        if issued_trade_id(pair) != trade_id {
            return Err(OwnerError::Policy(TradeRefusal::Stale));
        }
        let first = match &trade.issuance {
            None if trade.session.accepts == [None, None] => None,
            Some(TradeIssuance::FirstAccepted(first))
                if trade.session.accepts[first.side.index()] == Some(first.command)
                    && trade.session.accepts[1 - first.side.index()].is_none()
                    && trade.actors[first.side.index()].fence == first.fence =>
            {
                Some(*first)
            }
            Some(TradeIssuance::Transferring(issued))
                if issued.trigger == command
                    && issued.trade_id == trade_id
                    && trade.session.phase() == Phase::Transferring =>
            {
                // Same pending command observes the same occurrence; no fresh effect is issued.
                return Ok(NativeAcceptOutcome::Transferring);
            }
            _ => return Err(OwnerError::Ingress(TradeIngressRefusal::UnissuedAcceptance)),
        };
        let prepared = self
            .prepare_accept(context, actor, pair, command, now)
            .await?;
        let trade = self
            .trades
            .get_mut(&prepared.key)
            .ok_or(OwnerError::Authority)?;
        match prepared.session.phase() {
            Phase::Ready => {
                let outcome = TerminalSemanticOutcome::new("ACCEPTED", &trade_hex(&trade_id), 1)?;
                ingress.terminalize(command.command_id(), outcome, || {
                    let accepted = first.unwrap_or(TerminalTradeAccept {
                        side: prepared.side,
                        command,
                        fence: actor.fence,
                    });
                    trade.session = prepared.session;
                    trade.intent = prepared.intent;
                    trade.issuance = Some(TradeIssuance::FirstAccepted(accepted));
                })?;
                Ok(NativeAcceptOutcome::Accepted)
            }
            Phase::Transferring => {
                let first =
                    first.ok_or(OwnerError::Ingress(TradeIngressRefusal::UnissuedAcceptance))?;
                if first.side == prepared.side || prepared.intent.is_none() {
                    return Err(OwnerError::Authority);
                }
                // FND pending commands are a contiguous suffix (including recovery).
                // Unlike the first accept, this command must stay pending, so it cannot
                // use terminalize's order check. Check the actual ingress suffix instead.
                let pending_suffix = match ingress.next_command_id() {
                    Some(next) => next.get().checked_sub(command.command_id().get()),
                    None => u64::MAX
                        .checked_sub(command.command_id().get())
                        .and_then(|remaining| remaining.checked_add(1)),
                };
                let outstanding =
                    u64::try_from(ingress.outstanding()).map_err(|_| OwnerError::Authority)?;
                if pending_suffix != Some(outstanding) {
                    return Err(OwnerError::Ingress(TradeIngressRefusal::PendingOutOfOrder));
                }
                let issued = IssuedTradeOccurrence {
                    trade_id,
                    occurrence: command_identity(b"oteryn:player-trade-occurrence:v1", command),
                    first,
                    trigger: command,
                    intent: Box::new(
                        prepared
                            .intent
                            .as_ref()
                            .ok_or(OwnerError::Authority)?
                            .clone(),
                    ),
                };
                trade.session = prepared.session;
                trade.intent = prepared.intent;
                trade.issuance = Some(TradeIssuance::Transferring(issued));
                Ok(NativeAcceptOutcome::Transferring)
            }
            _ => Err(OwnerError::Authority),
        }
    }

    /// Retain the one immutable writer request before a continuation can await storage. The
    /// identity comes from the server allocator; repeating this preparation must name the
    /// exact same identity. No new occurrence or request is created by a retry.
    pub(crate) fn freeze_transfer(
        &mut self,
        runtime: &ChannelRuntimeV1,
        generation: &ActiveGeneration,
        actor: ExactActorRef,
        identity: PlayerTradeEventIdentity,
    ) -> Result<(), OwnerError> {
        if runtime_key(runtime) != self.binding {
            return Err(OwnerError::Authority);
        }
        let key = self.transferring_key(actor)?;
        let trade = self.trades.get_mut(&key).ok_or(OwnerError::Authority)?;
        if let Some(retained) = &trade.retained_swap {
            return if retained.identity == identity {
                Ok(())
            } else {
                Err(OwnerError::Ingress(
                    TradeIngressRefusal::ConflictChangedInput,
                ))
            };
        }
        let Some(TradeIssuance::Transferring(issued)) = &trade.issuance else {
            return Err(OwnerError::Authority);
        };
        let request = FrozenPlayerTrade::from_issued(issued, runtime, generation, identity)?;
        trade.retained_swap = Some(RetainedTradeSwap {
            identity,
            request,
            resolution: None,
        });
        Ok(())
    }

    /// Continue the same pending second accept. Every unresolved attempt first asks the real
    /// recovery-fenced reader while holding the occurrence lock; an absent receipt then
    /// allows the real writer to retry exactly the retained request. Ambiguous storage errors and
    /// cancellation retain TRANSFERRING, both actor indexes, the request and any known result.
    ///
    /// This borrows the supplied FND ingress for the whole continuation and never fabricates
    /// a terminal record. Its installation in the actual GameSession owner is still a shared
    /// integration requirement. The caller must hold the sole Channel trade book and its
    /// physical item/entry exclusion throughout this call and every ambiguous continuation.
    pub(crate) async fn drive_transfer(
        &mut self,
        context: &TradeReadContext<'_, '_, '_>,
        actor: ExactActorRef,
        ingress: &mut CommandIngress,
        first_current_fence: CurrentCharacterItemFence,
    ) -> Result<NativeTradeCompletion, OwnerError> {
        self.resolve_transfer(context, actor, ingress, first_current_fence)
            .await?;
        self.terminalize_resolved_transfer(actor, ingress)
    }

    fn transferring_key(&self, actor: ExactActorRef) -> Result<([u8; 16], u64), OwnerError> {
        let key = *self
            .by_actor
            .get(&actor.placement_identity())
            .ok_or(OwnerError::Policy(TradeRefusal::Stale))?;
        let trade = self.trades.get(&key).ok_or(OwnerError::Authority)?;
        let Some(TradeIssuance::Transferring(issued)) = &trade.issuance else {
            return Err(OwnerError::Ingress(TradeIngressRefusal::UnissuedAcceptance));
        };
        if !trade.actors.iter().any(|named| named.actor == actor)
            || trade.session.phase() != Phase::Transferring
            || trade.session.transfer() != Some(&issued.intent.binding)
            || trade.intent.as_ref() != Some(issued.intent())
        {
            return Err(OwnerError::Authority);
        }
        Ok(key)
    }

    fn require_pending_transfer(
        &self,
        actor: ExactActorRef,
        ingress: &CommandIngress,
    ) -> Result<([u8; 16], u64), OwnerError> {
        let key = self.transferring_key(actor)?;
        let trade = self.trades.get(&key).ok_or(OwnerError::Authority)?;
        let Some(TradeIssuance::Transferring(issued)) = &trade.issuance else {
            return Err(OwnerError::Authority);
        };
        // Only the triggering actor's retained native command can finish this operation.
        // The first accepter's historical terminal record is never terminalized again.
        if !trade.actors.iter().any(|named| {
            named.actor == actor && named.fence.game_session_id == issued.trigger.game_session_id()
        }) {
            return Err(OwnerError::Authority);
        }
        let command = issued.trigger.command_id();
        let semantic = self.accept_semantic(issued.trigger, issued.trade_id)?;
        match ingress.classify_duplicate(command, &semantic) {
            DuplicateDisposition::PendingOriginal => {}
            DuplicateDisposition::ConflictChangedInput => {
                return Err(OwnerError::Ingress(
                    TradeIngressRefusal::ConflictChangedInput,
                ));
            }
            DuplicateDisposition::OutcomeExpired => {
                return Err(OwnerError::Ingress(TradeIngressRefusal::OutcomeExpired));
            }
            DuplicateDisposition::ReplayRetainedOutcome(_) => {
                return Err(OwnerError::Ingress(TradeIngressRefusal::UnexpectedTerminal));
            }
            DuplicateDisposition::NotDuplicate => {
                return Err(OwnerError::Ingress(TradeIngressRefusal::MissingPending));
            }
        }
        let suffix = match ingress.next_command_id() {
            Some(next) => next.get().checked_sub(command.get()),
            None => u64::MAX
                .checked_sub(command.get())
                .and_then(|remaining| remaining.checked_add(1)),
        };
        let outstanding =
            u64::try_from(ingress.outstanding()).map_err(|_| OwnerError::Authority)?;
        if suffix != Some(outstanding) {
            return Err(OwnerError::Ingress(TradeIngressRefusal::PendingOutOfOrder));
        }
        Ok(key)
    }

    /// Cache only a result returned by native storage. Keeping the cache separate from FND
    /// terminalization makes a terminal-retention refusal safe: the next call retries output,
    /// not gameplay. There is no public setter or supplied outcome/proof argument.
    async fn resolve_transfer(
        &mut self,
        context: &TradeReadContext<'_, '_, '_>,
        actor: ExactActorRef,
        ingress: &CommandIngress,
        first_current_fence: CurrentCharacterItemFence,
    ) -> Result<(), OwnerError> {
        let key = self.require_pending_transfer(actor, ingress)?;
        let trade = self.trades.get(&key).ok_or(OwnerError::Authority)?;
        let Some(TradeIssuance::Transferring(issued)) = &trade.issuance else {
            return Err(OwnerError::Authority);
        };
        let retained = trade
            .retained_swap
            .as_ref()
            .ok_or(OwnerError::Ingress(TradeIngressRefusal::UnfrozenTransfer))?;
        if retained.resolution.is_some() {
            return Ok(());
        }
        let resolution = match context
            .root
            .reconcile_player_trade(context.recovery, &retained.request)
            .await?
        {
            Some(committed) => ResolvedTradeSwap::Committed(Box::new(committed)),
            None => {
                // The recovery reader has waited for any previous occurrence transaction.
                // Native refusal of this subsequent attempt is definitive for this retained
                // request; errors before that read completes never close an unknown swap.
                match context
                    .root
                    .commit_player_trade(
                        PlayerTradeCommitAuthority {
                            recovery: context.recovery,
                            node: context.node,
                            runtime: context.runtime,
                            issued,
                            first_current_fence,
                        },
                        &retained.request,
                    )
                    .await
                {
                    Ok(PlayerTradeOutcome::Committed(committed))
                    | Ok(PlayerTradeOutcome::AlreadyCommitted(committed)) => {
                        ResolvedTradeSwap::Committed(Box::new(committed))
                    }
                    Err(PlayerTradeError::NoRoom) => {
                        ResolvedTradeSwap::Refused(CloseReason::NoRoom)
                    }
                    Err(PlayerTradeError::Stale) => ResolvedTradeSwap::Refused(CloseReason::Stale),
                    Err(PlayerTradeError::InvalidInput)
                    | Err(PlayerTradeError::AuthorityRejected)
                    | Err(PlayerTradeError::ConflictingOccurrence)
                    | Err(PlayerTradeError::Audit(_)) => {
                        ResolvedTradeSwap::Refused(CloseReason::Rejected)
                    }
                    // The native commit adapter distinguishes an explicit PostgreSQL
                    // COMMIT rejection from CommitOutcomeUnknown. Only this result proves
                    // rollback here, after the recovery reader proved prior absence.
                    Err(PlayerTradeError::Storage(DurabilityError::CommitRejected)) => {
                        ResolvedTradeSwap::Refused(CloseReason::Rejected)
                    }
                    Err(error @ PlayerTradeError::Storage(_)) => return Err(error.into()),
                }
            }
        };
        self.trades
            .get_mut(&key)
            .and_then(|trade| trade.retained_swap.as_mut())
            .ok_or(OwnerError::Authority)?
            .resolution = Some(resolution);
        Ok(())
    }

    fn terminalize_resolved_transfer(
        &mut self,
        actor: ExactActorRef,
        ingress: &mut CommandIngress,
    ) -> Result<NativeTradeCompletion, OwnerError> {
        let key = self.require_pending_transfer(actor, ingress)?;
        let trade = self.trades.get_mut(&key).ok_or(OwnerError::Authority)?;
        let Some(TradeIssuance::Transferring(issued)) = &trade.issuance else {
            return Err(OwnerError::Authority);
        };
        let resolution = trade
            .retained_swap
            .as_ref()
            .and_then(|retained| retained.resolution.as_ref())
            .ok_or(OwnerError::Authority)?;
        let (observed, disposition, result) = match resolution {
            ResolvedTradeSwap::Committed(committed) => (
                ObservedSwap::Committed,
                "COMPLETED",
                NativeTradeCompletion::Completed(committed.clone()),
            ),
            ResolvedTradeSwap::Refused(reason) => {
                let (observed, disposition) = match reason {
                    CloseReason::NoRoom => (ObservedSwap::NoRoom, "NO_ROOM"),
                    CloseReason::Stale => (ObservedSwap::Stale, "STALE"),
                    CloseReason::Rejected => (ObservedSwap::Rejected, "REJECTED"),
                    CloseReason::Completed | CloseReason::Cancelled => {
                        return Err(OwnerError::Authority);
                    }
                };
                (
                    observed,
                    disposition,
                    NativeTradeCompletion::Refused(*reason),
                )
            }
        };
        let mut closed = trade.session.clone();
        closed.record_outcome_data(&issued.intent.binding, observed)?;
        let outcome = TerminalSemanticOutcome::new(disposition, &trade_hex(&issued.trade_id), 1)?;
        ingress.terminalize(issued.trigger.command_id(), outcome, || {
            trade.session = closed;
        })?;
        // Both actor indexes and owner-local reservation intent disappear only after the
        // native terminal operation atomically installed this known closed state.
        self.prune_closed();
        Ok(result)
    }

    async fn prepare_accept(
        &mut self,
        context: &TradeReadContext<'_, '_, '_>,
        actor: TradeActor,
        pair: CommandRef,
        command: CommandRef,
        now: Instant,
    ) -> Result<PreparedAccept, OwnerError> {
        self.expire(now)?;
        let key = *self
            .by_actor
            .get(&actor.actor.placement_identity())
            .ok_or(OwnerError::Policy(TradeRefusal::Stale))?;
        let trade = self.trades.get(&key).ok_or(OwnerError::Authority)?;
        let side = if trade.actors[0] == actor {
            Side::First
        } else if trade.actors[1] == actor {
            Side::Second
        } else {
            return Err(OwnerError::Authority);
        };
        if trade.session.pair != Some(pair)
            || command.game_session_id() != actor.fence.game_session_id
        {
            return Err(OwnerError::Policy(TradeRefusal::Stale));
        }
        if trade.session.phase() == Phase::Transferring {
            return Err(OwnerError::Policy(TradeRefusal::Transferring));
        }
        let actors = trade.actors;
        let needs_intent = trade.session.accepts[1 - side.index()].is_some();
        let expected = trade.session.data.clone(); // expected before-binding DATA only
        let items = trade.session.data.offers.each_ref().map(|offer| {
            offer
                .as_ref()
                .and_then(|o| o.state.item_instance_id.as_slice().try_into().ok())
        });
        let current = self.read(context, actors, items).await;
        let destinations = match &current {
            Ok(data) if needs_intent && data == &expected => {
                match Self::destination_intent(context, data) {
                    Ok(addresses) => Some(addresses),
                    Err(error) => {
                        if matches!(error, OwnerError::NoRoom) {
                            self.trades
                                .get_mut(&key)
                                .ok_or(OwnerError::Authority)?
                                .session
                                .phase = Phase::Closed(CloseReason::NoRoom);
                            self.prune_closed();
                        }
                        return Err(error);
                    }
                }
            }
            _ => None,
        };
        let ms = self.millis(now)?;
        let trade = self.trades.get_mut(&key).ok_or(OwnerError::Authority)?;
        let mut session = trade.session.clone();
        match current {
            Ok(current) => {
                if let Err(error) =
                    session.record_revalidated_accept(side, pair, command, &current, ms)
                {
                    if matches!(session.phase(), Phase::Closed(_)) {
                        trade.session = session;
                    }
                    return Err(error.into());
                }
            }
            Err(error) => {
                let _ = trade.session.invalidate();
                self.prune_closed();
                return Err(error);
            }
        }
        let intent = if let Some(destinations) = destinations {
            let binding = session.transfer().ok_or(OwnerError::Authority)?.clone();
            Some(ReservationIntent {
                binding,
                destinations,
            })
        } else {
            None
        };
        Ok(PreparedAccept {
            key,
            side,
            session,
            intent,
        })
    }
    fn destination_intent(
        context: &TradeReadContext<'_, '_, '_>,
        data: &TradeData,
    ) -> Result<[crate::durability::item_transfer::ContainerEntryPosition; 2], OwnerError> {
        let mut slots = Vec::with_capacity(2);
        for backpack in &data.backpacks {
            let native = context
                .generation
                .native_gameplay()
                .ok_or(OwnerError::NotTradeable)?;
            let record = native
                .item_policy(
                    &backpack.backpack.definition.production_key,
                    &backpack.backpack.definition.revision_ref,
                )
                .ok_or(OwnerError::NotTradeable)?
                .record();
            if record.production_definition.family != backpack.backpack.definition.family
                || record.production_definition.production_key
                    != backpack.backpack.definition.production_key
                || record.production_definition.revision_ref
                    != backpack.backpack.definition.revision_ref
            {
                return Err(OwnerError::NoRoom);
            }
            let ReferenceItemField::Known(container) = &record.semantics.container else {
                return Err(OwnerError::NoRoom);
            };
            let ReferenceItemField::Known(capacity) = container.capacity else {
                return Err(OwnerError::NoRoom);
            };
            if backpack.entries.len() >= usize::from(capacity) {
                return Err(OwnerError::NoRoom);
            }
            let ordinal = backpack
                .entries
                .iter()
                .map(|entry| entry.placement_ordinal)
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or(OwnerError::NoRoom)?;
            slots.push(crate::durability::item_transfer::ContainerEntryPosition {
                parent_item_instance_id: backpack.backpack.item_instance_id,
                placement_ordinal: ordinal,
            });
        }
        slots.try_into().map_err(|_| OwnerError::Authority)
    }
    /// Must be called by actual issued/committed physical consumers at integration. Before
    /// transfer closes; during transfer returns Transferring and retains intent data/index.
    pub(crate) fn invalidate(&mut self, actor: ExactActorRef) -> Result<(), OwnerError> {
        let key = *self
            .by_actor
            .get(&actor.placement_identity())
            .ok_or(OwnerError::Policy(TradeRefusal::Stale))?;
        let result = self
            .trades
            .get_mut(&key)
            .ok_or(OwnerError::Authority)?
            .session
            .invalidate();
        self.prune_closed();
        result.map_err(OwnerError::from)
    }
    pub(crate) fn invalidate_item(&mut self, item: [u8; 16]) -> Result<(), OwnerError> {
        for trade in self.trades.values() {
            let touches = trade.session.data.backpacks.iter().any(|bag| {
                bag.backpack.item_instance_id == item
                    || bag
                        .entries
                        .iter()
                        .any(|entry| entry.item.item_instance_id == item)
            });
            if touches && trade.session.phase() == Phase::Transferring {
                return Err(OwnerError::Policy(TradeRefusal::Transferring));
            }
        }
        for trade in self.trades.values_mut() {
            if trade.session.data.backpacks.iter().any(|bag| {
                bag.backpack.item_instance_id == item
                    || bag
                        .entries
                        .iter()
                        .any(|entry| entry.item.item_instance_id == item)
            }) {
                trade.session.invalidate()?;
            }
        }
        self.prune_closed();
        Ok(())
    }
    pub(crate) fn reconnect(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Result<(), OwnerError> {
        let key = *self
            .by_actor
            .get(&actor.placement_identity())
            .ok_or(OwnerError::Policy(TradeRefusal::Stale))?;
        let trade = self.trades.get_mut(&key).ok_or(OwnerError::Authority)?;
        let side = if trade.actors[0].actor == actor {
            Side::First
        } else {
            Side::Second
        };
        let result = trade.session.reconnect(side, session);
        self.prune_closed();
        result.map_err(OwnerError::from)
    }
    /// Unknown retains the book/index and intent. This marker never accepts a terminal DATA
    /// enum; only drive_transfer consumes the native writer/reconciler result to close it.
    pub(crate) fn note_ambiguous_outcome(
        &mut self,
        actor: ExactActorRef,
        binding: &TransferBinding,
    ) -> Result<(), OwnerError> {
        let key = *self
            .by_actor
            .get(&actor.placement_identity())
            .ok_or(OwnerError::Policy(TradeRefusal::Stale))?;
        self.trades
            .get_mut(&key)
            .ok_or(OwnerError::Authority)?
            .session
            .record_outcome_data(binding, ObservedSwap::Unknown)?;
        Ok(())
    }
}

/// This operation borrows the real native owners and returns no reusable proof token.
fn qualify_runtime_pair(
    runtime: &ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    objects: &LocalObjectRuntime,
    actors: [TradeActor; 2],
) -> Result<(), OwnerError> {
    runtime.owner_fence().map_err(|_| OwnerError::Authority)?;
    let binding = runtime.binding();
    let mut positions = Vec::with_capacity(2);
    for named in actors {
        if named.actor.world_id() != binding.world_id()
            || named.actor.channel_id() != binding.channel_id()
            || named.actor.scope_generation() != binding.scope_generation()
            || named.fence.runtime_scope
                != RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
            || named.fence.scope_ownership_generation != binding.scope_generation()
        {
            return Err(OwnerError::Authority);
        }
        let control = runtime
            .player_control_facts(named.actor, named.fence.game_session_id)
            .map_err(|_| OwnerError::Authority)?;
        let lease = runtime
            .bound_attacker_lease(named.actor, named.fence.game_session_id)
            .map_err(|_| OwnerError::Authority)?;
        if control.control_loss.is_some()
            || lease.character_id().as_bytes() != named.fence.character_id.as_bytes()
            || lease.generation() != named.fence.character_lease_generation
        {
            return Err(OwnerError::Authority);
        }
        positions.push(
            runtime
                .read_actor_position(named.actor)
                .map_err(|_| OwnerError::Authority)?
                .position(),
        );
    }
    let [a, b] = [positions[0], positions[1]];
    if a.floor != b.floor || a.x.abs_diff(b.x).max(a.y.abs_diff(b.y)) > 2 {
        return Err(OwnerError::TooFar);
    }
    // Reuse the actual visibility engine's bounded/canonical query over the current
    // carrier census, including cutoff competition from creatures and NPCs.
    let visible = runtime.visible_entities();
    let mut index = InterestIndex::new();
    for actor in visible
        .players
        .iter()
        .chain(&visible.creatures)
        .chain(&visible.npcs)
    {
        if let Ok(position) =
            VisibilityPosition::new(actor.position.x, actor.position.y, actor.position.floor)
        {
            index.upsert(InterestEntity {
                identity: actor.actor.placement_identity(),
                position,
                revision: actor.revision,
            });
        }
    }
    for side in [0, 1] {
        let query = index
            .query(
                &actors[side].actor.placement_identity(),
                VisibilitySettings::REFERENCE,
            )
            .map_err(|_| OwnerError::NotVisible)?;
        if !query
            .entities()
            .iter()
            .any(|entity| entity.identity == actors[1 - side].actor.placement_identity())
        {
            return Err(OwnerError::NotVisible);
        }
    }
    for (position, exempt) in [a, b]
        .into_iter()
        .map(|position| (position, true))
        .chain(source_sight_steps(a, b).map_err(|_| OwnerError::UnknownMap)?)
    {
        let tile = qualified_combat_tile(room, runtime, objects, position)
            .map_err(|_| OwnerError::UnknownMap)?;
        if !tile.ground_present() || (!exempt && tile.block_projectile()) {
            return Err(OwnerError::UnknownMap);
        }
    }
    Ok(())
}

// Real runtime issuance and Content are available only in this owning module.
// These are the original native writer bodies, not a durability test-root shim.
pub(crate) use native_writer::{FrozenPlayerTrade, PlayerTradeCommitAuthority};
mod native_writer {
    use super::{IssuedTradeOccurrence, ReservationIntent};
    use crate::content::{ActiveGeneration, ReferenceItemField};
    use crate::domain::CharacterId;
    use crate::durability::character_authority::{
        ReconciledCharacterAuthority, assert_recovery_fence, uuid_text,
    };
    use crate::durability::db::{
        begin_semantic_transaction, begin_type2_transaction, commit_semantic_transaction,
        lock_admission_relations,
    };
    use crate::durability::item_mint::CORPSE_MATERIALIZATION_PURPOSE_KEY;
    use crate::durability::item_mint_audit as audit;
    use crate::durability::item_timed_state::load_timed_state;
    use crate::durability::item_transfer::{
        ContainerEntryPosition, CurrentCharacterItemFence, character_item_fence_without_root_lock,
        load_entries, load_slot, lock_character_item_root,
    };
    use crate::durability::player_trade::{
        CommittedPlayerTrade, OneItemPlayerTradeLineV1, OneItemPlayerTradeLocationV1,
        OneItemPlayerTradeParticipantV1, OneItemPlayerTradeV1, PLAYER_TRADE_CAUSE,
        PlayerTradeError, PlayerTradeEventIdentity, PlayerTradeOutcome, PlayerTradeReceipt,
        TradeData, audit_command, audit_timed, check_player_trade, encode_player_trade_event,
        first_fence_continues,
    };
    use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
    use crate::durability::{DurabilityError, DurabilityRoot};
    use crate::foundation::{
        ChannelRuntimeAssignmentBinding, ChannelRuntimeV1, CommandRef, RuntimeScopeRefV1,
    };
    use prost::Message;
    use sha2::{Digest, Sha256};
    use sqlx::Row;

    /// Immutable retry input. Only a real private component issuance can supply the
    /// captured before-data; no constructor accepts an arbitrary ReservationIntent.
    /// This binds the component intent, not the uninstalled session ingress owner.
    #[derive(Debug, Clone)]
    pub(crate) struct FrozenPlayerTrade {
        trade_id: [u8; 16],
        occurrence: [u8; 16],
        first_accept: CommandRef,
        first_fence: CurrentCharacterItemFence,
        trigger: CommandRef,
        intent: ReservationIntent,
        identity: PlayerTradeEventIdentity,
        runtime_binding: ChannelRuntimeAssignmentBinding,
        content_digest: [u8; 32],
        capacities: [u16; 2],
        binding: [u8; 32],
    }

    fn ordered_characters(data: &TradeData) -> [usize; 2] {
        if data.traders[0].fence.character_id.as_bytes()
            < data.traders[1].fence.character_id.as_bytes()
        {
            [0, 1]
        } else {
            [1, 0]
        }
    }

    impl FrozenPlayerTrade {
        pub(crate) fn from_issued(
            issued: &IssuedTradeOccurrence,
            runtime: &ChannelRuntimeV1,
            generation: &ActiveGeneration,
            identity: PlayerTradeEventIdentity,
        ) -> Result<Self, PlayerTradeError> {
            runtime
                .owner_fence()
                .map_err(|_| PlayerTradeError::AuthorityRejected)?;
            let digest = runtime.content_pin().server_artifact_digest();
            if digest == [0; 32] || generation.identity().server_artifact_digest() != digest {
                return Err(PlayerTradeError::AuthorityRejected);
            }
            audit::check_uuid_v7(&identity.transaction_id)?;
            audit::check_uuid_v7(&identity.event_id)?;
            if identity.transaction_id == identity.event_id
                || identity.occurred_at_unix_ms <= 0
                || identity
                    .occurred_at_unix_ms
                    .checked_add(audit::AUDIT_RETENTION_P90D_MS)
                    .is_none()
            {
                return Err(PlayerTradeError::InvalidInput);
            }
            let intent = issued.intent().clone();
            let data = &intent.binding.data;
            let [a, b] = data.traders;
            let first_side = data
                .traders
                .iter()
                .position(|p| p.fence == issued.first_fence())
                .ok_or(PlayerTradeError::InvalidInput)?;
            let RuntimeScopeRefV1::Channel {
                world_id,
                channel_id,
            } = a.fence.runtime_scope
            else {
                return Err(PlayerTradeError::InvalidInput);
            };
            if a.fence.character_id == b.fence.character_id
                || a.account_id == b.account_id
                || a.fence.game_session_id == b.fence.game_session_id
                || a.fence.runtime_scope != b.fence.runtime_scope
                || a.fence.scope_ownership_generation != b.fence.scope_ownership_generation
                || runtime.binding().world_id() != world_id
                || runtime.binding().channel_id() != channel_id
                || runtime.binding().scope_generation() != a.fence.scope_ownership_generation
                || intent.binding.accepts[first_side] != issued.first_accept()
                || intent.binding.accepts[1 - first_side] != issued.trigger()
                || intent.binding.trigger != issued.trigger()
                || data.backpacks[0].backpack.item_instance_id
                    == data.backpacks[1].backpack.item_instance_id
            {
                return Err(PlayerTradeError::InvalidInput);
            }
            let native = generation
                .native_gameplay()
                .ok_or(PlayerTradeError::InvalidInput)?;
            let mut capacities = [0_u16; 2];
            for side in [0, 1] {
                let trader = data.traders[side];
                let bag = &data.backpacks[side];
                let offer = data.offers[side]
                    .as_ref()
                    .ok_or(PlayerTradeError::InvalidInput)?;
                if intent.binding.accepts[side].game_session_id() != trader.fence.game_session_id
                    || !offer.matches_backpack(bag, world_id.as_bytes())
                    || offer.state_revision == 0
                    || offer.state_revision == u64::MAX
                    || bag.entries.len() > 20
                    || data.backpack_versions[side].len() != bag.entries.len() + 1
                    || intent.destinations[side].parent_item_instance_id
                        != bag.backpack.item_instance_id
                    || bag
                        .entries
                        .iter()
                        .map(|e| e.placement_ordinal)
                        .max()
                        .unwrap_or(0)
                        .checked_add(1)
                        != Some(intent.destinations[side].placement_ordinal)
                {
                    return Err(PlayerTradeError::InvalidInput);
                }
                let policy = native
                    .item_policy(
                        &bag.backpack.definition.production_key,
                        &bag.backpack.definition.revision_ref,
                    )
                    .ok_or(PlayerTradeError::InvalidInput)?
                    .record();
                if policy.production_definition.family != bag.backpack.definition.family
                    || policy.production_definition.production_key
                        != bag.backpack.definition.production_key
                    || policy.production_definition.revision_ref
                        != bag.backpack.definition.revision_ref
                {
                    return Err(PlayerTradeError::InvalidInput);
                }
                let ReferenceItemField::Known(container) = &policy.semantics.container else {
                    return Err(PlayerTradeError::InvalidInput);
                };
                let ReferenceItemField::Known(capacity) = container.capacity else {
                    return Err(PlayerTradeError::InvalidInput);
                };
                if !(1..=20).contains(&capacity) || bag.entries.len() >= usize::from(capacity) {
                    return Err(PlayerTradeError::NoRoom);
                }
                capacities[side] = capacity;
                let definition = offer
                    .state
                    .definition
                    .as_ref()
                    .ok_or(PlayerTradeError::InvalidInput)?;
                let policy = native
                    .item_policy(&definition.production_key, &definition.revision_ref)
                    .ok_or(PlayerTradeError::InvalidInput)?
                    .record();
                if policy.production_definition.family != definition.family
                    || policy.production_definition.production_key != definition.production_key
                    || policy.production_definition.revision_ref != definition.revision_ref
                {
                    return Err(PlayerTradeError::InvalidInput);
                }
                let ReferenceItemField::Known(physical) = &policy.semantics.physical else {
                    return Err(PlayerTradeError::InvalidInput);
                };
                if physical.pickupable != ReferenceItemField::Known(true) {
                    return Err(PlayerTradeError::InvalidInput);
                }
                match &policy.semantics.trade_restrictions {
                    ReferenceItemField::Unknown => {}
                    ReferenceItemField::Known(flags)
                        if crate::domain::player_trade::tradeable_flag_allows(flags) => {}
                    _ => return Err(PlayerTradeError::InvalidInput),
                }
                for (member, version) in std::iter::once(&bag.backpack)
                    .chain(bag.entries.iter().map(|e| &e.item))
                    .zip(&data.backpack_versions[side])
                {
                    if member.item_instance_id != version.item || version.state_revision == 0 {
                        return Err(PlayerTradeError::InvalidInput);
                    }
                }
            }
            let mut frozen = Self {
                trade_id: *issued.trade_id(),
                occurrence: *issued.occurrence(),
                first_accept: issued.first_accept(),
                first_fence: issued.first_fence(),
                trigger: issued.trigger(),
                intent,
                identity,
                runtime_binding: runtime.binding(),
                content_digest: digest,
                capacities,
                binding: [0; 32],
            };
            let message = frozen.message(frozen.first_fence.connection_generation.get())?;
            check_player_trade(&message)?;
            frozen.binding = frozen.request_binding(&message);
            Ok(frozen)
        }

        fn first_side(&self) -> Result<usize, PlayerTradeError> {
            self.intent
                .binding
                .data
                .traders
                .iter()
                .position(|p| p.fence == self.first_fence)
                .ok_or(PlayerTradeError::InvalidInput)
        }

        fn message(
            &self,
            current_first_connection: u64,
        ) -> Result<OneItemPlayerTradeV1, PlayerTradeError> {
            let data = &self.intent.binding.data;
            let first_side = self.first_side()?;
            let mut participants = Vec::with_capacity(2);
            for side in ordered_characters(data) {
                let trader = data.traders[side];
                participants.push(OneItemPlayerTradeParticipantV1 {
                    character_id: trader.fence.character_id.as_bytes().to_vec(),
                    account_id: trader.account_id.as_bytes().to_vec(),
                    command: Some(audit_command(self.intent.binding.accepts[side])),
                    accepted_connection_generation: trader.fence.connection_generation.get(),
                    current_connection_generation: if side == first_side {
                        current_first_connection
                    } else {
                        trader.fence.connection_generation.get()
                    },
                    character_lease_generation: trader.fence.character_lease_generation,
                });
            }
            let mut lines = Vec::with_capacity(2);
            for side in [0, 1] {
                let offer = data.offers[side]
                    .as_ref()
                    .ok_or(PlayerTradeError::InvalidInput)?;
                let destination = self.intent.destinations[1 - side];
                lines.push(OneItemPlayerTradeLineV1 {
                    before: Some(offer.state.clone()),
                    after: Some(offer.state.clone()),
                    state_revision_before: offer.state_revision,
                    state_revision_after: offer
                        .state_revision
                        .checked_add(1)
                        .ok_or(PlayerTradeError::Stale)?,
                    source: Some(OneItemPlayerTradeLocationV1 {
                        character_id: data.traders[side].fence.character_id.as_bytes().to_vec(),
                        parent_item_instance_id: offer.position.parent_item_instance_id.to_vec(),
                        placement_ordinal: offer.position.placement_ordinal,
                        placed_transaction_id: offer.placed_transaction_id.to_vec(),
                    }),
                    destination: Some(OneItemPlayerTradeLocationV1 {
                        character_id: data.traders[1 - side]
                            .fence
                            .character_id
                            .as_bytes()
                            .to_vec(),
                        parent_item_instance_id: destination.parent_item_instance_id.to_vec(),
                        placement_ordinal: destination.placement_ordinal,
                        placed_transaction_id: self.identity.transaction_id.to_vec(),
                    }),
                    unchanged_timed_state: audit_timed(offer.timed),
                });
            }
            lines.sort_by(|a, b| {
                a.before
                    .as_ref()
                    .map(|s| &s.item_instance_id)
                    .cmp(&b.before.as_ref().map(|s| &s.item_instance_id))
            });
            Ok(OneItemPlayerTradeV1 {
                typed_cause: PLAYER_TRADE_CAUSE.into(),
                trade_id: self.trade_id.to_vec(),
                occurrence_id: self.occurrence.to_vec(),
                first_accept: Some(audit_command(self.first_accept)),
                trigger: Some(audit_command(self.trigger)),
                world_id: self.runtime_binding.world_id().as_bytes().to_vec(),
                channel_id: self.runtime_binding.channel_id().as_bytes().to_vec(),
                runtime_scope_ownership_generation: self.runtime_binding.scope_generation().get(),
                content_digest: self.content_digest.to_vec(),
                participants,
                lines,
            })
        }

        fn request_binding(&self, baseline: &OneItemPlayerTradeV1) -> [u8; 32] {
            fn part(hash: &mut Sha256, bytes: &[u8]) {
                hash.update((bytes.len() as u64).to_be_bytes());
                hash.update(bytes);
            }
            let mut hash = Sha256::new();
            part(&mut hash, b"OTERYN_PLAYER_TRADE_REQUEST/v1");
            part(&mut hash, &baseline.encode_to_vec());
            part(
                &mut hash,
                self.intent.binding.pair.game_session_id().as_bytes(),
            );
            hash.update(self.intent.binding.pair.command_id().get().to_be_bytes());
            hash.update(self.identity.transaction_id);
            hash.update(self.identity.event_id);
            hash.update(self.identity.occurred_at_unix_ms.to_be_bytes());
            hash.update(self.runtime_binding.source_revision().to_be_bytes());
            // All captured backpack state participates in the retry binding. An
            // unrelated item/checkpoint change invalidates this open trade too.
            for side in [0, 1] {
                hash.update(self.capacities[side].to_be_bytes());
                let bag = &self.intent.binding.data.backpacks[side];
                hash.update((bag.entries.len() as u64).to_be_bytes());
                hash.update(
                    (self.intent.binding.data.backpack_versions[side].len() as u64).to_be_bytes(),
                );
                for member in
                    std::iter::once(&bag.backpack).chain(bag.entries.iter().map(|e| &e.item))
                {
                    part(&mut hash, &member.item_instance_id);
                    part(&mut hash, member.definition.family.as_bytes());
                    part(&mut hash, member.definition.production_key.as_bytes());
                    part(&mut hash, member.definition.revision_ref.as_bytes());
                    hash.update(member.quantity.to_be_bytes());
                }
                for entry in &bag.entries {
                    hash.update(entry.placement_ordinal.to_be_bytes());
                }
                for version in &self.intent.binding.data.backpack_versions[side] {
                    hash.update(version.item);
                    hash.update(version.state_revision.to_be_bytes());
                    hash.update(version.placed_transaction_id);
                    hash.update(version.key_number.to_be_bytes());
                    match audit_timed(version.timed) {
                        Some(timed) => {
                            hash.update([1]);
                            part(&mut hash, &timed.encode_to_vec());
                        }
                        None => hash.update([0]),
                    }
                }
            }
            hash.finalize().into()
        }
    }

    /// Actual borrowed owners are required on every new attempt. The first accept
    /// may resume its same GameSession at a higher connection generation; all other
    /// captured first-side fence components and the pending trigger remain pinned.
    pub(crate) struct PlayerTradeCommitAuthority<'a, 'f, 's> {
        pub(crate) recovery: &'a ReconciledCharacterAuthority<'f, 's>,
        pub(crate) node: &'a NodeIncarnationProof,
        pub(crate) runtime: &'a ChannelRuntimeV1,
        /// Borrowed from the still-TRANSFERRING native component for the entire attempt.
        pub(crate) issued: &'a IssuedTradeOccurrence,
        pub(crate) first_current_fence: CurrentCharacterItemFence,
    }

    async fn lock_player_trade_cause(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        request: &FrozenPlayerTrade,
    ) -> Result<(), DurabilityError> {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:player-trade:' || encode($1,'hex'),0))")
            .bind(request.occurrence.as_slice()).execute(&mut **tx).await?;
        Ok(())
    }

    fn number(row: &sqlx::postgres::PgRow, column: &str) -> Result<u64, DurabilityError> {
        row.try_get::<String, _>(column)?
            .parse::<u64>()
            .map_err(|_| DurabilityError::InvalidStoredState)
    }

    async fn load_player_trade(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        request: &FrozenPlayerTrade,
    ) -> Result<Option<Result<CommittedPlayerTrade, PlayerTradeError>>, DurabilityError> {
        let Some(row)=sqlx::query("SELECT trade_id::text,transaction_id::text,event_id::text,request_binding FROM game_player_trades WHERE occurrence_id=encode($1,'hex')::uuid")
            .bind(request.occurrence.as_slice()).fetch_optional(&mut **tx).await? else{return Ok(None);};
        if row.try_get::<Vec<u8>, _>("request_binding")? != request.binding {
            return Ok(Some(Err(PlayerTradeError::ConflictingOccurrence)));
        }
        let lines=sqlx::query("SELECT item_instance_id::text,source_character_id::text,destination_character_id::text,destination_parent_item_instance_id::text,destination_ordinal::text FROM game_player_trade_receipts WHERE occurrence_id=encode($1,'hex')::uuid ORDER BY line_ordinal LIMIT 3")
            .bind(request.occurrence.as_slice()).fetch_all(&mut **tx).await?;
        if lines.len() != 2 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let receipts = lines
            .iter()
            .map(|line| {
                Ok(PlayerTradeReceipt {
                    item_instance_id: uuid_text(line.try_get("item_instance_id")?)?,
                    source_character_id: CharacterId::from_bytes(uuid_text(
                        line.try_get("source_character_id")?,
                    )?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                    destination_character_id: CharacterId::from_bytes(uuid_text(
                        line.try_get("destination_character_id")?,
                    )?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                    destination: ContainerEntryPosition {
                        parent_item_instance_id: uuid_text(
                            line.try_get("destination_parent_item_instance_id")?,
                        )?,
                        placement_ordinal: number(line, "destination_ordinal")?,
                    },
                })
            })
            .collect::<Result<Vec<_>, DurabilityError>>()?;
        let receipts: [PlayerTradeReceipt; 2] = receipts
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        if receipts[0].item_instance_id >= receipts[1].item_instance_id
            || receipts.iter().any(|line| {
                line.source_character_id == line.destination_character_id
                    || line.destination.placement_ordinal == 0
            })
            || receipts[0].source_character_id != receipts[1].destination_character_id
            || receipts[1].source_character_id != receipts[0].destination_character_id
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(Some(Ok(CommittedPlayerTrade {
            occurrence: request.occurrence,
            trade_id: uuid_text(row.try_get("trade_id")?)?,
            transaction_id: uuid_text(row.try_get("transaction_id")?)?,
            event_id: uuid_text(row.try_get("event_id")?)?,
            receipts,
        })))
    }

    async fn trade_identity_reused(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        request: &FrozenPlayerTrade,
    ) -> Result<bool, DurabilityError> {
        Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_player_trades WHERE transaction_id=encode($1,'hex')::uuid OR event_id=encode($2,'hex')::uuid OR trade_id=encode($3,'hex')::uuid) OR EXISTS(SELECT 1 FROM game_player_trade_participants WHERE (game_session_id=encode($4,'hex')::uuid AND command_id=$5::text::numeric) OR (game_session_id=encode($6,'hex')::uuid AND command_id=$7::text::numeric)) OR EXISTS(SELECT 1 FROM game_item_audit_outbox WHERE transaction_id=encode($1,'hex')::uuid OR event_id=encode($2,'hex')::uuid) OR EXISTS(SELECT 1 FROM game_item_instances WHERE minted_transaction_id=encode($1,'hex')::uuid)")
            .bind(request.identity.transaction_id.as_slice()).bind(request.identity.event_id.as_slice()).bind(request.trade_id.as_slice())
            .bind(request.first_accept.game_session_id().as_bytes().as_slice()).bind(request.first_accept.command_id().get().to_string())
            .bind(request.trigger.game_session_id().as_bytes().as_slice()).bind(request.trigger.command_id().get().to_string())
            .fetch_one(&mut **tx).await?)
    }

    async fn current_trade_content_and_assignment(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        node: &NodeIncarnationProof,
        request: &FrozenPlayerTrade,
    ) -> Result<bool, DurabilityError> {
        let binding = request.runtime_binding;
        let world = binding.world_id();
        let channel = binding.channel_id();
        let key = crate::durability::runtime_scope_assignment::scope_key(world, channel);
        let Some(assignment) =
            crate::durability::runtime_scope_assignment::load_assignment(tx, &key, false).await?
        else {
            return Ok(false);
        };
        let Some(holder) = assignment.holder else {
            return Ok(false);
        };
        let RuntimeScopeRefV1::Channel {
            world_id: actual_world,
            channel_id: actual_channel,
        } = assignment.scope
        else {
            return Ok(false);
        };
        if assignment.state
            != crate::durability::runtime_scope_assignment::AssignmentState::Assigned
            || holder != node.fact()
            || !binding.matches_committed_assignment(
                actual_world,
                actual_channel,
                holder.node_id(),
                holder.registration_revision(),
                assignment.ownership_generation,
                assignment.source_revision,
                &assignment.decision_identity,
            )
        {
            return Ok(false);
        }
        let digest:Option<Vec<u8>>=sqlx::query_scalar("SELECT server_artifact_digest FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid ORDER BY activation_sequence DESC LIMIT 1")
            .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
        Ok(digest.as_deref() == Some(request.content_digest.as_slice()))
    }

    /// Under both current Character roots, both offered item locks and both ordered
    /// slot locks, re-read every captured member/version. No old read flag or client
    /// location is accepted as the current physical source.
    async fn trade_before_refusal(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        request: &FrozenPlayerTrade,
    ) -> Result<Option<PlayerTradeError>, DurabilityError> {
        let data = &request.intent.binding.data;
        let world = request.runtime_binding.world_id();
        for side in [0, 1] {
            let Some(bag) = load_slot(tx, data.traders[side].fence.character_id).await? else {
                return Ok(Some(PlayerTradeError::Stale));
            };
            if bag != data.backpacks[side].backpack {
                return Ok(Some(PlayerTradeError::Stale));
            }
            let entries = load_entries(tx, bag.item_instance_id).await?;
            if entries.len() >= usize::from(request.capacities[side]) {
                return Ok(Some(PlayerTradeError::NoRoom));
            }
            if entries != data.backpacks[side].entries {
                return Ok(Some(PlayerTradeError::Stale));
            }
            for version in &data.backpack_versions[side] {
                let row=sqlx::query("SELECT i.world_id::text,i.lifecycle,i.state_revision::text,COALESCE(k.key_number,0)::integer AS key_number,COALESCE(s.placed_transaction_id,e.placed_transaction_id)::text AS placed_transaction_id,(s.item_instance_id IS NOT NULL) AS in_slot,(e.item_instance_id IS NOT NULL) AS in_entry FROM game_item_instances i LEFT JOIN game_item_keys k USING(item_instance_id) LEFT JOIN game_item_container_slots s ON s.item_instance_id=i.item_instance_id AND s.character_id=encode($2,'hex')::uuid LEFT JOIN game_item_container_entries e ON e.item_instance_id=i.item_instance_id AND e.character_id=encode($2,'hex')::uuid AND e.parent_item_instance_id=encode($3,'hex')::uuid WHERE i.item_instance_id=encode($1,'hex')::uuid")
                    .bind(version.item.as_slice()).bind(data.traders[side].fence.character_id.as_bytes().as_slice()).bind(bag.item_instance_id.as_slice()).fetch_optional(&mut **tx).await?;
                let Some(row) = row else {
                    return Ok(Some(PlayerTradeError::Stale));
                };
                if row.try_get::<bool, _>("in_slot")? == row.try_get::<bool, _>("in_entry")?
                    || row.try_get::<i16, _>("lifecycle")? != 1
                    || uuid_text(row.try_get("world_id")?)? != *world.as_bytes()
                    || number(&row, "state_revision")? != version.state_revision
                    || uuid_text(row.try_get("placed_transaction_id")?)?
                        != version.placed_transaction_id
                    || u32::try_from(row.try_get::<i32, _>("key_number")?)
                        .map_err(|_| DurabilityError::InvalidStoredState)?
                        != version.key_number
                    || load_timed_state(tx, &version.item)
                        .await
                        .map_err(|_| DurabilityError::Unavailable)?
                        != version.timed
                {
                    return Ok(Some(PlayerTradeError::Stale));
                }
            }
            let offer = data.offers[side]
                .as_ref()
                .ok_or(DurabilityError::InvalidStoredState)?;
            let item: [u8; 16] = offer
                .state
                .item_instance_id
                .as_slice()
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            let forbidden:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid) OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid) OR EXISTS(SELECT 1 FROM game_item_mint_receipts WHERE item_instance_id=encode($1,'hex')::uuid AND loot_purpose_key=$2) OR EXISTS(SELECT 1 FROM game_character_equipment_slots WHERE item_instance_id=encode($1,'hex')::uuid)")
                .bind(item.as_slice()).bind(CORPSE_MATERIALIZATION_PURPOSE_KEY).fetch_one(&mut **tx).await?;
            if forbidden {
                return Ok(Some(PlayerTradeError::Stale));
            }
        }
        Ok(None)
    }

    impl DurabilityRoot {
        pub(crate) async fn reconcile_player_trade(
            &self,
            recovery: &ReconciledCharacterAuthority<'_, '_>,
            request: &FrozenPlayerTrade,
        ) -> Result<Option<CommittedPlayerTrade>, PlayerTradeError> {
            let recovery = recovery
                .record_for(self)
                .map_err(|_| PlayerTradeError::AuthorityRejected)?;
            let request = request.clone();
            self.try_issue_semantic_pass()?
                .run(move |holder, deadline| {
                    Box::pin(async move {
                        let mut tx = begin_semantic_transaction(holder, deadline).await?;
                        assert_recovery_fence(&mut tx, &recovery).await?;
                        lock_admission_relations(&mut tx).await?;
                        lock_player_trade_cause(&mut tx, &request).await?;
                        let outcome = match load_player_trade(&mut tx, &request).await? {
                            None => Ok(None),
                            Some(Ok(committed)) => Ok(Some(committed)),
                            Some(Err(error)) => Err(error),
                        };
                        commit_semantic_transaction(tx, deadline).await?;
                        Ok(outcome)
                    })
                })
                .await?
        }

        /// One native item-only transaction: both whole source entries become new
        /// reciprocal entries, never a merge and never using a just-freed source
        /// slot to pass capacity. No CharacterRevision is advanced.
        pub(crate) async fn commit_player_trade(
            &self,
            authority: PlayerTradeCommitAuthority<'_, '_, '_>,
            request: &FrozenPlayerTrade,
        ) -> Result<PlayerTradeOutcome, PlayerTradeError> {
            let recovery = authority
                .recovery
                .record_for(self)
                .map_err(|_| PlayerTradeError::AuthorityRejected)?;
            let node = authority.node.clone();
            let first_current = authority.first_current_fence;
            // Evaluated from the actual borrowed runtime, and applied only after
            // historical outcome lookup. A replay requires recovery, not a live actor.
            let runtime_matches = authority.runtime.owner_fence().is_ok()
                && authority.runtime.binding() == request.runtime_binding
                && authority.runtime.content_pin().server_artifact_digest()
                    == request.content_digest;
            let issuance_matches = authority.issued.trade_id() == &request.trade_id
                && authority.issued.occurrence() == &request.occurrence
                && authority.issued.first_accept() == request.first_accept
                && authority.issued.first_fence() == request.first_fence
                && authority.issued.trigger() == request.trigger
                && authority.issued.intent() == &request.intent;
            let request = request.clone();
            self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
                let mut tx=begin_type2_transaction(holder,deadline).await?;
                assert_recovery_fence(&mut tx,&recovery).await?;
                lock_admission_relations(&mut tx).await?;
                lock_player_trade_cause(&mut tx,&request).await?;
                if let Some(committed)=load_player_trade(&mut tx,&request).await? {
                    let committed=match committed {Ok(value)=>value,Err(error)=>return Ok(Err(error))};
                    tx.commit(deadline).await?;
                    return Ok(Ok(PlayerTradeOutcome::AlreadyCommitted(committed)));
                }
                if trade_identity_reused(&mut tx,&request).await? {return Ok(Err(PlayerTradeError::ConflictingOccurrence));}
                if !runtime_matches || !issuance_matches || !first_fence_continues(request.first_fence,first_current) {
                    return Ok(Err(PlayerTradeError::AuthorityRejected));
                }
                let first_side=match request.first_side(){Ok(side)=>side,Err(error)=>return Ok(Err(error))};
                let mut fences=request.intent.binding.data.traders.map(|p|p.fence);
                fences[first_side]=first_current;
                let order=ordered_characters(&request.intent.binding.data);
                let world=request.runtime_binding.world_id();let channel=request.runtime_binding.channel_id();
                // Same native Content activation owner lock as equipment. Acquire
                // before any Character root; later independent digest reads stay pinned.
                sqlx::query("SELECT pg_advisory_xact_lock_shared(hashtextextended('oteryn:content-activation:' || encode($1::bytea,'hex')::uuid::text || ':' || encode($2::bytea,'hex')::uuid::text,0))")
                    .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&mut **tx).await?;
                let mut accounts=[String::new(),String::new()];
                for side in order {
                    let Some(account)=character_item_fence_without_root_lock(&mut tx,&node,&fences[side],request.intent.binding.accepts[side],fences[side].character_id,*world.as_bytes(),*channel.as_bytes()).await?
                        else{return Ok(Err(PlayerTradeError::AuthorityRejected));};
                    if uuid_text(&account)?!=*request.intent.binding.data.traders[side].account_id.as_bytes() {
                        return Ok(Err(PlayerTradeError::AuthorityRejected));
                    }
                    accounts[side]=account;
                }
                if accounts[0]==accounts[1] || !current_trade_content_and_assignment(&mut tx,&node,&request).await? {
                    return Ok(Err(PlayerTradeError::AuthorityRejected));
                }
                let mut root_revisions=[0_u64;2];
                for side in order {
                    if !lock_character_item_root(&mut tx,&fences[side],&accounts[side]).await? {return Ok(Err(PlayerTradeError::AuthorityRejected));}
                    let revision:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                        .bind(fences[side].character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
                    root_revisions[side]=revision.parse().map_err(|_|DurabilityError::InvalidStoredState)?;
                }
                let message=match request.message(first_current.connection_generation.get()){Ok(message)=>message,Err(error)=>return Ok(Err(error))};
                let item_ids=message.lines.iter().map(|line|line.before.as_ref().map(|s|s.item_instance_id.clone()).ok_or(DurabilityError::InvalidStoredState)).collect::<Result<Vec<_>,DurabilityError>>()?;
                // Only the two offered instance rows, ordered by their native IDs.
                let locked=sqlx::query("SELECT item_instance_id FROM game_item_instances WHERE item_instance_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid) ORDER BY item_instance_id FOR UPDATE")
                    .bind(item_ids[0].as_slice()).bind(item_ids[1].as_slice()).fetch_all(&mut **tx).await?;
                if locked.len()!=2{return Ok(Err(PlayerTradeError::Stale));}
                // Runtime deliberately has no UPDATE grant on container slots.
                // The owned bounded SQL function grants only these ordered locks.
                let locked:i16=sqlx::query_scalar("SELECT game_player_trade_lock_slots(encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
                    .bind(fences[order[0]].character_id.as_bytes().as_slice()).bind(fences[order[1]].character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
                if locked!=2{return Ok(Err(PlayerTradeError::Stale));}
                if let Some(refusal)=trade_before_refusal(&mut tx,&request).await? {return Ok(Err(refusal));}
                let tuple=tx.tuple().tuple();
                let envelope=match encode_player_trade_event(request.identity,message.clone(),tuple){Ok(bytes)=>bytes,Err(error)=>return Ok(Err(error.into()))};
                apply_player_trade(&mut tx,&request,&message,&root_revisions,&envelope,tuple).await?;
                // Native deferred SQL guards prove the entire two-line shape. Any
                // failure before or during commit leaves no definitive success.
                let Some(committed)=load_player_trade(&mut tx,&request).await? else{return Err(DurabilityError::InvalidStoredState);};
                let committed=match committed {Ok(value)=>value,Err(error)=>return Ok(Err(error))};
                tx.commit(deadline).await?;
                Ok(Ok(PlayerTradeOutcome::Committed(committed)))
            })).await?
        }
    }

    async fn apply_player_trade(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        request: &FrozenPlayerTrade,
        message: &OneItemPlayerTradeV1,
        root_revisions: &[u64; 2],
        envelope: &[u8],
        tuple: audit::Type2EventTuple,
    ) -> Result<(), DurabilityError> {
        sqlx::query("INSERT INTO game_player_trades(occurrence_id,trade_id,transaction_id,event_id,first_game_session_id,first_command_id,trigger_game_session_id,trigger_command_id,request_binding,world_id,channel_id,runtime_scope_ownership_generation,content_digest,occurred_at,envelope_sha256,committed_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric,encode($7,'hex')::uuid,$8::text::numeric,$9,encode($10,'hex')::uuid,encode($11,'hex')::uuid,$12::text::numeric,$13,$14,sha256($15),floor(extract(epoch FROM clock_timestamp())*1000)::bigint)")
            .bind(request.occurrence.as_slice()).bind(request.trade_id.as_slice()).bind(request.identity.transaction_id.as_slice()).bind(request.identity.event_id.as_slice())
            .bind(request.first_accept.game_session_id().as_bytes().as_slice()).bind(request.first_accept.command_id().get().to_string())
            .bind(request.trigger.game_session_id().as_bytes().as_slice()).bind(request.trigger.command_id().get().to_string()).bind(request.binding.as_slice())
            .bind(message.world_id.as_slice()).bind(message.channel_id.as_slice()).bind(message.runtime_scope_ownership_generation.to_string())
            .bind(request.content_digest.as_slice()).bind(request.identity.occurred_at_unix_ms).bind(envelope).execute(&mut **tx).await?;
        let data = &request.intent.binding.data;
        for (ordinal, side) in ordered_characters(data).into_iter().enumerate() {
            let participant = &message.participants[ordinal];
            let trader = data.traders[side];
            let bag = &data.backpacks[side];
            let bag_version = data.backpack_versions[side]
                .first()
                .ok_or(DurabilityError::InvalidStoredState)?;
            let count = i16::try_from(bag.entries.len())
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            let maximum = bag
                .entries
                .iter()
                .map(|entry| entry.placement_ordinal)
                .max()
                .ok_or(DurabilityError::InvalidStoredState)?;
            sqlx::query("INSERT INTO game_player_trade_participants(occurrence_id,participant_ordinal,character_id,account_id,game_session_id,command_id,connection_generation,current_connection_generation,character_lease_generation,character_revision,backpack_item_instance_id,backpack_placed_transaction_id,backpack_state_revision,backpack_capacity,entry_count_before,max_ordinal_before) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric,$7::text::numeric,$8::text::numeric,$9::text::numeric,$10::text::numeric,encode($11,'hex')::uuid,encode($12,'hex')::uuid,$13::text::numeric,$14,$15,$16::text::numeric)")
                .bind(request.occurrence.as_slice()).bind(i16::try_from(ordinal+1).map_err(|_|DurabilityError::InvalidStoredState)?)
                .bind(trader.fence.character_id.as_bytes().as_slice()).bind(trader.account_id.as_bytes().as_slice())
                .bind(trader.fence.game_session_id.as_bytes().as_slice()).bind(request.intent.binding.accepts[side].command_id().get().to_string())
                .bind(participant.accepted_connection_generation.to_string()).bind(participant.current_connection_generation.to_string())
                .bind(trader.fence.character_lease_generation.to_string()).bind(root_revisions[side].to_string())
                .bind(bag.backpack.item_instance_id.as_slice()).bind(bag_version.placed_transaction_id.as_slice()).bind(bag_version.state_revision.to_string())
                .bind(i16::try_from(request.capacities[side]).map_err(|_|DurabilityError::InvalidStoredState)?).bind(count).bind(maximum.to_string())
                .execute(&mut **tx).await?;
        }
        for (ordinal, line) in message.lines.iter().enumerate() {
            let state = line
                .before
                .as_ref()
                .ok_or(DurabilityError::InvalidStoredState)?;
            let definition = state
                .definition
                .as_ref()
                .ok_or(DurabilityError::InvalidStoredState)?;
            let source = line
                .source
                .as_ref()
                .ok_or(DurabilityError::InvalidStoredState)?;
            let destination = line
                .destination
                .as_ref()
                .ok_or(DurabilityError::InvalidStoredState)?;
            let timed = line.unchanged_timed_state.as_ref();
            let row = timed.and_then(|t| t.row.as_ref());
            let remaining = row
                .and_then(|r| r.remaining_ms)
                .map(i64::try_from)
                .transpose()
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            sqlx::query("INSERT INTO game_player_trade_receipts(occurrence_id,line_ordinal,item_instance_id,source_character_id,source_parent_item_instance_id,source_placement_ordinal,source_placed_transaction_id,destination_character_id,destination_parent_item_instance_id,destination_ordinal,definition_family,definition_production_key,definition_revision_ref,quantity,key_number,state_revision_before,timed_revision,timed_charges,timed_remaining_ms,timed_deadline_at) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric,encode($7,'hex')::uuid,encode($8,'hex')::uuid,encode($9,'hex')::uuid,$10::text::numeric,$11,$12,$13,$14,$15,$16::text::numeric,$17::text::numeric,$18,$19,$20)")
                .bind(request.occurrence.as_slice()).bind(i16::try_from(ordinal+1).map_err(|_|DurabilityError::InvalidStoredState)?)
                .bind(state.item_instance_id.as_slice()).bind(source.character_id.as_slice()).bind(source.parent_item_instance_id.as_slice()).bind(source.placement_ordinal.to_string())
                .bind(source.placed_transaction_id.as_slice()).bind(destination.character_id.as_slice()).bind(destination.parent_item_instance_id.as_slice()).bind(destination.placement_ordinal.to_string())
                .bind(&definition.family).bind(&definition.production_key).bind(&definition.revision_ref).bind(i64::from(state.quantity)).bind(i32::try_from(state.key_number).map_err(|_|DurabilityError::InvalidStoredState)?)
                .bind(line.state_revision_before.to_string()).bind(row.map(|r|r.revision.to_string())).bind(row.and_then(|r|r.charges).map(i64::from)).bind(remaining).bind(timed.and_then(|t|t.deadline_at))
                .execute(&mut **tx).await?;
        }
        // Every before-row has been locked and independently re-read. Delete BOTH
        // old locations before inserting either new location; never UPDATE an entry.
        for line in &message.lines {
            let state = line
                .before
                .as_ref()
                .ok_or(DurabilityError::InvalidStoredState)?;
            let source = line
                .source
                .as_ref()
                .ok_or(DurabilityError::InvalidStoredState)?;
            let removed=sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND parent_item_instance_id=encode($3,'hex')::uuid AND placement_ordinal=$4::text::numeric AND placed_transaction_id=encode($5,'hex')::uuid")
                .bind(state.item_instance_id.as_slice()).bind(source.character_id.as_slice()).bind(source.parent_item_instance_id.as_slice()).bind(source.placement_ordinal.to_string()).bind(source.placed_transaction_id.as_slice()).execute(&mut **tx).await?;
            if removed.rows_affected() != 1 {
                return Err(DurabilityError::InvalidStoredState);
            }
        }
        for line in &message.lines {
            let state = line
                .before
                .as_ref()
                .ok_or(DurabilityError::InvalidStoredState)?;
            let updated=sqlx::query("UPDATE game_item_instances SET last_transaction_id=encode($2,'hex')::uuid WHERE item_instance_id=encode($1,'hex')::uuid AND lifecycle=1 AND quantity=$3 AND state_revision=$4::text::numeric")
                .bind(state.item_instance_id.as_slice()).bind(request.identity.transaction_id.as_slice()).bind(i64::from(state.quantity)).bind(line.state_revision_before.to_string()).execute(&mut **tx).await?;
            if updated.rows_affected() != 1 {
                return Err(DurabilityError::InvalidStoredState);
            }
        }
        for line in &message.lines {
            let state = line
                .before
                .as_ref()
                .ok_or(DurabilityError::InvalidStoredState)?;
            let destination = line
                .destination
                .as_ref()
                .ok_or(DurabilityError::InvalidStoredState)?;
            sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid)")
                .bind(state.item_instance_id.as_slice()).bind(message.world_id.as_slice()).bind(destination.character_id.as_slice()).bind(destination.parent_item_instance_id.as_slice()).bind(destination.placement_ordinal.to_string()).bind(request.identity.transaction_id.as_slice()).execute(&mut **tx).await?;
        }
        let anchor = message
            .lines
            .first()
            .and_then(|l| l.before.as_ref())
            .ok_or(DurabilityError::InvalidStoredState)?;
        sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state,published_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,$6,$6+$7,$8,sha256($8),1,NULL)")
            .bind(request.identity.event_id.as_slice()).bind(request.identity.transaction_id.as_slice()).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id()).bind(anchor.item_instance_id.as_slice()).bind(request.identity.occurred_at_unix_ms).bind(audit::AUDIT_RETENTION_P90D_MS).bind(envelope).execute(&mut **tx).await?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "player_trade_tests.rs"]
mod tests;
