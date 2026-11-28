//! Row 164: the Misc page exports and imports the preferences, restores the last backup or
//! resets to defaults. Export writes the saved options and keeps the open draft's pending edits;
//! the other actions replace the saved options and reload the draft from the result.
use super::*;
use bevy::{text::EditableText, ui_widgets::Activate};
use std::path::Path;

#[derive(Component)]
pub(super) struct PathField;
#[derive(Component, Clone, Copy)]
pub(super) enum Action {
    Export,
    Import,
    Restore,
    Defaults,
}

/// The export file next to the preferences file.
fn default_path() -> String {
    crate::play::PreferenceStore::path()
        .map(|path| path.with_extension("export.json").display().to_string())
        .unwrap_or_default()
}

pub(super) fn build(commands: &mut Commands, parent: Entity, skins: &Skins) {
    section::label(
        commands,
        parent,
        skins,
        "Options Transfer: file to export to or import from",
    );
    let field = section::field(
        commands,
        parent,
        skins,
        "misc.transfer.path",
        &default_path(),
    );
    commands.entity(field).insert(PathField);
    let bar = section::bar(commands, parent);
    for (caption, action, id) in [
        ("Export", Action::Export, "misc.transfer.export"),
        ("Import", Action::Import, "misc.transfer.import"),
        ("Restore Backup", Action::Restore, "misc.transfer.restore"),
        (
            "Reset to Defaults",
            Action::Defaults,
            "misc.transfer.defaults",
        ),
    ] {
        let button = section::action(commands, bar, skins, caption, 140., id, false);
        commands.entity(button).insert(action).observe(activate);
    }
}

