//! Detached PLAYER_TRADE0 §4 lifecycle policy, not a wired ChannelRuntime trade owner.
//!
//! Every offer/accept must first pass independently current native actor/session, visibility,
//! range and sightline checks in the owning runtime, and current leaf/item admission. No bool,
//! cached view, matching data or return value here proves those facts. The unowned private
//! owner/visibility/reservation seams are not replaced by a fake proof or default validator.
//! TRANSFERRING below records the policy obligation to retain real owner reservations; this
//! component does not acquire or release item/entry reservations or authorize a database swap.

use crate::domain::player_trade::{LeafOffer, Trader};
use crate::durability::item_transfer::CharacterBackpack;
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

/// Independently loaded owner data, not current-authority evidence. Complete item state and
/// backpack changes must also reach the owner's issued/committed-change invalidation hook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TradeData {
    pub traders: [Trader; 2],
    pub backpacks: [CharacterBackpack; 2],
    pub offers: [Option<LeafOffer>; 2],
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

#[cfg(test)]
#[path = "player_trade_tests.rs"]
mod tests;
