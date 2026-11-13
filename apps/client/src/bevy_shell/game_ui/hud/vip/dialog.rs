use super::*;
pub(super) fn build(commands: &mut Commands, skins: &Skins, draft: &Draft) -> (Entity, Entity) {
    let root = commands
        .spawn((
            Dialog(draft.serial),
            super::super::super::GameModal,
            GlobalZIndex(100),
            Name::new("vip.dialog"),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.45)),
        ))
        .id();
    let panel = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            ScrollArea,
            TabGroup {
                order: 5,
                modal: true,
            },
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(285.),
                max_width: percent(95.),
                max_height: percent(95.),
                overflow: Overflow::scroll_y(),
                padding: UiRect {
                    top: px(28.),
                    bottom: px(12.),
                    left: px(12.),
                    right: px(12.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(panel);
    source::dialog_caption(
        commands,
        panel,
        if draft.contact.is_some() {
            "Edit VIP"
        } else {
            "Add VIP"
        },
        skins.font.clone(),
    );
    let caption = draft
        .contact
        .as_ref()
        .map_or("Character name", |entry| entry.name.as_str());
    let title = commands
        .spawn(source::text(
            caption,
            source::font(skins.font.clone(), false),
        ))
        .id();
    commands.entity(panel).add_child(title);
    let frame = commands
        .spawn((
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                display: if draft
                    .contact
                    .as_ref()
                    .is_some_and(|entry| entry.description.is_none())
                {
                    Display::None
                } else {
                    Display::Flex
                },
                ..default()
            },
        ))
        .id();
    let field = commands
        .spawn((
            Field(draft.serial),
            TextInput,
            EditableText {
                max_characters: Some(128),
                ..EditableText::new(&draft.text)
            },
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::default(),
            TextCursorStyle::default(),
            TabIndex(0),
            Name::new("vip.text"),
            Node {
                width: percent(100.),
                height: px(if draft.contact.is_some() { 50. } else { 25. }),
                padding: UiRect::all(px(4.)),
                ..default()
            },
        ))
        .id();
    commands.entity(frame).add_child(field);
    commands.entity(panel).add_child(frame);
    let control = |command| Control {
        generation: draft.generation,
        revision: draft.revision,
        command,
    };
    if draft.contact.is_some() {
        let iconrow = commands
            .spawn(Node {
                width: percent(100.),
                column_gap: px(4.),
                ..default()
            })
            .id();
        let icon = commands
            .spawn((
                DraftLabel(Label::Icon),
                source::text("", source::font(skins.font.clone(), false)),
                Node {
                    flex_grow: 1.,
                    ..default()
                },
            ))
            .id();
        commands.entity(iconrow).add_child(icon);
        for (step, label, id) in [(-1, "‹", "vip.icon.previous"), (1, "›", "vip.icon.next")] {
            let button = button(
                commands,
                skins,
                control(Action::Icon(draft.serial, step)),
                label,
                24.,
                id,
            );
            commands.entity(iconrow).add_child(button);
        }
        if draft.icon.is_none() {
            commands.entity(iconrow).insert(Visibility::Hidden);
        }
        commands.entity(panel).add_child(iconrow);
        if draft.notify.is_some() {
            let notify = toggle(
                commands,
                skins,
                control(Action::Notify(draft.serial)),
                Label::Notify,
                "vip.notify",
                true,
            );
            commands.entity(panel).add_child(notify);
        }
        if draft.groups.is_some() {
            for group in &draft.catalog {
                let group = toggle(
                    commands,
                    skins,
                    control(Action::Group(draft.serial, group.group)),
                    Label::Group(group.group),
                    "vip.group",
                    false,
                );
                commands.entity(panel).add_child(group);
            }
        }
    }
    let feedback = commands
        .spawn((
            DraftLabel(Label::Feedback),
            source::text("", source::font(skins.font.clone(), false)),
        ))
        .insert(TextLayout::default())
        .id();
    commands.entity(panel).add_child(feedback);
    let footer = commands
        .spawn(Node {
            width: percent(100.),
            justify_content: JustifyContent::End,
            column_gap: px(4.),
            ..default()
        })
        .id();
    if let Some(contact) = &draft.contact {
        let remove = button(
            commands,
            skins,
            control(Action::Remove(contact.entry)),
            "Remove",
            60.,
            "vip.remove",
        );
        commands.entity(footer).add_child(remove);
    }
    for (label, action, id) in [
        ("OK", Action::Save(draft.serial), "vip.save"),
        ("Cancel", Action::Cancel(draft.serial), "vip.cancel"),
    ] {
        let button = button(commands, skins, control(action), label, 55., id);
        if matches!(action, Action::Save(_))
            && draft
                .contact
                .as_ref()
                .is_some_and(|entry| !entry.editable())
        {
            commands.entity(button).insert(InteractionDisabled);
        }
        commands.entity(footer).add_child(button);
    }
    commands.entity(panel).add_child(footer);
    (root, field)
}

fn toggle(
    commands: &mut Commands,
    skins: &Skins,
    control: Control,
    label: Label,
    id: &str,
    framed: bool,
) -> Entity {
    let mut row = source::checkbox_row(framed);
    row.width = percent(100.);
    let entity = commands
        .spawn((
            source::checkbox(row),
            control,
            Toggle(label),
            Name::new(id.to_owned()),
        ))
        .observe(input::activate)
        .observe(input::toggle)
        .id();
    if framed {
        commands
            .entity(entity)
            .insert(source::frame(skins.inset.clone(), [1.; 4]));
    }
    let indicator = commands
        .spawn((
            ToggleIndicator(label),
            source::checkbox_indicator(skins.unchecked.clone()),
        ))
        .id();
    let caption = commands
        .spawn((
            DraftLabel(label),
            source::text("", source::font(skins.font.clone(), false)),
            Node {
                min_width: px(0.),
                flex_grow: 1.,
                flex_shrink: 1.,
                flex_basis: px(0.),
                ..default()
            },
        ))
        .id();
    commands.entity(entity).add_children(&[indicator, caption]);
    entity
}
