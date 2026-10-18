//! VIP0 §8: bounded node presence owner and genuine native combined reader.
//! Registered as a durability child (`#[path = "../vip/presence.rs"] mod vip_presence` in
//! `durability/mod.rs`). The World relay (`chat::relay` kind 3) feeds it presence hints and
//! listener state through `gameplay_transport::chat_relay`.
//! Read observations and queued updates are DATA, never continuing delivery authority.
//! `gameplay_transport::vip_intent` registers each VIP_V1 session's watcher and delivers its
//! status updates as domain 21 deltas (CP D1444).
//! House (Instance-scope) watchers fail closed: every read refuses a non-Channel session scope
//! until HOUSE-RUNTIME-1b (#2165) gives House sessions a native scope proof.
#![allow(
    dead_code,
    reason = "Production reads per session (`next_current_update`); the batched RL-11 reader, the \
              visibility refresh and the DB-outage withdrawal path wait on the node scheduler"
)]

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence, uuid_text};
use super::character_progression::assert_session_fence;
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use crate::domain::{AccountId, CharacterId};
use crate::foundation::{GameSessionId, RuntimeScopeRefV1, WorldId};
use sqlx::{Postgres, Row, Transaction};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

const LIST_MAX: usize = 100; // Accepted VIP0-RL-08; no registry mutation here.
const CONFIRM_MS: u64 = 10_000;
const RECONCILE_MS: u64 = 60_000;

