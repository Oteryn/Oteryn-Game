use super::*;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};

#[test]
fn native_turn_stop_and_follow_send_target_move_without_a_step()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    outbox.target_move_initialize(true);
    enable_combat(&link);
    let mut view = view()?;
    let key = |code, modifiers| NormalizedInputEvent::Key {
        code,
        modifiers,
        state: ButtonState::Pressed,
        repeat: false,
    };
    view.arrow(StepDir::East);
    assert!(view.target_move_input(
        &link,
        &[key(
            KeyCode::ARROW_UP,
            Modifiers::NONE.with(Modifier::Control)
        )]
    ));
    let crate::combat_input::PlayCommand::ServerMechanic(turn) = commands.try_recv()? else {
        return Err("missing turn".into());
    };
    assert_eq!((turn.capability, turn.command_type), (64, 81));
    assert_eq!(turn.payload, [26, 2, 8, 1]);
    assert_eq!(view.next_step(), None);
    assert_eq!(view.facing(), StepDir::South);
    outbox.target_move_result(
        &turn,
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload: vec![8, 1],
        },
    );
    view.apply_target_turn(&link)?;
    assert_eq!(view.facing(), StepDir::North);
    assert_eq!(view.scene().facing(), StepDir::North);
    assert_eq!(view.own(), TileCoord::new(100, 200));
    assert!(view.target_move_input(&link, &[key(KeyCode::ESCAPE, Modifiers::NONE)]));
    assert_eq!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::Combat(crate::combat_input::CombatCommand::Stop)
    );
    let crate::combat_input::PlayCommand::ServerMechanic(stop) = commands.try_recv()? else {
        return Err("missing stop".into());
    };
    assert_eq!(stop.payload, [16, 1]);
    // An empty selection cannot become a follow command.
    assert!(view.target_move_input(
        &link,
        &[key(
            KeyCode::new(9)?,
            Modifiers::NONE.with(Modifier::Control)
        )]
    ));
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn native_follow_uses_the_selected_generation_and_decodes_the_reply()
-> Result<(), Box<dyn std::error::Error>> {
    let target = EntityRef {
        identity: [3; 16],
        generation: 7,
    };
    let entity = oteryn_session::WorldSpatialEntity {
        motion: None,
        display_name: None,
        kind: oteryn_session::EntityKind::Player,
        entity: target,
        position: ActorPosition {
            x: 101,
            y: 200,
            floor: 0,
        },
        detail: oteryn_session::EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    let mut view = view()?.with_entities(vec![entity])?;
    view.click(TileCoord::new(101, 200))?;
    assert!(
        view.inputs.is_empty(),
        "selecting a friendly player must not attack"
    );
    let (link, mut commands, outbox) = play_channel();
    outbox.target_move_initialize(true);
    let event = NormalizedInputEvent::Key {
        code: KeyCode::new(9)?,
        modifiers: Modifiers::NONE.with(Modifier::Control),
        state: ButtonState::Pressed,
        repeat: false,
    };
    assert!(view.target_move_input(&link, std::slice::from_ref(&event)));
    let crate::combat_input::PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("missing follow".into());
    };
    assert_eq!(
        oteryn_session::target_move::decode_target_move_intent(&command.payload),
        Ok(oteryn_session::target_move::TargetMoveIntent::Follow(
            target
        ))
    );
    outbox.target_move_result(
        &command,
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload: vec![8, 1],
        },
    );
    assert_eq!(link.movement_feedback(), Some("Movement accepted"));
    view.entities.clear();
    assert!(view.target_move_input(&link, &[event]));
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn ground_click_does_not_generate_client_side_steps() -> Result<(), BatchError> {
    let mut view = view()?;
    view.click(TileCoord::new(98, 200))?;
    assert_eq!(view.next_step(), None);
    assert_eq!(view.own(), TileCoord::new(100, 200));
    Ok(())
}

