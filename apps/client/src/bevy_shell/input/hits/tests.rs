use super::*;
use crate::{
    bevy_shell::{
        game_ui::{GameRegions, WorldRegion, modal, viewport::WorldViewport},
        owner::{ActivePlay, HostOwner, Phase},
    },
    combat_input::PlayCommand,
    play::{PlayView, play_channel},
};
use bevy::{
    ecs::system::RunSystemOnce,
    input_focus::{FocusCause, InputFocus},
    picking::{backend::HitData, pointer::Location},
    text::EditableText,
    winit::RawWinitWindowEvent,
};
use winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, ElementState, MouseButton, WindowEvent},
    window::WindowId,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
struct Fixture {
    app: App,
    commands: tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    world: Entity,
    editor: Entity,
    pointer: Entity,
    window: Entity,
}
fn fixture(dpi: f32, cover: bool) -> Result<Fixture, Box<dyn std::error::Error>> {
    fixture_with(dpi, cover, true, |_, _, _| Ok(()))
}
type Prepare = fn(&crate::play::Outbox, &crate::play::PlayLink, &mut PlayView) -> TestResult;
fn fixture_with(
    dpi: f32,
    cover: bool,
    loot: bool,
    prepare: Prepare,
) -> Result<Fixture, Box<dyn std::error::Error>> {
    let mut owner = HostOwner::new(None)?;
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_quick_loot_selected(true);
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    let mut preferences = crate::play::ReferencePreferences::default();
    preferences.gameplay.secondary_click_loots_nearby = loot;
    view.set_preferences(preferences)?;
    prepare(&outbox, &link, &mut view)?;
    let (w, h) = view.scene().view().viewport_px();
    owner.play = Some(ActivePlay { view, link });
    owner.phase = Phase::Playing;
    let input = super::super::NativeInput {
        adapter: oteryn_input_platform::InputPlatformAdapter::new(),
        generation: Some(owner.generation()),
        physical_pointer: None,
    };
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<HoverMap>()
        .init_resource::<bevy::picking::pointer::PointerMap>()
        .init_resource::<InputFocus>()
        .add_message::<RawWinitWindowEvent>()
        .insert_non_send(owner)
        .insert_non_send(input)
        .add_systems(Update, super::super::route);
    modal::install(&mut app);
    crate::bevy_shell::game_ui::initialize_item_pointer_test_state(&mut app);
    let world = app.world_mut().spawn((WorldRegion, Node::default())).id();
    let floating = app.world_mut().spawn(Node::default()).id();
    let editor = app
        .world_mut()
        .spawn((Node::default(), EditableText::new("NPC reply")))
        .id();
    app.world_mut().entity_mut(floating).add_child(editor);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, FocusCause::Navigated);
    let mut window = Window::default();
    window.resolution.set_physical_resolution(w + 80, h + 32);
    window.resolution.set_scale_factor_override(Some(dpi));
    let window = app.world_mut().spawn((window, PrimaryWindow)).id();
    let target = bevy::camera::NormalizedRenderTarget::Window(
        bevy::window::WindowRef::Entity(window)
            .normalize(None)
            .ok_or("window")?,
    );
    let pointer = app
        .world_mut()
        .spawn((
            PointerId::Mouse,
            PointerLocation::new(Location {
                target,
                position: Vec2::splat(64. / dpi),
            }),
        ))
        .id();
    app.world_mut()
        .resource_mut::<HoverMap>()
        .entry(PointerId::Mouse)
        .or_default()
        .insert(
            if cover { editor } else { world },
            HitData::new(window, 0., None, None),
        );
    app.insert_resource(WorldViewport {
        origin: UVec2::ZERO,
        size: UVec2::new(w + 80, h + 32),
    })
    .insert_resource(GameRegions {
        vitals: world,
        minimap: world,
        equipment: world,
        containers: world,
        battle: world,
        skills: world,
        vip: world,
        analyzers: world,
        chat: world,
        bars: world,
        world,
    });
    Ok(Fixture {
        app,
        commands,
        world,
        editor,
        pointer,
        window,
    })
}
fn mouse(app: &mut App, point: Vec2) {
    app.world_mut().write_message(RawWinitWindowEvent {
        window_id: WindowId::dummy(),
        event: WindowEvent::CursorMoved {
            device_id: DeviceId::dummy(),
            position: PhysicalPosition::new(f64::from(point.x), f64::from(point.y)),
        },
    });
    app.world_mut().write_message(RawWinitWindowEvent {
        window_id: WindowId::dummy(),
        event: WindowEvent::MouseInput {
            device_id: DeviceId::dummy(),
            state: ElementState::Pressed,
            button: MouseButton::Right,
        },
    });
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
