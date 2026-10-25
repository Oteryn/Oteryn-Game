//! Channel-owned PLAYER-TRADE0 book. All issuance runs under lane -> runtime;
//! the leaf book lock is never held over an await. Frozen swaps survive cancellation.
use super::{ComposedFreshAdmission, connection::AdmittedSession, item_view::ItemKey};
use crate::durability::item_transfer::{CharacterBackpack, CurrentCharacterItemFence};
use crate::durability::player_trade::{
    PlayerTradeContent, PlayerTradeRequest, TradeLeaf, TradeParticipant, TradeReceipt,
};
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef, GameSessionId};
use oteryn_gameplay_economy::player_trade::{Accept, Phase, Refusal, Trade, TradeFlag};
use oteryn_gameplay_entities::carrier::ChannelRuntimeAssignmentBinding;
use oteryn_protocol_oteryn::player_trade::{self as wire, Intent, ResultCode, State};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::{collections::HashMap, sync::Mutex};

#[path = "player_trade/command.rs"]
mod command;
#[path = "player_trade/observation.rs"]
mod observation;
#[path = "player_trade/ports.rs"]
mod ports;

// A busy durability holder proves neither a stale session nor a trade refusal.
// Commands still fail closed; observations retain the last projection until a read succeeds.
enum CheckError {
    Refused(ResultCode),
    Unavailable,
}
impl From<ResultCode> for CheckError {
    fn from(code: ResultCode) -> Self {
        Self::Refused(code)
    }
}
impl From<crate::durability::DurabilityError> for CheckError {
    fn from(error: crate::durability::DurabilityError) -> Self {
        match error {
            crate::durability::DurabilityError::RootUnavailable => Self::Unavailable,
            _ => Self::Refused(ResultCode::Rejected),
        }
    }
}
impl From<CheckError> for ResultCode {
    fn from(error: CheckError) -> Self {
        match error {
            CheckError::Refused(code) => code,
            CheckError::Unavailable => Self::Rejected,
        }
    }
}

