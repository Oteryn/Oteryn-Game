use super::*;
use crate::play::{PlayView, play_channel};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
fn fixture() -> std::result::Result<App, Box<dyn std::error::Error>> {
    let (link, commands, _) = play_channel();
    link.use_shared_frontend();
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { link, view });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .insert_resource(TestCommands(commands))
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok(app)
}
#[derive(Resource)]
struct TestCommands(tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>);
fn click(app: &mut App, kind: Kind) -> Result {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.kind == kind).then_some(e))
        .ok_or("control")?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
fn binding(app: &App) -> std::result::Result<Binding, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .battle_config_binding())
}
#[test]
fn battle_config_window_shows_owner_data_only() -> Result {
    let mut app = fixture()?;
    {
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        let mut lists = vec![BattleListPreferences {
            name: "Saved hunters".into(),
            search: "rat".into(),
            creatures: false,
            max_health: 42,
            vocation: Some(4),
            guild: GuildFilter::OutsideGuild,
            sort: BattleSort::HealthDescending,
            ..default()
        }];
        link.sync_battle_lists(&mut lists, u64::MAX);
        open(owner);
    }
    app.update();
    let expected = binding(&app)?;
    assert_eq!(
        app.world().resource::<State>().draft.as_ref(),
        expected.lists.preferences.first()
    );
    let fields = app
        .world_mut()
        .query::<(&Field, &EditableText)>()
        .iter(app.world())
        .map(|(field, text)| (field.0, text.value().to_string()))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(fields.get(&0).map(String::as_str), Some("Saved hunters"));
    assert_eq!(fields.get(&1).map(String::as_str), Some("rat"));
    assert_eq!(fields.get(&2).map(String::as_str), Some("42"));
    let labels = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect::<Vec<_>>();
    for label in [
        "Saved hunters",
        "Players: true",
        "Creatures: false",
        "Profession: Knight",
        "Guild: Outside my Guild",
        "Sort: Health: high first",
    ] {
        assert!(labels.iter().any(|text| text == label), "{label}");
    }
    assert!(
        app.world_mut()
            .resource_mut::<TestCommands>()
            .0
            .try_recv()
            .is_err()
    );
    Ok(())
}
#[test]
fn battle_config_window_registered() -> Result {
    let mut app = fixture()?;
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| matches!(c, crate::bevy_shell::ui::Control::BattleConfig).then_some(e))
        .ok_or("toolbar")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(app.world().resource::<State>().root.is_some());
    click(&mut app, Kind::Close)?;
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    let key = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(5)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Alt),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[key], false);
    app.update();
    assert!(app.world().resource::<State>().root.is_some());
    Ok(())
}
#[test]
fn battle_config_confirmation_and_modal_admission() -> Result {
    let mut app = fixture()?;
    open(app.world().non_send::<HostOwner>());
    app.update();
    click(&mut app, Kind::Add)?;
    app.update();
    let before = binding(&app)?.lists;
    click(&mut app, Kind::Remove)?;
    assert!(app.world().resource::<State>().confirm.is_some());
    assert_eq!(binding(&app)?.lists, before);
    app.update();
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == 0 {
            text.editor_mut().set_text("Changed draft");
        }
    }
    app.update();
    assert!(app.world().resource::<State>().confirm.is_none());
    click(&mut app, Kind::Remove)?;
    assert_eq!(binding(&app)?.lists, before);
    app.update();
    click(&mut app, Kind::ConfirmRemove)?;
    app.update();
    assert_eq!(binding(&app)?.lists.preferences.len(), 1);
    let modal = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(999), Node::default()))
        .id();
    let before = binding(&app)?.lists;
    click(&mut app, Kind::Add)?;
    assert_eq!(binding(&app)?.lists, before);
    // Close is session-only even under another modal.
    click(&mut app, Kind::Close)?;
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    app.world_mut().entity_mut(modal).despawn();
    Ok(())
}

#[test]
fn battle_config_draft_controls_save_through_local_owner() -> Result {
    let mut app = fixture()?;
    open(app.world().non_send::<HostOwner>());
    app.update();
    let before = binding(&app)?.lists;
    for kind in [
        Kind::Players,
        Kind::Creatures,
        Kind::Sort,
        Kind::Vocation,
        Kind::Guild,
    ] {
        click(&mut app, kind)?;
        app.update();
        assert_eq!(binding(&app)?.lists, before);
    }
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        text.editor_mut().set_text(match field.0 {
            0 => "Custom list",
            1 => "dragon",
            _ => "55",
        });
    }
    click(&mut app, Kind::Save)?;
    app.update();
    let saved = binding(&app)?.lists;
    let list = &saved.preferences[0];
    assert_eq!(list.name, "Custom list");
    assert_eq!(list.search, "dragon");
    assert_eq!(list.max_health, 55);
    assert!(!list.players);
    assert!(!list.creatures);
    assert_eq!(list.sort, BattleSort::HealthAscending);
    assert_eq!(list.vocation, Some(0));
    assert_eq!(list.guild, GuildFilter::SameGuild);
    assert!(
        app.world_mut()
            .resource_mut::<TestCommands>()
            .0
            .try_recv()
            .is_err()
    );
    Ok(())
}

#[test]
fn battle_config_shortcut_respects_chat_and_focus() -> Result {
    let mut app = fixture()?;
    let key = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(5)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Alt),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(std::slice::from_ref(&key), true);
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    link.open_chat();
    if !link.preference_chat_open() {
        link.chat_console_mode();
    }
    assert!(link.preference_chat_open());
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[key], false);
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    Ok(())
}

#[test]
fn battle_config_close_precedes_invalid_draft_and_modal() -> Result {
    let mut app = fixture()?;
    open(app.world().non_send::<HostOwner>());
    app.update();
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == 2 {
            text.editor_mut().set_text("bad");
        }
    }
    app.world_mut().non_send_mut::<HostOwner>().settings.open = true;
    let before = binding(&app)?.lists.preferences;
    click(&mut app, Kind::Close)?;
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    assert_eq!(binding(&app)?.lists.preferences, before);
    Ok(())
}

#[test]
fn battle_config_escape_and_replaced_session_controls() -> Result {
    let mut app = fixture()?;
    open(app.world().non_send::<HostOwner>());
    app.update();
    let stale = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == Kind::Add).then_some(entity))
        .ok_or("add")?;
    let (replacement, _commands, _) = play_channel();
    replacement.use_shared_frontend();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = replacement;
    let before = binding(&app)?.lists;
    app.world_mut().trigger(Activate { entity: stale });
    assert_eq!(binding(&app)?.lists, before);
    assert!(!binding(&app)?.open);
    app.update();
    open(app.world().non_send::<HostOwner>());
    app.update();
    app.world_mut().non_send_mut::<HostOwner>().settings.open = true;
    app.world_mut()
        .spawn((GameModal, GlobalZIndex(999), Node::default()));
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: bevy::input::keyboard::Key::Escape,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    assert!(!binding(&app)?.open);
    Ok(())
}
