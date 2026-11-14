use super::*;
use bevy::asset::AssetPlugin;
use oteryn_foundation::CancellationToken;
use oteryn_platform_client::native_login::{CharacterAvailability, OwnerCharacter};
use std::{
    thread,
    time::{Duration, Instant},
};
#[test]
fn picker_buttons_return_server_channel_through_existing_callback()
-> Result<(), Box<dyn std::error::Error>> {
    let (bridge, callback) = SelectionBridge::new();
    let mut owner = HostOwner::new(None)?;
    owner.settings.preferences.miscellaneous.confirm_exit = true;
    owner.selection = bridge;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<InputFocus>()
        .insert_non_send(owner)
        .add_systems(Startup, setup)
        .add_systems(Update, (poll, project).chain());
    app.add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Startup, crate::bevy_shell::ui::setup)
        .add_systems(Update, keyboard);
    crate::bevy_shell::game_ui::modal::install(&mut app);
    crate::bevy_shell::exit::install(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    app.update();
    let worker = thread::spawn(move || {
        callback(
            &[crate::character_selection::CharacterRow {
                character: OwnerCharacter {
                    character_id: "server-character".into(),
                    world_id: "server-world".into(),
                    name: "Server character".into(),
                    availability: CharacterAvailability::Available,
                },
                world_name: "Server world".into(),
                channels: vec![("server-channel".into(), "Server channel".into())],
                available: true,
            }],
            "server-character",
            CancellationToken::new(),
        )
    });
    let deadline = Instant::now() + Duration::from_secs(3);
    while app
        .world()
        .non_send::<HostOwner>()
        .selection
        .pending()
        .is_none()
        && Instant::now() < deadline
    {
        app.update();
        thread::yield_now();
    }
    app.update();
    let find = |app: &mut App, channel: bool| -> Result<Entity, &'static str> {
        app.world_mut()
            .query::<(Entity, &Target)>()
            .iter(app.world())
            .find(|(_, target)| {
                if channel {
                    matches!(target.choice, Choice::Channel)
                } else {
                    matches!(target.choice, Choice::Play)
                }
            })
            .map(|(entity, _)| entity)
            .ok_or("picker button missing")
    };
    let initial_play = find(&mut app, false)?;
    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        Some(initial_play)
    );
    app.world_mut()
        .write_message(bevy::window::WindowCloseRequested { window });
    app.update();
    let blocked_play = find(&mut app, false)?;
    app.world_mut().trigger(Activate {
        entity: blocked_play,
    });
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .selection
            .pending()
            .is_some(),
        "foreground quit confirmation fences direct Play activation"
    );
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: bevy::input::keyboard::Key::Escape,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        });
    app.update();
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .selection
            .pending()
            .is_some(),
        "quit Escape must not cancel the underlying selector callback"
    );
    let channel = find(&mut app, true)?;
    app.world_mut().trigger(Activate { entity: channel });
    app.update();
    let play = find(&mut app, false)?;
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(play));
    app.world_mut().trigger(Activate { entity: play });
    app.update();
    assert_eq!(
        worker.join().map_err(|_| "selector worker")?,
        SelectionAction::Play {
            character_id: "server-character".into(),
            channel_id: Some("server-channel".into())
        }
    );
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .selection
            .pending()
            .is_none()
    );
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    Ok(())
}
#[test]
fn scaled_picker_keeps_footer_inside_frame_and_selected_row_on_resize()
-> Result<(), Box<dyn std::error::Error>> {
    use bevy::{
        app::{HierarchyPropagatePlugin, PropagateSet},
        camera::{ComputedCameraValues, RenderTargetInfo},
        text::{FontCx, RemSize},
        ui::{
            ComputedUiRenderTargetInfo, ComputedUiTargetCamera, UiSystems, ui_layout_system,
            ui_surface::UiSurface, update::propagate_ui_target_cameras,
        },
    };
    // Feed source-declared single-line heights into Taffy without requiring
    // proprietary font assets or a platform font loader in this layout test.
    fn measure_source_lines(
        scale: Res<UiScale>,
        mut text: Query<(
            &bevy::text::LineHeight,
            &TextFont,
            &mut bevy::ui::ContentSize,
        )>,
    ) {
        for (height, font, mut content) in &mut text {
            let height = match (height, font.font_size) {
                (bevy::text::LineHeight::Px(height), _) => *height,
                (
                    bevy::text::LineHeight::RelativeToFont(factor),
                    bevy::text::FontSize::Px(size),
                ) => factor * size,
                _ => continue,
            };
            content.set(bevy::ui::NodeMeasure::Fixed(bevy::ui::FixedMeasure {
                size: Vec2::new(100. * scale.0, height * scale.0),
            }));
        }
    }
    let (bridge, callback) = SelectionBridge::new();
    let mut owner = HostOwner::new(None)?;
    owner.selection = bridge;
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        source::GlobalUiPlugin,
        HierarchyPropagatePlugin::<ComputedUiTargetCamera>::new(PostUpdate),
        HierarchyPropagatePlugin::<ComputedUiRenderTargetInfo>::new(PostUpdate),
    ))
    .init_asset::<Font>()
    .init_asset::<Image>()
    .init_resource::<InputFocus>()
    .init_resource::<UiScale>()
    .init_resource::<UiSurface>()
    .init_resource::<FontCx>()
    .init_resource::<RemSize>()
    .init_resource::<Assets<TextureAtlasLayout>>()
    .insert_non_send(owner)
    .configure_sets(
        PostUpdate,
        (
            UiSystems::Prepare,
            UiSystems::Propagate,
            UiSystems::Content,
            UiSystems::Layout,
        )
            .chain(),
    )
    .configure_sets(
        PostUpdate,
        PropagateSet::<ComputedUiTargetCamera>::default().in_set(UiSystems::Propagate),
    )
    .configure_sets(
        PostUpdate,
        PropagateSet::<ComputedUiRenderTargetInfo>::default().in_set(UiSystems::Propagate),
    )
    .add_systems(Startup, setup)
    .add_systems(Update, (poll, project).chain())
    .add_systems(
        PostUpdate,
        propagate_ui_target_cameras.in_set(UiSystems::Prepare),
    )
    .add_systems(
        PostUpdate,
        bevy::ui::widget::update_image_content_size_system.in_set(UiSystems::Content),
    )
    .add_systems(PostUpdate, measure_source_lines.in_set(UiSystems::Content))
    .add_systems(PostUpdate, ui_layout_system.in_set(UiSystems::Layout));
    app.world_mut().spawn(Camera2d);
    app.world_mut().spawn((
        Window {
            resolution: (1024, 720).into(),
            ..default()
        },
        bevy::window::PrimaryWindow,
    ));
    app.update();
    let worker = thread::spawn(move || {
        let rows = (0..8)
            .map(|index| crate::character_selection::CharacterRow {
                character: OwnerCharacter {
                    character_id: format!("server-{index}"),
                    world_id: "world".into(),
                    name: format!("Character {index}"),
                    availability: CharacterAvailability::Available,
                },
                world_name: "World".into(),
                channels: vec![("channel".into(), "Channel".into())],
                available: true,
            })
            .collect::<Vec<_>>();
        callback(&rows, "server-7", CancellationToken::new())
    });
    let deadline = Instant::now() + Duration::from_secs(3);
    while app
        .world()
        .non_send::<HostOwner>()
        .selection
        .pending()
        .is_none()
        && Instant::now() < deadline
    {
        app.world_mut().non_send_mut::<HostOwner>().selection.poll();
        thread::yield_now();
    }
    for (width, height, scale) in [
        (1024, 720, 2.),
        (800, 400, 2.),
        (1024, 720, 1.),
        (1024, 720, 2.),
    ] {
        app.world_mut().resource_mut::<UiScale>().0 = scale;
        for mut camera in app
            .world_mut()
            .query::<&mut Camera>()
            .iter_mut(app.world_mut())
        {
            camera.computed = ComputedCameraValues {
                target_info: Some(RenderTargetInfo {
                    physical_size: UVec2::new(width, height),
                    scale_factor: 1.,
                }),
                ..default()
            };
        }
        for mut window in app
            .world_mut()
            .query::<&mut Window>()
            .iter_mut(app.world_mut())
        {
            window.resolution.set_physical_resolution(width, height);
        }
        for _ in 0..3 {
            app.update();
        }
        let panel = app
            .world_mut()
            .query_filtered::<&ChildOf, With<source::DialogCaption>>()
            .iter(app.world())
            .next()
            .ok_or("caption")?
            .parent();
        let panel_node = app
            .world()
            .get::<ComputedNode>(panel)
            .ok_or("panel geometry")?;
        let panel_transform = app
            .world()
            .get::<UiGlobalTransform>(panel)
            .ok_or("panel position")?;
        let bottom = panel_transform.translation.y + panel_node.size().y / 2.;
        let controls = app
            .world_mut()
            .query::<(&PickerControl, &ComputedNode, &UiGlobalTransform)>()
            .iter(app.world())
            .map(|(id, node, transform)| {
                (
                    id.id.clone(),
                    transform.translation - node.size() / 2.,
                    node.size(),
                )
            })
            .collect::<Vec<_>>();
        for id in ["selection.play", "selection.cancel"] {
            let (_, min, size) = controls
                .iter()
                .find(|(name, _, _)| name == id)
                .ok_or("footer button")?;
            assert!(
                min.y >= 0. && min.y + size.y <= bottom && min.y + size.y <= height as f32,
                "{id} {}+{} outside panelbottom {bottom} /window {height} atscale {scale}",
                min.y,
                size.y
            );
            assert!(size.y > 0.);
        }
        assert_eq!(app.world().resource::<PickerState>().selected, Some(7));
        assert!(controls.iter().any(|(id, _, _)| id == "selection.row.7"));
    }
    let id = app
        .world()
        .non_send::<HostOwner>()
        .selection
        .pending()
        .ok_or("pending")?
        .id;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .selection
        .respond(id, SelectionAction::Cancel);
    assert_eq!(
        worker.join().map_err(|_| "selector worker")?,
        SelectionAction::Cancel
    );
    Ok(())
}
