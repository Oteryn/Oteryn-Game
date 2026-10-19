use std::cell::{Cell, RefCell};

use super::*;
use crate::domain::tutorial::{
    HUNTING_HINT_DRUID, HUNTING_HINT_KNIGHT, HUNTING_HINT_MONK, HUNTING_HINT_PALADIN,
    HUNTING_HINT_SORCERER, ISLAND_ARRIVAL, ISLAND_HINT, ISLET_TEMPLE, TutorialProgress,
    TutorialStep,
};
use crate::gameplay_transport::world_spatial::StepDirection;

/// The in-memory compare-and-set store of the tutorial: the semantics of
/// `character_tutorial.rs` (version 0 is no row, a write bumps the version by one).
#[derive(Debug)]
pub(crate) struct MemoryTutorialStore {
    pub(crate) row: Cell<Option<StoredTutorialProgress>>,
    pub(crate) actor: Cell<Result<TutorialActor, ()>>,
    /// A concurrent writer stores this row just before the next write is compared.
    pub(crate) racing: Cell<Option<StoredTutorialProgress>>,
    pub(crate) fail_reads: Cell<bool>,
    pub(crate) fail_writes: Cell<bool>,
    pub(crate) writes: RefCell<Vec<TutorialProgress>>,
}

impl MemoryTutorialStore {
    pub(crate) fn new(vocation: TutorialVocation, row: Option<TutorialProgress>) -> Self {
        Self {
            row: Cell::new(row.map(|progress| StoredTutorialProgress {
                progress,
                version: 1,
            })),
            actor: Cell::new(Ok(TutorialActor {
                vocation,
                has_access: false,
            })),
            racing: Cell::new(None),
            fail_reads: Cell::new(false),
            fail_writes: Cell::new(false),
            writes: RefCell::new(Vec::new()),
        }
    }

    pub(crate) fn progress(&self) -> Option<TutorialProgress> {
        self.row.get().map(|stored| stored.progress)
    }
}

impl TutorialStore for MemoryTutorialStore {
    async fn read_progress(
        &self,
        _session: GameSessionId,
    ) -> Result<Option<StoredTutorialProgress>, ()> {
        if self.fail_reads.get() {
            return Err(());
        }
        Ok(self.row.get())
    }

    async fn write_progress(
        &self,
        _session: GameSessionId,
        expected_version: u64,
        progress: TutorialProgress,
    ) -> Result<TutorialProgressWrite, ()> {
        if self.fail_writes.get() {
            return Err(());
        }
        if let Some(racing) = self.racing.take() {
            self.row.set(Some(racing));
        }
        let current = self.row.get().map_or(0, |stored| stored.version);
        if current != expected_version {
            return Ok(TutorialProgressWrite::Conflict {
                stored: self.row.get(),
            });
        }
        let version = current + 1;
        self.row
            .set(Some(StoredTutorialProgress { progress, version }));
        self.writes.borrow_mut().push(progress);
        Ok(TutorialProgressWrite::Written { version })
    }

    async fn read_actor(&self, _session: GameSessionId) -> Result<TutorialActor, ()> {
        self.actor.get()
    }
}

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn run(test: impl Future<Output = TestResult>) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .build()?
        .block_on(test)
}

#[allow(clippy::expect_used)]
pub(crate) fn session() -> GameSessionId {
    let mut bytes = [0x31; 16];
    bytes[6] = 0x71;
    bytes[8] = 0xb1;
    GameSessionId::decode(&bytes).expect("session")
}

const START_THAIS: TutorialCell = TutorialCell::new(32369, 32238, 7);
/// The islet cell west of the forcefield.
const BEFORE_FORCEFIELD: TutorialCell = TutorialCell::new(32537, 32514, 7);

const fn islet() -> TutorialProgress {
    TutorialProgress::new(TutorialStep::Islet)
}

const fn island() -> TutorialProgress {
    TutorialProgress {
        step: TutorialStep::Island,
        hunting_hint_pending: true,
        corruptor_loot_pending: true,
    }
}

/// A vocationless first login on the islet temple, then the step through the forcefield.
async fn on_the_island(
    sessions: &TutorialSessions,
    store: &MemoryTutorialStore,
) -> Result<(), Box<dyn std::error::Error>> {
    sessions
        .admit(
            store,
            session(),
            TutorialAdmission::FirstLogin,
            ISLET_TEMPLE,
        )
        .await;
    sessions
        .after_step(
            store,
            session(),
            BEFORE_FORCEFIELD,
            ISLET_FORCEFIELD,
            ISLAND_ARRIVAL,
        )
        .await;
    if store.progress() != Some(island()) {
        return Err(format!("not on the island: {:?}", store.progress()).into());
    }
    Ok(())
}

