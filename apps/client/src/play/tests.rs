use super::mailbox::MAX_PUSHED_MARKERS;
use super::*;
use crate::combat_input::{CombatCommand, CombatReport, PlayCommand};
use oteryn_client_runtime::ClientRuntime;
use oteryn_platform_client::native_login::PublicClass;
use oteryn_session::{
    ActorPosition, AppliedDelta, AttackIntentDisposition, CommandStatus, EntityRef, LogoutResult,
    MAX_QUEUED_EVENTS, SessionError, SessionEvent, SpellCastDisposition, StepDirection,
    StepDisposition, StepOutcome, WorldSpatialObservation,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

#[path = "../features/0233-touch-controls/tests/play.rs"]
mod touch_controls;

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

pub(super) fn view() -> Result<PlayView, BatchError> {
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
            CombatCommand::Cast(_)
            | CombatCommand::CastAt { .. }
            | CombatCommand::CastCursor { .. }
            | CombatCommand::CastFacing { .. } => {
                CombatReport::Cast(SpellCastDisposition::NotEnoughMana)
            }
            CombatCommand::Stop => CombatReport::Stop(AttackIntentDisposition::Ok),
            CombatCommand::Modes(_) => CombatReport::Modes(AttackIntentDisposition::Ok),
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

// Feature test mounts follow the same directory discovery as production adapters.
include!(concat!(env!("OUT_DIR"), "/play_feature_tests.rs"));

#[path = "tests_additional.rs"]
mod additional;

#[test]
fn owner_drop_stops_session_with_auxiliary_sender_alive() -> Result<(), String> {
    on_session_test_thread(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| error.to_string())?;
        runtime.block_on(async {
            let (link, commands, outbox) = play_channel();
            let auxiliary = link.commands.clone();
            let fake = Fake {
                serves: Arc::default(),
                fail_step: false,
                fail_serve_after: usize::MAX,
                queued: Vec::new(),
                drained: Arc::default(),
                combats: Arc::default(),
                logouts: Arc::default(),
            };
            drop(link);
            let completed = tokio::time::timeout(
                Duration::from_millis(100),
                run_session(fake, commands, outbox),
            )
            .await;
            assert!(
                completed.is_ok(),
                "owner disposal must stop the session even with an Arena sender alive"
            );
            drop(auxiliary);
            Ok::<(), String>(())
        })
    })
}

struct HandoffPeer {
    modal: bool,
    initial_modal: bool,
    feature_calls: Arc<AtomicUsize>,
    serve_calls: Arc<AtomicUsize>,
}
impl Stepper for HandoffPeer {
    fn feature_selected(&self, capability: u32) -> bool {
        capability == if self.modal { 450 } else { 463 }
    }
    async fn feature(
        &mut self,
        _command: &oteryn_client_runtime::raw_feature::FeatureCommand,
    ) -> Result<oteryn_session::GatedCommandResult, SessionError> {
        if std::mem::take(&mut self.initial_modal) {
            let reply = oteryn_client_runtime::server_modal::Reply::Dialog(
                oteryn_client_runtime::server_modal::Dialog {
                    id: 73,
                    title: "Source activity".into(),
                    message: "Choose".into(),
                    choices: Vec::new(),
                    buttons: vec![(4, "Accept".into())],
                    cancel_button: None,
                    image: None,
                },
            );
            return Ok(oteryn_session::GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: CommandStatus::Accepted,
                payload: reply.encode().map_err(|_| SessionError::SessionUnusable)?,
            });
        }
        if self.feature_calls.fetch_add(1, Ordering::SeqCst) == 0 {
            Err(SessionError::ActivityScopeChanged)
        } else {
            Err(SessionError::SessionUnusable)
        }
    }
    async fn serve(&mut self, _duration: Duration) -> Result<(), SessionError> {
        self.serve_calls.fetch_add(1, Ordering::SeqCst);
        Err(SessionError::SessionUnusable)
    }
    async fn step(&mut self, _direction: StepDirection) -> Result<StepOutcome, SessionError> {
        Ok(moved_to(100, 199, 0))
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        Vec::new()
    }
    async fn combat(&mut self, _command: CombatCommand) -> Result<CombatReport, SessionError> {
        Ok(CombatReport::Stop(AttackIntentDisposition::Ok))
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Ok(None)
    }
}

