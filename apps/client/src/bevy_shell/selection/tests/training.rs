use super::*;

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
                channels: vec![crate::character_selection::ChannelChoice {
                    id: "server-channel".into(),
                    name: "Server channel".into(),
                    pvp_type: oteryn_platform_contracts::DirectoryPvpType::OptionalPvp,
                    region: "EU".into(),
                }],
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
            .map(|channel| {
                vec![crate::character_selection::ChannelChoice {
                    id: channel.into(),
                    name: "Server channel".into(),
                    pvp_type: oteryn_platform_contracts::DirectoryPvpType::OptionalPvp,
                    region: "EU".into(),
                }]
            })
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
