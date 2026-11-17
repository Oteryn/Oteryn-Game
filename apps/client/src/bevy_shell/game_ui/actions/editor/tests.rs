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
/// The visible button whose caption reads `caption`, as a player sees it in the editor.
fn captioned(app: &mut App, caption: &str) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .find_map(|(text, parent)| (text.0 == caption).then_some(parent.parent()))
        .ok_or_else(|| format!("no button captioned {caption}").into())
}
#[test]
fn assignment_buttons_reach_real_local_owner_and_reserved_key_validation() -> TestResult {
    let (mut app, mut commands) = super::super::tests::fixture()?;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .settings
        .preferences
        .rebind("local-spell-action-bar-spell-1", "CTRL+F1")?;
    let generation = app.world().non_send::<HostOwner>().generation();
    app.world_mut()
        .resource_mut::<EditorState>()
        .open(0, generation);
    app.update();
    assert!(
        app.world_mut()
            .query::<(&Field, &EditableText)>()
            .iter(app.world())
            .any(|(field, text)| field.0 == 3 && text.value() == "CTRL+F1")
    );
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
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .settings
            .preferences
            .keybindings
            .get("local-spell-action-bar-spell-1")
            .map(String::as_str),
        Some("SUPER+ALT+F12")
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

#[test]
fn action_assignment_waits_for_queued_editor_changes() -> TestResult {
    let (link, _commands, _outbox) = crate::play::play_channel();
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    let generation = owner.generation();
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        bevy::input::InputPlugin,
        bevy::input_focus::InputDispatchPlugin,
        bevy::window::WindowPlugin::default(),
        bevy::text::TextPlugin,
        bevy::ui_widgets::TextInputPlugin,
        source::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_resource::<EditorState>()
    .init_resource::<InputFocus>()
    .init_resource::<UiScale>()
    .init_resource::<bevy::text::RemSize>()
    .init_resource::<bevy::picking::events::PointerState>()
    .add_message::<bevy::picking::events::PointerRelease>()
    .insert_non_send(owner)
    .add_systems(
        Startup,
        (
            crate::bevy_shell::ui::setup,
            |mut skins: ResMut<Skins>, mut fonts: ResMut<Assets<Font>>| {
                skins.font = fonts.add(Font::from_bytes(
                    include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/tests/assets/FiraMono-subset.ttf"
                    ))
                    .to_vec(),
                ));
            },
            setup,
        )
            .chain(),
    )
    .add_systems(Update, project)
    .add_systems(PostUpdate, bevy::ui::widget::update_editable_text_styles);
    install(&mut app);
    super::super::super::modal::install(&mut app);
    app.update();
    app.world_mut()
        .resource_mut::<EditorState>()
        .open(0, generation);
    app.update();
    set(&mut app, 2, "hello");
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == 2 {
            text.queue_edit(bevy::text::TextEdit::TextEnd(false));
            text.queue_edit(bevy::text::TextEdit::Insert(" world".into()));
        }
    }
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
            .contains("hello"),
        "queued input must finish before assignment"
    );
    app.update();
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .spell_slot_labels()[0]
            .contains("hello world")
    );
    let editor = app
        .world_mut()
        .query::<(Entity, &Field)>()
        .iter(app.world())
        .find_map(|(entity, field)| (field.0 == 2).then_some(entity))
        .ok_or("text field")?;
    let clipboard = std::sync::Arc::new(std::sync::Mutex::new(None));
    {
        let mut text = app
            .world_mut()
            .get_mut::<EditableText>(editor)
            .ok_or("editor")?;
        text.queue_edit(bevy::text::TextEdit::SelectAll);
        text.pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(
            std::sync::Arc::clone(&clipboard),
        ));
    }
    app.world_mut().trigger(Activate { entity: save });
    app.update();
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .spell_slot_labels()[0]
            .contains("hello world")
    );
    *clipboard.lock().map_err(|_| "clipboard")? = Some(Ok("pasted text".into()));
    app.update();
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .spell_slot_labels()[0]
            .contains("pasted text")
    );
    let clipboard = std::sync::Arc::new(std::sync::Mutex::new(None));
    app.world_mut()
        .get_mut::<EditableText>(editor)
        .ok_or("editor")?
        .pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(
        std::sync::Arc::clone(&clipboard),
    ));
    app.world_mut().trigger(Activate { entity: save });
    app.world_mut()
        .resource_mut::<EditorState>()
        .open(1, generation);
    *clipboard.lock().map_err(|_| "clipboard")? = Some(Ok("stale paste".into()));
    app.update();
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .spell_slot_labels()[0]
            .contains("pasted text")
    );
    Ok(())
}