#[test]
fn a_vocationless_first_login_on_the_islet_is_tracked_and_stores_the_citizen_track() -> TestResult {
    run(async {
        let (sessions, store) = (
            TutorialSessions::default(),
            MemoryTutorialStore::new(TutorialVocation::None, None),
        );
        sessions
            .admit(
                &store,
                session(),
                TutorialAdmission::FirstLogin,
                ISLET_TEMPLE,
            )
            .await;
        assert!(sessions.tracks(session()));
        assert_eq!(*store.writes.borrow(), [islet()]);
        assert_eq!(
            sessions.stored(session()),
            Some(StoredTutorialProgress {
                progress: islet(),
                version: 1
            })
        );
        // The temple placement shows nothing.
        assert!(sessions.take_notices(session()).is_empty());
        Ok(())
    })
}

#[test]
fn other_starts_returning_characters_without_a_row_and_failed_reads_are_not_tracked() -> TestResult
{
    run(async {
        for (admission, at) in [
            (TutorialAdmission::FirstLogin, START_THAIS),
            (TutorialAdmission::Returning, ISLET_TEMPLE),
            (TutorialAdmission::Respawned, ISLET_TEMPLE),
        ] {
            let (sessions, store) = (
                TutorialSessions::default(),
                MemoryTutorialStore::new(TutorialVocation::None, None),
            );
            sessions.admit(&store, session(), admission, at).await;
            assert!(!sessions.tracks(session()), "{admission:?}");
            assert!(store.writes.borrow().is_empty(), "{admission:?}");
            // An untracked session refuses no step and reads nothing after one.
            assert!(!sessions.refuses_step(session(), ISLET_TEMPLE, ISLET_HOLE));
            store.fail_reads.set(true);
            sessions
                .after_step(
                    &store,
                    session(),
                    BEFORE_FORCEFIELD,
                    ISLET_FORCEFIELD,
                    ISLAND_ARRIVAL,
                )
                .await;
            assert!(store.writes.borrow().is_empty());
            assert!(sessions.take_notices(session()).is_empty());
        }
        let (sessions, store) = (
            TutorialSessions::default(),
            MemoryTutorialStore::new(TutorialVocation::None, Some(island())),
        );
        store.fail_reads.set(true);
        sessions
            .admit(
                &store,
                session(),
                TutorialAdmission::Returning,
                ISLET_TEMPLE,
            )
            .await;
        assert!(!sessions.tracks(session()));
        Ok(())
    })
}

#[test]
fn a_returning_citizen_resumes_and_a_respawn_goes_back_to_the_islet() -> TestResult {
    run(async {
        let (sessions, store) = (
            TutorialSessions::default(),
            MemoryTutorialStore::new(TutorialVocation::None, Some(island())),
        );
        sessions
            .admit(
                &store,
                session(),
                TutorialAdmission::Returning,
                ISLAND_ARRIVAL,
            )
            .await;
        assert!(sessions.tracks(session()));
        assert!(
            store.writes.borrow().is_empty(),
            "a citizen login writes nothing"
        );
        sessions.forget(session());
        sessions
            .admit(
                &store,
                session(),
                TutorialAdmission::Respawned,
                ISLET_TEMPLE,
            )
            .await;
        assert_eq!(
            store.progress(),
            Some(TutorialProgress {
                step: TutorialStep::Islet,
                ..island()
            })
        );
        Ok(())
    })
}

#[test]
fn a_placement_the_actor_is_not_on_is_not_written() -> TestResult {
    run(async {
        // A vocationless departed Character logging in again would be placed on the temple; it
        // stands elsewhere, so nothing is written.
        let departed = TutorialProgress::new(TutorialStep::Departed);
        let (sessions, store) = (
            TutorialSessions::default(),
            MemoryTutorialStore::new(TutorialVocation::None, Some(departed)),
        );
        sessions
            .admit(&store, session(), TutorialAdmission::Returning, START_THAIS)
            .await;
        assert!(sessions.tracks(session()));
        assert_eq!(store.progress(), Some(departed));
        assert!(store.writes.borrow().is_empty());
        // The forcefield step that did not land on the island arrival writes nothing either.
        let (sessions, store) = (
            TutorialSessions::default(),
            MemoryTutorialStore::new(TutorialVocation::None, None),
        );
        sessions
            .admit(
                &store,
                session(),
                TutorialAdmission::FirstLogin,
                ISLET_TEMPLE,
            )
            .await;
        sessions
            .after_step(
                &store,
                session(),
                BEFORE_FORCEFIELD,
                ISLET_FORCEFIELD,
                ISLET_FORCEFIELD,
            )
            .await;
        assert_eq!(store.progress(), Some(islet()));
        assert!(sessions.take_notices(session()).is_empty());
        Ok(())
    })
}

