//! The native actor owner's side of the durable last position (CHAR-POSITION-1, decision
//! CHAR-POSITION-0, `reviews/OTERYN_GAME_CHAR_POSITION0_LOGOUT_POSITION_DECISION_2026-09-30.md`
//! §3.2 and §3.3), with the Character's saved health and mana (CHAR-VITALS-1, owner decision
//! 2026-10-13, `reviews/OTERYN_GAME_CHAR_VITALS0_DURABLE_VITALS_DECISION_2026-10-13.md`).
//!
//! - **Admission (§3.3).** When a new runtime actor is admitted, its durable position is read
//!   under the session fence before the first-entry Channel-owner step: a pending respawn, else
//!   the Character's last position. It replaces `entry_start` as that step's input: the actor
//!   is placed in the same owner step that initializes its position, before the lock is
//!   released, so party presence and the familiar's login placement, which run after it, read
//!   the placed cell. A pending respawn's cell, or a valid last position (same World, same
//!   coordinate frame, a Walkable cell of the pinned movement cells that no closed door blocks,
//!   and not a house tile), places the actor there or by the NPC-0 §6.1 fallback (nearest free
//!   admissible cell within 3, else `entry_start`). Anything else (a house tile, another World
//!   or frame, no row) leaves the actor at `entry_start`. A read that fails (a storage or fence
//!   error) or a placement that cannot complete leaves the admission not playable, as a pending
//!   respawn that cannot be placed does: the actor never becomes input-eligible at a location
//!   its durable row did not decide. Resume reads nothing.
//! - **Tracking.** Only a session whose placement was decided from a successful read (or a
//!   consumed respawn) is tracked, so a failed read never lets the new session overwrite the
//!   stored position with `entry_start`.
//! - **Periodic write (§3.2, `CHARPOS0-RL-01`).** Every [`LAST_POSITION_WRITE_INTERVAL`], a
//!   tracked actor whose cell (or vitals) changed since its last write is written under the
//!   session fence.
//!   The writer runs on its own loop and wakes at the earliest absolute deadline, each deadline
//!   measured from the capture the stored row holds, with [`WRITE_PASS_SLACK`] left for the
//!   pass itself; a capture later than the interval is reported. A pass works in batches of
//!   [`LAST_POSITION_WRITE_BATCH`] sessions, each one fence read and one fenced write
//!   transaction, so even [`MAX_TRACKED_SESSIONS`] sessions due together take at most
//!   [`MAX_WRITE_PASS`] of durability work (each pass bounded by [`DB_PASS_DEADLINE`]), and a
//!   due session's batch completes at most [`MAX_WRITE_LAG`] after its deadline, within the
//!   slack: every due session is captured and written, or its batch failed, within the
//!   interval.
//! - **Death.** A dead actor is never captured. The in-session placement of its respawn upserts
//!   the respawn cell in the transaction that places the actor and consumes the respawn
//!   (DEATH-RESTART-SAFETY-1); a respawn kept for the next admission is written by that
//!   admission's consumption. No capture of the dead actor may follow either.
//! - **Final write (§3.2 "At logout").** The terminal release captures the fenced actor's cell
//!   and the store upserts it inside the release transaction.
//! - **Shutdown.** Before readiness is withdrawn, every tracked actor is written, whatever its
//!   interval, so a planned restart loses no movement. A failed write is retried; the flush is
//!   done only when every tracked session was written or refused (the node bounds the wait).
//! - **Vitals (CHAR-VITALS-1).** Every capture also reads the living actor's health and mana
//!   under the same runtime lock (`None` when both are at their maxima; the vitals last written
//!   when they cannot be read), so the row's cell and vitals are written by one upsert under one
//!   key and fence. A periodic write is due when the cell or the vitals changed. A respawn
//!   placement writes no vitals: the respawned actor is at its maxima (DEATH-2 §4.5). A newly
//!   created actor placed from a stored row takes that row's vitals, capped at its maxima, in the
//!   owner step that creates its spell state ([`admitted_vitals`]); a respawn admission, a first
//!   login or a row without vitals starts at the maxima (SPELL-D2).
//!
//! Q16 extends this same tracker/cadence to actual named-House residents. A fresh
//! admission with scoped House metadata is restored before the generic Channel
//! first-entry steps above. Its immutable native entry acknowledgement and origin
//! routing cell remain separate from this current position projection.
//!
//! Each capture takes its `write_sequence` under the runtime lock from one process-wide
//! generator that is strictly increasing and never below the wall clock in microseconds, so a
//! write captured later always orders after one captured earlier in the same lease, including
//! the final write after an in-flight periodic one.

