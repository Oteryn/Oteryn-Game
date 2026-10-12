//! The Channel owner's side of the durable last position (CHAR-POSITION-1, decision
//! CHAR-POSITION-0, `reviews/OTERYN_GAME_CHAR_POSITION0_LOGOUT_POSITION_DECISION_2026-09-30.md`
//! §3.2 and §3.3).
//!
//! - **Admission (§3.3).** When a new runtime actor is admitted and no pending respawn is placed,
//!   the Character's last position is read under the session fence. A valid one (same World,
//!   same coordinate frame, a Walkable cell of the pinned movement cells that no closed door
//!   blocks, and not a house tile) places the actor there, or by the NPC-0 §6.1 fallback
//!   (nearest free admissible cell within 3, else `entry_start`). Anything else leaves the actor
//!   at `entry_start`. Resume reads nothing.
//! - **Tracking.** Only a session whose placement was decided from a successful read (or a
//!   consumed respawn) is tracked, so a failed read never lets the new session overwrite the
//!   stored position with `entry_start`.
//! - **Periodic write (§3.2, `CHARPOS0-RL-01`).** Every [`LAST_POSITION_WRITE_INTERVAL`], a
//!   tracked actor whose cell changed since its last write is written under the session fence.
//! - **Final write (§3.2 "At logout").** The terminal release captures the fenced actor's cell
//!   and the store upserts it inside the release transaction.
//! - **Shutdown.** Before readiness is withdrawn, every tracked actor is written once, whatever
//!   its interval, so a planned restart loses no movement.
//!
//! Each capture takes its `write_sequence` under the runtime lock from one process-wide
//! generator that is strictly increasing and never below the wall clock in microseconds, so a
//! write captured later always orders after one captured earlier in the same lease, including
//! the final write after an in-flight periodic one.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::{ComposedFreshAdmission, operator_event, respawn_cell_admissible};
use crate::durability::character_last_position::{
    FinalLastPosition, LastPositionCell, LastPositionRecord, LastPositionWrite, StoredLastPosition,
};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{ExactActorRef, GameSessionId, MovementLocalPosition, WorldId};

/// `CHARPOS0-RL-01`: the interval between periodic last-position writes of one live Character
/// while its position has changed. The registry bounds it to 60..=300 seconds.
pub(super) const LAST_POSITION_WRITE_INTERVAL: Duration = Duration::from_secs(300);

/// The delay before a tracked session is next due: the interval less one owner cadence, so a
/// changed position is written within the interval whichever cadence tick finds it due.
const NEXT_DUE: Duration = LAST_POSITION_WRITE_INTERVAL.saturating_sub(super::QUEST_RETRY_CADENCE);

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

    /// The seam's side: wait for the request, write every tracked session once, mark it done.
    pub(super) async fn serve(&self, authority: &ComposedFreshAdmission<'_, '_, '_>) {
        self.requested.cancelled().await;
        authority.write_due_last_positions(true).await;
        self.done.cancel();
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
    /// The cell last written or read for this Character; `None` when nothing is stored.
    written: Option<LastPositionCell>,
    due: Instant,
}