#[test]
fn the_islet_hole_is_refused_with_its_text_and_writes_nothing() -> TestResult {
    run(async {
        let (sessions, store) = (
            TutorialSessions::default(),
            MemoryTutorialStore::new(TutorialVocation::None, None),
        );
        sessions
            .admit(
                &store,
                session(),
                TutorialAdmission::FirstLogin,
                ISLET_TEMPLE,
            )
            .await;
        assert!(sessions.refuses_step(session(), ISLET_TEMPLE, ISLET_HOLE));
        assert!(!sessions.refuses_step(session(), ISLET_TEMPLE, BEFORE_FORCEFIELD));
        assert_eq!(
            sessions.take_notices(session()),
            [TutorialNotice::StatusText(ISLET_HOLE_MESSAGE.to_owned())]
        );
        assert!(sessions.take_notices(session()).is_empty(), "drained");
        assert_eq!(*store.writes.borrow(), [islet()]);
        Ok(())
    })
}

#[test]
fn the_forcefield_stores_the_island_and_shows_the_island_hint_only_when_vocationless() -> TestResult
{
    run(async {
        let (sessions, store) = (
            TutorialSessions::default(),
            MemoryTutorialStore::new(TutorialVocation::None, None),
        );
        on_the_island(&sessions, &store).await?;
        assert_eq!(
            sessions.take_notices(session()),
            [TutorialNotice::Hint(ISLAND_HINT)]
        );
        let (sessions, store) = (
            TutorialSessions::default(),
            MemoryTutorialStore::new(TutorialVocation::None, None),
        );
        sessions
            .admit(
                &store,
                session(),
                TutorialAdmission::FirstLogin,
                ISLET_TEMPLE,
            )
            .await;
        store.actor.set(Ok(TutorialActor {
            vocation: TutorialVocation::Knight,
            has_access: false,
        }));
        sessions
            .after_step(
                &store,
                session(),
                BEFORE_FORCEFIELD,
                ISLET_FORCEFIELD,
                ISLAND_ARRIVAL,
            )
            .await;
        assert_eq!(store.progress(), Some(island()));
        assert!(sessions.take_notices(session()).is_empty());
        Ok(())
    })
}

#[test]
fn each_base_vocation_gets_its_hunting_hint_once_and_a_vocationless_one_stays_armed() -> TestResult
{
    run(async {
        for (vocation, hint) in [
            (TutorialVocation::Knight, Some(HUNTING_HINT_KNIGHT)),
            (TutorialVocation::Paladin, Some(HUNTING_HINT_PALADIN)),
            (TutorialVocation::Sorcerer, Some(HUNTING_HINT_SORCERER)),
            (TutorialVocation::Druid, Some(HUNTING_HINT_DRUID)),
            (TutorialVocation::Monk, Some(HUNTING_HINT_MONK)),
            (TutorialVocation::None, None),
            (TutorialVocation::Other, None),
        ] {
            let (sessions, store) = (
                TutorialSessions::default(),
                MemoryTutorialStore::new(TutorialVocation::None, None),
            );
            on_the_island(&sessions, &store).await?;
            drop(sessions.take_notices(session()));
            store.actor.set(Ok(TutorialActor {
                vocation,
                has_access: false,
            }));
            let tile = HUNTING_TILES[0];
            let before = TutorialCell::new(tile.x, tile.y + 1, tile.floor);
            for _ in 0..2 {
                sessions
                    .after_step(&store, session(), before, tile, tile)
                    .await;
            }
            assert_eq!(
                sessions.take_notices(session()),
                hint.map(TutorialNotice::Hint)
                    .into_iter()
                    .collect::<Vec<_>>(),
                "{vocation:?}"
            );
            assert_eq!(
                store.progress().map(|stored| stored.hunting_hint_pending),
                Some(hint.is_none()),
                "{vocation:?}"
            );
        }
        Ok(())
    })
}

