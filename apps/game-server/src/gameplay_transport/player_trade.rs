//! PLAYER_TRADE0 §4 native borrowed Channel TradeOwner plus its original leaf lifecycle.
//!
//! Native TradeOwner re-reads actor/control/lease/visibility/range/map and durable inventory
//! at every offer/accept. Durable inventory reads live in durability::player_trade;
//! this gameplay child retains the owner-local book. The existing leaf remains policy DATA.
//! Physical cross-command reservations, sole-book installation and atomic swap are uninstalled.
//! TRANSFERRING retains owner-local intent DATA on ambiguity, never a swap grant or custody.

#![allow(
    dead_code,
    reason = "private Trade component awaits physical reservations and atomic writer integration"
)]

use crate::durability::player_trade::{LeafOffer, TradeData, TradeReadError};
use crate::foundation::{CommandRef, GameSessionId, RuntimeScopeRefV1};

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
/// pair identifies the counter-offer internally; TRADE-WIRE owns the future wire trade_id map.
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
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, ScopeOwnershipGeneration};
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
    Authority,
    TooFar,
    NotVisible,
    UnknownMap,
    NotTradeable,
    NoBackpack,
    NoRoom,
    Busy,
    Storage(DurabilityError),
}
impl std::fmt::Display for OwnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Policy(refusal) => write!(f, "trade policy: {refusal:?}"),
            Self::Storage(error) => write!(f, "trade storage: {error}"),
            other => write!(f, "trade owner: {other:?}"),
        }
    }
}
impl std::error::Error for OwnerError {}
impl From<DurabilityError> for OwnerError {
    fn from(value: DurabilityError) -> Self {
        Self::Storage(value)
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
    binding: (RuntimeScopeRefV1, ScopeOwnershipGeneration, u64, [u8; 32]),
    origin: Instant,
    trades: BTreeMap<([u8; 16], u64), OwnedTrade>,
    by_actor: BTreeMap<[u8; 16], ([u8; 16], u64)>,
}

fn runtime_key(
    runtime: &ChannelRuntimeV1,
) -> (RuntimeScopeRefV1, ScopeOwnershipGeneration, u64, [u8; 32]) {
    let binding = runtime.binding();
    (
        RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
        binding.scope_generation(),
        binding.source_revision(),
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
        match current {
            Ok(current) => trade
                .session
                .record_revalidated_accept(side, pair, command, &current, ms)?,
            Err(error) => {
                let _ = trade.session.invalidate();
                self.prune_closed();
                return Err(error);
            }
        }
        if let Some(destinations) = destinations {
            let binding = trade
                .session
                .transfer()
                .ok_or(OwnerError::Authority)?
                .clone();
            trade.intent = Some(ReservationIntent {
                binding,
                destinations,
            });
        }
        Ok(trade.session.phase())
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
    /// Unknown retains the book/index and intent. A terminal DATA enum is not native receipt
    /// proof; no runtime release API is exposed until the genuine swap reader is integrated.
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

#[cfg(test)]
#[path = "player_trade_tests.rs"]
mod tests;
