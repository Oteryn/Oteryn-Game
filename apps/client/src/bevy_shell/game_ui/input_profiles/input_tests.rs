use super::tests::{TestResult, click, fixture};
use super::*;
use bevy::input_focus::InputFocus;
#[test]
fn input_profiles_escape_closes_with_confirmation_and_blocked_input() -> TestResult {
    use bevy::input::keyboard::{Key as LogicalKey, KeyboardInput, NativeKey};
    let mut app = fixture()?;
    click(&mut app, Kind::Delete)?;
    app.update();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .request_logout_confirmation();
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::Escape,
        logical_key: LogicalKey::Unidentified(NativeKey::Unidentified),
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    assert!(app.world().resource::<State>().confirm.is_none());
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .settings
            .preferences
            .hotkey_contexts
            .presets
            .len(),
        1
    );
    Ok(())
}
#[test]
fn input_profiles_focused_chat_arrows_do_not_open_or_move() -> TestResult {
    use bevy::ecs::system::RunSystemOnce;
    use oteryn_input_actions::{ButtonState, KeyCode as OwnerKey, Modifiers, NormalizedInputEvent};
    let mut app = fixture()?;
    click(&mut app, Kind::Close)?;
    app.update();
    let editor = app
        .world_mut()
        .spawn((Node::default(), TextInput, EditableText::new("chat")))
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, bevy::input_focus::FocusCause::Navigated);
    let blocked = app
        .world_mut()
        .run_system_once(
            |focus: Res<InputFocus>, editors: Query<(), With<EditableText>>, guard: ModalGuard| {
                focus.get().is_some_and(|entity| editors.contains(entity))
                    || !guard.allows_focus(None)
            },
        )
        .map_err(|error| format!("{error:?}"))?;
    assert!(blocked);
    let (link, mut commands, _outbox) = crate::play::play_channel();
    link.use_shared_frontend();
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    owner.play.as_mut().ok_or("play")?.link = link;
    owner.route_input(
        &[NormalizedInputEvent::Key {
            code: OwnerKey::ARROW_UP,
            modifiers: Modifiers::NONE,
            state: ButtonState::Pressed,
            repeat: false,
        }],
        blocked,
    );
    assert!(!owner.settings.open);
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn input_profiles_bound_generation_and_changed_owner_reject_actions() -> TestResult {
    let mut app = fixture()?;
    let controls: Vec<_> = app
        .world_mut()
        .query::<&Control>()
        .iter(app.world())
        .cloned()
        .collect();
    let copy = controls
        .iter()
        .find(|control| control.kind == Kind::Copy)
        .ok_or("copy")?;
    let close = controls
        .iter()
        .find(|control| control.kind == Kind::Close)
        .ok_or("close")?;
    let mut state = app.world_mut().remove_resource::<State>().ok_or("state")?;
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    let original = owner.settings.preferences.clone();
    let mut changed = original.clone();
    changed.add_hotkey_profile("External")?;
    owner.settings.apply(changed.clone());
    let draft = ["Rejected".into(), String::new(), String::new()];
    perform(&mut owner, &mut state, copy, &draft, true);
    assert_eq!(owner.settings.preferences, changed);
    owner.settings.apply(original.clone());
    for control in [copy, close] {
        let mut stale = control.clone();
        stale.key.generation = owner.generation().saturating_add(1);
        perform(&mut owner, &mut state, &stale, &draft, true);
        assert!(is_open(&owner));
        assert_eq!(owner.settings.preferences, original);
        stale.key.generation = owner.generation();
        stale.key.scope = Some([7; 16]);
        perform(&mut owner, &mut state, &stale, &draft, true);
        assert!(is_open(&owner));
        assert_eq!(owner.settings.preferences, original);
    }
    // A pending confirmation and rejected input admission cannot trap Close.
    state.confirm = Some(draft.clone());
    state.dirty = true;
    perform(&mut owner, &mut state, close, &draft, false);
    assert!(!owner.settings.open);
    assert_eq!(owner.settings.preferences, original);
    Ok(())
}
