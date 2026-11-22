use super::*;
use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{Stepper, play_channel};
use oteryn_client_runtime::world_interaction::{
    Action, Command as WorldCommand, Selection, Target,
};
use oteryn_session::world_map::{MapViewHeader, WorldMapViewSnapshot};
use oteryn_session::{
    AppliedDelta, EntityDetail, EntityKind, EntityRef, SessionError, SpellTargetPosition,
    StepDirection, StepOutcome, WorldSpatialObservation,
};
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex, PoisonError};

fn at(x: i32, y: i32, floor: i16) -> ActorPosition {
    ActorPosition { x, y, floor }
}

#[test]
fn a_raw_frame_round_trips_wire_native_wire_and_the_native_frame_is_identity() {
    let wire = at(100, 200, 7);
    let native = WireFrame::Raw.to_native(wire);
    assert_eq!(native, at(100, 200, -7));
    assert_eq!(WireFrame::Raw.to_wire(native), wire);
    assert_eq!(WireFrame::Raw.floor_to_native(7), -7);
    let source = at(100, 200, -8);
    assert_eq!(WireFrame::Native.to_native(source), source);
    assert_eq!(WireFrame::Native.to_wire(source), source);
    assert_eq!(WireFrame::default(), WireFrame::Native);
}

#[test]
fn the_frame_is_derived_from_the_wire_position_and_the_map_origin_only() {
    let origin = at(10, 20, -7);
    // Equal floors prove Native, opposite floors prove Raw, whatever the previous frame.
    assert_eq!(
        WireFrame::derive(WireFrame::Raw, at(10, 20, -7), origin),
        WireFrame::Native
    );
    assert_eq!(
        WireFrame::derive(WireFrame::Native, at(10, 20, 7), origin),
        WireFrame::Raw
    );
    // Floor 0 is ambiguous: the previous frame stays.
    for previous in [WireFrame::Native, WireFrame::Raw] {
        assert_eq!(
            WireFrame::derive(previous, at(10, 20, 0), at(10, 20, 0)),
            previous
        );
        // A step in flight (another tile) or an unrelated floor decides nothing.
        assert_eq!(WireFrame::derive(previous, at(11, 20, 7), origin), previous);
        assert_eq!(WireFrame::derive(previous, at(10, 20, 3), origin), previous);
    }
}

fn map_snapshot(origin: ActorPosition) -> SessionEvent {
    SessionEvent::WorldMapSnapshot(AppliedDelta {
        server_sequence: 1,
        base_revision: 1,
        new_revision: 2,
        value: WorldMapViewSnapshot {
            header: MapViewHeader {
                content_generation: [0; 32],
                bundle_digest: [0; 32],
                reset_epoch: 1,
                origin,
                first_visible_floor: 0,
            },
            tiles: Vec::new(),
        },
    })
}

fn spatial(position: ActorPosition) -> SessionEvent {
    SessionEvent::WorldSpatial(AppliedDelta {
        server_sequence: 2,
        base_revision: 1,
        new_revision: 2,
        value: WorldSpatialObservation {
            content_generation: [0; 32],
            actor_position: position,
        },
    })
}

fn entity(position: ActorPosition) -> oteryn_session::WorldSpatialEntity {
    oteryn_session::WorldSpatialEntity {
        motion: None,
        display_name: None,
        kind: EntityKind::Creature,
        entity: EntityRef {
            identity: [9; 16],
            generation: 1,
        },
        position,
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    }
}

fn pushed_own(outbox: &crate::play::Outbox) -> Option<ActorPosition> {
    crate::play::mailbox::lock(&outbox.0).pushed.own
}

