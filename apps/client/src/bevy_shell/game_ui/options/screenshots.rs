//! Screenshot destination edits use the same scoped Apply/OK/Cancel owner as other options.
use super::*;
use bevy::{
    text::EditableText,
    ui_widgets::{Activate, TextInput},
};

#[derive(Component)]
pub(super) struct Folder(String);

pub(super) fn build(
    commands: &mut Commands,
    parent: Entity,
    skins: &Skins,
    draft: &crate::play::ReferencePreferences,
) {
    let label = commands.spawn(source::text(
        "F12 saves a BMP of the window. Folder: absolute path, or empty for the default Oteryn/screenshots folder. Use folder, then Apply or OK.",
        source::font(skins.font.clone(), false),
    )).id();
    let field = commands
        .spawn((
            Folder(draft.screenshots.effective().directory),
            TextInput,
            EditableText::new(draft.screenshots.effective().directory),
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            bevy::text::TextCursorStyle::default(),
            bevy::input_focus::tab_navigation::TabIndex(0),
            Name::new("screenshots.folder"),
            Node {
                width: percent(100.),
                height: px(24.),
                ..default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
        ))
        .id();
    let button = commands
        .spawn((
            source::button(skins.buttons.clone(), 100.),
            Name::new("screenshots.use-folder"),
        ))
        .observe(use_folder)
        .id();
    let caption = commands
        .spawn(source::button_caption("Use folder", skins.font.clone()))
        .id();
    commands.entity(button).add_child(caption);
    let open = commands
        .spawn((
            source::button(skins.buttons.clone(), 180.),
            Name::new("screenshots.open-directory"),
        ))
        .observe(open_directory)
        .id();
    let caption = commands
        .spawn(source::button_caption(
            "Open screenshots directory",
            skins.font.clone(),
        ))
        .id();
    commands.entity(open).add_child(caption);
    commands
        .entity(parent)
        .add_children(&[label, field, button, open]);
}
pub(super) fn project(
    owner: NonSend<HostOwner>,
    mut fields: Query<(&mut Folder, &mut EditableText)>,
) {
    let Some(draft) = owner.preferences_draft() else {
        return;
    };
    let directory = draft.screenshots.effective().directory;
    for (mut held, mut field) in &mut fields {
        if held.0 != directory {
            field.editor_mut().set_text(&directory);
            held.0.clone_from(&directory);
        }
    }
}

#[derive(Resource)]
struct FolderOpener(fn(&std::path::Path) -> std::io::Result<()>);

fn system_open(folder: &std::path::Path) -> std::io::Result<()> {
    let status = crate::bevy_shell::diagnostics::folder_command(folder).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "file manager exited with {status}"
        )))
    }
}

fn open_directory(
    event: On<Activate>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<OptionsState>,
    guard: ModalGuard,
    opener: Option<Res<FolderOpener>>,
) {
    if !guard.allows(event.entity)
        || state.page != Page::Screenshots
        || state.generation != Some(owner.generation())
        || top_server_dialog(&owner)
    {
        return;
    }
    let configured = owner.settings.preferences.screenshots.effective().directory;
    let open: fn(&std::path::Path) -> std::io::Result<()> = opener.map_or(system_open, |o| o.0);
    if state
        .opening_folder
        .as_ref()
        .is_some_and(|task| !task.is_finished())
    {
        return;
    }
    match std::thread::Builder::new()
        .name("screenshots-folder".into())
        .spawn(move || {
            let result =
                crate::play::screenshot::screenshot_directory(std::path::Path::new(&configured))
                    .and_then(|directory| {
                        crate::play::screenshot::ensure_screenshot_directory(&directory)?;
                        open(&directory)
                    });
            if let Err(error) = result {
                warn!("Open screenshots directory: {error}");
            }
        }) {
        Ok(task) => state.opening_folder = Some(task),
        Err(error) => warn!("Open screenshots directory: {error}"),
    }
}

