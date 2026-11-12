use super::*;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};

#[test]
fn native_turn_stop_and_follow_send_target_move_without_a_step()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    outbox.target_move_initialize(true);
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
