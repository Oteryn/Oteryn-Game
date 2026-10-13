//! The Channel owner's side of the durable last position (CHAR-POSITION-1, decision
//! CHAR-POSITION-0, `reviews/OTERYN_GAME_CHAR_POSITION0_LOGOUT_POSITION_DECISION_2026-09-30.md`
//! §3.2 and §3.3).
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
//!   tracked actor whose cell changed since its last write is written under the session fence.
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
//!
//! Each capture takes its `write_sequence` under the runtime lock from one process-wide
//! generator that is strictly increasing and never below the wall clock in microseconds, so a
//! write captured later always orders after one captured earlier in the same lease, including
//! the final write after an in-flight periodic one.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::{ComposedFreshAdmission, operator_event, respawn_cell, respawn_cell_admissible};
use crate::domain::CharacterRevision;
use crate::durability::DB_PASS_DEADLINE;
use crate::durability::character_death::ConsumedRespawn;
use crate::durability::character_last_position::{
    FinalLastPosition, LAST_POSITION_WRITE_BATCH, LastPositionCell, LastPositionRecord,
    LastPositionWrite, PeriodicLastPosition, StoredLastPosition,
};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, FirstEntryPosition, GameSessionId,
    MovementLocalPosition, WorldId,
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
pub(crate) struct LastPositionFlush {
    requested: oteryn_foundation::CancellationToken,
    done: oteryn_foundation::CancellationToken,
}

impl LastPositionFlush {
    /// Ask the seam for the shutdown write.
    pub(crate) fn request(&self) {
        self.requested.cancel();
    }

    /// Resolves once the shutdown write is done.
    pub(crate) async fn done(&self) {
        self.done.cancelled().await;
    }