#[test]
fn use_button_saves_subtype_and_activates_direct_item_use() -> TestResult {
    use crate::play::{BarAction, ItemUseMode, PlayLink, SettingsState, play_channel};
    let (mut app, _) = super::super::tests::fixture()?;
    let (link, mut commands, outbox) = play_channel();
    let item = oteryn_session::ItemEntry {
        appearance_id: 0,
        handle: 7.try_into()?,
        item_definition_ref: 42.try_into()?,
        count: 1.try_into()?,
        sub_type: 7,
    };
    PlayLink::bevy_items_fixture(
        &outbox,
        Some(oteryn_session::CharacterInventory {
            entries: vec![item],
            ..Default::default()
        }),
        Some(Default::default()),
        true,
        true,
    );
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = link;
    let generation = app.world().non_send::<HostOwner>().generation();
    app.world_mut()
        .resource_mut::<EditorState>()
        .open(0, generation);
    app.update();
    let object = captioned(&mut app, "Object 42 (7)")?;
    app.world_mut().trigger(Activate { entity: object });
    let save = control(&mut app, |edit| matches!(edit, Edit::Use))?;
    app.world_mut().trigger(Activate { entity: save });
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    let mut settings = SettingsState::new(None);
    settings.sync_bar_actions(link);
    assert_eq!(
        settings.preferences.bar_actions.0.get(&0),
        Some(&BarAction::Use {
            definition: item.item_definition_ref,
            sub_type: Some(7),
            mode: ItemUseMode::Use,
        })
    );
    link.cast_spell_slot(std::num::NonZeroU32::MIN);
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::WorldContext(command)
        if command.target == oteryn_client_runtime::world_interaction::Target::Item(item.handle)
        && command.action == oteryn_client_runtime::world_interaction::Action::Use)
    );
    Ok(())
}

#[test]
fn assignment_waits_for_pending_edits_and_revalidates_slot() -> TestResult {
    let (mut app, _) = super::super::tests::fixture()?;
    let generation = app.world().non_send::<HostOwner>().generation();
    app.world_mut()
        .resource_mut::<EditorState>()
        .open(0, generation);
    app.update();
    set(&mut app, 2, "previous");
    let field = app
        .world_mut()
        .query::<(Entity, &Field)>()
        .iter(app.world())
        .find_map(|(entity, field)| (field.0 == 2).then_some(entity))
        .ok_or("text field")?;
    app.world_mut()
        .get_mut::<EditableText>(field)
        .ok_or("editor")?
        .queue_edit(bevy::text::TextEdit::Insert("last".into()));
    let save = control(&mut app, |edit| matches!(edit, Edit::InsertText))?;
    app.world_mut().trigger(Activate { entity: save });
    assert!(
        app.world().resource::<EditorState>().message.is_empty(),
        "must not assign stale text while edits are queued"
    );
    {
        let mut text = app
            .world_mut()
            .get_mut::<EditableText>(field)
            .ok_or("editor")?;
        text.pending_edits.clear();
        text.editor_mut().set_text("completed");
    }
    app.update();
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .spell_slot_labels()[0]
            .contains("completed")
    );
    app.world_mut()
        .get_mut::<EditableText>(field)
        .ok_or("editor")?
        .queue_edit(bevy::text::TextEdit::Insert("stale".into()));
    app.world_mut().trigger(Activate { entity: save });
    app.world_mut()
        .resource_mut::<EditorState>()
        .open(1, generation);
    app.update();
    assert!(app.world().resource::<EditorState>().message.is_empty());
    Ok(())
}

