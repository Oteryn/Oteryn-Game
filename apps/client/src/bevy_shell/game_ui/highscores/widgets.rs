use super::*;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::TextCursorStyle,
    ui_widgets::{ScrollArea, TextInput},
};
pub(super) fn frame(commands: &mut Commands, skin: &Skins, parent: Entity) -> Entity {
    let frame = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            TabGroup {
                order: 3,
                modal: true,
            },
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(1050.),
                height: px(620.),
                max_width: percent(96.),
                max_height: percent(94.),
                padding: UiRect {
                    top: px(24.),
                    bottom: px(10.),
                    left: px(10.),
                    right: px(10.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(frame);
    source::dialog_caption(
        commands,
        frame,
        "Highscores — Ctrl+Shift+F9",
        skin.font.clone(),
    );
    frame
}
#[allow(clippy::too_many_arguments)] // The session catalogue accompanies the retained panel.
pub(super) fn body(
    commands: &mut Commands,
    skin: &Skins,
    frame: Entity,
    action: Action,
    panel: &HighscoresPanel,
    worlds: &[(oteryn_client_runtime::highscores::wire::WorldId, String)],
    feedback: Option<&str>,
    now: i64,
) -> Entity {
    let root = commands
        .spawn(Node {
            width: percent(100.),
            flex_grow: 1.,
            min_height: px(0.),
            flex_direction: FlexDirection::Column,
            row_gap: px(8.),
            ..default()
        })
        .id();
    commands.entity(frame).add_child(root);
    let choices = row(commands, root);
    commands.entity(choices).insert((
        ScrollArea,
        Node {
            width: percent(100.),
            max_height: px(76.),
            flex_shrink: 0.,
            overflow: Overflow::scroll_y(),
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(6.),
            row_gap: px(4.),
            ..default()
        },
    ));
    for world in std::iter::once(None).chain(worlds.iter().map(|(id, _)| Some(*id))) {
        button(
            commands,
            skin,
            choices,
            action,
            Kind::World(world),
            &format!("World: {}", model::world_label(world, worlds)),
            240.,
            request_action(panel, Kind::World(world)).is_some(),
        );
    }
    label(
        commands,
        skin,
        root,
        &format!(
            "Selected world: {}",
            model::world_label(panel.world, worlds)
        ),
    );
    label(
        commands,
        skin,
        root,
        "World type unavailable for this session. Named worlds come from your character list.",
    );
    let filters = row(commands, root);
    label(commands, skin, filters, "Other World ID (blank = current):");
    let mut text = EditableText::new(
        panel
            .world
            .map(oteryn_client_ui::highscores::world_text)
            .unwrap_or_default(),
    );
    text.max_characters = Some(36);
    let field = commands
        .spawn((
            WorldField,
            TextInput,
            text,
            TabIndex(0),
            source::font(skin.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::no_wrap(),
            TextCursorStyle::default(),
            Node {
                width: px(310.),
                height: px(26.),
                padding: px(3.).all(),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(30, 30, 30)),
        ))
        .id();
    commands.entity(filters).add_child(field);
    button(
        commands,
        skin,
        filters,
        action,
        Kind::WorldInput,
        "Apply world",
        100.,
        panel.selected && !panel.pending,
    );
    let filters = row(commands, root);
    let category = oteryn_client_ui::highscores_loyalty::categories(panel.loyalty_selected)
        .into_iter()
        .find(|(id, _)| *id == panel.category)
        .map(|(_, label)| label)
        .unwrap_or("Unavailable");
    button(
        commands,
        skin,
        filters,
        action,
        Kind::Category,
        &format!("Category: {category}"),
        270.,
        request_action(panel, Kind::Category).is_some(),
    );
    let vocation = oteryn_client_ui::highscores::VOCATIONS
        .get(panel.vocation as usize)
        .copied()
        .unwrap_or("Unavailable");
    button(
        commands,
        skin,
        filters,
        action,
        Kind::Vocation,
        &format!("Vocation: {vocation}"),
        180.,
        request_action(panel, Kind::Vocation).is_some(),
    );
    button(
        commands,
        skin,
        filters,
        action,
        Kind::Refresh,
        "Refresh",
        90.,
        request_action(panel, Kind::Refresh).is_some(),
    );
    let scroll = commands
        .spawn((
            ScrollArea,
            Node {
                width: percent(100.),
                flex_grow: 1.,
                min_height: px(0.),
                overflow: Overflow::scroll(),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(scroll);
    table(commands, skin.font.clone(), scroll, panel, worlds);
    if !panel.selected {
        label(
            commands,
            skin,
            root,
            "Highscores unavailable in this session",
        );
    }
    if panel.pending {
        label(commands, skin, root, "Loading…");
    }
    if let Some(reply) = &panel.reply {
        label(commands, skin, root, &format!("Page {}", reply.page));
        if reply.rows.is_empty() {
            label(commands, skin, root, "No rankings on this page");
        }
        if let Some(own) = &reply.own_row {
            label(
                commands,
                skin,
                root,
                &format!(
                    "Your rank: {} — {} | {} | level {} | {} points",
                    own.rank, own.name, own.vocation, own.level, own.value
                ),
            );
        } else {
            label(
                commands,
                skin,
                root,
                "Your character is not ranked in this list",
            );
        }
    }
    let updated = label(commands, skin, root, &updated_text(panel, now));
    commands.entity(updated).insert(Updated);
    if let Some(feedback) = feedback.or(panel.feedback.as_deref()) {
        label(commands, skin, root, feedback);
    }
    let paging = row(commands, root);
    for (kind, caption) in [
        (Kind::First, "First"),
        (Kind::Previous, "Previous"),
        (Kind::Next, "Next"),
        (Kind::Last, "Last"),
        (Kind::Own, "Own rank"),
    ] {
        button(
            commands,
            skin,
            paging,
            action,
            kind,
            caption,
            90.,
            request_action(panel, kind).is_some(),
        );
    }
    button(
        commands,
        skin,
        paging,
        action,
        Kind::Close,
        "Close",
        90.,
        true,
    );
    root
}
pub(super) fn table(
    commands: &mut Commands,
    font: Handle<Font>,
    parent: Entity,
    panel: &HighscoresPanel,
    worlds: &[(oteryn_client_runtime::highscores::wire::WorldId, String)],
) {
    cells(
        commands,
        font.clone(),
        parent,
        ["Rank", "Name", "Vocation", "World", "Level", "Points"].map(String::from),
        source::TEXT_COLOR,
        Color::srgb_u8(35, 35, 35),
    );
    for row in table_rows(panel, worlds) {
        cells(
            commands,
            font.clone(),
            parent,
            row.cells,
            row.color,
            row.background,
        );
    }
}
fn cells(
    commands: &mut Commands,
    font: Handle<Font>,
    parent: Entity,
    values: [String; 6],
    color: Color,
    background: Color,
) {
    let row = commands
        .spawn((
            Node {
                width: px(1000.),
                min_height: px(24.),
                flex_shrink: 0.,
                ..default()
            },
            BackgroundColor(background),
        ))
        .id();
    commands.entity(parent).add_child(row);
    for (value, width) in values.into_iter().zip([55., 180., 150., 340., 75., 200.]) {
        let cell = commands
            .spawn(source::text(value, source::font(font.clone(), false)))
            .insert((
                TextColor(color),
                Node {
                    width: px(width),
                    padding: px(3.).all(),
                    flex_shrink: 0.,
                    ..default()
                },
            ))
            .id();
        commands.entity(row).add_child(cell);
    }
}
fn row(commands: &mut Commands, parent: Entity) -> Entity {
    let entity = commands
        .spawn(Node {
            width: percent(100.),
            column_gap: px(6.),
            row_gap: px(4.),
            flex_wrap: FlexWrap::Wrap,
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn label(commands: &mut Commands, skin: &Skins, parent: Entity, value: &str) -> Entity {
    let entity = commands
        .spawn(source::text(value, source::font(skin.font.clone(), false)))
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
#[allow(clippy::too_many_arguments)] // The widget needs the same action binding and enabled state as its caption.
fn button(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    action: Action,
    kind: Kind,
    caption: &str,
    width: f32,
    enabled: bool,
) {
    let entity = commands
        .spawn(source::button(skin.buttons.clone(), width))
        .id();
    if enabled {
        commands
            .entity(entity)
            .insert(Action { kind, ..action })
            .observe(activate);
    } else {
        commands
            .entity(entity)
            .insert(bevy::ui::InteractionDisabled);
    }
    let text = commands
        .spawn(source::button_caption(caption, skin.font.clone()))
        .id();
    commands.entity(entity).add_child(text);
    commands.entity(parent).add_child(entity);
}