#[derive(Debug)]
pub(crate) enum PresenceError {
    AuthorityRejected,
    Unavailable(DurabilityError),
    Capacity,
    Clock,
    StaleWork,
    Busy,
    RevisionExhausted,
}
impl From<DurabilityError> for PresenceError {
    fn from(e: DurabilityError) -> Self {
        Self::Unavailable(e)
    }
}
impl From<sqlx::Error> for PresenceError {
    fn from(e: sqlx::Error) -> Self {
        Self::Unavailable(e.into())
    }
}
type Result<T> = std::result::Result<T, PresenceError>;
fn corrupt() -> PresenceError {
    DurabilityError::InvalidStoredState.into()
}
/// The owner clock in UTC milliseconds: the wall clock read once, then advanced by the
/// monotonic clock, so owner deadlines never move backward with a wall-clock step. Local
/// because every path-loaded durability root compiles this module without the node clock.
fn clock() -> Result<u64> {
    static ANCHOR: std::sync::OnceLock<Option<(u64, std::time::Instant)>> =
        std::sync::OnceLock::new();
    let (utc_ms, at) = (*ANCHOR.get_or_init(|| {
        let elapsed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?;
        Some((
            u64::try_from(elapsed.as_millis()).ok()?,
            std::time::Instant::now(),
        ))
    }))
    .ok_or(PresenceError::Clock)?;
    u64::try_from(at.elapsed().as_millis())
        .ok()
        .and_then(|elapsed| utc_ms.checked_add(elapsed))
        .ok_or(PresenceError::Clock)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Status {
    Unconfirmed,
    Offline,
    Online,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Visibility {
    Unconfirmed,
    Hidden,
    Shown,
}
#[derive(Debug, Clone)]
struct EntryRead {
    target: CharacterId,
    entry: u64,
    invitation: Option<u64>,
    visibility: Visibility,
    online: bool,
    notify: bool,
}
impl EntryRead {
    fn status(&self) -> Status {
        match self.visibility {
            Visibility::Unconfirmed => Status::Unconfirmed,
            Visibility::Hidden => Status::Offline,
            Visibility::Shown => {
                if self.online {
                    Status::Online
                } else {
                    Status::Offline
                }
            }
        }
    }
}

/// Private fields: produced only by the actual native reader (or explicitly synthetic pure tests).
/// A committed observation is not a current scope/session/transport/consent capability.
struct WatcherRead {
    fence: CurrentCharacterItemFence,
    account: AccountId,
    world: WorldId,
    list_revision: u64,
    consent_revision: u64,
    complete: bool,
    requested: BTreeSet<u64>,
    entries: Vec<EntryRead>,
}
enum Selection {
    RecipientOnly,
    Full,
    Online {
        list_revision: u64,
        entries: BTreeSet<u64>,
    },
}

impl DurabilityRoot {
    async fn read_current_vip_presence(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        selection: Selection,
    ) -> Result<WatcherRead> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| PresenceError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    match read_in_transaction(&mut tx, &recovery, &node, fence, selection).await {
                        Ok(read) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(read))
                        }
                        Err(PresenceError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}
async fn read_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    selection: Selection,
) -> Result<WatcherRead> {
    assert_recovery_fence(tx, recovery).await?;
    lock_admission_relations(tx).await?;
    let account = assert_session_fence(tx, &fence, node)
        .await?
        .map_err(|_| PresenceError::AuthorityRejected)?;
    let account = AccountId::from_bytes(uuid_text(&account)?).map_err(|_| corrupt())?;
    // House seam (HOUSE-RUNTIME-1b, #2165): a House session's Instance scope has no native
    // scope-only fence yet, so its watcher fails closed here and shows nothing.
    let RuntimeScopeRefV1::Channel {
        world_id: world, ..
    } = fence.runtime_scope
    else {
        return Err(PresenceError::AuthorityRejected);
    };
    if matches!(selection, Selection::RecipientOnly) {
        return Ok(WatcherRead {
            fence,
            account,
            world,
            list_revision: 0,
            consent_revision: 0,
            complete: false,
            requested: BTreeSet::new(),
            entries: Vec::new(),
        });
    }
    let revision:Option<String>=sqlx::query_scalar("SELECT revision::text FROM game_vip_lists WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR SHARE")
        .bind(account.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let list_revision = revision.map(parse).transpose()?.unwrap_or(0);
    let consent:Option<String>=sqlx::query_scalar("SELECT vip_consent_revision::text FROM game_character_social_settings WHERE character_id=encode($1,'hex')::uuid AND source_policy='PARTYPVP0-PARTIES-AND-PVP-V1' FOR SHARE")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    // VIP0's admitted missing-row default is SHOWN/revision0, not a missing-column/schema fallback.
    let consent_revision = consent.map(parse).transpose()?.unwrap_or(0);
    let (complete, requested) = match selection {
        Selection::Online {
            list_revision: expected,
            entries,
        } if expected == list_revision && !entries.is_empty() && entries.len() <= LIST_MAX => {
            (false, entries)
        }
        _ => (true, BTreeSet::new()),
    };
    let filter = (!complete).then(|| requested.iter().map(u64::to_string).collect::<Vec<_>>());
    // Admission relation locks serialize native consent/settings/session writers. No target
    // Account or exact placement is selected; only inviter-specific relationship + coarse truth.
    let rows=sqlx::query("SELECT e.entry_no::text,e.character_id::text,e.notify,c.invitation_id::text,c.consent,COALESCE(p.vip_status,1::smallint) AS vip_status,p.source_policy,EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s WHERE s.character_id=e.character_id AND s.world_id=e.world_id AND s.session_state IN(1,2)) AS online,t.world_id= e.world_id AND t.lifecycle=1 AS target_valid FROM game_vip_entries e JOIN game_character_roots t ON t.character_id=e.character_id LEFT JOIN game_vip_consents c ON c.account_id=e.account_id AND c.world_id=e.world_id AND c.character_id=e.character_id AND c.inviter_character_id=encode($3,'hex')::uuid LEFT JOIN game_character_social_settings p ON p.character_id=e.character_id WHERE e.account_id=encode($1,'hex')::uuid AND e.world_id=encode($2,'hex')::uuid AND ($4::text[] IS NULL OR e.entry_no::text=ANY($4::text[])) ORDER BY e.entry_no LIMIT 101")
        .bind(account.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(filter).fetch_all(&mut **tx).await?;
    if rows.len() > LIST_MAX {
        return Err(PresenceError::Capacity);
    }
    let mut seen = BTreeSet::new();
    let entries = rows
        .iter()
        .map(|row| decode_entry(row, &mut seen))
        .collect::<Result<Vec<_>>>()?;
    Ok(WatcherRead {
        fence,
        account,
        world,
        list_revision,
        consent_revision,
        complete,
        requested,
        entries,
    })
}
fn decode_entry(row: &sqlx::postgres::PgRow, seen: &mut BTreeSet<u64>) -> Result<EntryRead> {
    let entry = parse(row.try_get("entry_no")?)?;
    if entry == 0 || !seen.insert(entry) || !row.try_get::<bool, _>("target_valid")? {
        return Err(corrupt());
    }
    let target =
        CharacterId::from_bytes(uuid_text(row.try_get("character_id")?)?).map_err(|_| corrupt())?;
    let invitation = row
        .try_get::<Option<String>, _>("invitation_id")?
        .map(parse)
        .transpose()?;
    let accepted = match row.try_get::<Option<i16>, _>("consent")? {
        None => false,
        Some(1) => false,
        Some(2) if invitation.is_some_and(|id| id > 0) => true,
        _ => return Err(corrupt()),
    };
    if row
        .try_get::<Option<String>, _>("source_policy")?
        .is_some_and(|p| p != "PARTYPVP0-PARTIES-AND-PVP-V1")
    {
        return Err(corrupt());
    }
    let shown = match row.try_get::<i16, _>("vip_status")? {
        1 => true,
        2 => false,
        _ => return Err(corrupt()),
    };
    Ok(EntryRead {
        target,
        entry,
        invitation,
        visibility: if !accepted {
            Visibility::Unconfirmed
        } else if shown {
            Visibility::Shown
        } else {
            Visibility::Hidden
        },
        online: row.try_get("online")?,
        notify: row.try_get("notify")?,
    })
}
fn parse(value: String) -> Result<u64> {
    value.parse().map_err(|_| corrupt())
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Refresh {
    Reconcile,
    Visibility,
    PresenceHint { target: CharacterId, online: bool },
    Delivery,
}
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct StatusUpdate {
    session: GameSessionId,
    entry: u64,
    status: Status,
    announce: bool,
    revision: u64,
    serial: u64,
    epoch: u64,
    fence: CurrentCharacterItemFence,
    deadline_ms: u64,
}
impl std::fmt::Debug for StatusUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatusUpdate").finish_non_exhaustive()
    }
}
impl StatusUpdate {
    pub(crate) const fn entry(&self) -> u64 {
        self.entry
    }
    pub(crate) const fn status(&self) -> Status {
        self.status
    }
    pub(crate) const fn announce(&self) -> bool {
        self.announce
    }
    pub(crate) const fn revision(&self) -> u64 {
        self.revision
    }
}
#[derive(Clone)]
struct EntryState {
    read: EntryRead,
    desired: Status,
    last_sent: Option<Status>,
    confirmed_from: u64,
}
#[derive(Clone)]
struct Watcher {
    ticket: u64,
    fence: CurrentCharacterItemFence,
    account: Option<AccountId>,
    entries: BTreeMap<u64, EntryState>,
    pending: BTreeMap<u64, StatusUpdate>,
    inflight: Option<StatusUpdate>,
    revision: u64,
    list_revision: u64,
    consent_revision: u64,
    full_at: Option<u64>,
    dirty: bool,
    reading: bool,
    retry_at: u64,
}
struct Completion {
    session: GameSessionId,
    ticket: u64,
    epoch: u64,
    started: u64,
    now: u64,
    reason: Refresh,
    serial: u64,
}
struct State {
    world: WorldId,
    capacity: usize,
    serial: u64,
    epoch: u64,
    listener: bool,
    last_clock: u64,
    sessions: BTreeMap<GameSessionId, Watcher>,
    index: BTreeMap<CharacterId, BTreeSet<GameSessionId>>,
}
impl State {
    fn new(world: WorldId, capacity: usize) -> Result<Self> {
        if capacity == 0 || capacity.checked_mul(LIST_MAX).is_none() {
            return Err(PresenceError::Capacity);
        }
        Ok(Self {
            world,
            capacity,
            serial: 0,
            epoch: 0,
            listener: false,
            last_clock: 0,
            sessions: BTreeMap::new(),
            index: BTreeMap::new(),
        })
    }
    fn next_serial(&mut self) -> Result<u64> {
        self.serial = self
            .serial
            .checked_add(1)
            .ok_or(PresenceError::RevisionExhausted)?;
        Ok(self.serial)
    }
    fn observe_time(&mut self, now: u64) -> Result<()> {
        if now < self.last_clock {
            return Err(PresenceError::Clock);
        }
        self.last_clock = now;
        Ok(())
    }
    fn reserve(
        &mut self,
        fence: CurrentCharacterItemFence,
        floor: u64,
        now: u64,
    ) -> Result<(u64, u64, Selection)> {
        self.expire(now)?;
        self.reserve_at(fence, floor, now)
    }
    fn reserve_at(
        &mut self,
        fence: CurrentCharacterItemFence,
        floor: u64,
        now: u64,
    ) -> Result<(u64, u64, Selection)> {
        if !self.listener {
            return Err(PresenceError::Unavailable(DurabilityError::Unavailable));
        }
        let session = fence.game_session_id;
        if !self.sessions.contains_key(&session) && self.sessions.len() >= self.capacity {
            return Err(PresenceError::Capacity);
        }
        let ticket = self.next_serial()?;
        let watcher = self.sessions.entry(session).or_insert_with(|| Watcher {
            ticket,
            fence,
            account: None,
            entries: BTreeMap::new(),
            pending: BTreeMap::new(),
            inflight: None,
            revision: floor,
            list_revision: 0,
            consent_revision: 0,
            full_at: None,
            dirty: true,
            reading: false,
            retry_at: 0,
        });
        watcher.ticket = ticket;
        watcher.reading = true;
        watcher.retry_at = now.checked_add(CONFIRM_MS).ok_or(PresenceError::Clock)?;
        watcher.revision = watcher.revision.max(floor);
        watcher.pending.retain(|_, p| p.revision > floor);
        if watcher
            .inflight
            .as_ref()
            .is_some_and(|p| p.revision <= floor)
        {
            watcher.inflight = None;
        }
        let online = watcher
            .entries
            .iter()
            .filter(|(_, e)| e.desired == Status::Online || e.last_sent == Some(Status::Online))
            .map(|(id, _)| *id)
            .collect::<BTreeSet<_>>();
        let full = watcher.dirty
            || watcher.fence != fence
            || watcher
                .full_at
                .is_none_or(|at| now.saturating_sub(at) >= RECONCILE_MS)
            || online.is_empty();
        Ok((
            ticket,
            self.epoch,
            if full {
                Selection::Full
            } else {
                Selection::Online {
                    list_revision: watcher.list_revision,
                    entries: online,
                }
            },
        ))
    }
    fn replace_watcher(&mut self, session: GameSessionId, next: Option<Watcher>) {
        // Only accepted entries (shown or hidden) are indexed: a pending, rejected or absent
        // consent can never consume a hint, so it must not let a target's hints dirty the session.
        let accepted = |w: &Watcher| {
            w.entries
                .values()
                .filter(|e| e.read.visibility != Visibility::Unconfirmed)
                .map(|e| e.read.target)
                .collect::<BTreeSet<_>>()
        };
        let before = self
            .sessions
            .get(&session)
            .map(accepted)
            .unwrap_or_default();
        let after = next.as_ref().map(accepted).unwrap_or_default();
        for target in before.difference(&after) {
            if let Some(sessions) = self.index.get_mut(target) {
                sessions.remove(&session);
                if sessions.is_empty() {
                    self.index.remove(target);
                }
            }
        }
        for target in after.difference(&before) {
            self.index.entry(*target).or_default().insert(session);
        }
        match next {
            Some(next) => {
                self.sessions.insert(session, next);
            }
            None => {
                self.sessions.remove(&session);
            }
        }
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "One bounded status transition retains exact session, epoch and timing bindings"
    )]
    fn enqueue(
        w: &mut Watcher,
        session: GameSessionId,
        entry: u64,
        status: Status,
        announce: bool,
        epoch: u64,
        serial: u64,
        now: u64,
    ) -> Result<()> {
        w.revision = w
            .revision
            .checked_add(1)
            .ok_or(PresenceError::RevisionExhausted)?;
        w.pending.insert(
            entry,
            StatusUpdate {
                session,
                entry,
                status,
                announce,
                revision: w.revision,
                serial,
                epoch,
                fence: w.fence,
                deadline_ms: now.checked_add(CONFIRM_MS).ok_or(PresenceError::Clock)?,
            },
        );
        Ok(())
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "Actual read completion binds one reserved work ticket, epoch and conservative time interval"
    )]
    fn finish(
        &mut self,
        session: GameSessionId,
        ticket: u64,
        epoch: u64,
        read: WatcherRead,
        started: u64,
        now: u64,
        reason: Refresh,
    ) -> Result<()> {
        self.expire(now)?;
        let serial = self.next_serial()?;
        let next = self.prepare_finish(
            Completion {
                session,
                ticket,
                epoch,
                started,
                now,
                reason,
                serial,
            },
            read,
        )?;
        self.replace_watcher(session, Some(next));
        Ok(())
    }
    fn prepare_finish(&self, work: Completion, read: WatcherRead) -> Result<Watcher> {
        let Completion {
            session,
            ticket,
            epoch,
            started,
            now,
            reason,
            serial,
        } = work;
        if !self.listener
            || epoch != self.epoch
            || read.world != self.world
            || read.fence.game_session_id != session
        {
            return Err(PresenceError::StaleWork);
        }
        // Work on one bounded watcher copy; malformed input/overflow cannot partially
        // replace its authoritative-read projection or reverse index.
        let mut next = self
            .sessions
            .get(&session)
            .filter(|w| w.ticket == ticket)
            .ok_or(PresenceError::StaleWork)?
            .clone();
        let w = &mut next;
        if read.entries.len() > LIST_MAX {
            return Err(PresenceError::Capacity);
        }
        let replaced = w.fence != read.fence;
        if replaced {
            w.pending.clear();
            w.inflight = None;
            for e in w.entries.values_mut() {
                e.last_sent = None;
            }
        }
        w.fence = read.fence;
        w.account = Some(read.account);
        w.list_revision = read.list_revision;
        w.consent_revision = read.consent_revision;
        w.reading = false;
        w.retry_at = now;
        let ids = read
            .entries
            .iter()
            .map(|r| r.entry)
            .collect::<BTreeSet<_>>();
        if ids.len() != read.entries.len() {
            return Err(corrupt());
        }
        if read.complete {
            w.entries.retain(|id, _| ids.contains(id));
            w.pending.retain(|id, _| ids.contains(id));
            if w.inflight.as_ref().is_some_and(|p| !ids.contains(&p.entry)) {
                w.inflight = None;
            }
            w.full_at = Some(started);
            w.dirty = false;
        } else {
            for missing in read.requested.difference(&ids) {
                if let Some(e) = w.entries.get_mut(missing) {
                    e.desired = Status::Unconfirmed;
                    w.dirty = true;
                    if w.inflight.as_ref().is_some_and(|p| p.entry == *missing) {
                        w.inflight = None;
                    }
                    Self::enqueue(
                        w,
                        session,
                        *missing,
                        Status::Unconfirmed,
                        false,
                        epoch,
                        serial,
                        now,
                    )?;
                }
            }
        }
        let timely = now
            .checked_sub(started)
            .is_some_and(|elapsed| elapsed < CONFIRM_MS);
        for e in read.entries {
            if !read.complete
                && w.entries
                    .get(&e.entry)
                    .is_none_or(|old| old.read.target != e.target)
            {
                return Err(corrupt());
            }
            let before = w.entries.remove(&e.entry);
            let compatible = before.as_ref().is_some_and(|old| {
                old.read.target == e.target && old.read.invitation == e.invitation
            }) && !replaced;
            let old_sent = before
                .as_ref()
                .filter(|_| !replaced)
                .and_then(|old| old.last_sent);
            let mut status = e.status();
            if status == Status::Online && !timely {
                status = Status::Unconfirmed;
                w.dirty = true;
            }
            let announce = matches!(reason,Refresh::PresenceHint{target,online} if target==e.target && online==e.online)
                && e.notify
                && e.visibility == Visibility::Shown
                && compatible
                && before.as_ref().is_some_and(|old| {
                    old.read.visibility == Visibility::Shown && old.read.online != e.online
                })
                && old_sent.is_some_and(|old| old != status)
                && timely;
            let entry = e.entry;
            let changed_binding = !compatible;
            let permits_notice = e.notify && e.visibility == Visibility::Shown;
            if w.inflight.as_ref().is_some_and(|p| {
                p.entry == entry
                    && (p.status != status || changed_binding || (p.announce && !permits_notice))
            }) {
                w.inflight = None;
            }
            if (changed_binding || !permits_notice || !matches!(reason, Refresh::Delivery))
                && let Some(p) = w.pending.get_mut(&entry)
            {
                p.announce = false;
            }
            let unchanged_pending = w.pending.get(&entry).is_some_and(|p| {
                p.status == status && p.fence == w.fence && p.deadline_ms > now && !changed_binding
            });
            w.entries.insert(
                entry,
                EntryState {
                    read: e,
                    desired: status,
                    last_sent: old_sent,
                    confirmed_from: started,
                },
            );
            let represented_inflight = w.inflight.as_ref().is_some_and(|p| {
                p.entry == entry
                    && p.status == status
                    && p.fence == w.fence
                    && p.deadline_ms > now
                    && !changed_binding
            });
            if old_sent != Some(status) && !represented_inflight {
                if !unchanged_pending {
                    w.pending.remove(&entry);
                    Self::enqueue(w, session, entry, status, announce, epoch, serial, started)?;
                } else if announce && let Some(p) = w.pending.get_mut(&entry) {
                    p.announce = true;
                }
            } else {
                w.pending.remove(&entry);
            }
        }
        Ok(next)
    }
    fn failed(&mut self, session: GameSessionId, ticket: u64, epoch: u64, now: u64) -> Result<()> {
        self.expire(now)?;
        if epoch != self.epoch {
            return Ok(());
        }
        let serial = self.next_serial()?;
        if self
            .sessions
            .get(&session)
            .is_some_and(|w| w.ticket == ticket && w.account.is_none())
        {
            // No successful native read ever qualified this empty capacity reservation.
            self.sessions.remove(&session);
            return Ok(());
        }
        if let Some(w) = self
            .sessions
            .get_mut(&session)
            .filter(|w| w.ticket == ticket)
        {
            w.reading = false;
            w.dirty = true;
            w.retry_at = now.checked_add(CONFIRM_MS).ok_or(PresenceError::Clock)?;
            w.inflight = None;
            w.pending.clear();
            let ids = w
                .entries
                .iter()
                .filter(|(_, e)| e.desired == Status::Online || e.last_sent == Some(Status::Online))
                .map(|(id, _)| *id)
                .collect::<Vec<_>>();
            for id in ids {
                if let Some(e) = w.entries.get_mut(&id) {
                    e.desired = Status::Unconfirmed;
                }
                w.pending.remove(&id);
                Self::enqueue(
                    w,
                    session,
                    id,
                    Status::Unconfirmed,
                    false,
                    epoch,
                    serial,
                    now,
                )?;
            }
        }
        Ok(())
    }
    fn expire(&mut self, now: u64) -> Result<()> {
        self.observe_time(now)?;
        let serial = self.next_serial()?;
        let epoch = self.epoch;
        let mut abandoned = Vec::new();
        for (session, w) in &mut self.sessions {
            if w.reading && now >= w.retry_at {
                // Cancellation/drop has no completion callback. Timeout the DATA work,
                // invalidate its ticket immediately, and allow bounded reconciliation.
                w.ticket = serial;
                w.reading = false;
                w.dirty = true;
                if w.account.is_none() {
                    abandoned.push(*session);
                }
            }
            let expired = w
                .entries
                .iter()
                .filter(|(_, e)| {
                    e.desired == Status::Online
                        && now.saturating_sub(e.confirmed_from) >= CONFIRM_MS
                })
                .map(|(id, _)| *id)
                .collect::<Vec<_>>();
            for id in expired {
                if let Some(e) = w.entries.get_mut(&id) {
                    e.desired = Status::Unconfirmed;
                }
                w.pending.remove(&id);
                if w.inflight
                    .as_ref()
                    .is_some_and(|p| p.entry == id && p.status == Status::Online)
                {
                    w.inflight = None;
                }
                Self::enqueue(
                    w,
                    *session,
                    id,
                    Status::Unconfirmed,
                    false,
                    epoch,
                    serial,
                    now,
                )?;
            }
        }
        for session in abandoned {
            self.sessions.remove(&session);
        }
        Ok(())
    }
    fn listener_down(&mut self, now: u64) -> Result<()> {
        self.expire(now)?;
        self.epoch = self
            .epoch
            .checked_add(1)
            .ok_or(PresenceError::RevisionExhausted)?;
        self.listener = false;
        let serial = self.next_serial()?;
        for (session, w) in &mut self.sessions {
            w.pending.clear();
            w.inflight = None;
            w.reading = false;
            w.dirty = true;
            let ids = w
                .entries
                .iter()
                .filter(|(_, e)| e.last_sent == Some(Status::Online) || e.desired == Status::Online)
                .map(|(id, _)| *id)
                .collect::<Vec<_>>();
            for id in ids {
                if let Some(e) = w.entries.get_mut(&id) {
                    e.desired = Status::Unconfirmed;
                }
                Self::enqueue(
                    w,
                    *session,
                    id,
                    Status::Unconfirmed,
                    false,
                    self.epoch,
                    serial,
                    now,
                )?;
            }
        }
        Ok(())
    }
    fn listener_reconnected(&mut self, now: u64) -> Result<()> {
        self.observe_time(now)?;
        self.epoch = self
            .epoch
            .checked_add(1)
            .ok_or(PresenceError::RevisionExhausted)?;
        self.listener = true;
        for w in self.sessions.values_mut() {
            w.dirty = true;
            w.reading = false;
            w.pending.clear();
            w.inflight = None;
            // The cleared withdrawals are replaced only by a fresh read, so an earlier failed
            // read's backoff must not delay it past the confirmation bound (RL-09 reconcile).
            w.retry_at = now;
        }
        Ok(())
    }
    fn next(&mut self, session: GameSessionId, now: u64) -> Result<Option<StatusUpdate>> {
        self.expire(now)?;
        let w = self
            .sessions
            .get_mut(&session)
            .ok_or(PresenceError::StaleWork)?;
        if w.inflight.is_some() {
            return Err(PresenceError::Busy);
        }
        // Coalesced per entry, dequeued by revision: entry order would let a newer
        // revision overtake an older pending one and break monotonic delivery.
        let Some(id) = w
            .pending
            .iter()
            .min_by_key(|(_, p)| p.revision)
            .map(|(id, _)| *id)
        else {
            return Ok(None);
        };
        let update = w.pending.remove(&id).ok_or_else(corrupt)?;
        if update.status == Status::Online && (update.deadline_ms <= now || !self.listener) {
            return Err(PresenceError::StaleWork);
        }
        if !w.entries.contains_key(&id) {
            return Err(corrupt());
        }
        w.inflight = Some(update.clone());
        Ok(Some(update))
    }
    fn withdrawal(
        &mut self,
        session: GameSessionId,
        floor: u64,
        now: u64,
    ) -> Result<Option<StatusUpdate>> {
        self.expire(now)?;
        let serial = self.next_serial()?;
        let epoch = self.epoch;
        let w = self
            .sessions
            .get_mut(&session)
            .ok_or(PresenceError::StaleWork)?;
        w.revision = w.revision.max(floor);
        // An unavailable listener/read can never carry forward a positive status/notice.
        w.pending.retain(|_, p| p.status == Status::Unconfirmed);
        let old_withdrawal = w
            .inflight
            .as_ref()
            .filter(|p| p.status == Status::Unconfirmed && p.revision <= floor)
            .map(|p| p.entry);
        if w.inflight
            .as_ref()
            .is_some_and(|p| p.status != Status::Unconfirmed || p.revision <= floor)
        {
            w.inflight = None;
        }
        if let Some(id) = old_withdrawal {
            Self::enqueue(
                w,
                session,
                id,
                Status::Unconfirmed,
                false,
                epoch,
                serial,
                now,
            )?;
        }
        let withdraw = w
            .entries
            .iter()
            .filter(|(_, e)| e.desired == Status::Online || e.last_sent == Some(Status::Online))
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        for id in withdraw {
            if let Some(e) = w.entries.get_mut(&id) {
                e.desired = Status::Unconfirmed;
            }
            let represented = w.pending.get(&id).is_some_and(|p| p.revision > floor)
                || w.inflight.as_ref().is_some_and(|p| {
                    p.entry == id && p.status == Status::Unconfirmed && p.revision > floor
                });
            if !represented {
                Self::enqueue(
                    w,
                    session,
                    id,
                    Status::Unconfirmed,
                    false,
                    epoch,
                    serial,
                    now,
                )?;
            }
        }
        let reissue = w
            .pending
            .iter()
            .filter(|(_, p)| p.revision <= floor)
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        for id in reissue {
            Self::enqueue(
                w,
                session,
                id,
                Status::Unconfirmed,
                false,
                epoch,
                serial,
                now,
            )?;
        }
        self.next(session, now)
    }
    fn due(&mut self, now: u64, budget: usize) -> Result<Vec<GameSessionId>> {
        self.expire(now)?;
        if !self.listener {
            return Ok(Vec::new());
        }
        Ok(self
            .sessions
            .iter()
            .filter(|(_, w)| {
                !w.reading
                    && now >= w.retry_at
                    && (w.dirty
                        || w.full_at
                            .is_none_or(|at| now.saturating_sub(at) >= RECONCILE_MS)
                        || w.entries.values().any(|e| {
                            (e.desired == Status::Online || e.last_sent == Some(Status::Online))
                                && now.saturating_sub(e.confirmed_from) >= CONFIRM_MS
                        }))
            })
            .take(budget.min(self.capacity))
            .map(|(id, _)| *id)
            .collect())
    }
    fn delivered(&mut self, update: &StatusUpdate, now: u64) -> Result<()> {
        self.expire(now)?;
        let w = self
            .sessions
            .get_mut(&update.session)
            .ok_or(PresenceError::StaleWork)?;
        if w.inflight.as_ref() != Some(update)
            || w.fence != update.fence
            || self.epoch != update.epoch
            || (update.status == Status::Online && update.deadline_ms <= now)
        {
            return Err(PresenceError::StaleWork);
        }
        let entry = w
            .entries
            .get_mut(&update.entry)
            .ok_or(PresenceError::StaleWork)?;
        if entry.desired != update.status
            || (update.announce
                && (!entry.read.notify || entry.read.visibility != Visibility::Shown))
        {
            return Err(PresenceError::StaleWork);
        }
        entry.last_sent = Some(update.status);
        w.inflight = None;
        Ok(())
    }
}