#[test]
fn multiaction_button_collects_objects_and_saves_them_as_one_slot() -> TestResult {
    use crate::play::{BarAction, ItemUseMode, PlayLink, SettingsState, play_channel};
    let (mut app, _) = super::super::tests::fixture()?;
    let (link, mut commands, outbox) = play_channel();
    let item = oteryn_session::ItemEntry {
        appearance_id: 0,
        handle: 7.try_into()?,
        item_definition_ref: 42.try_into()?,
        count: 1.try_into()?,
        sub_type: 7,
    };
    PlayLink::bevy_items_fixture(
        &outbox,
        Some(oteryn_session::CharacterInventory {
            entries: vec![item],
            ..Default::default()
        }),
        Some(Default::default()),
        true,
        true,
    );
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = link;
    let generation = app.world().non_send::<HostOwner>().generation();
    app.world_mut()
        .resource_mut::<EditorState>()
        .open(0, generation);
    app.update();
    let multi = control(&mut app, |edit| matches!(edit, Edit::Multi))?;
    app.world_mut().trigger(Activate { entity: multi });
    assert!(
        app.world()
            .resource::<EditorState>()
            .message
            .starts_with("Multiaction I: Empty")
    );
    // An empty draft is refused and stays open.
    app.world_mut().trigger(Activate { entity: multi });
    assert!(
        app.world()
            .resource::<EditorState>()
            .message
            .starts_with("Assign at least one")
    );
    let object = captioned(&mut app, "Object 42 (7)")?;
    app.world_mut().trigger(Activate { entity: object });
    for wanted in [
        (|edit| matches!(edit, Edit::Use)) as fn(Edit) -> bool,
        |edit| matches!(edit, Edit::SelfActor),
    ] {
        let button = control(&mut app, wanted)?;
        app.world_mut().trigger(Activate { entity: button });
    }
    let message = app.world().resource::<EditorState>().message.clone();
    assert!(message.contains("I: Use object 42") && message.contains("II: Use object 42"));
    {
        let owner = app.world().non_send::<HostOwner>();
        let mut settings = SettingsState::new(None);
        settings.sync_bar_actions(&owner.play.as_ref().ok_or("play")?.link);
        assert_eq!(
            settings.preferences.bar_actions.0.get(&0),
            None,
            "the draft is not saved before Multiaction is pressed again"
        );
    }
    app.world_mut().trigger(Activate { entity: multi });
    assert_eq!(
        app.world().resource::<EditorState>().message,
        "Multiaction saved"
    );
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    let mut settings = SettingsState::new(None);
    settings.sync_bar_actions(link);
    let use_with = |mode| BarAction::Use {
        definition: item.item_definition_ref,
        sub_type: Some(7),
        mode,
    };
    assert_eq!(
        settings.preferences.bar_actions.0.get(&0),
        Some(&BarAction::Multi {
            actions: vec![use_with(ItemUseMode::Use), use_with(ItemUseMode::SelfActor)],
        })
    );
    assert!(commands.try_recv().is_err(), "editing never uses the item");
    Ok(())
}