/// One captured position: its record and the sequence it was captured with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Capture {
    record: LastPositionRecord,
    write_sequence: u64,
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
    ) {
        if let Ok(mut tracks) = self.tracks.lock() {
            tracks.insert(
                session,
                Track {
                    actor,
                    written,
                    due: Instant::now() + NEXT_DUE,
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

    /// The sessions due at `now`, or every tracked session when `all`.
    fn due(&self, now: Instant, all: bool) -> Vec<(GameSessionId, Track)> {
        self.tracks
            .lock()
            .map(|tracks| {
                tracks
                    .iter()
                    .filter(|(_, track)| all || track.due <= now)
                    .map(|(session, track)| (*session, *track))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// A write of `cell` for `actor` is settled: the next one is due after the interval. A
    /// track replaced by another actor in the meantime is left alone.
    fn settled(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
        cell: Option<LastPositionCell>,
    ) {
        if let Ok(mut tracks) = self.tracks.lock()
            && let Some(track) = tracks.get_mut(&session)
            && track.actor == actor
        {
            if cell.is_some() {
                track.written = cell;
            }
            track.due = Instant::now() + NEXT_DUE;
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

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Capture `actor`'s current cell under the runtime lock, with its record and a fresh write
    /// sequence. `None` when the actor has no current position in the pinned context.
    async fn capture_last_position(&self, actor: ExactActorRef) -> Option<Capture> {
        let runtime = self.runtime.lock().await;
        let snapshot = runtime.read_actor_position(actor).ok()?;
        if snapshot.context() != runtime.pinned_movement_context() {
            return None;
        }
        let pin = runtime.content_pin();
        Some(Capture {
            record: LastPositionRecord {
                world_id: self.world_id,
                cell: cell_of(snapshot.position()),
                coordinate_frame_digest: pin.frame_binding_digest(),
                map_revision_digest: runtime.map_revision_digest(),
                server_artifact_digest: pin.server_artifact_digest(),
            },
            write_sequence: self.last_positions.next_sequence(),
        })
    }

    /// The record of a cell the admission placed, with a fresh write sequence.
    async fn placed_last_position(&self, cell: MovementLocalPosition) -> FinalLastPosition {
        let runtime = self.runtime.lock().await;
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
        self.capture_last_position(actor)
            .await
            .map(|capture| FinalLastPosition {
                record: capture.record,
                write_sequence: capture.write_sequence,
            })
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
        drop(runtime);
        Some(self.placed_last_position(snapshot.position()).await)
    }

    /// Track `session` after its respawn consumption wrote `placed`.
    pub(super) fn track_respawned(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
        placed: &FinalLastPosition,
    ) {
        self.last_positions
            .track(session, actor, Some(placed.record.cell));
    }

    /// §3.3 steps 3 and 4 for an admission with no pending respawn: read the last position
    /// under `fence` and place the actor by it. A failed read leaves the actor at
    /// `entry_start` and the session untracked, so it never overwrites the stored row.
    pub(super) async fn restore_last_position(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
        fence: CurrentCharacterGameplayFence,
    ) {
        let stored = match self
            .root
            .read_character_last_position(self.character, self.holder, fence)
            .await
        {
            Ok(stored) => stored,
            Err(error) => {
                operator_event(&format!("last_position_read_failed reason={error}"));
                return;
            }
        };
        let Some(stored) = stored else {
            self.last_positions.track(session, actor, None);
            return;
        };
        let mut runtime = self.runtime.lock().await;
        let pinned_digest = runtime.content_pin().server_artifact_digest();
        let frame_digest = runtime.content_pin().frame_binding_digest();
        if stored_position_applies(&stored, self.world_id, frame_digest) {
            let door = self.door.lock().await;
            let placed = runtime.place_admitted_respawn(
                actor,
                session,
                Some(position_of(stored.record.cell)),
                |cell| {
                    respawn_cell_admissible(
                        self.movement_cells.index(),
                        self.movement_cells.scope(),
                        self.world_id,
                        pinned_digest,
                        door.blocking_cells(),
                        cell,
                    ) && !house_cell(self.movement_cells, cell)
                },
            );
            if placed.is_err() {
                operator_event("last_position_placement_failed");
                return;
            }
        }
        drop(runtime);
        self.last_positions
            .track(session, actor, Some(stored.record.cell));
    }

    /// §3.2 periodic write (`CHARPOS0-RL-01`), run by the owner cadence; with `all`, the
    /// shutdown write of every tracked session. A cell unchanged since its last write writes
    /// nothing; a failed write is retried at the next cadence.
    pub(super) async fn write_due_last_positions(&self, all: bool) {
        for (session, track) in self.last_positions.due(Instant::now(), all) {
            let Some(capture) = self.capture_last_position(track.actor).await else {
                // The actor is gone: its session ended without a retire of this track.
                self.last_positions.forget(session);
                continue;
            };
            if track.written == Some(capture.record.cell) {
                self.last_positions.settled(session, track.actor, None);
                continue;
            }
            let fence = match self.current_quest_fence(session).await {
                Ok(Some(fence)) => fence,
                Ok(None) => {
                    self.last_positions.forget(session);
                    continue;
                }
                Err(()) => continue,
            };
            match self
                .root
                .write_character_last_position(
                    self.character,
                    self.holder,
                    fence,
                    capture.write_sequence,
                    capture.record,
                )
                .await
            {
                Ok(
                    LastPositionWrite::Written
                    | LastPositionWrite::Superseded
                    | LastPositionWrite::PendingObligation,
                ) => {
                    self.last_positions
                        .settled(session, track.actor, Some(capture.record.cell));
                }
                Err(error) => {
                    operator_event(&format!("last_position_write_failed reason={error}"));
                }
            }
        }
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
        // The cadence tick that finds a session due runs at most one cadence after it is due.
        assert!(NEXT_DUE + super::super::QUEST_RETRY_CADENCE <= LAST_POSITION_WRITE_INTERVAL);
        assert!(!NEXT_DUE.is_zero());
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
        positions.track(session, actor, None);
        let now = Instant::now();
        assert!(positions.due(now, false).is_empty());
        assert_eq!(positions.due(now, true).len(), 1);
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
        positions.settled(session, actor, Some(cell));
        assert_eq!(
            positions.tracked(session).and_then(|track| track.written),
            Some(cell)
        );
        positions.forget(session);
        assert!(positions.tracked(session).is_none());
    }
}
