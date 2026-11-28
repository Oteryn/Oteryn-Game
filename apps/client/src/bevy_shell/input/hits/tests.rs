use super::qualification::{Fixture, Prepare, TestResult, fixture_for_view, mouse};
use super::*;
use crate::{bevy_shell::owner::HostOwner, combat_input::PlayCommand, play::PlayView};
use bevy::{
    ecs::system::RunSystemOnce, input_focus::InputFocus, picking::backend::HitData,
    winit::RawWinitWindowEvent,
};
use winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, WindowEvent},
    window::WindowId,
};
fn fixture(dpi: f32, cover: bool) -> Result<Fixture, Box<dyn std::error::Error>> {
    fixture_with(dpi, cover, true, |_, _, _| Ok(()))
}
fn fixture_with(
    dpi: f32,
    cover: bool,
    loot: bool,
    prepare: Prepare,
) -> Result<Fixture, Box<dyn std::error::Error>> {
    let view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    fixture_for_view(view, dpi, cover, loot, prepare, false)
}

#[test]
fn nonmodal_textbox_click_keeps_focus_and_cannot_loot_or_walk_behind_ui() -> TestResult {
    let mut f = fixture(1., true)?;
    mouse(&mut f.app, Vec2::splat(64.));
    f.app.update();
    assert_eq!(f.app.world().resource::<InputFocus>().get(), Some(f.editor));
    assert!(f.commands.try_recv().is_err());
    Ok(())
}
#[test]
fn actual_world_click_clears_editor_focus_and_reaches_existing_loot_caller_at_dpi() -> TestResult {
    for dpi in [1., 1.5, 2.] {
        let mut f = fixture(dpi, false)?;
        mouse(&mut f.app, Vec2::splat(64.));
        f.app.update();
        assert_eq!(f.app.world().resource::<InputFocus>().get(), None);
        let PlayCommand::ServerMechanic(command) = f.commands.try_recv()? else {
            return Err("world action absent".into());
        };
        assert_eq!((command.capability, command.command_type), (98, 114));
    }
    Ok(())
}
#[test]
fn an_earlier_press_cannot_borrow_the_later_final_world_hover() -> TestResult {
    let mut f = fixture(1., false)?;
    f.app
        .world_mut()
        .get_mut::<PointerLocation>(f.pointer)
        .ok_or("pointer")?
        .location
        .as_mut()
        .ok_or("location")?
        .position = Vec2::splat(160.);
    mouse(&mut f.app, Vec2::splat(64.));
    f.app.world_mut().write_message(RawWinitWindowEvent {
        window_id: WindowId::dummy(),
        event: WindowEvent::CursorMoved {
            device_id: DeviceId::dummy(),
            position: PhysicalPosition::new(160., 160.),
        },
    });
    f.app.update();
    assert_eq!(f.app.world().resource::<InputFocus>().get(), Some(f.editor));
    assert!(f.commands.try_recv().is_err());
    Ok(())
}
#[derive(Resource)]
struct Probe {
    world: Entity,
    point: Vec2,
}
fn probe(hits: WorldPointerHits, p: Res<Probe>) -> (bool, bool) {
    (
        hits.matches_primary_pointer(p.point),
        hits.allows_world_start(p.world, p.point),
    )
}
#[test]
fn upstream_world_descendants_pass_but_mixed_or_nonprimary_hits_fail_closed() -> TestResult {
    let mut f = fixture(2., false)?;
    f.app.insert_resource(Probe {
        world: f.world,
        point: Vec2::splat(64.),
    });
    let child = f.app.world_mut().spawn_empty().id();
    f.app.world_mut().entity_mut(f.world).add_child(child);
    {
        let mut hover = f.app.world_mut().resource_mut::<HoverMap>();
        let map = hover.entry(PointerId::Mouse).or_default();
        map.clear();
        map.insert(child, HitData::new(f.window, 0., None, None));
    }
    assert_eq!(
        f.app
            .world_mut()
            .run_system_once(probe)
            .map_err(|error| format!("{error:?}"))?,
        (true, true)
    );
    f.app
        .world_mut()
        .resource_mut::<HoverMap>()
        .entry(PointerId::Mouse)
        .or_default()
        .insert(f.editor, HitData::new(f.window, 0., None, None));
    assert_eq!(
        f.app
            .world_mut()
            .run_system_once(probe)
            .map_err(|error| format!("{error:?}"))?,
        (true, false)
    );
    let other = f.app.world_mut().spawn_empty().id();
    f.app
        .world_mut()
        .get_mut::<PointerLocation>(f.pointer)
        .ok_or("pointer")?
        .location
        .as_mut()
        .ok_or("location")?
        .target = bevy::camera::NormalizedRenderTarget::Window(
        bevy::window::WindowRef::Entity(other)
            .normalize(None)
            .ok_or("target")?,
    );
    assert_eq!(
        f.app
            .world_mut()
            .run_system_once(probe)
            .map_err(|error| format!("{error:?}"))?,
        (false, false)
    );
    Ok(())
}

