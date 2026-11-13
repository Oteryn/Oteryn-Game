use super::picking_tests::{current_pointer, pointer, route};
use super::*;
use bevy::{
    picking::{
        backend::HitData,
        events::{PointerDragDrop, PointerPress},
        pointer::{PointerButton, PointerId},
    },
    window::PrimaryWindow,
};
fn spell_key() -> Result<NormalizedInputEvent, oteryn_input_actions::InputError> {
    Ok(NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(30)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(oteryn_input_actions::Modifier::Shift),
        repeat: false,
    })
}
fn arm(owner: &mut HostOwner, cursor: bool) -> TestResult {
    let play = owner.play.as_mut().ok_or("play")?;
    if cursor {
        play.view.cursor_spell_input(&[spell_key()?]);
        assert!(play.view.combat_status().contains("Click a tile"));
    } else {
        assert!(play.link.inventory_select(8.try_into()?));
        assert!(play.link.inventory_use_with());
    }
    Ok(())
}
#[test]
fn normalized_cursor_spell_press_casts_without_arming_a_ground_drag() -> TestResult {
    priority_click(true)
}
#[test]
fn normalized_use_with_press_sends_its_command_without_arming_a_ground_drag() -> TestResult {
    priority_click(false)
}
fn priority_click(cursor: bool) -> TestResult {
    let (mut owner, ui, gesture, viewport, point, mut rx) = fixture(false, false)?;
    if !cursor {
        arm(&mut owner, false)?;
    }
    let mut app = App::new();
    super::super::super::initialize_item_pointer_test_state(&mut app);
    app.insert_non_send(owner)
        .insert_resource(ui)
        .insert_resource(gesture)
        .add_observer(picking::press);
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    let world = app
        .world_mut()
        .spawn((
            WorldRegion,
            Node::default(),
            ComputedNode {
                size: viewport.size.as_vec2(),
                ..default()
            },
            UiGlobalTransform::from_translation(
                viewport.origin.as_vec2() + viewport.size.as_vec2() * 0.5,
            ),
        ))
        .observe(picking::world_drop)
        .id();
    current_pointer(&mut app, window, point, 1.);
    // Picking precedes normalized keyboard routing in the real schedule. Select the
    // cursor spell through that keyboard route before the following native press.
    app.world_mut().trigger(PointerPress {
        entity: world,
        pointer: pointer(window, point, 1.),
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
        count: 1,
    });
    if cursor {
        route(&mut app, viewport, point, spell_key()?)?;
        assert!(
            app.world()
                .non_send::<HostOwner>()
                .play
                .as_ref()
                .ok_or("play")?
                .view
                .combat_status()
                .contains("Click a tile")
        );
    }
    route(&mut app, viewport, point, motion(point)?)?;
    route(&mut app, viewport, point, button(ButtonState::Pressed))?;
    if cursor {
        assert!(
            !app.world()
                .non_send::<HostOwner>()
                .play
                .as_ref()
                .ok_or("play")?
                .view
                .combat_status()
                .contains("Click a tile")
        );
    }
    let end = point + Vec2::X * 48.;
    route(&mut app, viewport, end, motion(end)?)?;
    current_pointer(&mut app, window, end, 1.);
    app.world_mut()
        .resource_mut::<bevy::picking::hover::HoverMap>()
        .entry(PointerId::Mouse)
        .or_default()
        .insert(world, HitData::new(Entity::PLACEHOLDER, 0., None, None));
    app.world_mut().trigger(PointerDragDrop {
        entity: world,
        dropped: world,
        pointer: pointer(window, end, 1.),
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
    });
    route(&mut app, viewport, end, button(ButtonState::Released))?;
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    let play = owner.play.as_mut().ok_or("play")?;
    play.view
        .tick(&play.link)
        .map_err(|error| format!("mode dispatch: {error:?}"))?;
    let commands = std::iter::from_fn(|| rx.try_recv().ok()).collect::<Vec<_>>();
    if cursor {
        assert!(
            commands.iter().any(|command| matches!(
                command,
                PlayCommand::Combat(crate::combat_input::CombatCommand::CastAt { .. })
            )),
            "ordinary cursor spell still dispatches"
        );
    } else {
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, PlayCommand::WorldContext(_))),
            "ordinary UseWith still dispatches"
        );
    }
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, PlayCommand::MoveItem(_))),
        "a consumed world click cannot also move the ground item"
    );
    Ok(())
}
#[test]
fn active_ground_drag_and_prompt_revalidate_existing_cursor_or_use_with_owner_mode() -> TestResult {
    for cursor in [false, true] {
        for prompt_open in [false, true] {
            let (mut owner, mut ui, mut gesture, viewport, point, mut rx) =
                fixture(prompt_open, false)?;
            start(&mut owner, &mut ui, &mut gesture, viewport, point)?;
            assert!(gesture.active);
            if prompt_open {
                gesture.picking.drop_on(
                    0,
                    owner.generation(),
                    Entity::PLACEHOLDER,
                    picking::Destination::Slot(Slot {
                        generation: owner.generation(),
                        location: Location::Inventory,
                        item: None,
                        destination: Some(ItemMoveDestination::MainBackpack),
                    }),
                );
                step(
                    &mut owner,
                    &mut ui,
                    &mut gesture,
                    viewport,
                    Vec2::ZERO,
                    button(ButtonState::Released),
                )?;
                let (item, destination) = owner
                    .play
                    .as_ref()
                    .ok_or("play")?
                    .link
                    .item_movement()
                    .prompt
                    .ok_or("prompt")?;
                let prompt = Prompt {
                    generation: owner.generation(),
                    item,
                    destination,
                };
                ui.prompt = Some(prompt);
                ui.split_text = Some("3".into());
                arm(&mut owner, cursor)?;
                super::super::super::amount::confirm(&owner, &mut ui, prompt);
            } else {
                arm(&mut owner, cursor)?;
                note_world(&mut gesture);
                step(
                    &mut owner,
                    &mut ui,
                    &mut gesture,
                    viewport,
                    point + Vec2::X * 48.,
                    button(ButtonState::Released),
                )?;
            }
            assert!(
                rx.try_recv().is_err(),
                "existing mode owns world input cursor={cursor}, prompt={prompt_open}"
            );
        }
    }
    Ok(())
}