fn use_folder(
    event: On<Activate>,
    fields: Query<&EditableText, With<Folder>>,
    mut owner: NonSendMut<HostOwner>,
    state: Res<OptionsState>,
    guard: ModalGuard,
) {
    if !guard.allows(event.entity)
        || state.page != Page::Screenshots
        || state.generation != Some(owner.generation())
        || top_server_dialog(&owner)
    {
        return;
    }
    let Ok(field) = fields.single() else {
        return;
    };
    owner.edit_preferences(|draft| {
        draft.screenshots.directory = field.value().to_string();
        draft.screenshots.folder = field.value().to_string();
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn folder_action_uses_applied_capture_directory() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = tempfile::tempdir()?;
        let folder = fixture.path().join("captures with spaces");
        let mut app = super::super::tests::app(Page::Screenshots, None)?;
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .settings
            .preferences
            .screenshots
            .directory = folder.to_string_lossy().into_owned();
        app.insert_resource(FolderOpener(|directory| {
            use std::io::Write;
            let mut file = std::fs::File::options()
                .append(true)
                .create(true)
                .open(directory.join("opened"))?;
            file.write_all(b"1")?;
            std::thread::sleep(std::time::Duration::from_millis(100));
            Ok(())
        }));
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Font>()
            .init_asset::<Image>()
            .add_systems(
                Startup,
                (
                    crate::bevy_shell::ui::setup,
                    |mut commands: Commands, skins: Res<Skins>, owner: NonSend<HostOwner>| {
                        let parent = commands.spawn(Node::default()).id();
                        build(&mut commands, parent, &skins, &owner.settings.preferences);
                    },
                )
                    .chain(),
            );
        app.update();
        let button = app
            .world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .find_map(|(e, name)| (name.as_str() == "screenshots.open-directory").then_some(e))
            .ok_or("installed folder control")?;
        app.world_mut().trigger(Activate { entity: button });
        app.world_mut().trigger(Activate { entity: button });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !folder.join("opened").exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        std::thread::sleep(std::time::Duration::from_millis(150));
        assert_eq!(std::fs::read(folder.join("opened"))?, b"1");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(folder.metadata()?.permissions().mode() & 0o077, 0);
        }
        Ok(())
    }
    #[test]
    fn destination_uses_draft_cancel_and_persisted_apply() -> Result<(), Box<dyn std::error::Error>>
    {
        let fixture = tempfile::tempdir()?;
        let root = fixture.path().join("gap60-settings");
        let path = root.join("preferences.json");
        let folder = root.join("captures").to_string_lossy().into_owned();
        let mut app = super::super::tests::app(Page::Screenshots, Some(path.clone()))?;
        app.world_mut()
            .spawn((Folder(String::new()), EditableText::new(&folder)));
        let button = app.world_mut().spawn_empty().observe(use_folder).id();
        app.world_mut().trigger(Activate { entity: button });
        let owner = app.world().non_send::<HostOwner>();
        assert_eq!(
            owner
                .preferences_draft()
                .ok_or("draft")?
                .screenshots
                .directory,
            folder
        );
        assert!(owner.settings.preferences.screenshots.directory.is_empty());
        app.world_mut().non_send_mut::<HostOwner>().cancel();
        app.world_mut().non_send_mut::<HostOwner>().open_graphics();
        assert!(
            app.world()
                .non_send::<HostOwner>()
                .preferences_draft()
                .ok_or("draft")?
                .screenshots
                .directory
                .is_empty()
        );
        app.world_mut().trigger(Activate { entity: button });
        app.world_mut().non_send_mut::<HostOwner>().apply(true);
        assert_eq!(
            crate::play::SettingsState::new(Some(path))
                .preferences
                .screenshots
                .directory,
            folder
        );
        Ok(())
    }
}

#[cfg(test)]
mod automatic_tests {
    use super::*;
    use crate::play::screenshot::CaptureAction;
    use bevy::ui_widgets::ValueChange;

