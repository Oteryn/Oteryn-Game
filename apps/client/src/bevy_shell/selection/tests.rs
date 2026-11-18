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
    app.add_plugins(MinimalPlugins);
    crate::bevy_shell::ui::window_layout::test_ui_layout(&mut app, UVec2::new(1024, 720));
    app.add_plugins(bevy::ui_widgets::ScrollAreaPlugin)
        .insert_non_send(owner)
        .add_systems(Startup, setup)
        .add_systems(Update, (poll, project).chain());
    app.add_systems(Startup, crate::bevy_shell::ui::setup)
        .add_systems(Update, keyboard.before(project))
        .add_systems(
            PostUpdate,
            reveal_selection.after(bevy::ui::UiSystems::PostLayout),
        );
    crate::bevy_shell::game_ui::modal::install(&mut app);
    crate::bevy_shell::exit::install(&mut app);
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: (1024, 720).into(),
                ..default()
            },
            bevy::window::PrimaryWindow,
        ))
        .id();
    app.update();
    // Keep the headless camera from the layout fixture as the sole UI camera.
    let shell_camera = app
        .world_mut()
        .query::<(Entity, &Camera)>()
        .iter(app.world())
        .find_map(|(entity, camera)| (camera.order == 1).then_some(entity))
        .ok_or("shell camera")?;
    app.world_mut().despawn(shell_camera);
    let account_callback = callback.clone();
    let worker = thread::spawn(move || {
        let row = crate::character_selection::CharacterRow {
            character: OwnerCharacter {
                character_id: "server-character".into(),
                world_id: "server-world".into(),
                name: "Server character".into(),
                availability: CharacterAvailability::Available,
            },
            world_name: "Server world".into(),
            channels: vec![("server-channel".into(), "Server channel".into())],
            available: true,
        };
        let mut rows = vec![row; 20];
        for (index, row) in rows.iter_mut().enumerate().take(19) {
            row.character.character_id = format!("character-{index}");
        }
        callback(&rows, "character-0", CancellationToken::new())
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
    assert!(
        app.world_mut()
            .query::<&PickerControl>()
            .iter(app.world())
            .any(|control| control.id == "selection.account"),
        "account services must be reachable from the picker"
    );
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
    let list = |app: &mut App| -> Result<Entity, &'static str> {
        app.world_mut()
            .query_filtered::<(Entity, &Name), With<bevy::ui_widgets::ScrollArea>>()
            .iter(app.world())
            .find(|(_, name)| name.as_str() == "selection.list")
            .map(|(entity, _)| entity)
            .ok_or("character list")
    };
    let scroll = |app: &mut App| -> Result<f32, &'static str> {
        let entity = list(app)?;
        Ok(app
            .world()
            .get::<ScrollPosition>(entity)
            .ok_or("scroll")?
            .0
            .y)
    };
    use bevy::{
        camera::RenderTarget,
        picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput},
    };
    let center = |app: &App, entity| -> Result<Vec2, &'static str> {
        Ok(app
            .world()
            .get::<UiGlobalTransform>(entity)
            .ok_or("UI transform")?
            .translation)
    };
    let size = |app: &App, entity| -> Result<Vec2, &'static str> {
        Ok(app
            .world()
            .get::<ComputedNode>(entity)
            .ok_or("UI geometry")?
            .size())
    };
    let row = |app: &mut App, index| -> Result<Entity, &'static str> {
        app.world_mut()
            .query::<(Entity, &Target)>()
            .iter(app.world())
            .find_map(|(entity, target)| {
                matches!(target.choice, Choice::Row(row) if row == index).then_some(entity)
            })
            .ok_or("row")
    };
    let visible = |app: &mut App, index| -> Result<Vec2, Box<dyn std::error::Error>> {
        let list = list(app)?;
        let row = row(app, index)?;
        let center_y = center(app, list)?.y;
        let half_height = size(app, list)?.y / 2.;
        let position = center(app, row)?;
        let row_half = size(app, row)?.y / 2.;
        assert!(position.y - row_half >= center_y - half_height - 1.);
        assert!(position.y + row_half <= center_y + half_height + 1.);
        Ok(position)
    };
    let send = |app: &mut App, position, action| -> Result<(), &'static str> {
        let target = RenderTarget::default()
            .normalize(Some(window))
            .ok_or("primary target")?;
        app.world_mut().write_message(PointerInput::new(
            PointerId::Mouse,
            Location { target, position },
            action,
        ));
        app.update();
        Ok(())
    };
    let entity = list(&mut app)?;
    let position = center(&app, entity)?;
    send(
        &mut app,
        position,
        PointerAction::Move { delta: Vec2::ZERO },
    )?;
    send(
        &mut app,
        position,
        PointerAction::Scroll {
            unit: bevy::input::mouse::MouseScrollUnit::Pixel,
            x: 0.,
            y: -1000.,
            phase: bevy::input::touch::TouchPhase::Moved,
        },
    )?;
    let offset = scroll(&mut app)?;
    assert!(offset > 0.);
    let last = visible(&mut app, 19)?;
    for action in [
        PointerAction::Move {
            delta: last - position,
        },
        PointerAction::Press(PointerButton::Primary),
        PointerAction::Release(PointerButton::Primary),
    ] {
        send(&mut app, last, action)?;
    }
    assert_eq!(app.world().resource::<PickerState>().selected, Some(19));
    assert_eq!(scroll(&mut app)?, offset);
    let channel = find(&mut app, true)?;
    app.world_mut().trigger(Activate { entity: channel });
    app.update();
    assert_eq!(scroll(&mut app)?, offset);
    let toggle = app
        .world_mut()
        .query_filtered::<Entity, With<OutfitToggle>>()
        .single(app.world())?;
    app.world_mut()
        .trigger(bevy::ui_widgets::ValueChange::<bool> {
            source: toggle,
            value: true,
            is_final: true,
        });
    app.update();
    app.update();
    visible(&mut app, 19)?;
    for (key, indices) in [
        (KeyCode::ArrowUp, (11..19).rev().collect::<Vec<_>>()),
        (KeyCode::ArrowDown, (12..20).collect()),
    ] {
        for index in indices {
            app.world_mut()
                .write_message(bevy::input::keyboard::KeyboardInput {
                    key_code: key,
                    logical_key: bevy::input::keyboard::Key::Unidentified(
                        bevy::input::keyboard::NativeKey::Unidentified,
                    ),
                    state: bevy::input::ButtonState::Pressed,
                    text: None,
                    repeat: false,
                    window,
                });
            app.update();
            app.update();
            assert_eq!(app.world().resource::<PickerState>().selected, Some(index));
            visible(&mut app, index)?;
        }
    }
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
    let account_worker = thread::spawn(move || account_callback(&[], "", CancellationToken::new()));
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
    let account = app
        .world_mut()
        .query::<(Entity, &PickerControl)>()
        .iter(app.world())
        .find(|(_, c)| c.id == "selection.account")
        .map(|(e, _)| e)
        .ok_or("Account Services button")?;
    app.world_mut().trigger(Activate { entity: account });
    assert_eq!(
        account_worker
            .join()
            .map_err(|_| "account selector worker")?,
        SelectionAction::AccountServices
    );
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

