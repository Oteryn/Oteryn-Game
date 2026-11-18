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
    commands
        .entity(parent)
        .add_children(&[label, field, button]);
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
