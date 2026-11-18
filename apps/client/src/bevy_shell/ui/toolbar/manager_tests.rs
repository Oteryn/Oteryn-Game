use super::*;
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn click(app: &mut App, predicate: impl Fn(&Edit) -> bool) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Edit)>()
        .iter(app.world())
        .find(|(_, action)| predicate(action))
        .map(|(entity, _)| entity)
        .ok_or("missing editor button")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
#[test]
fn manage_buttons_persists_visibility_order_and_restores_defaults() -> TestResult {
    use bevy::{asset::AssetPlugin, input_focus::InputFocus};
    let fixture = crate::play::screenshot::TestDir::new()?;
    let folder = fixture.path().to_path_buf();
    let path = folder.join("preferences.json");
    let mut owner = HostOwner::new(Some(path.clone()))?;
    let (link, _commands, _outbox) = crate::play::play_channel();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<crate::bevy_shell::game_ui::hud_panes::SidebarWidgets>()
        .init_resource::<InputFocus>()
        .init_resource::<ButtonInput<KeyCode>>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup)
        .add_systems(Update, (project_toolbar, project_manager).chain());
    app.update();
    // While playing, the manager opens from the sidebar control buttons (grid.rs).
    app.world_mut().resource_mut::<Manager>().open = true;
    app.update();
    // The default set is the Tibia control-button set (ending with the reward wall); Market is
    // added and moved up and the battle list button is removed.
    assert_eq!(
        order(app.world().non_send::<HostOwner>()),
        super::super::grid::DEFAULT
    );
    click(&mut app, |action| {
        matches!(action, Edit::Toggle("shell.market"))
    })?;
    click(&mut app, |action| matches!(action, Edit::Toggle("battle")))?;
    click(&mut app, |action| {
        matches!(action, Edit::Move("shell.market", true))
    })?;
    for &(id, _, _) in super::super::grid::EXTRA {
        click(
            &mut app,
            |action| matches!(action, Edit::Toggle(entry) if *entry == id),
        )?;
    }
    assert!(
        !app.world_mut()
            .query::<&Edit>()
            .iter(app.world())
            .any(|e| matches!(e, Edit::Toggle("shell.options" | "store")))
    );
    let expected = order(app.world().non_send::<HostOwner>());
    assert!(!expected.iter().any(|id| id == "battle"));
    assert_eq!(
        expected.iter().rev().take(2).collect::<Vec<_>>(),
        ["rewards", "shell.market"]
    );
    let restored = HostOwner::new(Some(path))?;
    assert_eq!(
        restored.settings.preferences.toolbar_order.as_ref(),
        Some(&expected)
    );
    // While playing, the text buttons stay hidden and the sidebar grid follows the editor.
    assert!(
        app.world_mut()
            .query::<(&Control, &Node)>()
            .iter(app.world())
            .filter(|(control, _)| TOOLBAR.iter().any(|entry| entry.1.id() == control.id()))
            .all(|(_, node)| node.display == Display::None)
    );
    let region = app.world_mut().spawn(Node::default()).id();
    app.world_mut()
        .resource_scope(|world, skins: Mut<crate::bevy_shell::ui::Skins>| {
            super::super::grid::spawn(&mut world.commands(), &skins, region);
        });
    app.world_mut().flush();
    bevy::ecs::system::RunSystemOnce::run_system_once(app.world_mut(), super::super::grid::project)
        .map_err(|error| error.to_string())?;
    app.world_mut().flush();
    let grid = app
        .world_mut()
        .query_filtered::<&Children, With<super::super::grid::ControlGrid>>()
        .single(app.world())?
        .iter()
        .collect::<Vec<_>>();
    let shown: Vec<_> = grid
        .into_iter()
        .filter_map(|entity| {
            let id = match *app
                .world()
                .get::<super::super::grid::ControlButton>(entity)?
            {
                super::super::grid::ControlButton::Grid(control) => control.id(),
                super::super::grid::ControlButton::Extra(extra) => extra.id(),
                _ => return None,
            };
            let node = app.world().get::<Node>(entity)?;
            (node.display != Display::None).then_some(id.to_owned())
        })
        .collect();
    assert_eq!(shown, expected);
    click(&mut app, |action| matches!(action, Edit::Reset))?;
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .settings
            .preferences
            .toolbar_order
            .is_none()
    );
    click(&mut app, |action| matches!(action, Edit::Close))?;
    assert!(!app.world().resource::<Manager>().open);
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let mut preferences = owner.settings.preferences.clone();
        preferences.toolbar_order = Some(Vec::new());
        // Global gives the Market no chord; a player-bound one stays on the input path.
        preferences.rebind("0063-market-open", "CTRL+SHIFT+F9")?;
        owner.settings.apply(preferences);
    }
    app.update();
    // An empty set hides every grid button; the manager stays in the fixed column.
    for (control, node) in app
        .world_mut()
        .query::<(&Control, &Node)>()
        .iter(app.world())
    {
        if TOOLBAR.iter().any(|entry| entry.1.id() == control.id()) {
            assert_eq!(node.display, Display::None);
        }
    }
    // Hiding the Market button leaves the player's Market chord on the production input path.
    use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};
    let event = NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(66)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
        repeat: false,
    };
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    owner.route_input(&[event], false);
    assert!(owner.play.as_ref().ok_or("play")?.link.market_panel().open);
    for (hidden, moving, up, expected) in [
        ("store", "vip", true, ["vip", "battle"]),
        ("shell.options", "battle", false, ["vip", "battle"]),
    ] {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let mut preferences = owner.settings.preferences.clone();
        preferences.toolbar_order = Some(["battle", hidden, "vip"].map(str::to_owned).into());
        owner.settings.apply(preferences);
        app.world_mut().resource_mut::<Manager>().open = true;
        app.update();
        click(
            &mut app,
            |action| matches!(action, Edit::Move(id, direction) if *id == moving && *direction == up),
        )?;
        assert_eq!(order(app.world().non_send::<HostOwner>()), expected);
    }
    Ok(())
}