#[test]
fn an_ordinary_bundle_reaches_the_view_in_the_native_frame_and_leaves_in_the_wire_frame() {
    let (_link, _commands, outbox) = play_channel();
    // The wire is +7 on an ordinary bundle while the map origin is -7; the map may even come
    // after the position within one batch.
    outbox.push(vec![
        spatial(at(100, 200, 7)),
        map_snapshot(at(100, 200, -7)),
    ]);
    assert_eq!(
        crate::play::mailbox::lock(&outbox.0).wire_frame(),
        WireFrame::Raw
    );
    // A later step moves off the origin tile; the frame is kept and the position converted.
    outbox.push(vec![spatial(at(101, 200, 7))]);
    assert_eq!(pushed_own(&outbox), Some(at(101, 200, -7)));
    // Entity lists and appearances from the session are converted exactly once.
    outbox.entities(vec![entity(at(102, 200, 7))]);
    let mailbox = crate::play::mailbox::lock(&outbox.0);
    assert_eq!(
        mailbox
            .pushed
            .entities
            .as_ref()
            .map(|list| list[0].position),
        Some(at(102, 200, -7))
    );
    assert_eq!(mailbox.wire_position(at(101, 200, -7)), at(101, 200, 7));
}

#[test]
fn a_native_bundle_and_a_fixture_world_stay_in_the_wire_frame() {
    let (_link, _commands, outbox) = play_channel();
    // No map: a fixture world keeps the wire frame untouched.
    outbox.push(vec![spatial(at(100, 200, 7))]);
    assert_eq!(pushed_own(&outbox), Some(at(100, 200, 7)));
    // A pinned activity bundle: the wire is already native.
    let (_link, _commands, native) = play_channel();
    native.push(vec![
        spatial(at(100, 200, -8)),
        map_snapshot(at(100, 200, -8)),
    ]);
    native.push(vec![spatial(at(101, 200, -8))]);
    assert_eq!(pushed_own(&native), Some(at(101, 200, -8)));
    assert_eq!(
        crate::play::mailbox::lock(&native.0).wire_position(at(101, 200, -8)),
        at(101, 200, -8)
    );
}

struct Recorder {
    battle_state: Option<oteryn_session::ActorCombatState>,
    walks: Arc<Mutex<Vec<ActorPosition>>>,
    combats: Arc<Mutex<Vec<CombatCommand>>>,
    contexts: Arc<Mutex<Vec<(WorldCommand, bool)>>>,
}

impl Stepper for Recorder {
    fn battle_state(&self) -> Option<oteryn_session::ActorCombatState> {
        self.battle_state
    }

    fn world_position(&self) -> Option<ActorPosition> {
        Some(at(100, 200, 7))
    }

    fn feature_snapshot(&self, domain: u32) -> Option<Vec<u8>> {
        let SessionEvent::WorldMapSnapshot(map) = map_snapshot(at(100, 200, 7)) else {
            return None;
        };
        (domain == 17)
            .then(|| oteryn_session::world_map::encode_world_map_snapshot(&map.value).ok())
            .flatten()
    }

    fn drain(&mut self) -> Vec<SessionEvent> {
        Vec::new()
    }

    async fn combat(&mut self, command: CombatCommand) -> Result<CombatReport, SessionError> {
        self.combats
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(command);
        Err(SessionError::SessionUnusable)
    }

    async fn logout(&mut self) -> Result<Option<oteryn_session::LogoutResult>, SessionError> {
        Ok(None)
    }

    async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
        Err(SessionError::SessionUnusable)
    }

    async fn serve(&mut self, _: std::time::Duration) -> Result<(), SessionError> {
        Ok(())
    }

    async fn target_walk(
        &mut self,
        position: ActorPosition,
    ) -> Result<oteryn_client_runtime::target_move::WalkResponse, SessionError> {
        self.walks
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(position);
        Err(SessionError::SessionUnusable)
    }

    async fn world_context(
        &mut self,
        command: &WorldCommand,
        raw_wire: bool,
    ) -> Result<String, SessionError> {
        self.contexts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((command.clone(), raw_wire));
        Ok(String::new())
    }
}

