use super::mailbox::MAX_PUSHED_MARKERS;
use super::*;
use crate::combat_input::{CombatCommand, CombatReport};
use oteryn_client_runtime::ClientRuntime;
use oteryn_session::{
    ActorPosition, AppliedDelta, AttackIntentDisposition, CommandStatus, LogoutResult,
    MAX_QUEUED_EVENTS, SessionEvent, SpellCastDisposition, StepDirection, StepDisposition,
    StepOutcome, WorldSpatialObservation,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

fn key(x: u32, y: u16, floor: i16, ordinal: u8) -> Vec<u8> {
    let floor_byte = u64::from(floor.unsigned_abs());
    let key = u64::from(x) << 32 | u64::from(y) << 16 | floor_byte << 8 | u64::from(ordinal);
    key.to_be_bytes().to_vec()
}

fn entry(placement: Vec<u8>) -> WorldObjectOverlayEntry {
    WorldObjectOverlayEntry {
        content_generation: [0; 32],
        placement,
        state: Vec::new(),
        revision: 1,
    }
}

fn moved_to(x: i32, y: i32, floor: i16) -> StepOutcome {
    StepOutcome {
        command_id: 1,
        status: CommandStatus::Accepted,
        disposition: StepDisposition::Moved,
        result_server_sequence: 1,
        world_spatial_delta: Some(AppliedDelta {
            server_sequence: 2,
            base_revision: 1,
            new_revision: 2,
            value: WorldSpatialObservation {
                content_generation: [0; 32],
                actor_position: ActorPosition { x, y, floor },
            },
        }),
        world_object_overlay_delta: None,
        gate_message_id: None,
    }
}

fn view() -> Result<PlayView, BatchError> {
    PlayView::new(
        TileCoord::new(100, 200),
        0,
        &[
            entry(key(102, 199, 0, 0)),
            entry(key(5, 5, 3, 0)),
            entry(b"x".to_vec()),
        ],
    )
}

#[test]
fn inputs_keep_their_order_and_a_repeated_cast_is_one() -> Result<(), BatchError> {
    let mut view = view()?;
    let one = NonZeroU32::MIN;
    view.arrow(StepDir::North);
    view.cast(one);
    view.cast(one);
    view.arrow(StepDir::East);
    assert_eq!(
        view.inputs.iter().copied().collect::<Vec<_>>(),
        vec![
            Input::Arrow(StepDir::North),
            Input::Cast(one),
            Input::Arrow(StepDir::East)
        ]
    );
    // A cast waiting at the front holds the step behind it.
    view.inputs.pop_front();
    assert_eq!(view.next_step(), None);
    Ok(())
}

#[test]
fn any_session_error_returns_to_login_with_its_public_class() {
    let error = SessionError::SessionUnusable;
    assert_eq!(public_class(&error), PublicClass::SessionUnavailable);
}

struct Fake {
    serves: Arc<AtomicUsize>,
    fail_step: bool,
    fail_serve_after: usize,
    queued: Vec<SessionEvent>,
    drained: Arc<AtomicUsize>,
    combats: Arc<Mutex<Vec<CombatCommand>>>,
    /// The answer to each logout in turn; empty answers `None`.
    logouts: Arc<Mutex<VecDeque<Option<LogoutResult>>>>,
}

fn lock_logouts(
    logouts: &Mutex<VecDeque<Option<LogoutResult>>>,
) -> MutexGuard<'_, VecDeque<Option<LogoutResult>>> {
    logouts.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Stepper for Fake {
    fn drain(&mut self) -> Vec<SessionEvent> {
        let events = std::mem::take(&mut self.queued);
        self.drained.fetch_add(events.len(), Ordering::SeqCst);
        events
    }

    async fn combat(&mut self, command: CombatCommand) -> Result<CombatReport, SessionError> {
        lock_combats(&self.combats).push(command);
        Ok(match command {
            CombatCommand::Cast(_) => CombatReport::Cast(SpellCastDisposition::NotEnoughMana),
            CombatCommand::Attack(_) => CombatReport::Attack(AttackIntentDisposition::Ok),
        })
    }

    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Ok(lock_logouts(&self.logouts).pop_front().flatten())
    }

    async fn step(&mut self, _direction: StepDirection) -> Result<StepOutcome, SessionError> {
        if self.fail_step {
            Err(SessionError::SessionUnusable)
        } else {
            Ok(moved_to(100, 199, 0))
        }
    }

    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        let count = self.serves.fetch_add(1, Ordering::SeqCst) + 1;
        if count > self.fail_serve_after {
            return Err(SessionError::SessionUnusable);
        }
        tokio::time::sleep(duration).await;
        Ok(())
    }
}