#[test]
fn ground_click_uses_session_floor_and_waits_for_the_previous_receipt() -> Result<(), BatchError> {
    let mut view = PlayView::new(TileCoord::new(-10, 200), -3, &[])?;
    let (link, mut commands, outbox) = play_channel();
    outbox.target_walk_initialize(true);
    view.click(TileCoord::new(-8, 201))?;
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(
        commands.try_recv(),
        Ok(crate::combat_input::PlayCommand::TargetWalk(
            ActorPosition {
                x: -8,
                y: 201,
                floor: -3
            }
        ))
    );
    view.click(TileCoord::new(-9, 202))?;
    assert_eq!(view.tick(&link), Ok(()));
    assert!(commands.try_recv().is_err());
    assert_eq!(view.own(), TileCoord::new(-10, 200));
    oteryn_client_ui::target_move::apply(
        &mut super::super::mailbox::lock(&outbox.0).target_walk,
        oteryn_client_runtime::target_move::WalkResponse {
            status: CommandStatus::Rejected,
            disposition: Some(oteryn_client_runtime::target_move::WalkDisposition::Rejected),
        },
    );
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(
        commands.try_recv(),
        Ok(crate::combat_input::PlayCommand::TargetWalk(
            ActorPosition {
                x: -9,
                y: 202,
                floor: -3
            }
        ))
    );
    assert_eq!(view.own(), TileCoord::new(-10, 200));
    Ok(())
}

#[test]
fn an_unselected_walk_sends_nothing_and_leaves_arrows_usable() -> Result<(), BatchError> {
    let mut view = view()?;
    let (link, mut commands, _outbox) = play_channel();
    view.click(TileCoord::new(98, 200))?;
    assert_eq!(view.tick(&link), Ok(()));
    assert!(commands.try_recv().is_err());
    assert_eq!(view.tick(&link), Ok(()));
    assert!(view.hud_status().contains("Server walking unavailable"));
    view.arrow(StepDir::North);
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(
        commands.try_recv(),
        Ok(crate::combat_input::PlayCommand::Step(StepDir::North))
    );
    Ok(())
}

#[cfg(bevy_shell)]
impl crate::play::Outbox {
    pub(crate) fn bevy_target_walk_selected(&self) {
        self.target_move_initialize(true);
        self.target_walk_initialize(true);
    }
}

#[test]
fn escape_stops_authoritative_combat_and_movement() -> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    outbox.target_move_initialize(true);
    enable_combat(&link);
    let mut view = view()?;
    assert!(view.target_move_input(
        &link,
        &[NormalizedInputEvent::Key {
            code: KeyCode::ESCAPE,
            modifiers: Modifiers::NONE,
            state: ButtonState::Pressed,
            repeat: false,
        }]
    ));
    assert_eq!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::Combat(crate::combat_input::CombatCommand::Stop)
    );
    let crate::combat_input::PlayCommand::ServerMechanic(stop) = commands.try_recv()? else {
        return Err("missing movement stop".into());
    };
    assert_eq!(stop.payload, [16, 1]);
    Ok(())
}

#[test]
fn escape_stops_combat_when_target_movement_is_unavailable()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, _) = play_channel();
    enable_combat(&link);
    let mut view = view()?;
    assert!(view.target_move_input(
        &link,
        &[NormalizedInputEvent::Key {
            code: KeyCode::ESCAPE,
            modifiers: Modifiers::NONE,
            state: ButtonState::Pressed,
            repeat: false,
        }]
    ));
    assert_eq!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::Combat(crate::combat_input::CombatCommand::Stop)
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}

fn enable_combat(link: &PlayLink) {
    crate::play::mailbox::lock(&link.mailbox).battle.combat = Some(oteryn_session::ActorCombatState {
        target: Some(EntityRef { identity: [3; 16], generation: 7 }),
        modes: oteryn_session::FightModes::DEFAULT,
        in_fight: true,
    });
}

#[test]
fn escape_without_combat_capability_only_stops_movement() -> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    outbox.target_move_initialize(true);
    let mut view = view()?;
    assert!(view.target_move_input(&link, &[NormalizedInputEvent::Key {
        code: KeyCode::ESCAPE, modifiers: Modifiers::NONE,
        state: ButtonState::Pressed, repeat: false,
    }]));
    let crate::combat_input::PlayCommand::ServerMechanic(stop) = commands.try_recv()? else {
        return Err("Escape attempted unselected combat capability".into());
    };
    assert_eq!(stop.payload, [16, 1]);
    assert!(commands.try_recv().is_err());
    Ok(())
}

