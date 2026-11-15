use super::*;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::TextCursorStyle,
    ui::InteractionDisabled,
    ui_widgets::{ScrollArea, TextInput},
};
pub(super) fn build(
    commands: &mut Commands,
    skins: &Skins,
    state: &State,
    panel: &StashPanel,
    focus: &mut InputFocus,
) -> Entity {
    let overlay = commands
        .spawn((
            GameModal,
            GlobalZIndex(166),
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
            BackgroundColor(Color::srgba(0., 0., 0., 0.45)),
        ))
        .id();
    let root = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(620.),
                max_width: percent(95.),
                max_height: percent(95.),
                padding: UiRect {
                    top: px(24.),
                    left: px(12.),
                    right: px(12.),
                    bottom: px(12.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                ..default()
            },
        ))
        .id();
    commands.entity(overlay).add_child(root);
    source::dialog_caption(commands, root, "Supply Stash — Ctrl+S", skins.font.clone());
    if let Some(amount) = &state.amount {
        if let Some(index) = visible_rows(panel).get(amount.row) {
            label(
                commands,
                skins,
                root,
                format!("Withdraw {}", panel.rows[*index].definition_key),
            );
        }
        label(commands, skins, root, format!("Amount (1–{})", amount.max));
        let mut text = EditableText::new(amount.value.to_string());
        text.max_characters = Some(10);
        let field = commands
            .spawn((
                Quantity,
                TextInput,
                text,
                source::font(skins.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                TextLayout::no_wrap(),
                TextCursorStyle::default(),
                TabIndex(0),
                source::frame(skins.inset.clone(), [1.; 4]),
                Node {
                    width: px(150.),
                    height: px(25.),
                    padding: px(4.).all(),
                    ..default()
                },
            ))
            .id();
        commands.entity(root).add_child(field);
        focus.set(field, bevy::input_focus::FocusCause::Navigated);
        label(
            commands,
            skins,
            root,
            "Up/Down: ±10 · PageUp: maximum · PageDown: minimum",
        );
        button(
            commands,
            skins,
            root,
            state,
            Kind::Confirm,
            "Withdraw (Enter)",
            true,
        );
        button(
            commands,
            skins,
            root,
            state,
            Kind::Cancel,
            "Cancel (Esc)",
            true,
        );
    } else {
        for (field, caption, value) in [
            (Field::Search, "Search", &panel.query),
            (Field::Category, "Category", &panel.category),
            (Field::Trader, "Trader", &panel.trader),
        ] {
            label(commands, skins, root, caption);
            let mut text = EditableText::new(value);
            text.max_characters = Some(128);
            let entity = commands
                .spawn((
                    field,
                    TextInput,
                    text,
                    source::font(skins.font.clone(), false),
                    TextColor(source::TEXT_COLOR),
                    TextLayout::no_wrap(),
                    TextCursorStyle::default(),
                    TabIndex(0),
                    source::frame(skins.inset.clone(), [1.; 4]),
                    Node {
                        width: percent(100.),
                        height: px(25.),
                        padding: px(4.).all(),
                        flex_shrink: 0.,
                        ..default()
                    },
                ))
                .id();
            commands.entity(root).add_child(entity);
        }
        let enabled = panel.selected && !panel.pending;
        let filters = line(commands, root);
        for (kind, caption) in [
            (Kind::Filter, "Apply filters".to_owned()),
            (Kind::Sort, format!("Sort: {:?}", panel.sort)),
        ] {
            button(commands, skins, filters, state, kind, &caption, enabled);
        }
        let list = commands
            .spawn((
                ScrollArea,
                crate::bevy_shell::game_ui::install::RememberScroll("stash", 0),
                Node {
                    height: px(230.),
                    min_height: px(0.),
                    overflow: Overflow::scroll_y(),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(3.),
                    ..default()
                },
            ))
            .id();
        commands.entity(root).add_child(list);
        let rows = visible_rows(panel);
        if rows.is_empty() {
            label(commands, skins, list, "No matching items");
        }
        for (visible, index) in rows.into_iter().enumerate() {
            let row = &panel.rows[index];
            // Definition keys are the available names; no appearance binding is sent.
            let entity = button(
                commands,
                skins,
                list,
                state,
                Kind::Row(visible),
                &oteryn_client_ui::stash::row_label(row),
                enabled && row.quantity > 0,
            );
            commands.entity(entity).insert(Row);
        }
        label(
            commands,
            skins,
            root,
            format!("Page {}", panel.page.saturating_add(1)),
        );
        let footer = line(commands, root);
        for (kind, caption, enabled) in [
            (Kind::Previous, "Previous", enabled && panel.page > 0),
            (Kind::Next, "Next", enabled && panel.has_more),
            (Kind::Refresh, "Refresh", enabled),
            (Kind::Close, "Close (Esc)", true),
        ] {
            button(commands, skins, footer, state, kind, caption, enabled);
        }
    }
    if !panel.selected {
        label(
            commands,
            skins,
            root,
            "Stash is unavailable in this session",
        );
    }
    if panel.pending {
        label(commands, skins, root, "Waiting for the server…");
    }
    if let Some(feedback) = &panel.feedback {
        label(commands, skins, root, feedback);
    }
    overlay
}
fn label(commands: &mut Commands, skins: &Skins, parent: Entity, value: impl Into<String>) {
    let text = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(parent).add_child(text);
}
// Every button carries the exact displayed session and projection revision.
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    state: &State,
    kind: Kind,
    caption: &str,
    enabled: bool,
) -> Entity {
    let mut entity = commands.spawn((
        source::button(skins.buttons.clone(), 130.),
        Control {
            generation: state.generation,
            revision: state.revision,
            kind,
        },
    ));
    if !enabled {
        entity.insert(InteractionDisabled);
    }
    let entity = entity.observe(activate).id();
    if matches!(kind, Kind::Row(_)) {
        commands.entity(entity).insert(Node {
            width: percent(100.),
            min_height: px(28.),
            height: Val::Auto,
            padding: px(4.).all(),
            flex_shrink: 0.,
            align_items: AlignItems::Center,
            ..default()
        });
    }
    let text = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    if matches!(kind, Kind::Row(_)) {
        commands.entity(text).insert(Node {
            max_width: percent(100.),
            flex_shrink: 1.,
            ..default()
        });
    }
    commands.entity(entity).add_child(text);
    commands.entity(parent).add_child(entity);
    entity
}

fn line(commands: &mut Commands, parent: Entity) -> Entity {
    let entity = commands
        .spawn(Node {
            column_gap: px(5.),
            flex_wrap: FlexWrap::Wrap,
            ..default()
        })
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
