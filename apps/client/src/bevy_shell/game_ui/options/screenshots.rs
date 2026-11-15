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
        let root = std::env::temp_dir().join(format!("gap60-settings-{}", std::process::id()));
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
        std::fs::remove_dir_all(root)?;
        Ok(())
    }
}
