use super::*;
use crate::bevy_shell::owner::{ActivePlay, Phase};
use crate::play::{PlayView, play_channel};
use bevy::{asset::AssetPlugin, input_focus::InputFocus, ui_widgets::Activate};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn fixture() -> Result<App, Box<dyn std::error::Error>> {
    let (link, _commands, _outbox) = play_channel();
    link.use_shared_frontend();
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(ActivePlay { view, link });
    owner.phase = Phase::Playing;
    open(&mut owner);
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<InputFocus>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok(app)
}
fn click(app: &mut App, kind: Kind) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == kind).then_some(entity))
        .ok_or("control")?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
fn field(app: &mut App, index: usize, value: &str) {
    let world = app.world_mut();
    for (field, mut text) in world.query::<(&Field, &mut EditableText)>().iter_mut(world) {
        if field.0 == index {
            *text = EditableText::new(value);
        }
    }
}
#[test]
fn input_profiles_window_shows_owner_data_only() -> TestResult {
    let mut app = fixture()?;
    let owner = app.world().non_send::<HostOwner>();
    let expected = owner.settings.preferences.clone();
    assert_eq!(
        app.world()
            .resource::<State>()
            .snapshot
            .as_ref()
            .ok_or("snapshot")?
            .preferences,
        expected
    );
    let labels: Vec<_> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.clone())
        .collect();
    for name in expected.hotkey_contexts.presets.keys() {
        assert!(labels.contains(name));
    }
    for binding in bindings() {
        assert!(labels.contains(&binding_label(&binding)));
    }
    for (_, label) in crate::play::SHORTCUTS {
        assert!(labels.iter().any(|text| text.contains(label)));
    }
    assert!(labels.contains(&"Chat off".into()));
    assert!(labels.contains(&"Chat on".into()));
    Ok(())
}
#[test]
fn input_profiles_actions_are_bound_and_confirmed() -> TestResult {
    let mut app = fixture()?;
    field(&mut app, 0, "Raid");
    click(&mut app, Kind::Copy)?;
    app.update();
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .settings
            .preferences
            .hotkey_contexts
            .selected,
        "Raid"
    );
    click(&mut app, Kind::Delete)?;
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .settings
            .preferences
            .hotkey_contexts
            .presets
            .contains_key("Raid")
    );
    app.update();
    field(&mut app, 0, "changed draft");
    click(&mut app, Kind::ConfirmDelete)?;
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .settings
            .preferences
            .hotkey_contexts
            .presets
            .contains_key("Raid")
    );
    app.update();
    click(&mut app, Kind::Delete)?;
    app.update();
    click(&mut app, Kind::ConfirmDelete)?;
    app.update();
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .settings
            .preferences
            .hotkey_contexts
            .presets
            .contains_key("Raid")
    );
    click(&mut app, Kind::Context(true))?;
    app.update();
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .settings
            .preferences
            .hotkey_contexts
            .chat_on
    );
    click(&mut app, Kind::ShortcutAdd("market".into()))?;
    app.update();
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .settings
            .preferences
            .shortcuts
            .contains(&"market".into())
    );
    Ok(())
}
#[test]
fn input_profiles_rejected_when_session_replaced_or_target_changed() -> TestResult {
    let mut app = fixture()?;
    let control = app
        .world_mut()
        .query::<&Control>()
        .iter(app.world())
        .find(|c| c.kind == Kind::Copy)
        .cloned()
        .ok_or("copy")?;
    let before = app
        .world()
        .non_send::<HostOwner>()
        .settings
        .preferences
        .clone();
    let modal = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(999), Node::default()))
        .id();
    field(&mut app, 0, "Rejected");
    click(&mut app, Kind::Copy)?;
    assert_eq!(
        app.world().non_send::<HostOwner>().settings.preferences,
        before
    );
    app.world_mut().entity_mut(modal).despawn();
    app.update();
    field(&mut app, 0, "Accepted");
    click(&mut app, Kind::Copy)?;
    click(&mut app, Kind::Copy)?;
    app.update();
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .settings
            .preferences
            .hotkey_contexts
            .presets
            .len(),
        2
    );
    let mut state = app.world_mut().remove_resource::<State>().ok_or("state")?;
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    let accepted = owner.settings.preferences.clone();
    perform(
        &mut owner,
        &mut state,
        &control,
        &["Stale".into(), String::new(), String::new()],
        true,
    );
    assert_eq!(owner.settings.preferences, accepted);
    owner.close();
    perform(
        &mut owner,
        &mut state,
        &control,
        &["Replaced".into(), String::new(), String::new()],
        true,
    );
    assert!(
        !owner
            .settings
            .preferences
            .hotkey_contexts
            .presets
            .contains_key("Replaced")
    );
    Ok(())
}
#[test]
fn input_profiles_window_registered() -> TestResult {
    assert!(include_str!("../mod.rs").contains("input_profiles::install(app)"));
    assert!(
        include_str!("../../ui.rs")
            .contains("(Control::InputProfiles, \"Hotkey profiles / shortcuts\"")
    );
    assert!(include_str!("../../ui/toolbar.rs").contains("| Control::InputProfiles"));
    assert_eq!(
        crate::bevy_shell::ui::Control::InputProfiles.id(),
        "shell.input_profiles"
    );
    let mut app = fixture()?;
    click(&mut app, Kind::Close)?;
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
    assert!(app.world().resource::<State>().root.is_some());
    // Closing is admitted even if another owner modal or gameplay block appears.
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .request_logout_confirmation();
    click(&mut app, Kind::Close)?;
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    Ok(())
}