const MAX_TRADES: usize = 512;
const COMMANDS_PER_SECOND: u32 = 8;
#[derive(Clone)]
struct Live {
    admitted: AdmittedSession,
    actor: ExactActorRef,
    name: String,
    revision: u64,
    state: State,
    inventory_changed: bool,
    rate: Rate,
}
#[derive(Default, Clone)]
struct Rate {
    window_ms: u64,
    commands: u32,
}
impl Rate {
    fn admit(&mut self, now: u64) -> bool {
        if now < self.window_ms {
            return false;
        }
        if now - self.window_ms >= 1000 {
            self.commands = 0;
            self.window_ms = now;
        }
        if self.commands >= COMMANDS_PER_SECOND {
            return false;
        }
        self.commands += 1;
        true
    }
}
fn has_capacity(open: usize, existing: bool) -> bool {
    existing || open < MAX_TRADES
}
#[derive(Clone)]
struct Record {
    sessions: [GameSessionId; 2],
    binding: ChannelRuntimeAssignmentBinding,
    digest: [u8; 32],
    lifecycle: Trade<CommandRef>,
    offers: [Option<TradeLeaf>; 2],
    items: [Option<[u8; 16]>; 2],
    backpacks: [Option<CharacterBackpack>; 2],
    projections: [Option<wire::Offer>; 2],
    commands: [CommandRef; 2],
    pair: Option<CommandRef>,
    id: Option<[u8; 16]>,
    accepts: [Option<TradeParticipant>; 2],
    staged: Option<(usize, TradeParticipant)>,
    request: Option<PlayerTradeRequest>,
}
#[derive(Default)]
pub struct TradeBook(Mutex<Book>, tokio::sync::Notify);
#[derive(Default)]
struct Book {
    #[cfg(test)]
    completed: Option<PlayerTradeRequest>,
    live: HashMap<GameSessionId, Live>,
    index: HashMap<GameSessionId, GameSessionId>,
    records: HashMap<GameSessionId, Record>,
}
impl Book {
    fn record(&self, session: GameSessionId) -> Option<Record> {
        self.records.get(self.index.get(&session)?).cloned()
    }
    fn publish(&mut self, record: &Record) -> Option<()> {
        for side in 0..2 {
            let partner = self.live.get(&record.sessions[1 - side])?;
            let message = if record.projections[side].is_none() {
                format!("{} wants to trade with you.", partner.name)
            } else {
                String::new()
            };
            let partner = partner.actor;
            let live = self.live.get_mut(&record.sessions[side])?;
            let state = State {
                trade_id: record.id,
                partner: Some(entity(partner)),
                own: record.projections[side],
                counter: record.projections[side].and(record.projections[1 - side]),
                accepted: [
                    record.accepts[side].is_some(),
                    record.accepts[1 - side].is_some(),
                ],
                phase: match record.lifecycle.phase() {
                    Phase::Offered => 1,
                    Phase::Ready => 2,
                    Phase::Transferring => 3,
                    Phase::Closed(_) => return None,
                },
                reason: 0,
                message,
                look: String::new(),
            };
            if live.state != state {
                live.revision = live.revision.checked_add(1)?;
                live.state = state;
            }
        }
        self.index.insert(record.sessions[0], record.sessions[0]);
        self.index.insert(record.sessions[1], record.sessions[0]);
        self.records.insert(record.sessions[0], record.clone());
        Some(())
    }
    fn close(&mut self, record: &Record, result: ResultCode) -> Option<()> {
        // A disconnect may close a copied record while its observation awaits I/O.
        // The first closure wins; an old snapshot also cannot close a successor trade.
        if !self.records.get(&record.sessions[0]).is_some_and(|current| {
            current.sessions == record.sessions && current.commands[0] == record.commands[0]
        }) {
            return Some(());
        }
        #[cfg(test)]
        if result == ResultCode::Completed {
            self.completed = record.request.clone();
        }
        self.records.remove(&record.sessions[0]);
        for session in record.sessions {
            self.index.remove(&session);
            if let Some(live) = self.live.get_mut(&session) {
                live.revision = live.revision.checked_add(1)?;
                live.state.phase = 4;
                live.state.reason = match result {
                    ResultCode::Completed => 1,
                    ResultCode::Cancelled | ResultCode::TooFar => 2,
                    ResultCode::NoRoom => 3,
                    ResultCode::Stale => 4,
                    _ => 5,
                };
                live.state.accepted = [false; 2];
                live.state.message = if result == ResultCode::TooFar {
                    ResultCode::Cancelled.message()
                } else {
                    result.message()
                }
                .into();
                live.state.look.clear();
                live.inventory_changed |= result == ResultCode::Completed;
            }
        }
        Some(())
    }
}
impl TradeBook {
    async fn wait_staged(
        &self,
        session: GameSessionId,
        deadline: tokio::time::Instant,
    ) -> Option<()> {
        loop {
            let mut changed = std::pin::pin!(self.1.notified());
            changed.as_mut().enable();
            if !self
                .0
                .lock()
                .ok()?
                .record(session)
                .is_some_and(|r| r.staged.is_some())
            {
                return Some(());
            }
            tokio::time::timeout_at(deadline, changed).await.ok()?;
        }
    }
    #[cfg(test)]
    pub(in crate::gameplay_transport) fn completed_request(&self) -> Option<PlayerTradeRequest> {
        self.0.lock().ok()?.completed.clone()
    }
    /// Called under the mutation's spell lane. An attempt on an offered leaf is refused
    /// and invalidates the dialog; other backpack mutations invalidate before issuance.
    pub(super) fn physical(&self, session: GameSessionId, item: Option<[u8; 16]>) -> bool {
        let Ok(mut book) = self.0.lock() else {
            return false;
        };
        let Some(record) = book.record(session) else {
            return true;
        };
        if record.request.is_some() {
            return false;
        }
        let offered = item.is_some_and(|item| record.items.contains(&Some(item)));
        book.close(&record, ResultCode::Stale).is_some() && !offered
    }
    /// Services that choose inventory inputs internally cannot spend an offered leaf.
    /// Refuse this attempt and close the dialog before resolving their inputs.
    pub(super) fn physical_service(&self, session: GameSessionId) -> bool {
        let Ok(mut book) = self.0.lock() else {
            return false;
        };
        let Some(record) = book.record(session) else {
            return true;
        };
        if record.request.is_none() {
            let _ = book.close(&record, ResultCode::Stale);
        }
        false
    }
    pub(super) fn transitioning(&self, session: GameSessionId) -> bool {
        self.0.lock().map_or(true, |book| {
            book.record(session).is_some_and(|r| r.request.is_some())
        })
    }
    pub(super) fn cancel(&self, session: GameSessionId) {
        if let Ok(mut book) = self.0.lock()
            && let Some(record) = book.record(session)
            && record.request.is_none()
        {
            let _ = book.close(&record, ResultCode::Cancelled);
        }
    }
    pub(super) fn forget(&self, session: GameSessionId) {
        self.cancel(session);
        if let Ok(mut book) = self.0.lock()
            && !book.index.contains_key(&session)
        {
            book.live.remove(&session);
        }
    }
    pub(super) fn terminalized(&self, session: GameSessionId, id: u64) {
        if let Ok(mut book) = self.0.lock()
            && let Some(mut record) = book.record(session)
            && let Some((side, participant)) = record.staged.clone()
            && participant.command.game_session_id() == session
            && participant.command.command_id().get() == id
        {
            record.accepts[side] = Some(participant);
            record.staged = None;
            let _ = book.publish(&record);
        }
        self.1.notify_waiters();
    }
}
fn entity(actor: ExactActorRef) -> EntityRef {
    EntityRef {
        identity: actor.placement_identity(),
        generation: actor.actor_local_generation(),
    }
}
fn result(reason: Refusal) -> ResultCode {
    match reason {
        Refusal::TooFar => ResultCode::TooFar,
        Refusal::NotTradeable => ResultCode::NotTradeable,
        Refusal::NoRoom | Refusal::Overweight => ResultCode::NoRoom,
        Refusal::Stale | Refusal::ClockReversed => ResultCode::Stale,
        Refusal::Transferring => ResultCode::PartnerBusy,
        Refusal::Rejected | Refusal::UnknownWeight => ResultCode::Rejected,
    }
}
fn identity(command: CommandRef, tag: &[u8]) -> [u8; 16] {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(b"oteryn:player-trade:v1:");
    hash.update(tag);
    hash.update(command.game_session_id().as_bytes());
    hash.update(command.command_id().get().to_be_bytes());
    let digest = hash.finalize();
    let mut id = [0; 16];
    id.copy_from_slice(&digest[..16]);
    id[6] = (id[6] & 15) | 0x70;
    id[8] = (id[8] & 63) | 0x80;
    id
}
impl PlayerTradeContent for super::item_ref_admission::NativeItemWeights<'_> {
    fn player_trade_flags(
        &self,
        definition: &crate::durability::item_mint::TypedDefinitionRef,
    ) -> Option<(TradeFlag, bool)> {
        use crate::content::ReferenceItemField as F;
        let s = &self
            .0
            .item_policy(&definition.production_key, &definition.revision_ref)?
            .record()
            .semantics;
        let flag = match &s.trade_restrictions {
            F::Unknown => TradeFlag::Unknown,
            F::Known(r) => match r.tradeable {
                F::Unknown => TradeFlag::Unknown,
                F::Known(v) => TradeFlag::Known(v),
                _ => TradeFlag::Ambiguous,
            },
            _ => TradeFlag::Ambiguous,
        };
        let pickupable = matches!(&s.physical, F::Known(p) if p.pickupable == F::Known(true));
        Some((flag, pickupable))
    }
}
pub(super) use ports::{authority_methods, composed_methods};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn trade_delivered(&self, session: GameSessionId, revision: u64) {
        if let Ok(mut book) = self.states.player_trades.0.lock()
            && let Some(live) = book.live.get_mut(&session)
            && live.revision == revision
        {
            live.inventory_changed = false;
        }
    }
}

#[cfg(test)]
#[path = "player_trade/tests.rs"]
mod tests;
