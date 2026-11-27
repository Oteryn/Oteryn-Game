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

#[cfg(test)]
#[path = "player_trade/classification_tests.rs"]
mod classification_tests;
#[path = "player_trade/command.rs"]
mod command;
#[path = "player_trade/observation.rs"]
mod observation;
#[path = "player_trade/ports.rs"]
mod ports;
#[path = "player_trade/preflight.rs"]
pub(super) mod preflight;

// A busy durability holder proves neither a stale session nor a trade refusal.
// This conversion is used only for read-only preflight/observation passes.
// Commit outcomes bypass CheckError and retain reconciliation semantics.
// Commands still fail closed; observations retain the last projection until a read succeeds.
pub(super) enum CheckError {
    Refused(ResultCode),
    Unavailable,
    Caused(CommandOutcome),
}
impl From<ResultCode> for CheckError {
    fn from(code: ResultCode) -> Self {
        Self::Refused(code)
    }
}
impl From<crate::durability::DurabilityError> for CheckError {
    fn from(error: crate::durability::DurabilityError) -> Self {
        match error {
            // A semantic pass deadline (DB_PASS_DEADLINE) outlasts the foreground
            // command window, so a timed-out read cannot be retried in budget: it keeps
            // its own named cause instead of reading as retry_budget_exhausted.
            crate::durability::DurabilityError::RootUnavailable
            | crate::durability::DurabilityError::Database(
                crate::durability::SqlxFailureClass::RetryableStatement,
            ) => Self::Unavailable,
            error => Self::Caused(CommandOutcome {
                code: ResultCode::Rejected,
                cause: format!("durability_{error}").replace(' ', "_"),
            }),
        }
    }
}
impl From<CheckError> for ResultCode {
    fn from(error: CheckError) -> Self {
        match error {
            CheckError::Refused(code) => code,
            CheckError::Unavailable => Self::Rejected,
            CheckError::Caused(outcome) => outcome.code,
        }
    }
}

// Internal only: retain the originating refusal until the command boundary logs it.
pub(super) struct CommandOutcome {
    code: ResultCode,
    cause: String,
}
impl CommandOutcome {
    fn rejected(cause: &str) -> Self {
        Self {
            code: ResultCode::Rejected,
            cause: cause.to_owned(),
        }
    }
    fn budget_exhausted() -> Self {
        Self::rejected("retry_budget_exhausted")
    }
}
impl From<ResultCode> for CommandOutcome {
    fn from(code: ResultCode) -> Self {
        Self {
            code,
            cause: "other".to_owned(),
        }
    }
}
impl From<Refusal> for CommandOutcome {
    fn from(reason: Refusal) -> Self {
        Self {
            code: result(reason),
            cause: match reason {
                Refusal::Rejected => "refusal_rejected",
                Refusal::UnknownWeight => "refusal_unknown_weight",
                _ => "other",
            }
            .to_owned(),
        }
    }
}
impl From<CheckError> for CommandOutcome {
    fn from(error: CheckError) -> Self {
        match error {
            CheckError::Refused(code) => code.into(),
            CheckError::Caused(outcome) => outcome,
            CheckError::Unavailable => Self::budget_exhausted(),
        }
    }
}
impl From<Refusal> for CheckError {
    fn from(reason: Refusal) -> Self {
        Self::Caused(reason.into())
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
    contents: [Vec<wire::ContentsItem>; 2],
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
    switching: std::collections::HashSet<GameSessionId>,
    live: HashMap<GameSessionId, Live>,
    index: HashMap<GameSessionId, GameSessionId>,
    records: HashMap<GameSessionId, Record>,
}
impl Record {
    fn reserves(&self, item: [u8; 16]) -> bool {
        self.items.contains(&Some(item)) || self.offers.iter().flatten().any(|leaf| {
            (0..=leaf.contents().len()).any(|index| {
                u8::try_from(index).ok()
                    .and_then(|index| leaf.instance_at(index).ok().flatten()) == Some(item)
            })
        })
    }
}
impl Book {
    fn record_ref(&self, session: GameSessionId) -> Option<&Record> {
        self.records.get(self.index.get(&session)?)
    }
    fn record(&self, session: GameSessionId) -> Option<Record> {
        self.record_ref(session).cloned()
    }
    fn publish(&mut self, record: &Record) -> Option<()> {
        if record
            .sessions
            .iter()
            .any(|session| self.switching.contains(session))
        {
            return None;
        }
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
                contents: [
                    record.contents[side].clone(),
                    if record.projections[side].is_some() {
                        record.contents[1 - side].clone()
                    } else {
                        Vec::new()
                    },
                ],
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
        if !self
            .records
            .get(&record.sessions[0])
            .is_some_and(|current| {
                current.sessions == record.sessions && current.commands[0] == record.commands[0]
            })
        {
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
pub(super) struct SwitchTradeHold<'a>(&'a TradeBook, GameSessionId);

impl Drop for SwitchTradeHold<'_> {
    fn drop(&mut self) {
        if let Ok(mut book) = self.0.0.lock() {
            book.switching.remove(&self.1);
        }
    }
}
impl TradeBook {
    pub(super) fn hold_for_switch(
        &self,
        session: GameSessionId,
    ) -> Result<SwitchTradeHold<'_>, oteryn_protocol_oteryn::channel_switch::SwitchLock> {
        use oteryn_protocol_oteryn::channel_switch::SwitchLock;
        let mut book = self.0.lock().map_err(|_| SwitchLock::Checkpoint)?;
        if let Some(record) = book.record(session) {
            return Err(if record.request.is_some() {
                SwitchLock::ValueTransaction
            } else {
                SwitchLock::Trade
            });
        }
        if !book.switching.insert(session) {
            return Err(SwitchLock::Transition);
        }
        Ok(SwitchTradeHold(self, session))
    }

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
    #[cfg(test)]
    pub(in crate::gameplay_transport) fn retain_test_request(
        &self,
        binding: ChannelRuntimeAssignmentBinding,
        fence: CurrentCharacterItemFence,
    ) -> Result<(), crate::durability::DurabilityError> {
        use crate::durability::DurabilityError::Unavailable;

        let mut peer = fence;
        let mut character = *fence.character_id.as_bytes();
        character[15] ^= 1;
        peer.character_id =
            crate::domain::CharacterId::from_bytes(character).map_err(|_| Unavailable)?;
        let mut session = *fence.game_session_id.as_bytes();
        session[15] ^= 1;
        peer.game_session_id = GameSessionId::decode(&session).map_err(|_| Unavailable)?;
        let pair = CommandRef::new(
            fence.game_session_id,
            CommandId::new(1).map_err(|_| Unavailable)?,
        );
        let accept_id = CommandId::new(2).map_err(|_| Unavailable)?;
        // The test stops at holder checkout before the frozen leaves are read.
        let leaf: TradeLeaf = serde_json::from_value(serde_json::json!({"item": {}, "entry": {}}))
            .map_err(|_| Unavailable)?;
        let participants = [fence, peer].map(|accepted| TradeParticipant {
            accepted,
            command: CommandRef::new(accepted.game_session_id, accept_id),
            leaf: leaf.clone(),
        });
        let digest = [1; 32];
        let request = PlayerTradeRequest {
            pair,
            occurrence_id: identity(pair, b"occurrence"),
            transaction_id: identity(pair, b"transaction"),
            event_id: identity(pair, b"event"),
            occurred_at_unix_ms: 1,
            participants: participants.clone(),
            content_digest: digest,
            weight_digest: digest,
            node_id: [1; 16],
            node_registration_revision: 1,
            content_revision: "content-1".into(),
            ruleset_revision: "rules-1".into(),
            sim_revision: "sim-1".into(),
        };
        let record = Record {
            sessions: [fence.game_session_id, peer.game_session_id],
            binding,
            digest,
            lifecycle: Trade::offered(pair, 0),
            offers: [None, None],
            items: [None, None],
            backpacks: [None, None],
            projections: [None, None],
            contents: [Vec::new(), Vec::new()],
            commands: participants.each_ref().map(|p| p.command),
            pair: Some(pair),
            id: Some(identity(pair, b"pair")),
            accepts: participants.map(Some),
            staged: None,
            request: Some(request),
        };
        self.0
            .lock()
            .map_err(|_| Unavailable)?
            .records
            .insert(fence.game_session_id, record);
        Ok(())
    }