#[test]
fn modal_and_support_handoffs_recover_before_servicing_destination() -> Result<(), String> {
    on_session_test_thread(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| error.to_string())?;
        runtime.block_on(async {
            for modal in [true, false] {
                let (link, commands, outbox) = play_channel();
                link.use_shared_frontend();
                let feature_calls = Arc::default();
                let serve_calls = Arc::default();
                let peer = HandoffPeer {
                    modal,
                    initial_modal: false,
                    feature_calls: Arc::clone(&feature_calls),
                    serve_calls: Arc::clone(&serve_calls),
                };
                if !modal {
                    assert!(
                        link.commands
                            .send(PlayCommand::SupportDelivery(
                                oteryn_client_runtime::raw_feature::FeatureCommand {
                                    capability: 463,
                                    kind: 1,
                                    payload: Vec::new(),
                                },
                                std::path::PathBuf::from("unused-handoff-delivery"),
                            ))
                            .is_ok()
                    );
                }
                outbox.push(vec![SessionEvent::ServerMessage(
                    oteryn_session::AppliedDelta {
                        server_sequence: 1000,
                        base_revision: 0,
                        new_revision: 1,
                        value: oteryn_session::ServerMessage::StatusText("source namespace".into()),
                    },
                )]);
                run_session(peer, commands, outbox).await;
                {
                    let mailbox = super::mailbox::lock(&link.mailbox);
                    assert!(mailbox.pushed.server_messages.iter().next().is_none());
                    assert!(
                        mailbox.pushed.scope_changed,
                        "all recovered handoffs restart the message sequence namespace"
                    );
                }

                if modal {
                    assert_eq!(
                        feature_calls.load(Ordering::SeqCst),
                        2,
                        "handoff must release the pending modal query"
                    );
                } else {
                    assert_eq!(
                        serve_calls.load(Ordering::SeqCst),
                        1,
                        "support handoff must keep play alive"
                    );
                }
            }
            Ok::<(), String>(())
        })
    })
}

#[test]
fn modal_answer_handoff_drops_source_answer_and_queries_destination() -> Result<(), String> {
    on_session_test_thread(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| error.to_string())?;
        runtime.block_on(async {
            let (link, commands, outbox) = play_channel();
            link.use_shared_frontend();
            let feature_calls = Arc::default();
            let mut peer = HandoffPeer {
                modal: true,
                initial_modal: true,
                feature_calls: Arc::clone(&feature_calls),
                serve_calls: Arc::default(),
            };
            let mut driver =
                super::feature_0450_server_modal::protocol::Driver::new(true).with_outbox(&outbox);
            driver
                .poll(&mut peer)
                .await
                .map_err(|error| error.to_string())?;
            let panel = link.server_modal_panel().ok_or("source modal")?;
            assert!(link.server_modal_submit(
                panel.revision,
                oteryn_client_runtime::server_modal::Answer {
                    dialog_id: 73,
                    button_id: 4,
                    choice_id: None,
                },
                Default::default()
            ));
            super::session_loop::run_session_with_modal(peer, commands, outbox, driver).await;
            assert_eq!(feature_calls.load(Ordering::SeqCst), 2);
            assert!(link.server_modal_panel().is_none());
            Ok::<(), String>(())
        })
    })
}

fn on_session_test_thread(test: fn() -> Result<(), String>) -> Result<(), String> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(test)
        .map_err(|error| error.to_string())?
        .join()
        .map_err(|_| "session regression thread failed".to_owned())?
}