use crate::map::boot::BundleCollisionIndexExt;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::{ComposedFreshAdmission, operator_event, respawn_cell, respawn_cell_admissible};
use crate::domain::CharacterRevision;
use crate::durability::DB_PASS_DEADLINE;
use crate::durability::bed_regen::BedPremiumGate;
use crate::durability::character_build::BuildOccurrence;
use crate::durability::character_death::ConsumedRespawn;
use crate::durability::character_last_position::{
    FinalLastPosition, HouseLastPosition, LAST_POSITION_WRITE_BATCH, LastPositionCell,
    LastPositionRecord, LastPositionWrite, LastVitals, PeriodicLastPosition,
    ScopedPeriodicLastPosition, StoredLastPosition,
};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::fresh_admission::FreshAdmissionStore;
use {
    crate::foundation::{
        ChannelRuntimeV1, ExactActorRef, FirstEntryPosition, GameSessionId, MovementLocalPosition,
        WorldId,
    },
    oteryn_gameplay_domain::domain::carrier_values::CarrierError,
};

/// `CHARPOS0-RL-01`: the interval between periodic last-position writes of one live Character
/// while its position has changed. The registry bounds it to 60..=300 seconds.
pub(super) const LAST_POSITION_WRITE_INTERVAL: Duration = Duration::from_secs(300);

/// The part of the interval left for a write pass itself, after the deadline it woke for: its
/// captures and its batches of fence reads and writes ([`MAX_WRITE_LAG`]).
const WRITE_PASS_SLACK: Duration = Duration::from_secs(60);

/// The delay from a session's last durable (or unchanged) capture to its next deadline.
const NEXT_DUE: Duration = LAST_POSITION_WRITE_INTERVAL.saturating_sub(WRITE_PASS_SLACK);

/// The most sessions a Channel tracks at once: the 500 player slots its carrier reserves for
/// player admission (`SPAWN_ACTIVATION_PLAYER_HEADROOM`, the D128 target of 500 concurrent
/// players per Channel). Not the listener's connection limit: a track outlives its socket while
/// the actor is held for reconnect, and a freed connection admits another character.
const MAX_TRACKED_SESSIONS: usize = 500;

/// The batches of a pass in which every tracked session of a Channel is due.
const MAX_WRITE_BATCHES: usize = MAX_TRACKED_SESSIONS.div_ceil(LAST_POSITION_WRITE_BATCH);

/// The most durability work of one batch: one root pass reading the sessions' fences and one
/// write transaction, each bounded by [`DB_PASS_DEADLINE`] (a pass whose holder is busy fails at
/// once rather than waits).
const MAX_BATCH_WORK: Duration = DB_PASS_DEADLINE.saturating_mul(2);

/// The most durability work of one write pass, every tracked session due in it.
const MAX_WRITE_PASS: Duration = MAX_BATCH_WORK.saturating_mul(MAX_WRITE_BATCHES as u32);

/// The latest a due session's batch completes after its deadline: the rest of the pass running
/// at that deadline, [`WRITER_MIN_PAUSE`], then the pass that takes the session, up to its own
/// batch. A deadline recurs only [`NEXT_DUE`] after a capture, so no session is in both passes:
/// together they hold at most [`MAX_TRACKED_SESSIONS`] sessions, in at most
/// `MAX_WRITE_BATCHES + 1` batches. It must fit in [`WRITE_PASS_SLACK`], so every due session
/// is captured and written (or its batch failed) within [`LAST_POSITION_WRITE_INTERVAL`].
const MAX_WRITE_LAG: Duration = MAX_BATCH_WORK
    .saturating_mul(MAX_WRITE_BATCHES as u32 + 1)
    .saturating_add(WRITER_MIN_PAUSE);

// `CHARPOS0-RL-01` under load, at compile time: a pass of every tracked session, and the lag of
// any due session behind its deadline, fit in the slack the deadline leaves.
const _: () = assert!(MAX_WRITE_PASS.as_millis() <= WRITE_PASS_SLACK.as_millis());
const _: () = assert!(MAX_WRITE_LAG.as_millis() <= WRITE_PASS_SLACK.as_millis());
// A track outlives its socket, so the tracked sessions may exceed the open connections.
const _: () = assert!(MAX_TRACKED_SESSIONS > super::MAX_CONNECTIONS);

/// The longest the writer sleeps, so a newly tracked session is seen without a wake-up.
const WRITER_MAX_SLEEP: Duration = super::QUEST_RETRY_CADENCE;

