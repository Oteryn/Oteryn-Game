use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn control(app: &mut App, wanted: fn(Edit) -> bool) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Edit)>()
        .iter(app.world())
        .find_map(|(entity, edit)| wanted(*edit).then_some(entity))
        .ok_or_else(|| "editor action".into())
}
fn set(app: &mut App, index: usize, value: &str) {
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == index {
            text.editor_mut().set_text(value);
        }
    }
}
#[test]
fn assignment_buttons_reach_real_local_owner_and_reserved_key_validation() -> TestResult {
    let (mut app, mut commands) = super::super::tests::fixture()?;
    let generation = app.world().non_send::<HostOwner>().generation();
    app.world_mut()
        .resource_mut::<EditorState>()
        .open(0, generation);
    app.update();
    set(&mut app, 2, "A real saved greeting");
    let save = control(&mut app, |edit| matches!(edit, Edit::Text))?;
    app.world_mut().trigger(Activate { entity: save });
    assert_eq!(
        app.world().resource::<EditorState>().message,
        "Saved action assignment"
    );
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .spell_slot_labels()[0]
            .contains("A real saved greeting")
    );
    assert!(
        commands.try_recv().is_err(),
        "editing must not activate a command"
    );
    let bind = control(&mut app, |edit| matches!(edit, Edit::Key))?;
    set(&mut app, 3, "ESCAPE");
    app.world_mut().trigger(Activate { entity: bind });
    assert!(
        app.world()
            .resource::<EditorState>()
            .message
            .starts_with("Choose a valid")
    );
    set(&mut app, 3, "SUPER+ALT+F12");
    app.world_mut().trigger(Activate { entity: bind });
    assert_eq!(
        app.world().resource::<EditorState>().message,
        "Saved action assignment"
    );
    let clear = control(&mut app, |edit| matches!(edit, Edit::Clear))?;
    app.world_mut().trigger(Activate { entity: clear });
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .spell_slot_labels()[0]
            .contains("A real saved greeting")
    );
    let close = control(&mut app, |edit| matches!(edit, Edit::Close))?;
    app.world_mut().trigger(Activate { entity: close });
    assert!(!app.world().resource::<EditorState>().active());
    assert!(app.world().resource::<InputFocus>().get().is_none());
    Ok(())
}

#[test]
fn stale_editor_cannot_write_into_current_session() -> TestResult {
    let (mut app, mut commands) = super::super::tests::fixture()?;
    app.world_mut()
        .resource_mut::<EditorState>()
        .open(0, u64::MAX);
    set(&mut app, 2, "Stale assignment");
    let save = control(&mut app, |edit| matches!(edit, Edit::Text))?;
    app.world_mut().trigger(Activate { entity: save });
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .spell_slot_labels()[0]
            .contains("Stale assignment")
    );
    assert!(commands.try_recv().is_err());
    app.update();
    assert!(!app.world().resource::<EditorState>().active());
    Ok(())
}

#[test]
fn escape_belongs_to_top_server_dialog_before_action_editor() -> TestResult {
    let (mut app, _commands) = super::super::tests::fixture()?;
    let generation = app.world().non_send::<HostOwner>().generation();
    app.world_mut()
        .resource_mut::<EditorState>()
        .open(0, generation);
    app.update();
    let modal = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .bevy_server_modal_fixture();
    app.init_resource::<ButtonInput<KeyCode>>();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();
    assert!(
        app.world().resource::<EditorState>().active(),
        "Escape also closed the lower action editor"
    );
    drop(modal);
    app.update();
    assert!(!app.world().resource::<EditorState>().active());
    Ok(())
}
