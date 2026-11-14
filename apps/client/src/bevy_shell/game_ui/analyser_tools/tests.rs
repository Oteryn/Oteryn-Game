use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn fixture() -> Result<
    (
        App,
        tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
        Outbox,
    ),
    Box<dyn std::error::Error>,
> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_analyser_initialize(true);
    link.analyser_xp_goal("123");
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
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
    Ok((app, commands, outbox))
}
fn control(
    app: &mut App,
    action: Option<usize>,
    confirmed: bool,
) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.action == action && c.confirmed == confirmed).then_some(e))
        .ok_or("control absent".into())
}
#[test]
fn analyser_tools_window_shows_owner_data_only() -> TestResult {
    let (mut app, _commands, _outbox) = fixture()?;
    open(app.world().non_send::<HostOwner>());
    app.update();
    let owner = app.world().non_send::<HostOwner>();
    let expected = owner
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .analyser_text()
        .replace("\r\n", "\n");
    assert_eq!(
        app.world_mut()
            .query_filtered::<&Text, With<OwnerText>>()
            .single(app.world())?
            .0,
        expected
    );
    let count = app
        .world_mut()
        .query::<&Control>()
        .iter(app.world())
        .count();
    assert_eq!(count, ANALYSER_TOOLS_LABELS.len() + 1);
    let fields = app
        .world_mut()
        .query::<(&Field, &EditableText)>()
        .iter(app.world())
        .map(|(field, text)| (field.0, text.editor.text().to_string()))
        .collect::<Vec<_>>();
    assert!(fields.contains(&(0, "123".into())));
    Ok(())
}
#[test]
fn analyser_tools_window_registered() -> TestResult {
    let (mut app, mut commands, _) = fixture()?;
    app.add_systems(Update, crate::bevy_shell::ui::project_toolbar);
    app.update();
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| matches!(c, crate::bevy_shell::ui::Control::AnalyserTools).then_some(e))
        .ok_or("toolbar")?;
    assert_eq!(
        app.world().get::<Node>(toolbar).ok_or("node")?.display,
        Display::Flex
    );
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    let close = control(&mut app, None, false)?;
    app.world_mut().trigger(Activate { entity: close });
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    let chord = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(28)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control)
            .with(oteryn_input_actions::Modifier::Shift),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[chord], false);
    app.update();
    assert!(app.world().resource::<State>().root.is_some());
    assert!(commands.try_recv().is_err());
    app.world_mut().non_send_mut::<HostOwner>().close();
    app.update();
    assert_eq!(
        app.world().get::<Node>(toolbar).ok_or("node")?.display,
        Display::None
    );
    Ok(())
}
#[test]
fn analyser_tools_confirmation_invalidated_by_draft_edit() -> TestResult {
    let (mut app, _commands, _outbox) = fixture()?;
    open(app.world().non_send::<HostOwner>());
    app.update();
    let reset = control(&mut app, Some(14), false)?;
    app.world_mut().trigger(Activate { entity: reset });
    app.update();
    assert!(app.world().resource::<State>().confirmation.is_some());
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == 1 {
            text.editor_mut().set_text("changed");
        }
    }
    app.update();
    assert!(app.world().resource::<State>().confirmation.is_none());
    assert!(control(&mut app, Some(14), true).is_err());
    Ok(())
}

#[test]
fn analyser_tools_shortcut_preserves_focused_chat_and_arrow_input() -> TestResult {
    use oteryn_input_actions::{Modifier, Modifiers};
    let (mut app, mut commands, _outbox) = fixture()?;
    let editor = app.world_mut().spawn(EditableText::new("chat draft")).id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, bevy::input_focus::FocusCause::Navigated);
    let chord = Modifiers::one(Modifier::Control).with(Modifier::Shift);
    crate::bevy_shell::controls::tests::shortcut(&mut app, 28, chord)?;
    // ArrowUp uses the same focused-editor admission as the winit input adapter.
    crate::bevy_shell::controls::tests::shortcut(&mut app, 82, Modifiers::default())?;
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    assert_eq!(
        app.world()
            .get::<EditableText>(editor)
            .ok_or("editor")?
            .editor
            .text()
            .to_string(),
        "chat draft"
    );
    assert!(commands.try_recv().is_err());
    app.world_mut().resource_mut::<InputFocus>().clear();
    {
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        link.open_chat();
        if !link.preference_chat_open() {
            link.chat_console_mode();
        }
        assert!(link.preference_chat_open());
    }
    crate::bevy_shell::controls::tests::shortcut(&mut app, 28, chord)?;
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn analyser_tools_owner_refresh_preserves_draft_and_invalidates_confirmation() -> TestResult {
    let (mut app, _commands, _outbox) = fixture()?;
    open(app.world().non_send::<HostOwner>());
    app.update();
    let reset = control(&mut app, Some(14), false)?;
    app.world_mut().trigger(Activate { entity: reset });
    app.update();
    assert!(app.world().resource::<State>().confirmation.is_some());
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == 0 {
            text.editor_mut().set_text("987");
        }
    }
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .analyser_select("hunting");
    app.update();
    assert!(app.world().resource::<State>().confirmation.is_none());
    assert!(
        app.world_mut()
            .query::<(&Field, &EditableText)>()
            .iter(app.world())
            .any(|(f, t)| f.0 == 0 && t.editor.text() == "987")
    );
    // Escape uses the same session-only close admission as the Close control.
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
    Ok(())
}
