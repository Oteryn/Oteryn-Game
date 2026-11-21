use super::*;
#[test]
fn parity_a_shift_world_click_uses_existing_look_command() -> Result<(), Box<dyn std::error::Error>>
{
    use oteryn_input_actions::{
        ButtonState, Modifier, Modifiers, MouseButton, PointerCoordinate, PointerDelta,
        PointerMotion, PointerPosition,
    };
    for button in [MouseButton::PRIMARY, MouseButton::SECONDARY] {
        let mut owner = HostOwner::new(None)?;
        let (link, mut commands, outbox) = crate::play::play_channel();
        crate::play::PlayLink::bevy_items_fixture(&outbox, None, None, false, false);
        let target = oteryn_session::EntityRef {
            identity: [3; 16],
            generation: 7,
        };
        let view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?
            .with_entities(vec![oteryn_session::WorldSpatialEntity {
            motion: None,
            entity: target,
            kind: oteryn_session::EntityKind::Creature,
            position: oteryn_session::ActorPosition {
                x: 101,
                y: 100,
                floor: 7,
            },
            display_name: None,
            detail: oteryn_session::EntityDetail::Actor {
                direction: oteryn_session::StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        }])?;
        owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
        owner.phase = crate::bevy_shell::owner::Phase::Playing;
        let (x, y) = (0..600)
            .step_by(8)
            .flat_map(|y| (0..1000).step_by(8).map(move |x| (x, y)))
            .find(|(x, y)| {
                owner.play.as_ref().is_some_and(|play| {
                    play.view.click_tile(f64::from(*x), f64::from(*y))
                        == Some(oteryn_renderer::TileCoord::new(101, 100))
                })
            })
            .ok_or("visible target pixel")?;
        owner.route_input(
            &[
                NormalizedInputEvent::PointerMoved {
                    position: PointerPosition::new(
                        PointerCoordinate::new(x)?,
                        PointerCoordinate::new(y)?,
                    ),
                    motion: PointerMotion::new(PointerDelta::new(0)?, PointerDelta::new(0)?),
                },
                NormalizedInputEvent::MouseButton {
                    button,
                    state: ButtonState::Pressed,
                    modifiers: Modifiers::one(Modifier::Shift),
                },
            ],
            false,
        );
        assert!(commands.try_recv().is_err());
        owner.route_input(
            &[NormalizedInputEvent::MouseButton {
                button,
                state: ButtonState::Released,
                modifiers: Modifiers::one(Modifier::Shift),
            }],
            false,
        );
        assert!(
            matches!(commands.try_recv()?, crate::combat_input::PlayCommand::WorldContext(command) if command.target == oteryn_client_ui::world_interaction::menu::Target::Creature(target) && command.action == oteryn_client_ui::world_interaction::menu::Action::Look)
        );
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
