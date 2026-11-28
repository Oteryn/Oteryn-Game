// Included by the Bevy furniture test module. The live row supplies its
// received map window and forwards only the command emitted by the menu.
#[test]
#[ignore = "requires a live I09 map window"]
fn qualification_i09_rotate_menu() -> Result {
    let args = std::env::var("OTERYN_I09_CLICK")?
        .split_whitespace()
        .map(str::parse::<i32>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let [actor_x, actor_y, floor, click_x, click_y] = args.as_slice() else {
        return Err("I09 menu requires actor x/y/floor and click x/y".into());
    };
    let snapshot = std::fs::read(std::env::var_os("OTERYN_I09_MENU_MAP").ok_or("no live map")?)?;
    let map = oteryn_session::world_map::decode_world_map_snapshot(&snapshot)
        .map_err(|error| format!("I09 menu map: {error:?}"))?;
    let mut view = PlayView::new(
        TileCoord::new(*actor_x, *actor_y),
        (*floor).try_into()?,
        &[],
    )?;
    view.set_live_map(&map)?;
    let (mut app, commands, outbox) = crate::bevy_shell::input::hits::qualification_right_click(
        view,
        TileCoord::new(*click_x, *click_y),
    )?;
    install_test_dialogs(&mut app);
    app.update();
    click_rotate(&mut app)?;
    let dispatched = std::sync::Arc::new(std::sync::Mutex::new(None));
    let peer = I09NativeTransport(std::sync::Arc::clone(&dispatched));
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            tokio::time::timeout(
                std::time::Duration::from_secs(3),
                crate::play::run_session(peer, commands, outbox),
            )
            .await
        })?;
    let Command {
        target: Target::Object(object),
        action: Action::Use,
        selection,
    } = dispatched
        .lock()
        .map_err(|_| "I09 native transport lock")?
        .take()
        .ok_or("I09 native session loop did not dispatch Rotate")?
    else {
        return Err("I09 Rotate did not emit a world-object USE".into());
    };
    let selection = selection.map(|value| {
        let pos = value.position;
        ((pos.x, pos.y, pos.floor), value.binding)
    });
    let command = (object.placement, object.expected_revision, selection);
    println!("I09_MENU_USE={}", serde_json::to_string(&command)?);
    Ok(())
}

fn click_rotate(app: &mut App) -> Result {
    use bevy::{
        camera::RenderTarget,
        picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput},
        ui::UiGlobalTransform,
        window::PrimaryWindow,
    };
    // Run layout and visibility before locating the actual button geometry.
    app.update();
    for mut visibility in app
        .world_mut()
        .query::<&mut InheritedVisibility>()
        .iter_mut(app.world_mut())
    {
        *visibility = InheritedVisibility::VISIBLE;
    }
    let button = choice_entity(app, "Rotate").ok_or("missing choice: Rotate")?;
    let position = app
        .world()
        .get::<UiGlobalTransform>(button)
        .ok_or("Rotate geometry")?
        .translation;
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())?;
    let location = Location {
        target: RenderTarget::default()
            .normalize(Some(window))
            .ok_or("I09 window target")?,
        position,
    };
    for action in [
        PointerAction::Move { delta: Vec2::ZERO },
        PointerAction::Press(PointerButton::Primary),
        PointerAction::Release(PointerButton::Primary),
    ] {
        app.world_mut().write_message(PointerInput::new(
            PointerId::Mouse,
            location.clone(),
            action,
        ));
        app.update();
    }
    Ok(())
}

// The production session loop owns the Bevy channel. This transport hands
// off only the command it dispatches and returns no simulated server result.
struct I09NativeTransport(std::sync::Arc<std::sync::Mutex<Option<Command>>>);

impl crate::play::Stepper for I09NativeTransport {
    async fn world_context(
        &mut self,
        command: &Command,
        _raw_wire: bool,
    ) -> std::result::Result<String, oteryn_session::SessionError> {
        let mut dispatched = self
            .0
            .lock()
            .map_err(|_| oteryn_session::SessionError::SessionUnusable)?;
        assert!(dispatched.replace(command.clone()).is_none());
        Err(oteryn_session::SessionError::SessionUnusable)
    }

    async fn step(
        &mut self,
        _: oteryn_session::StepDirection,
    ) -> std::result::Result<oteryn_session::StepOutcome, oteryn_session::SessionError> {
        Err(oteryn_session::SessionError::SessionUnusable)
    }

    async fn serve(
        &mut self,
        duration: std::time::Duration,
    ) -> std::result::Result<(), oteryn_session::SessionError> {
        tokio::time::sleep(duration).await;
        Ok(())
    }

    fn drain(&mut self) -> Vec<oteryn_session::SessionEvent> {
        Vec::new()
    }

    async fn combat(
        &mut self,
        _: crate::combat_input::CombatCommand,
    ) -> std::result::Result<crate::combat_input::CombatReport, oteryn_session::SessionError> {
        Err(oteryn_session::SessionError::SessionUnusable)
    }

    async fn logout(
        &mut self,
    ) -> std::result::Result<Option<oteryn_session::LogoutResult>, oteryn_session::SessionError>
    {
        Err(oteryn_session::SessionError::SessionUnusable)
    }
}