    #[cfg(test)]
    pub(in crate::gameplay_transport) fn remove_test_request(
        &self,
        session: GameSessionId,
    ) -> Result<(), crate::durability::DurabilityError> {
        self.0
            .lock()
            .map_err(|_| crate::durability::DurabilityError::Unavailable)?
            .records
            .remove(&session);
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn seed_offered_for_test(
        &self,
        binding: ChannelRuntimeAssignmentBinding,
        session: GameSessionId,
        partner: GameSessionId,
        item: [u8; 16],
    ) -> Option<()> {
        let id = CommandId::new(1).ok()?;
        let command = CommandRef::new(session, id);
        let record = Record {
            sessions: [session, partner],
            binding,
            digest: [0; 32],
            lifecycle: Trade::offered(command, 0),
            offers: [None, None],
            items: [Some(item), None],
            backpacks: [None, None],
            projections: [None, None],
            contents: [Vec::new(), Vec::new()],
            commands: [command, CommandRef::new(partner, id)],
            pair: None,
            id: None,
            accepts: [None, None],
            staged: None,
            request: None,
        };
        let mut book = self.0.lock().ok()?;
        book.index.insert(session, session);
        book.index.insert(partner, session);
        book.records.insert(session, record);
        Some(())
    }

    /// Called under the mutation's spell lane. An attempt on an offered leaf is refused
    /// and invalidates the dialog; other backpack mutations invalidate before issuance.
    pub(super) fn physical(&self, session: GameSessionId, item: Option<[u8; 16]>) -> bool {
        self.physical_items(session, [item])
    }

    pub(super) fn physical_items<const N: usize>(
        &self,
        session: GameSessionId,
        items: [Option<[u8; 16]>; N],
    ) -> bool {
        let Ok(mut book) = self.0.lock() else {
            return false;
        };
        let Some(record) = book.record(session) else {
            return true;
        };
        if record.request.is_some() {
            return false;
        }
        let offered = items.into_iter().flatten().any(|item| record.reserves(item));
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
    /// chsw §8: an open direct trade of `session` (a poisoned book counts as open).
    pub(super) fn open(&self, session: GameSessionId) -> bool {
        self.0
            .lock()
            .map_or(true, |book| book.record(session).is_some())
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
