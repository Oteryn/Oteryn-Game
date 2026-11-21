use super::*;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
);
fn fixture() -> std::result::Result<Fixture, Box<dyn std::error::Error>> {
    let (link, receiver, outbox) = crate::play::play_channel();
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 0, &[])?,
        link,
    });
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<InputFocus>()
        .init_resource::<super::super::guide::State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .insert_non_send(owner)
        .add_systems(Startup, (crate::bevy_shell::ui::setup, setup).chain())
        .add_systems(Update, (keyboard, project).chain());
    super::super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    for (sequence, health) in [(1, 0), (2, 100)] {
        outbox.push(vec![oteryn_session::SessionEvent::ActorVitals(
            oteryn_session::AppliedDelta {
                server_sequence: sequence,
                base_revision: sequence - 1,
                new_revision: sequence,
                value: oteryn_session::ActorVitals {
                    health,
                    max_health: 100,
                    ..default()
                },
            },
        )]);
        {
            let mut owner = app.world_mut().non_send_mut::<HostOwner>();
            let play = owner.play.as_mut().ok_or("play")?;
            play.view.tick(&play.link).map_err(|e| format!("{e:?}"))?;
        }
        app.update();
    }
    Ok((app, receiver))
}
fn tick(app: &mut App) -> Result {
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    let play = owner.play.as_mut().ok_or("play")?;
    play.view.tick(&play.link).map_err(|e| format!("{e:?}"))?;
    Ok(())
}
#[test]
fn death_continue_and_leave_are_mouse_accessible() -> Result {
    let (mut app, mut receiver) = fixture()?;
    let mut buttons = Vec::new();
    for caption in ["Continue", "Leave"] {
        let label = app
            .world_mut()
            .query::<(Entity, &Text)>()
            .iter(app.world())
            .find_map(|(e, t)| (t.0 == caption).then_some(e))
            .ok_or("death button missing")?;
        buttons.push(app.world().get::<ChildOf>(label).ok_or("parent")?.parent());
    }
    app.world_mut().trigger(Activate { entity: buttons[0] });
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .view
            .death_modal_notice()
            .is_none()
    );
    // Replaying the old Leave control after acknowledgement must do nothing.
    app.world_mut().trigger(Activate { entity: buttons[1] });
    tick(&mut app)?;
    assert!(receiver.try_recv().is_err());
    Ok(())
}

#[test]
fn death_leave_and_escape_use_safe_logout_and_keypad_enter_acknowledges() -> Result {
    for key in [None, Some(KeyCode::Escape), Some(KeyCode::NumpadEnter)] {
        let (mut app, mut receiver) = fixture()?;
        if let Some(key_code) = key {
            app.world_mut()
                .write_message(bevy::input::keyboard::KeyboardInput {
                    key_code,
                    logical_key: bevy::input::keyboard::Key::Unidentified(
                        bevy::input::keyboard::NativeKey::Unidentified,
                    ),
                    state: bevy::input::ButtonState::Pressed,
                    text: None,
                    repeat: false,
                    window: Entity::PLACEHOLDER,
                });
        } else {
            let label = app
                .world_mut()
                .query::<(Entity, &Text)>()
                .iter(app.world())
                .find_map(|(e, t)| (t.0 == "Leave").then_some(e))
                .ok_or("Leave")?;
            let entity = app.world().get::<ChildOf>(label).ok_or("button")?.parent();
            app.world_mut().trigger(Activate { entity });
        }
        app.update();
        tick(&mut app)?;
        if key == Some(KeyCode::NumpadEnter) {
            assert!(receiver.try_recv().is_err());
            assert!(
                app.world()
                    .non_send::<HostOwner>()
                    .play
                    .as_ref()
                    .ok_or("play")?
                    .view
                    .death_modal_notice()
                    .is_none()
            );
        } else {
            assert!(matches!(
                receiver.try_recv()?,
                crate::combat_input::PlayCommand::Logout
            ));
        }
    }
    Ok(())
}