fn walk_reply(
    outbox: &crate::play::Outbox,
    disposition: oteryn_client_runtime::target_move::WalkDisposition,
) {
    use oteryn_client_runtime::target_move::{WalkDisposition, WalkResponse};
    oteryn_client_ui::target_move::apply(
        &mut super::super::mailbox::lock(&outbox.0).target_walk,
        WalkResponse {
            status: if disposition == WalkDisposition::Rejected {
                CommandStatus::Rejected
            } else {
                CommandStatus::Accepted
            },
            disposition: Some(disposition),
        },
    );
}

fn own_at(outbox: &crate::play::Outbox, sequence: u64, x: i32) {
    outbox.push(vec![oteryn_session::SessionEvent::WorldSpatial(
        oteryn_session::AppliedDelta {
            server_sequence: sequence,
            base_revision: sequence,
            new_revision: sequence + 1,
            value: oteryn_session::WorldSpatialObservation {
                content_generation: [0; 32],
                actor_position: ActorPosition {
                    x,
                    y: 200,
                    floor: 0,
                },
            },
        },
    )]);
}

/// The server's terminal refusal after a blocked step (`serve/target_move.rs` `no_way`).
fn no_way(outbox: &crate::play::Outbox, sequence: u64) {
    outbox.push(vec![oteryn_session::SessionEvent::TutorialNotice(
        oteryn_session::AppliedDelta {
            server_sequence: sequence,
            base_revision: 0,
            new_revision: 1,
            value: oteryn_client_runtime::tutorial_notices::TutorialNotice::StatusText(
                "There is no way.".into(),
            ),
        },
    )]);
}

const GOAL: ActorPosition = ActorPosition {
    x: 104,
    y: 200,
    floor: 0,
};

#[test]
fn t14_click_walk_re_requests_the_same_goal_after_a_blocked_step_until_arrival()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::combat_input::PlayCommand;
    use oteryn_client_runtime::target_move::WalkDisposition;
    let mut view = PlayView::new(TileCoord::new(100, 200), 0, &[])?;
    let (link, mut commands, outbox) = play_channel();
    outbox.target_walk_initialize(true);
    view.click(TileCoord::new(104, 200))?;
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(commands.try_recv()?, PlayCommand::TargetWalk(GOAL));
    walk_reply(&outbox, WalkDisposition::Accepted);
    own_at(&outbox, 1, 101);
    assert_eq!(view.tick(&link), Ok(()));
    assert!(commands.try_recv().is_err());
    // A blocked step: Tibia Global asks for a new route to the same destination.
    no_way(&outbox, 2);
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(commands.try_recv()?, PlayCommand::TargetWalk(GOAL));
    walk_reply(&outbox, WalkDisposition::Accepted);
    own_at(&outbox, 3, 104);
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(link.target_walk_state().goal, None, "arrival ends the walk");
    no_way(&outbox, 4);
    assert_eq!(view.tick(&link), Ok(()));
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn t14_click_walk_retries_are_bounded_and_a_final_no_path_stops_them()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::combat_input::PlayCommand;
    use oteryn_client_runtime::target_move::WalkDisposition;
    let mut view = PlayView::new(TileCoord::new(100, 200), 0, &[])?;
    let (link, mut commands, outbox) = play_channel();
    outbox.target_walk_initialize(true);
    view.click(TileCoord::new(104, 200))?;
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(commands.try_recv()?, PlayCommand::TargetWalk(GOAL));
    for sequence in 1..=oteryn_client_ui::target_move::MAX_WALK_RETRIES {
        walk_reply(&outbox, WalkDisposition::Accepted);
        no_way(&outbox, u64::from(sequence));
        assert_eq!(view.tick(&link), Ok(()));
        assert_eq!(commands.try_recv()?, PlayCommand::TargetWalk(GOAL));
    }
    walk_reply(&outbox, WalkDisposition::Accepted);
    no_way(&outbox, 10);
    assert_eq!(view.tick(&link), Ok(()));
    assert!(commands.try_recv().is_err(), "the retries are bounded");
    assert_eq!(link.target_walk_state().goal, None);

    // A re-request the server answers with no path is final.
    view.click(TileCoord::new(104, 200))?;
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(commands.try_recv()?, PlayCommand::TargetWalk(GOAL));
    walk_reply(&outbox, WalkDisposition::Accepted);
    no_way(&outbox, 11);
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(commands.try_recv()?, PlayCommand::TargetWalk(GOAL));
    walk_reply(&outbox, WalkDisposition::NoPath);
    no_way(&outbox, 12);
    assert_eq!(view.tick(&link), Ok(()));
    assert!(commands.try_recv().is_err());
    assert_eq!(link.target_walk_state().goal, None);
    Ok(())
}