#[test]
fn empty_slot_multiaction_takes_objects_from_the_visible_object_picker() -> TestResult {
    use crate::play::{BarAction, ItemUseMode, PlayLink, SettingsState, play_channel};
    let (mut app, _) = super::super::tests::fixture()?;
    let (link, mut commands, outbox) = play_channel();
    let item = oteryn_session::ItemEntry {
        appearance_id: 0,
        handle: 9.try_into()?,
        item_definition_ref: 3031.try_into()?,
        count: 1.try_into()?,
        sub_type: 0,
    };
    PlayLink::bevy_items_fixture(
        &outbox,
        Some(oteryn_session::CharacterInventory {
            entries: vec![item],
            ..Default::default()
        }),
        Some(Default::default()),
        true,
        true,
    );
    // The server reports the carried object as equippable, as for a single Equip action.
    outbox.bevy_hotkey_equip_items(vec![3031]);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = link;
    let generation = app.world().non_send::<HostOwner>().generation();
    app.world_mut()
        .resource_mut::<EditorState>()
        .open(2, generation);
    app.update();
    let multi = control(&mut app, |edit| matches!(edit, Edit::Multi))?;
    app.world_mut().trigger(Activate { entity: multi });
    let equip = control(&mut app, |edit| matches!(edit, Edit::Equip))?;
    let cursor = control(&mut app, |edit| matches!(edit, Edit::Cursor))?;
    // Without a chosen object the object actions refuse, as before.
    app.world_mut().trigger(Activate { entity: equip });
    assert_eq!(app.world().resource::<EditorState>().message, REFUSED);
    // The player picks the carried object in the editor itself.
    let object = captioned(&mut app, "Object 3031")?;
    app.world_mut().trigger(Activate { entity: object });
    assert!(
        app.world()
            .resource::<EditorState>()
            .message
            .starts_with("Selected Object 3031")
    );
    for button in [equip, cursor] {
        app.world_mut().trigger(Activate { entity: button });
        app.update();
    }
    let message = app.world().resource::<EditorState>().message.clone();
    assert!(
        message.contains("I: Equip object 3031") && message.contains("II: Use object 3031"),
        "{message}"
    );
    app.world_mut().trigger(Activate { entity: multi });
    assert_eq!(
        app.world().resource::<EditorState>().message,
        "Multiaction saved"
    );
    let owner = app.world().non_send::<HostOwner>();
    let mut settings = SettingsState::new(None);
    settings.sync_bar_actions(&owner.play.as_ref().ok_or("play")?.link);
    assert_eq!(
        settings.preferences.bar_actions.0.get(&2),
        Some(&BarAction::Multi {
            actions: vec![
                BarAction::Equip {
                    definition: item.item_definition_ref,
                    upgrade_tier: 0,
                },
                BarAction::Use {
                    definition: item.item_definition_ref,
                    sub_type: Some(0),
                    mode: ItemUseMode::Cursor,
                },
            ],
        })
    );
    assert!(
        commands.try_recv().is_err(),
        "choosing an object never uses it"
    );
    Ok(())
}

/// Row 9 (slots 400-449) is as editable as row 1: the secondary click opens the editor and the
/// saved text reaches the local owner.
#[test]
fn ninth_row_slot_accepts_an_assignment_through_the_editor() -> TestResult {
    use super::super::ActionControl;
    let (mut app, mut commands) = super::super::tests::fixture()?;
    let control_of = |app: &mut App, wanted: ActionControl| {
        app.world_mut()
            .query::<(Entity, &ActionControl)>()
            .iter(app.world())
            .find_map(|(entity, control)| {
                let same = match (*control, wanted) {
                    (ActionControl::Slot(a), ActionControl::Slot(b))
                    | (ActionControl::Lock(a), ActionControl::Lock(b)) => a == b,
                    _ => false,
                };
                same.then_some(entity)
            })
            .ok_or("action control")
    };
    // Bars are shown from the options (UI-B); the bar's own control is only its lock.
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let mut preferences = owner.settings.preferences.clone();
        preferences.bar_layout.rows[8].enabled = true;
        owner.settings.apply(preferences);
        let owner = &mut *owner;
        let link = &owner.play.as_ref().ok_or("play")?.link;
        owner.settings.sync_bar_actions(link);
    }
    app.update();
    let lock = control_of(&mut app, ActionControl::Lock(8))?;
    app.world_mut().trigger(Activate { entity: lock });
    app.update();
    let slot = 8 * super::super::ROW_SLOTS;
    let entity = control_of(&mut app, ActionControl::Slot(slot))?;
    app.world_mut()
        .trigger(bevy::picking::events::PointerClick {
            entity,
            pointer: bevy::picking::events::Pointer::new(
                bevy::picking::pointer::PointerId::Mouse,
                bevy::picking::pointer::Location {
                    target: bevy::camera::NormalizedRenderTarget::None {
                        width: 100,
                        height: 100,
                    },
                    position: Vec2::ZERO,
                },
            ),
            button: bevy::picking::pointer::PointerButton::Secondary,
            hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0., None, None),
            duration: std::time::Duration::ZERO,
            count: 1,
        });
    app.update();
    // The secondary click opens the slot menu (UI-B); its Set Text entry opens the editor.
    let set_text = app
        .world_mut()
        .query::<(Entity, &super::super::MenuEntry)>()
        .iter(app.world())
        .find_map(|(entity, entry)| {
            (entry.choice == super::super::MenuChoice::SetText).then_some(entity)
        })
        .ok_or("Set Text")?;
    app.world_mut().trigger(Activate { entity: set_text });
    app.update();
    assert!(app.world().resource::<EditorState>().active());
    set(&mut app, 2, "Ninth row greeting");
    let save = control(&mut app, |edit| matches!(edit, Edit::Text))?;
    app.world_mut().trigger(Activate { entity: save });
    app.update();
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
            .spell_slot_labels()[slot]
            .contains("Ninth row greeting")
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn equip_editor_preserves_carried_assignment_tier() -> TestResult {
    let (mut app, _) = super::super::tests::fixture()?;
    let (link, _commands, outbox) = crate::play::play_channel();
    outbox.bevy_hotkey_equip_items(vec![256]);
    let expected = BarAction::Equip {
        definition: 256u32.try_into()?,
        upgrade_tier: 3,
    };
    assert!(link.assign_bar_action(0, expected.clone()));
    let generation = {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.settings.sync_bar_actions(&link);
        owner.play.as_mut().ok_or("play")?.link = link;
        owner.generation()
    };
    app.world_mut()
        .resource_mut::<EditorState>()
        .open(0, generation);
    app.update();
    let equip = control(&mut app, |edit| matches!(edit, Edit::Equip))?;
    app.world_mut().trigger(Activate { entity: equip });
    let owner = &mut *app.world_mut().non_send_mut::<HostOwner>();
    owner
        .settings
        .sync_bar_actions(&owner.play.as_ref().ok_or("play")?.link);
    assert_eq!(
        owner.settings.preferences.bar_actions.0.get(&0),
        Some(&expected)
    );
    let multi = control(&mut app, |edit| matches!(edit, Edit::Multi))?;
    app.world_mut().trigger(Activate { entity: multi });
    app.world_mut().trigger(Activate { entity: equip });
    app.world_mut().trigger(Activate { entity: multi });
    let owner = &mut *app.world_mut().non_send_mut::<HostOwner>();
    owner
        .settings
        .sync_bar_actions(&owner.play.as_ref().ok_or("play")?.link);
    assert_eq!(
        owner.settings.preferences.bar_actions.0.get(&0),
        Some(&BarAction::Multi {
            actions: vec![expected]
        })
    );
    Ok(())
}