#[test]
fn picker_arrow_navigation_and_double_click_use_current_request()
-> Result<(), Box<dyn std::error::Error>> {
    use bevy::picking::{
        backend::HitData,
        events::Pointer,
        pointer::{Location, PointerButton, PointerId},
    };
    let (bridge, callback) = SelectionBridge::new();
    let mut owner = HostOwner::new(None)?;
    owner.selection = bridge;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<InputFocus>()
        .insert_non_send(owner)
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Startup, setup)
        .add_systems(Update, (poll, project, keyboard).chain());
    app.update();
    let worker = thread::spawn(move || {
        let rows = (0..3)
            .map(|index| crate::character_selection::CharacterRow {
                character: OwnerCharacter {
                    character_id: format!("character{index}"),
                    world_id: "world".into(),
                    name: format!("Player {index}"),
                    availability: CharacterAvailability::Available,
                },
                world_name: "World".into(),
                channels: vec![],
                available: true,
            })
            .collect::<Vec<_>>();
        callback(&rows, "character0", CancellationToken::new())
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
    for (code, selected) in [
        (KeyCode::ArrowDown, 1),
        (KeyCode::ArrowUp, 0),
        (KeyCode::ArrowDown, 1),
    ] {
        app.world_mut()
            .write_message(bevy::input::keyboard::KeyboardInput {
                key_code: code,
                logical_key: bevy::input::keyboard::Key::Unidentified(
                    bevy::input::keyboard::NativeKey::Unidentified,
                ),
                state: bevy::input::ButtonState::Pressed,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        app.update();
        assert_eq!(
            app.world().resource::<PickerState>().selected,
            Some(selected)
        );
    }
    app.update();
    for _ in 0..2 {
        let entity = app
            .world_mut()
            .query::<(Entity, &Target)>()
            .iter(app.world())
            .find_map(|(entity, target)| matches!(target.choice, Choice::Row(1)).then_some(entity))
            .ok_or("second row")?;
        app.world_mut().trigger(Activate { entity });
        app.world_mut().trigger(PointerClick {
            entity,
            pointer: Pointer::new(
                PointerId::Mouse,
                Location {
                    target: bevy::camera::NormalizedRenderTarget::None {
                        width: 100,
                        height: 100,
                    },
                    position: Vec2::ZERO,
                },
            ),
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
            duration: Duration::ZERO,
            count: 1,
        });
        app.update();
    }
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .selection
            .pending()
            .is_none()
    );
    assert_eq!(
        worker.join().map_err(|_| "selector worker")?,
        SelectionAction::Play {
            character_id: "character1".into(),
            channel_id: None
        }
    );
    Ok(())
}

#[test]
fn picker_training_returns_server_channel_through_existing_callback()
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
                    matches!(target.choice, Choice::Train)
                }
            })
            .map(|(entity, _)| entity)
            .ok_or("picker button missing")
    };
    let initial_play = find(&mut app, false)?;
    let _ = initial_play;
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

    app.world_mut().trigger(Activate { entity: play });
    app.update();
    assert_eq!(
        worker.join().map_err(|_| "selector worker")?,
        SelectionAction::Train {
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
fn picker_marks_several_characters_and_trains_them_through_existing_callback()
-> Result<(), Box<dyn std::error::Error>> {
    let (bridge, callback) = SelectionBridge::new();
    let mut owner = HostOwner::new(None)?;
    owner.selection = bridge;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<InputFocus>()
        .insert_non_send(owner)
        .add_systems(Startup, setup)
        .add_systems(Update, (poll, project).chain());
    app.add_systems(Startup, crate::bevy_shell::ui::setup);
    crate::bevy_shell::game_ui::modal::install(&mut app);
    app.update();
    let row = |id: &str, channel: Option<&str>| crate::character_selection::CharacterRow {
        character: OwnerCharacter {
            character_id: id.into(),
            world_id: "server-world".into(),
            name: format!("Server {id}"),
            availability: CharacterAvailability::Available,
        },
        world_name: "Server world".into(),
        channels: channel
            .map(|channel| vec![(channel.into(), "Server channel".into())])
            .unwrap_or_default(),
        available: true,
    };
    let rows = vec![row("first", Some("first-channel")), row("second", None)];
    let worker = thread::spawn(move || callback(&rows, "first", CancellationToken::new()));
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
    let press = |app: &mut App, wanted: &dyn Fn(&Choice) -> bool| -> Result<(), &'static str> {
        let entity = app
            .world_mut()
            .query::<(Entity, &Target)>()
            .iter(app.world())
            .find(|(_, target)| wanted(&target.choice))
            .map(|(entity, _)| entity)
            .ok_or("picker button missing")?;
        app.world_mut().trigger(Activate { entity });
        app.update();
        Ok(())
    };
    // An empty batch cannot be sent.
    press(&mut app, &|choice| matches!(choice, Choice::TrainMarked))?;
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .selection
            .pending()
            .is_some()
    );
    press(&mut app, &|choice| matches!(choice, Choice::Channel))?;
    press(&mut app, &|choice| matches!(choice, Choice::Mark))?;
    // Every row stays in the scrolling list; the second row is reachable without paging.
    press(&mut app, &|choice| matches!(choice, Choice::Row(1)))?;
    press(&mut app, &|choice| matches!(choice, Choice::Mark))?;
    press(&mut app, &|choice| matches!(choice, Choice::TrainMarked))?;
    assert_eq!(
        worker.join().map_err(|_| "selector worker")?,
        SelectionAction::TrainMany {
            characters: vec![
                crate::character_selection::TrainingSelection {
                    character_id: "first".into(),
                    channel_id: Some("first-channel".into()),
                },
                crate::character_selection::TrainingSelection {
                    character_id: "second".into(),
                    channel_id: None,
                },
            ]
        }
    );
    Ok(())
}

