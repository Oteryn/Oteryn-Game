//! TUTORIAL-1: the Channel owner's side of the Newhaven tutorial (CP D1426; owner Q3 and CP Q1 on
//! #2186). The rules are [`crate::domain::tutorial::apply`]; this module feeds it the authoritative
//! events of an admitted session, stores its progress and queues the notices the connection sends
//! under capability 31 `TUTORIAL_NOTICES_V1` (state domain 23).
//!
//! - **Admission.** After a playable fresh admission the stored progress is read under the
//!   session's gameplay fence. A Character with a row is on the tutorial track
//!   ([`TutorialGate::Newhaven`]). One without a row is when this is its first login and it
//!   entered on the Newhaven island ([`TutorialGate::for_start`] of the cell it was placed on),
//!   or when it was placed on the islet: only the World start and the tutorial respawn place a
//!   Character there, so a returning one missed the first write of its first login. A failed
//!   read is decided as a missing row and the session reads the row again before its next
//!   write; a returning Character beyond the islet whose read failed is not tracked in this
//!   session. Every other Character is not tracked and nothing here reads or writes for it. The
//!   tracked session then applies `Death` when the admission placed a pending respawn, the
//!   island reconciliation below, and `Login`.
//! - **Island reconciliation.** The map moves the Character through the forcefield before its
//!   transition is written, so a failed actor read or write leaves the islet stored for a
//!   Character on the island. A tracked Character beyond the islet whose stored step is the islet
//!   (or no row) therefore gets the forcefield transition written at its admission and at each
//!   step it ends beyond the islet, without the placement the map already made, until a write
//!   succeeds.
//! - **Steps.** A step onto the islet hole is refused before Movement runs and queues the
//!   reference text; the progress is unchanged and nothing is written. After a step that moved,
//!   the forcefield and the hunting tiles apply `StepIn` for the cell stepped onto. Other cells
//!   read nothing.
//! - **Placement.** A transition whose effect places the Character is committed only when the
//!   Character already is on that cell: the World start, the respawn and the forcefield's own
//!   destination place it, and this module never moves an actor. When the Character is elsewhere
//!   the transition is not written (fail closed). The one case the reducer would place on its
//!   own, a departed vocationless Character logging in again, therefore writes nothing.
//! - **Writes.** A write is the compare-and-set of `character_tutorial.rs` on the version read.
//!   A conflict takes the stored progress it returns and applies the event again, at most
//!   [`WRITE_ATTEMPTS`] times. A failed read or write changes nothing and queues nothing. A
//!   notice is queued only for a written transition (the hole text writes nothing).
//! - **Notices.** At most [`MAX_QUEUED_TUTORIAL_NOTICES`] per session; the connection drains them
//!   after each client step and sends them only to a session that selected capability 31. A
//!   resume or channel transfer sends the snapshot only: the queue is drained and dropped there,
//!   so a notice is never replayed.
//!
//! Not composed here: the chase step (CHASE-1) never enters the hole, forcefield or hunting
//! cells as a client would and applies no `StepIn`; `Depart` (the `a_seagull` and `anna` travel)
//! and `CorruptorLoot` have no runtime caller yet; an in-session respawn applies no `Death` (only
//! a respawn placed at admission does). No account in this World has gamemaster access, so every
//! actor reads `has_access: false`.

use std::collections::{HashMap, VecDeque};
use std::future::Future;

use oteryn_protocol_oteryn::tutorial_notices::TutorialNotice;

use super::{ComposedFreshAdmission, operator_event};
use crate::domain::tutorial::{
    HUNTING_TILES, ISLET_FORCEFIELD, ISLET_HOLE, ISLET_HOLE_MESSAGE, NEWHAVEN_ISLAND,
    NEWHAVEN_ISLET, TutorialActor, TutorialCell, TutorialEffect, TutorialEvent, TutorialGate,
    TutorialStep, TutorialVocation, apply,
};
use crate::durability::character_tutorial::{StoredTutorialProgress, TutorialProgressWrite};
use crate::foundation::{GameSessionId, MovementLocalPosition};

