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
    let folder = std::env::temp_dir().join(format!("gap23-toolbar-{}", std::process::id()));
    std::fs::create_dir_all(&folder)?;
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
        .init_resource::<InputFocus>()
        .init_resource::<ButtonInput<KeyCode>>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup)
        .add_systems(Update, (project_toolbar, project_manager).chain());
    app.update();
    let manage = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find(|(_, control)| matches!(control, Control::ManageButtons))
        .map(|(entity, _)| entity)
        .ok_or("missing Manage Buttons")?;
    app.world_mut().trigger(Activate { entity: manage });
    app.update();
    click(&mut app, |action| {
        matches!(action, Edit::Toggle("shell.market"))
    })?;
    click(&mut app, |action| {
        matches!(action, Edit::Move("store", true))
    })?;
    let expected = order(app.world().non_send::<HostOwner>());
    assert!(!expected.iter().any(|id| id == "shell.market"));
    let restored = HostOwner::new(Some(path))?;
    assert_eq!(
        restored.settings.preferences.toolbar_order.as_ref(),
        Some(&expected)
    );
    let actual: Vec<_> = app
        .world_mut()
        .query::<(&Control, &Node)>()
        .iter(app.world())
        .filter(|(control, node)| control.id() == "shell.market" && node.display == Display::None)
        .collect();
    assert_eq!(actual.len(), 1);
    // The actual child order, rather than just the preference vector, follows the editor.
    let parent = app
        .world()
        .get::<ChildOf>(manage)
        .ok_or("toolbar parent")?
        .parent();
    let children = app
        .world()
        .get::<Children>(parent)
        .ok_or("toolbar children")?;
    let shown: Vec<_> = children
        .iter()
        .filter_map(|entity| {
            let control = app.world().get::<Control>(entity)?;
            let node = app.world().get::<Node>(entity)?;
            (node.display != Display::None && !matches!(control, Control::ManageButtons))
                .then_some(control.id().to_owned())
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
        owner.settings.apply(preferences);
    }
    app.update();
    assert_eq!(
        app.world()
            .get::<Node>(manage)
            .ok_or("manage node")?
            .display,
        Display::Flex
    );
    for (control, node) in app
        .world_mut()
        .query::<(&Control, &Node)>()
        .iter(app.world())
    {
        if TOOLBAR.iter().any(|entry| entry.1.id() == control.id()) {
            assert_eq!(node.display, Display::None);
        }
    }
    // Hiding the Market button leaves Ctrl+M on the production input path.
    use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};
    let event = NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(16)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control),
        repeat: false,
    };
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    owner.route_input(&[event], false);
    assert!(owner.play.as_ref().ok_or("play")?.link.market_panel().open);
    std::fs::remove_dir_all(folder)?;
    Ok(())
}
