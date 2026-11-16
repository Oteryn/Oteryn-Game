//! Row 164: the Misc page exports and imports the preferences, restores the last backup or
//! resets to defaults. Each action saves at once and reloads the open draft from the result.
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
    let settings = &mut owner.settings;
    let result = match action {
        Action::Export | Action::Import if path.is_empty() => {
            Err(std::io::Error::other("Enter a file path"))
        }
        Action::Export => settings.export_to(Path::new(&path)),
        Action::Import => settings.import_from(Path::new(&path)),
        Action::Restore => settings.restore_backup(),
        Action::Defaults => settings.reset_defaults(),
    };
    match result {
        Ok(()) => {
            let saved = owner.settings.preferences.clone();
            owner.edit_preferences(|draft| *draft = saved);
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
    use crate::play::{PreferenceStore, SettingsState};
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