#[test]
fn a_conflict_reapplies_on_the_stored_row_and_a_failed_write_changes_nothing() -> TestResult {
    run(async {
        let (sessions, store) = (
            TutorialSessions::default(),
            MemoryTutorialStore::new(TutorialVocation::None, None),
        );
        sessions
            .admit(
                &store,
                session(),
                TutorialAdmission::FirstLogin,
                ISLET_TEMPLE,
            )
            .await;
        // Another writer stored the islet at version 4: the forcefield is applied again to it.
        store.racing.set(Some(StoredTutorialProgress {
            progress: islet(),
            version: 4,
        }));
        sessions
            .after_step(
                &store,
                session(),
                BEFORE_FORCEFIELD,
                ISLET_FORCEFIELD,
                ISLAND_ARRIVAL,
            )
            .await;
        assert_eq!(
            sessions.stored(session()),
            Some(StoredTutorialProgress {
                progress: island(),
                version: 5
            })
        );
        assert_eq!(
            sessions.take_notices(session()),
            [TutorialNotice::Hint(ISLAND_HINT)]
        );
        // A concurrent writer that already stored the island leaves nothing to write: no notice.
        let (sessions, store) = (
            TutorialSessions::default(),
            MemoryTutorialStore::new(TutorialVocation::None, None),
        );
        sessions
            .admit(
                &store,
                session(),
                TutorialAdmission::FirstLogin,
                ISLET_TEMPLE,
            )
            .await;
        store.racing.set(Some(StoredTutorialProgress {
            progress: island(),
            version: 7,
        }));
        sessions
            .after_step(
                &store,
                session(),
                BEFORE_FORCEFIELD,
                ISLET_FORCEFIELD,
                ISLAND_ARRIVAL,
            )
            .await;
        assert_eq!(sessions.stored(session()).map(|s| s.version), Some(7));
        assert!(sessions.take_notices(session()).is_empty());
        // A failed write keeps the stored progress and queues nothing.
        let (sessions, store) = (
            TutorialSessions::default(),
            MemoryTutorialStore::new(TutorialVocation::None, None),
        );
        sessions
            .admit(
                &store,
                session(),
                TutorialAdmission::FirstLogin,
                ISLET_TEMPLE,
            )
            .await;
        store.fail_writes.set(true);
        sessions
            .after_step(
                &store,
                session(),
                BEFORE_FORCEFIELD,
                ISLET_FORCEFIELD,
                ISLAND_ARRIVAL,
            )
            .await;
        assert_eq!(
            sessions.stored(session()).map(|s| s.progress),
            Some(islet())
        );
        assert!(sessions.take_notices(session()).is_empty());
        Ok(())
    })
}

#[test]
fn the_notice_queue_is_bounded_and_forget_ends_the_tracking() -> TestResult {
    run(async {
        let (sessions, store) = (
            TutorialSessions::default(),
            MemoryTutorialStore::new(TutorialVocation::None, None),
        );
        sessions
            .admit(
                &store,
                session(),
                TutorialAdmission::FirstLogin,
                ISLET_TEMPLE,
            )
            .await;
        for _ in 0..MAX_QUEUED_TUTORIAL_NOTICES + 3 {
            assert!(sessions.refuses_step(session(), ISLET_TEMPLE, ISLET_HOLE));
        }
        assert_eq!(
            sessions.take_notices(session()).len(),
            MAX_QUEUED_TUTORIAL_NOTICES
        );
        assert!(sessions.refuses_step(session(), ISLET_TEMPLE, ISLET_HOLE));
        sessions.forget(session());
        assert!(!sessions.tracks(session()));
        assert!(sessions.take_notices(session()).is_empty());
        assert!(!sessions.refuses_step(session(), ISLET_TEMPLE, ISLET_HOLE));
        Ok(())
    })
}

#[test]
fn a_step_goes_onto_the_neighbouring_cell_and_the_admission_follows_the_decision() {
    let from = ISLET_TEMPLE;
    for (direction, to) in [
        (StepDirection::North, TutorialCell::new(32534, 32512, 7)),
        (StepDirection::South, TutorialCell::new(32534, 32514, 7)),
        (StepDirection::East, TutorialCell::new(32535, 32513, 7)),
        (StepDirection::West, ISLET_HOLE),
    ] {
        assert_eq!(stepped_onto(from, direction), Some(to), "{direction:?}");
    }
    assert_eq!(
        stepped_onto(TutorialCell::new(i32::MAX, 0, 7), StepDirection::East),
        None
    );
    assert_eq!(
        admission_of(Some(
            &super::super::last_position::AdmissionPosition::FirstLogin
        )),
        TutorialAdmission::FirstLogin
    );
    assert_eq!(admission_of(None), TutorialAdmission::Returning);
}