fn lock_combats(combats: &Mutex<Vec<CombatCommand>>) -> MutexGuard<'_, Vec<CombatCommand>> {
    combats.lock().unwrap_or_else(PoisonError::into_inner)
}

type Task = (PlayLink, Arc<AtomicUsize>, ClientRuntime);

fn task(fail_step: bool, fail_serve_after: usize) -> Result<Task, String> {
    let serves = Arc::new(AtomicUsize::new(0));
    let (link, commands, events) = play_channel();
    let runtime = ClientRuntime::new().map_err(|error| error.to_string())?;
    let fake = Fake {
        serves: Arc::clone(&serves),
        fail_step,
        fail_serve_after,
        queued: Vec::new(),
        drained: Arc::new(AtomicUsize::new(0)),
        combats: Arc::new(Mutex::new(Vec::new())),
        logouts: Arc::default(),
    };
    runtime
        .spawn(run_session(fake, commands, events))
        .map_err(|error| error.to_string())?;
    Ok((link, serves, runtime))
}

fn wait_for(link: &PlayLink) -> Option<PlayEvent> {
    for _ in 0..200 {
        if let Some(event) = link.poll() {
            return Some(event);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    None
}

#[test]
fn an_idle_session_keeps_servicing_liveness() -> Result<(), String> {
    let (link, serves, runtime) = task(false, usize::MAX)?;
    std::thread::sleep(Duration::from_millis(300));
    assert!(serves.load(Ordering::SeqCst) >= 5);
    drop(link);
    runtime.shutdown(Duration::from_millis(250));
    Ok(())
}

#[test]
fn a_hotkey_reaches_the_session_task_in_press_order_and_its_answer_reaches_the_view()
-> Result<(), String> {
    let serves = Arc::new(AtomicUsize::new(0));
    let combats = Arc::new(Mutex::new(Vec::new()));
    let (link, commands, events) = play_channel();
    let runtime = ClientRuntime::new().map_err(|error| error.to_string())?;
    let fake = Fake {
        serves,
        fail_step: false,
        fail_serve_after: usize::MAX,
        queued: Vec::new(),
        drained: Arc::new(AtomicUsize::new(0)),
        combats: Arc::clone(&combats),
        logouts: Arc::default(),
    };
    runtime
        .spawn(run_session(fake, commands, events))
        .map_err(|error| error.to_string())?;
    let mut view = view().map_err(|error| format!("{error:?}"))?;
    let spell = NonZeroU32::new(2).ok_or("spell")?;
    let first = NonZeroU32::new(1).ok_or("spell")?;
    view.cast(spell);
    view.cast(first);
    // Past the bar: ignored.
    view.cast(NonZeroU32::new(9).ok_or("spell")?);
    assert_eq!(view.tick(&link), Ok(()));
    for _ in 0..200 {
        assert_eq!(view.tick(&link), Ok(()));
        if lock_combats(&combats).len() == 2 && view.combat_status().ends_with("Not enough mana") {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        *lock_combats(&combats),
        [CombatCommand::Cast(spell), CombatCommand::Cast(first)]
    );
    assert_eq!(
        view.combat_status(),
        "spells 1-4 Spell cast".replace("Spell cast", "Not enough mana")
    );
    runtime.shutdown(Duration::from_millis(250));
    Ok(())
}

#[test]
fn a_tick_sends_a_step_and_applies_its_outcome_without_waiting() -> Result<(), String> {
    let (link, _serves, runtime) = task(false, usize::MAX)?;
    let mut view = view().map_err(|error| format!("{error:?}"))?;
    view.arrow(StepDir::North);
    assert_eq!(view.tick(&link), Ok(()));
    let mut moved = false;
    for _ in 0..200 {
        assert_eq!(view.tick(&link), Ok(()));
        if view.own() == TileCoord::new(100, 199) {
            moved = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(moved);
    runtime.shutdown(Duration::from_millis(250));
    Ok(())
}

#[test]
fn a_step_error_ends_the_task_with_its_public_class() -> Result<(), String> {
    let (link, _serves, runtime) = task(true, usize::MAX)?;
    link.request(StepDir::North);
    assert!(matches!(
        wait_for(&link),
        Some(PlayEvent::Ended(PublicClass::SessionUnavailable))
    ));
    runtime.shutdown(Duration::from_millis(250));
    Ok(())
}

#[test]
fn a_liveness_error_while_idle_returns_to_login() -> Result<(), String> {
    let (link, _serves, runtime) = task(false, 2)?;
    let mut view = view().map_err(|error| format!("{error:?}"))?;
    let mut ended = None;
    for _ in 0..200 {
        if let Err(end) = view.tick(&link) {
            ended = Some(end);
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(ended, Some(PlayEnd::Ended(PublicClass::SessionUnavailable)));
    runtime.shutdown(Duration::from_millis(250));
    Ok(())
}

#[test]
fn a_refused_logout_shows_its_line_and_an_accepted_one_ends_play() -> Result<(), String> {
    let logouts = Arc::new(Mutex::new(VecDeque::from([
        Some(LogoutResult::InFight {
            retry_after_ms: 2_001,
        }),
        None,
        Some(LogoutResult::Accepted),
    ])));
    let (link, commands, events) = play_channel();
    let runtime = ClientRuntime::new().map_err(|error| error.to_string())?;
    let fake = Fake {
        serves: Arc::new(AtomicUsize::new(0)),
        fail_step: false,
        fail_serve_after: usize::MAX,
        queued: Vec::new(),
        drained: Arc::new(AtomicUsize::new(0)),
        combats: Arc::new(Mutex::new(Vec::new())),
        logouts: Arc::clone(&logouts),
    };
    runtime
        .spawn(run_session(fake, commands, events))
        .map_err(|error| error.to_string())?;
    let mut view = view().map_err(|error| format!("{error:?}"))?;
    let tick_until = |view: &mut PlayView, done: &dyn Fn(&PlayView) -> bool| {
        for _ in 0..200 {
            let ticked = view.tick(&link);
            if ticked.is_err() || done(view) {
                return ticked;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    };
    view.logout();
    // A second press before the tick is the same logout.
    view.logout();
    assert_eq!(
        tick_until(&mut view, &|view| view.combat_status().contains("fight")),
        Ok(())
    );
    assert_eq!(
        view.combat_status(),
        "spells 1-4 You may not logout during a fight (3 s)"
    );
    assert_eq!(lock_logouts(&logouts).len(), 2);
    view.logout();
    assert_eq!(
        tick_until(&mut view, &|view| view.combat_status().contains("again")),
        Ok(())
    );
    view.logout();
    assert_eq!(tick_until(&mut view, &|_| false), Err(PlayEnd::LoggedOut));
    runtime.shutdown(Duration::from_millis(250));
    Ok(())
}

/// Queues pushed events like `Session` does: past the cap, the session fails closed.
struct Pushing {
    queue: Vec<SessionEvent>,
    sequence: u64,
}

impl Stepper for Pushing {
    async fn combat(&mut self, _command: CombatCommand) -> Result<CombatReport, SessionError> {
        Ok(CombatReport::Cast(SpellCastDisposition::NotEnoughMana))
    }

    async fn step(&mut self, _direction: StepDirection) -> Result<StepOutcome, SessionError> {
        Ok(moved_to(100, 199, 0))
    }

    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Ok(Some(LogoutResult::Busy))
    }

    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        for _ in 0..200 {
            self.sequence += 1;
            self.queue.push(SessionEvent::WorldSpatial(AppliedDelta {
                server_sequence: self.sequence,
                base_revision: self.sequence,
                new_revision: self.sequence + 1,
                value: WorldSpatialObservation {
                    content_generation: [0; 32],
                    actor_position: ActorPosition {
                        x: 100,
                        y: 200,
                        floor: 0,
                    },
                },
            }));
        }
        if self.queue.len() > MAX_QUEUED_EVENTS {
            return Err(SessionError::EventQueueOverflow {
                limit: MAX_QUEUED_EVENTS,
            });
        }
        tokio::time::sleep(duration).await;
        Ok(())
    }

    fn drain(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.queue)
    }
}

#[test]
fn pushed_events_are_drained_so_the_queue_never_overflows() -> Result<(), String> {
    let (link, commands, events) = play_channel();
    let runtime = ClientRuntime::new().map_err(|error| error.to_string())?;
    let pushing = Pushing {
        queue: Vec::new(),
        sequence: 0,
    };
    runtime
        .spawn(run_session(pushing, commands, events))
        .map_err(|error| error.to_string())?;
    let mut view = view().map_err(|error| format!("{error:?}"))?;
    let mut pushed = 0;
    for _ in 0..400 {
        while let Some(event) = link.poll() {
            match event {
                PlayEvent::Pushed(batch) => {
                    pushed += 1;
                    view.apply_pushed(&batch)
                        .map_err(|error| format!("{error:?}"))?;
                }
                PlayEvent::Ended(class) => return Err(format!("session ended: {class:?}")),
                PlayEvent::LoggedOut => return Err("logged out".to_owned()),
                PlayEvent::Stepped(_) | PlayEvent::Combat(_) | PlayEvent::LogoutRefused(_) => {}
            }
        }
        if pushed > 4 {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(pushed > 4);
    runtime.shutdown(Duration::from_millis(250));
    Ok(())
}

fn spatial_event(sequence: u64, x: i32) -> SessionEvent {
    SessionEvent::WorldSpatial(AppliedDelta {
        server_sequence: sequence,
        base_revision: sequence,
        new_revision: sequence + 1,
        value: WorldSpatialObservation {
            content_generation: [0; 32],
            actor_position: ActorPosition {
                x,
                y: 200,
                floor: 0,
            },
        },
    })
}

fn overlay_event(sequence: u64, placement: Vec<u8>) -> SessionEvent {
    SessionEvent::WorldObjectOverlay(AppliedDelta {
        server_sequence: sequence,
        base_revision: sequence,
        new_revision: sequence + 1,
        value: entry(placement),
    })
}

#[test]
fn an_undrained_mailbox_stays_bounded_and_keeps_the_latest_state() {
    let (link, _commands, outbox) = play_channel();
    for n in 0..5_000u64 {
        let events = vec![
            spatial_event(n, i32::try_from(n).unwrap_or(0)),
            overlay_event(n, n.to_be_bytes().to_vec()),
            overlay_event(n, b"same".to_vec()),
        ];
        outbox.push(events);
    }
    let event = link.poll();
    assert!(matches!(event, Some(PlayEvent::Pushed(_))));
    let Some(PlayEvent::Pushed(pushed)) = event else {
        return;
    };
    assert_eq!(pushed.own.map(|position| position.x), Some(4_999));
    assert!(pushed.markers.len() <= MAX_PUSHED_MARKERS);
    assert!(link.poll().is_none());
}

#[test]
fn pushes_older_than_a_step_outcome_are_delivered_before_it() {
    let (link, _commands, outbox) = play_channel();
    outbox.push(vec![spatial_event(1, 50)]);
    outbox.stepped(moved_to(100, 199, 0));
    outbox.push(vec![spatial_event(3, 60)]);
    assert!(matches!(
        link.poll(),
        Some(PlayEvent::Pushed(Pushed { own: Some(position), .. })) if position.x == 50
    ));
    assert!(matches!(link.poll(), Some(PlayEvent::Stepped(_))));
    assert!(matches!(
        link.poll(),
        Some(PlayEvent::Pushed(Pushed { own: Some(position), .. })) if position.x == 60
    ));
    assert!(link.poll().is_none());
}

// Feature test mounts follow the same directory discovery as production adapters.
include!(concat!(env!("OUT_DIR"), "/play_feature_tests.rs"));
