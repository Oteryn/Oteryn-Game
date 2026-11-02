//! Bounded, non-blocking mailbox between the session task and the shell.

use super::{ServerMessageInbox, StepDir, coalesce_pushed};
use crate::combat_input::{CombatCommand, CombatReport, PlayCommand};
use oteryn_platform_client::native_login::PublicClass;
use oteryn_session::{
    ActorPosition, LogoutResult, SessionEvent, StepOutcome, WorldObjectOverlayEntry,
    WorldSpatialEntity,
};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

/// What the session task reports to the shell.
#[derive(Debug)]
pub enum PlayEvent {
    Stepped(Box<StepOutcome>),
    /// Deltas pushed outside any step, coalesced since the last poll.
    Pushed(Pushed),
    /// The server's answer to a combat command.
    Combat(CombatReport),
    /// The server refused a logout (`None`: before its logout authority); play goes on.
    LogoutRefused(Option<LogoutResult>),
    /// The server committed the logout and closed the session.
    LoggedOut,
    /// The session ended; return to login with this public class.
    Ended(PublicClass),
}

/// Pushed deltas coalesced to latest state, so an undrained shell holds a bounded amount.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Pushed {
    /// Transient domain-16 updates, reduced before publication to the shell.
    pub quests: Vec<oteryn_session::AppliedDelta<oteryn_client_runtime::quest_log::QuestLog>>,
    /// Transient domain-57 messages retained in order; they are never coalesced to one value.
    pub server_messages: ServerMessageInbox,
    /// The latest own-actor position pushed by domain 1.
    pub own: Option<ActorPosition>,
    /// The latest full list of the other visible entities, when it changed.
    pub entities: Option<Vec<WorldSpatialEntity>>,
    /// The latest domain-2 entry per placement, in first-pushed order of placement.
    pub markers: Vec<WorldObjectOverlayEntry>,
}

impl Pushed {
    /// Folds `newer` over `self`: its position and entries win.
    fn absorb(&mut self, newer: Self) {
        self.own = newer.own.or(self.own);
        self.entities = newer.entities.or(self.entities.take());
        self.server_messages.append(newer.server_messages);
        for entry in newer.markers {
            self.set_marker(entry);
        }
    }
}

/// Most distinct placements held while the shell is not draining; a later new placement is
/// dropped until the next poll, an update of a held one is still kept.
pub(super) const MAX_PUSHED_MARKERS: usize = 1024;

#[derive(Debug, Default)]
pub(super) struct Mailbox {
    pub(super) wheel: oteryn_client_ui::wheel_allocation::WheelPanel,
    pub(super) wheel_shutdown: bool,
    pub(super) exiva: oteryn_client_ui::exiva_settings::ExivaPanel,
    pub(super) quest: oteryn_client_ui::quest_log::QuestPanel,
    pub(super) quest_pending: bool,
    pub(super) quest_shutdown: bool,
    pub(super) stash: oteryn_client_ui::stash::StashPanel,
    pub(super) guild: oteryn_client_ui::guild::GuildPanel,
    /// Pushes that arrived before `stepped` was stored: older than its outcome.
    before: Pushed,
    stepped: Option<Box<StepOutcome>>,
    pushed: Pushed,
    /// Combat answers not yet polled; the oldest is dropped past [`MAX_COMBAT_REPORTS`].
    combat: VecDeque<CombatReport>,
    /// The latest refused logout not yet polled.
    logout_refused: Option<Option<LogoutResult>>,
    ended: Option<PlayEvent>,
    pub(super) task_board: oteryn_client_ui::task_board::TaskBoardPanel,
    pub(super) npc_shop: oteryn_client_ui::npc_shop::NpcShopPanel,
    pub(super) prey: oteryn_client_ui::prey::PreyPanel,
    pub(super) prey_shutdown: bool,
    pub(super) imbue: oteryn_client_ui::imbue::ImbuePanel,
    pub(super) imbue_shutdown: bool,
    pub(super) player_trade: oteryn_client_ui::player_trade::PlayerTradePanel,
    pub(super) vip: oteryn_client_ui::vip::VipPanel,
    pub(super) party: oteryn_client_ui::party::PartyPanel,
    pub(super) hireling: oteryn_client_ui::hireling::HirelingPanel,
    pub(super) market: oteryn_client_ui::market::MarketPanel,
    pub(super) market_shutdown: bool,
}

const MAX_COMBAT_REPORTS: usize = 8;
pub(super) type SharedMailbox = Arc<Mutex<Mailbox>>;