#[test]
fn t14_a_silent_server_cancel_re_requests_the_route_after_the_walk_stalls()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::combat_input::PlayCommand;
    use oteryn_client_runtime::target_move::WalkDisposition;
    let mut view = PlayView::new(TileCoord::new(100, 200), 0, &[])?;
    let (link, mut commands, outbox) = play_channel();
    outbox.target_walk_initialize(true);
    view.click(TileCoord::new(104, 200))?;
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(commands.try_recv()?, PlayCommand::TargetWalk(GOAL));
    walk_reply(&outbox, WalkDisposition::Accepted);
    own_at(&outbox, 1, 102);
    assert_eq!(view.tick(&link), Ok(()));
    assert!(commands.try_recv().is_err());
    // The server dropped the walk without a notice; no step has arrived for the stall window.
    let stalled = std::time::Instant::now()
        .checked_sub(oteryn_client_ui::target_move::WALK_STALL)
        .ok_or("clock")?;
    super::super::mailbox::lock(&outbox.0)
        .target_walk
        .last_progress = Some(stalled);
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(commands.try_recv()?, PlayCommand::TargetWalk(GOAL));
    Ok(())
}

#[test]
fn t14_a_slow_step_inside_the_server_step_time_is_not_a_stall()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::combat_input::PlayCommand;
    use oteryn_client_runtime::target_move::WalkDisposition;
    use oteryn_protocol_oteryn::{actor_motion::ActorMotion, world_spatial_entities as wire};
    // A paralyzed character at speed 40: a diagonal step on default ground takes 4,650 ms.
    const SLOW_STEP_MS: u32 = 4_650;
    let mut view = PlayView::new(TileCoord::new(100, 200), 0, &[])?;
    let (link, mut commands, outbox) = play_channel();
    outbox.target_walk_initialize(true);
    view.click(TileCoord::new(104, 200))?;
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(commands.try_recv()?, PlayCommand::TargetWalk(GOAL));
    walk_reply(&outbox, WalkDisposition::Accepted);
    let own = |x| wire::WorldSpatialEntity {
        motion: Some(ActorMotion {
            translation_revision: 1,
            step_duration_ms: SLOW_STEP_MS,
            prewalk_ms: [1_100, 1_100, 1_100, 1_100, SLOW_STEP_MS, SLOW_STEP_MS, SLOW_STEP_MS, SLOW_STEP_MS],
            ..ActorMotion::default()
        }),
        display_name: None,
        kind: wire::EntityKind::Player,
        entity: wire::EntityRef {
            identity: [1; 16],
            generation: 1,
        },
        position: ActorPosition { x, y: 200, floor: 0 },
        detail: wire::EntityDetail::Actor {
            direction: oteryn_session::StepDirection::East,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    outbox.hotkey_own_actor(Some(own(101)));
    own_at(&outbox, 1, 101);
    assert_eq!(view.tick(&link), Ok(()));
    assert!(commands.try_recv().is_err());
    let since = |elapsed: std::time::Duration| {
        std::time::Instant::now()
            .checked_sub(elapsed)
            .ok_or("clock")
    };
    // Past the fixed stall window but inside the server's own step time: still walking.
    super::super::mailbox::lock(&outbox.0)
        .target_walk
        .last_progress = Some(since(std::time::Duration::from_millis(4_000))?);
    assert_eq!(view.tick(&link), Ok(()));
    assert!(commands.try_recv().is_err(), "a slow step is not re-requested");
    assert_eq!(link.target_walk_state().retries, 0);
    // No step for the stall window beyond that step time: the walk was dropped.
    super::super::mailbox::lock(&outbox.0)
        .target_walk
        .last_progress = Some(since(
        oteryn_client_ui::target_move::WALK_STALL
            + std::time::Duration::from_millis(u64::from(SLOW_STEP_MS)),
    )?);
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(commands.try_recv()?, PlayCommand::TargetWalk(GOAL));
    Ok(())
}

#[test]
fn t14_user_input_ends_the_click_walk_and_its_retries() -> Result<(), Box<dyn std::error::Error>> {
    use crate::combat_input::PlayCommand;
    use oteryn_client_runtime::target_move::WalkDisposition;
    let mut view = PlayView::new(TileCoord::new(100, 200), 0, &[])?;
    let (link, mut commands, outbox) = play_channel();
    outbox.target_walk_initialize(true);
    outbox.target_move_initialize(true);
    view.click(TileCoord::new(104, 200))?;
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(commands.try_recv()?, PlayCommand::TargetWalk(GOAL));
    walk_reply(&outbox, WalkDisposition::Accepted);
    assert!(view.target_move_input(
        &link,
        &[NormalizedInputEvent::Key {
            code: KeyCode::ESCAPE,
            modifiers: Modifiers::NONE,
            state: ButtonState::Pressed,
            repeat: false,
        }]
    ));
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::ServerMechanic(_)
    ));
    no_way(&outbox, 1);
    assert_eq!(view.tick(&link), Ok(()));
    assert!(commands.try_recv().is_err());
    assert_eq!(link.target_walk_state().goal, None);

    // A manual step ends a walk as well.
    view.click(TileCoord::new(104, 200))?;
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(commands.try_recv()?, PlayCommand::TargetWalk(GOAL));
    walk_reply(&outbox, WalkDisposition::Accepted);
    view.arrow(StepDir::North);
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(commands.try_recv()?, PlayCommand::Step(StepDir::North));
    no_way(&outbox, 2);
    assert_eq!(view.tick(&link), Ok(()));
    assert!(commands.try_recv().is_err());
    Ok(())
}