/// The shortest pause between passes, so a failing store is not retried in a tight loop.
const WRITER_MIN_PAUSE: Duration = Duration::from_secs(1);

/// The shutdown flush's passes and the pause between them; the node's own flush budget
/// (3 s) normally ends the wait first.
const FLUSH_ATTEMPTS: usize = 8;
const FLUSH_RETRY_PAUSE: Duration = Duration::from_millis(250);

/// The node's shutdown write of every tracked last position: requested by the node before it
/// withdraws readiness (which fences every session write), and marked done by the gameplay seam
/// once each tracked session was written or refused.
#[derive(Debug, Default)]
pub struct LastPositionFlush {
    requested: oteryn_foundation::CancellationToken,
    done: oteryn_foundation::CancellationToken,
}

impl LastPositionFlush {
    /// Ask the seam for the shutdown write.
    pub fn request(&self) {
        self.requested.cancel();
    }

    /// Resolves once the shutdown write is done.
    pub async fn done(&self) {
        self.done.cancelled().await;
    }
    /// The seam's side: wait for the request, write every tracked session, retrying failed
    /// writes, and mark it done only once none failed. The node stops waiting at its budget.
    pub(super) async fn serve(&self, authority: &ComposedFreshAdmission<'_, '_, '_>) {
        self.requested.cancelled().await;
        // OPS-SAVE-2: admissions and player commands stop, and those in flight finish, before
        // the first snapshot, so no write below is overtaken. Server-driven work is not input.
        authority.states.input_gate.quiesce().await;
        let mut monks_settled = std::collections::HashSet::new();
        let mut training_settled = std::collections::HashSet::new();
        let mut wheel_gifts_settled = std::collections::HashSet::new();
        for _ in 0..FLUSH_ATTEMPTS {
            // OPS-SAVE-1: the monk values of the live actors go with the positions.
            // Input is already quiesced through the shared node gate. Keep a
            // participant from moving between two owner snapshots in this pass;
            // all checkpoints below remain the existing exact-scope writers.
            let transition = authority.activity_runtimes.transitions.lock().await;
            let mut failed = authority.drain_paid_quest_callbacks().await
                + authority.write_due_last_positions(true).await
                + authority.flush_wheel_gifts(&mut wheel_gifts_settled).await
                + authority.flush_monk_states(&mut monks_settled).await
                + authority.flush_spell_training(&mut training_settled).await;
            match authority.activity_runtimes.activity_owners() {
                Ok(owners) => {
                    for owner in owners {
                        let view = authority.runtime_view(&owner);
                        failed += view.drain_paid_quest_callbacks().await
                            + view.write_due_last_positions(true).await
                            + view.flush_monk_states(&mut monks_settled).await
                            + view.flush_spell_training(&mut training_settled).await;
                    }
                }
                Err(()) => failed += 1,
            }
            drop(transition);
            if failed == 0 {
                self.done.cancel();
                return;
            }
            tokio::time::sleep(FLUSH_RETRY_PAUSE).await;
        }
        // Never done: the node's budget ends its wait, and the shutdown is reported incomplete.
        operator_event("last_position_flush_incomplete");
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// OPS-SAVE-2: the spell-training checkpoint of every tracked live actor, the forced
    /// actor-end save run while the session is still Active (a terminal session is fenced out).
    /// Returns the number of unproven saves, which the caller retries; settled sessions are
    /// skipped. Familiar state is not flushed: its save is a logout save, valid only after the
    /// grace deadline.
    pub(super) async fn flush_spell_training(
        &self,
        settled: &mut std::collections::HashSet<GameSessionId>,
    ) -> usize {
        let mut failed = 0;
        for (session, actor) in self.states.last_positions.tracked_actors() {
            if settled.contains(&session) {
                continue;
            }
            // In-memory pre-filter: an actor with nothing to checkpoint costs no store read.
            if !self.has_training_to_flush(session, actor).await {
                settled.insert(session);
                continue;
            }
            if self.save_spell_training(actor, session, true).await
                == super::actor_spell::TrainingSave::Unknown
            {
                failed += 1;
            } else {
                settled.insert(session);
            }
        }
        failed
    }
    async fn has_training_to_flush(&self, session: GameSessionId, actor: ExactActorRef) -> bool {
        let runtime = self.runtime.lock().await;
        if runtime.player_control_facts(actor, session).is_ok() {
            // Proficiency progresses independently of spell training. Queued deaths may
            // still owe this contributor credit when shutdown takes its first snapshot.
            let pending = self.states.proficiencies.lock().map_or(true, |states| {
                states
                    .get(&session)
                    .is_some_and(|state| state.checkpoint_due(self.owner_now().get(), true))
            });
            if pending
                || !self
                    .states
                    .attack
                    .lock()
                    .await
                    .kills()
                    .proficiency_principals(session)
                    .is_empty()
            {
                return true;
            }
        }
        self.states
            .spell_states
            .lock()
            .await
            .has_training_to_flush(&runtime, actor, session)
    }
}
/// The tracked sessions and the write-sequence generator. Never held across an await.
#[derive(Debug, Default)]
pub struct LastPositions {
    tracks: std::sync::Mutex<HashMap<GameSessionId, Track>>,
    sequence: AtomicU64,
}

#[derive(Debug, Clone, Copy)]
struct Track {
    actor: ExactActorRef,
    /// The Character revision of the admission fence. A periodic write fills the fence's
    /// `expected_character_revision` with it; the write does not read it (no root lock).
    revision: CharacterRevision,
    /// The cell and vitals last written or read for this Character; `None` when nothing is
    /// stored.
    written: Option<Saved>,
    written_at: Instant,
    trial_refill: Option<BuildOccurrence>,
    /// When the cell and vitals the stored row holds (or unchanged ones) were last captured.
    checked: Instant,
}

impl Track {
    fn due(&self) -> Instant {
        self.checked + NEXT_DUE
    }
}

/// What a stored row holds that a periodic write compares with a capture: the cell and the
/// vitals (CHAR-VITALS-1). The digests are of the pinned content and do not change in a process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Saved {
    cell: LastPositionCell,
    vitals: Option<LastVitals>,
    rest: Option<[u32; 4]>,
}