/// Native root-bound owner. No externally supplied current/consent/visibility predicate.
pub(crate) struct PresenceOwner {
    root: DurabilityRoot,
    state: Mutex<State>,
}
impl PresenceOwner {
    pub(crate) fn new(
        root: DurabilityRoot,
        world: WorldId,
        max_local_sessions: usize,
    ) -> Result<Self> {
        Ok(Self {
            root,
            state: Mutex::new(State::new(world, max_local_sessions)?),
        })
    }
    pub(crate) async fn refresh_current(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        last_seen_revision: u64,
        reason: Refresh,
    ) -> Result<()> {
        let started = clock()?;
        if let Refresh::PresenceHint { target, .. } = reason {
            self.target_hint(target)?;
        }
        let (ticket, epoch, selection) = self.state.lock().map_err(|_| corrupt())?.reserve(
            fence,
            last_seen_revision,
            started,
        )?;
        let selection = if matches!(reason, Refresh::Visibility) {
            selection
        } else {
            Selection::Full
        };
        let read = self
            .root
            .read_current_vip_presence(authority, node, fence, selection)
            .await;
        let now = clock()?;
        let result = match read {
            Ok(read) => self.state.lock().map_err(|_| corrupt())?.finish(
                fence.game_session_id,
                ticket,
                epoch,
                read,
                started,
                now,
                reason,
            ),
            Err(e) => Err(e),
        };
        if result.is_err() {
            self.state.lock().map_err(|_| corrupt())?.failed(
                fence.game_session_id,
                ticket,
                epoch,
                now,
            )?;
        }
        result
    }
    /// Revalidates native truth before handing out one bounded candidate. This is not the
    /// unresolved final socket/client custody boundary; the candidate must never be a grant.
    pub(crate) async fn next_current_update(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        last_seen_revision: u64,
    ) -> Result<Option<StatusUpdate>> {
        let (listener, epoch) = {
            let s = self.state.lock().map_err(|_| corrupt())?;
            (s.listener, s.epoch)
        };
        if listener
            && self
                .refresh_current(
                    authority,
                    node,
                    fence,
                    last_seen_revision,
                    Refresh::Delivery,
                )
                .await
                .is_ok()
        {
            return self
                .state
                .lock()
                .map_err(|_| corrupt())?
                .next(fence.game_session_id, clock()?);
        }
        // Downgrades still require a genuine current recipient read. This path can send
        // only UNCONFIRMED; it cannot turn unavailable VIP rows/listener into ONLINE.
        let current = self
            .root
            .read_current_vip_presence(authority, node, fence, Selection::RecipientOnly)
            .await?;
        let mut s = self.state.lock().map_err(|_| corrupt())?;
        if current.world != s.world || epoch != s.epoch {
            return Err(PresenceError::StaleWork);
        }
        let w = s
            .sessions
            .get_mut(&fence.game_session_id)
            .filter(|w| w.fence == current.fence)
            .ok_or(PresenceError::StaleWork)?;
        let _ = w; // End the exact-recipient binding borrow before bounded dequeue.
        s.withdrawal(fence.game_session_id, last_seen_revision, clock()?)
    }
    /// DB-independent withdrawal DATA only, never current recipient/delivery authority.
    /// Actual connection custody must bind this session before any socket handoff (CP).
    /// Useful during full DB outage; no ONLINE/OFFLINE or notices can pass this boundary.
    pub(crate) fn next_withdrawal_candidate(
        &self,
        session: GameSessionId,
        last_seen_revision: u64,
    ) -> Result<Option<StatusUpdate>> {
        self.state
            .lock()
            .map_err(|_| corrupt())?
            .withdrawal(session, last_seen_revision, clock()?)
    }
    pub(crate) fn delivered(&self, update: &StatusUpdate) -> Result<()> {
        self.state
            .lock()
            .map_err(|_| corrupt())?
            .delivered(update, clock()?)
    }
    /// The relay listener is up: the node can confirm presence (VIP-0 §8.2).
    pub(crate) fn listening(&self) -> bool {
        self.state.lock().is_ok_and(|s| s.listener)
    }
    pub(crate) fn listener_down(&self) -> Result<()> {
        self.state
            .lock()
            .map_err(|_| corrupt())?
            .listener_down(clock()?)
    }
    /// Lifecycle input only: restores no ONLINE or current authority; real reads must reconcile.
    pub(crate) fn listener_reconnected(&self) -> Result<()> {
        self.state
            .lock()
            .map_err(|_| corrupt())?
            .listener_reconnected(clock()?)
    }
    pub(crate) fn target_hint(&self, target: CharacterId) -> Result<()> {
        let mut s = self.state.lock().map_err(|_| corrupt())?;
        let sessions = s.index.get(&target).cloned().unwrap_or_default();
        for id in sessions {
            if let Some(w) = s.sessions.get_mut(&id) {
                w.dirty = true;
            }
        }
        Ok(())
    }
    pub(crate) fn forget(&self, session: GameSessionId) -> Result<()> {
        let mut s = self.state.lock().map_err(|_| corrupt())?;
        s.replace_watcher(session, None);
        Ok(())
    }
    pub(crate) fn due_sessions(&self, budget: usize) -> Result<Vec<GameSessionId>> {
        self.state
            .lock()
            .map_err(|_| corrupt())?
            .due(clock()?, budget)
    }
    /// `session` needs a native read before its next update: it has no watcher yet, or it is
    /// dirty, due for its reconcile or holds an ONLINE older than its confirmation.
    pub(crate) fn is_due(&self, session: GameSessionId) -> Result<bool> {
        let now = clock()?;
        let mut s = self.state.lock().map_err(|_| corrupt())?;
        if !s.sessions.contains_key(&session) {
            return Ok(true);
        }
        let capacity = s.capacity;
        Ok(s.due(now, capacity)?.contains(&session))
    }
    /// The next queued update of `session` without a new read. Each queued ONLINE still carries
    /// the confirmation deadline of the read that made it, and an expired one is refused.
    pub(crate) fn next_queued(&self, session: GameSessionId) -> Result<Option<StatusUpdate>> {
        self.state
            .lock()
            .map_err(|_| corrupt())?
            .next(session, clock()?)
    }
    /// A local write changed `session`'s list or consent: its next update reads first.
    pub(crate) fn invalidate(&self, session: GameSessionId) -> Result<()> {
        if let Some(w) = self
            .state
            .lock()
            .map_err(|_| corrupt())?
            .sessions
            .get_mut(&session)
        {
            w.dirty = true;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "presence_tests.rs"]
mod tests;

/// Fresh native inputs for one batch. These references are consumed for this call only;
/// the owner caches neither a recovery capability nor a node/session fence as authority.
pub(crate) struct BatchWatcher<'a, 'f, 's> {
    pub(crate) authority: &'a ReconciledCharacterAuthority<'f, 's>,
    pub(crate) node: &'a NodeIncarnationProof,
    pub(crate) fence: CurrentCharacterItemFence,
    pub(crate) last_seen_revision: u64,
}
struct BatchReadInput {
    recovery: CharacterRecoveryFenceV1,
    node: NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
}
impl DurabilityRoot {
    async fn read_current_vip_batch(
        &self,
        inputs: Vec<BatchReadInput>,
    ) -> Result<Vec<WatcherRead>> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    match read_batch_in_transaction(&mut tx, inputs).await {
                        Ok(reads) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(reads))
                        }
                        Err(PresenceError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}

/// The only per-watcher SQL is unchanged native authority validation. VIP list/view
/// revisions and all target/consent facts are read in two bulk statements after validation.
/// The admission relation lock domain is the same one used by the native VIP1 preparation.
async fn read_batch_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    inputs: Vec<BatchReadInput>,
) -> Result<Vec<WatcherRead>> {
    for input in &inputs {
        assert_recovery_fence(tx, &input.recovery).await?;
    }
    lock_admission_relations(tx).await?;
    let mut reads = Vec::with_capacity(inputs.len());
    for input in inputs {
        let account = assert_session_fence(tx, &input.fence, &input.node)
            .await?
            .map_err(|_| PresenceError::AuthorityRejected)?;
        let account = AccountId::from_bytes(uuid_text(&account)?).map_err(|_| corrupt())?;
        // House seam (#2165): an Instance-scope watcher fails the whole batch closed.
        let RuntimeScopeRefV1::Channel {
            world_id: world, ..
        } = input.fence.runtime_scope
        else {
            return Err(PresenceError::AuthorityRejected);
        };
        reads.push(WatcherRead {
            fence: input.fence,
            account,
            world,
            list_revision: 0,
            consent_revision: 0,
            complete: true,
            requested: BTreeSet::new(),
            entries: Vec::new(),
        });
    }
    let accounts = reads
        .iter()
        .map(|r| r.account.as_bytes().to_vec())
        .collect::<Vec<_>>();
    let worlds = reads
        .iter()
        .map(|r| r.world.as_bytes().to_vec())
        .collect::<Vec<_>>();
    let characters = reads
        .iter()
        .map(|r| r.fence.character_id.as_bytes().to_vec())
        .collect::<Vec<_>>();
    let revisions = sqlx::query(r"
WITH watchers AS (
 SELECT encode(a,'hex')::uuid AS account,encode(w,'hex')::uuid AS world,encode(c,'hex')::uuid AS actor,slot
 FROM unnest($1::bytea[],$2::bytea[],$3::bytea[]) WITH ORDINALITY AS x(a,w,c,slot)
)
SELECT w.slot,l.revision::text,p.vip_consent_revision::text,p.source_policy
FROM watchers w LEFT JOIN game_vip_lists l ON l.account_id=w.account AND l.world_id=w.world
LEFT JOIN game_character_social_settings p ON p.character_id=w.actor ORDER BY w.slot
").bind(&accounts).bind(&worlds).bind(&characters).fetch_all(&mut **tx).await?;
    if revisions.len() != reads.len() {
        return Err(corrupt());
    }
    for (index, row) in revisions.iter().enumerate() {
        if usize::try_from(row.try_get::<i64, _>("slot")?).ok() != index.checked_add(1) {
            return Err(corrupt());
        }
        if row
            .try_get::<Option<String>, _>("source_policy")?
            .is_some_and(|p| p != "PARTYPVP0-PARTIES-AND-PVP-V1")
        {
            return Err(corrupt());
        }
        reads[index].list_revision = row
            .try_get::<Option<String>, _>("revision")?
            .map(parse)
            .transpose()?
            .unwrap_or(0);
        reads[index].consent_revision = row
            .try_get::<Option<String>, _>("vip_consent_revision")?
            .map(parse)
            .transpose()?
            .unwrap_or(0);
    }
    // MATERIALIZED targets deduplicates session/settings reads across lists/inviters. The
    // inviter-specific consent join happens only afterwards; target sharing grants no consent.
    // LIMIT101 is independently applied to each verified Account+World list (accepted cap100).
    let rows = sqlx::query(r"
WITH watchers AS (
 SELECT encode(a,'hex')::uuid AS account,encode(w,'hex')::uuid AS world,encode(c,'hex')::uuid AS actor,slot
 FROM unnest($1::bytea[],$2::bytea[],$3::bytea[]) WITH ORDINALITY AS x(a,w,c,slot)
), entries AS MATERIALIZED (
 SELECT w.slot,w.actor,e.* FROM watchers w JOIN LATERAL (
 SELECT e.* FROM game_vip_entries e WHERE e.account_id=w.account AND e.world_id=w.world ORDER BY e.entry_no LIMIT 101
 ) e ON TRUE
), targets AS MATERIALIZED (
 SELECT DISTINCT character_id,world_id FROM entries
), facts AS MATERIALIZED (
 SELECT t.character_id,t.world_id,COALESCE(p.vip_status,1::smallint) AS vip_status,p.source_policy,
 r.world_id=t.world_id AND r.lifecycle=1 AS target_valid,
 EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s WHERE s.character_id=t.character_id AND s.world_id=t.world_id AND s.session_state IN(1,2)) AS online
 FROM targets t JOIN game_character_roots r ON r.character_id=t.character_id
 LEFT JOIN game_character_social_settings p ON p.character_id=t.character_id
)
SELECT e.slot,e.entry_no::text,e.character_id::text,e.notify,c.invitation_id::text,c.consent,
 f.vip_status,f.source_policy,f.target_valid,f.online FROM entries e
 JOIN facts f ON f.character_id=e.character_id AND f.world_id=e.world_id
 LEFT JOIN game_vip_consents c ON c.account_id=e.account_id AND c.world_id=e.world_id AND c.character_id=e.character_id AND c.inviter_character_id=e.actor
 ORDER BY e.slot,e.entry_no
").bind(&accounts).bind(&worlds).bind(&characters).fetch_all(&mut **tx).await?;
    let mut seen = vec![BTreeSet::new(); reads.len()];
    for row in rows {
        let index = usize::try_from(row.try_get::<i64, _>("slot")?)
            .ok()
            .and_then(|n| n.checked_sub(1))
            .filter(|n| *n < reads.len())
            .ok_or_else(corrupt)?;
        if reads[index].entries.len() >= LIST_MAX {
            return Err(PresenceError::Capacity);
        }
        reads[index]
            .entries
            .push(decode_entry(&row, &mut seen[index])?);
    }
    Ok(reads)
}

type Reservation = (GameSessionId, u64, u64);
struct BatchReservations<'a> {
    owner: &'a PresenceOwner,
    tickets: Vec<Reservation>,
    armed: bool,
}
impl Drop for BatchReservations<'_> {
    fn drop(&mut self) {
        if self.armed
            && let Ok(mut state) = self.owner.state.lock()
        {
            let now = clock().unwrap_or(state.last_clock).max(state.last_clock);
            state.abort_batch(&self.tickets, now);
        }
    }
}
impl State {
    fn abort_batch(&mut self, tickets: &[Reservation], now: u64) {
        for &(session, ticket, epoch) in tickets {
            let failed = self.failed(session, ticket, epoch, now).is_err();
            if let Some(w) = self
                .sessions
                .get_mut(&session)
                .filter(|w| w.ticket == ticket)
            {
                // Invalidate even cancelled tickets immediately, without requiring another read.
                w.ticket = 0;
                w.reading = false;
                w.dirty = true;
                if failed {
                    w.pending.clear();
                    w.inflight = None;
                    for entry in w.entries.values_mut() {
                        entry.desired = Status::Unconfirmed;
                    }
                }
            }
        }
    }
    fn finish_batch(
        &mut self,
        tickets: &[Reservation],
        reads: Vec<WatcherRead>,
        started: u64,
        now: u64,
    ) -> Result<()> {
        self.expire(now)?;
        if reads.len() != tickets.len() {
            return Err(corrupt());
        }
        let mut staged = Vec::with_capacity(reads.len());
        for ((session, ticket, epoch), read) in tickets.iter().copied().zip(reads) {
            let serial = self.next_serial()?;
            let next = self.prepare_finish(
                Completion {
                    session,
                    ticket,
                    epoch,
                    started,
                    now,
                    reason: Refresh::Reconcile,
                    serial,
                },
                read,
            )?;
            staged.push((session, next));
        }
        // No watcher or reverse-index replacement occurs until every observation is valid.
        for (session, next) in staged {
            self.replace_watcher(session, Some(next));
        }
        Ok(())
    }
}
impl PresenceOwner {
    /// Bounded full node batch for reconcile, listener recovery or visibility work. Every
    /// watcher supplies fresh independent authority; this returns observations, never grants.
    pub(crate) async fn refresh_batch_current(
        &self,
        watchers: &[BatchWatcher<'_, '_, '_>],
    ) -> Result<()> {
        let started = clock()?;
        if watchers.len() > self.state.lock().map_err(|_| corrupt())?.capacity {
            return Err(PresenceError::Capacity);
        }
        let mut seen = BTreeSet::new();
        let mut inputs = Vec::with_capacity(watchers.len());
        for watcher in watchers {
            if !seen.insert(watcher.fence.game_session_id) {
                return Err(PresenceError::StaleWork);
            }
            inputs.push(BatchReadInput {
                recovery: watcher
                    .authority
                    .record_for(&self.root)
                    .map_err(|_| PresenceError::AuthorityRejected)?,
                node: watcher.node.clone(),
                fence: watcher.fence,
            });
        }
        let mut reservations = BatchReservations {
            owner: self,
            tickets: Vec::new(),
            armed: true,
        };
        {
            let mut state = self.state.lock().map_err(|_| corrupt())?;
            if watchers.len() > state.capacity {
                return Err(PresenceError::Capacity);
            }
            state.expire(started)?;
            for watcher in watchers {
                let (ticket, epoch, _) =
                    state.reserve_at(watcher.fence, watcher.last_seen_revision, started)?;
                reservations
                    .tickets
                    .push((watcher.fence.game_session_id, ticket, epoch));
            }
        }
        if inputs.is_empty() {
            reservations.armed = false;
            return Ok(());
        }
        let reads = self.root.read_current_vip_batch(inputs).await?;
        self.state.lock().map_err(|_| corrupt())?.finish_batch(
            &reservations.tickets,
            reads,
            started,
            clock()?,
        )?;
        reservations.armed = false;
        Ok(())
    }
}
