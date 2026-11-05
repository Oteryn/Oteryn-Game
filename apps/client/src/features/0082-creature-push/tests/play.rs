use super::*;
use oteryn_input_actions::{
    ButtonState, InputError, Modifiers, MouseButton, NormalizedInputEvent, PointerCoordinate,
    PointerDelta, PointerMotion, PointerPosition,
};
fn pointer(x: i32, y: i32) -> Result<NormalizedInputEvent, InputError> {
    Ok(NormalizedInputEvent::PointerMoved {
        position: PointerPosition::new(PointerCoordinate::new(x)?, PointerCoordinate::new(y)?),
        motion: PointerMotion::new(PointerDelta::new(0)?, PointerDelta::new(0)?),
    })
}
fn button(state: ButtonState) -> NormalizedInputEvent {
    NormalizedInputEvent::MouseButton {
        button: MouseButton::PRIMARY,
        state,
        modifiers: Modifiers::NONE,
    }
}
#[test]
fn native_drag_sends_push_once_without_attacking_or_walking()
-> Result<(), Box<dyn std::error::Error>> {
    for kind in [
        oteryn_session::EntityKind::Creature,
        oteryn_session::EntityKind::Player,
    ] {
        let target = EntityRef {
            identity: [4; 16],
            generation: 8,
        };
        let entity = oteryn_session::WorldSpatialEntity {
            display_name: None,
            kind,
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
        let ground = oteryn_session::WorldSpatialEntity {
            display_name: None,
            kind: oteryn_session::EntityKind::GroundItem,
            entity: EntityRef {
                identity: [9; 16],
                generation: 0,
            },
            position: entity.position,
            detail: oteryn_session::EntityDetail::Object {
                item_definition_ref: 42,
                quantity: 1,
                item_handle: Some(std::num::NonZeroU64::MIN),
            },
        };
        let mut view = view()?.with_entities(vec![ground, entity])?;
        assert!(view.item_drag_source(TileCoord::new(101, 200)).is_some());
        let (link, mut commands, outbox) = play_channel();
        outbox.creature_push_initialize(true);
        outbox.target_move_initialize(true);
        let [x, y] = view.scene.view().tile_to_screen(TileCoord::new(101, 200));
        view.world_pointer_input(
            &link,
            &[
                pointer(x as i32 + 1, y as i32 + 1)?,
                button(ButtonState::Pressed),
            ],
        )?;
        assert!(commands.try_recv().is_err());
        assert!(view.item_drag_source(TileCoord::new(101, 200)).is_none());
        view.walk_to(TileCoord::new(105, 200));
        view.arrow(StepDir::East);
        view.world_pointer_input(
            &link,
            &[
                pointer(x as i32 + 49, y as i32 + 1)?,
                button(ButtonState::Released),
            ],
        )?;
        let crate::combat_input::PlayCommand::ServerMechanic(stop) = commands.try_recv()? else {
            return Err("missing walk cancellation".into());
        };
        assert_eq!(
            (stop.capability, stop.command_type, stop.payload),
            (64, 81, vec![16, 1])
        );
        let crate::combat_input::PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
            return Err("missing push".into());
        };
        assert_eq!((command.capability, command.command_type), (82, 98));
        assert_eq!(
            oteryn_session::creature_push::decode_creature_push_intent(&command.payload),
            Ok(oteryn_session::creature_push::CreaturePushIntent {
                target,
                destination: ActorPosition {
                    x: 102,
                    y: 200,
                    floor: 0
                }
            })
        );
        assert!(view.inputs.is_empty());

        view.world_pointer_input(&link, &[button(ButtonState::Released)])?;
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}

#[test]
fn native_drag_cancels_on_focus_loss_or_stale_target_and_decodes_refusal()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    outbox.creature_push_initialize(true);
    let mut view = view()?;
    let target = EntityRef {
        identity: [4; 16],
        generation: 8,
    };
    view.creature_drag.source = Some((TileCoord::new(101, 200), 0, Some(target)));
    view.world_pointer_input(
        &link,
        &[
            NormalizedInputEvent::FocusChanged { focused: false },
            button(ButtonState::Released),
        ],
    )?;
    assert!(view.creature_drag.source.is_none());
    assert!(commands.try_recv().is_err());
    let [x, y] = view.scene.view().tile_to_screen(TileCoord::new(102, 200));
    view.creature_drag.source = Some((TileCoord::new(101, 200), 0, Some(target)));
    view.world_pointer_input(
        &link,
        &[
            pointer(x as i32 + 1, y as i32 + 1)?,
            button(ButtonState::Released),
        ],
    )?;
    assert!(commands.try_recv().is_err());
    outbox.creature_push_result(
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload: vec![8, 5],
        },
    );
    assert_eq!(link.movement_feedback(), Some("Not enough room to push"));
    assert_eq!(view.own(), TileCoord::new(100, 200));
    Ok(())
}

#[test]
fn creature_pointer_preserves_cl1_ground_clicks_and_cl2_cursor_targeting()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, _) = play_channel();
    let mut view = view()?;
    let tile = TileCoord::new(103, 200);
    let [x, y] = view.scene.view().tile_to_screen(tile);
    let events = [
        pointer(x as i32 + 1, y as i32 + 1)?,
        button(ButtonState::Pressed),
    ];
    assert!(!view.world_pointer_input(&link, &events)?);
    view.primary_click(tile, &link)?;
    assert!(matches!(view.inputs.front(), Some(Input::TargetWalk(_))));
    view.inputs.clear();
    view.cursor_spell = Some(std::num::NonZeroU32::MIN);
    assert!(!view.world_pointer_input(&link, &events)?);
    view.primary_click(tile, &link)?;
    assert!(matches!(view.inputs.front(), Some(Input::CastAt(_, _))));
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn death_discards_queued_cl3_controls_without_discarding_logout()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    for capability in [64, 82] {
        link.commands
            .send(crate::combat_input::PlayCommand::ServerMechanic(
                oteryn_client_runtime::server_mechanic::ServerMechanicCommand {
                    capability,
                    command_type: if capability == 64 { 81 } else { 98 },
                    payload: vec![],
                },
            ))?;
    }
    link.request_logout();
    outbox.vitals(Some(oteryn_session::ActorVitals {
        health: 0,
        max_health: 150,
        ..Default::default()
    }));
    assert_eq!(
        outbox.death_command(&mut commands)?,
        crate::combat_input::PlayCommand::Logout
    );
    assert!(outbox.death_command(&mut commands).is_err());
    Ok(())
}