/// Using or moving an object walks to it on the server; that walk replaces the click walk, so
/// its "There is no way." must not re-request the old destination.
#[test]
fn t14_an_action_that_walks_to_its_object_ends_the_click_walk()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::combat_input::PlayCommand;
    use oteryn_client_runtime::world_interaction::{Action, Command, Target};
    for (index, command) in [
        PlayCommand::WorldContext(Command {
            target: Target::Item(oteryn_session::ItemHandle::MIN),
            action: Action::Use,
            selection: None,
        }),
        PlayCommand::MoveItem(oteryn_client_runtime::inventory::MoveRequest {
            source: oteryn_session::ItemHandle::MIN,
            destination: oteryn_client_runtime::inventory::ItemMoveDestination::MainBackpack,
            count: None,
        }),
    ]
    .into_iter()
    .enumerate()
    {
        let (link, commands, outbox) = play_channel();
        // The session start re-initializes the walk state; the marker shows it has run.
        super::super::mailbox::lock(&outbox.0).target_walk.retries = u8::MAX;
        let walk_outbox = outbox.clone();
        let fake = super::Fake {
            serves: std::sync::Arc::default(),
            fail_step: false,
            fail_serve_after: usize::MAX,
            queued: Vec::new(),
            drained: std::sync::Arc::default(),
            combats: std::sync::Arc::default(),
            logouts: std::sync::Arc::default(),
        };
        let runtime = oteryn_client_runtime::ClientRuntime::new()?;
        runtime.spawn(super::run_session(fake, commands, outbox))?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while link.target_walk_state().retries == u8::MAX && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        {
            let mut mailbox = super::super::mailbox::lock(&link.mailbox);
            mailbox.target_walk.selected = true;
            assert_eq!(
                oteryn_client_ui::target_move::request(&mut mailbox.target_walk, GOAL),
                Some(GOAL)
            );
        }
        walk_reply(
            &walk_outbox,
            oteryn_client_runtime::target_move::WalkDisposition::Accepted,
        );
        assert_eq!(link.target_walk_state().goal, Some(GOAL));
        link.commands.send(command)?;
        while link.target_walk_state().goal.is_some() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert_eq!(link.target_walk_state().goal, None, "command {index}");
        runtime.shutdown(std::time::Duration::from_millis(250));
    }
    Ok(())
}