/// The most notices one session holds before its connection drains them; the oldest goes first.
pub(crate) const MAX_QUEUED_TUTORIAL_NOTICES: usize = 8;

/// The tries of one event: the first write and one re-apply after a conflict.
pub(crate) const WRITE_ATTEMPTS: usize = 2;

/// The durable reads and writes of the tutorial, under the session's current gameplay fence.
/// `Err` is a failed read or a refused fence: it proves nothing and changes nothing.
pub(crate) trait TutorialStore {
    fn read_progress(
        &self,
        session: GameSessionId,
    ) -> impl Future<Output = Result<Option<StoredTutorialProgress>, ()>>;

    fn write_progress(
        &self,
        session: GameSessionId,
        expected_version: u64,
        progress: crate::domain::tutorial::TutorialProgress,
    ) -> impl Future<Output = Result<TutorialProgressWrite, ()>>;

    /// What the reference scripts read of the Character: its current vocation.
    fn read_actor(&self, session: GameSessionId)
    -> impl Future<Output = Result<TutorialActor, ()>>;
}

/// How the admitted actor's first entry was decided (CHAR-POSITION-0 §3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TutorialAdmission {
    /// No last position: the Character's first login, placed at the World start.
    FirstLogin,
    /// A stored last position.
    Returning,
    /// A pending respawn: the Character died before this admission.
    Respawned,
}

/// The tutorial admission of a decided first-entry position. A placement not decided by this
/// admission (a reconciled actor) is a returning one.
pub(super) const fn admission_of(
    decided: Option<&super::last_position::AdmissionPosition>,
) -> TutorialAdmission {
    use super::last_position::AdmissionPosition;
    match decided {
        Some(AdmissionPosition::FirstLogin) => TutorialAdmission::FirstLogin,
        Some(AdmissionPosition::Respawn(_)) => TutorialAdmission::Respawned,
        Some(AdmissionPosition::Stored(_)) | None => TutorialAdmission::Returning,
    }
}

#[derive(Debug)]
struct TutorialSession {
    stored: Option<StoredTutorialProgress>,
    /// Whether `stored` was read. A session tracked after a failed admission read reads its row
    /// again before its next write.
    loaded: bool,
    notices: VecDeque<TutorialNotice>,
}

/// The track of a Character without a stored row, or whose row could not be read (see the
/// module doc).
const fn gate_without_row(admission: TutorialAdmission, at: TutorialCell) -> TutorialGate {
    match admission {
        TutorialAdmission::FirstLogin => TutorialGate::for_start(at),
        TutorialAdmission::Returning | TutorialAdmission::Respawned => {
            if NEWHAVEN_ISLET.contains(at) {
                TutorialGate::Newhaven
            } else {
                TutorialGate::Inert
            }
        }
    }
}

/// A cell of the island beyond the islet: only the forcefield moves a Character there from the
/// islet.
const fn beyond_the_islet(cell: TutorialCell) -> bool {
    NEWHAVEN_ISLAND.contains(cell) && !NEWHAVEN_ISLET.contains(cell)
}

/// Whether a stored progress still has to go through the forcefield.
fn before_the_forcefield(stored: Option<StoredTutorialProgress>) -> bool {
    matches!(
        stored.map(|stored| stored.progress.step),
        None | Some(TutorialStep::Islet)
    )
}

/// The tracked tutorial sessions of one Channel owner. A leaf lock, never held across an await.
#[derive(Debug, Default)]
pub(crate) struct TutorialSessions {
    sessions: std::sync::Mutex<HashMap<GameSessionId, TutorialSession>>,
}

/// A cell of the tutorial frame from a runtime position (legacy floor).
pub(crate) const fn tutorial_cell(position: MovementLocalPosition) -> TutorialCell {
    TutorialCell::new(position.x, position.y, position.floor)
}

