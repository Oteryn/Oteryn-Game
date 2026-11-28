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
pub(super) type TestResult = Result<(), Box<dyn std::error::Error>>;
pub(super) struct Fixture {
    pub(super) app: App,
    pub(super) outbox: crate::play::Outbox,
    pub(super) commands: tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    pub(super) world: Entity,
    #[cfg(test)]
    pub(super) editor: Entity,
    pub(super) pointer: Entity,
    pub(super) window: Entity,
}
pub(super) type Prepare =
    fn(&crate::play::Outbox, &crate::play::PlayLink, &mut PlayView) -> TestResult;
pub(super) fn fixture_for_view(
    mut view: PlayView,
    dpi: f32,
    cover: bool,
    loot: bool,
    prepare: Prepare,
    picking: bool,
) -> Result<Fixture, Box<dyn std::error::Error>> {
    let mut owner = HostOwner::for_input_fixture()?;
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    #[cfg(test)]
    outbox.bevy_quick_loot_selected(true);
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
    if picking {
        app.add_plugins((
            bevy::asset::AssetPlugin::default(),
            bevy::window::WindowPlugin {
                primary_window: None,
                close_when_requested: false,
                ..default()
            },
            bevy::input::InputPlugin,
            bevy::input_focus::InputFocusPlugin,
            bevy::transform::TransformPlugin,
            bevy::image::ImagePlugin::default(),
            bevy::image::TextureAtlasPlugin,
            bevy::mesh::MeshPlugin,
            bevy::camera::CameraPlugin,
            bevy::text::TextPlugin,
            bevy::ui::UiPlugin,
            bevy::picking::DefaultPickingPlugins,
            bevy::ui_widgets::ButtonPlugin,
        ))
        .init_resource::<Assets<TextureAtlasLayout>>();
    }
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
    if !picking {
        app.world_mut()
            .resource_mut::<HoverMap>()
            .entry(PointerId::Mouse)
            .or_default()
            .insert(
                if cover { editor } else { world },
                HitData::new(window, 0., None, None),
            );
    }
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
        outbox,
        commands,
        world,
        #[cfg(test)]
        editor,
        pointer,
        window,
    })
}
pub(super) fn mouse(app: &mut App, point: Vec2) {
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

#[cfg(feature = "qualification-i09")]
pub(in crate::bevy_shell) fn qualification_input(
    view: PlayView,
) -> Result<
    (
        App,
        tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
        crate::play::Outbox,
    ),
    Box<dyn std::error::Error>,
> {
    let f = fixture_for_view(view, 1., false, false, |_, _, _| Ok(()), true)?;
    let Fixture {
        mut app,
        commands,
        outbox,
        pointer,
        world,
        window,
        ..
    } = f;
    let view = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("active play")?
        .view;
    let (w, h) = view.scene().view().viewport_px();

    use bevy::camera::{ComputedCameraValues, RenderTargetInfo};
    app.world_mut().spawn((
        Camera2d,
        IsDefaultUiCamera,
        Camera {
            computed: ComputedCameraValues {
                target_info: Some(RenderTargetInfo {
                    physical_size: UVec2::new(w, h),
                    scale_factor: 1.,
                }),
                ..default()
            },
            ..default()
        },
    ));
    app.world_mut()
        .get_mut::<Window>(window)
        .ok_or("I09 window")?
        .resolution
        .set_physical_resolution(w, h);
    app.world_mut().entity_mut(world).insert((
        Node {
            width: percent(100.),
            height: percent(100.),
            ..default()
        },
        InheritedVisibility::VISIBLE,
    ));
    app.world_mut().resource_mut::<WorldViewport>().size = UVec2::new(w, h);
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.world_mut().entity_mut(pointer).despawn();
    Ok((app, commands, outbox))
}

#[cfg(feature = "qualification-i09")]
pub(in crate::bevy_shell) fn qualification_right_click(
    app: &mut App,
    tile: oteryn_renderer::TileCoord,
) -> TestResult {
    let view = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("active play")?
        .view;
    let [x, y] = view.scene().view().tile_to_screen(tile);
    let world = app.world().resource::<GameRegions>().world;
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())?;
    let click = Vec2::new(x + 1., y + 1.);
    use bevy::picking::pointer::{PointerAction, PointerInput};
    let target = bevy::window::WindowRef::Entity(window)
        .normalize(None)
        .ok_or("I09 pick window")?;
    app.update();
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        Location {
            target: bevy::camera::NormalizedRenderTarget::Window(target),
            position: click,
        },
        PointerAction::Move { delta: Vec2::ZERO },
    ));
    app.update();
    app.update();
    if !app
        .world()
        .resource::<HoverMap>()
        .get(&PointerId::Mouse)
        .is_some_and(|hits| hits.contains_key(&world))
    {
        return Err("I09 UI picking did not hit the world".into());
    }
    mouse(app, click);
    app.update();
    Ok(())
}