#[test]
fn a_target_walk_and_a_cursor_cast_on_an_ordinary_bundle_send_the_wire_floor()
-> Result<(), Box<dyn std::error::Error>> {
    let (_link, _commands, outbox) = play_channel();
    outbox.push(vec![
        spatial(at(100, 200, 7)),
        map_snapshot(at(100, 200, -7)),
    ]);
    let walks = Arc::new(Mutex::new(Vec::new()));
    let combats = Arc::new(Mutex::new(Vec::new()));
    let mut session = Recorder {
        battle_state: None,
        walks: Arc::clone(&walks),
        combats: Arc::clone(&combats),
        contexts: Arc::default(),
    };
    let runtime = oteryn_client_runtime::ClientRuntime::new()?;
    let spell = NonZeroU32::MIN;
    runtime.block_on(async {
        // The view works in the native map frame: floor -7 here is the ground floor.
        let _ = outbox.target_walk_run(&mut session, at(103, 200, -7)).await;
        let _ = outbox
            .battle_run(
                &mut session,
                crate::combat_input::PlayCommand::Combat(CombatCommand::CastAt {
                    spell,
                    position: SpellTargetPosition {
                        x: 103,
                        y: 201,
                        floor: -7,
                    },
                }),
            )
            .await;
    })?;
    assert_eq!(*walks.lock().map_err(|_| "walks")?, [at(103, 200, 7)]);
    assert_eq!(
        *combats.lock().map_err(|_| "combats")?,
        [CombatCommand::CastAt {
            spell,
            position: SpellTargetPosition {
                x: 103,
                y: 201,
                floor: 7
            }
        }]
    );
    Ok(())
}

#[test]
fn a_world_context_command_keeps_its_native_selection_and_carries_the_raw_frame()
-> Result<(), Box<dyn std::error::Error>> {
    let (_link, _commands, outbox) = play_channel();
    outbox.push(vec![
        spatial(at(100, 200, 7)),
        map_snapshot(at(100, 200, -7)),
    ]);
    let contexts = Arc::new(Mutex::new(Vec::new()));
    let mut session = Recorder {
        battle_state: None,
        walks: Arc::default(),
        combats: Arc::default(),
        contexts: Arc::clone(&contexts),
    };
    // Selections are revalidated against native map tiles; the runtime send takes only the
    // position target and creature checks to the wire frame.
    let command = WorldCommand {
        target: Target::Position(at(101, 200, -7)),
        action: Action::UseOn(std::num::NonZeroU64::MIN),
        selection: Some(Selection {
            position: at(101, 200, -7),
            binding: None,
        }),
    };
    let runtime = oteryn_client_runtime::ClientRuntime::new()?;
    runtime.block_on(async {
        let _ = outbox.world_context_run(&mut session, &command).await;
    })?;
    assert_eq!(*contexts.lock().map_err(|_| "contexts")?, [(command, true)]);
    Ok(())
}

struct Dropper {
    frame: Option<Vec<SessionEvent>>,
    moved: std::sync::mpsc::Sender<oteryn_client_runtime::inventory::MoveRequest>,
}

impl Stepper for Dropper {
    fn drain(&mut self) -> Vec<SessionEvent> {
        self.frame.take().unwrap_or_default()
    }

    async fn move_item(
        &mut self,
        request: oteryn_client_runtime::inventory::MoveRequest,
    ) -> Result<Option<oteryn_client_runtime::inventory::MoveResponse>, SessionError> {
        self.moved
            .send(request)
            .map_err(|_| SessionError::SessionUnusable)?;
        Ok(None)
    }

    async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
        Err(SessionError::SessionUnusable)
    }

    async fn logout(&mut self) -> Result<Option<oteryn_session::LogoutResult>, SessionError> {
        Ok(None)
    }

    async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
        Err(SessionError::SessionUnusable)
    }

    async fn serve(&mut self, duration: std::time::Duration) -> Result<(), SessionError> {
        tokio::time::sleep(duration).await;
        Ok(())
    }
}