/// The cell a step from `from` in `direction` goes onto (north is `y - 1`).
pub(crate) const fn stepped_onto(
    from: TutorialCell,
    direction: super::world_spatial::StepDirection,
) -> Option<TutorialCell> {
    use super::world_spatial::StepDirection;
    let (x, y) = match direction {
        StepDirection::North => (Some(from.x), from.y.checked_sub(1)),
        StepDirection::South => (Some(from.x), from.y.checked_add(1)),
        StepDirection::East => (from.x.checked_add(1), Some(from.y)),
        StepDirection::West => (from.x.checked_sub(1), Some(from.y)),
    };
    match (x, y) {
        (Some(x), Some(y)) => Some(TutorialCell::new(x, y, from.floor)),
        _ => None,
    }
}

/// Whether the effect leaves the Character where it already is: a placement holds only on its own
/// cell.
fn placement_holds(effect: Option<TutorialEffect>, at: TutorialCell) -> bool {
    match effect {
        Some(TutorialEffect::Place(cell) | TutorialEffect::PlaceWithHint { cell, .. }) => {
            cell == at
        }
        _ => true,
    }
}

/// The client notice of an effect.
fn notice_of(effect: TutorialEffect) -> Option<TutorialNotice> {
    match effect {
        TutorialEffect::Hint(hint) | TutorialEffect::PlaceWithHint { hint, .. } => {
            Some(TutorialNotice::Hint(hint))
        }
        TutorialEffect::PushBack { .. } => {
            Some(TutorialNotice::StatusText(ISLET_HOLE_MESSAGE.to_owned()))
        }
        TutorialEffect::Place(_) | TutorialEffect::CorruptorVocationLoot => None,
    }
}

impl TutorialSession {
    fn queue(&mut self, notice: TutorialNotice) {
        if self.notices.len() >= MAX_QUEUED_TUTORIAL_NOTICES {
            self.notices.pop_front();
        }
        self.notices.push_back(notice);
    }
}

impl TutorialSessions {
    fn with<T>(
        &self,
        session: GameSessionId,
        f: impl FnOnce(&mut TutorialSession) -> T,
    ) -> Option<T> {
        self.sessions.lock().ok()?.get_mut(&session).map(f)
    }

    /// Whether `session` is on the tutorial track.
    pub(crate) fn tracks(&self, session: GameSessionId) -> bool {
        self.with(session, |_| ()).is_some()
    }

    /// The stored progress of a tracked session.
    #[cfg(test)]
    pub(crate) fn stored(&self, session: GameSessionId) -> Option<StoredTutorialProgress> {
        self.with(session, |entry| entry.stored).flatten()
    }

