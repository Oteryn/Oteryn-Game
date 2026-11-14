use super::*;
fn label(commands: &mut Commands, skins: &Skins, parent: Entity, text: &str) {
    let child = commands
        .spawn(source::text(text, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(parent).add_child(child);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    revision: Key,
    kind: Kind,
    caption: &str,
) {
    let child = commands
        .spawn((
            Control {
                key: revision,
                kind,
            },
            source::button(skins.buttons.clone(), 310.),
        ))
        .observe(activate)
        .id();
    label(commands, skins, child, caption);
    commands.entity(parent).add_child(child);
}
fn field(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    index: usize,
    value: String,
    caption: &str,
) {
    label(commands, skins, parent, caption);
    let mut editable = EditableText::new(value);
    editable.max_characters = Some(if index == 0 { 32 } else { 128 });
    editable.allow_newlines = false;
    let child = commands
        .spawn((
            Field(index),
            TextInput,
            editable,
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::default(),
            TextCursorStyle::default(),
            bevy::input_focus::tab_navigation::TabIndex(0),
            Node {
                min_height: px(24.),
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(child);
}
pub(super) fn build(commands: &mut Commands, skins: &Skins, owner: &HostOwner, state: &mut State) {
    let prefs = &owner.settings.preferences;
    let revision = Key {
        generation: owner.generation(),
        scope: owner.settings.character_scope(),
        revision: state.revision,
    };
    let root = commands
        .spawn((
            GameModal,
            bevy::ui_widgets::ScrollArea,
            bevy::input_focus::tab_navigation::TabGroup::new(32),
            Name::new("input_profiles.window"),
            GlobalZIndex(120),
            source::frame(skins.frame.clone(), [2., 17., 2., 2.]),
            Node {
                position_type: PositionType::Absolute,
                left: px(180.),
                top: px(40.),
                width: px(600.),
                max_height: percent(90.),
                overflow: Overflow::scroll_y(),
                padding: px(16.).all(),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.),
                ..default()
            },
        ))
        .id();
    state.root = Some(root);
    source::dialog_caption(
        commands,
        root,
        "Hotkey profiles / shortcuts",
        skins.font.clone(),
    );
    label(
        commands,
        skins,
        root,
        "The active profile applies immediately. Choose a chat context to edit its keys. The chat-on map activates while the chat window is open.",
    );
    label(commands, skins, root, owner.settings.scope_label());
    label(commands, skins, root, &owner.settings.feedback);
    button(commands, skins, root, revision, Kind::Close, "Close");
    label(
        commands,
        skins,
        root,
        &format!("Active profile: {}", prefs.hotkey_contexts.selected),
    );
    for name in prefs.hotkey_contexts.presets.keys() {
        button(
            commands,
            skins,
            root,
            revision,
            Kind::Profile(name.clone()),
            name,
        );
    }
    let name = state.confirm.as_ref().map_or_else(
        || prefs.hotkey_contexts.selected.clone(),
        |draft| draft[0].clone(),
    );
    field(commands, skins, root, 0, name, "Profile name");
    for (kind, caption) in [
        (Kind::Copy, "Copy as new"),
        (Kind::Rename, "Rename"),
        (Kind::Delete, "Delete"),
    ] {
        button(commands, skins, root, revision, kind, caption);
    }
    if state.confirm.is_some() {
        label(
            commands,
            skins,
            root,
            &format!("Delete profile \"{}\"?", prefs.hotkey_contexts.selected),
        );
        button(
            commands,
            skins,
            root,
            revision,
            Kind::ConfirmDelete,
            "Confirm delete",
        );
        button(
            commands,
            skins,
            root,
            revision,
            Kind::CancelDelete,
            "Cancel delete",
        );
    }
    label(
        commands,
        skins,
        root,
        if prefs.hotkey_contexts.chat_on {
            "Editing: Chat on"
        } else {
            "Editing: Chat off"
        },
    );
    button(
        commands,
        skins,
        root,
        revision,
        Kind::Context(false),
        "Chat off",
    );
    button(
        commands,
        skins,
        root,
        revision,
        Kind::Context(true),
        "Chat on",
    );
    let bindings = bindings();
    let binding = bindings
        .iter()
        .find(|binding| Some(binding.id) == state.selected.as_deref());
    label(
        commands,
        skins,
        root,
        &binding.map_or_else(String::new, binding_label),
    );
    let primary = state.confirm.as_ref().map_or_else(
        || binding.map_or("", |b| prefs.binding_chord(b)).into(),
        |draft| draft[1].clone(),
    );
    let secondary = state.confirm.as_ref().map_or_else(
        || {
            binding
                .and_then(|b| prefs.binding_secondary(b))
                .unwrap_or("")
                .into()
        },
        |draft| draft[2].clone(),
    );
    field(commands, skins, root, 1, primary, "Primary key");
    button(
        commands,
        skins,
        root,
        revision,
        Kind::Primary,
        "Set primary",
    );
    field(
        commands,
        skins,
        root,
        2,
        secondary,
        "Secondary (blank clears)",
    );
    button(
        commands,
        skins,
        root,
        revision,
        Kind::Secondary,
        "Set secondary",
    );
    label(commands, skins, root, "Actions");
    for binding in bindings {
        button(
            commands,
            skins,
            root,
            revision,
            Kind::Action(binding.id.into()),
            &binding_label(&binding),
        );
    }
    label(commands, skins, root, "Available buttons");
    for &(id, caption) in crate::play::SHORTCUTS {
        if !prefs.shortcuts.iter().any(|shown| shown == id) {
            button(
                commands,
                skins,
                root,
                revision,
                Kind::ShortcutAdd(id.into()),
                &format!("Add {caption}"),
            );
        }
    }
    label(commands, skins, root, "Displayed buttons (in order)");
    for (row, id) in prefs.shortcuts.iter().enumerate() {
        if let Some((_, caption)) = crate::play::SHORTCUTS
            .iter()
            .find(|(known, _)| *known == id)
        {
            label(commands, skins, root, caption);
            button(
                commands,
                skins,
                root,
                revision,
                Kind::ShortcutRemove(row),
                "Remove",
            );
            if row > 0 {
                button(
                    commands,
                    skins,
                    root,
                    revision,
                    Kind::ShortcutMove(row, row - 1),
                    "Move up",
                );
            }
            if row + 1 < prefs.shortcuts.len() {
                button(
                    commands,
                    skins,
                    root,
                    revision,
                    Kind::ShortcutMove(row, row + 1),
                    "Move down",
                );
            }
        }
    }
}