#[test]
fn a_ground_drop_on_an_ordinary_bundle_keeps_the_native_map_floor() -> Result<(), String> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            use oteryn_session::item_movement::WorldTilePosition;
            let (sent, received) = std::sync::mpsc::channel();
            let session = Dropper {
                frame: Some(vec![
                    spatial(at(100, 200, 7)),
                    map_snapshot(at(100, 200, -7)),
                ]),
                moved: sent,
            };
            let (link, commands, outbox) = play_channel();
            let runtime = oteryn_client_runtime::ClientRuntime::new().map_err(|e| e.to_string())?;
            runtime
                .spawn(crate::play::run_session(session, commands, outbox))
                .map_err(|e| e.to_string())?;
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
            while crate::play::mailbox::lock(&link.mailbox).wire_frame() != WireFrame::Raw {
                if std::time::Instant::now() >= deadline {
                    return Err("frame deadline".into());
                }
                std::thread::yield_now();
            }
            // The server checks a drop against its native map position (loop_move drop_cell),
            // so the ground destination is never taken to the raw wire frame.
            let destination = oteryn_session::ItemMoveDestination::Ground(WorldTilePosition {
                x: 101,
                y: 200,
                floor: -7,
            });
            link.commands
                .send(crate::combat_input::PlayCommand::MoveItem(
                    oteryn_client_runtime::inventory::MoveRequest {
                        source: std::num::NonZeroU64::MIN,
                        destination,
                        count: None,
                    },
                ))
                .map_err(|e| e.to_string())?;
            let request = received
                .recv_timeout(std::time::Duration::from_secs(3))
                .map_err(|e| e.to_string())?;
            assert_eq!(request.destination, destination);
            drop(link);
            runtime.shutdown(std::time::Duration::from_secs(1));
            Ok(())
        })
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|_| "ground drop thread failed".to_owned())?
}

#[test]
fn picked_ground_item_look_reaches_dispatch_with_its_native_tile_and_raw_frame()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    let native = at(100, 200, -7);
    outbox.push(vec![spatial(at(100, 200, 7)), map_snapshot(native)]);
    outbox.world_context_initialize(true, false);
    let handle = std::num::NonZeroU64::new(5).ok_or("handle")?;
    let mut ground = entity(at(100, 200, 7));
    ground.entity.generation = 0;
    ground.kind = EntityKind::GroundItem;
    ground.detail = EntityDetail::Object {
        item_definition_ref: 1,
        quantity: 1,
        item_handle: Some(handle),
    };
    let mut view = crate::play::tests::view()?;
    outbox.entities(vec![ground]);
    view.tick(&link).map_err(|error| format!("{error:?}"))?;
    let menu = view
        .world_context(oteryn_renderer::TileCoord::new(native.x, native.y), &link)
        .ok_or("ground menu")?;
    assert!(view.world_context_choose(&link, &menu, Action::Look));
    let crate::combat_input::PlayCommand::WorldContext(command) = commands.try_recv()? else {
        return Err("expected world context".into());
    };
    let contexts = Arc::new(Mutex::new(Vec::new()));
    let mut session = Recorder {
        battle_state: None,
        walks: Arc::default(),
        combats: Arc::default(),
        contexts: Arc::clone(&contexts),
    };
    let runtime = oteryn_client_runtime::ClientRuntime::new()?;
    runtime.block_on(outbox.world_context_run(&mut session, &command))??;
    let recorded = contexts.lock().map_err(|_| "contexts")?;
    let [(command, true)] = recorded.as_slice() else {
        return Err("expected raw frame".into());
    };
    assert_eq!(command.target, Target::Item(handle));
    assert_eq!(command.action, Action::Look);
    assert_eq!(command.selection.as_ref().map(|s| s.position), Some(native));
    Ok(())
}

