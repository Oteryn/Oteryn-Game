use super::*;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::TextCursorStyle,
    ui::InteractionDisabled,
    ui_widgets::{ScrollArea, TextInput},
};
use oteryn_client_ui_bevy as source;
pub(super) fn build(
    commands: &mut Commands,
    skins: &Skins,
    key: Key,
    snapshot: &Snapshot,
) -> Entity {
    let root = commands
        .spawn((
            super::super::GameModal,
            GlobalZIndex(170),
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
            BackgroundColor(Color::srgba(0., 0., 0., 0.4)),
        ))
        .id();
    let panel = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(600.),
                max_width: percent(95.),
                max_height: percent(90.),
                padding: UiRect::axes(px(12.), px(24.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(panel);
    let title = match snapshot {
        Snapshot::Depot(_) => "Depot",
        Snapshot::Stash(_) => "Stash",
        Snapshot::Search(_) => "Depot Search",
    };
    source::dialog_caption(commands, panel, title, skins.font.clone());
    let body = commands
        .spawn((
            ScrollArea,
            Node {
                height: px(360.),
                min_height: px(0.),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                row_gap: px(5.),
                ..default()
            },
        ))
        .id();
    commands.entity(panel).add_child(body);
    match snapshot {
        Snapshot::Depot(bound) => {
            let p = &bound.panel;
            text(commands, skins, body, depot::panel_text(p));
            let ready = p.selected && !p.pending;
            let (box_number, page) = p
                .reply
                .as_ref()
                .map_or((1, 0), |reply| (reply.box_number, reply.page));
            let navigation = row(commands, body);
            button(
                commands,
                skins,
                navigation,
                "Previous box",
                key,
                Kind::Browse {
                    box_number: box_number.saturating_sub(1),
                    page: 0,
                },
                ready && box_number > 1,
            );
            button(
                commands,
                skins,
                navigation,
                "Next box",
                key,
                Kind::Browse {
                    box_number: box_number + 1,
                    page: 0,
                },
                ready && box_number < 17,
            );
            button(
                commands,
                skins,
                navigation,
                "Refresh",
                key,
                Kind::Browse { box_number, page },
                ready,
            );
            let navigation = row(commands, body);
            button(
                commands,
                skins,
                navigation,
                "Previous page",
                key,
                Kind::Browse {
                    box_number,
                    page: page.saturating_sub(1),
                },
                ready && page > 0,
            );
            button(
                commands,
                skins,
                navigation,
                "Next page",
                key,
                Kind::Browse {
                    box_number,
                    page: page + 1,
                },
                ready
                    && p.reply
                        .as_ref()
                        .is_some_and(|r| (r.page + 1) * 32 < r.total),
            );
            if let Some(reply) = &p.reply {
                for (index, entry) in reply.entries.iter().enumerate() {
                    button(
                        commands,
                        skins,
                        body,
                        &format!("Retrieve item {} × {}", entry.definition, entry.count),
                        key,
                        Kind::Retrieve(index),
                        ready,
                    );
                }
            }
            text(commands, skins, body, "Carried items — deposit whole stack");
            for item in depot::deposit_items(p) {
                button(
                    commands,
                    skins,
                    body,
                    &format!("Deposit item {} × {}", item.item_definition_ref, item.count),
                    key,
                    Kind::Deposit(item.handle),
                    ready && p.reply.as_ref().is_some_and(|r| r.town.is_some()),
                );
            }
        }
        Snapshot::Stash(bound) => {
            let p = &bound.panel;
            let ready = p.selected && !p.pending;
            field(commands, skins, body, key.window, &p.query);
            let navigation = row(commands, body);
            button(
                commands,
                skins,
                navigation,
                "Search",
                key,
                Kind::Filter,
                ready,
            );
            button(
                commands,
                skins,
                navigation,
                "Refresh",
                key,
                Kind::Refresh,
                ready,
            );
            button(
                commands,
                skins,
                navigation,
                "Previous",
                key,
                Kind::Previous,
                ready && p.page > 0,
            );
            button(
                commands,
                skins,
                navigation,
                "Next",
                key,
                Kind::Next,
                ready && p.has_more,
            );
            text(commands, skins, body, stash::panel_text(p));
            for (index, entry) in p.rows.iter().enumerate() {
                button(
                    commands,
                    skins,
                    body,
                    &format!("Withdraw {}…", stash::row_label(entry)),
                    key,
                    Kind::Withdraw(index),
                    ready && entry.quantity > 0 && !entry.migration_required,
                );
            }
        }
        Snapshot::Search(bound) => {
            let p = &bound.panel;
            field(commands, skins, body, key.window, &p.item_id.to_string());
            button(
                commands,
                skins,
                body,
                "Search item id",
                key,
                Kind::Search,
                p.selected && !p.pending,
            );
            text(commands, skins, body, depot_search::panel_text(p));
            if p.pending {
                text(commands, skins, body, "Waiting for server");
            }
        }
    }
    button(commands, skins, panel, "Close", key, Kind::Close, true);
    root
}
fn row(commands: &mut Commands, parent: Entity) -> Entity {
    let entity = commands
        .spawn(Node {
            column_gap: px(4.),
            ..default()
        })
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn text(commands: &mut Commands, skins: &Skins, parent: Entity, text: impl Into<String>) {
    let entity = commands
        .spawn(source::text(
            text.into().replace('\r', ""),
            source::font(skins.font.clone(), false),
        ))
        .insert(TextLayout::default())
        .id();
    commands.entity(parent).add_child(entity);
}
#[allow(clippy::too_many_arguments)] // Each button binds its displayed owner snapshot through the window key.
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    caption: &str,
    key: Key,
    kind: Kind,
    enabled: bool,
) {
    let mut entity = commands.spawn((
        source::button(skins.buttons.clone(), 120.),
        Control { key, kind },
    ));
    if let Kind::Withdraw(index) = kind {
        entity.insert(Name::new(format!("stash.withdraw.{index}")));
    }
    if !enabled {
        entity.insert(InteractionDisabled);
    }
    let entity = entity.observe(actions::activate).id();
    let label = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(label);
    commands.entity(parent).add_child(entity);
}
fn field(commands: &mut Commands, skins: &Skins, parent: Entity, window: usize, value: &str) {
    let entity = commands
        .spawn((
            Field(window),
            TextInput,
            EditableText {
                max_characters: Some(128),
                ..EditableText::new(value)
            },
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::no_wrap(),
            TextCursorStyle::default(),
            TabIndex(0),
            Node {
                width: percent(100.),
                height: px(26.),
                padding: UiRect::all(px(4.)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.08, 0.08, 0.08)),
        ))
        .id();
    commands.entity(parent).add_child(entity);
}
