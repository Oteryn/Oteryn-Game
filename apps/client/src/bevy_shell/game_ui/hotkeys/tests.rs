use super::*;
#[test]
fn hotkeys_add_edit_remove() {
    let mut p = crate::play::ReferencePreferences::default();
    let action = crate::play::CustomAction::Text {
        text: "hello".into(),
        send: false,
    };
    assert!(save(&mut p, None, "F9", action.clone()).is_ok());
    assert!(save(&mut p, Some("F9"), "F10", action.clone()).is_err());
    assert!(p.custom_hotkeys.contains_key("F9"));
    assert!(save(&mut p, Some("F9"), "SUPER+F10", action).is_ok());
    assert!(!p.custom_hotkeys.contains_key("F9"));
    p.custom_hotkeys.remove("SUPER+F10");
    assert!(!p.custom_hotkeys.contains_key("SUPER+F10"));
}
#[test]
fn hotkeys_list() {
    let mut p = crate::play::ReferencePreferences::default();
    p.custom_hotkeys.clear();
    p.custom_hotkeys.insert(
        "F9".into(),
        crate::play::CustomAction::Text {
            text: "hello".into(),
            send: true,
        },
    );
    assert_eq!(rows(&p), vec!["F9 | Text (send automatically) | hello"]);
}
#[test]
fn hotkey_text_macro_auto_send() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("oteryn-hotkeys.json");
    let mut store = crate::play::PreferenceStore::defaults(path.clone());
    for send in [false, true] {
        let a = action(0, "hello", send, "", "").ok_or("missing text action")?;
        assert_eq!(
            a,
            crate::play::CustomAction::Text {
                text: "hello".into(),
                send
            }
        );
        let mut p = crate::play::ReferencePreferences::default();
        save(&mut p, None, "F9", a.clone())?;
        store.apply(p)?;
        let restored = crate::play::PreferenceStore::read_transfer(&path)?;
        assert_eq!(restored.custom_hotkeys.get("F9"), Some(&a));
    }
    std::fs::remove_file(path)?;
    Ok(())
}
#[test]
fn hotkey_use_object_modes() {
    for (mode, target) in [
        (1, crate::play::ObjectTarget::SelfActor),
        (2, crate::play::ObjectTarget::CurrentTarget),
        (3, crate::play::ObjectTarget::Cursor),
    ] {
        assert_eq!(
            action(mode, "", false, "100", "0"),
            Some(crate::play::CustomAction::Object {
                definition: 100,
                sub_type: 0,
                target
            })
        );
    }
}