    #[test]
    fn low_health_crossing_requests_one_capture() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = super::super::tests::app(Page::Screenshots, None)?;
        let checkbox = app
            .world_mut()
            .spawn(Control::Check("screenshots.low-health"))
            .observe(super::super::toggle)
            .id();
        app.world_mut().trigger(ValueChange {
            source: checkbox,
            value: true,
            is_final: true,
        });
        let (link, _, outbox) = crate::play::play_channel();
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.apply(true);
        for (seq, health, save) in [(1, 100, false), (2, 4, true), (3, 1, false), (4, 0, false)] {
            outbox.push(vec![oteryn_session::SessionEvent::ActorVitals(
                oteryn_session::AppliedDelta {
                    server_sequence: seq,
                    base_revision: seq - 1,
                    new_revision: seq,
                    value: oteryn_session::ActorVitals {
                        health,
                        max_health: 100,
                        ..default()
                    },
                },
            )]);
            assert_eq!(
                matches!(
                    owner
                        .settings
                        .prepare_screenshots(std::time::Instant::now(), Some(&link)),
                    CaptureAction::Save
                ),
                save
            );
            owner.settings.cancel_screenshot();
        }
        Ok(())
    }
    #[test]
    fn master_switch_preserves_death_selection_and_suppresses_capture()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut app = super::super::tests::app(Page::Screenshots, None)?;
        {
            let mut owner = app.world_mut().non_send_mut::<HostOwner>();
            owner.settings.preferences.screenshots.on_death = true;
            owner.cancel();
            owner.open_graphics();
        }
        let checkbox = app
            .world_mut()
            .spawn(Control::Check("screenshots.automatic"))
            .observe(super::super::toggle)
            .id();
        app.world_mut().trigger(ValueChange {
            source: checkbox,
            value: false,
            is_final: true,
        });
        let (link, _, outbox) = crate::play::play_channel();
        let now = std::time::Instant::now();
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        assert!(owner.settings.preferences.screenshots.on_death);
        owner.apply(true);
        assert!(owner.settings.preferences.screenshots.on_death);
        for (seq, health) in [(1, 100), (2, 0)] {
            outbox.push(vec![oteryn_session::SessionEvent::ActorVitals(
                oteryn_session::AppliedDelta {
                    server_sequence: seq,
                    base_revision: seq - 1,
                    new_revision: seq,
                    value: oteryn_session::ActorVitals {
                        health,
                        max_health: 100,
                        ..default()
                    },
                },
            )]);
            assert!(matches!(
                owner.settings.prepare_screenshots(now, Some(&link)),
                CaptureAction::Idle
            ));
        }
        Ok(())
    }
    #[test]
    fn unchecking_death_capture_and_applying_prevents_a_death_capture()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut app = super::super::tests::app(Page::Screenshots, None)?;
        {
            let mut owner = app.world_mut().non_send_mut::<HostOwner>();
            owner.settings.preferences.screenshots.on_death = true;
            owner.cancel();
            owner.open_graphics();
        }
        let checkbox = app
            .world_mut()
            .spawn(Control::Check("screenshots.death"))
            .observe(super::super::toggle)
            .id();
        app.world_mut().trigger(ValueChange {
            source: checkbox,
            value: false,
            is_final: true,
        });
        let (link, _, outbox) = crate::play::play_channel();
        let now = std::time::Instant::now();
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        assert!(owner.settings.preferences.screenshots.on_death);
        owner.apply(true);
        for (seq, health) in [(1, 100), (2, 0)] {
            outbox.push(vec![oteryn_session::SessionEvent::ActorVitals(
                oteryn_session::AppliedDelta {
                    server_sequence: seq,
                    base_revision: seq - 1,
                    new_revision: seq,
                    value: oteryn_session::ActorVitals {
                        health,
                        max_health: 100,
                        ..default()
                    },
                },
            )]);
            assert!(matches!(
                owner.settings.prepare_screenshots(now, Some(&link)),
                CaptureAction::Idle
            ));
        }
        Ok(())
    }
}
