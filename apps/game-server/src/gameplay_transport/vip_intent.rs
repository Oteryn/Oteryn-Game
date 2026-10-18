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
use std::sync::{Arc, Mutex, OnceLock};

use oteryn_protocol_oteryn::vip::{
    VipConsentView, VipDelta, VipEntry, VipGroup, VipGroupSet, VipIntent, VipIntentResult,
    VipInvitation, VipResultCode, VipSnapshot, VipStatus, VipStatusUpdate, VipVisibility,
    VipWatcher, decode_vip_intent,
};

use super::ComposedFreshAdmission;
use super::connection::UseCommand;
use super::spell_premium_coordinator::SpellPremiumCoordinator;
use crate::domain::CharacterId;
use crate::durability::DurabilityError;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::vip::{
    VipAcceptRequest, VipAddRequest, VipAddResult, VipEndOperation, VipEndRequest, VipError,
    VipGateOperation, VipInvitationBinding, VipListKey, VipListOperation, VipListRequest,
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

struct VipSession {
    character: CharacterId,
    fence: CurrentCharacterItemFence,
    view: VipView,
    /// Status each entry was last shown with.
    shown: BTreeMap<u64, VipStatus>,
    /// invitation_id to handle, and handle to binding.
    handle_of: BTreeMap<u64, u64>,
    bindings: BTreeMap<u64, (VipInvitationBinding, HandleKind)>,
    next_handle: u64,
    /// Highest presence revision delivered to the client.
    presence_seen: u64,
    inflight: Option<StatusUpdate>,
    pending: VecDeque<VipOutgoing>,
    reread: bool,
    read_at: std::time::Instant,
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

    /// Replace the view with `next`, queueing one delta per change.
    fn replace(&mut self, next: VipView) {
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
        self.reread = false;
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

    /// Marks every local session the change of `list` or of these characters' consent concerns.
    fn touch(&self, list: VipListKey, characters: &[CharacterId]) {
        let touched = {
            let mut sessions = self.lock();
            sessions
                .iter_mut()
                .filter(|(_, s)| s.concerns(list, characters))
                .map(|(id, s)| {
                    s.reread = true;
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
        let view = self
            .root
            .admit_vip(self.character, self.holder, fence)
            .await
            .ok()?;
        let mut state = VipSession {
            character: fence.character_id,
            fence,
            view: view.clone(),
            shown: BTreeMap::new(),
            handle_of: BTreeMap::new(),
            bindings: BTreeMap::new(),
            next_handle: 0,
            presence_seen: 0,
            inflight: None,
            pending: VecDeque::new(),
            reread: false,
            read_at: std::time::Instant::now(),
        };
        let consent = state.consent(&view);
        let snapshot = VipSnapshot {
            groups: groups(&view),
            entries: view.entries.iter().map(|e| state.entry(e)).collect(),
            consent,
        };
        if let Some(presence) = self.chat.vip.presence() {
            // A replacement snapshot starts the watcher over: nothing earlier was shown.
            let _ = presence.forget(session);
        }
        self.chat.vip.lock().insert(session, state);
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
        let (list, fence, write) = match self.vip_resolve(command.game_session_id, intent) {
            Ok(resolved) => resolved,
            Err(code) => return (VipCommandOutcome::answer(code), Vec::new()),
        };
        // VIP-0 §8.2: a node that cannot confirm presence changes nothing.
        if !self
            .chat
            .vip
            .presence()
            .is_some_and(PresenceOwner::listening)
        {
            return (
                VipCommandOutcome::answer(VipResultCode::VipUnavailable),
                Vec::new(),
            );
        }
        let Ok(command_id) = CommandId::new(command.command_id) else {
            return (
                VipCommandOutcome::answer(VipResultCode::Rejected),
                Vec::new(),
            );
        };
        let command_ref = CommandRef::new(command.game_session_id, command_id);
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
        self.chat.vip.touch(list, &touched);
        let deltas = self.vip_reread(command.game_session_id).await;
        (outcome, deltas)
    }

    /// The writer request of `intent`, from the session's own list key, fence and handles.
    fn vip_resolve(
        &self,
        session: GameSessionId,
        intent: VipIntent,
    ) -> Result<(VipListKey, CurrentCharacterItemFence, Write), VipResultCode> {
        let sessions = self.chat.vip.lock();
        let s = sessions
            .get(&session)
            .ok_or(VipResultCode::VipUnavailable)?;
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
                Write::End(VipEndOperation::Accept(binding), vec![binding.inviter])
            }
            VipIntent::Reject { invitation } => {
                let binding =
                    binding(invitation, HandleKind::Invitation).ok_or(VipResultCode::NotFound)?;
                Write::End(VipEndOperation::Reject(binding), vec![binding.inviter])
            }
            VipIntent::Revoke { watcher } => {
                let binding =
                    binding(watcher, HandleKind::Watcher).ok_or(VipResultCode::NotFound)?;
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
        Ok((s.view.list, s.fence, write))
    }

    /// Re-reads `session`'s view and returns the deltas of what changed.
    async fn vip_reread(&self, session: GameSessionId) -> Vec<VipDelta> {
        let Some(fence) = self.chat.vip.lock().get(&session).map(|s| s.fence) else {
            return Vec::new();
        };
        let Ok(view) = self
            .root
            .admit_vip(self.character, self.holder, fence)
            .await
        else {
            return Vec::new();
        };
        let mut sessions = self.chat.vip.lock();
        let Some(s) = sessions.get_mut(&session) else {
            return Vec::new();
        };
        s.replace(view);
        s.pending.drain(..).map(|outgoing| outgoing.delta).collect()
    }

    /// The next delta of `session`: a queued view change first, then a re-read view when one
    /// is due, then one confirmed presence update. `None` when there is nothing to send.
    pub(crate) async fn vip_follow(&self, session: GameSessionId) -> Option<VipDelta> {
        let (fence, reread, seen, busy) = {
            let mut sessions = self.chat.vip.lock();
            let s = sessions.get_mut(&session)?;
            if let Some(outgoing) = s.pending.pop_front() {
                debug_assert!(!outgoing.presence);
                return Some(outgoing.delta);
            }
            (
                s.fence,
                s.reread || s.read_at.elapsed() >= VIP_RECONCILE,
                s.presence_seen,
                s.inflight.is_some(),
            )
        };
        if reread {
            let mut deltas = self.vip_reread(session).await.into_iter();
            let first = deltas.next();
            if let Some(first) = first {
                if let Some(s) = self.chat.vip.lock().get_mut(&session) {
                    for (index, delta) in deltas.enumerate() {
                        s.pending.insert(
                            index,
                            VipOutgoing {
                                delta,
                                presence: false,
                            },
                        );
                    }
                }
                return Some(first);
            }
        }
        if busy {
            return None;
        }
        let presence = self.chat.vip.presence()?;
        let update = if presence.is_due(session).unwrap_or(false) {
            presence
                .next_current_update(self.character, self.holder, fence, seen)
                .await
        } else {
            presence.next_queued(session)
        }
        .ok()
        .flatten()?;
        let mut sessions = self.chat.vip.lock();
        let s = sessions.get_mut(&session)?;
        if s.fence != fence {
            return None;
        }
        let delta = VipDelta::Status(VipStatusUpdate {
            entry_no: update.entry(),
            status: wire_status(update.status()),
            announce: update.announce(),
        });
        s.inflight = Some(update);
        Some(delta)
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
#[path = "../durability/vip_tests.rs"]
mod durability_vip_tests;