fn activate(
    event: On<Activate>,
    actions: Query<&Action>,
    fields: Query<&EditableText, With<PathField>>,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<OptionsState>,
    guard: ModalGuard,
) {
    if !section::editable(&owner, &state, &guard, event.entity, Page::Misc) {
        return;
    }
    let Ok(&action) = actions.get(event.entity) else {
        return;
    };
    let path = fields
        .iter()
        .next()
        .map(|field| field.value().to_string().trim().to_owned())
        .unwrap_or_default();
    let result = (|| {
        if !matches!(action, Action::Export)
            && let Some(play) = &owner.play
        {
            play.view.require_minimap_preferences_writer()?;
        }
        let settings = &mut owner.settings;
        match action {
            Action::Export | Action::Import if path.is_empty() => {
                Err(std::io::Error::other("Enter a file path"))
            }
            Action::Export => settings.export_to(Path::new(&path)),
            Action::Import => settings.import_from(Path::new(&path)),
            Action::Restore => settings.restore_backup(),
            Action::Defaults => settings.reset_defaults(),
        }
    })();
    match result {
        Ok(()) => {
            if !matches!(action, Action::Export) && !owner.settings.preference_transfer_pending() {
                let saved = owner.settings.preferences.clone();
                owner.edit_preferences(|draft| *draft = saved);
            }
            owner.settings.feedback =
                if !matches!(action, Action::Export) && owner.settings.preference_save_pending() {
                    "Saving preferences…"
                } else {
                    match action {
                        Action::Export => "Options exported",
                        Action::Import => "Options imported and saved",
                        Action::Restore => "Backup restored and saved",
                        Action::Defaults => "Defaults restored and saved",
                    }
                }
                .into();
        }
        Err(error) => owner.settings.feedback = error.to_string(),
    }
    state.revision = state.revision.wrapping_add(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::{
        MinimapBinding, PlayView, PreferenceStore, ReferencePreferences, SettingsState,
        play_channel,
    };
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn press(app: &mut App, action: Action) {
        let button = app.world_mut().spawn(action).observe(activate).id();
        app.world_mut().trigger(Activate { entity: button });
    }
    fn device(app: &App) -> Result<(String, String), Box<dyn std::error::Error>> {
        let owner = app.world().non_send::<HostOwner>();
        Ok((
            owner
                .preferences_draft()
                .ok_or("draft")?
                .audio_device
                .clone(),
            owner.settings.preferences.audio_device.clone(),
        ))
    }

    fn read_only_options_transfer(action: Action, name: &str) -> TestResult {
        let root =
            std::env::temp_dir().join(format!("bevy-options-reader-{name}-{}", std::process::id()));
        let result = (|| -> TestResult {
            let base = root.join("settings.json");
            let character = [3; 16];
            let active = PreferenceStore::character_path(&base, character);
            std::fs::create_dir_all(active.parent().ok_or("character settings directory")?)?;
            let mut saved = ReferencePreferences::default();
            saved.minimap.binding = Some(MinimapBinding {
                world: None,
                channel: None,
                generation: [1; 32],
                bundle: [2; 32],
                epoch: 1,
            });
            saved.minimap.tiles.push((100, 200, 7, 55));
            PreferenceStore::export_to(&saved, &active)?;

            let backup = active.with_extension("backup.json");
            let portable = root.join("portable.json");
            let mut incoming = saved.clone();
            incoming.minimap.tiles.push((101, 200, 7, 55));
            PreferenceStore::export_to(&incoming, &backup)?;
            if matches!(action, Action::Import) {
                PreferenceStore::export_to(&incoming, &portable)?;
            }
            let before = std::fs::read(&active)?;
            let backup_before = std::fs::read(&backup)?;

            let mut app = super::super::tests::app(Page::Misc, Some(base))?;
            let mut writer = PlayView::new(oteryn_renderer::TileCoord::new(100, 200), 7, &[])?;
            let mut reader = PlayView::new(oteryn_renderer::TileCoord::new(100, 200), 7, &[])?;
            writer.bind_test_minimap(root.join("maps"));
            reader.bind_test_minimap(root.join("maps"));
            let (link, _, _) = play_channel();
            {
                let mut owner = app.world_mut().non_send_mut::<HostOwner>();
                owner.cancel();
                owner.settings.select_character(Some(character));
                owner.play = Some(crate::bevy_shell::owner::ActivePlay { view: reader, link });
                owner.open_graphics();
            }
            app.world_mut().spawn((
                PathField,
                EditableText::new(portable.to_str().ok_or("portable settings path")?),
            ));

            press(&mut app, action);

            let owner = app.world().non_send::<HostOwner>();
            assert!(
                std::fs::read(&backup)? == backup_before,
                "reader rewrote the backup"
            );
            assert!(
                std::fs::read(&active)? == before,
                "reader rewrote character settings"
            );
            assert_eq!(owner.settings.preferences.minimap, saved.minimap);
            assert_eq!(
                owner.preferences_draft().ok_or("options draft")?.minimap,
                saved.minimap
            );
            if matches!(action, Action::Export) {
                assert_eq!(owner.settings.feedback, "Options exported");
                assert_eq!(
                    PreferenceStore::read_transfer(&portable)?.minimap,
                    saved.minimap
                );
            } else {
                assert_eq!(
                    owner.settings.feedback,
                    "The minimap is busy in another client"
                );
            }
            drop(app);
            drop(writer);
            Ok(())
        })();
        let _ = std::fs::remove_dir_all(root);
        result
    }

    #[test]
    fn read_only_options_transfer_refuses_import() -> TestResult {
        read_only_options_transfer(Action::Import, "import")
    }

    #[test]
    fn read_only_options_transfer_refuses_restore() -> TestResult {
        read_only_options_transfer(Action::Restore, "restore")
    }

    #[test]
    fn read_only_options_transfer_refuses_reset() -> TestResult {
        read_only_options_transfer(Action::Defaults, "defaults")
    }

    #[test]
    fn read_only_options_transfer_exports_saved_exploration() -> TestResult {
        read_only_options_transfer(Action::Export, "export")
    }

    #[test]
    fn options_transfer_reports_minimap_lock_filesystem_error() -> TestResult {
        let root =
            std::env::temp_dir().join(format!("bevy-options-lock-error-{}", std::process::id()));
        let result = (|| -> TestResult {
            let maps = root.join("maps");
            let lock = maps
                .join("11111111-1111-4111-8111-111111111111")
                .join(format!("{}-{}-1.lock", "01".repeat(32), "02".repeat(32)));
            // A directory at the lock path prevents opening it even with elevated permissions.
            std::fs::create_dir_all(&lock)?;
            let error = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(&lock)
                .err()
                .ok_or("a directory opened as a writable lock file")?;
            let expected = format!("Minimap storage is read-only: {error}");

            let mut app = super::super::tests::app(Page::Misc, Some(root.join("settings.json")))?;
            let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 200), -7, &[])?;
            view.bind_test_minimap(maps);
            let (link, _, _) = play_channel();
            app.world_mut().non_send_mut::<HostOwner>().play =
                Some(crate::bevy_shell::owner::ActivePlay { view, link });

            for action in [Action::Import, Action::Restore, Action::Defaults] {
                app.world_mut()
                    .non_send_mut::<HostOwner>()
                    .settings
                    .feedback
                    .clear();
                press(&mut app, action);
                assert_eq!(
                    app.world().non_send::<HostOwner>().settings.feedback,
                    expected
                );
            }
            Ok(())
        })();
        let _ = std::fs::remove_dir_all(root);
        result
    }

    #[test]
    fn export_keeps_the_pending_draft_edits() -> TestResult {
        let fixture = tempfile::tempdir()?;
        let path = fixture.path().join("preferences.json");
        let export = fixture.path().join("export.json");
        let result = (|| -> TestResult {
            let mut app = super::super::tests::app(Page::Misc, Some(path.clone()))?;
            app.world_mut()
                .non_send_mut::<HostOwner>()
                .edit_preferences(|draft| draft.audio_device = "Pending".into());
            app.world_mut().spawn((
                PathField,
                EditableText::new(export.to_str().ok_or("export path")?),
            ));
            press(&mut app, Action::Export);
            assert_eq!(
                app.world().non_send::<HostOwner>().settings.feedback,
                "Options exported"
            );
            assert!(
                PreferenceStore::read_transfer(&export)?
                    .audio_device
                    .is_empty()
            );
            assert_eq!(device(&app)?, ("Pending".into(), String::new()));
            app.world_mut().non_send_mut::<HostOwner>().apply(true);
            assert_eq!(
                SettingsState::new(Some(path.clone()))
                    .preferences
                    .audio_device,
                "Pending"
            );
            Ok(())
        })();
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&export);
        result
    }

    fn press_built(app: &mut App, id: &str) -> TestResult {
        let world = app.world_mut();
        let button = world
            .query::<(Entity, &Name)>()
            .iter(world)
            .find_map(|(entity, name)| (name.as_str() == id).then_some(entity))
            .ok_or("production transfer button missing")?;
        world
            .resource_mut::<bevy::input_focus::InputFocus>()
            .set(button, bevy::input_focus::FocusCause::Navigated);
        world.write_message(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::Enter,
            logical_key: bevy::input::keyboard::Key::Enter,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
        app.update();
        Ok(())
    }

    fn finish_transfer(app: &mut App) -> TestResult {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        assert!(owner.settings.preference_transfer_pending());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while owner.settings.preference_transfer_pending() {
            if std::time::Instant::now() >= deadline {
                return Err("background preferences transfer did not complete".into());
            }
            owner.poll([800, 600]);
            std::thread::yield_now();
        }
        Ok(())
    }

    #[test]
    fn transfer_exports_resets_imports_and_restores_the_saved_options() -> TestResult {
        bevy_tasks::IoTaskPool::get_or_init(|| {
            bevy_tasks::TaskPoolBuilder::new().num_threads(1).build()
        });
        let fixture = tempfile::tempdir()?;
        let configured = std::env::var_os("OTERYN_G05_SETTINGS_DIR");
        let root = configured.as_deref().map_or(fixture.path(), Path::new);
        let phase = std::env::var("OTERYN_G05_PHASE");
        let character = if configured.is_some() {
            let encoded = std::env::var("OTERYN_G05_CHARACTER")?;
            let mut bytes = [0; 16];
            assert!(encoded.len() == 32 && encoded.is_ascii());
            for (index, byte) in bytes.iter_mut().enumerate() {
                *byte = u8::from_str_radix(&encoded[index * 2..index * 2 + 2], 16)?;
            }
            bytes
        } else {
            [3; 16]
        };
        let path = root.join("preferences.json");
        let export = root.join("export.json");
        let active = PreferenceStore::character_path(&path, character);
        let expected = root.join("expected-after-restore.json");
        if configured.is_some() && phase.as_deref() == Ok("reopen") {
            let expected = PreferenceStore::read_transfer(&expected)?;
            assert_eq!(expected.audio_device, "Desk");
            assert_eq!(expected.audio.master_volume, 81);
            assert_eq!(PreferenceStore::read_transfer(&active)?, expected);
            let mut owner = HostOwner::new(Some(path))?;
            install_character(&mut owner, character, root)?;
            assert_eq!(owner.settings.preferences, expected);
            return Ok(());
        }
        assert!(configured.is_none() || phase.as_deref() == Ok("transfer"));
        let mut app = super::super::tests::app(Page::Misc, Some(path.clone()))?;
        app.add_plugins((
            bevy::input::InputPlugin,
            bevy::input_focus::InputDispatchPlugin,
            bevy::ui_widgets::ButtonPlugin,
        ));
        app.init_resource::<InputFocus>();
        app.world_mut()
            .spawn((Window::default(), bevy::window::PrimaryWindow));
        {
            let mut owner = app.world_mut().non_send_mut::<HostOwner>();
            install_character(&mut owner, character, root)?;
            owner.open_graphics();
        }
        let generation = app.world().non_send::<HostOwner>().generation();
        app.world_mut().resource_mut::<OptionsState>().generation = Some(generation);
        save_device(&mut app, "Headset", 37);
        let exported = PreferenceStore::read_transfer(&active)?;
        assert_eq!(exported.audio_device, "Headset");
        assert_eq!(exported.audio.master_volume, 37);
        let world = app.world_mut();
        let parent = world.spawn_empty().id();
        let skins = Skins {
            buttons: oteryn_client_ui_bevy::ButtonImages {
                up: default(),
                down: default(),
            },
            checked: default(),
            unchecked: default(),
            frame: default(),
            inset: default(),
            font: default(),
        };
        let mut queue = bevy::ecs::world::CommandQueue::default();
        build(&mut Commands::new(&mut queue, world), parent, &skins);
        queue.apply(world);
        let field = world
            .query_filtered::<Entity, With<PathField>>()
            .single(world)?;
        let mut editor = world
            .get_mut::<EditableText>(field)
            .ok_or("production path field")?;
        editor.editor_mut().set_text("");
        press_built(&mut app, "misc.transfer.export")?;
        assert_eq!(
            app.world().non_send::<HostOwner>().settings.feedback,
            "Enter a file path"
        );
        assert!(!export.exists());

        app.world_mut()
            .get_mut::<EditableText>(field)
            .ok_or("production path field")?
            .editor_mut()
            .set_text(export.to_str().ok_or("export path")?);
        press_built(&mut app, "misc.transfer.export")?;
        assert_eq!(PreferenceStore::read_transfer(&export)?, exported);

        save_device(&mut app, "Speakers", 62);
        let reset_backup = PreferenceStore::read_transfer(&active)?;
        assert_ne!(reset_backup, exported);
        let expected_defaults = ReferencePreferences::default();
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .settings
            .use_background_preferences();
        press_built(&mut app, "misc.transfer.defaults")?;
        finish_transfer(&mut app)?;
        assert_eq!(PreferenceStore::read_transfer(&active)?, expected_defaults);
        let backup = active.with_extension("backup.json");
        assert_eq!(PreferenceStore::read_transfer(&backup)?, reset_backup);

        // Use a non-default pre-import state whose backup differs from the export.
        press_built(&mut app, "misc.transfer.restore")?;
        finish_transfer(&mut app)?;
        assert_eq!(PreferenceStore::read_transfer(&active)?, reset_backup);
        save_device(&mut app, "Desk", 81);
        {
            let mut owner = app.world_mut().non_send_mut::<HostOwner>();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while owner.settings.preference_save_pending() {
                if std::time::Instant::now() >= deadline {
                    return Err("background preference save did not complete".into());
                }
                owner.poll([800, 600]);
                std::thread::yield_now();
            }
        }
        let overwritten = PreferenceStore::read_transfer(&active)?;
        assert_eq!(overwritten.audio_device, "Desk");
        assert_eq!(overwritten.audio.master_volume, 81);
        assert_ne!(overwritten, exported);
        assert_ne!(PreferenceStore::read_transfer(&backup)?, exported);
        press_built(&mut app, "misc.transfer.import")?;
        finish_transfer(&mut app)?;
        assert_eq!(PreferenceStore::read_transfer(&active)?, exported);
        assert_eq!(PreferenceStore::read_transfer(&backup)?, overwritten);
        assert_eq!(device(&app)?, ("Headset".into(), "Headset".into()));

        press_built(&mut app, "misc.transfer.restore")?;
        finish_transfer(&mut app)?;
        assert_eq!(PreferenceStore::read_transfer(&active)?, overwritten);
        assert_eq!(device(&app)?, ("Desk".into(), "Desk".into()));
        drop(app);
        let mut reopened = HostOwner::new(Some(path))?;
        reopened.settings.select_character(Some(character));
        assert_eq!(reopened.settings.preferences, overwritten);
        if configured.is_some() {
            PreferenceStore::export_to(&overwritten, &expected)?;
        }
        Ok(())
    }

    fn install_character(owner: &mut HostOwner, character: [u8; 16], root: &Path) -> TestResult {
        let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 200), 7, &[])?;
        view.set_preference_character(character);
        view.bind_test_minimap(root.join("maps"));
        let (link, _, _) = play_channel();
        assert!(owner.install_play(view, link));
        assert_eq!(owner.settings.character_scope(), Some(character));
        Ok(())
    }

    fn save_device(app: &mut App, device: &str, volume: u8) {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.edit_preferences(|draft| {
            draft.audio_device = device.into();
            draft.audio.master_volume = volume;
        });
        owner.apply(false);
    }
}
