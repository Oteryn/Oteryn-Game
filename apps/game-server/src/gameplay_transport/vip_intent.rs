//! VIP-2 (VIP0 §§5-8, CP D1444): the VIP list wire of capability 31 `VIP_V1`.
//!
//! One entry per admitted VIP_V1 session holds its own view (the list of its Account in its
//! World and its consent view), the per-session invitation and watcher handles, the status each
//! entry was last shown with and the presence update in flight. Admission runs the durable
//! producer [`DurabilityRoot::admit_vip`] and sends the snapshot with every entry
//! `UNCONFIRMED`; the presence owner (`durability::vip_presence`) then confirms each status, and
//! every change goes out as one domain 21 delta. A local write re-reads the view of the sessions
//! it concerns on this node; other changes arrive with the 60 s reconcile (VIP0-RL-09). The
//! view is DATA for the session projection, never authority for a later write: every intent is
//! decided by the receipted writers of `durability::vip` under their native fences.
//!
//! House (Instance-scope) sessions have no VIP view until HOUSE-RUNTIME-1b (#2165): their
//! admission read fails closed and every intent answers `VIP_UNAVAILABLE`.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use oteryn_protocol_oteryn::vip::{
    MAX_VIP_CONFIRM_MS, VipConsentView, VipDelta, VipEntry, VipGroup, VipGroupSet, VipIntent,
    VipIntentResult, VipInvitation, VipResultCode, VipSnapshot, VipStatus, VipStatusUpdate,
    VipVisibility, VipWatcher, decode_vip_intent,
};

use super::ComposedFreshAdmission;
use super::connection::UseCommand;
use super::spell_premium_coordinator::SpellPremiumCoordinator;
use crate::domain::CharacterId;
use crate::durability::DurabilityError;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::vip::{
    VipAcceptRequest, VipAddRequest, VipAddResult, VipEndKind, VipEndOperation, VipEndRequest,
    VipError, VipGateOperation, VipInvitationBinding, VipListKey, VipListOperation, VipListRequest,
    VipListResult, VipPremiumGate, VipRemoveRequest, VipResult, VipView,
};
use crate::durability::vip_presence::{PresenceOwner, Status, StatusUpdate};
use crate::foundation::{CommandId, CommandRef, GameSessionId};

/// How often a VIP_V1 connection looks for status updates and view changes.
pub(crate) const VIP_REFRESH: std::time::Duration = std::time::Duration::from_millis(250);
/// The view reconcile (VIP0-RL-09).
const VIP_RECONCILE: std::time::Duration = std::time::Duration::from_secs(60);
/// Deltas one tick may send; the rest waits for the next tick.
pub(crate) const VIP_DELTAS_PER_TICK: usize = 32;

/// One delta for the connection to send, and whether it is a presence update the owner must be
/// told was delivered.
pub(crate) struct VipOutgoing {
    pub(crate) delta: VipDelta,
    presence: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HandleKind {
    Invitation,
    Watcher,
}

/// Retired handle bindings a session keeps for the receipt check of a resent ending command:
/// two full consent views (VIP0-RL invitations and watchers). A reconnect retires the whole
/// view of the lost connection, and the one before it stays matchable too.
const RETIRED_HANDLES: usize = 2
    * (oteryn_protocol_oteryn::vip::MAX_VIP_INVITATIONS
        + oteryn_protocol_oteryn::vip::MAX_VIP_WATCHERS);

/// Bindings of the newest ending commands (Accept, Reject, Revoke) a session sent. A resent
/// command whose outcome the client never saw replays against its own binding however many
/// replacement snapshots retired its handle meanwhile. Ending writes share the list's rate
/// bucket (VIP0-RL-07), so the client resends a lost one long before this many newer ones.
const COMMANDED_HANDLES: usize = RETIRED_HANDLES;

/// VIP0-RL-16: the part of an ONLINE status's confirmation time its write may take. The frame
/// carries only the rest, so the client's deadline, which runs from receipt, never passes the
/// read's confirmation deadline by more than the network's own delay; a slower write ends the
/// connection.
const VIP_ONLINE_WRITE: std::time::Duration = std::time::Duration::from_secs(2);

struct VipSession {
    /// Which install of the session's state this is: an asynchronous read that started under
    /// another install (a replacement snapshot came in meanwhile) changes nothing.
    install: u64,
    character: CharacterId,
    fence: CurrentCharacterItemFence,
    view: VipView,
    /// Status each entry was last shown with.
    shown: BTreeMap<u64, VipStatus>,
    /// invitation_id to handle, and handle to binding.
    handle_of: BTreeMap<u64, u64>,
    bindings: BTreeMap<u64, (VipInvitationBinding, HandleKind)>,
    /// Bindings of handles no longer shown, the newest RETIRED_HANDLES, including those of the
    /// session's earlier connections (at least one full view). Only the receipt replay of a resent command reads them.
    retired: BTreeMap<u64, VipInvitationBinding>,
    /// Handle and binding of each of the newest COMMANDED_HANDLES ending commands, oldest
    /// first, kept across replacement snapshots. Only the receipt replay reads them.
    commanded: VecDeque<(u64, VipInvitationBinding)>,
    next_handle: u64,
    /// Highest presence revision delivered to the client.
    presence_seen: u64,
    inflight: Option<StatusUpdate>,
    /// When the write of the in-flight ONLINE status must have ended (VIP_ONLINE_WRITE).
    write_by: Option<std::time::Instant>,
    pending: VecDeque<VipOutgoing>,
    reread: bool,
    /// Local changes that concerned this session; a re-read keeps `reread` set when one came in
    /// while it was awaiting, since its snapshot may predate that change.
    touches: u64,
    read_at: std::time::Instant,
}

/// Each connection of a session numbers handles above every earlier connection's, with one
/// 2^32 range per connection_generation. A replacement snapshot on the same connection
/// continues past the old state's next_handle, so a handle is never reused within a session
/// and a resent stale handle answers NOT_FOUND.
fn handle_floor(generation: u64, previous: Option<u64>) -> u64 {
    ((generation & u64::from(u32::MAX)) << 32).max(previous.unwrap_or(0))
}

/// The earliest bound, with None meaning unbounded.
fn earliest(
    a: Option<std::time::Duration>,
    b: Option<std::time::Duration>,
) -> Option<std::time::Duration> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, None) => a,
        (None, b) => b,
    }
}