    /// The seam's side: wait for the request, write every tracked session, retrying failed
    /// writes, and mark it done only once none failed. The node stops waiting at its budget.
    pub(super) async fn serve(&self, authority: &ComposedFreshAdmission<'_, '_, '_>) {
        self.requested.cancelled().await;
        // OPS-SAVE-2: admissions and player commands stop, and those in flight finish, before
        // the first snapshot, so no write below is overtaken. Server-driven work is not input.
        authority.input_gate.quiesce().await;
        let mut monks_settled = std::collections::HashSet::new();
        let mut training_settled = std::collections::HashSet::new();
        for _ in 0..FLUSH_ATTEMPTS {
            // OPS-SAVE-1: the monk values of the live actors go with the positions.
            let failed = authority.write_due_last_positions(true).await
                + authority.flush_monk_states(&mut monks_settled).await
                + authority.flush_spell_training(&mut training_settled).await;
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
        for (session, actor) in self.last_positions.tracked_actors() {
            if settled.contains(&session) {
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
}

/// The tracked sessions and the write-sequence generator. Never held across an await.
#[derive(Debug, Default)]
pub(crate) struct LastPositions {
    tracks: std::sync::Mutex<HashMap<GameSessionId, Track>>,
    sequence: AtomicU64,
}

#[derive(Debug, Clone, Copy)]
struct Track {
    actor: ExactActorRef,
    /// The Character revision of the admission fence. A periodic write fills the fence's
    /// `expected_character_revision` with it; the write does not read it (no root lock).
    revision: CharacterRevision,
    /// The cell last written or read for this Character; `None` when nothing is stored.
    written: Option<LastPositionCell>,
    /// When the cell the stored row holds (or an unchanged one) was last captured.
    checked: Instant,
}

impl Track {
    fn due(&self) -> Instant {
        self.checked + NEXT_DUE
    }
}

/// One capture attempt under the runtime lock.
enum Captured {
    At(Capture),
    /// The actor is dead: the placement of its respawn writes the respawn cell.
    Dead,
    /// The actor has no current position in the pinned context.
    Gone,
}

/// One captured position: its record and the sequence it was captured with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Capture {
    record: LastPositionRecord,
    write_sequence: u64,
    at: Instant,
}

impl LastPositions {
    /// The next write sequence: strictly greater than every earlier one of this process and
    /// never below the wall clock in microseconds, so a later process of the same lease (none
    /// exists today) still orders after this one.
    fn next_sequence(&self) -> u64 {
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
        written: Option<LastPositionCell>,
        revision: CharacterRevision,
    ) {
        if let Ok(mut tracks) = self.tracks.lock() {
            tracks.insert(
                session,
                Track {
                    actor,
                    revision,
                    written,
                    checked: Instant::now(),
                },
            );
        }
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

    /// A capture at `at` of `actor` is durable (`cell`) or unchanged (`None`): its next deadline
    /// is measured from `at`. A track replaced by another actor in the meantime is left alone.
    fn settled(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
        cell: Option<LastPositionCell>,
        at: Instant,
    ) {
        if let Ok(mut tracks) = self.tracks.lock()
            && let Some(track) = tracks.get_mut(&session)
            && track.actor == actor
        {
            if cell.is_some() {
                track.written = cell;
            }
            track.checked = track.checked.max(at);
        }
    }

    /// DEATH-2: the in-session placement of `session`'s respawn consumed it and wrote (or an
    /// earlier attempt of it wrote) its respawn `cell`.
    pub(super) fn respawned(&self, session: GameSessionId, cell: LastPositionCell) {
        if let Ok(mut tracks) = self.tracks.lock()
            && let Some(track) = tracks.get_mut(&session)
        {
            track.written = Some(cell);
            track.checked = Instant::now();
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

/// Whether `placed` records exactly `cell`: the in-session respawn upserts only the cell it
/// places the actor at.
pub(super) fn respawn_cell_recorded(
    cell: MovementLocalPosition,
    placed: &FinalLastPosition,
) -> bool {
    placed.record.cell == cell_of(cell)
}

/// §3.3 "valid", the parts the stored row decides: the same World and the same coordinate
/// frame. The map and artifact digests are diagnostics only: a cell of another map revision is
/// re-checked against the active cells, never trusted or skipped.
fn stored_position_applies(
    stored: &StoredLastPosition,
    world_id: WorldId,
    coordinate_frame_digest: [u8; 32],
) -> bool {
    stored.record.world_id == world_id
        && stored.record.coordinate_frame_digest == coordinate_frame_digest
}

/// §3.3 "valid": a house tile is invalid until HOUSE-OWN-0 decides entry. A bundle World reads
/// its map facts; any other World reads its qualified house placements, and a cell whose
/// placement is unknown counts as a house tile (fail closed).
fn house_cell(
    movement_cells: &crate::content::NativeEntryMovementCells,
    cell: MovementLocalPosition,
) -> bool {
    use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
    use crate::map::view::MapFacts;
    let logical = crate::content::LogicalCell {
        x: cell.x,
        y: cell.y,
        z: i32::from(cell.floor),
    };
    if let NativeMovementCollisionIndex::Bundle(index) = movement_cells.index() {
        // The runtime floor is legacy `z` in 0..=15; the native floor is `-z`.
        let tile = i8::try_from(cell.floor)
            .ok()
            .filter(|z| (0..=15).contains(z))
            .and_then(|z| {
                Some(crate::map::overlay::TilePos {
                    x: u16::try_from(cell.x).ok()?,
                    y: u16::try_from(cell.y).ok()?,
                    floor: -z,
                })
            });
        return tile.is_none_or(|tile| index.map().facts.house_tile(tile));
    }
    !matches!(
        movement_cells
            .house_tiles()
            .house_at(movement_cells.scope(), logical),
        Ok(None)
    )
}

/// §3.3: what the durable rows decide for a new runtime actor, read under its session fence
/// before the first-entry Channel-owner step ([`ComposedFreshAdmission::read_admission_position`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum AdmissionPosition {
    /// Step 1: a pending respawn. The first-entry step places the actor at its cell; the
    /// respawn is consumed after the progression step.
    Respawn(ConsumedRespawn),
    /// Step 3: the stored last position, placed only if it is valid (else step 4).
    Stored(StoredLastPosition),
    /// No row: a first login, step 4.
    FirstLogin,
}

/// §3.3: the input that replaces `entry_start` in the first-entry owner step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FirstEntryInput {
    /// Step 4: the entry start itself.
    EntryStart,
    /// Step 1 or 3: this cell, else the NPC-0 §6.1 fallback, else step 4. `None` (an
    /// undecodable respawn record) goes straight to step 4.
    Recorded(Option<MovementLocalPosition>),
}

/// §3.3: the first-entry input `decided` selects. A pending respawn's recorded cell; a stored
/// cell only when it is of `world_id` and `frame_digest` and not a `house` tile (a saved house
/// tile goes straight to step 4, never through the radius fallback); else the entry start.
fn first_entry_input(
    decided: &AdmissionPosition,
    world_id: WorldId,
    frame_digest: [u8; 32],
    house: impl Fn(MovementLocalPosition) -> bool,
) -> FirstEntryInput {
    match decided {
        AdmissionPosition::Respawn(pending) => {
            FirstEntryInput::Recorded(respawn_cell(&pending.respawn_position))
        }
        AdmissionPosition::Stored(stored)
            if stored_position_applies(stored, world_id, frame_digest)
                && !house(position_of(stored.record.cell)) =>
        {
            FirstEntryInput::Recorded(Some(position_of(stored.record.cell)))
        }
        AdmissionPosition::Stored(_) | AdmissionPosition::FirstLogin => FirstEntryInput::EntryStart,
    }
}

/// §3.3 "it replaces `entry_start` as the input to `initialize_position`", in one Channel-owner
/// step: the actor's position is initialized and, before the caller releases the lock and any
/// owner reads it, moved to the cell `input` selects (`admissible` and free, or by the NPC-0 §6.1
/// fallback, else `entry_start`). An initialization that had already completed (`Reconciled`)
/// moves nothing. An error leaves the actor not playable.
pub(super) fn initialize_first_entry_at(
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    input: FirstEntryInput,
    admissible: impl Fn(MovementLocalPosition) -> bool,
) -> Result<FirstEntryPosition, CarrierError> {
    match (runtime.initialize_first_entry_position(actor)?, input) {
        (FirstEntryPosition::Initialized(_), FirstEntryInput::Recorded(cell)) => runtime
            .place_admitted_respawn(actor, session, cell, admissible)
            .map(FirstEntryPosition::Initialized),
        (initialized, _) => Ok(initialized),
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Capture `actor`'s current cell under the runtime lock, with its record and a fresh write
    /// sequence. A dead actor (DEATH-2 §4.2, until its respawn) is not captured.
    async fn capture_last_position(&self, actor: ExactActorRef) -> Captured {
        let runtime = self.runtime.lock().await;
        if self.spell_states.lock().await.is_dead(actor) {
            return Captured::Dead;
        }
        let Ok(snapshot) = runtime.read_actor_position(actor) else {
            return Captured::Gone;
        };
        if snapshot.context() != runtime.pinned_movement_context() {
            return Captured::Gone;
        }
        let placed = self.placed_record(&runtime, snapshot.position());
        Captured::At(Capture {
            record: placed.record,
            write_sequence: placed.write_sequence,
            at: Instant::now(),
        })
    }

    /// The record of `cell` in the pinned context, with a fresh write sequence; taken under the
    /// runtime lock the caller holds.
    pub(super) fn placed_record(
        &self,
        runtime: &crate::foundation::ChannelRuntimeV1,
        cell: MovementLocalPosition,
    ) -> FinalLastPosition {
        let pin = runtime.content_pin();
        FinalLastPosition {
            record: LastPositionRecord {
                world_id: self.world_id,
                cell: cell_of(cell),
                coordinate_frame_digest: pin.frame_binding_digest(),
                map_revision_digest: runtime.map_revision_digest(),
                server_artifact_digest: pin.server_artifact_digest(),
            },
            write_sequence: self.last_positions.next_sequence(),
        }
    }

    /// §3.2 "At logout": the final position of a tracked session's fenced actor, which the
    /// terminal release writes in its own transaction. `None` for an untracked session.
    pub(super) async fn final_last_position(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
    ) -> Option<FinalLastPosition> {
        let track = self.last_positions.tracked(session)?;
        if track.actor != actor {
            return None;
        }
        match self.capture_last_position(actor).await {
            Captured::At(capture) => Some(FinalLastPosition {
                record: capture.record,
                write_sequence: capture.write_sequence,
            }),
            Captured::Dead | Captured::Gone => None,
        }
    }

    /// The release store of `session`: `store` carrying the final position of a tracked
    /// session's fenced actor, or `store` itself.
    pub(super) async fn final_position_store(
        &self,
        store: &FreshAdmissionStore,
        session: GameSessionId,
        actor: ExactActorRef,
    ) -> FreshAdmissionStore {
        match self.final_last_position(session, actor).await {
            Some(position) => store.with_final_position(position),
            None => store.clone(),
        }
    }

    /// DEATH-2b with CHAR-POSITION-0 §3.3 step 1: the record the respawn consumption upserts for
    /// the placed cell; the session is tracked once the consumption settles.
    pub(super) async fn respawn_last_position(
        &self,
        actor: ExactActorRef,
    ) -> Option<FinalLastPosition> {
        let runtime = self.runtime.lock().await;
        let snapshot = runtime.read_actor_position(actor).ok()?;
        Some(self.placed_record(&runtime, snapshot.position()))
    }

    /// Track `session` after its respawn consumption under `fence` wrote `placed`.
    pub(super) fn track_respawned(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
        placed: &FinalLastPosition,
        fence: CurrentCharacterGameplayFence,
    ) {
        self.last_positions.track(
            session,
            actor,
            Some(placed.record.cell),
            fence.expected_character_revision,
        );
    }

    /// §3.3: the durable position of a new runtime actor, read under `fence` before the
    /// first-entry owner step: a pending respawn (step 1), else the last position (steps 3 and
    /// 4). `Err` for a failed read (a storage or fence error): the actor is left unpositioned
    /// and the admission is not playable.
    pub(super) async fn read_admission_position(
        &self,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<AdmissionPosition, ()> {
        match self
            .root
            .pending_respawn(self.character, self.holder, fence)
            .await
        {
            Ok(Some(pending)) => return Ok(AdmissionPosition::Respawn(pending)),
            Ok(None) => {}
            Err(error) => {
                operator_event(&format!("pending_respawn_read_failed reason={error}"));
                return Err(());
            }
        }
        match self
            .root
            .read_character_last_position(self.character, self.holder, fence)
            .await
        {
            Ok(Some(stored)) => Ok(AdmissionPosition::Stored(stored)),
            Ok(None) => Ok(AdmissionPosition::FirstLogin),
            Err(error) => {
                operator_event(&format!("last_position_read_failed reason={error}"));
                Err(())
            }
        }
    }

    /// §3.3 in the first-entry owner step, under the Channel-owner lock the caller holds:
    /// initialize `actor` and place it at the cell `decided` selects
    /// ([`initialize_first_entry_at`]), so every owner that reads its position after this step
    /// sees the placed cell.
    pub(super) async fn initialize_admitted_position(
        &self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        decided: &AdmissionPosition,
    ) -> Result<FirstEntryPosition, CarrierError> {
        let (pinned_digest, frame_digest) = {
            let pin = runtime.content_pin();
            (pin.server_artifact_digest(), pin.frame_binding_digest())
        };
        let input = first_entry_input(decided, self.world_id, frame_digest, |cell| {
            house_cell(self.movement_cells, cell)
        });
        // A respawn's fallback is DEATH-2b's; a stored cell's fallback never lands on a house
        // tile either.
        let stored = matches!(decided, AdmissionPosition::Stored(_));
        let door = self.door.lock().await;
        initialize_first_entry_at(runtime, actor, session, input, |cell| {
            respawn_cell_admissible(
                self.movement_cells.index(),
                self.movement_cells.scope(),
                self.world_id,
                pinned_digest,
                door.blocking_cells(),
                cell,
            ) && !(stored && house_cell(self.movement_cells, cell))
        })
    }

    /// §3.3 steps 3 and 4, once no pending respawn is left to consume: track `session` with the
    /// stored cell its first entry was decided by. `false` when that entry was decided by a
    /// pending respawn that is no longer pending: the admission is not playable, nothing is
    /// written and the session stays untracked, so it never overwrites the stored row.
    pub(super) fn track_restored(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
        decided: &AdmissionPosition,
        fence: CurrentCharacterGameplayFence,
    ) -> bool {
        let written = match decided {
            AdmissionPosition::Stored(stored) => Some(stored.record.cell),
            AdmissionPosition::FirstLogin => None,
            AdmissionPosition::Respawn(_) => {
                operator_event("admitted_respawn_no_longer_pending");
                return false;
            }
        };
        self.last_positions
            .track(session, actor, written, fence.expected_character_revision);
        true
    }

    /// §3.2 periodic writes (`CHARPOS0-RL-01`): sleep until the earliest deadline (at most
    /// [`WRITER_MAX_SLEEP`], at least [`WRITER_MIN_PAUSE`]), then write every due session. It
    /// never ends on its own and parks once `shutdown` is requested.
    pub(super) async fn run_last_position_writes(
        &self,
        shutdown: &oteryn_foundation::CancellationToken,
    ) {
        loop {
            let now = Instant::now();
            let wake = self
                .last_positions
                .next_due()
                .map_or(now + WRITER_MAX_SLEEP, |due| {
                    due.min(now + WRITER_MAX_SLEEP)
                })
                .max(now + WRITER_MIN_PAUSE);
            if super::first(
                tokio::time::sleep_until(tokio::time::Instant::from_std(wake)),
                shutdown.cancelled(),
            )
            .await
            .is_none()
            {
                std::future::pending::<()>().await;
            }
            self.write_due_last_positions(false).await;
        }
    }

    /// One pass of §3.2 writes: the due sessions, most overdue first, or with `all` the
    /// shutdown write of every tracked session, in batches of [`LAST_POSITION_WRITE_BATCH`]. A
    /// cell unchanged since its last write writes nothing; a dead actor is skipped. Returns the
    /// number of failed writes, which stay due.
    pub(super) async fn write_due_last_positions(&self, all: bool) -> usize {
        let due = self.last_positions.due(Instant::now(), all);
        let mut failed = 0;
        for batch in due.chunks(LAST_POSITION_WRITE_BATCH) {
            failed += self.write_last_position_batch(batch).await;
        }
        failed
    }

    /// One batch of a pass: capture each session under the runtime lock, then read the changed
    /// sessions' fences in one root pass and write them in one transaction, so the batch costs
    /// two durability passes whatever its size (`CHARPOS0-RL-01`). Returns the failed writes.
    async fn write_last_position_batch(&self, batch: &[(GameSessionId, Track)]) -> usize {
        let mut changed = Vec::with_capacity(batch.len());
        for &(session, track) in batch {
            let capture = match self.capture_last_position(track.actor).await {
                Captured::At(capture) => capture,
                // The placement of its respawn writes the respawn cell.
                Captured::Dead => continue,
                Captured::Gone => {
                    // The actor is gone: its session ended without a retire of this track.
                    self.last_positions.forget(session);
                    continue;
                }
            };
            if capture.at.saturating_duration_since(track.checked) > LAST_POSITION_WRITE_INTERVAL {
                operator_event("last_position_interval_exceeded");
            }
            if track.written == Some(capture.record.cell) {
                self.last_positions
                    .settled(session, track.actor, None, capture.at);
                continue;
            }
            changed.push((session, track, capture));
        }
        if changed.is_empty() {
            return 0;
        }
        let current = match FreshAdmissionStore::from_root(self.root.clone())
            .current_sessions_at(changed.iter().map(|(session, _, _)| *session).collect())
            .await
        {
            Ok(current) => current,
            Err(error) => {
                operator_event(&format!("last_position_write_failed reason={error}"));
                return changed.len();
            }
        };
        // `current` has one entry per changed session, in order; a missing one stays due.
        let mut failed = changed.len().saturating_sub(current.len());
        let mut writes = Vec::with_capacity(changed.len());
        let mut fenced = Vec::with_capacity(changed.len());
        for ((session, track, capture), current) in changed.into_iter().zip(current) {
            let Some(current) = current else {
                failed += 1;
                continue;
            };
            // The fence `current_quest_fence` builds, with the admission revision (not read).
            if current.session_state() == crate::foundation::GameSessionState::Terminal {
                self.last_positions.forget(session);
                continue;
            }
            let Ok(character_id) =
                crate::domain::CharacterId::from_bytes(*current.commit().character_id().as_bytes())
            else {
                failed += 1;
                continue;
            };
            writes.push(PeriodicLastPosition {
                fence: CurrentCharacterGameplayFence {
                    character_id,
                    game_session_id: session,
                    connection_generation: current.current_connection_generation(),
                    character_lease_generation: current.current_character_lease().generation(),
                    runtime_scope: current.current_runtime_scope(),
                    scope_ownership_generation: current.current_scope_generation(),
                    expected_character_revision: track.revision,
                },
                write_sequence: capture.write_sequence,
                record: capture.record,
            });
            fenced.push((session, track, capture));
        }
        if writes.is_empty() {
            return failed;
        }
        let outcomes = match self
            .root
            .write_character_last_positions(self.character, self.holder, writes)
            .await
        {
            Ok(outcomes) => outcomes,
            Err(error) => {
                operator_event(&format!("last_position_write_failed reason={error}"));
                return failed + fenced.len();
            }
        };
        failed += fenced.len().saturating_sub(outcomes.len());
        for ((session, track, capture), outcome) in fenced.into_iter().zip(outcomes) {
            match outcome {
                Ok(
                    LastPositionWrite::Written
                    | LastPositionWrite::Superseded
                    | LastPositionWrite::PendingObligation,
                ) => {
                    self.last_positions.settled(
                        session,
                        track.actor,
                        Some(capture.record.cell),
                        capture.at,
                    );
                }
                Err(error) => {
                    failed += 1;
                    operator_event(&format!("last_position_write_failed reason={error}"));
                }
            }
        }
        failed
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    fn world(seed: u8) -> WorldId {
        let mut bytes = [seed; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        WorldId::decode(&bytes).expect("uuid v7")
    }

    fn stored(world_id: WorldId, frame: [u8; 32]) -> StoredLastPosition {
        StoredLastPosition {
            record: LastPositionRecord {
                world_id,
                cell: LastPositionCell {
                    x: 5,
                    y: 6,
                    floor: 7,
                },
                coordinate_frame_digest: frame,
                map_revision_digest: [2; 32],
                server_artifact_digest: [3; 32],
            },
            key: crate::durability::character_last_position::LastPositionKey {
                character_lease_generation: 1,
                write_sequence: 1,
            },
        }
    }

    /// `CHARPOS0-RL-01`: the interval is within the registered 60..=300 seconds, so a changed
    /// position is written within 300 s.
    #[test]
    fn write_interval_is_within_the_registered_range() {
        assert!(LAST_POSITION_WRITE_INTERVAL <= Duration::from_secs(300));
        assert!(LAST_POSITION_WRITE_INTERVAL >= Duration::from_secs(60));
        // The writer wakes at the deadline itself; the slack is left for the pass.
        assert_eq!(NEXT_DUE + WRITE_PASS_SLACK, LAST_POSITION_WRITE_INTERVAL);
        assert!(WRITE_PASS_SLACK >= Duration::from_secs(30));
        assert!(WRITER_MIN_PAUSE < WRITE_PASS_SLACK && WRITER_MAX_SLEEP < WRITE_PASS_SLACK);
        assert!(!NEXT_DUE.is_zero());
    }

    /// `CHARPOS0-RL-01` under load: every tracked session of a Channel due at once is captured
    /// and written within the slack, each batch costing two passes bounded by the pass deadline,
    /// however slow the store is short of failing a pass. The bound is the reserved player
    /// slots, which exceed the listener's connection limit (Codex 4248578094).
    #[test]
    fn a_pass_of_every_tracked_session_fits_in_the_slack() {
        assert_eq!(MAX_TRACKED_SESSIONS, 500);
        assert_eq!(MAX_WRITE_BATCHES, 8);
        assert_eq!(MAX_BATCH_WORK, Duration::from_secs(4));
        assert!(MAX_WRITE_PASS <= WRITE_PASS_SLACK);
        assert!(MAX_WRITE_PASS < MAX_WRITE_LAG);
        assert!(MAX_WRITE_LAG <= WRITE_PASS_SLACK);
        assert!(NEXT_DUE + MAX_WRITE_LAG <= LAST_POSITION_WRITE_INTERVAL);
    }

    /// §3.2 "Order": each capture orders after every earlier one of the process.
    #[test]
    fn sequences_strictly_increase_from_the_wall_clock() {
        let positions = LastPositions::default();
        let first = positions.next_sequence();
        let second = positions.next_sequence();
        assert!(first > 1_600_000_000_000_000, "wall clock microseconds");
        assert!(second > first);
        positions.sequence.store(u64::MAX - 1, Ordering::Relaxed);
        assert_eq!(positions.next_sequence(), u64::MAX);
    }

    /// §3.3 "valid": another World or coordinate frame places at `entry_start`; another map or
    /// artifact digest is re-checked, not refused.
    #[test]
    fn a_stored_position_applies_only_in_its_world_and_frame() {
        let here = world(1);
        assert!(stored_position_applies(
            &stored(here, [1; 32]),
            here,
            [1; 32]
        ));
        assert!(!stored_position_applies(
            &stored(world(2), [1; 32]),
            here,
            [1; 32]
        ));
        assert!(!stored_position_applies(
            &stored(here, [9; 32]),
            here,
            [1; 32]
        ));
    }

    /// DEATH-RESTART-SAFETY-1 with §3.3 step 1: the in-session respawn upserts exactly the cell
    /// it places the actor at; any other cell refuses the placement.
    #[test]
    fn an_in_session_respawn_writes_only_the_cell_it_places() {
        let placed = FinalLastPosition {
            record: stored(world(1), [1; 32]).record,
            write_sequence: 1,
        };
        assert!(respawn_cell_recorded(
            MovementLocalPosition {
                x: 5,
                y: 6,
                floor: 7
            },
            &placed
        ));
        for other in [
            MovementLocalPosition {
                x: 4,
                y: 6,
                floor: 7,
            },
            MovementLocalPosition {
                x: 5,
                y: 5,
                floor: 7,
            },
            MovementLocalPosition {
                x: 5,
                y: 6,
                floor: 6,
            },
        ] {
            assert!(!respawn_cell_recorded(other, &placed), "{other:?}");
        }
    }

    /// Unchanged cells are not rewritten, and the interval gates the periodic write.
    #[test]
    fn tracks_are_due_after_the_interval_and_forgotten_on_retire() {
        let positions = LastPositions::default();
        let session = GameSessionId::decode(&{
            let mut bytes = [7; 16];
            bytes[6] = 0x70;
            bytes[8] = 0x80;
            bytes
        })
        .expect("uuid v7");
        let channel = crate::foundation::ChannelId::decode(session.as_bytes()).expect("uuid");
        let actor = ExactActorRef::transport_fixture(world(1), channel);
        positions.track(
            session,
            actor,
            None,
            CharacterRevision::new(1).expect("revision"),
        );
        let now = Instant::now();
        assert!(positions.due(now, false).is_empty());
        assert_eq!(positions.due(now, true).len(), 1);
        let deadline = positions.next_due().expect("tracked");
        assert!(deadline >= now + NEXT_DUE - Duration::from_secs(1));
        assert!(deadline <= Instant::now() + NEXT_DUE);
        assert_eq!(
            positions
                .due(now + NEXT_DUE + Duration::from_secs(1), false)
                .len(),
            1
        );
        let cell = LastPositionCell {
            x: 1,
            y: 2,
            floor: 7,
        };
        // The next deadline is measured from the capture, not from when its write settled.
        let captured = Instant::now();
        positions.settled(session, actor, Some(cell), captured);
        let track = positions.tracked(session).expect("tracked");
        assert_eq!(track.written, Some(cell));
        assert_eq!(track.due(), captured + NEXT_DUE);
        // An older capture never moves the deadline back.
        positions.settled(session, actor, None, now);
        assert_eq!(
            positions.tracked(session).map(|track| track.due()),
            Some(captured + NEXT_DUE)
        );
        // An in-session respawn's placement records its respawn cell.
        let respawn = LastPositionCell {
            x: 9,
            y: 9,
            floor: 7,
        };
        positions.respawned(session, respawn);
        assert_eq!(
            positions.tracked(session).and_then(|track| track.written),
            Some(respawn)
        );
        positions.forget(session);
        assert!(positions.tracked(session).is_none());
    }

    fn at(x: i32, y: i32) -> MovementLocalPosition {
        MovementLocalPosition { x, y, floor: 7 }
    }

    fn v7(seed: u8) -> [u8; 16] {
        let mut bytes = [seed; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes
    }

    /// The first-entry owner step's state: a runtime of `world(1)` whose pinned entry start is
    /// (3, 4, 7) and frame digest `[3; 32]`, with one committed player not yet positioned.
    fn admitted() -> (ChannelRuntimeV1, ExactActorRef, GameSessionId) {
        use crate::foundation::{ChannelContentPin, ChannelId, NodeId};
        let world_id = world(1);
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world_id,
            ChannelId::decode(&v7(0x61)).expect("channel"),
            NodeId::decode(&v7(0x62)).expect("node"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            ChannelContentPin::from_activation(
                world_id,
                1,
                [1; 32],
                [2; 32],
                [3; 32],
                [4; 32],
                (3, 4, 7),
            ),
        )
        .expect("runtime");
        let session = GameSessionId::decode(&v7(0x21)).expect("session");
        let reservation = runtime.reserve_fresh_session(session).expect("reserve");
        let player = runtime.commit_fresh_session(reservation).expect("commit");
        (runtime, player, session)
    }

    fn saved(world_id: WorldId, frame: [u8; 32], cell: MovementLocalPosition) -> AdmissionPosition {
        let mut row = stored(world_id, frame);
        row.record.cell = cell_of(cell);
        AdmissionPosition::Stored(row)
    }

    fn position(runtime: &ChannelRuntimeV1, actor: ExactActorRef) -> MovementLocalPosition {
        runtime
            .read_actor_position(actor)
            .expect("positioned")
            .position()
    }

    /// Codex 4249667405, §3.3 "it replaces `entry_start` as the input to
    /// `initialize_position`": a valid saved cell other than the entry start is the cell the
    /// first-entry owner step leaves the actor at, so the owners that run after that step (party
    /// presence, the familiar's login placement) read the saved cell, never `entry_start`.
    #[test]
    fn the_saved_cell_replaces_the_entry_start_in_the_first_entry_step() {
        let (mut runtime, player, session) = admitted();
        let decided = saved(world(1), [3; 32], at(20, 20));
        let input = first_entry_input(&decided, world(1), [3; 32], |_| false);
        assert_eq!(input, FirstEntryInput::Recorded(Some(at(20, 20))));
        let placed = initialize_first_entry_at(&mut runtime, player, session, input, |_| true)
            .expect("first entry");
        assert!(matches!(
            placed,
            FirstEntryPosition::Initialized(snapshot) if snapshot.position() == at(20, 20)
        ));
        assert_eq!(position(&runtime, player), at(20, 20));
        // An initialization that had already completed moves nothing.
        let again = initialize_first_entry_at(
            &mut runtime,
            player,
            session,
            FirstEntryInput::Recorded(Some(at(30, 30))),
            |_| true,
        )
        .expect("reconciled");
        assert!(matches!(again, FirstEntryPosition::Reconciled(_)));
        assert_eq!(position(&runtime, player), at(20, 20));
    }

    /// §3.3 "Occupied or blocked tile": the NPC-0 §6.1 fallback (north first), and with no
    /// admissible free cell within 3, step 4, the entry start; all within the owner step.
    #[test]
    fn a_blocked_saved_cell_uses_the_radius_fallback_then_the_entry_start() {
        let (mut runtime, player, session) = admitted();
        let input = FirstEntryInput::Recorded(Some(at(20, 20)));
        initialize_first_entry_at(&mut runtime, player, session, input, |cell| {
            cell != at(20, 20)
        })
        .expect("fallback");
        assert_eq!(position(&runtime, player), at(20, 19));
        let (mut runtime, player, session) = admitted();
        initialize_first_entry_at(&mut runtime, player, session, input, |_| false)
            .expect("entry start");
        assert_eq!(position(&runtime, player), at(3, 4));
    }

    /// §3.3 "valid": a saved house tile goes straight to step 4, as do another World or frame
    /// and a first login (no row); the entry start is then the step's own input.
    #[test]
    fn a_house_tile_another_frame_or_no_row_enters_at_the_entry_start() {
        let house = |cell: MovementLocalPosition| cell == at(20, 20);
        for decided in [
            saved(world(1), [3; 32], at(20, 20)),
            saved(world(2), [3; 32], at(21, 21)),
            saved(world(1), [9; 32], at(21, 21)),
            AdmissionPosition::FirstLogin,
        ] {
            assert_eq!(
                first_entry_input(&decided, world(1), [3; 32], house),
                FirstEntryInput::EntryStart,
                "{decided:?}"
            );
        }
        let (mut runtime, player, session) = admitted();
        initialize_first_entry_at(
            &mut runtime,
            player,
            session,
            FirstEntryInput::EntryStart,
            |_| true,
        )
        .expect("entry start");
        assert_eq!(position(&runtime, player), at(3, 4));
    }

    /// §3.3 step 1: a pending respawn's recorded cell is the first-entry input, even when it is
    /// a house tile; an undecodable record enters at step 4.
    #[test]
    fn a_pending_respawn_is_the_first_entry_input() {
        let occurrence =
            crate::durability::character_death::PlayerDeathOccurrence::from_bytes(v7(0x31))
                .expect("occurrence");
        let mut recorded = Vec::new();
        recorded.extend_from_slice(&5_i32.to_be_bytes());
        recorded.extend_from_slice(&6_i32.to_be_bytes());
        recorded.extend_from_slice(&7_i16.to_be_bytes());
        let pending = AdmissionPosition::Respawn(ConsumedRespawn {
            occurrence,
            respawn_position: recorded,
        });
        let input = first_entry_input(&pending, world(1), [3; 32], |_| true);
        assert_eq!(input, FirstEntryInput::Recorded(Some(at(5, 6))));
        let (mut runtime, player, session) = admitted();
        initialize_first_entry_at(&mut runtime, player, session, input, |_| true).expect("respawn");
        assert_eq!(position(&runtime, player), at(5, 6));
        let undecodable = AdmissionPosition::Respawn(ConsumedRespawn {
            occurrence,
            respawn_position: vec![1, 2, 3],
        });
        let input = first_entry_input(&undecodable, world(1), [3; 32], |_| false);
        assert_eq!(input, FirstEntryInput::Recorded(None));
        let (mut runtime, player, session) = admitted();
        initialize_first_entry_at(&mut runtime, player, session, input, |_| true).expect("step 4");
        assert_eq!(position(&runtime, player), at(3, 4));
    }
}
