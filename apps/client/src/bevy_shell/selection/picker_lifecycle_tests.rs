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

#[test]
fn picker_uses_the_bundled_font_in_installed_client()
-> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .insert_resource(crate::bevy_shell::BuiltInFont)
        .add_systems(Startup, setup);
    app.update();
    assert_eq!(app.world().resource::<Skin>().font, Handle::default());
    Ok(())
}