/// Keeps the newest RETIRED_HANDLES retired bindings. Handles only grow within a session, so the
/// smallest handle is the oldest.
fn retire(
    retired: &mut BTreeMap<u64, VipInvitationBinding>,
    gone: impl IntoIterator<Item = (u64, VipInvitationBinding)>,
) {
    retired.extend(gone);
    while retired.len() > RETIRED_HANDLES {
        retired.pop_first();
    }
}

/// Keeps the newest COMMANDED_HANDLES ending commands, oldest first; a resent handle moves to
/// the back. Unlike `retire`, a replacement snapshot or a new connection never evicts them.
fn remember_command(
    commanded: &mut VecDeque<(u64, VipInvitationBinding)>,
    handle: u64,
    binding: VipInvitationBinding,
) {
    commanded.retain(|(named, _)| *named != handle);
    commanded.push_back((handle, binding));
    while commanded.len() > COMMANDED_HANDLES {
        commanded.pop_front();
    }
}

impl VipSession {
    fn handle(&mut self, binding: VipInvitationBinding, kind: HandleKind) -> u64 {
        let handle = match self.handle_of.get(&binding.invitation_id) {
            Some(handle) => *handle,
            None => {
                self.next_handle = self.next_handle.saturating_add(1);
                self.handle_of
                    .insert(binding.invitation_id, self.next_handle);
                self.next_handle
            }
        };
        self.bindings.insert(handle, (binding, kind));
        handle
    }

    /// The binding `handle` named, shown, commanded or retired, whatever its kind now. Only the
    /// receipt replay reads it: a new ending command acts on a shown handle of its own kind only.
    fn named(&self, handle: u64) -> Option<VipInvitationBinding> {
        self.bindings
            .get(&handle)
            .map(|(binding, _)| *binding)
            .or_else(|| {
                self.commanded
                    .iter()
                    .rev()
                    .find(|(commanded, _)| *commanded == handle)
                    .map(|(_, binding)| *binding)
            })
            .or_else(|| self.retired.get(&handle).copied())
    }

    /// An ending command named `handle`: its binding stays matchable for the command's resend.
    fn command(&mut self, handle: u64, binding: VipInvitationBinding) {
        remember_command(&mut self.commanded, handle, binding);
    }

    /// The consent view of `view`, with this session's handles (made for new invitations).
    fn consent(&mut self, view: &VipView) -> VipConsentView {
        let live = view
            .invitations
            .iter()
            .map(|(invitation, _)| invitation.binding.invitation_id)
            .chain(
                view.watchers
                    .iter()
                    .map(|(binding, _)| binding.invitation_id),
            )
            .collect::<std::collections::BTreeSet<_>>();
        self.handle_of.retain(|id, _| live.contains(id));
        let gone = self
            .bindings
            .iter()
            .filter(|(_, (binding, _))| !live.contains(&binding.invitation_id))
            .map(|(handle, (binding, _))| (*handle, *binding))
            .collect::<Vec<_>>();
        retire(&mut self.retired, gone);
        self.bindings
            .retain(|_, (binding, _)| live.contains(&binding.invitation_id));
        let mut invitations = view
            .invitations
            .iter()
            .map(|(invitation, name)| VipInvitation {
                handle: self.handle(invitation.binding, HandleKind::Invitation),
                inviter_name: name.clone(),
                expires_at_ms: invitation.expires_at_ms,
            })
            .collect::<Vec<_>>();
        invitations.sort_by_key(|invitation| invitation.handle);
        let mut watchers = view
            .watchers
            .iter()
            .map(|(binding, name)| VipWatcher {
                handle: self.handle(*binding, HandleKind::Watcher),
                name: name.clone(),
            })
            .collect::<Vec<_>>();
        watchers.sort_by_key(|watcher| watcher.handle);
        VipConsentView {
            invitations,
            watchers,
            visibility: if view.hidden {
                VipVisibility::Hidden
            } else {
                VipVisibility::Shown
            },
        }
    }

