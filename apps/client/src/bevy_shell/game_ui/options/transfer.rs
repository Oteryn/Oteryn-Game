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
            if !matches!(action, Action::Export) {
                let saved = owner.settings.preferences.clone();
                owner.edit_preferences(|draft| *draft = saved);
            }
            owner.settings.feedback = match action {
                Action::Export => "Options exported",
                Action::Import => "Options imported and saved",
                Action::Restore => "Backup restored and saved",
                Action::Defaults => "Defaults restored and saved",
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
            saved.minimap.tiles.push((100, 200, -7, 55));
            PreferenceStore::export_to(&saved, &active)?;

            let backup = active.with_extension("backup.json");
            let portable = root.join("portable.json");
            let mut incoming = saved.clone();
            incoming.minimap.tiles.push((101, 200, -7, 55));
            PreferenceStore::export_to(&incoming, &backup)?;
            if matches!(action, Action::Import) {
                PreferenceStore::export_to(&incoming, &portable)?;
            }
            let before = std::fs::read(&active)?;
            let backup_before = std::fs::read(&backup)?;

            let mut app = super::super::tests::app(Page::Misc, Some(base))?;
            let mut writer = PlayView::new(oteryn_renderer::TileCoord::new(100, 200), -7, &[])?;
            let mut reader = PlayView::new(oteryn_renderer::TileCoord::new(100, 200), -7, &[])?;
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
    fn export_keeps_the_pending_draft_edits() -> TestResult {
        let id = std::process::id();
        let path = std::env::temp_dir().join(format!("bevy-options-export-draft-{id}.json"));
        let export =
            std::env::temp_dir().join(format!("bevy-options-export-draft-{id}.export.json"));
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

    #[test]
    fn transfer_exports_resets_imports_and_restores_the_saved_options() -> TestResult {
        let id = std::process::id();
        let path = std::env::temp_dir().join(format!("bevy-options-transfer-{id}.json"));
        let export = std::env::temp_dir().join(format!("bevy-options-transfer-{id}.export.json"));
        let result = (|| -> TestResult {
            let mut app = super::super::tests::app(Page::Misc, Some(path.clone()))?;
            app.world_mut()
                .non_send_mut::<HostOwner>()
                .edit_preferences(|draft| draft.audio_device = "Headset".into());
            app.world_mut().non_send_mut::<HostOwner>().apply(true);
            app.world_mut().non_send_mut::<HostOwner>().open_graphics();
            let field = app
                .world_mut()
                .spawn((PathField, EditableText::new("")))
                .id();
            press(&mut app, Action::Export);
            assert_eq!(
                app.world().non_send::<HostOwner>().settings.feedback,
                "Enter a file path"
            );
            app.world_mut()
                .get_mut::<EditableText>(field)
                .ok_or("field")?
                .editor_mut()
                .set_text(export.to_str().ok_or("export path")?);
            press(&mut app, Action::Export);
            assert_eq!(
                PreferenceStore::read_transfer(&export)?.audio_device,
                "Headset"
            );
            press(&mut app, Action::Defaults);
            assert_eq!(device(&app)?, (String::new(), String::new()));
            assert!(
                SettingsState::new(Some(path.clone()))
                    .preferences
                    .audio_device
                    .is_empty()
            );
            press(&mut app, Action::Import);
            assert_eq!(device(&app)?, ("Headset".into(), "Headset".into()));
            assert_eq!(
                SettingsState::new(Some(path.clone()))
                    .preferences
                    .audio_device,
                "Headset"
            );
            // The backup holds the options the import replaced.
            press(&mut app, Action::Restore);
            assert_eq!(device(&app)?, (String::new(), String::new()));
            assert!(
                SettingsState::new(Some(path.clone()))
                    .preferences
                    .audio_device
                    .is_empty()
            );
            Ok(())
        })();
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&export);
        let _ = std::fs::remove_file(path.with_extension("backup.json"));
        result
    }
}