impl Saved {
    const fn of(record: &LastPositionRecord) -> Self {
        Self {
            cell: record.cell,
            vitals: record.vitals,
            rest: record.rest,
        }
    }
}

#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RetainedCaptureBoundary {
    BeforeSlot,
    AfterCapture,
}

#[cfg(test)]
tokio::task_local! {
    pub(super) static RETAINED_CAPTURE_PAUSE: (RetainedCaptureBoundary, std::sync::Arc<tokio::sync::Notify>, std::sync::Arc<tokio::sync::Notify>);
}

#[cfg(test)]
async fn retained_capture_pause(at: RetainedCaptureBoundary) {
    if RETAINED_CAPTURE_PAUSE.try_with(|pause| pause.0 == at) == Ok(true) {
        RETAINED_CAPTURE_PAUSE.with(|pause| pause.1.notify_one());
        let proceed = RETAINED_CAPTURE_PAUSE.with(|pause| pause.2.clone());
        proceed.notified().await;
    }
}

#[derive(Clone, Copy)]
enum VocationCapture {
    Hold,
    Recheck(crate::durability::character_build::BuildOccurrence),
    Final,
}

#[derive(Clone, Copy)]
struct RetainedPosition {
    character: crate::domain::CharacterId,
    occurrence: crate::durability::character_build::BuildOccurrence,
}

/// One capture attempt under the runtime lock.
enum Captured {
    At(Box<Capture>),
    /// The actor is dead: the placement of its respawn writes the respawn cell.
    Dead,
    /// A retained vocation commit awaits live installation.
    Vocation(RetainedPosition),
    /// Another gameplay owner is still settling this actor.
    Held,
    /// The actor has no current position in the pinned context.
    Gone,
}

/// One captured position: its record and the sequence it was captured with.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Capture {
    record: LastPositionRecord,
    house: Option<HouseLastPosition>,
    write_sequence: u64,
    at: Instant,
    pending_vocation_refill: Option<BuildOccurrence>,
}