fn arm_use_with(
    outbox: &crate::play::Outbox,
    link: &crate::play::PlayLink,
    _: &mut PlayView,
) -> TestResult {
    let item = |handle: u64, count: u32| -> Result<_, Box<dyn std::error::Error>> {
        Ok(oteryn_session::ItemEntry {
            appearance_id: 0,
            handle: handle.try_into()?,
            item_definition_ref: 42.try_into()?,
            count: count.try_into()?,
            sub_type: 0,
        })
    };
    crate::play::PlayLink::bevy_items_fixture(
        outbox,
        Some(oteryn_session::CharacterInventory {
            main_backpack: Some(item(9, 1)?),
            entries: vec![item(8, 9)?],
            equipment: vec![],
        }),
        Some(Default::default()),
        false,
        true,
    );
    if !(link.inventory_select(8.try_into()?) && link.inventory_use_with()) {
        return Err("use with did not arm".into());
    }
    Ok(())
}
fn arm_cursor_spell(
    _: &crate::play::Outbox,
    _: &crate::play::PlayLink,
    view: &mut PlayView,
) -> TestResult {
    view.cursor_spell_input(&[oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(30)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Shift),
        repeat: false,
    }]);
    Ok(())
}
fn armed(owner: &HostOwner) -> Result<bool, Box<dyn std::error::Error>> {
    let play = owner.play.as_ref().ok_or("play")?;
    Ok(play.link.use_with_armed() || play.view.combat_status().contains("Click a tile"))
}
/// Tibia Global: a world right-click while Use with or a cursor spell is armed cancels the
/// crosshair; it neither loots nearby, opens the context menu nor leaves the crosshair armed.
#[test]
fn world_right_click_cancels_armed_use_with_and_cursor_spell_before_loot_or_menu() -> TestResult {
    for loot in [true, false] {
        for prepare in [arm_use_with as Prepare, arm_cursor_spell] {
            let mut f = fixture_with(1., false, loot, prepare)?;
            assert!(armed(f.app.world().non_send::<HostOwner>())?);
            mouse(&mut f.app, Vec2::splat(64.));
            f.app.update();
            let owner = f.app.world().non_send::<HostOwner>();
            assert!(
                !armed(owner)?,
                "right-click left the crosshair armed (loot {loot})"
            );
            assert!(!owner.modal_active(), "right-click opened the context menu");
            assert!(f.commands.try_recv().is_err(), "right-click looted nearby");
        }
    }
    Ok(())
}

#[test]
fn a_left_docked_action_slot_cannot_route_a_click_to_the_world() -> TestResult {
    let mut f = fixture(1., false)?;
    let anchor = f
        .app
        .world_mut()
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0.),
                ..default()
            },
            ChildOf(f.world),
        ))
        .id();
    let slot = f
        .app
        .world_mut()
        .spawn((bevy::ui_widgets::Button, Node::default(), ChildOf(anchor)))
        .id();
    let caption = f
        .app
        .world_mut()
        .spawn((Text::new("F1"), ChildOf(slot)))
        .id();
    {
        let mut hover = f.app.world_mut().resource_mut::<HoverMap>();
        let hits = hover.entry(PointerId::Mouse).or_default();
        hits.clear();
        hits.insert(caption, HitData::new(f.window, 0., None, None));
    }
    mouse(&mut f.app, Vec2::splat(64.));
    f.app.update();
    assert_eq!(f.app.world().resource::<InputFocus>().get(), Some(f.editor));
    assert!(f.commands.try_recv().is_err());
    Ok(())
}

#[test]
fn native_touch_on_world_joystick_emits_a_paced_step() -> TestResult {
    let mut f = fixture(2., false)?;
    let (w, h) = f
        .app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .view
        .scene()
        .view()
        .viewport_px();
    let scale = (w as f32 / 480.).min(h as f32 / 352.);
    let point = Vec2::new(40. + 118. * scale, 16. + h as f32 - 72. * scale);
    use bevy::picking::pointer::PointerInput;
    f.app
        .add_message::<bevy::window::WindowEvent>()
        .add_message::<PointerInput>()
        .add_systems(
            PreUpdate,
            (
                bevy::picking::input::touch_pick_events,
                PointerInput::receive,
            )
                .chain(),
        );
    for (phase, position) in [
        (bevy::input::touch::TouchPhase::Started, point),
        (bevy::input::touch::TouchPhase::Moved, point + Vec2::X),
    ] {
        f.app
            .world_mut()
            .write_message(bevy::window::WindowEvent::TouchInput(
                bevy::input::touch::TouchInput {
                    phase,
                    position: position / 2.,
                    window: f.window,
                    force: None,
                    id: 7,
                },
            ));
    }
    f.app
        .world_mut()
        .resource_mut::<HoverMap>()
        .entry(PointerId::Touch(7))
        .or_default()
        .insert(f.world, HitData::new(f.window, 0., None, None));
    f.app.world_mut().write_message(RawWinitWindowEvent {
        window_id: WindowId::dummy(),
        event: WindowEvent::Touch(winit::event::Touch {
            id: 7,
            phase: winit::event::TouchPhase::Started,
            device_id: DeviceId::dummy(),
            location: PhysicalPosition::new(f64::from(point.x), f64::from(point.y)),
            force: None,
        }),
    });
    f.app.world_mut().entity_mut(f.world).insert((
        ComputedNode {
            size: Vec2::splat(1000.),
            ..default()
        },
        bevy::ui::UiGlobalTransform::from_translation(point),
        InheritedVisibility::VISIBLE,
    ));
    f.app.insert_resource(bevy::ui::UiStack {
        partition: std::iter::once(0..1).collect(),
        uinodes: vec![f.world],
    });
    f.app.world_mut().write_message(RawWinitWindowEvent {
        window_id: WindowId::dummy(),
        event: WindowEvent::Touch(winit::event::Touch {
            id: 7,
            phase: winit::event::TouchPhase::Moved,
            device_id: DeviceId::dummy(),
            location: PhysicalPosition::new(f64::from(point.x + 1.), f64::from(point.y)),
            force: None,
        }),
    });
    f.app.update();
    let mut owner = f.app.world_mut().non_send_mut::<HostOwner>();
    let play = owner.play.as_mut().ok_or("play")?;
    play.view.tick(&play.link).map_err(|_| "play ended")?;
    assert_eq!(
        f.commands.try_recv()?,
        PlayCommand::Step(crate::input::StepDir::East)
    );
    Ok(())
}