#[test]
fn edited_chord_labels_and_executes_the_same_slot() -> TestResult {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (mut app, mut commands) = super::super::tests::fixture()?;
    let generation = {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner
            .settings
            .preferences
            .rebind("local-spell-action-bar-spell-1", "CTRL+F1")?;
        owner.generation()
    };
    app.world_mut()
        .resource_mut::<EditorState>()
        .open(0, generation);
    app.update();
    set(&mut app, 3, "SUPER+ALT+F12");
    let key = control(&mut app, |edit| matches!(edit, Edit::Key))?;
    app.world_mut().trigger(Activate { entity: key });
    let close = control(&mut app, |edit| matches!(edit, Edit::Close))?;
    app.world_mut().trigger(Activate { entity: close });
    app.update();
    assert!(
        app.world_mut()
            .query::<(&super::super::SlotHotkey, &Text)>()
            .iter(app.world())
            .any(|(slot, text)| slot.0 == 0 && text.0 == "M+A+F12")
    );
    for (key, modifiers, casts) in [
        (58, Modifiers::one(Modifier::Control), false),
        (
            69,
            Modifiers::one(Modifier::Super).with(Modifier::Alt),
            true,
        ),
    ] {
        let code = KeyCode::new(key)?;
        let events =
            [ButtonState::Pressed, ButtonState::Released].map(|state| NormalizedInputEvent::Key {
                code,
                state,
                modifiers,
                repeat: false,
            });
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.route_input(&events, false);
        let play = owner.play.as_mut().ok_or("play")?;
        assert_eq!(play.view.tick(&play.link), Ok(()));
        assert_eq!(commands.try_recv().ok().is_some_and(|command| matches!(command, crate::combat_input::PlayCommand::Combat(crate::combat_input::CombatCommand::Cast(id)) if id.get()==71)), casts);
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