#[test]
fn picker_is_the_global_character_list_with_columns_scroll_and_outfits()
-> Result<(), Box<dyn std::error::Error>> {
    let (bridge, callback) = SelectionBridge::new();
    let mut owner = HostOwner::new(None)?;
    owner.selection = bridge;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<InputFocus>()
        .insert_non_send(owner)
        .add_systems(Startup, setup)
        .add_systems(Update, (poll, project).chain());
    crate::bevy_shell::game_ui::modal::install(&mut app);
    app.world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow));
    app.update();
    let worker = thread::spawn(move || {
        let rows = (0..12)
            .map(|index| crate::character_selection::CharacterRow {
                character: OwnerCharacter {
                    character_id: format!("server-{index}"),
                    world_id: "world".into(),
                    name: format!("Character {index}"),
                    availability: CharacterAvailability::Available,
                },
                world_name: "Antica".into(),
                channels: Vec::new(),
                available: true,
            })
            .collect::<Vec<_>>();
        callback(&rows, "server-0", CancellationToken::new())
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
    app.update();
    let node = |app: &mut App, name: &str| {
        app.world_mut()
            .query::<(Entity, &Name, &Node)>()
            .iter(app.world())
            .find_map(|(entity, candidate, node)| {
                (candidate.as_str() == name).then(|| (entity, node.clone()))
            })
            .ok_or(format!("{name} absent"))
    };
    assert_eq!(node(&mut app, "selection.window")?.1.width, px(745.));
    let (list, list_node) = node(&mut app, "selection.list")?;
    assert_eq!(list_node.height, px(340.));
    assert_eq!(list_node.overflow, Overflow::scroll_y());
    let headings = |app: &mut App| -> Result<Vec<String>, String> {
        let (columns, _) = node(app, "selection.columns")?;
        let world = app.world();
        Ok(world
            .get::<Children>(columns)
            .into_iter()
            .flat_map(|children| children.iter())
            .filter_map(|cell| world.get::<Text>(cell).map(|text| text.0.clone()))
            .collect())
    };
    assert_eq!(
        headings(&mut app)?,
        ["Character", "Status", "Level", "Vocation", "World"]
    );
    // All twelve rows are in the list (no paging), each 29px high.
    let rows = app
        .world()
        .get::<Children>(list)
        .ok_or("rows")?
        .iter()
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 12);
    assert!(rows.iter().all(|row| {
        app.world()
            .get::<Node>(*row)
            .is_some_and(|node| node.height == px(29.))
    }));
    let labels: Vec<String> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect();
    for label in [
        "Create Character",
        "Manage Account",
        "Refresh",
        "Offline Training",
        "Ok",
        "Cancel",
        "Show Outfits",
    ] {
        assert!(labels.iter().any(|text| text == label), "{label}");
    }
    assert!(
        !labels
            .iter()
            .any(|text| text == "Play" || text.starts_with("Page "))
    );
    let toggle = app
        .world_mut()
        .query::<(Entity, &PickerControl)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.id == "selection.show_outfits").then_some(entity))
        .ok_or("show outfits")?;
    app.world_mut()
        .trigger(bevy::ui_widgets::ValueChange::<bool> {
            source: toggle,
            value: true,
            is_final: true,
        });
    app.update();
    assert_eq!(
        headings(&mut app)?,
        [
            "Outfit",
            "Character",
            "Status",
            "Level",
            "Vocation",
            "World"
        ]
    );
    let (list, _) = node(&mut app, "selection.list")?;
    let first = app
        .world()
        .get::<Children>(list)
        .and_then(|children| children.iter().next())
        .ok_or("row")?;
    assert_eq!(app.world().get::<Node>(first).ok_or("row")?.height, px(66.));
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