impl Mailbox {
    /// A slow shell may not silently lose an announcement or grow an unbounded event queue.
    fn check_server_messages(&mut self) {
        if self.before.server_messages.overflowed()
            || self.pushed.server_messages.overflowed()
            || self.before.server_messages.len() + self.pushed.server_messages.len()
                > oteryn_session::MAX_QUEUED_EVENTS
        {
            self.before.server_messages = ServerMessageInbox::default();
            self.pushed.server_messages = ServerMessageInbox::default();
            self.ended
                .get_or_insert(PlayEvent::Ended(PublicClass::SessionUnavailable));
        }
    }
}

pub(super) fn lock(mailbox: &SharedMailbox) -> MutexGuard<'_, Mailbox> {
    mailbox.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The task's end of the mailbox.
#[derive(Debug, Clone)]
pub struct Outbox(pub(super) SharedMailbox);

impl Outbox {
    pub(super) fn stepped(&self, outcome: StepOutcome) {
        let mut mailbox = lock(&self.0);
        let older = std::mem::take(&mut mailbox.pushed);
        mailbox.before.absorb(older);
        mailbox.check_server_messages();
        mailbox.stepped = Some(Box::new(outcome));
    }

    pub(super) fn push(&self, events: Vec<SessionEvent>) {
        let mut mailbox = lock(&self.0);
        coalesce_pushed(&mut mailbox.pushed, events);
        mailbox.check_server_messages();
        let deltas = std::mem::take(&mut mailbox.pushed.quests);
        super::feature_0016_quest_log::protocol::reduce(&mut mailbox.quest, deltas);
        if let Some(position) = mailbox.pushed.own
            && let (Ok(x), Ok(y), Ok(floor)) = (
                u16::try_from(position.x),
                u16::try_from(position.y),
                i8::try_from(position.floor),
            )
        {
            mailbox.hireling.target = Some((x, y, floor));
        }
    }

    pub(super) fn entities(&self, entities: Vec<WorldSpatialEntity>) {
        lock(&self.0).pushed.entities = Some(entities);
    }

    pub(super) fn combat(&self, report: CombatReport) {
        let mut mailbox = lock(&self.0);
        if mailbox.combat.len() == MAX_COMBAT_REPORTS {
            mailbox.combat.pop_front();
        }
        mailbox.combat.push_back(report);
    }

    pub(super) fn logout_refused(&self, result: Option<LogoutResult>) {
        lock(&self.0).logout_refused = Some(result);
    }

    /// The first end wins.
    pub(super) fn end(&self, class: PublicClass) {
        lock(&self.0).ended.get_or_insert(PlayEvent::Ended(class));
    }

    pub(super) fn logged_out(&self) {
        lock(&self.0).ended.get_or_insert(PlayEvent::LoggedOut);
    }
}

/// The shell's end of the session task: steps out, a bounded mailbox in, neither ever blocking.
#[derive(Debug)]
pub struct PlayLink {
    pub(super) commands: UnboundedSender<PlayCommand>,
    pub(super) mailbox: SharedMailbox,
}

impl PlayLink {
    pub fn request(&self, direction: StepDir) {
        // A closed task has already reported or will report `Ended`.
        let _ = self.commands.send(PlayCommand::Step(direction));
    }

    pub fn request_combat(&self, command: CombatCommand) {
        let _ = self.commands.send(PlayCommand::Combat(command));
    }

    pub fn request_logout(&self) {
        let _ = self.commands.send(PlayCommand::Logout);
    }

    /// The next event: pushes older than the step outcome, the outcome, coalesced pushes, then end.
    pub fn poll(&self) -> Option<PlayEvent> {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.before != Pushed::default() {
            return Some(PlayEvent::Pushed(std::mem::take(&mut mailbox.before)));
        }
        if let Some(outcome) = mailbox.stepped.take() {
            return Some(PlayEvent::Stepped(outcome));
        }
        if mailbox.pushed != Pushed::default() {
            return Some(PlayEvent::Pushed(std::mem::take(&mut mailbox.pushed)));
        }
        if let Some(report) = mailbox.combat.pop_front() {
            return Some(PlayEvent::Combat(report));
        }
        if let Some(result) = mailbox.logout_refused.take() {
            return Some(PlayEvent::LogoutRefused(result));
        }
        mailbox.ended.take()
    }
}

/// The channel pair between the shell and `run_session`.
pub fn play_channel() -> (PlayLink, UnboundedReceiver<PlayCommand>, Outbox) {
    let (commands, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let mailbox = SharedMailbox::default();
    (
        PlayLink {
            commands,
            mailbox: Arc::clone(&mailbox),
        },
        command_rx,
        Outbox(mailbox),
    )
}

impl Drop for PlayLink {
    fn drop(&mut self) {
        let mut mailbox = lock(&self.mailbox);
        mailbox.quest_shutdown = true;
        mailbox.wheel_shutdown = true;
        mailbox.prey_shutdown = true;
        mailbox.imbue_shutdown = true;
        mailbox.market_shutdown = true;
    }
}