    fn entry(&self, entry: &crate::durability::vip::VipEntryView) -> VipEntry {
        VipEntry {
            entry_no: entry.entry_no,
            name: entry.name.clone(),
            description: entry.description.clone(),
            icon: entry.icon,
            notify: entry.notify,
            groups: VipGroupSet::of(
                &(1..=8_u8)
                    .filter(|group| entry.groups & (1 << (group - 1)) != 0)
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_default(),
            status: self
                .shown
                .get(&entry.entry_no)
                .copied()
                .unwrap_or(VipStatus::Unconfirmed),
        }
    }

    /// Replace the view with `next`, read after the session's `touches` were `touches`,
    /// queueing one delta per change.
    fn replace(&mut self, next: VipView, touches: u64) {
        let old = std::mem::replace(&mut self.view, next.clone());
        if old.groups != next.groups {
            self.pending.push_back(VipOutgoing {
                delta: VipDelta::Groups(groups(&next)),
                presence: false,
            });
        }
        for entry in &old.entries {
            if !next.entries.iter().any(|e| e.entry_no == entry.entry_no) {
                self.shown.remove(&entry.entry_no);
                self.pending.push_back(VipOutgoing {
                    delta: VipDelta::EntryRemoved {
                        entry_no: entry.entry_no,
                    },
                    presence: false,
                });
            }
        }
        for entry in &next.entries {
            let before = old.entries.iter().find(|e| e.entry_no == entry.entry_no);
            if before != Some(entry) {
                if before.is_some_and(|before| before.target != entry.target) {
                    self.shown.remove(&entry.entry_no);
                }
                let delta = VipDelta::Entry(self.entry(entry));
                self.pending.push_back(VipOutgoing {
                    delta,
                    presence: false,
                });
            }
        }
        let before = self.consent(&old);
        let after = self.consent(&next);
        if before != after {
            self.pending.push_back(VipOutgoing {
                delta: VipDelta::Consent(after),
                presence: false,
            });
        }
        // A change committed while the read was awaiting may postdate its snapshot.
        self.reread = self.touches != touches;
        self.read_at = std::time::Instant::now();
    }

    fn concerns(&self, list: VipListKey, characters: &[CharacterId]) -> bool {
        self.view.list == list || characters.contains(&self.character)
    }
}

fn groups(view: &VipView) -> Vec<VipGroup> {
    view.groups
        .iter()
        .map(|(group_no, name)| VipGroup {
            group_no: *group_no,
            name: name.clone(),
        })
        .collect()
}

const fn wire_status(status: Status) -> VipStatus {
    match status {
        Status::Unconfirmed => VipStatus::Unconfirmed,
        Status::Offline => VipStatus::Offline,
        Status::Online => VipStatus::Online,
    }
}

/// The VIP_V1 sessions of this node and its presence owner.
#[derive(Default)]
pub(crate) struct VipRuntime {
    sessions: Mutex<HashMap<GameSessionId, VipSession>>,
    presence: OnceLock<Arc<PresenceOwner>>,
    /// The last [`VipSession::install`] given out.
    installs: AtomicU64,
    /// Local changes of any list or consent on this node, for an admission read that has no
    /// session state yet to mark.
    touches: AtomicU64,
}

impl VipRuntime {
    /// Installs the node's presence owner once; `false` when one is already installed.
    pub(crate) fn install_presence(&self, presence: Arc<PresenceOwner>) -> bool {
        self.presence.set(presence).is_ok()
    }