    /// Admission: track `session` if it is on the tutorial track, then apply `Death` for a
    /// respawn, the island reconciliation and `Login`, at the cell `at` the actor was placed on.
    pub(crate) async fn admit<S: TutorialStore>(
        &self,
        store: &S,
        session: GameSessionId,
        admission: TutorialAdmission,
        at: TutorialCell,
    ) {
        let read = store.read_progress(session).await;
        let gate = match read {
            Ok(Some(_)) => TutorialGate::Newhaven,
            Ok(None) => gate_without_row(admission, at),
            Err(()) => {
                operator_event("tutorial_progress_read_failed");
                gate_without_row(admission, at)
            }
        };
        if gate == TutorialGate::Inert {
            return;
        }
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.insert(
                session,
                TutorialSession {
                    stored: read.ok().flatten(),
                    loaded: read.is_ok(),
                    notices: VecDeque::new(),
                },
            );
        }
        let Ok(actor) = store.read_actor(session).await else {
            operator_event("tutorial_actor_read_failed");
            return;
        };
        if admission == TutorialAdmission::Respawned {
            self.commit(store, session, TutorialEvent::Death(actor), at)
                .await;
        }
        if beyond_the_islet(at) {
            self.reconcile_island(store, session, actor).await;
        }
        self.commit(store, session, TutorialEvent::Login(actor), at)
            .await;
    }

    /// The islet hole, before Movement runs: `true` when the step of `session` from `from` onto
    /// `to` is refused. The refusal writes nothing and queues the reference text.
    pub(crate) fn refuses_step(
        &self,
        session: GameSessionId,
        from: TutorialCell,
        to: TutorialCell,
    ) -> bool {
        if to != ISLET_HOLE {
            return false;
        }
        self.with(session, |entry| {
            // The hole rule reads no actor fact.
            let actor = TutorialActor {
                vocation: TutorialVocation::Other,
                has_access: false,
            };
            let transition = apply(
                TutorialGate::Newhaven,
                entry.stored.map(|stored| stored.progress),
                TutorialEvent::StepIn { actor, from, to },
            );
            let Some(effect @ TutorialEffect::PushBack { .. }) = transition.effect else {
                return false;
            };
            if let Some(notice) = notice_of(effect) {
                entry.queue(notice);
            }
            true
        })
        .unwrap_or(false)
    }

    /// After a step of `session` that moved from `from` onto `to` and ended on `landed`: the
    /// forcefield and the hunting tiles apply `StepIn`, and a step that ended beyond the islet
    /// before the forcefield transition was written reconciles it. Every other step reads
    /// nothing.
    pub(crate) async fn after_step<S: TutorialStore>(
        &self,
        store: &S,
        session: GameSessionId,
        from: TutorialCell,
        to: TutorialCell,
        landed: TutorialCell,
    ) {
        let Some((stored, loaded)) = self.with(session, |entry| (entry.stored, entry.loaded))
        else {
            return;
        };
        let step_in = to == ISLET_FORCEFIELD
            || (HUNTING_TILES.contains(&to)
                && (!loaded || stored.is_some_and(|stored| stored.progress.hunting_hint_pending)));
        let reconcile = to != ISLET_FORCEFIELD
            && beyond_the_islet(landed)
            && (!loaded || before_the_forcefield(stored));
        if !step_in && !reconcile {
            return;
        }
        let Ok(actor) = store.read_actor(session).await else {
            operator_event("tutorial_actor_read_failed");
            return;
        };
        if reconcile {
            self.reconcile_island(store, session, actor).await;
        }
        if step_in {
            self.commit(
                store,
                session,
                TutorialEvent::StepIn { actor, from, to },
                landed,
            )
            .await;
        }
    }

    /// The island reconciliation (see the module doc): the forcefield transition of a Character
    /// the map already moved beyond the islet, written only while the stored step is before the
    /// forcefield.
    async fn reconcile_island<S: TutorialStore>(
        &self,
        store: &S,
        session: GameSessionId,
        actor: TutorialActor,
    ) {
        let crossing = TutorialEvent::StepIn {
            actor,
            from: ISLET_FORCEFIELD,
            to: ISLET_FORCEFIELD,
        };
        if self
            .commit_at(store, session, crossing, None)
            .await
            .is_some()
        {
            operator_event("tutorial_island_reconciled");
        }
    }

    /// The stored progress of a tracked session, read again when its admission read failed.
    async fn loaded<S: TutorialStore>(
        &self,
        store: &S,
        session: GameSessionId,
    ) -> Option<Option<StoredTutorialProgress>> {
        let (stored, loaded) = self.with(session, |entry| (entry.stored, entry.loaded))?;
        if loaded {
            return Some(stored);
        }
        let Ok(read) = store.read_progress(session).await else {
            operator_event("tutorial_progress_read_failed");
            return None;
        };
        self.with(session, |entry| {
            if !entry.loaded {
                entry.stored = read;
                entry.loaded = true;
            }
            entry.stored
        })
    }

    /// Apply `event` to the stored progress and write it; the actor is on `at`. Returns the
    /// effect of a written (or unchanged) transition, `None` when nothing was committed.
    pub(crate) async fn commit<S: TutorialStore>(
        &self,
        store: &S,
        session: GameSessionId,
        event: TutorialEvent,
        at: TutorialCell,
    ) -> Option<TutorialEffect> {
        self.commit_at(store, session, event, Some(at)).await
    }

    /// [`Self::commit`]; without `at` the event is the island reconciliation: it applies only
    /// before the forcefield and its placement is not checked (the map already made it).
    async fn commit_at<S: TutorialStore>(
        &self,
        store: &S,
        session: GameSessionId,
        event: TutorialEvent,
        at: Option<TutorialCell>,
    ) -> Option<TutorialEffect> {
        for _ in 0..WRITE_ATTEMPTS {
            let stored = self.loaded(store, session).await?;
            if at.is_none() && !before_the_forcefield(stored) {
                return None;
            }
            let before = stored.map(|stored| stored.progress);
            let transition = apply(TutorialGate::Newhaven, before, event);
            if let Some(at) = at
                && !placement_holds(transition.effect, at)
            {
                operator_event("tutorial_placement_not_held");
                return None;
            }
            if !transition.changes(before) {
                return transition.effect;
            }
            let progress = transition.progress?;
            let expected = stored.map_or(0, |stored| stored.version);
            match store.write_progress(session, expected, progress).await {
                Ok(TutorialProgressWrite::Written { version }) => {
                    self.with(session, |entry| {
                        entry.stored = Some(StoredTutorialProgress { progress, version });
                        if let Some(notice) = transition.effect.and_then(notice_of) {
                            entry.queue(notice);
                        }
                    })?;
                    return transition.effect;
                }
                Ok(TutorialProgressWrite::Conflict { stored }) => {
                    self.with(session, |entry| entry.stored = stored)?;
                }
                Err(()) => {
                    operator_event("tutorial_progress_write_failed");
                    return None;
                }
            }
        }
        operator_event("tutorial_progress_write_conflicted");
        None
    }

    /// The queued notices of `session`, oldest first, for its connection to send.
    pub(crate) fn take_notices(&self, session: GameSessionId) -> Vec<TutorialNotice> {
        self.with(session, |entry| entry.notices.drain(..).collect())
            .unwrap_or_default()
    }

    /// The session ended (its retire): it is no longer tracked.
    pub(crate) fn forget(&self, session: GameSessionId) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.remove(&session);
        }
    }
}

