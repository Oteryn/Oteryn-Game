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
    set(&mut app, 1, "42");
    set(&mut app, 4, "7");
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