    fn presence(&self) -> Option<&PresenceOwner> {
        self.presence.get().map(AsRef::as_ref)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<GameSessionId, VipSession>> {
        self.sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The session ended: its state goes, and the presence owner forgets its watcher.
    pub(crate) fn forget(&self, session: GameSessionId) {
        self.lock().remove(&session);
        if let Some(presence) = self.presence() {
            let _ = presence.forget(session);
        }
    }

    /// The session's connection is lost and it waits in grace: the presence owner forgets its
    /// watcher, so a session nobody reads holds none of the owner's capacity. A resume sends a
    /// replacement snapshot, which starts a new watcher; the session state stays for its handle
    /// floor and retired handles.
    pub(crate) fn detach(&self, session: GameSessionId) {
        if let Some(presence) = self.presence() {
            let _ = presence.forget(session);
        }
    }

    /// Marks every local session the change of `list` or of these characters' consent concerns.
    fn touch(&self, list: VipListKey, characters: &[CharacterId]) {
        let touched = {
            let mut sessions = self.lock();
            self.touches.fetch_add(1, Ordering::SeqCst);
            sessions
                .iter_mut()
                .filter(|(_, s)| s.concerns(list, characters))
                .map(|(id, s)| {
                    s.reread = true;
                    s.touches = s.touches.wrapping_add(1);
                    *id
                })
                .collect::<Vec<_>>()
        };
        if let Some(presence) = self.presence() {
            for session in touched {
                let _ = presence.invalidate(session);
            }
        }
    }

    /// How long the next domain 21 write may take. An ONLINE presence update, and any VIP
    /// delta while an entry is shown ONLINE (an Entry delta re-states that status), must reach
    /// the client before the confirmation deadline (VIP0 §8.2). None means unbounded.
    pub(crate) fn write_bound(&self, session: GameSessionId) -> Option<std::time::Duration> {
        let (update, write_by) = self
            .lock()
            .get(&session)
            .map(|s| (s.inflight.clone(), s.write_by))?;
        let Some(presence) = self.presence() else {
            return update.map(|_| std::time::Duration::ZERO);
        };
        earliest(
            earliest(
                update
                    .as_ref()
                    .and_then(|update| presence.write_bound(update)),
                write_by.map(|by| by.saturating_duration_since(std::time::Instant::now())),
            ),
            presence
                .confirmation_left(session)
                .unwrap_or(Some(std::time::Duration::ZERO)),
        )
    }

    /// The presence update the connection just wrote was delivered.
    pub(crate) fn delivered(&self, session: GameSessionId) {
        let update = {
            let mut sessions = self.lock();
            let Some(s) = sessions.get_mut(&session) else {
                return;
            };
            let Some(update) = s.inflight.take() else {
                return;
            };
            s.write_by = None;
            s.shown.insert(update.entry(), wire_status(update.status()));
            s.presence_seen = s.presence_seen.max(update.revision());
            update
        };
        if let Some(presence) = self.presence() {
            let _ = presence.delivered(&update);
        }
    }
}

fn add_code(result: VipAddResult) -> VipIntentResult {
    match result {
        VipAddResult::Ok => VipIntentResult::of(VipResultCode::Ok),
        VipAddResult::UnknownName => VipIntentResult::of(VipResultCode::UnknownName),
        VipAddResult::AlreadyListed => VipIntentResult::of(VipResultCode::AlreadyListed),
        VipAddResult::ListFull => VipIntentResult::of(VipResultCode::ListFull),
        VipAddResult::Rejected => VipIntentResult::of(VipResultCode::Rejected),
        VipAddResult::Exhausted { seconds } => exhausted(seconds),
    }
}

fn list_code(result: VipListResult) -> VipIntentResult {
    match result {
        VipListResult::Ok => VipIntentResult::of(VipResultCode::Ok),
        VipListResult::NotFound => VipIntentResult::of(VipResultCode::NotFound),
        VipListResult::Rejected => VipIntentResult::of(VipResultCode::Rejected),
        VipListResult::GroupLimit => VipIntentResult::of(VipResultCode::GroupLimit),
        VipListResult::NameTaken => VipIntentResult::of(VipResultCode::NameTaken),
        VipListResult::Exhausted { seconds } => exhausted(seconds),
    }
}

fn end_code(result: VipResult) -> VipIntentResult {
    match result {
        VipResult::Ok => VipIntentResult::of(VipResultCode::Ok),
        VipResult::NotFound => VipIntentResult::of(VipResultCode::NotFound),
        VipResult::ListFull => VipIntentResult::of(VipResultCode::ListFull),
        VipResult::Exhausted { seconds } => exhausted(seconds),
    }
}

const fn exhausted(seconds: u8) -> VipIntentResult {
    VipIntentResult {
        code: VipResultCode::Exhausted,
        wait_seconds: seconds as u32,
    }
}

/// The outcome of one `VIP_INTENT`. `unknown` when the store could not say whether the write
/// committed: the connection then ends before the CommandId is sequenced, so the client's
/// resend replays the receipt.
pub(crate) struct VipCommandOutcome {
    pub(crate) result: VipIntentResult,
    pub(crate) unknown: bool,
}

impl VipCommandOutcome {
    pub(crate) const fn answer(code: VipResultCode) -> Self {
        Self {
            result: VipIntentResult::of(code),
            unknown: false,
        }
    }
}

/// What one intent asks of the writers, resolved against the session's own view and handles.
enum Write {
    Add(String),
    End(VipEndOperation, Vec<CharacterId>),
    List(VipListOperation, Vec<CharacterId>),
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// VIP-1b admission: the durable producer, then the snapshot (every entry `UNCONFIRMED`
    /// until the presence owner confirms it). `None` when the session has no VIP view (no
    /// fence, a House scope, or the store refused); the domain is then not sent.
    pub(crate) async fn vip_open(
        &self,
        session: GameSessionId,
        fence: Option<CurrentCharacterItemFence>,
    ) -> Option<VipSnapshot> {
        let fence = fence.filter(|fence| fence.game_session_id == session)?;
        let touches = self.chat.vip.touches.load(Ordering::SeqCst);
        let view = self
            .root
            .admit_vip(self.character, self.holder, fence)
            .await
            .ok()?;
        let mut sessions = self.chat.vip.lock();
        let old = sessions.get(&session);
        // A connection the session moved past while this read was awaiting installs nothing:
        // the newer connection's state stays.
        if old.is_some_and(|old| {
            old.fence.connection_generation.get() > fence.connection_generation.get()
        }) {
            return None;
        }
        // A replacement keeps the old state's handle floor and the bindings of its handles,
        // which a resent ending command is checked against.
        let (previous, retired, commanded) =
            old.map_or((None, BTreeMap::new(), VecDeque::new()), |old| {
                let mut retired = old.retired.clone();
                retire(
                    &mut retired,
                    old.bindings
                        .iter()
                        .map(|(handle, (binding, _))| (*handle, *binding)),
                );
                (Some(old.next_handle), retired, old.commanded.clone())
            });
        let mut state = VipSession {
            install: self.chat.vip.installs.fetch_add(1, Ordering::SeqCst) + 1,
            character: fence.character_id,
            fence,
            view: view.clone(),
            shown: BTreeMap::new(),
            handle_of: BTreeMap::new(),
            bindings: BTreeMap::new(),
            retired,
            commanded,
            next_handle: handle_floor(fence.connection_generation.get(), previous),
            presence_seen: 0,
            inflight: None,
            write_by: None,
            pending: VecDeque::new(),
            // A change committed on this node while the read was awaiting may postdate it.
            reread: self.chat.vip.touches.load(Ordering::SeqCst) != touches,
            touches: 0,
            read_at: std::time::Instant::now(),
        };
        let consent = state.consent(&view);
        let snapshot = VipSnapshot {
            groups: groups(&view),
            entries: view.entries.iter().map(|e| state.entry(e)).collect(),
            consent,
        };
        sessions.insert(session, state);
        drop(sessions);
        if let Some(presence) = self.chat.vip.presence() {
            // A replacement snapshot starts the watcher over: nothing earlier was shown.
            let _ = presence.forget(session);
        }
        Some(snapshot)
    }

    /// One `VIP_INTENT` of the admitted session, and the deltas of its own view that follow
    /// the result.
    pub(crate) async fn vip_command(
        &self,
        command: UseCommand,
        payload: &[u8],
    ) -> (VipCommandOutcome, Vec<VipDelta>) {
        let Ok(intent) = decode_vip_intent(payload) else {
            return (
                VipCommandOutcome::answer(VipResultCode::Rejected),
                Vec::new(),
            );
        };
        let Ok(command_id) = CommandId::new(command.command_id) else {
            return (
                VipCommandOutcome::answer(VipResultCode::Rejected),
                Vec::new(),
            );
        };
        let command_ref = CommandRef::new(command.game_session_id, command_id);
        // VIP0 §8.2: a node that cannot confirm presence changes nothing. A recorded command
        // may continue: its writer checks the kind and binding and replays without a mutation.
        if !self
            .chat
            .vip
            .presence()
            .is_some_and(PresenceOwner::listening)
        {
            let binding = self
                .chat
                .vip
                .lock()
                .get(&command.game_session_id)
                .map(|session| (session.view.list, session.fence));
            let Some((list, fence)) = binding else {
                return (
                    VipCommandOutcome::answer(VipResultCode::VipUnavailable),
                    Vec::new(),
                );
            };
            match self
                .root
                .vip_receipt_recorded(self.character, fence, list, command_ref)
                .await
            {
                Ok(true) => {}
                // Nothing is recorded, so the refusal changes nothing and is final.
                Ok(false) => {
                    return (
                        VipCommandOutcome::answer(VipResultCode::VipUnavailable),
                        Vec::new(),
                    );
                }
                // The store could not say whether an earlier attempt committed. The outcome
                // stays unknown, so the CommandId is not sequenced and a resend can replay it.
                Err(error) => {
                    return self
                        .vip_finish(
                            command.game_session_id,
                            list,
                            &[fence.character_id],
                            Err(error),
                        )
                        .await;
                }
            }
        }
        let end = match &intent {
            VipIntent::Accept { invitation } => Some((VipEndKind::Accept, *invitation)),
            VipIntent::Reject { invitation } => Some((VipEndKind::Reject, *invitation)),
            VipIntent::Revoke { watcher } => Some((VipEndKind::Revoke, *watcher)),
            // Their writers resolve no per-session handle; their receipts replay inside them.
            VipIntent::Add { .. }
            | VipIntent::Remove { .. }
            | VipIntent::Edit { .. }
            | VipIntent::GroupAdd { .. }
            | VipIntent::GroupRename { .. }
            | VipIntent::GroupRemove { .. }
            | VipIntent::Withdraw { .. }
            | VipIntent::SetStatus(_) => None,
        };
        if let Some((kind, handle)) = end {
            // VIP0 §6: a handle this session still knows, shown or retired, names the request
            // whose digest the receipt must match. A handle it never held names no request, so
            // it resolves below as a stale view (NOT_FOUND) and never replays a receipt.
            let binding = self
                .chat
                .vip
                .lock()
                .get(&command.game_session_id)
                .map(|session| (session.view.list, session.fence, session.named(handle)));
            let Some((list, fence, named)) = binding else {
                return (
                    VipCommandOutcome::answer(VipResultCode::VipUnavailable),
                    Vec::new(),
                );
            };
            if let Some(named) = named {
                match self
                    .root
                    .replay_vip_end(self.character, fence, list, command_ref, kind, named)
                    .await
                {
                    Ok(None) => {}
                    Ok(Some(receipt)) => {
                        return self
                            .vip_finish(
                                command.game_session_id,
                                list,
                                &[fence.character_id],
                                Ok(end_code(receipt.result)),
                            )
                            .await;
                    }
                    Err(error) => {
                        return self
                            .vip_finish(
                                command.game_session_id,
                                list,
                                &[fence.character_id],
                                Err(error),
                            )
                            .await;
                    }
                }
            }
        }
        let (list, fence, write) = match self.vip_resolve(command.game_session_id, intent) {
            Ok(resolved) => resolved,
            Err(code) => return (VipCommandOutcome::answer(code), Vec::new()),
        };
        let (result, touched) = match write {
            Write::Add(name) => {
                let request = VipAddRequest::new(list, command_ref, &name);
                let receipt = self
                    .root
                    .add_vip(self.character, self.holder, fence, self, request)
                    .await;
                (
                    receipt.map(|r| add_code(r.result)),
                    vec![fence.character_id],
                )
            }
            Write::List(operation, mut touched) => {
                touched.push(fence.character_id);
                let request = VipListRequest {
                    list,
                    command: command_ref,
                    operation,
                };
                let receipt = self
                    .root
                    .modify_vip_list(self.character, self.holder, fence, self, request)
                    .await;
                (receipt.map(|r| list_code(r.result)), touched)
            }
            Write::End(operation, mut touched) => {
                touched.push(fence.character_id);
                let receipt = match operation {
                    VipEndOperation::Accept(invitation) => {
                        self.root
                            .accept_vip_invitation(
                                self.character,
                                self.holder,
                                fence,
                                VipAcceptRequest {
                                    list,
                                    command: command_ref,
                                    invitation,
                                },
                            )
                            .await
                    }
                    VipEndOperation::Remove { entry_no } => {
                        self.root
                            .remove_vip_entry(
                                self.character,
                                self.holder,
                                fence,
                                VipRemoveRequest {
                                    list,
                                    command: command_ref,
                                    entry_no,
                                },
                            )
                            .await
                    }
                    operation => {
                        self.root
                            .end_vip_consent(
                                self.character,
                                self.holder,
                                fence,
                                VipEndRequest {
                                    list,
                                    command: command_ref,
                                    operation,
                                },
                            )
                            .await
                    }
                };
                (receipt.map(|r| end_code(r.result)), touched)
            }
        };
        self.vip_finish(command.game_session_id, list, &touched, result)
            .await
    }

    async fn vip_finish(
        &self,
        session: GameSessionId,
        list: VipListKey,
        touched: &[CharacterId],
        result: Result<VipIntentResult, VipError>,
    ) -> (VipCommandOutcome, Vec<VipDelta>) {
        let outcome = match result {
            Ok(result) => VipCommandOutcome {
                result,
                unknown: false,
            },
            Err(VipError::Unavailable(_)) => VipCommandOutcome {
                result: VipIntentResult::of(VipResultCode::VipUnavailable),
                unknown: true,
            },
            Err(VipError::AuthorityRejected | VipError::ConflictingCommand) => {
                VipCommandOutcome::answer(VipResultCode::Rejected)
            }
        };
        if outcome.unknown || outcome.result.code != VipResultCode::Ok {
            return (outcome, Vec::new());
        }
        self.chat.vip.touch(list, touched);
        let deltas = self.vip_reread_all(session).await;
        (outcome, deltas)
    }

    /// The writer request of `intent`, from the session's own list key, fence and handles.
    fn vip_resolve(
        &self,
        session: GameSessionId,
        intent: VipIntent,
    ) -> Result<(VipListKey, CurrentCharacterItemFence, Write), VipResultCode> {
        let mut sessions = self.chat.vip.lock();
        let s = sessions
            .get_mut(&session)
            .ok_or(VipResultCode::VipUnavailable)?;
        // The handle and binding an ending command names, kept for its resend.
        let mut commanded = None;
        let target_of = |entry_no: u64| {
            s.view
                .entries
                .iter()
                .filter(|e| e.entry_no == entry_no)
                .map(|e| e.target)
                .collect::<Vec<_>>()
        };
        let binding = |handle: u64, kind: HandleKind| {
            s.bindings
                .get(&handle)
                .filter(|(_, k)| *k == kind)
                .map(|(binding, _)| *binding)
        };
        let write = match intent {
            VipIntent::Add { name } => Write::Add(name),
            VipIntent::Remove { entry_no } => {
                Write::End(VipEndOperation::Remove { entry_no }, target_of(entry_no))
            }
            VipIntent::Withdraw { entry_no } => {
                Write::End(VipEndOperation::Withdraw { entry_no }, target_of(entry_no))
            }
            VipIntent::Accept { invitation } => {
                // An unknown handle is a stale view: no such invitation (one answer, VIP-0 §5).
                let binding =
                    binding(invitation, HandleKind::Invitation).ok_or(VipResultCode::NotFound)?;
                commanded = Some((invitation, binding));
                Write::End(VipEndOperation::Accept(binding), vec![binding.inviter])
            }
            VipIntent::Reject { invitation } => {
                let binding =
                    binding(invitation, HandleKind::Invitation).ok_or(VipResultCode::NotFound)?;
                commanded = Some((invitation, binding));
                Write::End(VipEndOperation::Reject(binding), vec![binding.inviter])
            }
            VipIntent::Revoke { watcher } => {
                let binding =
                    binding(watcher, HandleKind::Watcher).ok_or(VipResultCode::NotFound)?;
                commanded = Some((watcher, binding));
                Write::End(VipEndOperation::Revoke(binding), vec![binding.inviter])
            }
            VipIntent::Edit {
                entry_no,
                description,
                icon,
                notify,
                groups,
            } => Write::List(
                VipListOperation::Edit {
                    entry_no,
                    description,
                    icon,
                    notify,
                    groups: groups.groups(),
                },
                Vec::new(),
            ),
            VipIntent::GroupAdd { name } => {
                Write::List(VipListOperation::GroupAdd { name }, Vec::new())
            }
            VipIntent::GroupRename { group_no, name } => {
                Write::List(VipListOperation::GroupRename { group_no, name }, Vec::new())
            }
            VipIntent::GroupRemove { group_no } => {
                Write::List(VipListOperation::GroupRemove { group_no }, Vec::new())
            }
            VipIntent::SetStatus(visibility) => Write::List(
                VipListOperation::Status {
                    hidden: visibility == VipVisibility::Hidden,
                },
                // Every watcher of this character sees the change.
                s.view
                    .watchers
                    .iter()
                    .map(|(binding, _)| binding.inviter)
                    .collect(),
            ),
        };
        if let Some((handle, binding)) = commanded {
            s.command(handle, binding);
        }
        Ok((s.view.list, s.fence, write))
    }

    /// Re-reads `session`'s view, queues the deltas of what changed and hands the state to
    /// `take` under the same lock. `None` when the read failed, or the session ended or was
    /// installed anew (a replacement snapshot) while the read was awaiting: that state is left
    /// untouched, since this read may predate it.
    async fn vip_reread<T>(
        &self,
        session: GameSessionId,
        take: impl FnOnce(&mut VipSession) -> T,
    ) -> Option<T> {
        let (fence, install, touches) = self
            .chat
            .vip
            .lock()
            .get(&session)
            .map(|s| (s.fence, s.install, s.touches))?;
        let view = self
            .root
            .admit_vip(self.character, self.holder, fence)
            .await
            .ok()?;
        let mut sessions = self.chat.vip.lock();
        let s = sessions
            .get_mut(&session)
            .filter(|s| s.install == install && s.fence == fence)?;
        s.replace(view, touches);
        Some(take(s))
    }

    /// Re-reads `session`'s view and returns the deltas of what changed.
    async fn vip_reread_all(&self, session: GameSessionId) -> Vec<VipDelta> {
        self.vip_reread(session, |s| {
            s.pending.drain(..).map(|outgoing| outgoing.delta).collect()
        })
        .await
        .unwrap_or_default()
    }

    /// The next delta of `session`: a queued view change first, then a re-read view when one
    /// is due, then one confirmed presence update. `None` when there is nothing to send.
    pub(crate) async fn vip_follow(&self, session: GameSessionId) -> Option<VipDelta> {
        let (fence, install, reread, seen, busy) = {
            let mut sessions = self.chat.vip.lock();
            let s = sessions.get_mut(&session)?;
            if let Some(outgoing) = s.pending.pop_front() {
                debug_assert!(!outgoing.presence);
                return Some(outgoing.delta);
            }
            (
                s.fence,
                s.install,
                s.reread || s.read_at.elapsed() >= VIP_RECONCILE,
                s.presence_seen,
                s.inflight.is_some(),
            )
        };
        if reread {
            // The first change goes now; the rest stay queued for the next ticks.
            if let Some(Some(first)) = self.vip_reread(session, |s| s.pending.pop_front()).await {
                debug_assert!(!first.presence);
                return Some(first.delta);
            }
        }
        if busy {
            return None;
        }
        let presence = self.chat.vip.presence()?;
        let update = if presence.is_due(session).unwrap_or(false) {
            let read = presence.next_current_update(self.character, self.holder, fence, seen);
            let update = match presence.confirmation_left(session) {
                Ok(left) => bounded_read(left, read).await,
                Err(_) => None,
            };
            match update {
                Some(Ok(update)) => Ok(update),
                // VIP0 §8.2: without a current read (the store is unavailable) a shown ONLINE
                // is withdrawn to UNCONFIRMED. This path never yields ONLINE or OFFLINE, and the
                // fence check below binds the update to this exact session.
                // Dropped read reservations are reclaimed by expire's retry_at path.
                Some(Err(_)) | None => presence.next_withdrawal_candidate(session, seen),
            }
        } else {
            presence.next_queued(session)
        }
        .ok()
        .flatten()?;
        let mut sessions = self.chat.vip.lock();
        let s = sessions.get_mut(&session)?;
        if s.fence != fence || s.install != install {
            return None;
        }
        let (write_by, confirmed_for_ms) = if update.status() == Status::Online {
            let (write, confirmed_for_ms) = online_split(presence.write_bound(&update)?)?;
            (
                Some(std::time::Instant::now().checked_add(write)?),
                confirmed_for_ms,
            )
        } else {
            (None, 0)
        };
        let delta = VipDelta::Status(VipStatusUpdate {
            entry_no: update.entry(),
            status: wire_status(update.status()),
            announce: update.announce(),
            confirmed_for_ms,
        });
        s.inflight = Some(update);
        s.write_by = write_by;
        Some(delta)
    }
}

/// How the confirmation time `left` of an ONLINE status splits into the time its write may
/// take and the `confirmed_for_ms` its frame carries (VIP0 §8.2, VIP0-RL-16). The client's
/// deadline runs from receipt, so the two together stay within `left`. `None` when nothing is
/// left to carry.
fn online_split(left: std::time::Duration) -> Option<(std::time::Duration, u32)> {
    let write = (left / 2).min(VIP_ONLINE_WRITE);
    let carried = u32::try_from(left.saturating_sub(write).as_millis())
        .unwrap_or(u32::MAX)
        .min(MAX_VIP_CONFIRM_MS);
    (carried > 0).then_some((write, carried))
}

async fn bounded_read<T>(
    left: Option<std::time::Duration>,
    read: impl Future<Output = T>,
) -> Option<T> {
    match left {
        Some(left) if left.is_zero() => None,
        Some(left) => tokio::time::timeout(left, read).await.ok(),
        None => Some(read.await),
    }
}

impl VipPremiumGate for SpellPremiumCoordinator {
    fn vip_gate<T, F>(
        &self,
        account: [u8; 16],
        operation: F,
    ) -> impl Future<Output = Result<T, DurabilityError>> + Send
    where
        T: Send + 'static,
        F: VipGateOperation<T>,
    {
        self.with_premium_gate(account, operation)
    }
}

impl VipPremiumGate for ComposedFreshAdmission<'_, '_, '_> {
    fn vip_gate<T, F>(
        &self,
        account: [u8; 16],
        operation: F,
    ) -> impl Future<Output = Result<T, DurabilityError>> + Send
    where
        T: Send + 'static,
        F: VipGateOperation<T>,
    {
        self.with_premium_gate(account, operation)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        COMMANDED_HANDLES, CharacterId, MAX_VIP_CONFIRM_MS, RETIRED_HANDLES, VIP_ONLINE_WRITE,
        VipInvitationBinding, bounded_read, earliest, handle_floor, online_split, remember_command,
        retire,
    };
    use std::collections::{BTreeMap, VecDeque};
    use std::future::Future;
    use std::time::Duration;

    #[test]
    fn vip_handle_floor_separates_connection_generations() {
        for generation in 1..=3 {
            let floor = handle_floor(generation, None);
            assert_eq!(floor, generation << 32);
            assert!(floor > handle_floor(generation - 1, None) + u64::from(u32::MAX));
        }
    }

    #[test]
    fn vip_handle_floor_continues_after_a_replacement_snapshot() {
        let floor = handle_floor(2, None);
        assert_eq!(handle_floor(2, Some(floor + 7)), floor + 7);
        assert_eq!(handle_floor(2, Some(floor - 1)), floor);
    }

    #[test]
    fn vip_handle_floor_uses_only_the_low_32_generation_bits() {
        let generation = u64::from(u32::MAX) + 3;
        assert_eq!(handle_floor(generation, None), handle_floor(2, None));
    }

    #[test]
    fn vip_retired_handles_keep_only_the_newest() -> Result<(), String> {
        let bytes = [
            0x01, 0x90, 0x0a, 0x5d, 0xac, 0x96, 0x7a, 0xb2, 0x80, 0xe7, 0x01, 0x23, 0x45, 0x67,
            0x89, 0xab,
        ];
        let binding = |invitation_id| -> Result<VipInvitationBinding, String> {
            Ok(VipInvitationBinding {
                watcher_account: crate::domain::AccountId::from_bytes(bytes)
                    .map_err(|e| format!("{e:?}"))?,
                inviter: CharacterId::from_bytes(bytes).map_err(|e| format!("{e:?}"))?,
                invitation_id,
            })
        };
        let mut retired = BTreeMap::new();
        let count = u64::try_from(RETIRED_HANDLES).map_err(|e| e.to_string())? + 3;
        for handle in 1..=count {
            retire(&mut retired, [(handle, binding(handle)?)]);
        }
        assert_eq!(retired.len(), RETIRED_HANDLES);
        assert_eq!(retired.keys().next(), Some(&4));
        assert_eq!(retired.keys().next_back(), Some(&count));
        assert!(
            retired
                .iter()
                .all(|(handle, binding)| binding.invitation_id == *handle)
        );
        Ok(())
    }

    #[test]
    fn vip_commanded_handles_outlive_retired_eviction() -> Result<(), String> {
        let bytes = [
            0x01, 0x90, 0x0a, 0x5d, 0xac, 0x96, 0x7a, 0xb2, 0x80, 0xe7, 0x01, 0x23, 0x45, 0x67,
            0x89, 0xab,
        ];
        let binding = |invitation_id| -> Result<VipInvitationBinding, String> {
            Ok(VipInvitationBinding {
                watcher_account: crate::domain::AccountId::from_bytes(bytes)
                    .map_err(|e| format!("{e:?}"))?,
                inviter: CharacterId::from_bytes(bytes).map_err(|e| format!("{e:?}"))?,
                invitation_id,
            })
        };
        // An ending command on handle 1, then far more retirements than RETIRED_HANDLES holds:
        // the retired map loses handle 1, the commanded record keeps it.
        let mut commanded = VecDeque::new();
        let mut retired = BTreeMap::new();
        remember_command(&mut commanded, 1, binding(1)?);
        retire(&mut retired, [(1, binding(1)?)]);
        let count = u64::try_from(RETIRED_HANDLES).map_err(|e| e.to_string())? * 3;
        for handle in 2..=count {
            retire(&mut retired, [(handle, binding(handle)?)]);
        }
        assert!(!retired.contains_key(&1));
        assert_eq!(
            commanded
                .iter()
                .find(|(handle, _)| *handle == 1)
                .map(|(_, binding)| binding.invitation_id),
            Some(1)
        );
        // A resend moves the handle to the back instead of duplicating it.
        remember_command(&mut commanded, 2, binding(2)?);
        remember_command(&mut commanded, 1, binding(1)?);
        assert_eq!(
            commanded
                .iter()
                .map(|(handle, _)| *handle)
                .collect::<Vec<_>>(),
            vec![2, 1]
        );
        // Only the newest COMMANDED_HANDLES stay.
        let commands = u64::try_from(COMMANDED_HANDLES).map_err(|e| e.to_string())? + 5;
        for handle in 10..10 + commands {
            remember_command(&mut commanded, handle, binding(handle)?);
        }
        assert_eq!(commanded.len(), COMMANDED_HANDLES);
        assert_eq!(
            commanded.back().map(|(handle, _)| *handle),
            Some(9 + commands)
        );
        assert!(
            commanded
                .iter()
                .all(|(handle, b)| b.invitation_id == *handle)
        );
        Ok(())
    }

    #[test]
    fn vip_online_split_keeps_receipt_deadline_within_the_read_deadline() -> Result<(), String> {
        let max = Duration::from_millis(u64::from(MAX_VIP_CONFIRM_MS));
        for left_ms in [
            2, 3, 10, 999, 1_000, 4_000, 5_000, 9_999, 10_000, 12_000, 60_000,
        ] {
            let left = Duration::from_millis(left_ms);
            let (write, carried) =
                online_split(left).ok_or(format!("{left_ms} ms left must still be sendable"))?;
            let carried = Duration::from_millis(u64::from(carried));
            assert!(carried > Duration::ZERO);
            assert!(carried <= max);
            assert!(write <= VIP_ONLINE_WRITE);
            assert!(write <= left / 2);
            assert!(
                write + carried <= left,
                "{left_ms}: {write:?} + {carried:?}"
            );
        }
        // Most of a fresh 10 s confirmation still reaches the client.
        assert!(
            online_split(Duration::from_secs(10))
                .is_some_and(|(_, carried)| carried >= MAX_VIP_CONFIRM_MS - 2_000)
        );
        assert_eq!(online_split(Duration::ZERO), None);
        assert_eq!(online_split(Duration::from_micros(900)), None);
        Ok(())
    }

    #[test]
    fn vip_write_bound_uses_the_earliest_deadline() {
        let short = Some(Duration::from_secs(1));
        let long = Some(Duration::from_secs(2));
        let zero = Some(Duration::ZERO);
        assert_eq!(earliest(None, None), None);
        assert_eq!(earliest(short, None), short);
        assert_eq!(earliest(None, short), short);
        assert_eq!(earliest(short, long), short);
        assert_eq!(earliest(long, short), short);
        assert_eq!(earliest(zero, long), zero);
        assert_eq!(earliest(long, zero), zero);
    }

    // Neither tokio macros nor test-util is enabled; use the existing runtime pattern and a
    // tiny real timer instead of adding features solely for this test.
    fn run(future: impl Future<Output = ()>) -> std::io::Result<()> {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()?
            .block_on(future);
        Ok(())
    }

    #[test]
    fn vip_intent_bounded_read_withdraws_after_the_bound() -> std::io::Result<()> {
        run(async {
            let left = Duration::from_millis(20);
            let started = tokio::time::Instant::now();
            let result = bounded_read(Some(left), std::future::pending::<()>()).await;
            assert_eq!(result, None);
            assert!(started.elapsed() >= left);
        })
    }

    #[test]
    fn vip_intent_bounded_read_returns_ready_and_unbounded_values() -> std::io::Result<()> {
        run(async {
            assert_eq!(
                bounded_read(Some(Duration::from_millis(20)), async { 7 }).await,
                Some(7)
            );
            assert_eq!(bounded_read(None, async { 8 }).await, Some(8));
        })
    }

    #[test]
    fn vip_intent_bounded_read_at_zero_never_polls() -> std::io::Result<()> {
        run(async {
            let polled = std::cell::Cell::new(false);
            let read = std::future::poll_fn(|_| {
                polled.set(true);
                std::task::Poll::Ready(())
            });
            assert_eq!(bounded_read(Some(Duration::ZERO), read).await, None);
            assert!(!polled.get());
        })
    }
}

#[cfg(test)]
#[path = "../durability/vip_tests.rs"]
pub(super) mod durability_vip_tests;