#[test]
fn review_ordinary_admission_seeds_target_walk_and_entity_floor()
-> Result<(), Box<dyn std::error::Error>> {
    let (_, _, outbox) = play_channel();
    let walks = Arc::new(Mutex::new(Vec::new()));
    let combats = Arc::new(Mutex::new(Vec::new()));
    let mut peer = Recorder {
        battle_state: None,
        walks: Arc::clone(&walks),
        combats: Arc::clone(&combats),
        contexts: Default::default(),
    };
    crate::play::session_initialize::initialize(&peer, &outbox)?;
    assert_eq!(
        crate::play::mailbox::lock(&outbox.0)
            .map
            .as_deref()
            .map(|map| map.header.origin),
        Some(at(100, 200, 7))
    );
    outbox.entities(vec![entity(at(101, 200, 7))]);
    assert_eq!(
        crate::play::mailbox::lock(&outbox.0)
            .pushed
            .entities
            .as_ref()
            .ok_or("entities")?[0]
            .position
            .floor,
        7
    );
    let runtime = tokio::runtime::Builder::new_current_thread().build()?;
    let spell = NonZeroU32::MIN;
    runtime.block_on(async {
        let _ = outbox.target_walk_run(&mut peer, at(101, 200, 7)).await;
        let _ = outbox
            .battle_run(
                &mut peer,
                crate::combat_input::PlayCommand::Combat(CombatCommand::CastAt {
                    spell,
                    position: SpellTargetPosition {
                        x: 101,
                        y: 200,
                        floor: 7,
                    },
                }),
            )
            .await;
    });
    assert!(matches!(
        combats
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_slice(),
        [CombatCommand::CastAt {
            position: SpellTargetPosition { floor: 7, .. },
            ..
        }]
    ));
    assert_eq!(
        *walks.lock().unwrap_or_else(PoisonError::into_inner),
        [at(101, 200, 7)]
    );
    Ok(())
}

#[test]
fn queued_context_stop_does_not_stop_the_new_attack_target()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::{ActorCombatState, EntityRef, FightModes};

    let (link, mut commands, outbox) = play_channel();
    let first = EntityRef {
        identity: [1; 16],
        generation: 1,
    };
    let second = EntityRef {
        identity: [2; 16],
        generation: 1,
    };
    let state = |target| {
        Some(ActorCombatState {
            target: Some(target),
            modes: FightModes::DEFAULT,
            in_fight: true,
        })
    };
    outbox.battle_publish(state(first), None, &[]);
    assert!(link.request_context_combat(first, true));
    outbox.battle_result(
        crate::combat_input::CombatReport::Attack(oteryn_session::AttackIntentDisposition::Ok),
        state(second),
        None,
    );

    let mut wire = outbox.record_queued_combat_wire_for_test(&mut commands)?;
    assert!(wire.try_recv().is_err());
    Ok(())
}

impl crate::play::Outbox {
    pub(in crate::play) fn record_queued_combat_wire_for_test(
        &self,
        commands: &mut tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
    ) -> Result<
        tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
        Box<dyn std::error::Error>,
    > {
        use crate::combat_input::PlayCommand;
        use std::time::Duration;
        use tokio::sync::mpsc::unbounded_channel;

        let command = commands.try_recv()?;
        let battle_state = crate::play::mailbox::lock(&self.0).battle.combat;
        let outbox = self.clone();
        let thread = std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(move || {
                let (sender, commands) = unbounded_channel();
                sender
                    .send(command)
                    .map_err(|_| "combat queue closed".to_owned())?;
                drop(sender);
                let combats = Arc::new(Mutex::new(Vec::new()));
                let session = Recorder {
                    battle_state,
                    walks: Arc::default(),
                    combats: Arc::clone(&combats),
                    contexts: Arc::default(),
                };
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|error| error.to_string())?;
                runtime
                    .block_on(async {
                        tokio::time::timeout(
                            Duration::from_secs(2),
                            crate::play::run_session(session, commands, outbox),
                        )
                        .await
                    })
                    .map_err(|_| "combat session timed out".to_owned())?;
                let (sender, receiver) = unbounded_channel();
                for &command in &*combats
                    .lock()
                    .map_err(|_| "combat wire recorder".to_owned())?
                {
                    sender
                        .send(PlayCommand::Combat(command))
                        .map_err(|_| "combat receiver closed".to_owned())?;
                }
                Ok::<_, String>(receiver)
            })?;
        Ok(thread.join().map_err(|_| "combat session panicked")??)
    }
}