type TestResult = Result<(), Box<dyn std::error::Error>>;
fn editor_app() -> Result<(App, Entity), Box<dyn std::error::Error>> {
    let mut owner = HostOwner::new(None)?;
    owner.open_graphics();
    let generation = owner.generation();
    let mut app = App::new();
    app.insert_non_send(owner)
        .init_resource::<bevy::input_focus::InputFocus>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, capture);
    let editor = app
        .world_mut()
        .spawn(Editor {
            generation,
            selected: None,
            mode: 0,
            send: false,
            feedback: String::new(),
            capturing: false,
            picking: false,
        })
        .id();
    for index in 0..4 {
        app.world_mut().spawn((Field(index), EditableText::new("")));
    }
    Ok((app, editor))
}
fn press(app: &mut App, key_code: KeyCode, repeat: bool) {
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code,
            logical_key: bevy::input::keyboard::Key::Unidentified(
                bevy::input::keyboard::NativeKey::Unidentified,
            ),
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat,
            window: Entity::PLACEHOLDER,
        });
    app.update();
}
fn control(app: &mut App, control: Control) {
    let entity = app.world_mut().spawn(control).observe(activate).id();
    app.world_mut().trigger(Activate { entity });
}
fn set_value(app: &mut App, index: usize, value: &str) {
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
fn value(app: &mut App, index: usize) -> String {
    app.world_mut()
        .query::<(&Field, &EditableText)>()
        .iter(app.world())
        .find(|(field, _)| field.0 == index)
        .map(|(_, text)| text.value().to_string())
        .unwrap_or_default()
}
#[test]
fn capture_key_reports_conflicts_without_replacing_bindings() -> TestResult {
    let (mut app, editor) = editor_app()?;
    set_value(&mut app, 1, "hello");
    control(&mut app, Control::Capture);
    press(&mut app, KeyCode::ShiftLeft, false);
    press(&mut app, KeyCode::F9, true);
    assert!(app.world().get::<Editor>(editor).ok_or("editor")?.capturing);
    press(&mut app, KeyCode::F10, false);
    assert_eq!(value(&mut app, 0), "F10");
    assert!(
        app.world()
            .get::<Editor>(editor)
            .ok_or("editor")?
            .feedback
            .contains("conflict")
    );
    control(&mut app, Control::Save);
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .preferences_draft()
            .ok_or("draft")?
            .custom_hotkeys
            .contains_key("F10")
    );
    control(&mut app, Control::Capture);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::SuperLeft);
    press(&mut app, KeyCode::F9, false);
    assert_eq!(value(&mut app, 0), "SUPER+F9");
    assert!(
        !app.world()
            .get::<Editor>(editor)
            .ok_or("editor")?
            .feedback
            .contains("conflict")
    );
    control(&mut app, Control::Save);
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .preferences_draft()
            .ok_or("draft")?
            .custom_hotkeys
            .contains_key("SUPER+F9")
    );
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .settings
            .preferences
            .custom_hotkeys
            .contains_key("SUPER+F9")
    );
    // A fresh higher modal fences capture, and hiding the editor cancels capture.
    control(&mut app, Control::Capture);
    app.world_mut()
        .spawn((super::super::GameModal, GlobalZIndex(500), Node::default()));
    press(&mut app, KeyCode::F8, false);
    assert_eq!(value(&mut app, 0), "SUPER+F9");
    assert!(!app.world().get::<Editor>(editor).ok_or("editor")?.capturing);
    Ok(())
}
#[test]
fn object_picker_assigns_live_definition_and_subtype_and_rejects_stale_entry() -> TestResult {
    use crate::play::{PlayView, play_channel};
    use oteryn_session::{AppliedDelta, CharacterInventory, SessionEvent};
    let (mut app, editor) = editor_app()?;
    let (link, _commands, outbox) = play_channel();
    let item = oteryn_client_runtime::depot::ItemEntry {
        appearance_id: 0,
        handle: 7.try_into()?,
        item_definition_ref: 42.try_into()?,
        count: 3.try_into()?,
        sub_type: 5,
    };
    outbox.push(vec![SessionEvent::Inventory(AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value: CharacterInventory {
            entries: vec![item],
            ..default()
        },
    })]);
    let view = |view_id,
                container: u64,
                parent: Option<u64>,
                entry: u64|
     -> Result<_, Box<dyn std::error::Error>> {
        Ok(oteryn_session::container_tree::ContainerView {
            view_id,
            container_handle: container.try_into()?,
            parent_handle: parent.map(TryInto::try_into).transpose()?,
            capacity: 20,
            entries: vec![ItemEntry {
                handle: entry.try_into()?,
                ..item
            }],
            browse: None,
        })
    };
    outbox.push(vec![SessionEvent::ContainerViews(AppliedDelta {
        server_sequence: 2,
        base_revision: 0,
        new_revision: 1,
        value: oteryn_session::container_tree::ContainerViews {
            // A replaced parent view still proves ancestry; ground bags never seed it.
            views: vec![
                view(0, 10, Some(8), 11)?,
                view(1, 8, Some(7), 9)?,
                view(2, 20, None, 21)?,
            ],
        },
    })]);
    let carried = object_picker::carried(&link);
    assert_eq!(
        carried
            .iter()
            .map(|item| item.handle.get())
            .collect::<Vec<_>>(),
        vec![7, 9, 11]
    );
    app.world_mut().non_send_mut::<HostOwner>().play = Some(crate::bevy_shell::owner::ActivePlay {
        link,
        view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
    });
    app.insert_resource(Skins {
        buttons: source::ButtonImages {
            up: default(),
            down: default(),
        },
        checked: default(),
        unchecked: default(),
        frame: default(),
        inset: default(),
        font: default(),
    })
    .add_systems(Update, objects);
    app.world_mut().spawn((Objects, Node::default()));
    control(&mut app, Control::Pick);
    app.update();
    let card = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| {
            matches!(control, Control::Object(entry) if *entry == item).then_some(entity)
        })
        .ok_or("live object card")?;
    app.world_mut().trigger(Activate { entity: card });
    assert_eq!(value(&mut app, 2), "42");
    assert_eq!(value(&mut app, 3), "5");
    assert_eq!(app.world().get::<Editor>(editor).ok_or("editor")?.mode, 4);
    set_value(&mut app, 0, "F9");
    control(&mut app, Control::Save);
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .preferences_draft()
            .ok_or("draft")?
            .custom_hotkeys
            .get("F9"),
        Some(&CustomAction::Object {
            definition: 42,
            sub_type: 5,
            target: ObjectTarget::Use
        })
    );
    // The selected snapshot must still match authoritative inventory at activation.
    outbox.push(vec![SessionEvent::Inventory(AppliedDelta {
        server_sequence: 3,
        base_revision: 1,
        new_revision: 2,
        value: CharacterInventory::default(),
    })]);
    control(&mut app, Control::Pick);
    control(&mut app, Control::Object(item));
    assert!(
        app.world()
            .get::<Editor>(editor)
            .ok_or("editor")?
            .feedback
            .contains("no longer")
    );
    Ok(())
}

#[test]
fn hotkey_editor_exposes_key_capture_and_object_picker() {
    let mut app = App::new();
    app.insert_resource(Skins {
        buttons: source::ButtonImages {
            up: default(),
            down: default(),
        },
        checked: default(),
        unchecked: default(),
        frame: default(),
        inset: default(),
        font: default(),
    })
    .add_systems(Startup, |mut commands: Commands, skins: Res<Skins>| {
        let root = commands.spawn(Node::default()).id();
        build(&mut commands, root, &skins);
    });
    app.update();
    let labels: Vec<_> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.as_str())
        .collect();
    assert!(
        labels.contains(&"Capture key"),
        "editor requires a reachable key-capture control"
    );
    assert!(
        labels.contains(&"Choose carried object"),
        "editor requires a reachable object picker"
    );
}
