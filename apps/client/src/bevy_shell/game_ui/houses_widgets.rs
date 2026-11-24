use super::*;

pub(in crate::bevy_shell::game_ui::feature_dialogs) fn build(
    commands: &mut Commands,
    skin: &Skins,
    generation: u64,
    fields: &[String; 7],
    retained: [Option<Entity>; 7],
    model: &Panel,
    message: &str,
) -> Entity {
    let root = commands
        .spawn((
            GameModal,
            crate::bevy_shell::ui::window_layout::WindowLayer("houses"),
            GlobalZIndex(165),
            TabGroup {
                order: 3,
                modal: true,
            },
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .id();
    let panel = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(540.),
                max_width: percent(96.),
                max_height: percent(94.),
                padding: UiRect {
                    top: px(24.),
                    bottom: px(10.),
                    left: px(12.),
                    right: px(12.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(panel);
    use crate::bevy_shell::game_ui::cyclopedia_navigation::{Subject, bar};
    bar(commands, skin, panel, generation, Subject::Houses);
    let title = model.page.rows.get(model.index).map_or_else(
        || "Houses — Browser, Auctions and Transfers".to_owned(),
        |row| {
            let name = if row.name.is_empty() {
                &row.house
            } else {
                &row.name
            };
            format!("Edit House List — {name}")
        },
    );
    source::dialog_caption(commands, panel, &title, skin.font.clone());
    let content = commands
        .spawn((
            ScrollArea,
            Node {
                width: percent(100.),
                min_height: px(0.),
                flex_shrink: 1.,
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                ..default()
            },
        ))
        .id();
    commands.entity(panel).add_child(content);
    label(commands, skin, content, message);
    for (index, prompt) in [
        (0, "World UUID"),
        (1, "Town key"),
        (2, "House row (1 onwards)"),
        (3, "Maximum auction bid / transfer price (gold)"),
        (4, "Enter one member UUID per line."),
        (
            5,
            "Door: x y floor / transfer date: YYYY-MM-DD HH:MM:SS UTC",
        ),
        (6, "Type CONFIRM for changes"),
    ] {
        label(commands, skin, content, prompt);
        if index == 4 {
            label(
                commands,
                skin,
                content,
                "Names cannot be resolved by this server for access lists; an empty list removes all members. For transfers, enter the recipient character name here.",
            );
        }
        if let Some(field) = retained[index] {
            commands.entity(content).add_child(field);
            continue;
        }
        let mut editable = EditableText::new(&fields[index]);
        editable.allow_newlines = index == 4;
        editable.max_characters = Some(8192);
        editable.visible_lines = Some(if index == 4 { 4. } else { 1. });
        let field = commands
            .spawn((
                Field(index),
                TextInput,
                editable,
                TabIndex(index as i32),
                source::font(skin.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                TextLayout::default(),
                TextCursorStyle::default(),
                Node {
                    width: percent(100.),
                    min_height: px(26.),
                    padding: px(4.).all(),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
            ))
            .id();
        commands.entity(content).add_child(field);
    }
    let footer = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(5.),
            row_gap: px(5.),
            ..default()
        })
        .id();
    commands.entity(panel).add_child(footer);
    for (selection, caption) in [
        (Some(0), "Query"),
        (Some(1), "Next"),
        (Some(32), "Guildhalls"),
        (Some(33), "Houses and flats"),
        (Some(34), "All buildings"),
        (Some(35), "Rented"),
        (Some(36), "Auctioned"),
        (Some(37), "All statuses"),
        (Some(38), "Sort by name"),
        (Some(39), "Sort by area"),
        (Some(2), "Bid"),
        (Some(10), "Show entrance map"),
        (Some(20), "Owned / incoming"),
        (Some(30), "Previous owned"),
        (Some(29), "Next owned"),
        (Some(21), "Review move-out"),
        (Some(22), "Review cancel move-out"),
        (Some(23), "Review transfer"),
        (Some(24), "Review cancel transfer"),
        (Some(25), "Review accept transfer"),
        (Some(26), "Review reject transfer"),
        (Some(27), "Confirm reviewed action"),
        (Some(28), "Cancel review"),
        (Some(11), "Load guests"),
        (Some(12), "Load subowners"),
        (Some(13), "Load door"),
        (Some(3), "Save guests"),
        (Some(4), "Save subowners"),
        (Some(5), "Save door"),
        (None, "Close"),
    ] {
        let button = commands
            .spawn((
                source::button(skin.buttons.clone(), 120.),
                Control {
                    generation,
                    root,
                    selection,
                },
            ))
            .observe(activate)
            .id();
        let text = commands
            .spawn(source::button_caption(caption, skin.font.clone()))
            .id();
        commands.entity(button).add_child(text);
        commands.entity(footer).add_child(button);
    }
    root
}
