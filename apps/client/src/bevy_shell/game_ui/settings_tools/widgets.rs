//! Source-skinned projection of retained settings values and transient local drafts.
use super::*;
pub(super) fn label(commands: &mut Commands, skins: &Skins, parent: Entity, text: &str) {
    let child = commands
        .spawn(source::text(text, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(parent).add_child(child);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    bound: &Admission,
    kind: Kind,
    text: &str,
) {
    let child = commands
        .spawn((
            source::button(skins.buttons.clone(), 240.),
            Control {
                generation: bound.generation,
                revision: bound.revision,
                kind,
            },
        ))
        .observe(activate)
        .id();
    label(commands, skins, child, text);
    commands.entity(parent).add_child(child);
}
fn field(commands: &mut Commands, skins: &Skins, parent: Entity, folder: bool, value: &str) {
    let mut edit = EditableText::new(value);
    edit.max_characters = Some(1024);
    edit.allow_newlines = false;
    let child = commands
        .spawn((
            Field(folder),
            TextInput,
            edit,
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::default(),
            TextCursorStyle::default(),
            bevy::input_focus::tab_navigation::TabIndex(0),
            Node {
                min_height: px(26.),
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(child);
}
pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
) {
    if !visible(&owner) {
        if let Some(root) = state.root.take() {
            commands.entity(root).despawn();
        }
        state.bound = None;
        state.snapshot = None;
        state.page = None;
        return;
    }
    let snapshot = Snapshot::read(&owner.settings);
    let generation = owner.generation();
    let changed = state.snapshot.as_ref() != Some(&snapshot)
        || state
            .bound
            .as_ref()
            .is_none_or(|b| b.generation != generation);
    if changed {
        if state
            .snapshot
            .as_ref()
            .is_none_or(|old| old.preferences != snapshot.preferences)
        {
            let path = state.draft.path.clone();
            state.draft = Draft::read(&snapshot.preferences);
            state.draft.path = path;
        }
        state.confirm = None;
        if state
            .bound
            .as_ref()
            .is_none_or(|b| b.generation != generation)
        {
            state.bound = Some(Admission::new(generation));
        } else if let Some(bound) = &mut state.bound {
            bound.revision = bound.revision.saturating_add(1);
        }
        state.snapshot = Some(snapshot);
    }
    if !changed && !state.dirty && state.page == Some(owner.settings.page) {
        return;
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    state.page = Some(owner.settings.page);
    state.dirty = false;
    let Some(bound) = state.bound.as_ref() else {
        return;
    };
    let root = commands
        .spawn((
            GameModal,
            Name::new("settings_tools.window"),
            GlobalZIndex(120),
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            bevy::ui_widgets::ScrollArea,
            Node {
                position_type: PositionType::Absolute,
                left: percent(15.),
                top: percent(5.),
                width: percent(70.),
                max_height: percent(90.),
                overflow: Overflow::scroll_y(),
                padding: px(20.).all(),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                ..default()
            },
        ))
        .id();
    source::dialog_caption(&mut commands, root, "Settings tools", skins.font.clone());
    label(&mut commands, &skins, root, owner.settings.scope_label());
    for page in PAGES {
        button(
            &mut commands,
            &skins,
            root,
            bound,
            Kind::Page(page),
            page.label(),
        );
    }
    match owner.settings.page {
        SettingsPage::Help | SettingsPage::MinimapTransfer => {
            label(&mut commands, &skins, root, "Portable file path");
            field(&mut commands, &skins, root, false, &state.draft.path);
            button(
                &mut commands,
                &skins,
                root,
                bound,
                Kind::Browse,
                "Browse directory",
            );
            for entry in &state.entries {
                button(
                    &mut commands,
                    &skins,
                    root,
                    bound,
                    Kind::Path(entry.clone()),
                    &entry.display().to_string(),
                );
            }
            let actions: &[(Action, &str)] = if owner.settings.page == SettingsPage::Help {
                &[
                    (Action::Import, "Import settings"),
                    (Action::Export, "Export new file"),
                    (Action::Backup, "Restore backup"),
                    (Action::Defaults, "Restore defaults"),
                ]
            } else {
                &[
                    (Action::MinimapImport, "Import and replace"),
                    (Action::MinimapExport, "Export new file"),
                ]
            };
            if owner.settings.page == SettingsPage::MinimapTransfer {
                label(
                    &mut commands,
                    &skins,
                    root,
                    "Import replaces this world's exploration and markers. Export first to keep a copy. Files must match the current world and content generation. Join a world before transferring.",
                );
            }
            for (a, text) in actions {
                button(&mut commands, &skins, root, bound, Kind::Action(*a), text);
            }
        }
        SettingsPage::Language => {
            for (index, language) in Language::ALL.iter().enumerate() {
                button(
                    &mut commands,
                    &skins,
                    root,
                    bound,
                    Kind::Language(index),
                    &format!(
                        "{}{}",
                        if index == state.draft.language {
                            "✓ "
                        } else {
                            ""
                        },
                        language.name()
                    ),
                );
            }
            label(
                &mut commands,
                &skins,
                root,
                "Restart the client to apply this language to all windows.",
            );
            button(
                &mut commands,
                &skins,
                root,
                bound,
                Kind::Action(Action::SaveLanguage),
                "Save language",
            );
        }
        SettingsPage::Screenshots => {
            label(
                &mut commands,
                &skins,
                root,
                "Screenshot folder (BMP images)",
            );
            field(&mut commands, &skins, root, true, &state.draft.folder);
            label(
                &mut commands,
                &skins,
                root,
                "Pre-event frames (0–10 seconds, max 64 MiB)",
            );
            field(&mut commands, &skins, root, false, &state.draft.backlog);
            for (index, caption) in [
                "Game only (off: entire game window)",
                "Capture on death",
                "Capture on level advance",
                "Capture on trained skill advance",
                "Capture on achievement",
                "Backlog in seconds (off: frames)",
            ]
            .iter()
            .enumerate()
            {
                button(
                    &mut commands,
                    &skins,
                    root,
                    bound,
                    Kind::Flag(index),
                    &format!(
                        "{} {caption}",
                        if state.draft.flags[index] {
                            "✓"
                        } else {
                            "□"
                        }
                    ),
                );
            }
            button(
                &mut commands,
                &skins,
                root,
                bound,
                Kind::Action(Action::SaveScreenshot),
                "Save screenshot settings",
            );
            button(
                &mut commands,
                &skins,
                root,
                bound,
                Kind::Action(Action::Capture),
                "Capture now",
            );
        }
        SettingsPage::HelpGuide => {
            for (index, topic) in crate::play::help::Topic::ALL.iter().enumerate() {
                button(
                    &mut commands,
                    &skins,
                    root,
                    bound,
                    Kind::Topic(index),
                    topic.label(),
                );
            }
            for (index, caption) in [
                "Frequently asked questions (Help window)",
                "Playing manual",
                "Client help and troubleshooting",
            ]
            .iter()
            .enumerate()
            {
                button(
                    &mut commands,
                    &skins,
                    root,
                    bound,
                    Kind::LegacyTopic(index),
                    caption,
                );
            }
            if let Some(index) = state.draft.legacy_topic {
                if let Some(topic) = [
                    crate::play::HelpTopic::Questions,
                    crate::play::HelpTopic::Manual,
                    crate::play::HelpTopic::Client,
                ]
                .get(index)
                {
                    label(&mut commands, &skins, root, crate::play::help_text(*topic));
                }
            } else if let Some(topic) = crate::play::help::Topic::ALL.get(state.draft.topic) {
                label(
                    &mut commands,
                    &skins,
                    root,
                    &owner.settings.help_text(*topic),
                );
            }
            button(
                &mut commands,
                &skins,
                root,
                bound,
                Kind::Action(Action::Report),
                "Open Reports",
            );
            button(
                &mut commands,
                &skins,
                root,
                bound,
                Kind::Action(Action::BugReport),
                "Report a client problem",
            );
            button(
                &mut commands,
                &skins,
                root,
                bound,
                Kind::Action(Action::PlayerReport),
                "Report player behavior",
            );
        }
        _ => {
            button(
                &mut commands,
                &skins,
                root,
                bound,
                Kind::Action(Action::Close),
                "Close",
            );
        }
    }
    if let Some((a, draft)) = &state.confirm {
        let target = if *a == Action::Backup {
            owner.settings.settings_backup_path().map_or_else(
                || owner.settings.scope_label().into(),
                |p| p.display().to_string(),
            )
        } else if *a == Action::Defaults {
            owner.settings.scope_label().into()
        } else {
            draft.path.clone()
        };
        label(
            &mut commands,
            &skins,
            root,
            &format!(
                "Confirm {a:?}: {}. Current data will be replaced. Settings replacement saves a backup first.",
                target
            ),
        );
        button(
            &mut commands,
            &skins,
            root,
            bound,
            Kind::Confirm,
            "Confirm replacement",
        );
    }
    label(&mut commands, &skins, root, &owner.settings.feedback);
    button(
        &mut commands,
        &skins,
        root,
        bound,
        Kind::Action(Action::Close),
        "Close",
    );
    state.root = Some(root);
}
