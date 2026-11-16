//! Minimap import and export on the Misc page: the path field and the two transfer actions
//! that the settings owner applies to the active world's minimap at once.
use super::*;
use bevy::{
    text::EditableText,
    ui_widgets::{Activate, TextInput},
};

#[derive(Component)]
pub(super) struct TransferPath;
#[derive(Component, Clone, Copy)]
pub(super) struct Transfer {
    import: bool,
}

pub(super) fn build(commands: &mut Commands, parent: Entity, skins: &Skins) {
    let label = commands
        .spawn(source::text(
            "Minimap file: import replaces this world's map; export writes a new file.",
            source::font(skins.font.clone(), false),
        ))
        .id();
    let mut text = EditableText::new("");
    text.max_characters = Some(240);
    let field = commands
        .spawn((
            TransferPath,
            TextInput,
            text,
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            bevy::text::TextCursorStyle::default(),
            bevy::input_focus::tab_navigation::TabIndex(0),
            Name::new("minimap.transfer-path"),
            Node {
                width: percent(100.),
                height: px(24.),
                ..default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
        ))
        .id();
    commands.entity(parent).add_children(&[label, field]);
    for (import, caption) in [(true, "Import and replace"), (false, "Export new file")] {
        let button = commands
            .spawn((
                source::button(skins.buttons.clone(), 140.),
                Transfer { import },
            ))
            .observe(transfer)
            .id();
        let text = commands
            .spawn(source::button_caption(caption, skins.font.clone()))
            .id();
        commands.entity(button).add_child(text);
        commands.entity(parent).add_child(button);
    }
}

fn transfer(
    event: On<Activate>,
    buttons: Query<&Transfer>,
    fields: Query<&EditableText, With<TransferPath>>,
    mut owner: NonSendMut<HostOwner>,
    state: Res<OptionsState>,
    guard: ModalGuard,
) {
    let Ok(action) = buttons.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity)
        || state.page != Page::Misc
        || state.generation != Some(owner.generation())
        || top_server_dialog(&owner)
    {
        return;
    }
    let Ok(field) = fields.single() else {
        return;
    };
    let path = std::path::PathBuf::from(field.value().to_string().trim());
    let owner = &mut *owner;
    let Some(play) = &mut owner.play else {
        owner.settings.feedback = tr!("Join a world before exporting its minimap").into();
        return;
    };
    owner.settings.request_minimap_transfer(action.import, path);
    owner.settings.feedback = match owner.settings.apply_minimap_transfer(&mut play.view) {
        Ok(_) => tr!("Minimap transfer saved").into(),
        Err(error) => lfmt!("Minimap transfer failed: {error}", error = error),
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::{PlayView, play_channel};
    #[test]
    fn misc_page_exports_and_imports_the_active_world_minimap()
    -> Result<(), Box<dyn std::error::Error>> {
        let root =
            std::env::temp_dir().join(format!("bevy-minimap-transfer-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root)?;
        let export = root.join("portable.json");
        let mut app = super::super::tests::app(Page::Misc, None)?;
        let (link, _, _) = play_channel();
        let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 200), -7, &[])?;
        view.bind_test_minimap(root.join("maps"));
        app.world_mut().non_send_mut::<HostOwner>().play =
            Some(crate::bevy_shell::owner::ActivePlay { view, link });
        let field = app
            .world_mut()
            .spawn((TransferPath, EditableText::new(export.to_string_lossy())))
            .id();
        let button = |app: &mut App, import: bool| {
            app.world_mut()
                .spawn(Transfer { import })
                .observe(transfer)
                .id()
        };
        let export_button = button(&mut app, false);
        app.world_mut().trigger(Activate {
            entity: export_button,
        });
        assert_eq!(
            app.world().non_send::<HostOwner>().settings.feedback,
            "Minimap transfer saved"
        );
        assert!(export.is_file());
        app.world_mut().trigger(Activate {
            entity: export_button,
        });
        assert!(
            app.world()
                .non_send::<HostOwner>()
                .settings
                .feedback
                .starts_with("Minimap transfer failed: "),
            "export never overwrites an existing file"
        );
        let import_button = button(&mut app, true);
        app.world_mut().trigger(Activate {
            entity: import_button,
        });
        assert_eq!(
            app.world().non_send::<HostOwner>().settings.feedback,
            "Minimap transfer saved"
        );
        app.world_mut().entity_mut(field).insert(EditableText::new(
            root.join("missing.json").to_string_lossy(),
        ));
        app.world_mut().trigger(Activate {
            entity: import_button,
        });
        assert!(
            app.world()
                .non_send::<HostOwner>()
                .settings
                .feedback
                .starts_with("Minimap transfer failed: ")
        );
        app.world_mut().non_send_mut::<HostOwner>().play = None;
        std::fs::remove_dir_all(root)?;
        Ok(())
    }
}