impl TutorialStore for ComposedFreshAdmission<'_, '_, '_> {
    async fn read_progress(
        &self,
        session: GameSessionId,
    ) -> Result<Option<StoredTutorialProgress>, ()> {
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return Err(());
        };
        self.root
            .read_character_tutorial_progress(self.character, self.holder, fence)
            .await
            .map_err(|error| {
                operator_event(&format!("tutorial_progress_read_failed reason={error}"));
            })
    }

    async fn write_progress(
        &self,
        session: GameSessionId,
        expected_version: u64,
        progress: crate::domain::tutorial::TutorialProgress,
    ) -> Result<TutorialProgressWrite, ()> {
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return Err(());
        };
        self.root
            .write_character_tutorial_progress(
                self.character,
                self.holder,
                fence,
                expected_version,
                progress,
            )
            .await
            .map_err(|error| {
                operator_event(&format!("tutorial_progress_write_failed reason={error}"));
            })
    }

    async fn read_actor(&self, session: GameSessionId) -> Result<TutorialActor, ()> {
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return Err(());
        };
        let build = self
            .root
            .read_character_build_state(self.character, fence.character_id)
            .await
            .map_err(|_| ())?;
        Ok(TutorialActor {
            vocation: TutorialVocation::from_key(build.vocation()),
            has_access: false,
        })
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// The current runtime cell of `actor`, in the pinned movement context.
    pub(super) fn tutorial_cell_of(
        runtime: &mut crate::foundation::ChannelRuntimeV1,
        actor: crate::foundation::ExactActorRef,
    ) -> Option<TutorialCell> {
        let snapshot = runtime.borrow_movement_position().read(actor).ok()?;
        if snapshot.context() != runtime.pinned_movement_context() {
            return None;
        }
        Some(tutorial_cell(snapshot.position()))
    }

    /// Admission hook of a playable fresh admission (see the module doc).
    pub(super) async fn admit_tutorial(
        &self,
        session: GameSessionId,
        actor: crate::foundation::ExactActorRef,
        admission: TutorialAdmission,
    ) {
        let at = Self::tutorial_cell_of(&mut *self.runtime.lock().await, actor);
        let Some(at) = at else {
            return;
        };
        self.tutorial.admit(self, session, admission, at).await;
    }
}

#[cfg(test)]
#[path = "tutorial_session_tests.rs"]
pub(crate) mod tests;
