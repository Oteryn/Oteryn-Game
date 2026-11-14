use super::*;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    ui::InteractionDisabled,
    ui_widgets::{ScrollArea, TextInput},
};
use oteryn_client_ui_bevy as source;

pub(super) fn build(
    commands: &mut Commands,
    skins: &Skins,
    generation: u64,
    serial: u64,
    snapshot: &Snapshot,
) -> Entity {
    let root = commands
        .spawn((
            GameModal,
            GlobalZIndex(165),
            TabGroup {
                order: 2,
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
            BackgroundColor(Color::srgba(0., 0., 0., 0.45)),
        ))
        .id();
    let panel = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(760.),
                max_width: percent(95.),
                max_height: percent(90.),
                padding: UiRect::axes(px(12.), px(24.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollArea,
        ))
        .id();
    commands.entity(root).add_child(panel);
    let title = match snapshot {
        Snapshot::Highscores(_) => "Highscores",
        Snapshot::Character(_) => "Character information",
    };
    source::dialog_caption(commands, panel, title, skins.font.clone());
    let control = |kind| Control {
        generation,
        serial,
        kind,
    };
    match snapshot {
        Snapshot::Highscores(p) => {
            let enabled = p.selected && !p.pending;
            let row = controls_row(commands, panel);
            let categories = oteryn_client_ui::highscores_loyalty::categories(p.loyalty_selected);
            let index = categories
                .iter()
                .position(|(id, _)| *id == p.category)
                .unwrap_or(0);
            let (category, label) = categories[(index + 1) % categories.len()];
            button(
                commands,
                skins,
                row,
                &format!("Category: {} (next)", categories[index].1),
                control(Kind::Highscores(Action::Filter {
                    category,
                    vocation: p.vocation,
                })),
                enabled,
            );
            button(
                commands,
                skins,
                row,
                &format!(
                    "Vocation: {} (next)",
                    highscores::VOCATIONS[p.vocation as usize]
                ),
                control(Kind::Highscores(Action::Filter {
                    category: p.category,
                    vocation: (p.vocation + 1) % 6,
                })),
                enabled,
            );
            // The next label is descriptive metadata; rows below remain server-owned.
            text(commands, skins, panel, &format!("Next category: {label}"));
            text(commands, skins, panel, "World UUID (blank = current World)");
            field(
                commands,
                skins,
                panel,
                0,
                &p.world.map(highscores::world_text).unwrap_or_default(),
                enabled,
            );
            let row = controls_row(commands, panel);
            for (caption, kind, allowed) in [
                ("Select World", Kind::World, enabled),
                ("Refresh", Kind::Highscores(Action::Refresh), enabled),
                (
                    "Previous",
                    Kind::Highscores(Action::Previous),
                    enabled && p.reply.as_ref().is_some_and(|r| r.page > 1),
                ),
                (
                    "Next",
                    Kind::Highscores(Action::Next),
                    enabled && p.reply.as_ref().is_some_and(|r| r.has_more),
                ),
                (
                    "Show me",
                    Kind::Highscores(Action::ShowMe),
                    enabled && p.reply.as_ref().is_some_and(|r| r.own_page.is_some()),
                ),
            ] {
                button(commands, skins, row, caption, control(kind), allowed);
            }
            text(commands, skins, panel, &highscores::panel_text(p));
        }
        Snapshot::Character(p) => {
            let enabled = p.selected && !p.pending;
            for (index, label) in [
                "History offset",
                "Page size (1-50)",
                "Visible character UUID",
                "Title ID (0 clears)",
            ]
            .iter()
            .enumerate()
            {
                text(commands, skins, panel, label);
                field(commands, skins, panel, index, &p.fields[index], enabled);
            }
            let row = controls_row(commands, panel);
            for (caption, intent) in [
                ("Self / history", Intent::Query),
                ("Visible", Intent::Visible),
                ("Look", Intent::Look),
                ("Unlock title", Intent::Unlock),
                ("Select title", Intent::Select),
            ] {
                button(
                    commands,
                    skins,
                    row,
                    caption,
                    control(Kind::Character(intent)),
                    enabled,
                );
            }
            text(commands, skins, panel, &character_info::panel_text(p));
            if let Some(titles) = &p.own_titles {
                button(
                    commands,
                    skins,
                    panel,
                    "Clear selected title",
                    control(Kind::Title(0)),
                    enabled,
                );
                for title in &titles.earned {
                    button(
                        commands,
                        skins,
                        panel,
                        &format!(
                            "{}{}",
                            title.name(),
                            if titles.selected == Some(*title) {
                                " (selected)"
                            } else {
                                ""
                            }
                        ),
                        control(Kind::Title(title.id())),
                        enabled,
                    );
                }
            }
        }
    }
    button(commands, skins, panel, "Close", control(Kind::Close), true);
    root
}
fn controls_row(commands: &mut Commands, parent: Entity) -> Entity {
    let row = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(4.),
            row_gap: px(4.),
            ..default()
        })
        .id();
    commands.entity(parent).add_child(row);
    row
}
fn text(commands: &mut Commands, skins: &Skins, parent: Entity, value: &str) {
    let label = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(parent).add_child(label);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    caption: &str,
    control: Control,
    enabled: bool,
) {
    let entity = commands
        .spawn((source::button(skins.buttons.clone(), 140.), control))
        .observe(activate)
        .id();
    if !enabled {
        commands.entity(entity).insert(InteractionDisabled);
    }
    let label = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(label);
    commands.entity(parent).add_child(entity);
}
fn field(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    index: usize,
    value: &str,
    enabled: bool,
) {
    let mut editable = EditableText::new(value);
    editable.allow_newlines = false;
    editable.max_characters = Some(64);
    let entity = commands
        .spawn((
            Field(index),
            TextInput,
            editable,
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::default(),
            bevy::text::TextCursorStyle::default(),
            TabIndex(0),
            Node {
                width: percent(100.),
                min_height: px(24.),
                padding: UiRect::all(px(4.)),
                ..default()
            },
            source::frame(skins.inset.clone(), [1.; 4]),
        ))
        .id();
    if !enabled {
        commands.entity(entity).insert(InteractionDisabled);
    }
    commands.entity(parent).add_child(entity);
}