impl LastPositions {
    /// The next write sequence: strictly greater than every earlier one of this process and
    /// never below the wall clock in microseconds, so a later process of the same lease (none
    /// exists today) still orders after this one.
    pub(super) fn next_sequence(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|elapsed| u64::try_from(elapsed.as_micros()).ok())
            .unwrap_or(0);
        let mut previous = self.sequence.load(Ordering::Relaxed);
        loop {
            let next = previous.saturating_add(1).max(now).max(1);
            match self.sequence.compare_exchange_weak(
                previous,
                next,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return next,
                Err(current) => previous = current,
            }
        }
    }

    fn track(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
        written: Option<Saved>,
        revision: CharacterRevision,
    ) {
        if let Ok(mut tracks) = self.tracks.lock() {
            let now = Instant::now();
            tracks.insert(
                session,
                Track {
                    actor,
                    revision,
                    written,
                    written_at: now,
                    trial_refill: None,
                    checked: now,
                },
            );
        }
    }

    /// Native scope handoff remaps the same tracked actor without a second timer/sequence owner.
    pub(super) fn transferred(
        &self,
        source: GameSessionId,
        destination: GameSessionId,
        actor: ExactActorRef,
        record: &LastPositionRecord,
    ) -> bool {
        if let Ok(mut tracks) = self.tracks.lock()
            && let Some(mut track) = tracks
                .get(&source)
                .copied()
                .filter(|track| track.actor == actor)
        {
            tracks.remove(&source);
            track.written = Some(Saved::of(record));
            track.checked = Instant::now();
            track.written_at = track.checked;
            tracks.insert(destination, track);
            return true;
        }
        false
    }

    pub(super) fn forget(&self, session: GameSessionId) {
        if let Ok(mut tracks) = self.tracks.lock() {
            tracks.remove(&session);
        }
    }

    fn tracked(&self, session: GameSessionId) -> Option<Track> {
        self.tracks.lock().ok()?.get(&session).copied()
    }

    /// OPS-SAVE-1: every tracked session with its actor, for the shutdown flush of actor state.
    pub(super) fn tracked_actors(&self) -> Vec<(GameSessionId, ExactActorRef)> {
        self.tracks
            .lock()
            .map(|tracks| {
                tracks
                    .iter()
                    .map(|(session, track)| (*session, track.actor))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The sessions due at `now`, or every tracked session when `all`, the most overdue first.
    fn due(&self, now: Instant, all: bool) -> Vec<(GameSessionId, Track)> {
        let mut due: Vec<_> = self
            .tracks
            .lock()
            .map(|tracks| {
                tracks
                    .iter()
                    .filter(|(_, track)| all || track.due() <= now)
                    .map(|(session, track)| (*session, *track))
                    .collect()
            })
            .unwrap_or_default();
        due.sort_by_key(|(_, track)| track.checked);
        due
    }

    /// The earliest deadline of any tracked session.
    fn next_due(&self) -> Option<Instant> {
        self.tracks.lock().ok()?.values().map(Track::due).min()
    }

    /// A capture at `at` of `actor` is durable (`saved`) or unchanged (`None`): its next deadline
    /// is measured from `at`. A track replaced by another actor in the meantime is left alone.
    fn settled(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
        saved: Option<Saved>,
        at: Instant,
    ) {
        if let Ok(mut tracks) = self.tracks.lock()
            && let Some(track) = tracks.get_mut(&session)
            && track.actor == actor
        {
            if saved.is_some() && track.written_at <= at {
                track.written = saved;
                track.written_at = at;
            }
            track.checked = track.checked.max(at);
        }
    }

    /// DEATH-2: the in-session placement of `session`'s respawn consumed it and wrote (or an
    /// earlier attempt of it wrote) its respawn `cell`, with no vitals (the maxima).
    pub(super) fn respawned(&self, session: GameSessionId, cell: LastPositionCell) {
        if let Ok(mut tracks) = self.tracks.lock()
            && let Some(track) = tracks.get_mut(&session)
        {
            track.written = Some(Saved {
                cell,
                vitals: None,
                rest: None,
            });
            track.checked = Instant::now();
            track.written_at = track.checked;
            track.trial_refill = None;
        }
    }
}

const fn cell_of(position: MovementLocalPosition) -> LastPositionCell {
    LastPositionCell {
        x: position.x,
        y: position.y,
        floor: position.floor,
    }
}

const fn position_of(cell: LastPositionCell) -> MovementLocalPosition {
    MovementLocalPosition {
        x: cell.x,
        y: cell.y,
        floor: cell.floor,
    }
}

include!("last_position/admission.rs");

include!("last_position/capture.rs");

include!("last_position/periodic.rs");

impl LastPositions {
    pub(super) fn transfer_activity(
        &self,
        destination: &Self,
        old: GameSessionId,
        new: GameSessionId,
        actor: ExactActorRef,
    ) -> Result<(), ()> {
        let mut source = self.tracks.lock().map_err(|_| ())?;
        let mut target = destination.tracks.lock().map_err(|_| ())?;
        if target.contains_key(&new) {
            return if source.contains_key(&old) {
                Err(())
            } else {
                Ok(())
            };
        }
        target.try_reserve(1).map_err(|_| ())?;
        if let Some(mut track) = source.remove(&old) {
            track.actor = actor;
            // The durable handoff wrote its own placement. First periodic pass must
            // compare the new actual scope; the source's cell is never assumed current.
            track.written = None;
            track.checked = Instant::now();
            track.written_at = track.checked;
            target.insert(new, track);
        }
        Ok(())
    }
}

include!("last_position/tests.rs");
